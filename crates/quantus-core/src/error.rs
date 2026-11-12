use std::fmt;

use serde::{Deserialize, Serialize};

/// A stretch of the input, as 0-based character indices: `start` is the first character,
/// `end` the one after the last.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

/// Everything that can go wrong between an input string and a result.
///
/// `Parse` carries a 0-based character position into the input `source`; the UI shows it
/// 1-based, and the message quotes a short excerpt starting there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QuantusError {
    Parse { reason: String, position: usize, source: String },
    /// Two quantities do not fit together dimensionally.
    DimensionMismatch(String),
    /// A variable was used before the session defined it.
    UnresolvedVariable(String),
    /// A value was expected to be real (e.g. a complex result cannot be shown as a decimal).
    NotReal(String),
    /// A conversion that cannot be expressed as `factor * x + offset`.
    UnsupportedConversion(String),
    Other(String),
    /// Another error and the part of the input it is about.
    ///
    /// While the error travels up from the evaluator, `exact` is false and the span holds the
    /// starts of the first and the last token of the node it is about (the nodes know nothing
    /// more); [`Self::resolve`] turns that into the whole stretch of source.
    Located { error: Box<QuantusError>, span: Span, exact: bool },
}

impl QuantusError {
    pub fn parse(reason: impl Into<String>, position: usize, source: &str) -> Self {
        QuantusError::Parse { reason: reason.into(), position, source: source.to_string() }
    }

    pub fn other(message: impl Into<String>) -> Self {
        QuantusError::Other(message.into())
    }

    /// The error with the part of the input it is about, unless it already points somewhere:
    /// the innermost place wins.
    pub fn at(self, span: Span) -> Self {
        match self {
            located @ (QuantusError::Located { .. } | QuantusError::Parse { .. }) => located,
            error => QuantusError::Located { error: Box::new(error), span, exact: true },
        }
    }

    /// Like [`Self::at`], for the node of the input the error is about. A node a rewrite built
    /// points nowhere, and the error stays as it is.
    pub fn at_node(self, node: &crate::latex::Node) -> Self {
        match (&self, node.extent()) {
            (QuantusError::Located { .. } | QuantusError::Parse { .. }, _) | (_, None) => self,
            (_, Some((first, last))) => {
                QuantusError::Located { error: Box::new(self), span: Span { start: first, end: last }, exact: false }
            }
        }
    }

    /// The error without the place it points at.
    pub fn unlocated(&self) -> &QuantusError {
        match self {
            QuantusError::Located { error, .. } => error.unlocated(),
            error => error,
        }
    }

    /// The error without the place it points at, by value.
    pub fn into_unlocated(self) -> QuantusError {
        match self {
            QuantusError::Located { error, .. } => error.into_unlocated(),
            error => error,
        }
    }

    /// Turns the places this error points at into stretches of `source`, the input the error
    /// came from. A place that cannot be found in it is dropped.
    pub fn resolve(self, input: &str) -> Self {
        match self {
            QuantusError::Located { exact: true, .. } => self,
            QuantusError::Located { error, span, exact: false } => {
                match crate::latex::span::resolve(input, span.start, span.end) {
                    Some(span) => QuantusError::Located { error, span, exact: true },
                    None => *error,
                }
            }
            QuantusError::Parse { ref source, position, .. } if source == input => {
                let span = crate::latex::span::token_at(input, position);
                QuantusError::Located { error: Box::new(self), span, exact: true }
            }
            error => error,
        }
        .checked(input)
    }

    /// Drops a place that lies outside `source`.
    fn checked(self, source: &str) -> Self {
        match self {
            QuantusError::Located { error, span, .. } if span.end > source.chars().count() || span.start > span.end => {
                *error
            }
            error => error,
        }
    }

    /// The position in the input this error points at, if it has one.
    pub fn position(&self) -> Option<usize> {
        match self {
            QuantusError::Parse { position, .. } => Some(*position),
            QuantusError::Located { span, exact: true, .. } => Some(span.start),
            QuantusError::Located { error, .. } => error.position(),
            _ => None,
        }
    }

    /// The stretch of the input this error is about, once [resolved](Self::resolve).
    pub fn span(&self) -> Option<Span> {
        match self {
            QuantusError::Located { span, exact: true, .. } => Some(*span),
            _ => None,
        }
    }

    /// The message alone, without the position a parse error adds for a reader who sees
    /// no marking.
    pub fn reason(&self) -> String {
        match self.unlocated() {
            QuantusError::Parse { reason, .. } => reason.clone(),
            error => error.to_string(),
        }
    }
}

impl fmt::Display for QuantusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QuantusError::Parse { reason, position, source } => {
                write!(f, "{reason} (at position {}: '{}')", position.saturating_add(1), excerpt(source, *position))
            }
            QuantusError::DimensionMismatch(m)
            | QuantusError::UnresolvedVariable(m)
            | QuantusError::NotReal(m)
            | QuantusError::UnsupportedConversion(m)
            | QuantusError::Other(m) => f.write_str(m),
            QuantusError::Located { error, .. } => error.fmt(f),
        }
    }
}

/// A short window of up to 12 characters from `position`, so the message is self-explanatory.
fn excerpt(source: &str, position: usize) -> String {
    let chars: Vec<char> = source.chars().collect();
    if chars.is_empty() {
        return String::new();
    }
    let start = position.min(chars.len());
    let end = (start + 12).min(chars.len());
    let snippet: String = chars[start..end].iter().collect();
    if end < chars.len() {
        format!("{snippet}…")
    } else {
        snippet
    }
}

impl std::error::Error for QuantusError {}

pub type Result<T> = std::result::Result<T, QuantusError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_message_quotes_an_excerpt() {
        let error = QuantusError::parse("Bad.", 2, "2 +");
        assert_eq!(error.to_string(), "Bad. (at position 3: '+')");
        assert_eq!(error.position(), Some(2));
    }

    #[test]
    fn long_inputs_are_cut_with_an_ellipsis() {
        let error = QuantusError::parse("Bad.", 0, "1234567890123456");
        assert_eq!(error.to_string(), "Bad. (at position 1: '123456789012…')");
    }

    #[test]
    fn empty_source_and_position_past_the_end_give_an_empty_excerpt() {
        assert_eq!(QuantusError::parse("Bad.", 0, "").to_string(), "Bad. (at position 1: '')");
        assert_eq!(QuantusError::parse("Bad.", 5, "ab").to_string(), "Bad. (at position 6: '')");
    }

    #[test]
    fn the_innermost_place_wins() {
        let inner = Span { start: 2, end: 3 };
        let error = QuantusError::other("Bad.").at(inner).at(Span { start: 0, end: 5 });
        assert_eq!(error.span(), Some(inner));
        assert_eq!(error.to_string(), "Bad.");
        assert_eq!(error.unlocated(), &QuantusError::other("Bad."));
    }

    #[test]
    fn a_parse_error_points_at_its_token() {
        let error = QuantusError::parse("Bad.", 4, "2 + abc").resolve("2 + abc");
        assert_eq!(error.span(), Some(Span { start: 4, end: 7 }));
        assert_eq!(error.position(), Some(4));
        assert_eq!(error.reason(), "Bad.");
    }

    #[test]
    fn a_parse_error_from_another_source_is_left_alone() {
        let error = QuantusError::parse("Bad.", 4, "2 + abc").resolve("x");
        assert_eq!(error.span(), None);
        assert_eq!(error.position(), Some(4));
    }
}
