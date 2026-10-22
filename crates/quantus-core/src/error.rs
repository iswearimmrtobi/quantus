use std::fmt;

/// Everything that can go wrong between an input string and a result.
///
/// `Parse` carries a 0-based position into the input; the UI shows it 1-based.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QuantusError {
    Parse { reason: String, position: usize },
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
            QuantusError::Parse { reason, position } => {
                write!(f, "{reason} (at position {})", position + 1)
            }
            QuantusError::DimensionMismatch(m)
            | QuantusError::UnresolvedVariable(m)
            | QuantusError::NotReal(m)
            | QuantusError::UnsupportedConversion(m)
            | QuantusError::Other(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for QuantusError {}

pub type Result<T> = std::result::Result<T, QuantusError>;
