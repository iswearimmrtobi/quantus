//! Evaluation: AST to quantity, variable substitution, and the calculator with its steps.

pub mod calculator;
pub mod derivative;
pub mod evaluator;
pub mod linear_system;
pub mod solver;
pub mod substitution;
pub mod system;
mod system_steps;

pub use calculator::{Calculation, LatexCalculator, Step, StepKind};
pub use evaluator::{Evaluator, Variables};
