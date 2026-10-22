//! Evaluation: AST to quantity, variable substitution, and the calculator with its steps.

pub mod calculator;
pub mod evaluator;
pub mod substitution;

pub use calculator::{Calculation, LatexCalculator, Step, StepKind};
pub use evaluator::{Evaluator, Variables};
