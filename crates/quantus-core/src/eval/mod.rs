//! Evaluation: AST to quantity, variable substitution, and the calculator with its steps.

pub mod evaluator;
pub mod substitution;

pub use evaluator::{Evaluator, Variables};
