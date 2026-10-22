//! Recursive-descent parser for the LaTeX subset from F2.
//!
//! Grammar (each level binds tighter than the one above):
//! ```text
//! formula     := [ label '=' ] expression
//! expression  := term ( ('+' | '-') term )*
//! term        := unary ( ('\cdot' | '\times' | '*' | '/' | juxtaposition) unary )*
//! unary       := ('-' | '+') unary | power
//! power       := atom [ '^' ( '\circ' | '{\circ}' | unary ) ]
//! atom        := number | identifier | '\frac' arg arg | '\sqrt' ['[' expr ']'] arg
//!              | ('\sin' | '\cos' | '\tan' | '\ln' | '\log') ['^' arg] argument
//!              | '\binom' arg arg | '|' expression '|'
//!              | '\mathrm' arg | '\text' arg | '\qty' arg arg | '(' expression ')' | '{' expression '}'
//! ```
//!
//! The interesting part is how bare identifiers are classified, the uniqueness rule from F2:
//! 1. identifier **with** a subscript: variable (`v_1`, `\omega_0`),
//! 2. otherwise the longest run of adjacent identifiers that resolves against the unit table:
//!    unit (`kg`, `mN`, `\mu m` is µm),
//! 3. otherwise the constant table (`\pi`, `e`, `i`),
//! 4. otherwise a parse error naming the position.
//!
//! Step 2 is deliberately greedy, which is what makes implicit unit products illegal: `m s`
//! resolves to millisecond, not metre·second. Products of units need an explicit `\cdot`.

use super::ast::{Formula, Label, Node, Operator};
use super::lexer::{tokenize, Token, TokenType};
use super::normalizer;
use crate::num::Decimal;
use crate::units;
use crate::{QuantusError, Result};

const MULTIPLICATION_COMMANDS: [&str; 3] = ["\\cdot", "\\times", "\\ast"];
const CONSTANTS: [&str; 3] = ["\\pi", "e", "i"];

fn is_function_command(token: &Token) -> bool {
    token.kind == TokenType::Command && matches!(token.text.as_str(), "\\sin" | "\\cos" | "\\tan" | "\\ln" | "\\log")
}

/// `\mu` is the µ of `µm`, `\Omega` the Ω of `k\Omega`.
fn greek_symbol(text: &str) -> &str {
    match text {
        "\\mu" => "µ",
        "\\Omega" => "Ω",
        "\\omega" => "ω",
        other => other,
    }
}

pub fn parse(source: &str) -> Result<Formula> {
    Parser { source, tokens: tokenize(source)?, index: 0, unit_context: false, abs_depth: 0 }.parse()
}

struct Parser<'a> {
    source: &'a str,
    // Mutable because a brace-less argument may split a number token in two (`take_first_digit`).
    tokens: Vec<Token>,
    index: usize,
    /// Inside `\mathrm{}`/`\text{}`/`\qty{}{}` every identifier must be a unit.
    unit_context: bool,
    /// How many `|…|` are open. Inside one, a `|` closes it instead of starting a new one, which
    /// is what makes `|a|b|` unambiguous; brackets and braces start afresh.
    abs_depth: usize,
}

impl Parser<'_> {
    fn parse(mut self) -> Result<Formula> {
        if self.tokens.len() == 1 {
            return Err(self.error("The input is empty.", 0));
        }
        let label = self.parse_label()?;
        let expression = self.parse_expression()?;
        if self.current().kind == TokenType::Equals {
            return Err(self.error("Only one '=' is allowed, and only right after the label.", self.current().position));
        }
        self.expect(TokenType::End, "the end of the input")?;
        Ok(Formula { label, expression: normalizer::normalize(expression), source: self.source.to_string() })
    }

    // ---- structure -------------------------------------------------------------------

    /// `F =` or `r_1 =`; anything else means the input is a bare expression.
    fn parse_label(&mut self) -> Result<Option<Label>> {
        let start = self.index;
        if self.current().kind != TokenType::Identifier {
            return Ok(None);
        }
        let name = self.advance();
        let subscript = if self.current().kind == TokenType::Underscore {
            self.advance();
            Some(self.parse_subscript()?)
        } else {
            None
        };
        if self.current().kind != TokenType::Equals {
            self.index = start;
            return Ok(None);
        }
        self.advance();
        Ok(Some(Label { name: name.text, index: subscript, position: name.position }))
    }

    fn parse_expression(&mut self) -> Result<Node> {
        let mut left = self.parse_term()?;
        while matches!(self.current().kind, TokenType::Plus | TokenType::Minus) {
            let operator = if self.advance().kind == TokenType::Plus { Operator::Plus } else { Operator::Minus };
            let right = self.parse_term()?;
            let position = left.position();
            left = Node::binary(operator, left, right, position);
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> Result<Node> {
        let mut left = self.parse_unary()?;
        loop {
            let token = self.current();
            let operator = match token.kind {
                TokenType::Times => Operator::Times,
                TokenType::Divide => Operator::Divide,
                TokenType::Command if MULTIPLICATION_COMMANDS.contains(&token.text.as_str()) => Operator::Times,
                TokenType::Command if token.text == "\\div" => Operator::Divide,
                _ if self.starts_atom() => Operator::ImplicitTimes,
                _ => return Ok(left),
            };
            if operator != Operator::ImplicitTimes {
                self.advance();
            }
            let right = self.parse_unary()?;
            let position = left.position();
            left = Node::binary(operator, left, right, position);
        }
    }

    fn parse_unary(&mut self) -> Result<Node> {
        match self.current().kind {
            TokenType::Minus => {
                let token = self.advance();
                Ok(Node::Negation { operand: Box::new(self.parse_unary()?), position: token.position })
            }
            TokenType::Plus => {
                self.advance();
                self.parse_unary()
            }
            _ => self.parse_power(),
        }
    }

    fn parse_power(&mut self) -> Result<Node> {
        let base = self.parse_atom()?;
        if self.current().kind != TokenType::Caret {
            return Ok(base);
        }
        self.advance();
        let position = base.position();
        if self.at_degree_sign() {
            return Ok(Node::Degrees { operand: Box::new(base), position });
        }
        Ok(Node::Power { base: Box::new(base), exponent: Box::new(self.parse_unary()?), position })
    }

    /// `\circ` or `{\circ}` after a `^`; consumes it when present.
    fn at_degree_sign(&mut self) -> bool {
        let is_circ = |t: &Token| t.kind == TokenType::Command && t.text == "\\circ";
        if is_circ(self.current()) {
            self.advance();
            return true;
        }
        if self.current().kind == TokenType::LBrace
            && is_circ(self.peek(1))
            && self.peek(2).kind == TokenType::RBrace
        {
            self.index += 3;
            return true;
        }
        false
    }

    fn parse_atom(&mut self) -> Result<Node> {
        let token = self.current().clone();
        match token.kind {
            TokenType::Number => {
                self.advance();
                Ok(Node::Number { value: self.decimal(&token.text, token.position)?, position: token.position })
            }
            TokenType::Identifier => self.parse_identifier(),
            TokenType::Pipe => self.parse_abs(),
            TokenType::LParen => self.parse_group(TokenType::RParen, ")"),
            TokenType::LBrace => self.parse_group(TokenType::RBrace, "}"),
            TokenType::Command => self.parse_command(&token),
            _ => Err(self.error("Expected a number, a unit or a variable here.", token.position)),
        }
    }

    fn parse_abs(&mut self) -> Result<Node> {
        let open = self.advance();
        self.abs_depth += 1;
        let operand = self.parse_expression();
        self.abs_depth -= 1;
        let operand = operand?;
        if self.current().kind != TokenType::Pipe {
            return Err(self.error("The absolute value is missing its closing '|'.", open.position));
        }
        self.advance();
        Ok(Node::Abs { operand: Box::new(operand), position: open.position })
    }

    fn parse_group(&mut self, closing: TokenType, closing_text: &str) -> Result<Node> {
        let open = self.advance();
        let outer_abs_depth = std::mem::replace(&mut self.abs_depth, 0);
        let inner = self.parse_expression();
        self.abs_depth = outer_abs_depth;
        let inner = inner?;
        self.expect(closing, &format!("'{closing_text}'"))?;
        Ok(Node::Group { inner: Box::new(inner), position: open.position })
    }

    fn parse_command(&mut self, token: &Token) -> Result<Node> {
        let position = token.position;
        match token.text.as_str() {
            "\\frac" | "\\dfrac" | "\\tfrac" => {
                self.advance();
                let numerator = self.parse_argument(&token.text)?;
                let denominator = self.parse_argument(&token.text)?;
                Ok(Node::fraction(numerator, denominator, position))
            }
            "\\sin" | "\\cos" | "\\tan" | "\\ln" | "\\log" => self.parse_function(token),
            "\\binom" | "\\dbinom" | "\\tbinom" => {
                self.advance();
                let n = self.parse_argument(&token.text)?;
                let k = self.parse_argument(&token.text)?;
                Ok(Node::Binomial { n: Box::new(n), k: Box::new(k), position })
            }
            "\\sqrt" => {
                self.advance();
                let degree = if self.current().kind == TokenType::LBracket {
                    self.advance();
                    let degree = self.parse_expression()?;
                    self.expect(TokenType::RBracket, "']'")?;
                    Some(Box::new(degree))
                } else {
                    None
                };
                let radicand = self.parse_argument(&token.text)?;
                Ok(Node::Root { radicand: Box::new(radicand), degree, position })
            }
            // Explicit unit markup: \mathrm{kg}, \text{m/s^2}, siunitx' \si{} and \unit{}.
            // \operatorname{} belongs here too: that is how math editors wrap any identifier of
            // more than one letter, and in this language such an identifier is a unit.
            "\\mathrm" | "\\text" | "\\si" | "\\unit" | "\\operatorname" => {
                self.advance();
                self.in_unit_context(|p| p.parse_argument(&token.text))
            }
            // siunitx: \qty{12}{cm} is a number and a unit, i.e. an implicit product.
            "\\qty" | "\\SI" | "\\num" => {
                self.advance();
                let value = self.parse_argument(&token.text)?;
                if token.text == "\\num" {
                    return Ok(value);
                }
                let unit = self.in_unit_context(|p| p.parse_argument(&token.text))?;
                Ok(Node::binary(Operator::ImplicitTimes, value, unit, position))
            }
            other => Err(self.error(&format!("Unsupported command '{other}'."), position)),
        }
    }

    /// `\sin x`, `\sin(x)`, `\sin{x}`, `\sin^2 x`.
    ///
    /// A bracketed or braced argument is exactly that group, so `\sin(x)y` is `\sin(x) \cdot y`.
    /// Without brackets the argument is the implicit product that follows (`\sin 2\pi x`,
    /// `\sin 30^\circ`), cut off at an explicit operator or at the next function, which makes
    /// `\sin x \cos x` a product of two functions.
    fn parse_function(&mut self, token: &Token) -> Result<Node> {
        let position = token.position;
        let name = token.text.trim_start_matches('\\').to_string();
        self.advance();
        if self.current().kind == TokenType::Underscore {
            return Err(self.error(
                &format!("A base on {} is not supported. Write \\ln x / \\ln b instead.", token.text),
                self.current().position,
            ));
        }
        let exponent = if self.current().kind == TokenType::Caret {
            let caret = self.advance();
            let exponent = self.parse_argument(&token.text)?;
            if matches!(&exponent, Node::Negation { operand, .. } if matches!(**operand, Node::Number { .. }))
                || matches!(&exponent, Node::Group { inner, .. } if matches!(**inner, Node::Negation { .. }))
            {
                return Err(self.error(
                    &format!("{}^{{-1}} is not supported; write 1/{} x instead.", token.text, token.text),
                    caret.position,
                ));
            }
            Some(exponent)
        } else {
            None
        };

        let argument = if matches!(self.current().kind, TokenType::LParen | TokenType::LBrace) {
            self.parse_atom()?
        } else {
            self.parse_function_operand(&token.text)?
        };
        let function = Node::Function { name, argument: Box::new(argument), position };
        Ok(match exponent {
            Some(exponent) => Node::Power { base: Box::new(function), exponent: Box::new(exponent), position },
            None => function,
        })
    }

    fn parse_function_operand(&mut self, command: &str) -> Result<Node> {
        let first = self.current().clone();
        if !self.starts_atom() || is_function_command(&first) {
            return Err(self.error(
                &format!("'{command}' is missing its argument — write it as {command}{{…}}."),
                first.position,
            ));
        }
        let mut operand = self.parse_power()?;
        while self.starts_atom() && !is_function_command(self.current()) {
            let right = self.parse_power()?;
            let position = operand.position();
            operand = Node::binary(Operator::ImplicitTimes, operand, right, position);
        }
        Ok(operand)
    }

    /// A command argument: `{…}`, or a single token if no brace follows.
    ///
    /// The brace-less form is LaTeX's own rule and not a convenience of ours: `\frac12` is one
    /// half, not twelve over something. Editors emit it constantly (MathLive turns `2/3` into
    /// `\frac23`), so rejecting it would mean rejecting what our own input field produces.
    ///
    /// "One token" is one *character*, which is why a multi-digit number is split here: in
    /// `\frac12` the numerator is the 1 and the 2 stays behind as the denominator.
    fn parse_argument(&mut self, command: &str) -> Result<Node> {
        if self.current().kind == TokenType::LBrace {
            return self.parse_group(TokenType::RBrace, "}");
        }
        let token = self.current().clone();
        match token.kind {
            TokenType::Number => {
                let digit = self.take_first_digit();
                Ok(Node::Number { value: self.decimal(&digit, token.position)?, position: token.position })
            }
            TokenType::Identifier | TokenType::Command => self.parse_atom(),
            _ => Err(self.error(
                &format!("'{command}' is missing its argument — write it as {command}{{…}}."),
                token.position,
            )),
        }
    }

    /// Takes the first digit of the current number and leaves the rest in its place, so that the
    /// `12` in `\frac12` becomes the numerator `1` and a denominator `2` still to be read.
    ///
    /// A number with a decimal point stays whole: `\frac1{,}5` cannot mean anything sensible
    /// digit by digit, and silently dropping everything after the `1` would be worse than
    /// treating it as one argument.
    fn take_first_digit(&mut self) -> String {
        let token = self.current().clone();
        if token.text.len() == 1 || token.text.contains('.') {
            self.advance();
            return token.text;
        }
        let (first, rest) = token.text.split_at(1);
        self.tokens[self.index] =
            Token { kind: TokenType::Number, text: rest.to_string(), position: token.position + 1 };
        first.to_string()
    }

    fn in_unit_context<T>(&mut self, block: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        let previous = std::mem::replace(&mut self.unit_context, true);
        let result = block(self);
        self.unit_context = previous;
        result
    }

    // ---- identifiers: the uniqueness rule from F2 ------------------------------------

    fn parse_identifier(&mut self) -> Result<Node> {
        let token = self.current().clone();
        if !self.unit_context && self.peek(1).kind == TokenType::Underscore {
            self.advance();
            self.advance();
            let index = self.parse_subscript()?;
            return Ok(Node::Variable { name: token.text, index, position: token.position });
        }

        // The whole run has to resolve as one symbol: `m s` is the millisecond, and a product
        // of units is only ever written with an explicit \cdot.
        let run = self.identifier_run();
        let symbol: String = run.iter().map(|t| greek_symbol(&t.text)).collect();
        if let Some(unit) = units::find(&symbol) {
            self.index += run.len();
            return Ok(Node::Unit { unit, symbol, position: token.position });
        }

        if !self.unit_context && run.len() == 1 && CONSTANTS.contains(&token.text.as_str()) {
            self.advance();
            return Ok(Node::Constant { name: token.text, position: token.position });
        }

        let hint = if run.len() > 1 { " A product of units needs an explicit \\cdot." } else { "" };
        let reason = if self.unit_context {
            format!("'{symbol}' is not a known unit.{hint}")
        } else {
            format!("'{symbol}' is neither a unit nor a constant. Variables must carry a subscript, e.g. v_1.{hint}")
        };
        Err(self.error(&reason, token.position))
    }

    /// The adjacent identifiers eligible for the longest-match lookup. An identifier carrying a
    /// subscript belongs to a variable and ends the run before it.
    fn identifier_run(&self) -> Vec<Token> {
        let mut run = Vec::new();
        let mut offset = 0;
        while self.peek(offset).kind == TokenType::Identifier && self.peek(offset + 1).kind != TokenType::Underscore {
            run.push(self.peek(offset).clone());
            offset += 1;
        }
        run
    }

    /// The index of a variable: `r_1`, `r_{12}`, `x_{ges}`.
    fn parse_subscript(&mut self) -> Result<String> {
        if self.current().kind != TokenType::LBrace {
            let token = self.current().clone();
            if !matches!(token.kind, TokenType::Number | TokenType::Identifier) {
                return Err(self.error("Expected an index after '_'.", token.position));
            }
            self.advance();
            return Ok(token.text);
        }
        self.advance();
        let mut index = String::new();
        while self.current().kind != TokenType::RBrace {
            if self.current().kind == TokenType::End {
                return Err(self.error("The index is missing its closing '}'.", self.current().position));
            }
            index.push_str(&self.advance().text);
        }
        self.advance();
        if index.is_empty() {
            return Err(self.error("The index must not be empty.", self.current().position));
        }
        Ok(index)
    }

    // ---- token handling ---------------------------------------------------------------

    fn current(&self) -> &Token {
        &self.tokens[self.index]
    }

    fn peek(&self, offset: usize) -> &Token {
        &self.tokens[(self.index + offset).min(self.tokens.len() - 1)]
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens[self.index].clone();
        self.index += 1;
        token
    }

    fn expect(&mut self, kind: TokenType, description: &str) -> Result<()> {
        if self.current().kind != kind {
            return Err(self.error(&format!("Expected {description} here."), self.current().position));
        }
        if kind != TokenType::End {
            self.advance();
        }
        Ok(())
    }

    /// Does the current token start an atom, i.e. is this juxtaposition (`2kg`)?
    fn starts_atom(&self) -> bool {
        let token = self.current();
        match token.kind {
            TokenType::Number | TokenType::Identifier | TokenType::LParen | TokenType::LBrace => true,
            TokenType::Pipe => self.abs_depth == 0,
            TokenType::Command => !MULTIPLICATION_COMMANDS.contains(&token.text.as_str()) && token.text != "\\div",
            _ => false,
        }
    }

    fn decimal(&self, text: &str, position: usize) -> Result<Decimal> {
        Decimal::parse(text).ok_or_else(|| self.error(&format!("'{text}' is not a valid number."), position))
    }

    fn error(&self, reason: &str, position: usize) -> QuantusError {
        QuantusError::parse(reason, position, self.source)
    }
}
