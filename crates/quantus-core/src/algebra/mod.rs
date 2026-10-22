//! Symbolic algebraic expression simplification engine.

pub mod domain;
pub mod factored;
pub mod poly;
pub mod steps;

pub use domain::{format_restrictions_latex, format_restrictions_plain, DomainRestriction};
pub use factored::{Factor, FactoredPoly};
pub use poly::{Monomial, Poly, Term};
pub use steps::{AlgebraStep, RuleId, SimplificationResult};
