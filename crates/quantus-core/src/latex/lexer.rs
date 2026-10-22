//! Turns LaTeX source into tokens.
//!
//! Two things happen here that later stages rely on:
//! - **Numbers keep their decimal form as text.** `9{,}81`, `9,81` and `9.81` all become the
//!   token text `9.81`, so the value never passes through a float.
//! - **Purely visual markup disappears.** Spacing (`\,`, `\quad`) and the delimiter commands
//!   `\left`/`\right` are dropped, so the parser only ever sees structure.

use crate::{QuantusError, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenType {
    Number,
    /// A run of letters (`kg`) or a Greek-letter command (`\omega`).
    Identifier,
    /// Any other backslash command (`\frac`, `\cdot`, `\sqrt`, …), text including the backslash.
    Command,
    Plus,
    Minus,
    Times,
    Divide,
    Caret,
    Underscore,
    Equals,
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    End,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub kind: TokenType,
    pub text: String,
    /// 0-based character index into the source.
    pub position: usize,
}

const DEGREE: char = '°';

/// `\,` `\;` `\:` `\!` and friends: horizontal spacing, no meaning.
const SPACING_SYMBOLS: [char; 6] = [',', ';', ':', '!', ' ', '\\'];

const IGNORED_COMMANDS: [&str; 7] =
    ["\\left", "\\right", "\\quad", "\\qquad", "\\displaystyle", "\\textstyle", "\\limits"];

/// Greek letters are identifiers: variable names (`\omega_0`) or constants (`\pi`).
const GREEK_COMMANDS: [&str; 37] = [
    "\\alpha", "\\beta", "\\gamma", "\\delta", "\\epsilon", "\\varepsilon", "\\zeta", "\\eta",
    "\\theta", "\\vartheta", "\\iota", "\\kappa", "\\lambda", "\\mu", "\\nu", "\\xi", "\\pi",
    "\\rho", "\\sigma", "\\tau", "\\upsilon", "\\phi", "\\varphi", "\\chi", "\\psi", "\\omega",
    "\\Gamma", "\\Delta", "\\Theta", "\\Lambda", "\\Xi", "\\Pi", "\\Sigma", "\\Upsilon",
    "\\Phi", "\\Psi", "\\Omega",
];

pub fn tokenize(source: &str) -> Result<Vec<Token>> {
    Lexer { source, chars: source.chars().collect(), index: 0 }.tokenize()
}

struct Lexer<'a> {
    source: &'a str,
    chars: Vec<char>,
    index: usize,
}

impl Lexer<'_> {
    fn tokenize(mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace();
            let Some(&c) = self.chars.get(self.index) else {
                tokens.push(Token { kind: TokenType::End, text: String::new(), position: self.index });
                return Ok(tokens);
            };
            if c.is_ascii_digit() {
                tokens.push(self.number());
            } else if c.is_alphabetic() || c == DEGREE {
                tokens.push(self.identifier());
            } else if c == '\\' {
                tokens.extend(self.command()?);
            } else {
                tokens.push(self.symbol()?);
            }
        }
    }

    fn error(&self, reason: String, position: usize) -> QuantusError {
        QuantusError::parse(reason, position, self.source)
    }

    fn at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.index + offset).copied()
    }

    fn skip_whitespace(&mut self) {
        while self.at(0).is_some_and(|c| c.is_whitespace() || c == '~' || c == '&') {
            self.index += 1;
        }
    }

    fn digits(&mut self) -> String {
        let start = self.index;
        while self.at(0).is_some_and(|c| c.is_ascii_digit()) {
            self.index += 1;
        }
        self.chars[start..self.index].iter().collect()
    }

    fn number(&mut self) -> Token {
        let start = self.index;
        let mut text = self.digits();
        if let Some(fraction) = self.fractional_part() {
            text.push('.');
            text.push_str(&fraction);
        }
        Token { kind: TokenType::Number, text, position: start }
    }

    /// Accepts `,5`, `.5` and the LaTeX-typeset German form `{,}5`.
    fn fractional_part(&mut self) -> Option<String> {
        let start = self.index;
        if matches!(self.at(0), Some(',' | '.')) {
            self.index += 1;
        } else if self.at(0) == Some('{') && matches!(self.at(1), Some(',' | '.')) && self.at(2) == Some('}') {
            self.index += 3;
        } else {
            return None;
        }
        if !self.at(0).is_some_and(|c| c.is_ascii_digit()) {
            self.index = start; // a lone separator is not part of the number
            return None;
        }
        Some(self.digits())
    }

    fn identifier(&mut self) -> Token {
        let start = self.index;
        if self.at(0) == Some(DEGREE) {
            self.index += 1;
        }
        while self.at(0).is_some_and(char::is_alphabetic) {
            self.index += 1;
        }
        Token { kind: TokenType::Identifier, text: self.chars[start..self.index].iter().collect(), position: start }
    }

    /// Returns `None` for commands that carry no meaning for us (spacing, delimiters).
    fn command(&mut self) -> Result<Option<Token>> {
        let start = self.index;
        self.index += 1; // the backslash
        let Some(first) = self.at(0) else {
            return Err(self.error("A backslash must be followed by a command name.".into(), start));
        };

        if !first.is_alphabetic() {
            self.index += 1;
            if SPACING_SYMBOLS.contains(&first) {
                return Ok(None);
            }
            return Err(self.error(format!("Unknown command '\\{first}'."), start));
        }

        while self.at(0).is_some_and(char::is_alphabetic) {
            self.index += 1;
        }
        let text: String = self.chars[start..self.index].iter().collect();
        Ok(if IGNORED_COMMANDS.contains(&text.as_str()) {
            None
        } else if GREEK_COMMANDS.contains(&text.as_str()) {
            Some(Token { kind: TokenType::Identifier, text, position: start })
        } else {
            Some(Token { kind: TokenType::Command, text, position: start })
        })
    }

    fn symbol(&mut self) -> Result<Token> {
        let start = self.index;
        let c = self.chars[self.index];
        self.index += 1;
        let kind = match c {
            '+' => TokenType::Plus,
            '-' => TokenType::Minus,
            '*' | '·' | '×' => TokenType::Times,
            '/' => TokenType::Divide,
            '^' => TokenType::Caret,
            '_' => TokenType::Underscore,
            '=' => TokenType::Equals,
            '{' => TokenType::LBrace,
            '}' => TokenType::RBrace,
            '(' => TokenType::LParen,
            ')' => TokenType::RParen,
            '[' => TokenType::LBracket,
            ']' => TokenType::RBracket,
            _ => return Err(self.error(format!("Unexpected character '{c}'."), start)),
        };
        Ok(Token { kind, text: c.to_string(), position: start })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(source: &str) -> Vec<(TokenType, String)> {
        tokenize(source).unwrap().into_iter().map(|t| (t.kind, t.text)).collect()
    }

    #[test]
    fn decimal_separators_all_become_a_point() {
        for source in ["9{,}81", "9,81", "9.81"] {
            assert_eq!(texts(source)[0], (TokenType::Number, "9.81".to_string()), "{source}");
        }
    }

    #[test]
    fn a_lone_separator_is_not_part_of_the_number() {
        let error = tokenize("9.").unwrap_err();
        assert_eq!(error.position(), Some(1), "the number ends at 9, the point is unexpected");
    }

    #[test]
    fn spacing_and_delimiters_vanish() {
        let kinds: Vec<_> = texts("2\\,\\left(3\\right)\\quad~&4").into_iter().map(|t| t.0).collect();
        use TokenType::*;
        assert_eq!(kinds, [Number, LParen, Number, RParen, Number, End]);
    }

    #[test]
    fn greek_commands_are_identifiers_and_others_commands() {
        let tokens = texts("\\omega\\frac");
        assert_eq!(tokens[0], (TokenType::Identifier, "\\omega".to_string()));
        assert_eq!(tokens[1], (TokenType::Command, "\\frac".to_string()));
    }

    #[test]
    fn degree_sign_starts_an_identifier() {
        assert_eq!(texts("°C")[0], (TokenType::Identifier, "°C".to_string()));
    }

    #[test]
    fn positions_are_character_indices() {
        let tokens = tokenize("µm + 2").unwrap();
        assert_eq!(tokens.iter().map(|t| t.position).collect::<Vec<_>>(), [0, 3, 5, 6]);
    }

    #[test]
    fn errors_point_at_the_offending_character() {
        let error = tokenize("2 # 3").unwrap_err();
        assert_eq!(error.position(), Some(2));
        assert_eq!(error.to_string(), "Unexpected character '#'. (at position 3: '# 3')");
        assert_eq!(tokenize("2\\").unwrap_err().position(), Some(1));
        assert!(tokenize("\\#").unwrap_err().to_string().starts_with("Unknown command '\\#'."));
    }
}
