//! Symbolic algebraic expression simplification engine.

pub mod binomial;
pub mod domain;
pub mod factor;
pub mod factored;
pub mod poly;
pub mod steps;

pub use binomial::{
    check_difference_of_squares, check_perfect_cube, check_perfect_square_trinomial,
    check_sum_or_diff_of_cubes, exact_ibig_cbrt, exact_rational_cbrt, exact_rational_sqrt,
    exact_ubig_cbrt, exact_ubig_sqrt, expand_binomial_node, expand_binomial_poly,
    expand_difference_of_squares_product_poly, factor_binomial, factor_binomial_node,
    BinomialIdentity,
};
pub use domain::{
    collect_raw_denominators, extract_domain_restrictions, format_restrictions_latex,
    format_restrictions_plain, is_constant_zero, solve_denominator_roots, solve_poly_restrictions,
    solve_univariate_rational_roots, DomainRestriction, DomainTracker,
};
pub use factor::{factor_polynomial, factor_polynomial_node};
pub use factored::{Factor, FactoredPoly};
pub use poly::{Monomial, Poly, Term};
pub use steps::{AlgebraStep, RuleId, SimplificationResult};
