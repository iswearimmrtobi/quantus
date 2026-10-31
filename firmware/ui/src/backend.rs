//! What computes the results. On the P4 this will be `quantus-core`; until then a stub.

use crate::app::Mode;

pub trait Backend {
    /// The result of `latex` in `mode`, as LaTeX, or an error message.
    fn evaluate(&mut self, mode: Mode, latex: &str) -> Result<String, String>;
}

/// Answers with the input itself, so the way back from LaTeX shows on the screen. An input with
/// a `?` gives an error.
pub struct Stub;

impl Backend for Stub {
    fn evaluate(&mut self, _mode: Mode, latex: &str) -> Result<String, String> {
        if latex.contains('?') {
            Err("Syntax error".to_string())
        } else {
            Ok(latex.to_string())
        }
    }
}
