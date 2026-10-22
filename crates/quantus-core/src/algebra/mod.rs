//! Symbolic algebraic expression simplification engine.

pub mod domain;
pub mod factored;
pub mod poly;
pub mod steps;

pub use domain::{
    collect_raw_denominators, extract_domain_restrictions, format_restrictions_latex,
    format_restrictions_plain, is_constant_zero, solve_denominator_roots, solve_poly_restrictions,
    solve_univariate_rational_roots, DomainRestriction, DomainTracker,
};
pub use factored::{Factor, FactoredPoly};
pub use poly::{Monomial, Poly, Term};
pub use steps::{AlgebraStep, RuleId, SimplificationResult};
