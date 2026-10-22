use quantus_core::algebra::combine::{
    combine_all_fractions, combine_fractions_at, factor_denominators_in_node,
};
use quantus_core::algebra::domain::DomainTracker;
use quantus_core::algebra::expand::{
    collect_like_terms_step, expand_and_collect_all, expand_product_node,
};
use quantus_core::algebra::nested::{
    is_nested_fraction, simplify_all_nested_fractions, simplify_nested_fraction_at,
};
use quantus_core::algebra::reduce::{
    reduce_all_rationals, reduce_rational_node,
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

#[test]
fn test_expand_product_factor_pair() {
    let node = parse(r"(x - 2) \cdot (x - 4)");
    let (expanded, rule, _) = expand_product_node(&node).expect("Should expand factor pair");
    assert_eq!(rule, RuleId::ExpandPolynomial);
    let latex = serializer::expression(&expanded);
    assert!(latex.contains("x^{2} - 6x + 8") || latex.contains("x^{2} - 6 \\cdot x + 8"));
}

#[test]
fn test_expand_distribute_scalar() {
    let node = parse(r"3 \cdot (x^2 - 6x + 8)");
    let (expanded, rule, _) = expand_product_node(&node).expect("Should distribute scalar");
    assert_eq!(rule, RuleId::ExpandPolynomial);
    let latex = serializer::expression(&expanded);
    assert!(latex.contains("3x^{2} - 18x + 24") || latex.contains("3 \\cdot x^{2} - 18 \\cdot x + 24"));
}

#[test]
fn test_expand_binomial_power() {
    let node = parse(r"(x + 5)^2");
    let (expanded, rule, _) = expand_product_node(&node).expect("Should expand binomial");
    assert_eq!(rule, RuleId::ExpandBinomial);
    let latex = serializer::expression(&expanded);
    assert!(latex.contains("x^{2} + 10x + 25") || latex.contains("x^{2} + 10 \\cdot x + 25"));
}

#[test]
fn test_collect_like_terms() {
    let node = parse(r"3x^2 - 18x + 24 + x - 102");
    let (collected, step) = collect_like_terms_step(&node).expect("Should collect terms");
    assert_eq!(step.rule, Some(RuleId::CollectLikeTerms));
    let latex = serializer::expression(&collected);
    assert!(latex.contains("3") && latex.contains("x^{2}") && latex.contains("17") && latex.contains("78"));
}

#[test]
fn test_expand_and_collect_reference_expression_numerator() {
    // [3(x - 2)(x - 4) + (x - 102)] / [(x - 4)(x + 3)]
    // Step 4a: 3(x^2 - 6x + 8) + x - 102 (over (x - 4)(x + 3))
    // Step 4b: 3x^2 - 18x + 24 + x - 102 (over (x - 4)(x + 3))
    // Step 5: (3x^2 - 17x - 78) / ((x - 4)(x + 3))
    let node = parse(r"\frac{3 \cdot (x - 2) \cdot (x - 4) + (x - 102)}{(x - 4) \cdot (x + 3)}");
    let (final_node, steps) = expand_and_collect_all(&node);

    assert!(steps.len() >= 3, "Expected at least 3 expansion/collection steps, got {}", steps.len());

    // Step 1: expand factor pair (x - 2)(x - 4)
    assert_eq!(steps[0].rule, Some(RuleId::ExpandPolynomial));
    assert!(
        steps[0].after_latex.contains("x^{2} - 6x + 8")
            || steps[0].after_latex.contains("x^{2} - 6 \\cdot x + 8")
    );

    // Step 2: distribute 3
    assert_eq!(steps[1].rule, Some(RuleId::ExpandPolynomial));
    assert!(
        steps[1].after_latex.contains("3x^{2} - 18x + 24")
            || steps[1].after_latex.contains("3 \\cdot x^{2} - 18 \\cdot x + 24")
    );

    // Final step: collect like terms
    let last_step = steps.last().unwrap();
    assert_eq!(last_step.rule, Some(RuleId::CollectLikeTerms));
    assert!(
        last_step.after_latex.contains("3x^{2} - 17x - 78")
            || last_step.after_latex.contains("3 \\cdot x^{2} - 17 \\cdot x - 78")
    );

    // Check final fraction
    let final_latex = serializer::expression(&final_node);
    assert!(
        final_latex.contains("3x^{2} - 17x - 78")
            || final_latex.contains("3 \\cdot x^{2} - 17 \\cdot x - 78")
    );
    assert!(final_latex.contains("(x - 4) \\cdot (x + 3)"));
}

#[test]
fn test_cancel_common_factors_quadratic() {
    // (x^2 - 9) / (x^2 + 6x + 9) -> (x - 3) / (x + 3)
    let node = parse(r"\frac{x^2 - 9}{x^2 + 6x + 9}");
    let mut tracker = DomainTracker::new();
    let (reduced, steps) = reduce_all_rationals(&node, Some(&mut tracker));

    assert!(!steps.is_empty());
    let last_step = steps.last().unwrap();
    assert_eq!(last_step.rule, Some(RuleId::CancelCommonFactors));

    let final_latex = serializer::expression(&reduced);
    assert!(final_latex.contains("x - 3"), "Unexpected latex: {final_latex}");
    assert!(final_latex.contains("x + 3"), "Unexpected latex: {final_latex}");

    // Domain restriction x != -3 must be tracked
    let restrictions = tracker.format_plain();
    assert!(restrictions.contains("-3"));
}

#[test]
fn test_strict_multiplicative_only_cancellation() {
    // (x + 2) / x MUST NOT cancel additive terms to 2
    let node = parse(r"\frac{x + 2}{x}");
    let mut tracker = DomainTracker::new();
    let res = reduce_rational_node(&node, Some(&mut tracker));
    assert!(res.is_none(), "Additive term must not be canceled!");
}

#[test]
fn test_preserve_domain_restrictions_after_cancellation() {
    // (x^2 - 1) / (x - 1) -> x + 1 with x != 1
    let node = parse(r"\frac{x^2 - 1}{x - 1}");
    let mut tracker = DomainTracker::new();
    let (reduced, steps) = reduce_all_rationals(&node, Some(&mut tracker));

    assert_eq!(steps.last().unwrap().rule, Some(RuleId::CancelCommonFactors));
    let latex = serializer::expression(&reduced);
    assert_eq!(latex, "x + 1");

    let restrictions = tracker.format_plain();
    assert!(restrictions.contains("x ≠ 1") || restrictions.contains("x != 1"));
}

#[test]
fn test_reduce_special_cases() {
    // 0 / D -> 0
    let node_zero = parse(r"\frac{0}{x + 1}");
    let (res_zero, rule_zero, _) = reduce_rational_node(&node_zero, None).expect("Should reduce 0 / D");
    assert_eq!(rule_zero, RuleId::NormalizeExpression);
    assert_eq!(serializer::expression(&res_zero), "0");

    // P / 1 -> P
    let node_one = parse(r"\frac{2x + 1}{1}");
    let (res_one, rule_one, _) = reduce_rational_node(&node_one, None).expect("Should reduce P / 1");
    assert_eq!(rule_one, RuleId::NormalizeExpression);
    assert!(serializer::expression(&res_one).contains("2") && serializer::expression(&res_one).contains("x"));

    // P / P -> 1
    let node_pp = parse(r"\frac{x^2 + 5}{x^2 + 5}");
    let (res_pp, rule_pp, _) = reduce_rational_node(&node_pp, None).expect("Should reduce P / P");
    assert_eq!(rule_pp, RuleId::CancelCommonFactors);
    assert_eq!(serializer::expression(&res_pp), "1");

    // Opposite signs: (x - 3) / (3 - x) -> -1
    let node_opp = parse(r"\frac{x - 3}{3 - x}");
    let (res_opp, rule_opp, _) = reduce_rational_node(&node_opp, None).expect("Should reduce opposite signs");
    assert_eq!(rule_opp, RuleId::CancelCommonFactors);
    assert_eq!(serializer::expression(&res_opp), "-1");
}

#[test]
fn test_reduce_reference_expression_final_cancellation() {
    // (3x^2 - 17x - 78) / ((x - 4)(x + 3))
    // 3x^2 - 17x - 78 = (3x - 26)(x + 3)
    // Common factor (x + 3) cancels!
    // Result: (3x - 26) / (x - 4)
    let node = parse(r"\frac{3x^2 - 17x - 78}{(x - 4) \cdot (x + 3)}");
    let mut tracker = DomainTracker::new();
    let (final_node, steps) = reduce_all_rationals(&node, Some(&mut tracker));

    assert_eq!(steps.len(), 2, "Expected 2 steps: factor numerator, then cancel");
    assert_eq!(steps[0].rule, Some(RuleId::FactorPolynomial));
    assert_eq!(steps[1].rule, Some(RuleId::CancelCommonFactors));

    let final_latex = serializer::expression(&final_node);
    assert!(final_latex.contains("3x - 26") || final_latex.contains("3 \\cdot x - 26"));
    assert!(final_latex.contains("x - 4"));

    // Verify domain restriction x != -3 was captured during cancellation
    let restrictions = tracker.format_plain();
    assert!(restrictions.contains("-3"));
}
