//! The parser of the logic mode's lines (§5.2, §5.3): MathLive LaTeX in a language of its own,
//! since `+`, juxtaposition and the overline mean other things here than in algebra.
//!
//! A line is `f = …`, `f(A, B, C) = …` or just the right side; `AB` is the two variables `A`
//! and `B`, a longer name is written `\mathrm{Start}`, and the words `and`, `or`, `not`, `xor`,
//! `xnor`, `nand`, `nor` are operators.

use super::expr::Expr;
use super::text::msg;
use super::{all_rows, check_names, set_bit, Bits, Function, MAX_INPUTS};
use crate::{QuantusError, Result};

/// One parsed line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    /// The output's name, `f` in `f = …`; for a line without one, the next free of `f`, `g`, `h`, … .
    pub output: String,
    /// The inputs `f(A, B, C)` declares, in their order.
    pub inputs: Option<Vec<String>>,
    pub body: Body,
}

/// The right side of a line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Body {
    Expr(Expr),
    Minterms(MintermList),
}

impl Body {
    /// The variables in the order they first appear.
    pub fn variables(&self) -> Vec<String> {
        match self {
            Body::Expr(expr) => expr.variables(),
            Body::Minterms(_) => Vec::new(),
        }
    }
}

/// Parses one line per output; lines without a name get `f`, `g`, `h`, … , skipping the names
/// other lines give.
pub fn parse_lines(lines: &[&str]) -> Result<Vec<Line>> {
    let parsed: Vec<(Option<String>, Option<Vec<String>>, Body)> = lines.iter().map(|line| parse_parts(line)).collect::<Result<_>>()?;
    let mut taken: Vec<String> = parsed.iter().filter_map(|(name, ..)| name.clone()).collect();
    let mut lines = Vec::new();
    for (name, inputs, body) in parsed {
        let output = match name {
            Some(name) => name,
            None => {
                let free = ('f'..='z').map(String::from).find(|n| !taken.contains(n)).unwrap_or_else(|| format!("f_{}", taken.len()));
                taken.push(free.clone());
                free
            }
        };
        lines.push(Line { output, inputs, body });
    }
    Ok(lines)
}

/// Parses one line; without a name the output is `f`.
pub fn parse_line(latex: &str) -> Result<Line> {
    Ok(parse_lines(&[latex])?.remove(0))
}

/// Parses an expression alone, without `f =`.
pub fn parse_expr(latex: &str) -> Result<Expr> {
    let tokens = tokenize(latex)?;
    let expr = Parser::new(&tokens).whole()?;
    check_names(&expr.variables())?;
    Ok(expr)
}

fn parse_parts(latex: &str) -> Result<(Option<String>, Option<Vec<String>>, Body)> {
    let tokens = tokenize(latex)?;
    let (left, right) = match tokens.iter().position(|t| *t == Token::Eq) {
        Some(eq) => (Some(&tokens[..eq]), &tokens[eq + 1..]),
        None => (None, &tokens[..]),
    };
    let (output, inputs) = match left {
        Some(left) => {
            let (output, inputs) = left_side(left)?;
            (Some(output), inputs)
        }
        None => (None, None),
    };
    let body = if is_minterm_list(right) { Body::Minterms(minterm_list(right)?) } else { Body::Expr(Parser::new(right).whole()?) };
    let variables = body.variables();
    check_names(&variables)?;
    if let Some(inputs) = &inputs {
        check_names(inputs)?;
        if let Some(name) = variables.iter().find(|v| !inputs.contains(v)) {
            let output = output.as_deref().unwrap_or("f");
            return Err(QuantusError::other(msg!("core.logic.not_declared")));
        }
    }
    if let Some(name) = output.as_ref().filter(|o| variables.contains(o) || inputs.as_ref().is_some_and(|i| i.contains(o))) {
        return Err(QuantusError::other(msg!("core.logic.output_as_input")));
    }
    Ok((output, inputs, body))
}

/// A list of minterms, `\sum m(1, 3, 7) + d(5)`, or of maxterms, `\prod M(0, 2) \cdot d(5)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MintermList {
    /// Whether the indices are maxterms, the rows that are 0.
    pub maxterms: bool,
    /// The rows that are 1 (minterms) or 0 (maxterms), as typed.
    pub indices: Vec<usize>,
    pub dont_cares: Vec<usize>,
}

impl MintermList {
    /// The function of the list. With the inputs `declared` by `f(A, B, C)`, over those;
    /// otherwise over the fewest variables that hold the highest index, but at least the
    /// scene's `inputs`, which name them when there are as many, else `A`, `B`, `C`, … do.
    pub fn function(&self, declared: Option<&[String]>, inputs: &[String]) -> Result<Function> {
        let highest = self.indices.iter().chain(&self.dont_cares).copied().max();
        let needed = highest.map_or(0, |h| (usize::BITS - h.leading_zeros()) as usize);
        let names: Vec<String> = match declared {
            Some(declared) => declared.to_vec(),
            None if needed <= inputs.len() => inputs.to_vec(),
            None => {
                let count = needed;
                if count > MAX_INPUTS {
                    return Err(QuantusError::other(msg!("core.logic.too_many_variables")));
                }
                ('A'..).take(count).map(String::from).collect()
            }
        };
        let n = names.len();
        let last = (1usize << n) - 1;
        if let Some(index) = highest.filter(|h| *h > last) {
            return Err(QuantusError::other(msg!("core.logic.index_too_large")));
        }
        let mut listed: Bits = [0; 4];
        let mut dont_cares: Bits = [0; 4];
        for (rows, set) in [(&self.indices, &mut listed), (&self.dont_cares, &mut dont_cares)] {
            for &row in rows {
                set_bit(set, row, true);
            }
        }
        let ones = if self.maxterms {
            let all = all_rows(n);
            std::array::from_fn(|w| all[w] & !listed[w])
        } else {
            listed
        };
        Function::from_bits(names, ones, dont_cares)
    }
}

/// Whether the tokens are a minterm list rather than an expression: they start with `\sum` or
/// `\prod`, or with `m(` or `M(` followed by a number or `)`.
fn is_minterm_list(tokens: &[Token]) -> bool {
    match tokens {
        [Token::Sum | Token::Prod, ..] => true,
        [Token::Var(letter), Token::Open, Token::Num(_) | Token::Close, ..] => letter == "m" || letter == "M",
        _ => false,
    }
}

fn minterm_list(tokens: &[Token]) -> Result<MintermList> {
    let bad = || QuantusError::other(msg!("core.logic.bad_list"));
    let mut rest = tokens;
    let symbol = match rest.first() {
        Some(Token::Sum) => Some(false),
        Some(Token::Prod) => Some(true),
        _ => None,
    };
    if symbol.is_some() {
        rest = &rest[1..];
    }
    let letter = match rest.first() {
        Some(Token::Var(letter)) if letter == "m" || letter == "M" => {
            rest = &rest[1..];
            Some(letter == "M")
        }
        _ => None,
    };
    let maxterms = symbol.or(letter).ok_or_else(bad)?;
    let indices = numbers(&mut rest).ok_or_else(bad)?;
    if matches!(rest.first(), Some(Token::Or | Token::And)) {
        rest = &rest[1..];
    }
    let dont_cares = match rest.first() {
        None => Vec::new(),
        Some(Token::Var(d)) if d == "d" => {
            rest = &rest[1..];
            numbers(&mut rest).ok_or_else(bad)?
        }
        Some(_) => return Err(bad()),
    };
    if !rest.is_empty() {
        return Err(bad());
    }
    let mut seen: Vec<usize> = Vec::new();
    for &index in indices.iter().chain(&dont_cares) {
        if seen.contains(&index) {
            return Err(QuantusError::other(msg!("core.logic.index_twice")));
        }
        seen.push(index);
    }
    Ok(MintermList { maxterms, indices, dont_cares })
}

/// `(1, 3, 7)` at the start of `tokens`, which moves past it.
fn numbers(tokens: &mut &[Token]) -> Option<Vec<usize>> {
    let [Token::Open, rest @ ..] = *tokens else { return None };
    let close = rest.iter().position(|t| *t == Token::Close)?;
    let inner = &rest[..close];
    *tokens = &rest[close + 1..];
    if inner.is_empty() {
        return Some(Vec::new());
    }
    inner
        .iter()
        .enumerate()
        .map(|(i, token)| match (i % 2, token) {
            (0, Token::Num(digits)) => Some(Some(digits.parse().unwrap_or(usize::MAX))),
            (1, Token::Comma) if i + 1 < inner.len() => Some(None),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()
        .map(|items| items.into_iter().flatten().collect())
}

/// `f` or `f(A, B, C)`.
fn left_side(tokens: &[Token]) -> Result<(String, Option<Vec<String>>)> {
    let bad = || QuantusError::other(msg!("core.logic.bad_left_side"));
    let Some((Token::Var(output), rest)) = tokens.split_first() else { return Err(bad()) };
    if rest.is_empty() {
        return Ok((output.clone(), None));
    }
    let [Token::Open, inner @ .., Token::Close] = rest else { return Err(bad()) };
    let mut inputs = Vec::new();
    for (i, token) in inner.iter().enumerate() {
        match (i % 2, token) {
            (0, Token::Var(name)) => inputs.push(name.clone()),
            (1, Token::Comma) => {}
            _ => return Err(bad()),
        }
    }
    if inner.len() % 2 == 0 {
        return Err(bad());
    }
    Ok((output.clone(), Some(inputs)))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Token {
    /// A variable: `A`, `x_1`, `Start`.
    Var(String),
    /// A run of digits.
    Num(String),
    Not,
    /// The postfix `'`.
    Prime,
    And,
    Or,
    Xor,
    Xnor,
    Nand,
    Nor,
    Open,
    Close,
    Comma,
    Eq,
    /// `\sum`, `\Sigma`.
    Sum,
    /// `\prod`, `\Pi`.
    Prod,
}

impl Token {
    pub(super) fn text(&self) -> String {
        match self {
            Token::Var(s) | Token::Num(s) => s.clone(),
            Token::Not => "¬".into(),
            Token::Prime => "'".into(),
            Token::And => "∧".into(),
            Token::Or => "∨".into(),
            Token::Xor => "⊕".into(),
            Token::Xnor => "⊙".into(),
            Token::Nand => "↑".into(),
            Token::Nor => "↓".into(),
            Token::Open => "(".into(),
            Token::Close => ")".into(),
            Token::Comma => ",".into(),
            Token::Eq => "=".into(),
            Token::Sum => "∑".into(),
            Token::Prod => "∏".into(),
        }
    }

    /// Whether the token can begin an operand, so that it multiplies when written after one.
    fn begins_operand(&self) -> bool {
        matches!(self, Token::Var(_) | Token::Num(_) | Token::Not | Token::Open)
    }
}

/// The operator words; in plain letters only in lower case.
fn word(text: &str) -> Option<Token> {
    Some(match text {
        "and" => Token::And,
        "or" => Token::Or,
        "not" => Token::Not,
        "xor" => Token::Xor,
        "xnor" => Token::Xnor,
        "nand" => Token::Nand,
        "nor" => Token::Nor,
        _ => return None,
    })
}

/// Commands whose braced argument is a word: a name or an operator.
const WORD_COMMANDS: [&str; 8] = ["mathrm", "operatorname", "text", "textrm", "mathit", "mathbf", "mathsf", "textit"];
/// Commands that are spacing or sizing only.
const IGNORED_COMMANDS: [&str; 15] =
    ["left", "right", "big", "Big", "bigl", "bigr", "Bigl", "Bigr", "displaystyle", "quad", "qquad", "limits", "nolimits", "mathopen", "mathclose"];

pub(super) fn tokenize(latex: &str) -> Result<Vec<Token>> {
    let chars: Vec<char> = latex.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        i += 1;
        let token = match c {
            ' ' | '\t' | '\n' | '~' => continue,
            '\'' | '\u{2032}' => Token::Prime,
            '!' | '¬' => Token::Not,
            '*' | '&' | '·' | '∧' | '×' => Token::And,
            '+' | '|' | '∨' => Token::Or,
            '⊕' => Token::Xor,
            '⊙' | '↔' => Token::Xnor,
            '↑' => Token::Nand,
            '↓' => Token::Nor,
            '(' | '[' | '{' => Token::Open,
            ')' | ']' | '}' => Token::Close,
            ',' => Token::Comma,
            '=' => Token::Eq,
            '^' => {
                // `^{\prime}`, `^\prime`, `^'`.
                skip_spaces(&chars, &mut i);
                let rest: String = chars[i..].iter().collect();
                let superscript = if rest.starts_with('{') { braced(&chars, &mut i)?.replace(' ', "") } else { String::new() };
                if superscript == "\\prime" || superscript == "'" {
                    Token::Prime
                } else if superscript.is_empty() && rest.starts_with("\\prime") {
                    i += "\\prime".len();
                    Token::Prime
                } else if superscript.is_empty() && rest.starts_with('\'') {
                    i += 1;
                    Token::Prime
                } else {
                    return Err(unknown_character(c));
                }
            }
            c if c.is_ascii_digit() => {
                let start = i - 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                Token::Num(chars[start..i].iter().collect())
            }
            c if c.is_ascii_alphabetic() => {
                let start = i - 1;
                while i < chars.len() && chars[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let run: String = chars[start..i].iter().collect();
                if let Some(op) = word(&run) {
                    op
                } else {
                    tokens.extend(run.chars().map(|letter| Token::Var(letter.to_string())));
                    if chars.get(i) == Some(&'_') {
                        i += 1;
                        let index = subscript(&chars, &mut i)?;
                        if let Some(Token::Var(name)) = tokens.last_mut() {
                            name.push('_');
                            name.push_str(&index);
                        }
                    }
                    continue;
                }
            }
            '\\' => {
                let start = i;
                while i < chars.len() && chars[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let name: String = chars[start..i].iter().collect();
                if name.is_empty() {
                    // `\&`, `\{`, `\}`, `\|`; `\,`, `\;`, `\ ` and the like are spacing.
                    let next = chars.get(i).copied();
                    i += 1;
                    match next {
                        Some('&') => Token::And,
                        Some('|') => Token::Or,
                        Some('{') => Token::Open,
                        Some('}') => Token::Close,
                        _ => continue,
                    }
                } else {
                    match command(&name, &chars, &mut i, &mut tokens)? {
                        Some(token) => token,
                        None => continue,
                    }
                }
            }
            other => return Err(unknown_character(other)),
        };
        tokens.push(token);
    }
    Ok(tokens)
}

/// The token of the command `\name`, whose arguments start at `*i`; `None` for one that adds
/// nothing or that pushed its tokens itself.
fn command(name: &str, chars: &[char], i: &mut usize, tokens: &mut Vec<Token>) -> Result<Option<Token>> {
    Ok(Some(match name {
        "land" | "wedge" | "cdot" | "ast" | "times" => Token::And,
        "lor" | "vee" | "mid" | "vert" => Token::Or,
        "oplus" | "veebar" => Token::Xor,
        "odot" | "leftrightarrow" | "Leftrightarrow" | "equiv" => Token::Xnor,
        "uparrow" | "barwedge" => Token::Nand,
        "downarrow" => Token::Nor,
        "neg" | "lnot" => Token::Not,
        "prime" => Token::Prime,
        "top" => Token::Num("1".into()),
        "bot" => Token::Num("0".into()),
        "overline" | "bar" => {
            let argument = argument(name, chars, i)?;
            tokens.push(Token::Not);
            tokens.push(Token::Open);
            tokens.extend(tokenize(&argument)?);
            Token::Close
        }
        "sum" | "Sigma" | "prod" | "Pi" => {
            tokens.push(if matches!(name, "sum" | "Sigma") { Token::Sum } else { Token::Prod });
            skip_spaces(chars, i);
            if chars.get(*i) == Some(&'_') {
                *i += 1;
                let index = subscript(chars, i)?;
                tokens.push(Token::Var(index));
            }
            return Ok(None);
        }
        name if WORD_COMMANDS.contains(&name) => {
            let text: String = argument(name, chars, i)?.chars().filter(|c| !c.is_whitespace()).collect();
            if let Some(op) = word(&text.to_ascii_lowercase()) {
                op
            } else if text.chars().all(|c| c.is_ascii_digit()) && !text.is_empty() {
                Token::Num(text)
            } else if text.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                Token::Var(text)
            } else {
                tokens.extend(tokenize(&text)?);
                return Ok(None);
            }
        }
        name if IGNORED_COMMANDS.contains(&name) => return Ok(None),
        _ => return Err(QuantusError::other(msg!("core.logic.unknown_command"))),
    }))
}

fn unknown_character(c: char) -> QuantusError {
    QuantusError::other(msg!("core.logic.unknown_character"))
}

fn skip_spaces(chars: &[char], i: &mut usize) {
    while chars.get(*i) == Some(&' ') {
        *i += 1;
    }
}

/// The argument of `\name` at `*i`: `{…}` or a single character.
fn argument(name: &str, chars: &[char], i: &mut usize) -> Result<String> {
    skip_spaces(chars, i);
    match chars.get(*i) {
        Some('{') => braced(chars, i),
        Some(c) if c.is_ascii_alphanumeric() => {
            *i += 1;
            Ok(c.to_string())
        }
        _ => Err(QuantusError::other(msg!("core.logic.expected_brace"))),
    }
}

/// An index after `_`: one letter or digit, or `{…}` of them.
fn subscript(chars: &[char], i: &mut usize) -> Result<String> {
    let index: String = argument("_", chars, i)?.chars().filter(|c| !c.is_whitespace()).collect();
    if index.is_empty() || !index.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(QuantusError::other(msg!("core.logic.bad_index")));
    }
    Ok(index)
}

/// The content of the `{…}` at `*i`, which moves past it.
fn braced(chars: &[char], i: &mut usize) -> Result<String> {
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
    Err(QuantusError::other(msg!("core.logic.bracket_not_closed")))
}

pub(super) struct Parser<'a> {
    tokens: &'a [Token],
    index: usize,
}

impl<'a> Parser<'a> {
    pub(super) fn new(tokens: &'a [Token]) -> Self {
        Parser { tokens, index: 0 }
    }

    fn peek(&self) -> Option<&'a Token> {
        self.tokens.get(self.index)
    }

    fn eat(&mut self, token: &Token) -> bool {
        let found = self.peek() == Some(token);
        if found {
            self.index += 1;
        }
        found
    }

    /// The whole token list as one expression.
    pub(super) fn whole(&mut self) -> Result<Expr> {
        if self.tokens.is_empty() {
            return Err(QuantusError::other(msg!("core.logic.type_expression")));
        }
        let expr = self.nor()?;
        match self.peek() {
            None => Ok(expr),
            Some(token) => {
                let token = token.text();
                Err(QuantusError::other(msg!("core.logic.unexpected")))
            }
        }
    }

    /// The binary, non-associative levels: NOR over `lower`.
    fn binary(&mut self, operator: &Token, lower: fn(&mut Self) -> Result<Expr>, make: fn(Box<Expr>, Box<Expr>) -> Expr) -> Result<Expr> {
        let left = lower(self)?;
        if !self.eat(operator) {
            return Ok(left);
        }
        let right = lower(self)?;
        if self.peek() == Some(operator) {
            let op = if *operator == Token::Nand { "NAND" } else { "NOR" };
            return Err(QuantusError::other(msg!("core.logic.not_associative")));
        }
        Ok(make(Box::new(left), Box::new(right)))
    }

    fn nor(&mut self) -> Result<Expr> {
        self.binary(&Token::Nor, Self::or, Expr::Nor)
    }

    fn or(&mut self) -> Result<Expr> {
        let mut operands = vec![self.xor()?];
        while self.eat(&Token::Or) {
            operands.push(self.xor()?);
        }
        Ok(if operands.len() == 1 { operands.remove(0) } else { Expr::Or(operands) })
    }

    /// XOR and XNOR, from the left. A chain of XOR becomes one n-ary XOR; XNOR stays binary,
    /// since the n-ary XNOR is even parity.
    fn xor(&mut self) -> Result<Expr> {
        let mut left = self.nand()?;
        let mut chain = false;
        loop {
            if self.eat(&Token::Xor) {
                let right = self.nand()?;
                match &mut left {
                    Expr::Xor(operands) if chain => operands.push(right),
                    _ => left = Expr::Xor(vec![left, right]),
                }
                chain = true;
            } else if self.eat(&Token::Xnor) {
                let right = self.nand()?;
                left = Expr::Xnor(vec![left, right]);
                chain = false;
            } else {
                return Ok(left);
            }
        }
    }

    fn nand(&mut self) -> Result<Expr> {
        self.binary(&Token::Nand, Self::and, Expr::Nand)
    }

    /// AND, written or by juxtaposition.
    fn and(&mut self) -> Result<Expr> {
        let mut operands = vec![self.unary()?];
        loop {
            if self.eat(&Token::And) || self.peek().is_some_and(Token::begins_operand) {
                operands.push(self.unary()?);
            } else {
                break;
            }
        }
        Ok(if operands.len() == 1 { operands.remove(0) } else { Expr::And(operands) })
    }

    fn unary(&mut self) -> Result<Expr> {
        if self.eat(&Token::Not) {
            return Ok(Expr::not(self.unary()?));
        }
        let mut operand = self.primary()?;
        while self.eat(&Token::Prime) {
            operand = Expr::not(operand);
        }
        Ok(operand)
    }

    fn primary(&mut self) -> Result<Expr> {
        let Some(token) = self.peek() else { return Err(QuantusError::other(msg!("core.logic.ends_too_early"))) };
        self.index += 1;
        match token {
            Token::Var(name) => Ok(Expr::Var(name.clone())),
            Token::Num(digits) => match digits.as_str() {
                "0" => Ok(Expr::Const(false)),
                "1" => Ok(Expr::Const(true)),
                number => Err(QuantusError::other(msg!("core.logic.not_binary"))),
            },
            Token::Open => {
                let inner = self.nor()?;
                if !self.eat(&Token::Close) {
                    return Err(QuantusError::other(msg!("core.logic.bracket_not_closed")));
                }
                Ok(inner)
            }
            other => {
                let token = other.text();
                Err(QuantusError::other(msg!("core.logic.missing_operand")))
            }
        }
    }
}
