//! Basis-N: whole numbers in the bases 10, 16, 2 and 8 in a word of 8, 16, 32 or 64 bits, signed
//! (two's complement) or unsigned, with the bitwise operations of a Casio fx-991DE X.
//!
//! The input is LaTeX, but its own little language: `1F` is a hexadecimal number, not a product,
//! and `and` is an operator. So it has its own tokenizer and parser instead of the main ones.
//!
//! - Numbers are read in the set base; a prefix `d`, `h`, `b` or `o` (best as `\mathrm{h}`, or
//!   written straight before the digits where it cannot be a digit itself) reads one number in
//!   another base.
//! - `+ - \cdot \times \div \frac{}{}`; division truncates toward zero.
//! - `and`, `or`, `xor`, `xnor` (as `\operatorname{and}` or plain), `Not(x)`, `Neg(x)`.
//! - Shifts and rotations: `x << n` (`\ll`), `x >> n` (`\gg`; arithmetic when signed, logical
//!   when unsigned, as in C), and `shl`, `shr`, `sar`, `rol`, `ror` as `(x, n)` or `(x; n)`.
//! - `f32(x)`, `f64(x)`: the IEEE-754 pattern of the decimal number `x` (read as decimal in any
//!   base: `0{,}1`, `-1.5 \cdot 10^{-3}`, `2e10`), rounded to the nearest float.
//! - Precedence, highest first: sign, `Not`, `Neg`, functions; `\cdot` `\div`; `+` `-`; `<<` `>>`;
//!   `and`; `or` `xor` `xnor`.
//!
//! Every result wraps modulo 2ⁿ like a register, and the result says so ([`BaseNResult::wrapped`]).
//! A literal must fit the word: in decimal up to the top of the range (signed: up to 2ⁿ⁻¹, so that
//! `-128` can be typed in 8 bit), in the other bases any pattern of at most n bits (`FF` in 8 bit
//! signed is −1).

use serde::{Deserialize, Serialize};

use crate::forms::{FormKind, ResultForm};
use crate::ieee754::{self, Format, IeeeView};
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

/// The width of the word Basis-N computes in; written to JSON as the number of bits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WordSize {
    W8,
    W16,
    #[default]
    W32,
    W64,
}

impl WordSize {
    pub const ALL: [WordSize; 4] = [WordSize::W8, WordSize::W16, WordSize::W32, WordSize::W64];

    pub fn bits(self) -> u32 {
        match self {
            WordSize::W8 => 8,
            WordSize::W16 => 16,
            WordSize::W32 => 32,
            WordSize::W64 => 64,
        }
    }

    pub fn of_bits(bits: u64) -> Option<WordSize> {
        WordSize::ALL.into_iter().find(|w| u64::from(w.bits()) == bits)
    }
}

impl Serialize for WordSize {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_u32(self.bits())
    }
}

impl<'de> Deserialize<'de> for WordSize {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let bits = u64::deserialize(deserializer)?;
        WordSize::of_bits(bits).ok_or_else(|| serde::de::Error::custom(format!("word size {bits}: expected 8, 16, 32 or 64")))
    }
}

/// The word: its width in bits and whether it is read as two's complement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spec {
    pub width: u32,
    pub signed: bool,
}

impl Default for Spec {
    fn default() -> Self {
        Spec::new(WordSize::W32, true)
    }
}

impl Spec {
    pub fn new(word_size: WordSize, signed: bool) -> Spec {
        Spec { width: word_size.bits(), signed }
    }

    /// The `width` low bits set.
    pub fn mask(self) -> u64 {
        if self.width >= 64 { u64::MAX } else { (1 << self.width) - 1 }
    }

    pub fn min(self) -> i128 {
        if self.signed { -(1 << (self.width - 1)) } else { 0 }
    }

    pub fn max(self) -> i128 {
        if self.signed { (1 << (self.width - 1)) - 1 } else { (1 << self.width) - 1 }
    }

    /// The number a pattern stands for.
    pub fn read(self, bits: u64) -> i128 {
        let bits = bits & self.mask();
        if self.signed && bits >> (self.width - 1) & 1 == 1 { i128::from(bits) - (1 << self.width) } else { i128::from(bits) }
    }

    /// The pattern of `value` modulo 2ⁿ, and whether that changed the value.
    pub fn wrap(self, value: i128) -> (u64, bool) {
        // `as u64` keeps the low 64 bits of the two's complement, which is the value mod 2⁶⁴.
        let bits = value as u64 & self.mask();
        (bits, self.read(bits) != value)
    }
}

/// The bits of a Basis-N result, for the frontends to draw.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BitView {
    pub width: u32,
    pub signed: bool,
    /// `'0'` and `'1'`, the most significant bit first, all `width` of them.
    pub bits: String,
    /// At 32 and 64 bits, the pattern read as a float.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ieee: Option<IeeeView>,
}

impl BitView {
    pub fn new(bits: u64, spec: Spec) -> BitView {
        BitView { width: spec.width, signed: spec.signed, bits: pattern(bits, spec), ieee: None }
    }
}

/// The whole pattern, most significant bit first.
fn pattern(bits: u64, spec: Spec) -> String {
    format!("{:0width$b}", bits & spec.mask(), width = spec.width as usize)
}

/// The pattern in groups of four for copying: `\mathtt{0000\,1111}`.
pub fn grouped(bits: u64, spec: Spec) -> String {
    let pattern = pattern(bits, spec);
    let groups: Vec<&str> = pattern.as_bytes().chunks(4).map(|g| std::str::from_utf8(g).unwrap_or_default()).collect();
    format!("\\mathtt{{{}}}", groups.join("\\,"))
}

/// A Basis-N result: the pattern and how it is written in the set base and in the others.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BaseNResult {
    pub bits: u64,
    pub spec: Spec,
    pub latex: String,
    pub forms: Vec<ResultForm>,
    /// Whether a result on the way did not fit the word and was wrapped.
    pub wrapped: bool,
    /// The number typed into `f32(…)`/`f64(…)` when that is the whole input, as plain text.
    pub typed: Option<(Format, String)>,
}

impl BaseNResult {
    /// The number the pattern stands for, signed or unsigned as the word is.
    pub fn value(&self) -> i128 {
        self.spec.read(self.bits)
    }

    /// The bits, and at 32 and 64 bits the float they stand for, its value cut after
    /// `precision` significant digits.
    pub fn bit_view(&self, precision: usize) -> BitView {
        let mut view = BitView::new(self.bits, self.spec);
        view.ieee = Format::of_width(self.spec.width).map(|format| {
            let typed = self.typed.as_ref().filter(|(typed, _)| *typed == format).map(|(_, text)| text.as_str());
            ieee754::view(self.bits, format, typed, precision)
        });
        view
    }
}

pub fn evaluate(latex: &str, base: NumberBase, spec: Spec) -> Result<BaseNResult> {
    let tokens = tokenize(latex)?;
    if tokens.is_empty() {
        return Err(QuantusError::other(msg!("core.basen.type_number")));
    }
    let mut parser = Parser { tokens, index: 0, base, spec, wrapped: false };
    let value = parser.or()?;
    let value = parser.wrap(value);
    if parser.index < parser.tokens.len() {
        return Err(QuantusError::other(msg!("core.basen.unexpected", parser.tokens[parser.index].text())));
    }
    let bits = spec.wrap(value).0;
    let typed = match parser.tokens.as_slice() {
        [Token::Float(format, text)] => ieee754::plain_number(text).map(|plain| (*format, plain)),
        _ => None,
    };
    let forms = NumberBase::ALL
        .into_iter()
        .filter(|b| *b != base)
        .map(|b| ResultForm::new(b.form(), write(bits, spec, b)))
        .chain([ResultForm::new(FormKind::Bits, grouped(bits, spec))])
        .collect();
    Ok(BaseNResult { bits, spec, latex: write(bits, spec, base), forms, wrapped: parser.wrapped, typed })
}

/// `bits` in `base`: the number (signed or unsigned per `spec`) in decimal, the pattern without
/// leading zeros in the others.
pub fn write(bits: u64, spec: Spec, base: NumberBase) -> String {
    let bits = bits & spec.mask();
    let digits = match base {
        NumberBase::Dec => return spec.read(bits).to_string(),
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
    /// `,` or `;` between the two arguments of a shift function.
    Comma,
    ShiftLeft,
    ShiftRight,
    /// `f32(…)` or `f64(…)` with its argument as typed.
    Float(Format, String),
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
            Token::Comma => ",".into(),
            Token::ShiftLeft => "<<".into(),
            Token::ShiftRight => ">>".into(),
            Token::Float(format, text) => format!("{}({text})", if *format == Format::F32 { "f32" } else { "f64" }),
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
            '(' | '[' => (tokens.push(Token::Open), i += 1).1,
            // `{,}` is how a comma is written between digits; in Basis-N it separates arguments.
            '{' if chars.get(i + 1) == Some(&',') && chars.get(i + 2) == Some(&'}') => (tokens.push(Token::Comma), i += 3).1,
            '{' => (tokens.push(Token::Open), i += 1).1,
            ',' | ';' => (tokens.push(Token::Comma), i += 1).1,
            '<' | '>' => {
                // Only doubled: `<<`, `>>`; `\lt\lt` arrives here as `<<` too (see below).
                if chars.get(i + 1) != Some(&c) {
                    return Err(unexpected(c));
                }
                tokens.push(if c == '<' { Token::ShiftLeft } else { Token::ShiftRight });
                i += 2;
            }
            '\u{226a}' => (tokens.push(Token::ShiftLeft), i += 1).1,
            '\u{226b}' => (tokens.push(Token::ShiftRight), i += 1).1,
            ')' | ']' | '}' => (tokens.push(Token::Close), i += 1).1,
            c if c.is_ascii_alphanumeric() => {
                let start = i;
                while i < chars.len() && chars[i].is_ascii_alphanumeric() {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                match Format::of_name(&word).and_then(|format| float_argument(&chars, &mut i).map(|text| Token::Float(format, text))) {
                    Some(float) => tokens.push(float),
                    None => tokens.push(Token::Word(word)),
                }
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
                    "ll" => tokens.push(Token::ShiftLeft),
                    "gg" => tokens.push(Token::ShiftRight),
                    // MathLive writes a typed `<` as `\lt`: two in a row are a shift.
                    "lt" | "gt" => {
                        let rest: String = chars[i..].iter().collect::<String>();
                        let rest = rest.trim_start();
                        let pair = if name == "lt" { "\\lt" } else { "\\gt" };
                        if !rest.starts_with(pair) || rest[pair.len()..].starts_with(|c: char| c.is_ascii_alphabetic()) {
                            return Err(unexpected(if name == "lt" { '<' } else { '>' }));
                        }
                        i = chars.len() - rest.chars().count() + pair.len();
                        tokens.push(if name == "lt" { Token::ShiftLeft } else { Token::ShiftRight });
                    }
                    "semicolon" => tokens.push(Token::Comma),
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
                        let word: String = braced(&chars, &mut i)?.chars().filter(|c| !c.is_whitespace()).collect();
                        match Format::of_name(&word).and_then(|format| float_argument(&chars, &mut i).map(|text| Token::Float(format, text))) {
                            Some(float) => tokens.push(float),
                            None => tokens.push(Token::Word(word)),
                        }
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

/// After `f32`/`f64`: the argument in brackets as typed (`(0{,}1)`, `\left(1e3\right)`), and
/// `*i` past it; none without a bracket, and `*i` stays.
fn float_argument(chars: &[char], i: &mut usize) -> Option<String> {
    let rest: String = chars[*i..].iter().collect();
    let trimmed = rest.trim_start();
    let opening = if trimmed.starts_with("\\left(") { "\\left(" } else if trimmed.starts_with('(') { "(" } else { return None };
    let start = chars.len() - trimmed.chars().count() + opening.chars().count();
    let mut depth = 1;
    let mut at = start;
    while at < chars.len() {
        match chars[at] {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    let mut inner: String = chars[start..at].iter().collect();
                    if inner.trim_end().ends_with("\\right") {
                        inner.truncate(inner.trim_end().len() - "\\right".len());
                    }
                    *i = at + 1;
                    return Some(inner);
                }
            }
            _ => {}
        }
        at += 1;
    }
    None
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

/// A shift or rotation of the pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shift {
    /// Logical left: zeros in.
    Shl,
    /// Logical right: zeros in.
    Shr,
    /// Arithmetic right: copies of the sign bit in.
    Sar,
    Rol,
    Ror,
}

impl Shift {
    fn of_name(name: &str) -> Option<Shift> {
        Some(match name.to_ascii_lowercase().as_str() {
            "shl" => Shift::Shl,
            "shr" => Shift::Shr,
            "sar" => Shift::Sar,
            "rol" => Shift::Rol,
            "ror" => Shift::Ror,
            _ => return None,
        })
    }

    /// The pattern `bits` shifted by `count ≥ 0`: a shift by the width or more leaves 0 (or all
    /// sign bits for `Sar`), a rotation turns by `count mod width`.
    fn apply(self, bits: u64, count: i128, spec: Spec) -> u64 {
        let (width, mask) = (spec.width, spec.mask());
        let bits = bits & mask;
        let wide = count >= i128::from(width);
        let n = if wide { 0 } else { count as u32 };
        let turn = (count % i128::from(width)) as u32;
        let result = match self {
            Shift::Shl if wide => 0,
            Shift::Shl => bits.checked_shl(n).unwrap_or(0),
            Shift::Shr if wide => 0,
            Shift::Shr => bits >> n,
            Shift::Sar => {
                // Sign-extended from the word's top bit, whatever the signedness setting.
                let extended = Spec { width, signed: true }.read(bits);
                (extended >> if wide { width - 1 } else { n }) as u64
            }
            Shift::Rol | Shift::Ror if turn == 0 => bits,
            Shift::Rol => bits << turn | bits >> (width - turn),
            Shift::Ror => bits >> turn | bits << (width - turn),
        };
        result & mask
    }
}

/// Values are carried as `i128`: a literal may be 2ⁿ⁻¹ (to be negated), and each operation
/// computes from the numbers its operands stand for, then wraps the result into the word.
struct Parser {
    tokens: Vec<Token>,
    index: usize,
    base: NumberBase,
    spec: Spec,
    /// Whether a value was wrapped on the way.
    wrapped: bool,
}

impl Parser {
    /// `value` wrapped into the word, noting when that changed it.
    fn wrap(&mut self, value: i128) -> i128 {
        let (bits, wrapped) = self.spec.wrap(value);
        self.wrapped |= wrapped;
        self.spec.read(bits)
    }

    /// The pattern of an operand (wrapped into the word first).
    fn bits(&mut self, value: i128) -> u64 {
        let value = self.wrap(value);
        self.spec.wrap(value).0
    }

    /// The number a pattern stands for; bit operations never wrap.
    fn read(&self, bits: u64) -> i128 {
        self.spec.read(bits)
    }

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
    fn or(&mut self) -> Result<i128> {
        let mut left = self.and()?;
        while let Some(op) = self.word_is(&["or", "xor", "xnor"]) {
            self.index += 1;
            let right = self.and()?;
            let (left_bits, right_bits) = (self.bits(left), self.bits(right));
            left = self.read(match op.as_str() {
                "or" => left_bits | right_bits,
                "xor" => left_bits ^ right_bits,
                _ => !(left_bits ^ right_bits),
            });
        }
        Ok(left)
    }

    fn and(&mut self) -> Result<i128> {
        let mut left = self.shift()?;
        while self.word_is(&["and"]).is_some() {
            self.index += 1;
            let right = self.shift()?;
            let (left_bits, right_bits) = (self.bits(left), self.bits(right));
            left = self.read(left_bits & right_bits);
        }
        Ok(left)
    }

    /// `<<`, `>>`: below `+ -`, above `and`, as in C.
    fn shift(&mut self) -> Result<i128> {
        let mut left = self.sum()?;
        loop {
            let shift = match self.peek() {
                Some(Token::ShiftLeft) => Shift::Shl,
                Some(Token::ShiftRight) if self.spec.signed => Shift::Sar,
                Some(Token::ShiftRight) => Shift::Shr,
                _ => return Ok(left),
            };
            self.index += 1;
            let count = self.sum()?;
            left = self.apply(shift, left, count)?;
        }
    }

    /// `shift` of `value` by `count` bits; bits shifted out are no overflow.
    fn apply(&mut self, shift: Shift, value: i128, count: i128) -> Result<i128> {
        let bits = self.bits(value);
        let count = self.wrap(count);
        if count < 0 {
            return Err(QuantusError::other(msg!("core.basen.negative_shift", count)));
        }
        Ok(self.read(shift.apply(bits, count, self.spec)))
    }

    /// `shl(x, n)` and the like, after the name.
    fn function(&mut self, shift: Shift, name: &str) -> Result<i128> {
        if self.peek() != Some(&Token::Open) {
            return Err(QuantusError::other(msg!("core.basen.expected_bracket", name)));
        }
        self.index += 1;
        let value = self.or()?;
        if self.peek() != Some(&Token::Comma) {
            return Err(QuantusError::other(msg!("core.basen.two_arguments")));
        }
        self.index += 1;
        let count = self.or()?;
        if self.peek() != Some(&Token::Close) {
            return Err(QuantusError::other(msg!("core.basen.bracket_not_closed")));
        }
        self.index += 1;
        self.apply(shift, value, count)
    }

    fn sum(&mut self) -> Result<i128> {
        let mut left = self.product()?;
        loop {
            let plus = match self.peek() {
                Some(Token::Plus) => true,
                Some(Token::Minus) => false,
                _ => return Ok(left),
            };
            self.index += 1;
            let right = self.product()?;
            let (left_value, right_value) = (self.wrap(left), self.wrap(right));
            left = self.wrap(if plus { left_value + right_value } else { left_value - right_value });
        }
    }

    /// `left · right` wrapped; the product of two 64-bit numbers can exceed `i128`.
    fn multiply(&mut self, left: i128, right: i128) -> i128 {
        let (left, right) = (self.wrap(left), self.wrap(right));
        match left.checked_mul(right) {
            Some(product) => self.wrap(product),
            None => {
                self.wrapped = true;
                self.spec.read(self.spec.wrap(left.wrapping_mul(right)).0)
            }
        }
    }

    fn product(&mut self) -> Result<i128> {
        let mut left = self.unary()?;
        loop {
            let times = match self.peek() {
                Some(Token::Times) => true,
                Some(Token::Divide) => false,
                // `2(3)`: a bracket right after a value multiplies.
                Some(Token::Open) => {
                    let right = self.unary()?;
                    left = self.multiply(left, right);
                    continue;
                }
                _ => return Ok(left),
            };
            self.index += 1;
            let right = self.unary()?;
            left = if times {
                self.multiply(left, right)
            } else {
                // Truncated toward zero, from the numbers as the word reads them: unsigned
                // 8 bit, `-1 ÷ 2` is `255 ÷ 2`. MIN ÷ −1 wraps to MIN.
                let (left_value, right_value) = (self.wrap(left), self.wrap(right));
                if right_value == 0 {
                    return Err(QuantusError::other(msg!("core.common.division_zero")));
                }
                self.wrap(left_value / right_value)
            };
        }
    }

    fn unary(&mut self) -> Result<i128> {
        match self.peek() {
            Some(Token::Minus) => {
                self.index += 1;
                // Not wrapped yet: `-128` in 8 bit is the literal 128 negated.
                let operand = self.unary()?;
                Ok(self.wrap(-operand))
            }
            Some(Token::Plus) => {
                self.index += 1;
                self.unary()
            }
            Some(Token::Word(word)) if word.eq_ignore_ascii_case("not") || word.eq_ignore_ascii_case("neg") => {
                let not = word.eq_ignore_ascii_case("not");
                self.index += 1;
                let operand = self.unary()?;
                let bits = self.bits(operand);
                // Bit operations on the pattern: neither wraps.
                Ok(self.read(if not { !bits } else { bits.wrapping_neg() }))
            }
            _ => self.atom(),
        }
    }

    fn atom(&mut self) -> Result<i128> {
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
            Some(Token::Float(format, text)) => {
                self.index += 1;
                if self.spec.width < format.width() {
                    return Err(ieee754::needs_bits(format));
                }
                let plain = ieee754::plain_number(&text).ok_or_else(|| QuantusError::other(msg!("core.basen.not_float", text.trim())))?;
                // The pattern as the word reads it: in a 64-bit word an f32 fills the low half.
                Ok(self.read(ieee754::pattern(&plain, format)?))
            }
            Some(Token::Word(word)) if Shift::of_name(&word).is_some() => {
                self.index += 1;
                self.function(Shift::of_name(&word).unwrap_or(Shift::Shl), &word.to_ascii_lowercase())
            }
            Some(Token::Word(word)) => {
                self.index += 1;
                // `\mathrm{h}1F`: a prefix on its own, then the digits.
                if let (Some(base), [_]) = (word.chars().next().and_then(NumberBase::of_prefix), word.as_bytes()) {
                    if let Some(Token::Word(digits)) = self.peek().cloned() {
                        self.index += 1;
                        return literal(&digits, base, self.spec);
                    }
                }
                self.number(&word)
            }
            Some(other) => Err(QuantusError::other(msg!("core.basen.expected_number_before", other.text()))),
            None => Err(QuantusError::other(msg!("core.basen.input_ends_too_early"))),
        }
    }

    /// Digits in the set base, or `h1F`: a prefix that is no digit of the set base, then digits.
    fn number(&self, word: &str) -> Result<i128> {
        if let Ok(value) = literal(word, self.base, self.spec) {
            return Ok(value);
        }
        let mut chars = word.chars();
        if let Some(base) = chars.next().filter(|c| !c.is_digit(self.base.radix())).and_then(NumberBase::of_prefix) {
            let rest = chars.as_str();
            if !rest.is_empty() {
                return literal(rest, base, self.spec);
            }
        }
        literal(word, self.base, self.spec)
    }
}

/// `digits` in `base`: in decimal up to the top of the range (signed: up to 2ⁿ⁻¹, as the operand
/// of a minus), in the others any pattern of at most n bits, read as the word reads it.
fn literal(digits: &str, base: NumberBase, spec: Spec) -> Result<i128> {
    if digits.is_empty() || !digits.chars().all(|c| c.is_digit(base.radix())) {
        return Err(QuantusError::other(match base {
            NumberBase::Dec => msg!("core.basen.not_decimal", digits),
            NumberBase::Hex => msg!("core.basen.not_hexadecimal", digits),
            NumberBase::Bin => msg!("core.basen.not_binary", digits),
            NumberBase::Oct => msg!("core.basen.not_octal", digits),
        }));
    }
    let too_big = || QuantusError::other(msg!("core.basen.literal_too_big", digits, spec.width));
    let value = u128::from_str_radix(digits, base.radix()).map_err(|_| too_big())?;
    match base {
        NumberBase::Dec => {
            let top = if spec.signed { 1u128 << (spec.width - 1) } else { u128::from(spec.mask()) };
            if value > top { Err(too_big()) } else { Ok(value as i128) }
        }
        _ if value > u128::from(spec.mask()) => Err(too_big()),
        _ => Ok(spec.read(value as u64)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(latex: &str, base: NumberBase) -> String {
        eval_in(latex, base, Spec::default())
    }

    fn eval_in(latex: &str, base: NumberBase, spec: Spec) -> String {
        evaluate(latex, base, spec)
            .map(|r| if r.wrapped { format!("{} (wrapped)", r.latex) } else { r.latex })
            .unwrap_or_else(|e| format!("error: {e}"))
    }

    fn spec(width: WordSize, signed: bool) -> Spec {
        Spec::new(width, signed)
    }

    #[test]
    fn numbers_are_read_and_written_in_the_base() {
        assert_eq!(eval("1F+1", NumberBase::Hex), "20");
        assert_eq!(eval("1010+1", NumberBase::Bin), "1011");
        assert_eq!(eval("7+1", NumberBase::Oct), "10");
        assert_eq!(eval("A", NumberBase::Hex), "\\mathrm{A}");
        assert_eq!(eval("12", NumberBase::Bin), "error: '12' is not a binary number.");
    }

    #[test]
    fn negative_values_are_twos_complement() {
        assert_eq!(eval("0-1", NumberBase::Hex), "\\mathrm{FFFFFFFF}");
        assert_eq!(eval("FFFFFFFF", NumberBase::Hex), "\\mathrm{FFFFFFFF}");
        assert_eq!(evaluate("FFFFFFFF", NumberBase::Hex, Spec::default()).unwrap().value(), -1);
        assert_eq!(eval("2147483647+1", NumberBase::Dec), "-2147483648 (wrapped)");
        assert_eq!(eval("-2147483648", NumberBase::Dec), "-2147483648");
    }

    #[test]
    fn the_word_has_a_range_per_width_and_signedness() {
        for width in WordSize::ALL {
            for signed in [true, false] {
                let spec = spec(width, signed);
                let (min, max) = (spec.min(), spec.max());
                assert_eq!(eval_in(&max.to_string(), NumberBase::Dec, spec), max.to_string());
                assert_eq!(eval_in(&format!("{max}+1"), NumberBase::Dec, spec), format!("{min} (wrapped)"));
                assert_eq!(eval_in(&format!("{min}-1"), NumberBase::Dec, spec), format!("{max} (wrapped)"));
                assert_eq!(eval_in("0-1", NumberBase::Hex, spec).replace(" (wrapped)", ""), write(u64::MAX, spec, NumberBase::Hex));
            }
        }
        assert_eq!(eval_in("255", NumberBase::Dec, spec(WordSize::W8, false)), "255");
        assert_eq!(eval_in("18446744073709551615", NumberBase::Dec, spec(WordSize::W64, false)), "18446744073709551615");
        assert_eq!(eval_in("-9223372036854775808", NumberBase::Dec, spec(WordSize::W64, true)), "-9223372036854775808");
        assert_eq!(eval_in("FFFFFFFFFFFFFFFF", NumberBase::Hex, spec(WordSize::W64, true)), "\\mathrm{FFFFFFFFFFFFFFFF}");
    }

    #[test]
    fn a_literal_must_fit_the_word() {
        let byte = spec(WordSize::W8, true);
        assert_eq!(eval_in("-128", NumberBase::Dec, byte), "-128");
        assert_eq!(eval_in("FF", NumberBase::Hex, byte), "\\mathrm{FF}");
        assert_eq!(evaluate("FF", NumberBase::Hex, byte).unwrap().value(), -1);
        assert_eq!(eval_in("1FF", NumberBase::Hex, byte), "error: '1FF' does not fit 8 bits.");
        assert_eq!(eval_in("129", NumberBase::Dec, byte), "error: '129' does not fit 8 bits.");
        assert_eq!(eval_in("256", NumberBase::Dec, spec(WordSize::W8, false)), "error: '256' does not fit 8 bits.");
        // 128 alone is 2⁷: it fits only as the operand of a minus, elsewhere it wraps.
        assert_eq!(eval_in("128", NumberBase::Dec, byte), "-128 (wrapped)");
        assert_eq!(eval_in("h1FF", NumberBase::Dec, byte), "error: '1FF' does not fit 8 bits.");
        assert_eq!(eval_in("99999999999999999999999", NumberBase::Dec, spec(WordSize::W64, false)), "error: '99999999999999999999999' does not fit 64 bits.");
    }

    #[test]
    fn results_wrap_like_a_register() {
        let byte = spec(WordSize::W8, false);
        assert_eq!(eval_in("200+100", NumberBase::Dec, byte), "44 (wrapped)");
        assert_eq!(eval_in("16 \\cdot 16", NumberBase::Dec, byte), "0 (wrapped)");
        assert_eq!(eval_in("-1", NumberBase::Dec, byte), "255 (wrapped)");
        // Bit operations never wrap.
        assert_eq!(eval_in("\\operatorname{Not}(0)", NumberBase::Dec, byte), "255");
        assert_eq!(eval_in("\\operatorname{Neg}(1)", NumberBase::Dec, byte), "255");
        assert_eq!(eval_in("F0 xnor 0F", NumberBase::Hex, byte), "0");
        // 64 bit unsigned: the product exceeds i128.
        let long = spec(WordSize::W64, false);
        assert_eq!(eval_in("FFFFFFFFFFFFFFFF \\cdot FFFFFFFFFFFFFFFF", NumberBase::Hex, long), "1 (wrapped)");
        assert_eq!(eval_in("FFFFFFFFFFFFFFFF \\cdot 2", NumberBase::Hex, long), "\\mathrm{FFFFFFFFFFFFFFFE} (wrapped)");
    }

    #[test]
    fn division_is_signed_or_unsigned_by_the_word() {
        assert_eq!(eval_in("-1 \\div 2", NumberBase::Dec, spec(WordSize::W8, true)), "0");
        assert_eq!(eval_in("-1 \\div 2", NumberBase::Dec, spec(WordSize::W8, false)), "127 (wrapped)");
        assert_eq!(eval_in("FF \\div 2", NumberBase::Hex, spec(WordSize::W8, false)), "\\mathrm{7F}");
        assert_eq!(eval_in("FF \\div 2", NumberBase::Hex, spec(WordSize::W8, true)), "0");
        // MIN ÷ −1 wraps to MIN.
        assert_eq!(eval_in("-128 \\div -1", NumberBase::Dec, spec(WordSize::W8, true)), "-128 (wrapped)");
        assert_eq!(eval_in("-9223372036854775808 \\div -1", NumberBase::Dec, spec(WordSize::W64, true)), "-9223372036854775808 (wrapped)");
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
    fn shifts_move_the_pattern() {
        let byte = spec(WordSize::W8, true);
        let ubyte = spec(WordSize::W8, false);
        assert_eq!(eval_in(r"\operatorname{shl}(1, 3)", NumberBase::Dec, byte), "8");
        assert_eq!(eval_in("shl(81; 1)", NumberBase::Hex, byte), "2");
        assert_eq!(eval_in("shr(80, 7)", NumberBase::Hex, byte), "1");
        assert_eq!(eval_in("sar(80, 7)", NumberBase::Hex, byte), "\\mathrm{FF}");
        assert_eq!(eval_in("sar(80, 7)", NumberBase::Hex, ubyte), "\\mathrm{FF}");
        assert_eq!(eval_in("sar(40, 3)", NumberBase::Hex, byte), "8");
        // By the width or more: 0, or all sign bits for sar.
        for shift in ["shl", "shr"] {
            assert_eq!(eval_in(&format!("{shift}(FF, 8)"), NumberBase::Hex, ubyte), "0");
            assert_eq!(eval_in(&format!("{shift}(FF, C8)"), NumberBase::Hex, ubyte), "0");
        }
        assert_eq!(eval_in("sar(80, 8)", NumberBase::Hex, ubyte), "\\mathrm{FF}");
        assert_eq!(eval_in("sar(7F, 64)", NumberBase::Hex, ubyte), "0");
        assert_eq!(eval_in("shl(1, 3F)", NumberBase::Hex, spec(WordSize::W64, false)), "8000000000000000");
        assert_eq!(eval_in("shl(1, 63)", NumberBase::Dec, spec(WordSize::W64, true)), "-9223372036854775808");
        assert_eq!(eval_in("sar(h8000000000000000, 63)", NumberBase::Dec, spec(WordSize::W64, true)), "-1");
        // Shifting bits out is no overflow.
        assert!(!evaluate("shl(FF, 4)", NumberBase::Hex, ubyte).unwrap().wrapped);
        assert_eq!(eval_in("shl(1, -1)", NumberBase::Dec, byte), "error: A shift count must not be negative (-1).");
    }

    #[test]
    fn rotations_turn_within_the_word() {
        let ubyte = spec(WordSize::W8, false);
        assert_eq!(eval_in("rol(81, 1)", NumberBase::Hex, ubyte), "3");
        assert_eq!(eval_in("ror(81, 1)", NumberBase::Hex, ubyte), "\\mathrm{C0}");
        // Counts in hexadecimal: 0, 8, 16.
        for n in ["0", "8", "10"] {
            assert_eq!(eval_in(&format!("rol(A5, {n})"), NumberBase::Hex, ubyte), "\\mathrm{A5}");
        }
        assert_eq!(eval_in("rol(81, 9)", NumberBase::Hex, ubyte), "3");
        assert_eq!(eval_in("ror(1, 1)", NumberBase::Hex, spec(WordSize::W16, false)), "8000");
        assert_eq!(eval_in("ror(1, 1)", NumberBase::Hex, spec(WordSize::W64, false)), "8000000000000000");
        assert_eq!(eval_in("ROL(1, 41)", NumberBase::Hex, spec(WordSize::W64, false)), "2");
    }

    #[test]
    fn the_shift_operators_sit_between_sum_and_and() {
        assert_eq!(eval("1 + 1 << 2", NumberBase::Dec), "8");
        assert_eq!(eval("1 << 2 and 4", NumberBase::Dec), "4");
        assert_eq!(eval(r"1 \ll 4", NumberBase::Dec), "16");
        assert_eq!(eval(r"1\lt\lt 4", NumberBase::Dec), "16");
        assert_eq!(eval(r"64 \gg 2 \gg 1", NumberBase::Dec), "8");
        assert_eq!(eval(r"256 \gt\gt 4", NumberBase::Dec), "16");
        // >> is arithmetic when signed, logical when unsigned.
        assert_eq!(eval_in("-16 >> 2", NumberBase::Dec, spec(WordSize::W8, true)), "-4");
        assert_eq!(eval_in("F0 >> 2", NumberBase::Hex, spec(WordSize::W8, false)), "\\mathrm{3C}");
        assert_eq!(eval("1 < 2", NumberBase::Dec), "error: Basis-N does not know '<'.");
        assert_eq!(eval(r"1 \lt 2", NumberBase::Dec), "error: Basis-N does not know '<'.");
    }

    #[test]
    fn shift_functions_need_two_arguments() {
        assert_eq!(eval("shl(1)", NumberBase::Dec), "error: 'shl' takes two numbers: shl(x, n).");
        assert_eq!(eval("ror 1", NumberBase::Dec), "error: Expected '(' after 'ror'.");
        assert_eq!(eval(r"\operatorname{rol}\left(1{,}2\right)", NumberBase::Dec), "4");
        assert_eq!(eval("shl(1, 2) + shr(16, 2)", NumberBase::Dec), "8");
    }

    #[test]
    fn floats_are_typed_as_decimals_in_any_base() {
        let word = spec(WordSize::W32, false);
        assert_eq!(eval_in(r"f32(0{,}1)", NumberBase::Hex, word), "\\mathrm{3DCCCCCD}");
        assert_eq!(eval_in(r"\operatorname{f32}\left(0.1\right)", NumberBase::Hex, word), "\\mathrm{3DCCCCCD}");
        assert_eq!(eval_in("f32(1)", NumberBase::Bin, word), "111111100000000000000000000000");
        assert_eq!(eval_in(r"f32(-2{,}5 \cdot 10^{3})", NumberBase::Hex, word), "\\mathrm{C51C4000}");
        assert_eq!(eval_in("f64(0.1)", NumberBase::Hex, spec(WordSize::W64, false)), "\\mathrm{3FB999999999999A}");
        // In a 64-bit word an f32 fills the low half.
        assert_eq!(eval_in("f32(1)", NumberBase::Hex, spec(WordSize::W64, true)), "\\mathrm{3F800000}");
        // The pattern takes part in the rest of the input.
        assert_eq!(eval_in("f32(1) >> 23", NumberBase::Dec, word), "127");
        assert_eq!(eval_in("f32(1)", NumberBase::Hex, spec(WordSize::W16, true)), "error: f32 needs a word of at least 32 bits.");
        assert_eq!(eval_in("f64(1)", NumberBase::Hex, word), "error: f64 needs a word of at least 64 bits.");
        assert_eq!(eval_in("f32(x)", NumberBase::Hex, word), "error: 'x' is not a decimal number for f32 or f64.");
        // F32 in capitals is a hexadecimal number.
        assert_eq!(eval_in("F32(2)", NumberBase::Hex, word), "\\mathrm{1E64}");
    }

    #[test]
    fn a_word_of_32_or_64_bits_is_read_as_a_float_too() {
        let result = evaluate("f32(0{,}1)", NumberBase::Hex, Spec::default()).unwrap();
        let ieee = result.bit_view(50).ieee.unwrap();
        assert_eq!(ieee.exact, "0{,}100000001490116119384765625");
        assert_eq!(ieee.typed.as_deref(), Some("0{,}1"));
        assert_eq!(ieee.error.as_deref(), Some("0{,}000000001490116119384765625"));
        // Any pattern: 3F800000 is 1.0, and no typed number.
        let one = evaluate("3F800000", NumberBase::Hex, Spec::default()).unwrap().bit_view(50).ieee.unwrap();
        assert_eq!((one.exact.as_str(), one.typed), ("1", None));
        // Not at 8 and 16 bits; at 64 an f32 pattern is read as a (tiny) f64.
        assert!(evaluate("1", NumberBase::Hex, spec(WordSize::W16, true)).unwrap().bit_view(50).ieee.is_none());
        let wide = evaluate("f32(1)", NumberBase::Hex, spec(WordSize::W64, true)).unwrap().bit_view(50).ieee.unwrap();
        assert_eq!((wide.class, wide.typed), (crate::ieee754::FloatClass::Subnormal, None));
    }

    #[test]
    fn the_other_bases_are_forms() {
        let result = evaluate("255", NumberBase::Dec, Spec::default()).unwrap();
        let forms: Vec<_> = result.forms.iter().map(|f| f.latex.as_str()).collect();
        assert_eq!(forms, ["\\mathrm{FF}", "11111111", "377", "\\mathtt{0000\\,0000\\,0000\\,0000\\,0000\\,0000\\,1111\\,1111}"]);
        let result = evaluate("FF", NumberBase::Hex, spec(WordSize::W8, true)).unwrap();
        assert_eq!(result.forms[0].latex, "-1");
    }
}
