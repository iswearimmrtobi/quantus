use quantus_core::algebra::combine::{
    combine_all_fractions, combine_fractions_at, factor_denominators_in_node,
};
use quantus_core::algebra::nested::{
    is_nested_fraction, simplify_all_nested_fractions, simplify_nested_fraction_at,
};
use quantus_core::algebra::steps::RuleId;
use quantus_core::latex::ast::Node;
use quantus_core::latex::parser;
use quantus_core::latex::serializer;

fn parse(input: &str) -> Node {
    let formula = parser::parse(input).unwrap_or_else(|e| panic!("Parse error for {input}: {e}"));
    formula.expression
}

#[test]
fn test_nested_fraction_reciprocal_rule_a_over_b_over_c() {
    // 3 / ((x + 3)/(x - 2)) -> 3(x - 2) / (x + 3)
    let node = parse(r"\frac{3}{\frac{x + 3}{x - 2}}");
    assert!(is_nested_fraction(&node));

    let (simplified, rule, explanation) =
        simplify_nested_fraction_at(&node).expect("Should simplify nested fraction");
    assert_eq!(rule, RuleId::ApplyReciprocalRule);
    assert!(explanation.contains("reciprocal"));

    let latex = serializer::expression(&simplified);
    // Serializer renders products with \cdot: 3 \cdot (x - 2)
    assert!(
        latex.contains(r"3 \cdot (x - 2)") || latex.contains(r"3(x - 2)"),
        "Unexpected latex: {latex}"
    );
    assert!(latex.contains(r"x + 3"), "Unexpected latex: {latex}");
}

#[test]
fn test_nested_fraction_simplify_a_over_b_over_c() {
    // ((x - 102)/(x - 4)) / (x + 3) -> (x - 102) / ((x - 4)(x + 3))
    let node = parse(r"\frac{\frac{x - 102}{x - 4}}{x + 3}");
    assert!(is_nested_fraction(&node));

    let (simplified, rule, explanation) =
        simplify_nested_fraction_at(&node).expect("Should simplify nested fraction");
    assert_eq!(rule, RuleId::SimplifyNestedFraction);
    assert!(explanation.contains("complex fraction"));

    let latex = serializer::expression(&simplified);
    assert!(latex.contains("x - 102"), "Unexpected latex: {latex}");
    assert!(
        latex.contains(r"(x - 4) \cdot (x + 3)") || latex.contains(r"(x - 4)(x + 3)"),
        "Unexpected latex: {latex}"
    );
}

#[test]
fn test_nested_fraction_reciprocal_four_levels() {
    // ((a / b)) / ((c / d)) -> (a * d) / (b * c)
    let node = parse(r"\frac{\frac{x + 1}{x - 1}}{\frac{x + 2}{x - 2}}");
    assert!(is_nested_fraction(&node));

    let (simplified, rule, _) =
        simplify_nested_fraction_at(&node).expect("Should simplify nested fraction");
    assert_eq!(rule, RuleId::ApplyReciprocalRule);

    let latex = serializer::expression(&simplified);
    assert!(
        latex.contains(r"(x + 1) \cdot (x - 2)") || latex.contains(r"(x + 1)(x - 2)"),
        "Unexpected latex: {latex}"
    );
    assert!(
        latex.contains(r"(x - 1) \cdot (x + 2)") || latex.contains(r"(x - 1)(x + 2)"),
        "Unexpected latex: {latex}"
    );
}

#[test]
fn test_nested_fraction_reference_expression_steps() {
    // Reference expression from Milestone 7:
    // 3 / ((x + 3)/(x - 2)) + ((x - 102)/(x - 4))/(x + 3)
    let node = parse(r"\frac{3}{\frac{x + 3}{x - 2}} + \frac{\frac{x - 102}{x - 4}}{x + 3}");

    let (final_node, steps) = simplify_all_nested_fractions(&node);
    assert_eq!(steps.len(), 2, "Expected exactly 2 nested fraction steps");

    // Step 1: First complex fraction rewritten with reciprocal rule
    assert_eq!(steps[0].rule, Some(RuleId::ApplyReciprocalRule));
    assert!(
        steps[0].after_latex.contains(r"3 \cdot (x - 2)")
            || steps[0].after_latex.contains(r"3(x - 2)")
    );

    // Step 2: Second complex fraction rewritten with simplify nested fraction
    assert_eq!(steps[1].rule, Some(RuleId::SimplifyNestedFraction));
    assert!(
        steps[1].after_latex.contains(r"(x - 4) \cdot (x + 3)")
            || steps[1].after_latex.contains(r"(x - 4)(x + 3)")
    );

    // Result should have both simplified
    let final_latex = serializer::expression(&final_node);
    assert!(!is_nested_fraction(&final_node));
    assert!(final_latex.contains("3 \\cdot (x - 2)"));
    assert!(final_latex.contains("x - 102"));
    assert!(final_latex.contains("(x - 4) \\cdot (x + 3)"));
}

#[test]
fn test_nested_fraction_with_inner_fraction_sum() {
    // (1 + 1/x) / (1 - 1/x)
    let node = parse(r"\frac{1 + \frac{1}{x}}{1 - \frac{1}{x}}");
    assert!(is_nested_fraction(&node));

    let (simplified, rule, _) =
        simplify_nested_fraction_at(&node).expect("Should simplify nested fraction with inner sum");
    assert_eq!(rule, RuleId::ApplyReciprocalRule);

    let latex = serializer::expression(&simplified);
    assert!(latex.contains("x"));
}

#[test]
fn test_deeply_nested_fractions() {
    // 1 / (1 / (1 / x)) -> 1 / x
    let node = parse(r"\frac{1}{\frac{1}{\frac{1}{x}}}");
    let (final_node, steps) = simplify_all_nested_fractions(&node);
    assert_eq!(steps.len(), 1);
    let latex = serializer::expression(&final_node);
    assert_eq!(latex, r"\frac{1}{x}");
}

#[test]
fn test_combine_fractions_identical_denominators() {
    let node = parse(r"\frac{2}{x + 1} + \frac{3}{x + 1}");
    let (combined, rule, _) = combine_fractions_at(&node).expect("Should combine");
    assert_eq!(rule, RuleId::CombineFractions);
    let latex = serializer::expression(&combined);
    assert!(latex.contains("2 + 3"), "Unexpected latex: {latex}");
    assert!(latex.contains("x + 1"), "Unexpected latex: {latex}");
}

#[test]
fn test_combine_fractions_factor_denominators_step() {
    // 1/(x^2 - 9) + 1/(x + 3) -> factors x^2 - 9 into (x - 3)(x + 3)
    let node = parse(r"\frac{1}{x^2 - 9} + \frac{1}{x + 3}");
    let (new_node, step) = factor_denominators_in_node(&node).expect("Should factor denominator");
    assert_eq!(step.rule, Some(RuleId::FactorDenominators));
    assert!(step.after_latex.contains("(x - 3)"));
    assert!(step.after_latex.contains("(x + 3)"));

    // Now combining should use LCD (x - 3)(x + 3)
    let (combined, rule, _) = combine_fractions_at(&new_node).expect("Should combine");
    assert_eq!(rule, RuleId::CombineFractions);
    let latex = serializer::expression(&combined);
    assert!(latex.contains("(x - 3)"), "Unexpected latex: {latex}");
    assert!(latex.contains("(x + 3)"), "Unexpected latex: {latex}");
}

#[test]
fn test_combine_fractions_with_polynomial_term() {
    // 1 + 2 / (x - 1) -> (x - 1 + 2) / (x - 1)
    let node = parse(r"1 + \frac{2}{x - 1}");
    let (combined, rule, _) = combine_fractions_at(&node).expect("Should combine");
    assert_eq!(rule, RuleId::CombineFractions);
    let latex = serializer::expression(&combined);
    assert!(latex.contains("x - 1"), "Unexpected latex: {latex}");
    assert!(latex.contains("2"), "Unexpected latex: {latex}");
}

#[test]
fn test_combine_all_fractions_sequence() {
    // 1/(x^2 - 9) + 1/(x + 3) -> factors denominator, then combines
    let node = parse(r"\frac{1}{x^2 - 9} + \frac{1}{x + 3}");
    let (combined, steps) = combine_all_fractions(&node);
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].rule, Some(RuleId::FactorDenominators));
    assert_eq!(steps[1].rule, Some(RuleId::CombineFractions));
    let latex = serializer::expression(&combined);
    assert!(latex.contains(r"(x - 3) \cdot (x + 3)"));
}

#[test]
fn test_combine_fractions_repeated_factors_lcd() {
    // 1 / (x + 1)^2 + 1 / ((x + 1)(x - 1))
    // LCD is (x + 1)^2 (x - 1)
    let node = parse(r"\frac{1}{(x + 1)^2} + \frac{1}{(x + 1) \cdot (x - 1)}");
    let (combined, rule, _) = combine_fractions_at(&node).expect("Should combine");
    assert_eq!(rule, RuleId::CombineFractions);
    let latex = serializer::expression(&combined);
    // Denominator must contain (x + 1)^{2} and (x - 1)
    assert!(latex.contains(r"(x + 1)^{2}"), "Unexpected latex: {latex}");
    assert!(latex.contains(r"x - 1"), "Unexpected latex: {latex}");
}

#[test]
fn test_combine_fractions_reference_expression() {
    // 3(x - 2)/(x + 3) + (x - 102)/((x - 4)(x + 3))
    // Denominators: (x + 3) and (x - 4)(x + 3)
    // LCD: (x - 4)(x + 3)
    // Combined: [3(x - 2)(x - 4) + (x - 102)] / [(x - 4)(x + 3)]
    let node = parse(r"\frac{3 \cdot (x - 2)}{x + 3} + \frac{x - 102}{(x - 4) \cdot (x + 3)}");
    let (combined, rule, _) = combine_fractions_at(&node).expect("Should combine");
    assert_eq!(rule, RuleId::CombineFractions);

    let latex = serializer::expression(&combined);
    assert!(latex.contains(r"3 \cdot (x - 2) \cdot (x - 4)"));
    assert!(latex.contains(r"x - 102"));
    assert!(latex.contains(r"(x - 4) \cdot (x + 3)"));
}
