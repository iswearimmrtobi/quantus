//! Step 7.2: Comprehensive Unit Test Suite
//!
//! Validates all 14 required algebraic simplification categories:
//! 1. Simplifying nested fractions (single, double, deep nesting, sums inside nested fractions).
//! 2. Adding and subtracting fractions with identical denominators.
//! 3. Adding and subtracting fractions with different denominators.
//! 4. Finding the least common denominator with repeated polynomial factors.
//! 5. Expanding binomial squares and cubes.
//! 6. Recognizing differences of squares.
//! 7. Factoring quadratic polynomials (monic, non-monic, perfect-square trinomials, irreducible).
//! 8. Combining like terms and canonical descending degree sorting.
//! 9. Canceling common factors safely (preventing false cancellations like (x+2)/x -> 2).
//! 10. Preserving domain restrictions after cancellation.
//! 11. Handling negative coefficients, subtraction of fractions, and nested parentheses.
//! 12. Handling constant expressions and zero numerators.
//! 13. Detecting unsupported expressions gracefully without returning incorrect results.
//! 14. Verifying mathematical equivalence across all valid test cases.

use quantus_core::algebra::domain::format_restrictions_plain;
use quantus_core::algebra::steps::{RuleId, SimplificationResult};
use quantus_core::algebra::verify::verify_equivalence;
use quantus_core::algebra::simplify;
use quantus_core::latex::ast::Node;
use quantus_core::latex::{parser, serializer};

fn parse(input: &str) -> Node {
    parser::parse(input)
        .unwrap_or_else(|e| panic!("Parse error for '{input}': {e}"))
        .expression
}

fn run(input: &str) -> SimplificationResult {
    simplify(input).unwrap_or_else(|e| panic!("Simplification failed for '{input}': {e}"))
}

fn restrictions(result: &SimplificationResult) -> String {
    format_restrictions_plain(&result.domain_restrictions)
}

/// Helper asserting that result is mathematically equivalent to the parsed input.
fn assert_equivalent(input: &str, result: &SimplificationResult) {
    let orig = parse(input);
    let vres = verify_equivalence(&orig, &result.node, &result.domain_restrictions, 5);
    assert!(
        vres.is_equivalent(),
        "Mathematical equivalence failed for '{input}': got {vres:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// Category 1: Simplifying nested fractions
// ---------------------------------------------------------------------------------------------

#[test]
fn cat1_nested_fractions_single_level() {
    // 1 / (1 / x) -> x
    let res = run(r"\frac{1}{\frac{1}{x}}");
    assert_eq!(serializer::algebraic_expression(&res.node), "x");
    assert_eq!(restrictions(&res), "x != 0");
    assert_equivalent(r"\frac{1}{\frac{1}{x}}", &res);

    // x / ((x + 1) / (x - 2)) -> (x^2 - 2x) / (x + 1)
    let res = run(r"\frac{x}{\frac{x + 1}{x - 2}}");
    assert_eq!(
        serializer::algebraic_expression(&res.node),
        r"\frac{x^{2} - 2x}{x + 1}"
    );
    assert_eq!(restrictions(&res), "x != -1, 2");
    assert_equivalent(r"\frac{x}{\frac{x + 1}{x - 2}}", &res);
}

#[test]
fn cat1_nested_fractions_numerator_nested() {
    // ((x - 1) / (x + 2)) / (x - 3) -> (x - 1) / ((x + 2)(x - 3))
    let res = run(r"\frac{\frac{x - 1}{x + 2}}{x - 3}");
    assert_eq!(
        serializer::algebraic_expression(&res.node),
        r"\frac{x - 1}{(x - 3)(x + 2)}"
    );
    assert_eq!(restrictions(&res), "x != -2, 3");
    assert_equivalent(r"\frac{\frac{x - 1}{x + 2}}{x - 3}", &res);
}

#[test]
fn cat1_nested_fractions_deep_and_double_nesting() {
    // 1 / (1 / (1 / x)) -> 1 / x
    let res = run(r"\frac{1}{\frac{1}{\frac{1}{x}}}");
    assert_eq!(serializer::algebraic_expression(&res.node), r"\frac{1}{x}");
    assert_eq!(restrictions(&res), "x != 0");
    assert_equivalent(r"\frac{1}{\frac{1}{\frac{1}{x}}}", &res);

    // Product nested in denominator: 1 / (x * (1 / (x + 1))) -> (x + 1) / x
    let res = run(r"\frac{1}{x \cdot \frac{1}{x + 1}}");
    assert_eq!(serializer::algebraic_expression(&res.node), r"\frac{x + 1}{x}");
    assert_eq!(restrictions(&res), "x != -1, 0");
    assert_equivalent(r"\frac{1}{x \cdot \frac{1}{x + 1}}", &res);
}

#[test]
fn cat1_nested_fractions_sums_inside() {
    // (1 + 1/x) / (1 - 1/x) -> (x + 1) / (x - 1)
    let res = run(r"\frac{1 + \frac{1}{x}}{1 - \frac{1}{x}}");
    assert_eq!(
        serializer::algebraic_expression(&res.node),
        r"\frac{x + 1}{x - 1}"
    );
    assert_eq!(restrictions(&res), "x != 0, 1");
    assert_equivalent(r"\frac{1 + \frac{1}{x}}{1 - \frac{1}{x}}", &res);
}

#[test]
fn cat1_triple_nested_fraction_4th_order() {
    // 3-level nested fraction with 4th order polynomial numerator:
    // (((x^4 - 16)/(x - 1))/(x + 2)) / ((x^2 + 4)/(x^2 - 1)) -> x^2 - x - 2
    let input = r"\frac{\frac{\frac{x^4 - 16}{x - 1}}{x + 2}}{\frac{x^2 + 4}{x^2 - 1}}";
    let res = run(input);
    assert_eq!(serializer::algebraic_expression(&res.node), "x^{2} - x - 2");
    assert_eq!(restrictions(&res), "x != -2, -1, 1, x^2 + 4 != 0");
    assert_equivalent(input, &res);
}

// ---------------------------------------------------------------------------------------------
// Category 2: Adding and subtracting fractions with identical denominators
// ---------------------------------------------------------------------------------------------

#[test]
fn cat2_identical_denominators_addition() {
    // x/(x + 1) + 2/(x + 1) -> (x + 2)/(x + 1)
    let res = run(r"\frac{x}{x + 1} + \frac{2}{x + 1}");
    assert_eq!(
        serializer::algebraic_expression(&res.node),
        r"\frac{x + 2}{x + 1}"
    );
    assert_eq!(restrictions(&res), "x != -1");
    assert_equivalent(r"\frac{x}{x + 1} + \frac{2}{x + 1}", &res);
}

#[test]
fn cat2_identical_denominators_subtraction() {
    // (3x + 5)/(x - 2) - (2x + 1)/(x - 2) -> (x + 4)/(x - 2)
    let res = run(r"\frac{3x + 5}{x - 2} - \frac{2x + 1}{x - 2}");
    assert_eq!(
        serializer::algebraic_expression(&res.node),
        r"\frac{x + 4}{x - 2}"
    );
    assert_eq!(restrictions(&res), "x != 2");
    assert_equivalent(r"\frac{3x + 5}{x - 2} - \frac{2x + 1}{x - 2}", &res);
}

#[test]
fn cat2_identical_denominators_cancels_to_polynomial() {
    // x^2/(x + 3) + 5x/(x + 3) + 6/(x + 3) -> (x^2 + 5x + 6)/(x + 3) = x + 2
    let res = run(r"\frac{x^2}{x + 3} + \frac{5x}{x + 3} + \frac{6}{x + 3}");
    assert_eq!(serializer::algebraic_expression(&res.node), "x + 2");
    assert_eq!(restrictions(&res), "x != -3");
    assert_equivalent(r"\frac{x^2}{x + 3} + \frac{5x}{x + 3} + \frac{6}{x + 3}", &res);
}

// ---------------------------------------------------------------------------------------------
// Category 3: Adding and subtracting fractions with different denominators
// ---------------------------------------------------------------------------------------------

#[test]
fn cat3_different_denominators_monomials() {
    // 1/x + 1/z -> (x + z) / (x z)
    let res = run(r"\frac{1}{x} + \frac{1}{z}");
    assert_eq!(restrictions(&res), "x != 0, z != 0");
    assert_equivalent(r"\frac{1}{x} + \frac{1}{z}", &res);
}

#[test]
fn cat3_different_denominators_linear_binomials() {
    // 1/(x - 1) + 1/(x + 1) -> 2x / ((x - 1)(x + 1))
    let res = run(r"\frac{1}{x - 1} + \frac{1}{x + 1}");
    assert_eq!(
        serializer::algebraic_expression(&res.node),
        r"\frac{2x}{(x - 1)(x + 1)}"
    );
    assert_eq!(restrictions(&res), "x != -1, 1");
    assert_equivalent(r"\frac{1}{x - 1} + \frac{1}{x + 1}", &res);

    // 2/(x + 2) - 1/(x - 1) -> (2(x-1) - (x+2)) / ((x+2)(x-1)) = (x - 4) / ((x-1)(x+2))
    let res = run(r"\frac{2}{x + 2} - \frac{1}{x - 1}");
    assert_eq!(
        serializer::algebraic_expression(&res.node),
        r"\frac{x - 4}{(x - 1)(x + 2)}"
    );
    assert_eq!(restrictions(&res), "x != -2, 1");
    assert_equivalent(r"\frac{2}{x + 2} - \frac{1}{x - 1}", &res);
}

// ---------------------------------------------------------------------------------------------
// Category 4: Finding LCD with repeated polynomial factors
// ---------------------------------------------------------------------------------------------

#[test]
fn cat4_lcd_repeated_polynomial_factors() {
    // 1/(x + 1)^2 + 1/((x + 1)(x - 1)) -> ((x - 1) + (x + 1)) / ((x - 1)(x + 1)^2) = 2x / ((x - 1)(x + 1)^2)
    let res = run(r"\frac{1}{(x + 1)^2} + \frac{1}{(x + 1) \cdot (x - 1)}");
    assert_eq!(
        serializer::algebraic_expression(&res.node),
        r"\frac{2x}{(x - 1)(x + 1)^{2}}"
    );
    assert_eq!(restrictions(&res), "x != -1, 1");
    assert_equivalent(r"\frac{1}{(x + 1)^2} + \frac{1}{(x + 1) \cdot (x - 1)}", &res);
}

#[test]
fn cat4_lcd_repeated_powers_of_variable() {
    // 1/x^2 + 1/x^3 -> (x + 1) / x^3
    let res = run(r"\frac{1}{x^2} + \frac{1}{x^3}");
    assert_eq!(
        serializer::algebraic_expression(&res.node),
        r"\frac{x + 1}{x^{3}}"
    );
    assert_eq!(restrictions(&res), "x != 0");
    assert_equivalent(r"\frac{1}{x^2} + \frac{1}{x^3}", &res);
}

// ---------------------------------------------------------------------------------------------
// Category 5: Expanding binomial squares and cubes
// ---------------------------------------------------------------------------------------------

#[test]
fn cat5_expanding_binomial_squares() {
    // (x + 3)^2 -> x^2 + 6x + 9
    let res = run(r"(x + 3)^2");
    assert_eq!(serializer::algebraic_expression(&res.node), "x^{2} + 6x + 9");
    assert_equivalent(r"(x + 3)^2", &res);

    // (x - 4)^2 -> x^2 - 8x + 16
    let res = run(r"(x - 4)^2");
    assert_eq!(serializer::algebraic_expression(&res.node), "x^{2} - 8x + 16");
    assert_equivalent(r"(x - 4)^2", &res);

    // Multivariate: (x + z)^2 -> x^2 + 2xz + z^2
    let res = run(r"(x + z)^2");
    assert_equivalent(r"(x + z)^2", &res);

    // Textbook variables: (a + b)^2 and (a - b)^2
    let res = run(r"(a + b)^2");
    assert_equivalent(r"(a + b)^2", &res);

    let res = run(r"(a - b)^2");
    assert_equivalent(r"(a - b)^2", &res);
}

#[test]
fn cat5_expanding_binomial_cubes() {
    // (a + b)^3 -> a^3 + 3a^2 b + 3a b^2 + b^3
    let res = run(r"(a + b)^3");
    assert_equivalent(r"(a + b)^3", &res);
    // (x + 2)^3 -> x^3 + 6x^2 + 12x + 8
    let res = run(r"(x + 2)^3");
    assert_eq!(
        serializer::algebraic_expression(&res.node),
        "x^{3} + 6x^{2} + 12x + 8"
    );
    assert_equivalent(r"(x + 2)^3", &res);

    // (x - 1)^3 -> x^3 - 3x^2 + 3x - 1
    let res = run(r"(x - 1)^3");
    assert_eq!(
        serializer::algebraic_expression(&res.node),
        "x^{3} - 3x^{2} + 3x - 1"
    );
    assert_equivalent(r"(x - 1)^3", &res);
}

// ---------------------------------------------------------------------------------------------
// Category 6: Recognizing differences of squares
// ---------------------------------------------------------------------------------------------

#[test]
fn cat6_difference_of_squares_cancellation() {
    // (x^2 - 9) / (x - 3) -> x + 3
    let res = run(r"\frac{x^2 - 9}{x - 3}");
    assert_eq!(serializer::algebraic_expression(&res.node), "x + 3");
    assert_eq!(restrictions(&res), "x != 3");
    assert_equivalent(r"\frac{x^2 - 9}{x - 3}", &res);

    // (4x^2 - 25) / (2x - 5) -> 2x + 5
    let res = run(r"\frac{4x^2 - 25}{2x - 5}");
    assert_eq!(serializer::algebraic_expression(&res.node), "2x + 5");
    assert_eq!(restrictions(&res), "x != 5/2");
    assert_equivalent(r"\frac{4x^2 - 25}{2x - 5}", &res);
}

#[test]
fn cat6_difference_of_squares_fractions_combination() {
    // 1/(x - 3) - 1/(x + 3) -> 6 / ((x - 3)(x + 3))
    let res = run(r"\frac{1}{x - 3} - \frac{1}{x + 3}");
    assert_eq!(
        serializer::algebraic_expression(&res.node),
        r"\frac{6}{(x - 3)(x + 3)}"
    );
    assert_eq!(restrictions(&res), "x != -3, 3");
    assert_equivalent(r"\frac{1}{x - 3} - \frac{1}{x + 3}", &res);
}

// ---------------------------------------------------------------------------------------------
// Category 7: Factoring quadratic polynomials
// ---------------------------------------------------------------------------------------------

#[test]
fn cat7_factoring_monic_quadratics() {
    // (x^2 - 5x + 6) / (x - 2) -> x - 3
    let res = run(r"\frac{x^2 - 5x + 6}{x - 2}");
    assert_eq!(serializer::algebraic_expression(&res.node), "x - 3");
    assert_eq!(restrictions(&res), "x != 2");
    assert_equivalent(r"\frac{x^2 - 5x + 6}{x - 2}", &res);
}

#[test]
fn cat7_factoring_non_monic_quadratics() {
    // (2x^2 + 5x + 2) / (x + 2) = (2x + 1)(x + 2) / (x + 2) -> 2x + 1
    let res = run(r"\frac{2x^2 + 5x + 2}{x + 2}");
    assert_eq!(serializer::algebraic_expression(&res.node), "2x + 1");
    assert_eq!(restrictions(&res), "x != -2");
    assert_equivalent(r"\frac{2x^2 + 5x + 2}{x + 2}", &res);
}

#[test]
fn cat7_factoring_perfect_square_trinomials() {
    // (x^2 + 6x + 9) / (x + 3) -> x + 3
    let res = run(r"\frac{x^2 + 6x + 9}{x + 3}");
    assert_eq!(serializer::algebraic_expression(&res.node), "x + 3");
    assert_eq!(restrictions(&res), "x != -3");
    assert_equivalent(r"\frac{x^2 + 6x + 9}{x + 3}", &res);
}

#[test]
fn cat7_irreducible_quadratics_remain_intact() {
    // x^2 + x + 1 is irreducible over rationals; cannot cancel with (x + 1)
    let res = run(r"\frac{x^2 + x + 1}{x + 1}");
    assert_eq!(
        serializer::algebraic_expression(&res.node),
        r"\frac{x^{2} + x + 1}{x + 1}"
    );
    assert_eq!(restrictions(&res), "x != -1");
    assert_equivalent(r"\frac{x^2 + x + 1}{x + 1}", &res);
}

// ---------------------------------------------------------------------------------------------
// Category 8: Combining like terms and canonical descending degree sorting
// ---------------------------------------------------------------------------------------------

#[test]
fn cat8_combining_like_terms_reference_subexpression() {
    // 3x^2 - 18x + 24 + x - 102 -> 3x^2 - 17x - 78
    let res = run(r"3x^2 - 18x + 24 + x - 102");
    assert_eq!(
        serializer::algebraic_expression(&res.node),
        "3x^{2} - 17x - 78"
    );
    assert_equivalent(r"3x^2 - 18x + 24 + x - 102", &res);
}

#[test]
fn cat8_canonical_descending_degree_sorting() {
    // Unsorted: 5 + 2x^3 - 4x + x^2 -> 2x^3 + x^2 - 4x + 5
    let res = run(r"5 + 2x^3 - 4x + x^2");
    assert_eq!(
        serializer::algebraic_expression(&res.node),
        "2x^{3} + x^{2} - 4x + 5"
    );
    assert_equivalent(r"5 + 2x^3 - 4x + x^2", &res);
}

// ---------------------------------------------------------------------------------------------
// Category 9: Canceling common factors safely
// ---------------------------------------------------------------------------------------------

#[test]
fn cat9_safe_canceling_common_factors() {
    // (x^2 - 9) / (x^2 + 6x + 9) = (x - 3)(x + 3) / (x + 3)^2 -> (x - 3) / (x + 3)
    let res = run(r"\frac{x^2 - 9}{x^2 + 6x + 9}");
    assert_eq!(
        serializer::algebraic_expression(&res.node),
        r"\frac{x - 3}{x + 3}"
    );
    assert_eq!(restrictions(&res), "x != -3");
    assert_equivalent(r"\frac{x^2 - 9}{x^2 + 6x + 9}", &res);
}

#[test]
fn cat9_safety_never_cancel_additive_terms() {
    // CRITICAL: (x + 2) / x must NEVER cancel to 2!
    let res = run(r"\frac{x + 2}{x}");
    assert_eq!(serializer::algebraic_expression(&res.node), r"\frac{x + 2}{x}");
    assert_eq!(restrictions(&res), "x != 0");
    assert_equivalent(r"\frac{x + 2}{x}", &res);

    // Also: (x^2 + 1) / (x + 1) must not cancel x or 1
    let res = run(r"\frac{x^2 + 1}{x + 1}");
    assert_eq!(
        serializer::algebraic_expression(&res.node),
        r"\frac{x^{2} + 1}{x + 1}"
    );
    assert_eq!(restrictions(&res), "x != -1");
    assert_equivalent(r"\frac{x^2 + 1}{x + 1}", &res);
}

// ---------------------------------------------------------------------------------------------
// Category 10: Preserving domain restrictions after cancellation
// ---------------------------------------------------------------------------------------------

#[test]
fn cat10_domain_restrictions_preserved_after_cancellation() {
    // (x^2 - 1) / (x - 1) -> x + 1 with domain restriction x != 1
    let res = run(r"\frac{x^2 - 1}{x - 1}");
    assert_eq!(serializer::algebraic_expression(&res.node), "x + 1");
    assert_eq!(restrictions(&res), "x != 1");
    assert_equivalent(r"\frac{x^2 - 1}{x - 1}", &res);

    // x(x - 5) / x -> x - 5 with restriction x != 0
    let res = run(r"\frac{x \cdot (x - 5)}{x}");
    assert_eq!(serializer::algebraic_expression(&res.node), "x - 5");
    assert_eq!(restrictions(&res), "x != 0");
    assert_equivalent(r"\frac{x \cdot (x - 5)}{x}", &res);

    // (x - 2)(x + 3) / ((x - 2)(x - 4)) -> (x + 3) / (x - 4) with x != 2, 4
    let res = run(r"\frac{(x - 2) \cdot (x + 3)}{(x - 2) \cdot (x - 4)}");
    assert_eq!(
        serializer::algebraic_expression(&res.node),
        r"\frac{x + 3}{x - 4}"
    );
    assert_eq!(restrictions(&res), "x != 2, 4");
    assert_equivalent(r"\frac{(x - 2) \cdot (x + 3)}{(x - 2) \cdot (x - 4)}", &res);
}

// ---------------------------------------------------------------------------------------------
// Category 11: Negative coefficients, subtraction of fractions, and nested parentheses
// ---------------------------------------------------------------------------------------------

#[test]
fn cat11_subtraction_of_fractions_and_negative_coefficients() {
    // 1/x - (x - 1)/x^2 = (x - (x - 1)) / x^2 = 1 / x^2
    let res = run(r"\frac{1}{x} - \frac{x - 1}{x^2}");
    assert_eq!(serializer::algebraic_expression(&res.node), r"\frac{1}{x^{2}}");
    assert_eq!(restrictions(&res), "x != 0");
    assert_equivalent(r"\frac{1}{x} - \frac{x - 1}{x^2}", &res);

    // -(2 - x)/(x + 3) + (x - 1)/(x + 3) = ((x - 2) + (x - 1)) / (x + 3) = (2x - 3)/(x + 3)
    let res = run(r"- \frac{2 - x}{x + 3} + \frac{x - 1}{x + 3}");
    assert_eq!(
        serializer::algebraic_expression(&res.node),
        r"\frac{2x - 3}{x + 3}"
    );
    assert_eq!(restrictions(&res), "x != -3");
    assert_equivalent(r"- \frac{2 - x}{x + 3} + \frac{x - 1}{x + 3}", &res);
}

#[test]
fn cat11_nested_parentheses_simplification() {
    // ((x + 1) - (2x - (3 - x))) = x + 1 - (2x - 3 + x) = x + 1 - (3x - 3) = -2x + 4
    let res = run(r"((x + 1) - (2x - (3 - x)))");
    assert_eq!(serializer::algebraic_expression(&res.node), "-2x + 4");
    assert_equivalent(r"((x + 1) - (2x - (3 - x)))", &res);
}

// ---------------------------------------------------------------------------------------------
// Category 12: Constant expressions and zero numerators
// ---------------------------------------------------------------------------------------------

#[test]
fn cat12_zero_numerators_with_restrictions() {
    // (x - x) / (x + 1) -> 0 with x != -1
    let res = run(r"\frac{x - x}{x + 1}");
    assert_eq!(serializer::algebraic_expression(&res.node), "0");
    assert_eq!(restrictions(&res), "x != -1");
    assert_equivalent(r"\frac{x - x}{x + 1}", &res);

    // 0 / (x - 5) -> 0 with x != 5
    let res = run(r"\frac{0}{x - 5}");
    assert_eq!(serializer::algebraic_expression(&res.node), "0");
    assert_eq!(restrictions(&res), "x != 5");
    assert_equivalent(r"\frac{0}{x - 5}", &res);
}

#[test]
fn cat12_constant_expressions_and_reduction() {
    // 1/2 + 1/3 -> 5/6
    let res = run(r"\frac{1}{2} + \frac{1}{3}");
    assert_eq!(serializer::algebraic_expression(&res.node), r"\frac{5}{6}");
    assert!(res.domain_restrictions.is_empty());
    assert_equivalent(r"\frac{1}{2} + \frac{1}{3}", &res);

    // (2x + 4) / (x + 2) -> 2 with x != -2
    let res = run(r"\frac{2x + 4}{x + 2}");
    assert_eq!(serializer::algebraic_expression(&res.node), "2");
    assert_eq!(restrictions(&res), "x != -2");
    assert_equivalent(r"\frac{2x + 4}{x + 2}", &res);
}

// ---------------------------------------------------------------------------------------------
// Category 13: Detecting unsupported expressions gracefully
// ---------------------------------------------------------------------------------------------

#[test]
fn cat13_unsupported_expressions_handled_gracefully() {
    // Non-integer exponent: returns diagnostic without claiming false simplifications
    let res = run(r"x^{\pi} + 1");
    assert_eq!(serializer::expression(&res.node), r"x^{\pi} + 1");
    assert!(res.diagnostic.as_ref().unwrap().contains("non-integer exponents"));

    // Matrix input: handled with diagnostic
    let res = run(r"\begin{pmatrix} x & 1 \\ 0 & x \end{pmatrix}");
    assert!(res.diagnostic.as_ref().unwrap().contains("matrices"));

    // Transcendental function: simplifies rational subtrees, diagnostic for transcendental part
    let res = run(r"\sin(x) + \frac{x^2 - 1}{x - 1}");
    assert!(res.diagnostic.as_ref().unwrap().contains("transcendental functions"));
    assert_eq!(restrictions(&res), "x != 1");
}

#[test]
fn cat13_non_polynomial_denominators_are_not_dropped() {
    // Combining over the LCD once read this denominator as 1: 1/sin(x) + 1 became 2.
    let input = r"\frac{1}{\sin(x)} + 1";
    assert_eq!(serializer::expression(&run(input).node), serializer::expression(&parse(input)));
}

#[test]
fn cat13_square_roots_are_rationalized_and_collected() {
    let cases = [
        (r"\frac{1}{\sqrt{a}} + \frac{\sqrt{a}}{a}", r"\frac{2\sqrt{a}}{a}", "a != 0"),
        (r"\frac{1}{\sqrt{a}} + 1", r"\frac{a + \sqrt{a}}{a}", "a != 0"),
        (r"\sqrt{a} + \sqrt{a}", r"2\sqrt{a}", ""),
        (r"\frac{x}{\sqrt{x}}", r"\sqrt{x}", "x != 0"),
        (r"\frac{1}{x + \sqrt{a}} + \frac{1}{x}", r"\frac{2x + \sqrt{a}}{x(x + \sqrt{a})}", r"x != 0, \sqrt{a} + x != 0"),
    ];
    for (input, expected, domain) in cases {
        let res = run(input);
        assert_eq!(serializer::algebraic_expression(&res.node), expected, "{input}");
        assert_eq!(restrictions(&res), domain, "{input}");
    }

    let res = run(r"\frac{1}{\sqrt{a}} + \frac{\sqrt{a}}{a}");
    assert_eq!(res.steps[0].rule, Some(RuleId::RationalizeDenominator));
    assert!(res.steps.iter().all(|step| !step.explanation.contains('√')), "{:?}", res.steps);
}

// ---------------------------------------------------------------------------------------------
// Category 14: Verifying mathematical equivalence across test cases
// ---------------------------------------------------------------------------------------------

#[test]
fn cat14_mathematical_equivalence_verification_suite() {
    let test_cases = [
        r"\frac{1}{\frac{1}{x}}",
        r"\frac{\frac{x - 1}{x + 2}}{x - 3}",
        r"\frac{1 + \frac{1}{x}}{1 - \frac{1}{x}}",
        r"\frac{x}{x + 1} + \frac{2}{x + 1}",
        r"\frac{3x + 5}{x - 2} - \frac{2x + 1}{x - 2}",
        r"\frac{1}{x - 1} + \frac{1}{x + 1}",
        r"\frac{2}{x + 2} - \frac{1}{x - 1}",
        r"\frac{1}{(x + 1)^2} + \frac{1}{(x + 1) \cdot (x - 1)}",
        r"(x + 3)^2",
        r"(x - 4)^2",
        r"(x + 2)^3",
        r"\frac{x^2 - 9}{x - 3}",
        r"\frac{4x^2 - 25}{2x - 5}",
        r"\frac{x^2 - 5x + 6}{x - 2}",
        r"\frac{2x^2 + 5x + 2}{x + 2}",
        r"\frac{x^2 + 6x + 9}{x + 3}",
        r"\frac{x^2 - 9}{x^2 + 6x + 9}",
        r"\frac{x^2 - 1}{x - 1}",
        r"\frac{1}{x} - \frac{x - 1}{x^2}",
        r"- \frac{2 - x}{x + 3} + \frac{x - 1}{x + 3}",
        r"((x + 1) - (2x - (3 - x)))",
        r"\frac{x - x}{x + 1}",
        r"\frac{2x + 4}{x + 2}",
    ];

    for case in test_cases {
        let orig = parse(case);
        let res = run(case);
        let vres = verify_equivalence(&orig, &res.node, &res.domain_restrictions, 5);
        assert!(
            vres.is_equivalent(),
            "Mathematical equivalence failed for '{case}': got {vres:?}"
        );
    }
}

#[test]
fn cancels_fraction_with_negative_leading_numerator() {
    let result = run(r"\frac{-x^{3} - 2x^{2}}{x}");
    assert_eq!(serializer::algebraic_expression(&result.node), "-x^{2} - 2x");
    assert_eq!(restrictions(&result), "x != 0");
}

#[test]
fn cancels_a_quadratic_factor_the_factorizer_does_not_find() {
    let result = run(r"\frac{5x^{6} - 5x^{5} + 5x^{4} - 5x^{3} + 5x^{2} + 5}{x^{2} + 1}");
    assert_eq!(serializer::algebraic_expression(&result.node), "5x^{4} - 5x^{3} + 5");
}

#[test]
fn leaves_out_restrictions_on_factors_without_real_zeros() {
    let result = run(r"\frac{x^{6} - 1}{x^{3} + 1}");
    assert_eq!(restrictions(&result), "x != -1");
    // An irreducible factor written in the input is still listed.
    let result = run(r"\frac{x}{(x^{2} - 1)(x^{2} + 1)}");
    assert_eq!(restrictions(&result), "x != -1, 1, x^2 + 1 != 0");
}

#[test]
fn cancels_factors_of_expanded_polynomials_with_large_constant_terms() {
    // The expanded denominator ends in 206976, beyond what listing divisors can handle.
    let input = r"\frac{(x - 1)(x + 12)(x + 5)x(x - 7)^{2}}{-(x + 11)(x - 4)(x - 8)(x - 7)^{2}(x + 12)}";
    let result = run(input);
    assert_eq!(restrictions(&result), "x != -12, -11, 4, 7, 8");
    assert_equivalent(input, &result);
    let node = &result.node;
    let text = serializer::algebraic_expression(node);
    assert!(!text.contains("x - 7") && !text.contains("x + 12"), "{text}");
}

#[test]
fn finds_every_root_of_an_expanded_denominator_with_repeated_factors() {
    // (x + 15)^8 · x: the constant term of the combined denominator is 15^9.
    let result = run(r"\frac{-4}{(x + 15)^{8}} - \frac{-3}{x^{2} + 15x}");
    assert_eq!(restrictions(&result), "x != -15, 0");
}

#[test]
fn lists_a_repeated_factor_without_real_zeros_once() {
    let result = run(r"\frac{1}{(w^{2} + 4)^{2}(w + 3)}");
    assert_eq!(restrictions(&result), "w != -3, w^2 + 4 != 0");
}
