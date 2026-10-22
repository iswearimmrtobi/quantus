//! Binomial formulas in both directions and completing the square.
//!
//! - Compact input (`(a + b)^2`, `(a - b)(a + b)`) is expanded.
//! - Any other input whose result is a polynomial is written in binomial form when one of the
//!   three binomial formulas applies.
//! - An input that spells out a double product `2 \cdot A \cdot B` has its square completed.

use quantus_core::algebra::steps::{RuleId, SimplificationResult};
use quantus_core::algebra::simplify;
use quantus_core::algebra::verify::verify_equivalence;
use quantus_core::latex::{parser, serializer};

fn run(input: &str) -> SimplificationResult {
    simplify(input).unwrap_or_else(|e| panic!("Simplification failed for '{input}': {e}"))
}

/// Simplifies `input`, checks the result against `expected` and its equivalence to the input.
fn assert_simplifies(input: &str, expected: &str) -> SimplificationResult {
    let result = run(input);
    assert_eq!(serializer::algebraic_expression(&result.node), expected, "input: {input}");
    assert!(result.diagnostic.is_none(), "unexpected diagnostic for '{input}': {:?}", result.diagnostic);
    let original = parser::parse(input).unwrap().expression;
    let verification = verify_equivalence(&original, &result.node, &result.domain_restrictions, 5);
    assert!(verification.is_equivalent(), "not equivalent for '{input}': {verification:?}");
    result
}

fn last_rule(result: &SimplificationResult) -> Option<RuleId> {
    result.steps.last().and_then(|step| step.rule)
}

#[test]
fn expanded_square_of_sum_is_compressed() {
    let result = assert_simplifies(r"a^2 + 2 \cdot a \cdot b + b^2", "(a + b)^{2}");
    assert_eq!(last_rule(&result), Some(RuleId::ApplyBinomialIdentity));
    assert_simplifies(r"x^2 + 6x + 9", "(x + 3)^{2}");
}

#[test]
fn expanded_square_of_difference_is_compressed() {
    assert_simplifies(r"a^2 - 2 \cdot a \cdot b + b^2", "(a - b)^{2}");
    assert_simplifies(r"a^2 - 2ab + b^2", "(a - b)^{2}");
    assert_simplifies(r"4x^2 - 12x + 9", "(2x - 3)^{2}");
}

#[test]
fn difference_of_squares_is_compressed() {
    let result = assert_simplifies(r"x^2 - 9", "(x - 3)(x + 3)");
    assert_eq!(last_rule(&result), Some(RuleId::ApplyBinomialIdentity));
    assert_simplifies(r"2x^2 - 18", r"2 \cdot (x - 3)(x + 3)");
}

#[test]
fn compact_input_is_expanded() {
    assert_simplifies(r"(a + b)^2", r"a^{2} + 2 \cdot a \cdot b + b^{2}");
    assert_simplifies(r"(x - 4)^2", "x^{2} - 8x + 16");
    assert_simplifies(r"(x - 3)(x + 3)", "x^{2} - 9");
    assert_simplifies(r"3(x + 1)^2", "3x^{2} + 6x + 3");
    assert_simplifies(r"(x + 4)^2 - 5", "x^{2} + 8x + 11");
}

#[test]
fn explicit_double_product_completes_the_square() {
    let result = assert_simplifies(r"x^2 + 2 \cdot x \cdot 4 + 11", "(x + 4)^{2} - 5");
    assert_eq!(last_rule(&result), Some(RuleId::CompleteSquare));
    assert_simplifies(r"x^2 - 2 \cdot x \cdot 3 + 1", "(x - 3)^{2} - 8");
    assert_simplifies(r"a^2 + 2 \cdot a \cdot b + 3b^2", r"(a + b)^{2} + 2b^{2}");
}

#[test]
fn quadratics_without_double_product_stay_expanded() {
    let result = run(r"x^2 + 8x + 11");
    assert_eq!(serializer::algebraic_expression(&result.node), "x^{2} + 8x + 11");
    assert!(result.steps.is_empty());
    assert_simplifies(r"3x^2 - 18x + 24 + x - 102", "3x^{2} - 17x - 78");
}

#[test]
fn fractions_keep_their_canonical_form() {
    assert_simplifies(r"\frac{x^2 + 2x + 1}{x - 3}", r"\frac{x^{2} + 2x + 1}{x - 3}");
}
