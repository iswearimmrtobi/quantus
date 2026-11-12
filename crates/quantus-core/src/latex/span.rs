//! Finds the stretch of LaTeX source a node or a token came from, so an error can point at it.
//!
//! The nodes only know where their first token starts; [`resolve`] takes the starts of the
//! first and the last token of a node and widens that to whole source: to the end of the last
//! token, over the brackets and braces opened inside, and over the arguments of a trailing
//! command (`\mathrm{m}`) or subscript (`v_{12}`).

use super::lexer::{tokenize, Token, TokenType};
use crate::error::Span;

/// The stretch from the token starting at `first` to the end of the one at `last`, with the
/// brackets opened in between closed. `None` when `source` does not lex or the positions are
/// not in it.
pub fn resolve(source: &str, first: usize, last: usize) -> Option<Span> {
    let tokens = tokenize(source).ok()?;
    let tokens = &tokens[..tokens.len() - 1]; // the End token
    if first > last {
        return None;
    }
    let first_index = tokens.iter().position(|t| t.position <= first && first < t.end)?;
    let mut last_index = tokens.iter().rposition(|t| t.position <= last && last < t.end)?;
    if last_index < first_index {
        return None;
    }
    let mut start = tokens[first_index].position;
    // `\left(` begins at `\left`, which the lexer drops.
    start = before_left(source, start);

    // Close what was opened inside.
    let mut depth: i32 = 0;
    let mut pipes = 0;
    for token in &tokens[first_index..=last_index] {
        depth += nesting(token);
        if is_plain_pipe(token) {
            pipes += 1;
        }
    }
    while (depth > 0 || pipes % 2 == 1) && last_index + 1 < tokens.len() {
        last_index += 1;
        let token = &tokens[last_index];
        depth += nesting(token);
        if is_plain_pipe(token) {
            pipes += 1;
        }
    }
    // A command's arguments (`\mathrm{m}`, `\sqrt{2}`) and a subscript (`v_1`, `v_{12}`).
    loop {
        let next = last_index + 1;
        match tokens.get(next).map(|t| t.kind) {
            Some(TokenType::LBrace | TokenType::LBracket) if tokens[last_index].kind == TokenType::Command => {
                last_index = closing(tokens, next)?;
            }
            Some(TokenType::Underscore) if tokens[last_index].kind == TokenType::Identifier => {
                let argument = next + 1;
                last_index = match tokens.get(argument)?.kind {
                    TokenType::LBrace => closing(tokens, argument)?,
                    _ => argument,
                };
            }
            _ => break,
        }
    }
    // `{x-1}` as a whole argument is marked without its braces.
    if tokens[first_index].kind == TokenType::LBrace && closing(tokens, first_index) == Some(last_index) && first_index < last_index {
        return Some(Span { start: tokens[first_index].end, end: tokens[last_index].position });
    }
    Some(Span { start, end: tokens[last_index].end })
}

/// The token at `position`, for an error the parser found there; an empty span at the end of
/// the input when something is missing there.
pub fn token_at(source: &str, position: usize) -> Span {
    if let Ok(tokens) = tokenize(source) {
        if let Some(token) = tokens.iter().find(|t| t.kind != TokenType::End && t.position <= position && position < t.end) {
            return Span { start: token.position, end: token.end };
        }
    }
    // The lexer itself failed here: the character, or the whole unknown command.
    let chars: Vec<char> = source.chars().collect();
    if position >= chars.len() {
        return Span { start: chars.len(), end: chars.len() };
    }
    let mut end = position + 1;
    if chars[position] == '\\' {
        while chars.get(end).is_some_and(|c| c.is_alphabetic()) {
            end += 1;
        }
    }
    Span { start: position, end }
}

fn nesting(token: &Token) -> i32 {
    match token.kind {
        TokenType::LBrace | TokenType::LParen | TokenType::LBracket => 1,
        TokenType::RBrace | TokenType::RParen | TokenType::RBracket => -1,
        TokenType::Pipe if token.text == "\\left|" => 1,
        TokenType::Pipe if token.text == "\\right|" => -1,
        _ => 0,
    }
}

fn is_plain_pipe(token: &Token) -> bool {
    token.kind == TokenType::Pipe && token.text == "|"
}

/// The index of the token that closes the bracket at `open`.
fn closing(tokens: &[Token], open: usize) -> Option<usize> {
    let mut depth = 0;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        depth += nesting(token);
        if depth == 0 {
            return Some(index);
        }
    }
    None
}

/// `start`, or the start of a `\left` right before it.
fn before_left(source: &str, start: usize) -> usize {
    let before: String = source.chars().take(start).collect();
    let trimmed = before.trim_end();
    match trimmed.strip_suffix("\\left") {
        Some(rest) => rest.chars().count(),
        None => start,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The text of the span from the first token starting with `first` to the last one
    /// starting with `last`.
    fn text(source: &str, first: usize, last: usize) -> String {
        let span = resolve(source, first, last).expect("resolves");
        source.chars().skip(span.start).take(span.end - span.start).collect()
    }

    #[test]
    fn a_token_runs_to_its_end() {
        assert_eq!(text("2 + 9{,}81", 4, 4), "9{,}81");
        assert_eq!(text("x + \\alpha", 4, 4), "\\alpha");
    }

    #[test]
    fn brackets_opened_inside_are_closed() {
        assert_eq!(text("\\frac{1}{x-1} + 2", 0, 11), "\\frac{1}{x-1}");
        assert_eq!(text("2\\left(x+1\\right)", 6, 9), "\\left(x+1\\right)");
        assert_eq!(text("\\sin(x)", 0, 5), "\\sin(x)");
        assert_eq!(text("|x-1| + 2", 0, 3), "|x-1|");
        assert_eq!(text("\\frac{1}{x-1}", 8, 11), "x-1");
    }

    #[test]
    fn arguments_and_subscripts_belong_to_their_token() {
        assert_eq!(text("3\\,\\mathrm{m}", 3, 3), "\\mathrm{m}");
        assert_eq!(text("v_{12} + 1", 0, 0), "v_{12}");
        assert_eq!(text("v_1 + 1", 0, 0), "v_1");
    }

    #[test]
    fn a_sum_of_units_runs_over_both() {
        assert_eq!(text("1\\,\\mathrm{m} + 2\\,\\mathrm{s}", 0, 25), "1\\,\\mathrm{m} + 2\\,\\mathrm{s}");
    }

    #[test]
    fn positions_outside_the_source_resolve_to_nothing() {
        assert_eq!(resolve("1 + 2", 0, 9), None);
        assert_eq!(resolve("1 + 2", 4, 0), None);
    }

    #[test]
    fn a_parse_error_marks_its_token_or_the_end() {
        assert_eq!(token_at("2 \\cdot mgh", 8), Span { start: 8, end: 11 });
        assert_eq!(token_at("2 +", 3), Span { start: 3, end: 3 });
        assert_eq!(token_at("2 + \\foo", 4), Span { start: 4, end: 8 });
        assert_eq!(token_at("2 + #", 4), Span { start: 4, end: 5 });
    }
}
