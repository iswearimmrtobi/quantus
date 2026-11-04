//! Recursive-descent parser for the LaTeX subset from F2.
//!
//! Grammar (each level binds tighter than the one above):
//! ```text
//! formula     := [ label '=' ] expression [ '=' expression ] | system
//! system      := '\begin{cases}' equation ('\\' equation)* '\end{cases}' | equation (';' equation)*
//! equation    := expression ['&'] '=' ['&'] expression ['&']
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
//! Two of these choices are [`ParseOptions`]: `a` and `y` skip step 2 and are variables rather
//! than annum and year, and step 5 can be switched off so that `ab` stays an error.
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
/// Functions written as `\operatorname{name}(…)` and parsed as a [`Node::Call`] of that name,
/// ahead of the unit markup `\operatorname{}` otherwise is.
/// The call a chain of comparisons becomes: `relation < \le` with the operands as arguments.
pub(crate) const RELATION: &str = "relation";

/// The call a conversion `value \blacktriangleright unit` becomes.
pub(crate) const CONVERSION: &str = "convert";

/// Each is `(spelling, name)`: `ggT` and `gcd` are both the call `gcd`.
pub(crate) const NAMED_CALLS: [(&str, &str); 13] = [
    ("Identity", "Identity"),
    ("ref", "ref"),
    ("rref", "rref"),
    ("nPr", "nPr"),
    ("ggT", "gcd"),
    ("gcd", "gcd"),
    ("kgV", "lcm"),
    ("lcm", "lcm"),
    ("Int", "Int"),
    // Intg is the floor.
    ("Intg", "floor"),
    ("Rnd", "Rnd"),
    ("RndFix", "RndFix"),
    ("divR", "divR"),
];

/// How a call from [`NAMED_CALLS`] is written back, `None` for the ones with a command of their own.
pub(crate) fn named_call_spelling(name: &str) -> Option<&'static str> {
    if matches!(name, "gcd" | "floor") {
        return None;
    }
    NAMED_CALLS.iter().find(|(_, call)| *call == name).map(|(spelling, _)| *spelling)
}

/// One-argument functions LaTeX has no command for, written `\operatorname{arsinh}`, with the
/// name they get.
const OPERATOR_FUNCTIONS: [(&str, &str); 9] = [
    ("arsinh", "arsinh"),
    ("arcosh", "arcosh"),
    ("artanh", "artanh"),
    ("arcsinh", "arsinh"),
    ("arccosh", "arcosh"),
    ("arctanh", "artanh"),
    ("asinh", "arsinh"),
    ("acosh", "arcosh"),
    ("atanh", "artanh"),
];

/// Commands that take one argument and evaluate through `MathFunction`.
const FUNCTION_COMMANDS: [&str; 21] = [
    "\\sin", "\\cos", "\\tan", "\\ln", "\\log", "\\lg", "\\exp", "\\sinh", "\\cosh", "\\tanh", "\\coth", "\\cot",
    "\\sec", "\\csc", "\\arcsin", "\\arccos", "\\arctan",
    // Of a complex number (imaginary mode): real part, imaginary part, argument, conjugate.
    "\\Re", "\\Im", "\\arg", "\\overline",
];

/// The values a derivative's point gives, `|_{x=2, z=3}`, by variable.
type Bindings = Vec<(String, Node)>;
/// A point split into the value of the derivative's own variable and the others.
type Point = (Box<Node>, Bindings);

/// The `{d^n}{dx^n}` or `{\partial^2}{\partial x \partial z}` after a `\frac`.
struct DerivativeHeader {
    /// One entry per differentiation, outermost first: `[x, z]` for `\partial x \partial z`.
    variables: Vec<String>,
    partial: bool,
    /// The token index after the header.
    after: usize,
}

/// `body` differentiated by each of `variables`, the last one first, with the point on the
/// outermost derivative: `at` for its variable and `given` for the others.
fn nested_derivative(
    variables: &[String],
    body: Node,
    point: Option<Point>,
    partial: bool,
    position: usize,
) -> Node {
    let mut node = body;
    for variable in variables[1..].iter().rev() {
        node = Node::Derivative { variable: variable.clone(), body: Box::new(node), at: None, given: Vec::new(), partial, position };
    }
    let (at, given) = match point {
        Some((at, given)) => (Some(at), given),
        None => (None, Vec::new()),
    };
    Node::Derivative { variable: variables[0].clone(), body: Box::new(node), at, given, partial, position }
}

/// One letter or one Greek command: the spelling of an unindexed variable.
fn is_single_letter(text: &str) -> bool {
    text.starts_with('\\') || text.chars().count() == 1
}

/// A run of letters like `ab` that is no unit but reads as a product of single-letter variables:
/// every letter must be a variable on its own, so `xm` (x times metre?) stays an error.
fn is_variable_product(text: &str, options: &ParseOptions) -> bool {
    text.chars().count() > 1
        && text.chars().all(|c| {
            let letter = c.to_string();
            c.is_ascii_alphabetic() && (options.reads_as_variable(&letter) || units::find(&letter).is_none())
        })
}

fn is_function_command(token: &Token) -> bool {
    token.kind == TokenType::Command && FUNCTION_COMMANDS.contains(&token.text.as_str())
}

/// `-1` or `{-1}`.
fn is_minus_one(node: &Node) -> bool {
    match node {
        Node::Group { inner, .. } => is_minus_one(inner),
        Node::Negation { operand, .. } => {
            matches!(&**operand, Node::Number { value, .. } if value.to_rational() == dashu::rational::RBig::ONE)
        }
        _ => false,
    }
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

/// The user-adjustable parts of identifier classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseOptions {
    /// Outside a unit context, `a` and `y` are variables rather than annum and year.
    pub a_and_y_as_variables: bool,
    /// A run of letters that are each a variable (`ab`) is their product.
    pub letter_products: bool,
    /// Outside a unit context, `t`, `u` and `in` are the tonne, the atomic mass unit and the
    /// inch rather than variables.
    pub extra_unit_letters: bool,
    /// Outside a unit context, symbols are never units: every letter is a variable.
    pub units: bool,
    /// `i` outside a binding is the imaginary unit rather than a variable.
    pub imaginary_i: bool,
    /// The Geometry mode's notation: `\vec{a}` names a vector, `(1|2|3)` is a point and
    /// `\times` the cross product.
    pub geometry: bool,
}

impl Default for ParseOptions {
    fn default() -> Self {
        ParseOptions {
            a_and_y_as_variables: true,
            letter_products: true,
            extra_unit_letters: false,
            units: true,
            imaginary_i: true,
            geometry: false,
        }
    }
}

impl ParseOptions {
    /// Whether a symbol that names a unit is read as a variable instead (outside `\mathrm{}`).
    fn reads_as_variable(&self, symbol: &str) -> bool {
        !self.units
            || (self.a_and_y_as_variables && matches!(symbol, "a" | "y"))
            || (!self.extra_unit_letters && matches!(symbol, "t" | "u" | "in"))
    }
}

pub fn parse(source: &str) -> Result<Formula> {
    parse_with(source, &ParseOptions::default())
}

pub fn parse_with(source: &str, options: &ParseOptions) -> Result<Formula> {
    Parser {
        source,
        tokens: tokenize(source)?,
        index: 0,
        unit_context: false,
        abs_depth: 0,
        bound: Vec::new(),
        stops: Vec::new(),
        in_point: false,
        options: *options,
    }
    .parse()
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
    /// Inside the coordinates of a point (Geometry mode), where a plain `|` separates them.
    in_point: bool,
    options: ParseOptions,
}

impl Parser<'_> {
    fn parse(mut self) -> Result<Formula> {
        if self.tokens.len() == 1 {
            return Err(self.error("The input is empty.", 0));
        }
        if self.starts_cases() || self.has_top_level_semicolon() {
            let system = self.parse_system()?;
            self.expect(TokenType::End, "the end of the input")?;
            return Ok(Formula { label: None, expression: normalizer::normalize(system), source: self.source.to_string() });
        }
        let label = self.parse_label()?;
        let mut expression = self.parse_expression()?;
        if self.at_conversion() {
            // `5\,\mathrm{cm} \blacktriangleright \mathrm{in}`: the value written in the unit after it.
            let position = expression.position();
            self.advance();
            let target = self.conversion_target()?;
            expression = Node::Call { name: CONVERSION.into(), arguments: vec![expression, target], position };
        }
        // `x^2 - 5x + 6 = 0` is an equation to solve; `1 < 2 \le 2` and `a = b = c` a chain of
        // comparisons that is true or false.
        let position = expression.position();
        let mut operators = Vec::new();
        let mut operands = vec![expression];
        while matches!(self.current().kind, TokenType::Equals | TokenType::Relation) {
            let token = self.advance();
            operators.push(if token.kind == TokenType::Equals { "=".to_string() } else { token.text });
            operands.push(self.parse_expression()?);
        }
        let mut expression = operands.remove(0);
        if operators.len() == 1 && operators[0] == "=" {
            let right = operands.remove(0);
            expression = Node::Equation { left: Box::new(expression), right: Box::new(right), position };
        } else if !operators.is_empty() {
            operands.insert(0, expression);
            expression = Node::Call { name: format!("{RELATION} {}", operators.join(" ")), arguments: operands, position };
        }
        self.expect(TokenType::End, "the end of the input")?;
        Ok(Formula { label, expression: normalizer::normalize(expression), source: self.source.to_string() })
    }

    /// The unit after `▶`; `^\circ\mathrm{C}` and `^\circ F` stand alone there.
    fn conversion_target(&mut self) -> Result<Node> {
        if self.current().kind == TokenType::LBrace && self.peek(1).kind == TokenType::RBrace {
            self.index += 2;
        }
        if self.current().kind == TokenType::Caret {
            let position = self.advance().position;
            if self.at_degree_sign() {
                if let Some(unit) = self.celsius_after_degree_sign() {
                    return Ok(unit);
                }
            }
            return Err(self.error("After ▶ comes a unit, such as \\mathrm{in} or ^\\circ\\mathrm{C}.", position));
        }
        self.in_unit_context(|p| p.parse_expression())
    }

    /// The conversion arrow `\blacktriangleright` (`▶`), or `\triangleright`.
    fn at_conversion(&self) -> bool {
        self.current().kind == TokenType::Command && matches!(self.current().text.as_str(), "\\blacktriangleright" | "\\triangleright")
    }

    // ---- structure -------------------------------------------------------------------

    /// Whether the input opens with `\begin{cases}`.
    fn starts_cases(&self) -> bool {
        self.current().kind == TokenType::Command
            && self.current().text == "\\begin"
            && self.peek(1).kind == TokenType::LBrace
            && self.peek(2).text == "cases"
    }

    /// Whether a `;` outside all brackets separates equations: `x + y = 3; x - y = 1`.
    fn has_top_level_semicolon(&self) -> bool {
        let mut depth = 0usize;
        for token in &self.tokens[self.index..] {
            match token.kind {
                TokenType::LBrace | TokenType::LParen | TokenType::LBracket => depth += 1,
                TokenType::RBrace | TokenType::RParen | TokenType::RBracket => depth = depth.saturating_sub(1),
                TokenType::Comma if depth == 0 && token.text == ";" => return true,
                _ => {}
            }
        }
        false
    }

    /// `\begin{cases} a = b \\ c = d \end{cases}`, or the same equations separated by `;`. A
    /// trailing `\\` or `;` is allowed.
    fn parse_system(&mut self) -> Result<Node> {
        let position = self.current().position;
        let mut equations = Vec::new();
        if self.starts_cases() {
            self.advance();
            self.parse_environment_name("\\begin")?;
            while !self.is_end_command() {
                equations.push(self.parse_system_equation()?);
                match self.current().kind {
                    TokenType::RowBreak => {
                        self.advance();
                    }
                    _ if self.is_end_command() => {}
                    _ => return Err(self.error("Expected '\\\\' or '\\end' after the equation.", self.current().position)),
                }
            }
            self.advance(); // \end
            let closing = self.parse_environment_name("\\end")?;
            if closing != "cases" {
                return Err(self.error(&format!("\\begin{{cases}} is closed by \\end{{{closing}}}."), position));
            }
        } else {
            loop {
                equations.push(self.parse_system_equation()?);
                if !(self.current().kind == TokenType::Comma && self.current().text == ";") {
                    break;
                }
                self.advance();
                if self.current().kind == TokenType::End {
                    break;
                }
            }
        }
        if equations.is_empty() {
            return Err(self.error("The system has no equations.", position));
        }
        Ok(Node::System { equations, position })
    }

    /// One row of a system: `left = right`. Alignment `&`s around the `=` and an empty second
    /// column at the end of the row (as MathLive writes `cases`) are skipped.
    fn parse_system_equation(&mut self) -> Result<Node> {
        let left = self.parse_expression()?;
        self.skip_ampersand();
        if self.current().kind != TokenType::Equals {
            return Err(self.error("Every line of a system needs an '='.", self.current().position));
        }
        self.advance();
        self.skip_ampersand();
        let right = self.parse_expression()?;
        if self.current().kind == TokenType::Equals {
            return Err(self.error("Only one '=' is allowed per equation.", self.current().position));
        }
        if self.current().kind == TokenType::Ampersand
            && (self.peek(1).kind == TokenType::RowBreak || self.peek(1).text == "\\end")
        {
            self.advance();
        }
        let position = left.position();
        Ok(Node::Equation { left: Box::new(left), right: Box::new(right), position })
    }

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
        // `\pi = 3` compares the constant with 3: a constant names nothing to define (`e_1 = 5`
        // still does).
        let constant = CONSTANTS.contains(&name.text.as_str()) && (name.text != "i" || self.options.imaginary_i);
        if self.current().kind != TokenType::Equals || (subscript.is_none() && constant) {
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
            // The Geometry mode's `\times` is the cross product, which the types decide later.
            if self.options.geometry && (token.text == "\\times" || token.text == "×") {
                let position = left.position();
                self.advance();
                let right = self.parse_unary()?;
                left = Node::Call { name: "cross".into(), arguments: vec![left, right], position };
                continue;
            }
            let operator = match token.kind {
                TokenType::Times => Operator::Times,
                TokenType::Divide => Operator::Divide,
                TokenType::Command if MULTIPLICATION_COMMANDS.contains(&token.text.as_str()) => Operator::Times,
                TokenType::Command if token.text == "\\div" => Operator::Divide,
                TokenType::Command if token.text == "\\angle" => {
                    // `r\angle\theta`: a complex number in polar form.
                    let position = left.position();
                    self.advance();
                    // The angle runs to the next explicit operator: `2\angle\pi\,\mathrm{rad}`.
                    let mut angle = self.parse_unary()?;
                    while self.starts_atom() {
                        let right = self.parse_unary()?;
                        let at = angle.position();
                        angle = Node::binary(Operator::ImplicitTimes, angle, right, at);
                    }
                    left = Node::Call { name: "polar".into(), arguments: vec![left, angle], position };
                    continue;
                }
                TokenType::Command if token.text == "\\bmod" => {
                    let position = self.advance().position;
                    let right = self.parse_unary()?;
                    left = Node::Call { name: "mod".into(), arguments: vec![left, right], position };
                    continue;
                }
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
        let mut base = self.parse_atom()?;
        while self.current().kind == TokenType::Bang {
            let bang = self.advance();
            base = Node::Call { name: "factorial".into(), arguments: vec![base], position: bang.position };
        }
        if self.current().kind != TokenType::Caret {
            return Ok(base);
        }
        self.advance();
        let position = base.position();
        if self.at_degree_sign() {
            // `20\,^\circ\mathrm{C}` is twenty degrees Celsius, the way results are written,
            // not twenty angle degrees times a coulomb.
            if let Some(celsius) = self.celsius_after_degree_sign() {
                return Ok(Node::binary(Operator::ImplicitTimes, base, celsius, position));
            }
            return self.sexagesimal(base, position);
        }
        Ok(Node::Power { base: Box::new(base), exponent: Box::new(self.parse_unary()?), position })
    }

    /// The minutes and seconds that may follow a degree sign: `2^\circ 20' 30''`. Each is a
    /// number followed by one prime (minutes) or two (seconds), as `'`, `′`, `″` or
    /// `^{\prime}`; the result is one angle in degrees.
    fn sexagesimal(&mut self, degrees: Node, position: usize) -> Result<Node> {
        let mut minutes = None;
        let mut seconds = None;
        while self.current().kind == TokenType::Number {
            let Some((primes, consumed)) = self.primes_at(1) else { break };
            let slot = if primes == 1 && minutes.is_none() && seconds.is_none() {
                &mut minutes
            } else if primes == 2 && seconds.is_none() {
                &mut seconds
            } else {
                break;
            };
            let number = self.advance();
            let value = Decimal::parse(&number.text)
                .ok_or_else(|| self.error(&format!("'{}' is not a number.", number.text), number.position))?;
            *slot = Some(value);
            self.index += consumed;
        }
        if minutes.is_none() && seconds.is_none() {
            return Ok(Node::Degrees { operand: Box::new(degrees), position, sexagesimal: None });
        }
        let Node::Number { value: whole, .. } = &degrees else {
            return Err(self.error("Degrees, minutes and seconds must be plain numbers.", position));
        };
        let mut written = format!("{}^{{\\circ}}", super::numbers::format(whole));
        let mut operand = degrees.clone();
        for (part, per_degree, mark) in [(&minutes, 60, "'"), (&seconds, 3600, "''")] {
            if let Some(value) = part {
                written.push_str(&format!("{}{mark}", super::numbers::format(value)));
                let number = Node::Number { value: value.clone(), position };
                let share = Node::fraction(number, Node::Number { value: Decimal::from_i64(per_degree), position }, position);
                operand = Node::binary(Operator::Plus, operand, share, position);
            }
        }
        let operand = Node::Group { inner: Box::new(operand), position };
        Ok(Node::Degrees { operand: Box::new(operand), position, sexagesimal: Some(written) })
    }

    /// The primes starting `offset` tokens ahead: how many (1 or 2) and how many tokens they take.
    fn primes_at(&self, offset: usize) -> Option<(usize, usize)> {
        let is = |o: usize, kind: TokenType| self.peek(o).kind == kind;
        let is_prime = |o: usize| self.peek(o).kind == TokenType::Command && self.peek(o).text == "\\prime";
        if is(offset, TokenType::Prime) {
            return Some(if is(offset + 1, TokenType::Prime) { (2, 2) } else { (1, 1) });
        }
        if !is(offset, TokenType::Caret) {
            return None;
        }
        if is_prime(offset + 1) {
            return Some(if is_prime(offset + 2) { (2, 3) } else { (1, 2) });
        }
        if is(offset + 1, TokenType::LBrace) && is_prime(offset + 2) {
            if is(offset + 3, TokenType::RBrace) {
                return Some((1, 4));
            }
            if is_prime(offset + 3) && is(offset + 4, TokenType::RBrace) {
                return Some((2, 5));
            }
        }
        None
    }

    /// A `C` or `F` right after a degree sign, bare or as `\mathrm{C}` / `\text{C}`: degrees
    /// Celsius or Fahrenheit.
    fn celsius_after_degree_sign(&mut self) -> Option<Node> {
        let is_c = |t: &Token| t.kind == TokenType::Identifier && matches!(t.text.as_str(), "C" | "F");
        let position = self.current().position;
        let letter = |t: &Token| if t.text == "F" { "°F" } else { "°C" };
        let symbol = if is_c(self.current()) { letter(self.current()) } else { letter(self.peek(2)) };
        let consumed = if is_c(self.current()) {
            1
        } else if self.current().kind == TokenType::Command
            && matches!(self.current().text.as_str(), "\\mathrm" | "\\text")
            && self.peek(1).kind == TokenType::LBrace
            && is_c(self.peek(2))
            && self.peek(3).kind == TokenType::RBrace
        {
            4
        } else {
            return None;
        };
        let unit = units::find(symbol)?;
        self.index += consumed;
        Some(Node::Unit { unit, symbol: symbol.to_string(), position })
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
                if token.text.contains('.') && self.at_period() {
                    return self.recurring(&token);
                }
                Ok(Node::Number { value: self.decimal(&token.text, token.position)?, position: token.position })
            }
            TokenType::Identifier => self.parse_identifier(),
            TokenType::Pipe if self.in_point && token.text == "|" => Err(self.error(
                "In a point, '|' separates the coordinates; write an absolute value there as \\left|…\\right|.",
                token.position,
            )),
            TokenType::Pipe => self.parse_abs(),
            TokenType::LParen if self.options.geometry && self.point_ahead(self.index) => self.parse_point(None),
            TokenType::LParen => self.parse_bracket_or_prime_derivative(),
            TokenType::LBrace => self.parse_group(TokenType::RBrace, "}"),
            TokenType::LBracket if self.evaluated_integral_ahead() => self.parse_evaluated_integral(),
            TokenType::LBracket => self.parse_group(TokenType::RBracket, "]"),
            TokenType::Command => self.parse_command(&token),
            _ => Err(self.error("Expected a number, a unit or a variable here.", token.position)),
        }
    }

    /// `\overline{digits}` right after a decimal's digits: its period.
    fn at_period(&self) -> bool {
        self.current().kind == TokenType::Command
            && self.current().text == "\\overline"
            && self.peek(1).kind == TokenType::LBrace
            && self.peek(2).kind == TokenType::Number
            && !self.peek(2).text.contains('.')
            && self.peek(3).kind == TokenType::RBrace
    }

    /// A recurring decimal, `0{,}1\overline{6}`, as the exact fraction it stands for (`\frac{1}{6}`).
    fn recurring(&mut self, number: &Token) -> Result<Node> {
        let period = self.peek(2).text.clone();
        self.index += 4;
        let (whole, fixed) = number.text.split_once('.').unwrap_or((&number.text, ""));
        let ten = |n: usize| dashu::integer::IBig::from(10).pow(n);
        let digits = |text: &str| text.parse::<dashu::integer::IBig>().unwrap_or_default();
        // x = whole + (fixed·period − fixed) / (10^|fixed| · (10^|period| − 1))
        let denominator = ten(fixed.len()) * (ten(period.len()) - dashu::integer::IBig::ONE);
        let numerator = digits(whole) * &denominator + digits(&format!("{fixed}{period}")) - digits(fixed);
        let value = dashu::rational::RBig::from_parts(numerator, dashu::base::UnsignedAbs::unsigned_abs(denominator));
        let position = number.position;
        let integer = |n: dashu::integer::IBig| Node::Number { value: Decimal::new(n, 0), position };
        let (top, bottom) = value.into_parts();
        let fraction = Node::fraction(integer(top), integer(bottom.into()), position);
        Ok(Node::Group { inner: Box::new(fraction), position })
    }

    /// The arguments of a call in brackets, separated by `;` (or `,` with a space after it).
    fn call_arguments(&mut self, written: &str) -> Result<Vec<Node>> {
        let closing = match self.current().kind {
            TokenType::LParen => TokenType::RParen,
            TokenType::LBrace => TokenType::RBrace,
            TokenType::LBracket => TokenType::RBracket,
            _ => return Err(self.error(&format!("'{written}' needs its arguments in brackets: {written}(a; b)."), self.current().position)),
        };
        self.advance();
        let mut arguments = vec![self.parse_expression()?];
        while self.current().kind == TokenType::Comma {
            self.advance();
            arguments.push(self.parse_expression()?);
        }
        self.expect(closing, "a closing bracket")?;
        Ok(arguments)
    }

    /// `Ans` or `PreAns` in `\mathrm{}`, `\operatorname{}` or `\text{}`.
    fn answer(&self) -> Option<&'static str> {
        if self.peek(1).kind != TokenType::LBrace || self.peek(2).kind != TokenType::Identifier || self.peek(3).kind != TokenType::RBrace {
            return None;
        }
        match self.peek(2).text.as_str() {
            "Ans" => Some("Ans"),
            "PreAns" => Some("PreAns"),
            _ => None,
        }
    }

    /// `\operatorname{arsinh}` and the like: the function's name, from [`OPERATOR_FUNCTIONS`].
    fn operator_function(&self) -> Option<&'static str> {
        if self.peek(1).kind != TokenType::LBrace || self.peek(2).kind != TokenType::Identifier || self.peek(3).kind != TokenType::RBrace {
            return None;
        }
        let written = self.peek(2).text.as_str();
        OPERATOR_FUNCTIONS.iter().find(|(spelling, _)| *spelling == written).map(|(_, name)| *name)
    }

    /// `\operatorname{name}` (or `\mathrm{name}`) where `name` is one of [`NAMED_CALLS`].
    fn named_call(&self) -> Option<&'static str> {
        if self.peek(1).kind != TokenType::LBrace || self.peek(2).kind != TokenType::Identifier || self.peek(3).kind != TokenType::RBrace {
            return None;
        }
        let name = self.peek(2).text.as_str();
        NAMED_CALLS.iter().find(|(spelling, _)| *spelling == name).map(|(_, call)| *call)
    }

    fn parse_abs(&mut self) -> Result<Node> {
        let open = self.advance();
        self.abs_depth += 1;
        let in_point = std::mem::replace(&mut self.in_point, false);
        let operand = self.parse_expression();
        self.in_point = in_point;
        self.abs_depth -= 1;
        let operand = operand?;
        if self.current().kind != TokenType::Pipe {
            return Err(self.error("The absolute value is missing its closing '|'.", open.position));
        }
        self.advance();
        Ok(Node::Abs { operand: Box::new(operand), position: open.position })
    }

    /// `(…)`, or with primes after it the derivative of the bracket: `\left.(x^3 - x)'\right|_{x=2}`
    /// once, `(…)''` twice. The variable is the one the point names, or else the only letter
    /// in the bracket.
    fn parse_bracket_or_prime_derivative(&mut self) -> Result<Node> {
        let Some(close) = self.matching_paren(self.index) else {
            return self.parse_group(TokenType::RParen, ")");
        };
        let Some((order, consumed)) = self.primes_at(close + 1 - self.index) else {
            return self.parse_group(TokenType::RParen, ")");
        };
        let position = self.current().position;
        let variable = self.prime_variable(close, close + 1 + consumed)?;
        let body = self.with_bound(&variable, |p| p.parse_group(TokenType::RParen, ")"))?;
        self.index += consumed;
        let point = match self.parse_derivative_point()? {
            Some((bindings, at)) => Some(self.split_point(bindings, &variable, at)?),
            None => None,
        };
        Ok(nested_derivative(&vec![variable; order], body, point, false, position))
    }

    /// The token index of the `)` that closes the `(` at `open`.
    fn matching_paren(&self, open: usize) -> Option<usize> {
        let mut depth = 0usize;
        for (i, token) in self.tokens.iter().enumerate().skip(open) {
            match token.kind {
                TokenType::LParen => depth += 1,
                TokenType::RParen => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
                TokenType::End => return None,
                _ => {}
            }
        }
        None
    }

    /// What a prime after the bracket closing at `close` differentiates by: the letter of the
    /// point `|_{x=…}` at `after`, or else the only single letter in the bracket other than `e`.
    fn prime_variable(&self, close: usize, after: usize) -> Result<String> {
        let token = |i: usize| &self.tokens[i.min(self.tokens.len() - 1)];
        if token(after).kind == TokenType::Pipe
            && token(after + 1).kind == TokenType::Underscore
            && token(after + 2).kind == TokenType::LBrace
            && token(after + 3).kind == TokenType::Identifier
            && is_single_letter(&token(after + 3).text)
        {
            return Ok(token(after + 3).text.clone());
        }
        let mut letters: Vec<&str> = self.tokens[self.index + 1..close]
            .iter()
            .filter(|t| t.kind == TokenType::Identifier && t.text.chars().count() == 1 && t.text != "e")
            .map(|t| t.text.as_str())
            .collect();
        letters.sort_unstable();
        letters.dedup();
        match letters.as_slice() {
            [letter] => Ok((*letter).to_string()),
            _ => Err(self.error(
                "Say which variable the prime differentiates by with a point, as in \\left.(x^2)'\\right|_{x=1}.",
                token(close + 1).position,
            )),
        }
    }

    fn parse_group(&mut self, closing: TokenType, closing_text: &str) -> Result<Node> {
        let open = self.advance();
        let outer_abs_depth = std::mem::replace(&mut self.abs_depth, 0);
        let in_point = std::mem::replace(&mut self.in_point, false);
        let inner = self.parse_expression();
        self.abs_depth = outer_abs_depth;
        self.in_point = in_point;
        let inner = inner?;
        self.expect(closing, &format!("'{closing_text}'"))?;
        Ok(Node::Group { inner: Box::new(inner), position: open.position })
    }

    /// Whether the `(` at `index` opens a point (Geometry mode): a plain `|` or a `;` stands
    /// between it and its `)` outside any inner bracket or `\left|…\right|`. A `|` right after
    /// `(` or after a separator therefore starts no absolute value.
    fn point_ahead(&self, index: usize) -> bool {
        let mut depth = 0usize;
        for token in &self.tokens[index + 1..] {
            match token.kind {
                TokenType::LParen | TokenType::LBrace | TokenType::LBracket => depth += 1,
                TokenType::RParen | TokenType::RBrace | TokenType::RBracket if depth == 0 => return false,
                TokenType::RParen | TokenType::RBrace | TokenType::RBracket => depth -= 1,
                TokenType::Pipe if token.text == "\\left|" => depth += 1,
                TokenType::Pipe if token.text == "\\right|" => depth = depth.saturating_sub(1),
                TokenType::Pipe | TokenType::Comma if depth == 0 && token.text != "," => return true,
                TokenType::End => return false,
                _ => {}
            }
        }
        false
    }

    /// `(1|2|3)`, `(1;2)` or, with `name`, `P(1|2|3)`: a point of the Geometry mode, as the call
    /// `point` of its coordinates, or `named point` of the name and the coordinates.
    fn parse_point(&mut self, name: Option<Node>) -> Result<Node> {
        let open = self.advance();
        let outer = (std::mem::replace(&mut self.abs_depth, 1), std::mem::replace(&mut self.in_point, true));
        let coordinates = self.point_coordinates(open.position);
        (self.abs_depth, self.in_point) = outer;
        let coordinates = coordinates?;
        let position = name.as_ref().map_or(open.position, Node::position);
        Ok(match name {
            Some(name) => Node::Call { name: "named point".into(), arguments: [vec![name], coordinates].concat(), position },
            None => Node::Call { name: "point".into(), arguments: coordinates, position },
        })
    }

    fn point_coordinates(&mut self, open: usize) -> Result<Vec<Node>> {
        let mut coordinates = vec![self.parse_expression()?];
        let mut separator: Option<String> = None;
        loop {
            let token = self.current().clone();
            match token.kind {
                TokenType::RParen => {
                    self.advance();
                    break;
                }
                TokenType::Pipe | TokenType::Comma if token.text == "|" || token.text == ";" => {
                    if separator.get_or_insert_with(|| token.text.clone()) != &token.text {
                        return Err(self.error("A point takes one separator between its coordinates: '|' or ';'.", token.position));
                    }
                    self.advance();
                    coordinates.push(self.parse_expression()?);
                }
                _ => return Err(self.error("Expected '|', ';' or ')' in the point.", token.position)),
            }
        }
        if !(2..=3).contains(&coordinates.len()) {
            return Err(self.error("A point has two or three coordinates.", open));
        }
        Ok(coordinates)
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
            name if FUNCTION_COMMANDS.contains(&name) => self.parse_function(token),
            // Only a limit's target or an integral's bound can be infinite; elsewhere it is an
            // error when evaluated.
            "\\infty" => {
                self.advance();
                Ok(Node::Constant { name: "\\infty".into(), position })
            }
            "\\lfloor" | "\\lceil" => {
                let (name, closing) = if token.text == "\\lfloor" { ("floor", "\\rfloor") } else { ("ceil", "\\rceil") };
                self.advance();
                let inner = self.parse_expression()?;
                if !(self.current().kind == TokenType::Command && self.current().text == closing) {
                    return Err(self.error(&format!("'{}' is missing its '{closing}'.", token.text), self.current().position));
                }
                self.advance();
                Ok(Node::Call { name: name.into(), arguments: vec![inner], position })
            }
            "\\max" | "\\min" | "\\gcd" => {
                self.advance();
                let arguments = self.call_arguments(&token.text)?;
                Ok(Node::Call { name: token.text.trim_start_matches('\\').to_string(), arguments, position })
            }
            "\\operatorname" | "\\mathrm" | "\\text" if self.answer().is_some() => {
                // `\mathrm{Ans}`, `\mathrm{PreAns}`: the last results of the session.
                let name = self.answer().expect("checked").to_string();
                self.index += 4;
                Ok(Node::Variable { name, index: String::new(), position })
            }
            "\\operatorname" | "\\mathrm" if self.operator_function().is_some() => {
                let name = self.operator_function().expect("checked");
                // Stand on the closing brace, which `parse_function` steps over like a command.
                self.index += 3;
                self.parse_function(&Token { kind: TokenType::Command, text: format!("\\{name}"), position })
            }
            "\\operatorname" | "\\mathrm" if self.named_call().is_some() => {
                let name = self.named_call().expect("checked").to_string();
                self.index += 4;
                let arguments = self.call_arguments(&format!("\\operatorname{{{name}}}"))?;
                Ok(Node::Call { name, arguments, position })
            }
            "\\begin" => self.parse_matrix(token),
            "\\vec" if self.options.geometry => self.parse_vector_name(token),
            "\\det" => {
                self.advance();
                let operand = self.parse_atom()?;
                Ok(Node::Determinant { operand: Box::new(operand), position })
            }
            "\\sum" | "\\prod" => self.parse_sum(token),
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
            "\\oint" | "\\oiint" | "\\oiiint" => Err(self.error(
                &format!("Closed-path integrals ({}) are not supported; an integral over an interval is \\int_a^b.", token.text),
                position,
            )),
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
        let mut name = token.text.trim_start_matches('\\').to_string();
        self.advance();
        // `\log_b x` (also `\log_{b}`) is ln x / ln b.
        let base = if self.current().kind == TokenType::Underscore {
            if name != "log" {
                return Err(self.error(&format!("A base on {} is not supported.", token.text), self.current().position));
            }
            self.advance();
            Some(self.parse_argument(&token.text)?)
        } else {
            None
        };
        let exponent = if self.current().kind == TokenType::Caret {
            let caret = self.advance();
            let exponent = self.parse_argument(&token.text)?;
            let negative = matches!(&exponent, Node::Negation { operand, .. } if matches!(**operand, Node::Number { .. }))
                || matches!(&exponent, Node::Group { inner, .. } if matches!(**inner, Node::Negation { .. }));
            if negative {
                // \sin^{-1} is the inverse function, arcsin; any other negative power is ambiguous.
                let inverse = matches!(name.as_str(), "sin" | "cos" | "tan" | "sinh" | "cosh" | "tanh") && is_minus_one(&exponent);
                if !inverse {
                    return Err(self.error(
                        &format!("{}^{{-n}} is ambiguous; write 1/{} x or \\arc… instead.", token.text, token.text),
                        caret.position,
                    ));
                }
                // \sinh^{-1} is the area function arsinh.
                name = if name.ends_with('h') { format!("ar{name}") } else { format!("arc{name}") };
                None
            } else {
                Some(exponent)
            }
        } else {
            None
        };

        let argument = if matches!(self.current().kind, TokenType::LParen | TokenType::LBrace) {
            self.parse_atom()?
        } else {
            self.parse_function_operand(&token.text)?
        };
        let function = match base {
            None => Node::Function { name, argument: Box::new(argument), position },
            Some(base) => Node::fraction(
                Node::Function { name: "ln".into(), argument: Box::new(argument), position },
                Node::Function { name: "ln".into(), argument: Box::new(base), position },
                position,
            ),
        };
        Ok(match exponent {
            Some(exponent) => Node::Power { base: Box::new(function), exponent: Box::new(exponent), position },
            None => function,
        })
    }

    fn parse_function_operand(&mut self, command: &str) -> Result<Node> {
        // A signed argument, `\sin -30^\circ`: the sign belongs to the whole operand.
        if matches!(self.current().kind, TokenType::Minus | TokenType::Plus) {
            let sign = self.advance();
            let operand = self.parse_function_operand(command)?;
            return Ok(if sign.kind == TokenType::Minus {
                Node::Negation { operand: Box::new(operand), position: sign.position }
            } else {
                operand
            });
        }
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

    /// `\sum_{i=1}^{n} body`, and `\prod` alike. The body is a whole term, so
    /// `\sum_{i=1}^{3} i \cdot 2` sums the product and a following `+` ends it.
    fn parse_sum(&mut self, token: &Token) -> Result<Node> {
        let command = token.text.as_str();
        self.advance();
        self.expect(TokenType::Underscore, &format!("'_' with the index, as in {command}_{{i=1}}^{{n}}"))?;
        self.expect(TokenType::LBrace, "'{'")?;
        let index = self.parse_bound_name(command)?;
        self.expect(TokenType::Equals, "'=' after the index")?;
        let from = self.parse_expression()?;
        self.expect(TokenType::RBrace, "'}'")?;
        self.expect(TokenType::Caret, &format!("'^' with the upper limit, as in {command}_{{i=1}}^{{n}}"))?;
        let to = self.parse_argument(&token.text)?;
        let body = self.with_bound(&index, |p| p.parse_term())?;
        Ok(Node::Sum {
            index,
            from: Box::new(from),
            to: Box::new(to),
            body: Box::new(body),
            position: token.position,
            product: command == "\\prod",
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
        let side = self.take_limit_side();
        let target = self.parse_expression()?;
        self.expect(TokenType::RBrace, "'}'")?;
        let body = self.with_bound(&variable, |p| p.parse_term())?;
        Ok(Node::Limit { variable, target: Box::new(target), body: Box::new(body), side, position: token.position })
    }

    /// A one-sided target ends in `^+`, `^-`, `^{+}` or `^{-}` before the `}`. The suffix is
    /// removed from the tokens, so the target itself reads as an ordinary expression.
    fn take_limit_side(&mut self) -> i8 {
        let mut depth = 0;
        let mut close = None;
        for (i, t) in self.tokens.iter().enumerate().skip(self.index) {
            match t.kind {
                TokenType::LBrace => depth += 1,
                TokenType::RBrace if depth == 0 => {
                    close = Some(i);
                    break;
                }
                TokenType::RBrace => depth -= 1,
                TokenType::End => break,
                _ => {}
            }
        }
        let Some(close) = close else { return 0 };
        let kinds: Vec<TokenType> = self.tokens[self.index..close].iter().map(|t| t.kind).collect();
        let (sign, length) = match kinds.as_slice() {
            [.., TokenType::Caret, s @ (TokenType::Plus | TokenType::Minus)] => (*s, 2),
            [.., TokenType::Caret, TokenType::LBrace, s @ (TokenType::Plus | TokenType::Minus), TokenType::RBrace] => (*s, 4),
            _ => return 0,
        };
        if kinds.len() == length {
            return 0;
        }
        self.tokens.drain(close - length..close);
        if sign == TokenType::Plus { 1 } else { -1 }
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
        let (variable, body) = self.parse_integrand(token.position)?;
        Ok(Node::Integral {
            variable,
            lower: Box::new(lower),
            upper: Box::new(upper),
            body: Box::new(body),
            position: token.position,
        })
    }

    /// The body of an integral up to and including its differential: the variable and the body.
    fn parse_integrand(&mut self, position: usize) -> Result<(String, Node)> {
        let (variable, stop, length) = self.find_differential(position)?;
        self.stops.push(stop);
        let body = self.with_bound(&variable, |p| p.parse_expression());
        self.stops.pop();
        let body = body?;
        if self.index != stop {
            return Err(self.error(&format!("Expected 'd{variable}' here."), self.current().position));
        }
        self.index += length;
        Ok((variable, body))
    }

    /// Whether the `[` here opens an antiderivative evaluated between limits, `[\int f\,dx]_a^b`.
    fn evaluated_integral_ahead(&self) -> bool {
        let int = self.peek(1);
        int.kind == TokenType::Command && int.text == "\\int" && self.peek(2).kind != TokenType::Underscore
    }

    /// `\left[\int f\,dx\right]_a^b`: the antiderivative taken from a to b, the integral of f
    /// from a to b.
    fn parse_evaluated_integral(&mut self) -> Result<Node> {
        self.advance();
        let int = self.advance();
        let outer_abs_depth = std::mem::replace(&mut self.abs_depth, 0);
        let integrand = self.parse_integrand(int.position);
        self.abs_depth = outer_abs_depth;
        let (variable, body) = integrand?;
        self.expect(TokenType::RBracket, "']' after the differential")?;
        if self.current().kind != TokenType::Underscore {
            return Err(self.error("The bracket needs the limits to evaluate at, as in \\left[\\int x\\,dx\\right]_{0}^{1}.", self.current().position));
        }
        self.advance();
        let lower = self.parse_argument(&int.text)?;
        self.expect(TokenType::Caret, "'^' with the upper limit, as in \\right]_{0}^{1}")?;
        let upper = self.parse_argument(&int.text)?;
        Ok(Node::Integral {
            variable,
            lower: Box::new(lower),
            upper: Box::new(upper),
            body: Box::new(body),
            position: int.position,
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
            "cases" => {
                return Err(self.error("A system of equations must be the whole input.", token.position));
            }
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

    /// `\vec{a}`, `\vec a`, `\vec{a_1}` and `\vec{a}_1`: a named vector of the Geometry mode,
    /// as the call `vec` of the variable.
    fn parse_vector_name(&mut self, token: &Token) -> Result<Node> {
        self.advance();
        let braced = self.current().kind == TokenType::LBrace;
        if braced {
            self.advance();
        }
        let letter = self.current().clone();
        if letter.kind != TokenType::Identifier || letter.text.chars().count() != 1 {
            return Err(self.error("A vector is named by one letter, as in \\vec{a} or \\vec{a}_1.", letter.position));
        }
        self.advance();
        let mut index = String::new();
        if self.current().kind == TokenType::Underscore {
            self.advance();
            index = self.parse_subscript()?;
        }
        if braced {
            self.expect(TokenType::RBrace, "'}' after the name of the vector")?;
            if index.is_empty() && self.current().kind == TokenType::Underscore {
                self.advance();
                index = self.parse_subscript()?;
            }
        }
        let name = Node::Variable { name: letter.text, index, position: letter.position };
        Ok(Node::Call { name: "vec".into(), arguments: vec![name], position: token.position })
    }

    fn skip_ampersand(&mut self) {
        if self.current().kind == TokenType::Ampersand {
            self.advance();
        }
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

    /// `{d}{dx}` right after a `\frac`. Also `{d^2}{dx^2}` for a higher order, `\mathrm{d}` for
    /// `d`, and `\partial` for both `d`s, with one variable or several: `{\partial^2}{\partial x
    /// \partial z}`, `{\partial^3}{\partial x^2 \partial z}`.
    fn derivative_header(&self) -> Option<DerivativeHeader> {
        let kind = |i: usize| self.tokens[i.min(self.tokens.len() - 1)].kind;
        let text = |i: usize| self.tokens[i.min(self.tokens.len() - 1)].text.as_str();
        let mut i = self.index + 1;
        if kind(i) != TokenType::LBrace {
            return None;
        }
        i += 1;
        let partial = kind(i) == TokenType::Command && text(i) == "\\partial";
        if partial || (kind(i) == TokenType::Identifier && text(i) == "d") {
            i += 1;
        } else if kind(i) == TokenType::Command
            && text(i) == "\\mathrm"
            && kind(i + 1) == TokenType::LBrace
            && text(i + 2) == "d"
            && kind(i + 3) == TokenType::RBrace
        {
            i += 4;
        } else {
            return None;
        }
        let order = self.derivative_order(&mut i)?;
        if kind(i) != TokenType::RBrace || kind(i + 1) != TokenType::LBrace {
            return None;
        }
        i += 2;
        let mut variables = Vec::new();
        if partial {
            // \partial x \partial z …, each with its own power, adding up to the order.
            while kind(i) == TokenType::Command && text(i) == "\\partial" {
                if kind(i + 1) != TokenType::Identifier || !is_single_letter(text(i + 1)) {
                    return None;
                }
                let name = text(i + 1).to_string();
                i += 2;
                let count = self.derivative_order(&mut i)?;
                variables.extend(std::iter::repeat_n(name, count as usize));
            }
            if variables.len() != order as usize || kind(i) != TokenType::RBrace {
                return None;
            }
        } else {
            let (name, length) = self.differential_at(i)?;
            i += length;
            if self.derivative_order(&mut i)? != order || kind(i) != TokenType::RBrace {
                return None;
            }
            variables = vec![name; order as usize];
        }
        Some(DerivativeHeader { variables, partial, after: i + 1 })
    }

    /// The order after a `d` or `dx` at token `i`: `^2` or `^{2}`, moving `i` past it, or 1
    /// without a power. `None` for a power that is not a whole number from 1 to 9.
    fn derivative_order(&self, i: &mut usize) -> Option<u32> {
        let token = |i: usize| &self.tokens[i.min(self.tokens.len() - 1)];
        if token(*i).kind != TokenType::Caret {
            return Some(1);
        }
        let braced = token(*i + 1).kind == TokenType::LBrace;
        let number = token(*i + 1 + usize::from(braced));
        if number.kind != TokenType::Number || (braced && token(*i + 3).kind != TokenType::RBrace) {
            return None;
        }
        let order = number.text.parse::<u32>().ok().filter(|n| (1..=9).contains(n))?;
        *i += 2 + 2 * usize::from(braced);
        Some(order)
    }

    /// `\frac{d}{dx} body`, and `\left.\frac{d}{dx} body\right|_{x=a}` for the value at a point;
    /// `\frac{d^n}{dx^n}` is the derivative taken n times.
    ///
    /// The point may give several variables, `|_{x=2, z=3}`: the derivative's own one and the
    /// others it needs. Their letters are variables in the body, even one that is a unit (`t`).
    fn parse_derivative(&mut self, token: &Token) -> Result<Node> {
        let header = self.derivative_header().expect("checked by the caller");
        self.index = header.after;
        let mut bound = header.variables.clone();
        bound.extend(self.point_names_ahead());
        let body = self.with_bounds(&bound, |p| p.parse_term())?;
        let point = self.parse_derivative_point()?;
        // In `\frac{d}{dx}\frac{d}{dx} f |_{x=2}` the inner derivative has already taken the
        // point; it belongs to the outermost one.
        let (body, point) = match (body, point) {
            (Node::Derivative { variable, body, at: Some(at), given, partial, position }, None) => {
                let mut bindings = vec![(variable.clone(), *at)];
                bindings.extend(given);
                let inner = Node::Derivative { variable, body, at: None, given: Vec::new(), partial, position };
                (inner, Some((bindings, position)))
            }
            other => other,
        };
        let point = match point {
            Some((bindings, position)) => Some(self.split_point(bindings, &header.variables[0], position)?),
            None => None,
        };
        Ok(nested_derivative(&header.variables, body, point, header.partial, token.position))
    }

    /// The bindings of a point as the value of `variable` and the rest; it has to name `variable`.
    fn split_point(&self, bindings: Bindings, variable: &str, position: usize) -> Result<Point> {
        let mut at = None;
        let mut given = Vec::new();
        for (name, value) in bindings {
            if name == variable {
                at = Some(Box::new(value));
            } else {
                given.push((name, value));
            }
        }
        match at {
            Some(at) => Ok((at, given)),
            None => Err(self.error(&format!("The point has to be given for {variable}, as in _{{{variable}=2}}."), position)),
        }
    }

    /// The point `|_{x=a}` or `|_{x=a, z=b}` after a derivative's body, if there is one, with
    /// the position of its first name.
    fn parse_derivative_point(&mut self) -> Result<Option<(Bindings, usize)>> {
        if self.current().kind != TokenType::Pipe || self.peek(1).kind != TokenType::Underscore {
            return Ok(None);
        }
        self.index += 2;
        self.expect(TokenType::LBrace, "'{'")?;
        let position = self.current().position;
        let mut bindings: Bindings = Vec::new();
        loop {
            let name_position = self.current().position;
            let name = self.parse_bound_name("the evaluation point")?;
            if bindings.iter().any(|(n, _)| *n == name) {
                return Err(self.error(&format!("The point gives {name} twice."), name_position));
            }
            self.expect(TokenType::Equals, "'='")?;
            bindings.push((name, self.parse_expression()?));
            if self.current().kind != TokenType::Comma {
                break;
            }
            self.advance();
        }
        self.expect(TokenType::RBrace, "'}'")?;
        Ok(Some((bindings, position)))
    }

    /// The letters a point `|_{x=…, z=…}` further on names, so the body reads them as variables.
    /// Only a look ahead: the point itself is parsed after the body.
    fn point_names_ahead(&self) -> Vec<String> {
        let mut depth = 0_i32;
        let mut i = self.index;
        while i + 2 < self.tokens.len() {
            match self.tokens[i].kind {
                TokenType::LBrace | TokenType::LParen | TokenType::LBracket => depth += 1,
                TokenType::RBrace | TokenType::RParen | TokenType::RBracket => {
                    depth -= 1;
                    if depth < 0 {
                        return Vec::new();
                    }
                }
                TokenType::End => return Vec::new(),
                TokenType::Pipe
                    if depth == 0
                        && self.tokens[i + 1].kind == TokenType::Underscore
                        && self.tokens[i + 2].kind == TokenType::LBrace =>
                {
                    let mut names = Vec::new();
                    let mut j = i + 3;
                    let mut inner = 0_i32;
                    while j + 1 < self.tokens.len() && !(inner == 0 && self.tokens[j].kind == TokenType::RBrace) {
                        let token = &self.tokens[j];
                        match token.kind {
                            TokenType::LBrace | TokenType::LParen => inner += 1,
                            TokenType::RBrace | TokenType::RParen => inner -= 1,
                            TokenType::Identifier
                                if inner == 0
                                    && is_single_letter(&token.text)
                                    && self.tokens[j + 1].kind == TokenType::Equals =>
                            {
                                names.push(token.text.clone())
                            }
                            _ => {}
                        }
                        j += 1;
                    }
                    return names;
                }
                _ => {}
            }
            i += 1;
        }
        Vec::new()
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

    fn with_bounds<T>(&mut self, names: &[String], block: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        let before = self.bound.len();
        self.bound.extend(names.iter().cloned());
        let result = block(self);
        self.bound.truncate(before);
        result
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
        if self.options.geometry && !self.unit_context && token.text.chars().count() == 1 {
            if let Some(point) = self.named_point()? {
                return Ok(point);
            }
        }
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

        // `\Gamma(x)` with a bracket right after it is the Gamma function; `\Gamma` alone stays a
        // letter, as it is in physics.
        if !self.unit_context && token.text == "\\Gamma" && self.peek(1).kind == TokenType::LParen {
            self.advance();
            let argument = self.parse_group(TokenType::RParen, ")")?;
            return Ok(Node::Function { name: "Gamma".into(), argument: Box::new(argument), position: token.position });
        }

        // The whole run has to resolve as one symbol: `m s` is the millisecond, and a product
        // of units is only ever written with an explicit \cdot.
        let run = self.identifier_run();
        let symbol: String = run.iter().map(|t| greek_symbol(&t.text)).collect();

        // Disambiguation: outside explicit unit context, `a` and `y` (annum/year) and `t`, `u`,
        // `in` are free algebraic variables rather than units, as the settings say.
        let is_reclassified_variable = !self.unit_context && self.options.reads_as_variable(&symbol);

        if !is_reclassified_variable {
            if let Some(unit) = units::find(&symbol) {
                self.index += run.len();
                return Ok(Node::Unit { unit, symbol, position: token.position });
            }
        }

        // Where the run as a whole is no unit, its first symbol can still stand alone (`\pi x`,
        // `x y`) unless it is itself a unit: `m kg` stays an error, a product of units needs \cdot.
        let first = greek_symbol(&token.text);
        let first_is_unit =
            !(!self.unit_context && self.options.reads_as_variable(first)) && units::find(first).is_some();
        let is_constant = CONSTANTS.contains(&token.text.as_str()) && (token.text != "i" || self.options.imaginary_i);
        if !self.unit_context && !first_is_unit && is_constant {
            self.advance();
            return Ok(Node::Constant { name: token.text, position: token.position });
        }

        if !self.unit_context && !first_is_unit && is_single_letter(&token.text) {
            self.advance();
            return Ok(Node::Variable { name: token.text, index: String::new(), position: token.position });
        }

        if !self.unit_context && self.options.letter_products && is_variable_product(&token.text, &self.options) {
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

    /// `P(1|2|3)` or `P_1(1|2)`: a letter, perhaps with an index, right before a point.
    fn named_point(&mut self) -> Result<Option<Node>> {
        let start = self.index;
        let token = self.advance();
        let mut index = String::new();
        if self.current().kind == TokenType::Underscore {
            self.advance();
            match self.parse_subscript() {
                Ok(subscript) => index = subscript,
                Err(_) => {
                    self.index = start;
                    return Ok(None);
                }
            }
        }
        if self.current().kind != TokenType::LParen || !self.point_ahead(self.index) {
            self.index = start;
            return Ok(None);
        }
        let name = Node::Variable { name: token.text, index, position: token.position };
        self.parse_point(Some(name)).map(Some)
    }

    /// `ab` in `2ab` becomes the variable `a`, with `b` left behind as the next token to be read,
    /// so juxtaposition makes it `2 \cdot a \cdot b` and an exponent binds to `b` alone.
    fn take_first_letter(&mut self) -> Node {
        let token = self.current().clone();
        let (first, rest) = token.text.split_at(1);
        self.tokens[self.index] =
            Token { kind: TokenType::Identifier, text: rest.to_string(), position: token.position + 1 };
        // `iy` is `i \cdot y` where `i` is the imaginary unit.
        if first == "i" && self.options.imaginary_i {
            return Node::Constant { name: first.to_string(), position: token.position };
        }
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
            TokenType::Number | TokenType::Identifier | TokenType::LParen | TokenType::LBrace | TokenType::LBracket => true,
            // `|_{x=2}` after a derivative is its evaluation point, not an absolute value.
            TokenType::Pipe => self.abs_depth == 0 && self.peek(1).kind != TokenType::Underscore,
            TokenType::Command => {
                !MULTIPLICATION_COMMANDS.contains(&token.text.as_str())
                    && !matches!(
                        token.text.as_str(),
                        "\\div" | "\\end" | "\\bmod" | "\\rfloor" | "\\rceil" | "\\blacktriangleright" | "\\triangleright"
                    )
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
