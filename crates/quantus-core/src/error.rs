use std::fmt;

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
}

impl QuantusError {
    pub fn parse(reason: impl Into<String>, position: usize, source: &str) -> Self {
        QuantusError::Parse { reason: reason.into(), position, source: source.to_string() }
    }

    pub fn other(message: impl Into<String>) -> Self {
        QuantusError::Other(message.into())
    }

    /// The position in the input this error points at, if it has one.
    pub fn position(&self) -> Option<usize> {
        match self {
            QuantusError::Parse { position, .. } => Some(*position),
            _ => None,
        }
    }
}

impl fmt::Display for QuantusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QuantusError::Parse { reason, position, source } => {
                write!(f, "{reason} (at position {}: '{}')", position + 1, excerpt(source, *position))
            }
            QuantusError::DimensionMismatch(m)
            | QuantusError::UnresolvedVariable(m)
            | QuantusError::NotReal(m)
            | QuantusError::UnsupportedConversion(m)
            | QuantusError::Other(m) => f.write_str(m),
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
}
