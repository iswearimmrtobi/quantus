//! Recursive-descent parser for the LaTeX subset from F2.
//!
//! Grammar (each level binds tighter than the one above):
//! ```text
//! formula     := [ label '=' ] expression [ '=' expression ]
//! expression  := term ( ('+' | '-') term )*
//! term        := unary ( ('\cdot' | '\times' | '*' | '/' | juxtaposition) unary )*
//! unary       := ('-' | '+') unary | power
//! power       := atom [ '^' ( '\circ' | '{\circ}' | unary ) ]
//! atom        := number | identifier | '\frac' arg arg | '\sqrt' ['[' expr ']'] arg
//!              | ('\sin' | '\cos' | '\tan' | '\ln' | '\log') ['^' arg] argument
//!              | '\sum' '_{' name '=' expr '}' '^' arg term
//!              | '\int' '_' arg '^' arg expression differential
//!              | '\lim' '_{' name '\to' expr '}' term
//!              | '\frac{d}{dx}' term [ '|_{x=' expr '}' ]
//!              | '\begin{pmatrix}' expr ('&' expr)* ('\\' expr ('&' expr)*)* '\end{pmatrix}'
//!              | '\det' atom
//!              | '\binom' arg arg | '|' expression '|'
//!              | '\mathrm' arg | '\text' arg | '\qty' arg arg | '(' expression ')' | '{' expression '}'
//! ```
//!
//! The interesting part is how bare identifiers are classified, the uniqueness rule from F2:
//! 1. identifier **with** a subscript: variable (`v_1`, `\omega_0`),
//! 2. otherwise the longest run of adjacent identifiers that resolves against the unit table:
//!    unit (`kg`, `mN`, `\mu m` is µm),
//! 3. otherwise the constant table (`\pi`, `e`, `i`),
//! 4. otherwise a single letter (`x`, `\theta`) is a free variable without index,
//! 5. otherwise a run of letters that are each a variable (`ab`) is their product `a \cdot b`,
//! 6. otherwise a parse error naming the position.
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

/// One letter or one Greek command: the spelling of an unindexed variable.
fn is_single_letter(text: &str) -> bool {
    text.starts_with('\\') || text.chars().count() == 1
}

/// A run of letters like `ab` that is no unit but reads as a product of single-letter variables:
/// every letter must be a variable on its own, so `xm` (x times metre?) stays an error.
fn is_variable_product(text: &str) -> bool {
    text.chars().count() > 1
        && text
            .chars()
            .all(|c| c.is_ascii_alphabetic() && (c == 'a' || c == 'y' || units::find(&c.to_string()).is_none()))
}

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
    Parser { source, tokens: tokenize(source)?, index: 0, unit_context: false, abs_depth: 0, bound: Vec::new(), stops: Vec::new() }.parse()
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
    /// Names bound by an enclosing `\sum`, `\int` or `\lim`. They win over units and
    /// constants, so `i` in `\sum_{i=1}^{3} i` is the index and not the imaginary unit.
    bound: Vec<String>,
    /// Token positions where an enclosing integral's differential starts: juxtaposition stops
    /// there, so `x\,dx` is not the product of `x` and a unit `dx`.
    stops: Vec<usize>,
}

impl Parser<'_> {
    fn parse(mut self) -> Result<Formula> {
        if self.tokens.len() == 1 {
            return Err(self.error("The input is empty.", 0));
        }
        let label = self.parse_label()?;
        let mut expression = self.parse_expression()?;
        if self.current().kind == TokenType::Equals {
            // `x^2 - 5x + 6 = 0`: an equation to solve, not a label.
            self.advance();
            let right = self.parse_expression()?;
            let position = expression.position();
            expression = Node::Equation { left: Box::new(expression), right: Box::new(right), position };
        }
        if self.current().kind == TokenType::Equals {
            return Err(self.error("Only one '=' is allowed.", self.current().position));
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
            "\\frac" | "\\dfrac" | "\\tfrac" if self.derivative_header().is_some() => self.parse_derivative(token),
            "\\frac" | "\\dfrac" | "\\tfrac" => {
                self.advance();
                let numerator = self.parse_argument(&token.text)?;
                let denominator = self.parse_argument(&token.text)?;
                Ok(Node::fraction(numerator, denominator, position))
            }
            "\\sin" | "\\cos" | "\\tan" | "\\ln" | "\\log" => self.parse_function(token),
            "\\begin" => self.parse_matrix(token),
            "\\det" => {
                self.advance();
                let operand = self.parse_atom()?;
                Ok(Node::Determinant { operand: Box::new(operand), position })
            }
            "\\sum" => self.parse_sum(token),
            "\\int" => self.parse_integral(token),
            "\\lim" => self.parse_limit(token),
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

    /// `\sum_{i=1}^{n} body`. The body is a whole term, so `\sum_{i=1}^{3} i \cdot 2` sums the
    /// product and a following `+` ends it.
    fn parse_sum(&mut self, token: &Token) -> Result<Node> {
        self.advance();
        self.expect(TokenType::Underscore, "'_' with the index, as in \\sum_{i=1}^{n}")?;
        self.expect(TokenType::LBrace, "'{'")?;
        let index = self.parse_bound_name("\\sum")?;
        self.expect(TokenType::Equals, "'=' after the summation index")?;
        let from = self.parse_expression()?;
        self.expect(TokenType::RBrace, "'}'")?;
        self.expect(TokenType::Caret, "'^' with the upper limit, as in \\sum_{i=1}^{n}")?;
        let to = self.parse_argument(&token.text)?;
        let body = self.with_bound(&index, |p| p.parse_term())?;
        Ok(Node::Sum {
            index,
            from: Box::new(from),
            to: Box::new(to),
            body: Box::new(body),
            position: token.position,
        })
    }

    /// `\lim_{x \to a} body`, two-sided; the body is a term like a sum's.
    fn parse_limit(&mut self, token: &Token) -> Result<Node> {
        self.advance();
        self.expect(TokenType::Underscore, "'_' with the limit, as in \\lim_{x \\to 0}")?;
        self.expect(TokenType::LBrace, "'{'")?;
        let variable = self.parse_bound_name("\\lim")?;
        if !(self.current().kind == TokenType::Command && matches!(self.current().text.as_str(), "\\to" | "\\rightarrow")) {
            return Err(self.error("Expected '\\to' here.", self.current().position));
        }
        self.advance();
        let target = self.parse_expression()?;
        self.expect(TokenType::RBrace, "'}'")?;
        let body = self.with_bound(&variable, |p| p.parse_term())?;
        Ok(Node::Limit { variable, target: Box::new(target), body: Box::new(body), position: token.position })
    }

    /// `\int_a^b body \, dx`. The differential is found first, so the integration variable is
    /// known (and wins over a unit of the same name) while the body is read.
    fn parse_integral(&mut self, token: &Token) -> Result<Node> {
        self.advance();
        if self.current().kind != TokenType::Underscore {
            return Err(self.error("An integral needs limits, as in \\int_{0}^{1} x \\, dx.", self.current().position));
        }
        self.advance();
        let lower = self.parse_argument(&token.text)?;
        self.expect(TokenType::Caret, "'^' with the upper limit, as in \\int_{0}^{1}")?;
        let upper = self.parse_argument(&token.text)?;

        let (variable, stop, length) = self.find_differential(token.position)?;
        self.stops.push(stop);
        let body = self.with_bound(&variable, |p| p.parse_expression());
        self.stops.pop();
        let body = body?;
        if self.index != stop {
            return Err(self.error(&format!("Expected 'd{variable}' here."), self.current().position));
        }
        self.index += length;
        Ok(Node::Integral {
            variable,
            lower: Box::new(lower),
            upper: Box::new(upper),
            body: Box::new(body),
            position: token.position,
        })
    }

    /// `\begin{pmatrix} a & b \\ c & d \end{pmatrix}`; `vmatrix`, the bars of a determinant,
    /// gives the determinant of the matrix.
    fn parse_matrix(&mut self, token: &Token) -> Result<Node> {
        self.advance();
        let environment = self.parse_environment_name("\\begin")?;
        let determinant = match environment.as_str() {
            "pmatrix" | "bmatrix" | "matrix" => false,
            "vmatrix" => true,
            other => {
                return Err(self.error(&format!("Unsupported environment '{other}'."), token.position));
            }
        };

        let mut rows: Vec<Vec<Node>> = vec![Vec::new()];
        loop {
            let cell = self.parse_expression()?;
            rows.last_mut().expect("there is always a row").push(cell);
            match self.current().kind {
                TokenType::Ampersand => {
                    self.advance();
                }
                TokenType::RowBreak => {
                    self.advance();
                    if self.is_end_command() {
                        break;
                    }
                    rows.push(Vec::new());
                }
                _ if self.is_end_command() => break,
                _ => return Err(self.error("Expected '&', '\\\\' or '\\end' in the matrix.", self.current().position)),
            }
        }
        self.advance(); // \end
        let closing = self.parse_environment_name("\\end")?;
        if closing != environment {
            return Err(self.error(&format!("\\begin{{{environment}}} is closed by \\end{{{closing}}}."), token.position));
        }

        if rows.iter().any(|row| row.len() != rows[0].len()) {
            return Err(self.error("All rows of a matrix need the same number of entries.", token.position));
        }
        let matrix = Node::Matrix { rows, position: token.position };
        Ok(if determinant {
            Node::Determinant { operand: Box::new(matrix), position: token.position }
        } else {
            matrix
        })
    }

    fn is_end_command(&self) -> bool {
        self.current().kind == TokenType::Command && self.current().text == "\\end"
    }

    /// The `{name}` after `\begin` or `\end`.
    fn parse_environment_name(&mut self, command: &str) -> Result<String> {
        self.expect(TokenType::LBrace, &format!("'{{' after {command}"))?;
        let name = self.current().clone();
        if name.kind != TokenType::Identifier {
            return Err(self.error(&format!("{command} needs the name of an environment."), name.position));
        }
        self.advance();
        self.expect(TokenType::RBrace, "'}'")?;
        Ok(name.text)
    }

    /// `{d}{dx}` right after a `\frac`: the variable and the token index after the header.
    fn derivative_header(&self) -> Option<(String, usize)> {
        let kind = |i: usize| self.peek(i - self.index).kind;
        let text = |i: usize| self.peek(i - self.index).text.as_str();
        let mut i = self.index + 1;
        if kind(i) != TokenType::LBrace {
            return None;
        }
        i += 1;
        if kind(i) == TokenType::Identifier && text(i) == "d" && kind(i + 1) == TokenType::RBrace {
            i += 2;
        } else if kind(i) == TokenType::Command
            && text(i) == "\\mathrm"
            && kind(i + 1) == TokenType::LBrace
            && text(i + 2) == "d"
            && kind(i + 3) == TokenType::RBrace
            && kind(i + 4) == TokenType::RBrace
        {
            i += 5;
        } else {
            return None;
        }
        if kind(i) != TokenType::LBrace {
            return None;
        }
        let (name, length) = self.differential_at(i + 1)?;
        (kind(i + 1 + length) == TokenType::RBrace).then_some((name, i + 2 + length))
    }

    /// `\frac{d}{dx} body`, and `\left.\frac{d}{dx} body\right|_{x=a}` for the value at a point.
    fn parse_derivative(&mut self, token: &Token) -> Result<Node> {
        let (variable, after_header) = self.derivative_header().expect("checked by the caller");
        self.index = after_header;
        let body = self.with_bound(&variable, |p| p.parse_term())?;
        let at = if self.current().kind == TokenType::Pipe && self.peek(1).kind == TokenType::Underscore {
            self.index += 2;
            self.expect(TokenType::LBrace, "'{'")?;
            let named = self.parse_bound_name("the evaluation point")?;
            if named != variable {
                return Err(self.error(&format!("The point has to be given for {variable}, as in _{{{variable}=2}}."), self.current().position));
            }
            self.expect(TokenType::Equals, "'='")?;
            let at = self.parse_expression()?;
            self.expect(TokenType::RBrace, "'}'")?;
            Some(Box::new(at))
        } else {
            None
        };
        // In `\frac{d}{dx}\frac{d}{dx} f |_{x=2}` the inner derivative has already taken the
        // point; it belongs to the outermost one.
        let (body, at) = match (body, at) {
            (Node::Derivative { variable, body, at: Some(point), position }, None) => {
                (Node::Derivative { variable, body, at: None, position }, Some(point))
            }
            other => other,
        };
        Ok(Node::Derivative { variable, body: Box::new(body), at, position: token.position })
    }

    /// The single-letter name a sum, limit or integral binds.
    fn parse_bound_name(&mut self, command: &str) -> Result<String> {
        let token = self.current().clone();
        if token.kind != TokenType::Identifier || !is_single_letter(&token.text) {
            return Err(self.error(&format!("{command} needs a single letter as its variable."), token.position));
        }
        self.advance();
        Ok(token.text)
    }

    fn with_bound<T>(&mut self, name: &str, block: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        self.bound.push(name.to_string());
        let result = block(self);
        self.bound.pop();
        result
    }

    /// Looks ahead for the `dx` that closes the integral starting at `position`: the first
    /// differential outside brackets that is not claimed by a nested `\int`.
    /// Returns the variable, the token index of the differential and how many tokens it spans.
    fn find_differential(&self, position: usize) -> Result<(String, usize, usize)> {
        let mut depth = 0_i32;
        let mut nested = 0;
        let mut i = self.index;
        loop {
            let token = &self.tokens[i];
            if depth == 0 {
                if let Some((name, length)) = self.differential_at(i) {
                    if nested == 0 {
                        return Ok((name, i, length));
                    }
                    nested -= 1;
                    i += length;
                    continue;
                }
            }
            match token.kind {
                TokenType::End => {
                    return Err(self.error("The integral is missing its differential, e.g. \\, dx.", position));
                }
                TokenType::LParen | TokenType::LBrace | TokenType::LBracket => depth += 1,
                TokenType::RParen | TokenType::RBrace | TokenType::RBracket => depth -= 1,
                TokenType::Command if token.text == "\\int" && depth == 0 => nested += 1,
                _ => {}
            }
            if depth < 0 {
                return Err(self.error("The integral is missing its differential, e.g. \\, dx.", position));
            }
            i += 1;
        }
    }

    /// `dx`, `d x`, `d\theta` or `\mathrm{d}x` starting at token `i`: the variable and the
    /// number of tokens used.
    fn differential_at(&self, i: usize) -> Option<(String, usize)> {
        let token = |offset: usize| &self.tokens[(i + offset).min(self.tokens.len() - 1)];
        let is_name = |t: &Token| t.kind == TokenType::Identifier && is_single_letter(&t.text);
        let first = token(0);
        if first.kind == TokenType::Identifier {
            if let Some(rest) = first.text.strip_prefix('d') {
                if rest.chars().count() == 1 {
                    return Some((rest.to_string(), 1));
                }
                if rest.is_empty() && is_name(token(1)) {
                    return Some((token(1).text.clone(), 2));
                }
            }
        }
        if first.kind == TokenType::Command
            && first.text == "\\mathrm"
            && token(1).kind == TokenType::LBrace
            && token(2).kind == TokenType::Identifier
            && token(2).text == "d"
            && token(3).kind == TokenType::RBrace
            && is_name(token(4))
        {
            return Some((token(4).text.clone(), 5));
        }
        None
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

        if !self.unit_context && self.bound.contains(&token.text) {
            self.advance();
            return Ok(Node::Variable { name: token.text, index: String::new(), position: token.position });
        }

        // The whole run has to resolve as one symbol: `m s` is the millisecond, and a product
        // of units is only ever written with an explicit \cdot.
        let run = self.identifier_run();
        let symbol: String = run.iter().map(|t| greek_symbol(&t.text)).collect();

        // Disambiguation: outside explicit unit context, single-letter 'y' and 'a' (annum/year)
        // are free algebraic variables rather than unit aliases.
        let is_reclassified_variable = !self.unit_context && (symbol == "y" || symbol == "a");

        if !is_reclassified_variable {
            if let Some(unit) = units::find(&symbol) {
                self.index += run.len();
                return Ok(Node::Unit { unit, symbol, position: token.position });
            }
        }

        // Where the run as a whole is no unit, its first symbol can still stand alone (`\pi x`,
        // `x y`) unless it is itself a unit: `m kg` stays an error, a product of units needs \cdot.
        let first_is_unit = !is_reclassified_variable && units::find(greek_symbol(&token.text)).is_some();
        if !self.unit_context && !first_is_unit && CONSTANTS.contains(&token.text.as_str()) {
            self.advance();
            return Ok(Node::Constant { name: token.text, position: token.position });
        }

        if !self.unit_context && !first_is_unit && is_single_letter(&token.text) {
            self.advance();
            return Ok(Node::Variable { name: token.text, index: String::new(), position: token.position });
        }

        if !self.unit_context && is_variable_product(&token.text) {
            return Ok(self.take_first_letter());
        }

        let hint = if run.len() > 1 { " A product of units needs an explicit \\cdot." } else { "" };
        let reason = if self.unit_context {
            format!("'{symbol}' is not a known unit.{hint}")
        } else {
            format!("'{symbol}' is neither a unit nor a constant. A variable is a single letter or carries a subscript, e.g. x or v_1.{hint}")
        };
        Err(self.error(&reason, token.position))
    }

    /// `ab` in `2ab` becomes the variable `a`, with `b` left behind as the next token to be read,
    /// so juxtaposition makes it `2 \cdot a \cdot b` and an exponent binds to `b` alone.
    fn take_first_letter(&mut self) -> Node {
        let token = self.current().clone();
        let (first, rest) = token.text.split_at(1);
        self.tokens[self.index] =
            Token { kind: TokenType::Identifier, text: rest.to_string(), position: token.position + 1 };
        Node::Variable { name: first.to_string(), index: String::new(), position: token.position }
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
        if self.stops.contains(&self.index) {
            return false;
        }
        let token = self.current();
        match token.kind {
            TokenType::Number | TokenType::Identifier | TokenType::LParen | TokenType::LBrace => true,
            // `|_{x=2}` after a derivative is its evaluation point, not an absolute value.
            TokenType::Pipe => self.abs_depth == 0 && self.peek(1).kind != TokenType::Underscore,
            TokenType::Command => {
                !MULTIPLICATION_COMMANDS.contains(&token.text.as_str()) && token.text != "\\div" && token.text != "\\end"
            }
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
