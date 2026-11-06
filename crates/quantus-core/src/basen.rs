//! Basis-N: whole numbers in the bases 10, 16, 2 and 8, as 32-bit two's complement, with the
//! bitwise operations of a Casio fx-991DE X.
//!
//! The input is LaTeX, but its own little language: `1F` is a hexadecimal number, not a product,
//! and `and` is an operator. So it has its own tokenizer and parser instead of the main ones.
//!
//! - Numbers are read in the set base; a prefix `d`, `h`, `b` or `o` (best as `\mathrm{h}`, or
//!   written straight before the digits where it cannot be a digit itself) reads one number in
//!   another base.
//! - `+ - \cdot \times \div \frac{}{}`; division truncates toward zero.
//! - `and`, `or`, `xor`, `xnor` (as `\operatorname{and}` or plain), `Not(x)`, `Neg(x)`.
//! - Precedence, highest first: sign, `Not`, `Neg`; `\cdot` `\div`; `+` `-`; `and`; `or` `xor` `xnor`.
//!
//! Every value and every intermediate result must fit 32 bits: from −2147483648 to 2147483647
//! in decimal, any 32-bit pattern in the other bases (`FFFFFFFF` is −1).

use serde::{Deserialize, Serialize};

use crate::forms::{FormKind, ResultForm};
use crate::{QuantusError, Result};
use crate::text::msg;

/// The base numbers are read and written in.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NumberBase {
    #[default]
    Dec,
    Hex,
    Bin,
    Oct,
}

impl NumberBase {
    fn radix(self) -> u32 {
        match self {
            NumberBase::Dec => 10,
            NumberBase::Hex => 16,
            NumberBase::Bin => 2,
            NumberBase::Oct => 8,
        }
    }

    fn of_prefix(prefix: char) -> Option<NumberBase> {
        Some(match prefix {
            'd' => NumberBase::Dec,
            'h' => NumberBase::Hex,
            'b' => NumberBase::Bin,
            'o' => NumberBase::Oct,
            _ => return None,
        })
    }

    fn form(self) -> FormKind {
        match self {
            NumberBase::Dec => FormKind::Dec,
            NumberBase::Hex => FormKind::Hex,
            NumberBase::Bin => FormKind::Bin,
            NumberBase::Oct => FormKind::Oct,
        }
    }

    const ALL: [NumberBase; 4] = [NumberBase::Dec, NumberBase::Hex, NumberBase::Bin, NumberBase::Oct];
}

/// A Basis-N result: the value and how it is written in the set base and in the others.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BaseNResult {
    pub value: i32,
    pub latex: String,
    pub forms: Vec<ResultForm>,
}

pub fn evaluate(latex: &str, base: NumberBase) -> Result<BaseNResult> {
    let tokens = tokenize(latex)?;
    if tokens.is_empty() {
        return Err(QuantusError::other(msg!("core.basen.type_number")));
    }
    let mut parser = Parser { tokens, index: 0, base };
    let value = fit(parser.or()?)? as i32;
    if parser.index < parser.tokens.len() {
        return Err(QuantusError::other(msg!("core.basen.unexpected", parser.tokens[parser.index].text())));
    }
    let forms = NumberBase::ALL
        .into_iter()
        .filter(|b| *b != base)
        .map(|b| ResultForm::new(b.form(), write(value, b)))
        .collect();
    Ok(BaseNResult { value, latex: write(value, base), forms })
}

/// `value` in `base`: signed in decimal, the 32-bit pattern without leading zeros in the others.
pub fn write(value: i32, base: NumberBase) -> String {
    let bits = value as u32;
    let digits = match base {
        NumberBase::Dec => return value.to_string(),
        NumberBase::Hex => format!("{bits:X}"),
        NumberBase::Bin => format!("{bits:b}"),
        NumberBase::Oct => format!("{bits:o}"),
    };
    if digits.chars().any(|c| c.is_ascii_alphabetic()) { format!("\\mathrm{{{digits}}}") } else { digits }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Token {
    /// A run of letters and digits: a number, a prefix, or a word such as `and`.
    Word(String),
    Plus,
    Minus,
    Times,
    Divide,
    Open,
    Close,
}

impl Token {
    fn text(&self) -> String {
        match self {
            Token::Word(word) => word.clone(),
            Token::Plus => "+".into(),
            Token::Minus => "-".into(),
            Token::Times => "\\cdot".into(),
            Token::Divide => "\\div".into(),
            Token::Open => "(".into(),
            Token::Close => ")".into(),
        }
    }
}

/// Commands whose braced argument is one word: `\mathrm{h}`, `\operatorname{and}`.
const WORD_COMMANDS: [&str; 5] = ["mathrm", "operatorname", "text", "mathit", "textrm"];
/// Commands that are spacing or sizing only.
const IGNORED_COMMANDS: [&str; 8] = ["left", "right", "quad", "qquad", "displaystyle", "big", "Big", "mathbf"];

fn tokenize(latex: &str) -> Result<Vec<Token>> {
    let chars: Vec<char> = latex.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    let unexpected = |c: char| QuantusError::other(msg!("core.basen.unknown_character"));
    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' | '\n' => i += 1,
            '+' => (tokens.push(Token::Plus), i += 1).1,
            '-' | '\u{2212}' => (tokens.push(Token::Minus), i += 1).1,
            '*' | '\u{b7}' | '\u{d7}' => (tokens.push(Token::Times), i += 1).1,
            '/' | '\u{f7}' => (tokens.push(Token::Divide), i += 1).1,
            '(' | '[' | '{' => (tokens.push(Token::Open), i += 1).1,
            ')' | ']' | '}' => (tokens.push(Token::Close), i += 1).1,
            c if c.is_ascii_alphanumeric() => {
                let start = i;
                while i < chars.len() && chars[i].is_ascii_alphanumeric() {
                    i += 1;
                }
                tokens.push(Token::Word(chars[start..i].iter().collect()));
            }
            '\\' => {
                i += 1;
                let start = i;
                while i < chars.len() && chars[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let name: String = chars[start..i].iter().collect();
                if name.is_empty() {
                    // `\,`, `\ ` and the like: spacing.
                    i += 1;
                    continue;
                }
                match name.as_str() {
                    "cdot" | "times" | "ast" => tokens.push(Token::Times),
                    "div" => tokens.push(Token::Divide),
                    "frac" => {
                        // \frac{a}{b}: (a) ÷ (b); the braces become the brackets.
                        let numerator = braced(&chars, &mut i)?;
                        let denominator = braced(&chars, &mut i)?;
                        tokens.push(Token::Open);
                        tokens.extend(tokenize(&numerator)?);
                        tokens.extend([Token::Close, Token::Divide, Token::Open]);
                        tokens.extend(tokenize(&denominator)?);
                        tokens.push(Token::Close);
                    }
                    name if WORD_COMMANDS.contains(&name) => {
                        let word = braced(&chars, &mut i)?;
                        tokens.push(Token::Word(word.chars().filter(|c| !c.is_whitespace()).collect()));
                    }
                    name if IGNORED_COMMANDS.contains(&name) => {}
                    "land" | "wedge" => tokens.push(Token::Word("and".into())),
                    "lor" | "vee" => tokens.push(Token::Word("or".into())),
                    "oplus" => tokens.push(Token::Word("xor".into())),
                    "lnot" | "neg" => tokens.push(Token::Word("Not".into())),
                    other => return Err(QuantusError::other(msg!("core.basen.unknown_command"))),
                }
            }
            other => return Err(unexpected(other)),
        }
    }
    Ok(tokens)
}

/// The content of the `{…}` at `*i`, which moves past it.
fn braced(chars: &[char], i: &mut usize) -> Result<String> {
    while *i < chars.len() && chars[*i] == ' ' {
        *i += 1;
    }
    if chars.get(*i) != Some(&'{') {
        return Err(QuantusError::other(msg!("core.basen.expected_brace")));
    }
    let start = *i + 1;
    let mut depth = 0;
    while *i < chars.len() {
        match chars[*i] {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    *i += 1;
                    return Ok(chars[start..*i - 1].iter().collect());
                }
            }
            _ => {}
        }
        *i += 1;
    }
    Err(QuantusError::other(msg!("core.basen.not_closed")))
}

struct Parser {
    tokens: Vec<Token>,
    index: usize,
    base: NumberBase,
}

fn overflow() -> QuantusError {
    QuantusError::other(msg!("core.basen.overflow_32_bits"))
}

/// Values are carried as `i64` so that `-2147483648` can be typed; each result must fit 32 bits.
fn fit(value: i64) -> Result<i64> {
    i32::try_from(value).map(i64::from).map_err(|_| overflow())
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

    fn word_is(&self, words: &[&str]) -> Option<String> {
        match self.peek() {
            Some(Token::Word(word)) if words.iter().any(|w| w.eq_ignore_ascii_case(word)) => Some(word.to_ascii_lowercase()),
            _ => None,
        }
    }

    /// `or`, `xor`, `xnor`: the lowest level.
    fn or(&mut self) -> Result<i64> {
        let mut left = self.and()?;
        while let Some(op) = self.word_is(&["or", "xor", "xnor"]) {
            self.index += 1;
            let right = self.and()?;
            left = match op.as_str() {
                "or" => left | right,
                "xor" => left ^ right,
                _ => !(left ^ right),
            };
        }
        Ok(left)
    }

    fn and(&mut self) -> Result<i64> {
        let mut left = self.sum()?;
        while self.word_is(&["and"]).is_some() {
            self.index += 1;
            left &= self.sum()?;
        }
        Ok(left)
    }

    fn sum(&mut self) -> Result<i64> {
        let mut left = self.product()?;
        loop {
            let plus = match self.peek() {
                Some(Token::Plus) => true,
                Some(Token::Minus) => false,
                _ => return Ok(left),
            };
            self.index += 1;
            let right = self.product()?;
            left = fit(if plus { left + right } else { left - right })?;
        }
    }

    fn product(&mut self) -> Result<i64> {
        let mut left = self.unary()?;
        loop {
            let times = match self.peek() {
                Some(Token::Times) => true,
                Some(Token::Divide) => false,
                // `2(3)`: a bracket right after a value multiplies.
                Some(Token::Open) => {
                    let right = self.unary()?;
                    left = fit(left * right)?;
                    continue;
                }
                _ => return Ok(left),
            };
            self.index += 1;
            let right = self.unary()?;
            left = if times {
                fit(left * right)?
            } else if right == 0 {
                return Err(QuantusError::other(msg!("core.common.division_zero")));
            } else {
                fit(left / right)?
            };
        }
    }

    fn unary(&mut self) -> Result<i64> {
        match self.peek() {
            Some(Token::Minus) => {
                self.index += 1;
                Ok(-self.unary()?)
            }
            Some(Token::Plus) => {
                self.index += 1;
                self.unary()
            }
            Some(Token::Word(word)) if word.eq_ignore_ascii_case("not") || word.eq_ignore_ascii_case("neg") => {
                let not = word.eq_ignore_ascii_case("not");
                self.index += 1;
                let operand = self.unary()?;
                if not { Ok(!fit(operand)?) } else { fit(-operand) }
            }
            _ => self.atom(),
        }
    }

    fn atom(&mut self) -> Result<i64> {
        match self.peek().cloned() {
            Some(Token::Open) => {
                self.index += 1;
                let value = self.or()?;
                if self.peek() != Some(&Token::Close) {
                    return Err(QuantusError::other(msg!("core.basen.bracket_not_closed")));
                }
                self.index += 1;
                Ok(value)
            }
            Some(Token::Word(word)) => {
                self.index += 1;
                // `\mathrm{h}1F`: a prefix on its own, then the digits.
                if let (Some(base), [_]) = (word.chars().next().and_then(NumberBase::of_prefix), word.as_bytes()) {
                    if let Some(Token::Word(digits)) = self.peek().cloned() {
                        self.index += 1;
                        return literal(&digits, base);
                    }
                }
                self.number(&word)
            }
            Some(other) => Err(QuantusError::other(msg!("core.basen.expected_number_before", other.text()))),
            None => Err(QuantusError::other(msg!("core.basen.input_ends_too_early"))),
        }
    }

    /// Digits in the set base, or `h1F`: a prefix that is no digit of the set base, then digits.
    fn number(&self, word: &str) -> Result<i64> {
        if let Ok(value) = literal(word, self.base) {
            return Ok(value);
        }
        let mut chars = word.chars();
        if let Some(base) = chars.next().filter(|c| !c.is_digit(self.base.radix())).and_then(NumberBase::of_prefix) {
            let rest = chars.as_str();
            if !rest.is_empty() {
                return literal(rest, base);
            }
        }
        literal(word, self.base)
    }
}

/// `digits` in `base`: up to 2147483647 in decimal, any 32-bit pattern in the others.
fn literal(digits: &str, base: NumberBase) -> Result<i64> {
    let invalid = || {
        QuantusError::other(msg!("core.basen.not_number", match base {
            NumberBase::Dec => "decimal",
            NumberBase::Hex => "hexadecimal",
            NumberBase::Bin => "binary",
            NumberBase::Oct => "octal",
        }))
    };
    if digits.is_empty() || !digits.chars().all(|c| c.is_digit(base.radix())) {
        return Err(invalid());
    }
    match base {
        // 2147483648 only as the operand of a minus; `fit` catches it anywhere else.
        NumberBase::Dec => digits.parse::<i64>().ok().filter(|v| *v <= 1 << 31).ok_or_else(overflow),
        _ => u32::from_str_radix(digits, base.radix()).map(|bits| i64::from(bits as i32)).map_err(|_| overflow()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(latex: &str, base: NumberBase) -> String {
        evaluate(latex, base).map(|r| r.latex).unwrap_or_else(|e| format!("error: {e}"))
    }

    #[test]
    fn numbers_are_read_and_written_in_the_base() {
        assert_eq!(eval("1F+1", NumberBase::Hex), "20");
        assert_eq!(eval("1010+1", NumberBase::Bin), "1011");
        assert_eq!(eval("7+1", NumberBase::Oct), "10");
        assert_eq!(eval("A", NumberBase::Hex), "\\mathrm{A}");
        assert_eq!(eval("12", NumberBase::Bin), "error: '12' is not a number in binary.");
    }

    #[test]
    fn negative_values_are_twos_complement() {
        assert_eq!(eval("0-1", NumberBase::Hex), "\\mathrm{FFFFFFFF}");
        assert_eq!(eval("FFFFFFFF", NumberBase::Hex), "\\mathrm{FFFFFFFF}");
        assert_eq!(evaluate("FFFFFFFF", NumberBase::Hex).unwrap().value, -1);
        assert_eq!(eval("2147483647+1", NumberBase::Dec), "error: The result does not fit 32 bits (from -2147483648 to 2147483647).");
        assert_eq!(eval("-2147483648", NumberBase::Dec), "-2147483648");
    }

    #[test]
    fn prefixes_read_one_number_in_another_base() {
        assert_eq!(eval("\\mathrm{h}1F + \\mathrm{b}11", NumberBase::Dec), "34");
        assert_eq!(eval("h1F+o7", NumberBase::Dec), "38");
        // In hexadecimal, d and b are digits, so only the marked prefix works.
        assert_eq!(eval("d10", NumberBase::Hex), "\\mathrm{D10}");
        assert_eq!(eval("\\mathrm{d}10", NumberBase::Hex), "\\mathrm{A}");
    }

    #[test]
    fn arithmetic_truncates() {
        assert_eq!(eval("7 \\div 2", NumberBase::Dec), "3");
        assert_eq!(eval("-7 \\div 2", NumberBase::Dec), "-3");
        assert_eq!(eval("\\frac{10}{3} \\cdot 2", NumberBase::Dec), "6");
        assert_eq!(eval("2(3+4)", NumberBase::Dec), "14");
        assert_eq!(eval("1 \\div 0", NumberBase::Dec), "error: Division by zero.");
    }

    #[test]
    fn logic_follows_the_casio() {
        assert_eq!(eval("1010 \\operatorname{and} 0110", NumberBase::Bin), "10");
        assert_eq!(eval("1010 or 0110", NumberBase::Bin), "1110");
        assert_eq!(eval("1010 \\operatorname{xor} 0110", NumberBase::Bin), "1100");
        assert_eq!(eval("1010 xnor 0110", NumberBase::Bin), "11111111111111111111111111110011");
        assert_eq!(eval("\\operatorname{Not}(0)", NumberBase::Hex), "\\mathrm{FFFFFFFF}");
        assert_eq!(eval("\\operatorname{Neg}(1)", NumberBase::Dec), "-1");
        // and binds tighter than or, + tighter than and.
        assert_eq!(eval("1 or 2 and 3", NumberBase::Dec), "3");
        assert_eq!(eval("1+1 and 3", NumberBase::Dec), "2");
    }

    #[test]
    fn the_other_bases_are_forms() {
        let result = evaluate("255", NumberBase::Dec).unwrap();
        let forms: Vec<_> = result.forms.iter().map(|f| f.latex.as_str()).collect();
        assert_eq!(forms, ["\\mathrm{FF}", "11111111", "377"]);
    }
}
