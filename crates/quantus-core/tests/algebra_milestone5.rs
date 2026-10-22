//! Milestone 5: deterministic simplification pipeline & equivalence verification.

use quantus_core::algebra::domain::format_restrictions_plain;
use quantus_core::algebra::pipeline::{
    canonical_form, common_denominator_at, is_rational_normal_form, normalize_notation,
    normalize_product_at, simplify_node_with, PipelineConfig,
};
use quantus_core::algebra::steps::{RuleId, SimplificationResult};
use quantus_core::algebra::{simplify, simplify_node};
use quantus_core::latex::ast::Node;
use quantus_core::latex::{parser, serializer};

fn parse(input: &str) -> Node {
    parser::parse(input)
        .unwrap_or_else(|e| panic!("Parse error for {input}: {e}"))
        .expression
}

fn run(input: &str) -> SimplificationResult {
    simplify(input).unwrap_or_else(|e| panic!("Simplification of {input} failed: {e}"))
}

fn rules(result: &SimplificationResult) -> Vec<RuleId> {
    result.steps.iter().filter_map(|s| s.rule).collect()
}

fn restrictions(result: &SimplificationResult) -> String {
    format_restrictions_plain(&result.domain_restrictions)
}

/// Steps must chain: each step starts where the previous one ended, the first starts at the
/// input and the last ends at the result.
fn assert_steps_chain(input: &Node, result: &SimplificationResult) {
    let mut current = serializer::expression(input);
    for step in &result.steps {
        assert_eq!(step.before_latex, current, "step does not start where the previous ended");
        assert_ne!(step.before_latex, step.after_latex, "step without visible change");
        current = step.after_latex.clone();
    }
    assert_eq!(current, result.latex);
}

// ---------------------------------------------------------------------------------------------
// 5.1 The 17-stage deterministic pipeline
// ---------------------------------------------------------------------------------------------

#[test]
fn pipeline_reference_expression_runs_all_stages_in_order() {
    let input = r"\frac{3}{\frac{x + 3}{x - 2}} + \frac{\frac{x - 102}{x - 4}}{x + 3}";
    let result = run(input);
    assert_steps_chain(&parse(input), &result);

    assert_eq!(
        rules(&result),
        vec![
            RuleId::ApplyReciprocalRule,
            RuleId::SimplifyNestedFraction,
            RuleId::FindCommonDenominator,
            RuleId::CombineFractions,
            RuleId::ExpandPolynomial,
            RuleId::ExpandPolynomial,
            RuleId::CollectLikeTerms,
            RuleId::FactorPolynomial,
            RuleId::CancelCommonFactors,
        ]
    );

    let s = &result.steps;
    assert!(s[0].after_latex.contains(r"\frac{3 \cdot (x - 2)}{x + 3}"));
    assert!(s[1].after_latex.contains(r"\frac{x - 102}{(x - 4) \cdot (x + 3)}"));
    assert_eq!(
        s[3].after_latex,
        r"\frac{3 \cdot (x - 2) \cdot (x - 4) + x - 102}{(x - 4) \cdot (x + 3)}"
    );
    assert!(s[4].after_latex.contains(r"3 \cdot (x^{2} - 6 \cdot x + 8) + x - 102"));
    assert!(s[5].after_latex.contains(r"3 \cdot x^{2} - 18 \cdot x + 24 + x - 102"));
    assert_eq!(
        s[6].after_latex,
        r"\frac{3 \cdot x^{2} - 17 \cdot x - 78}{(x - 4) \cdot (x + 3)}"
    );
    // 3x^2 - 17x - 78 = (3x - 26)(x + 3), so (x + 3) cancels.
    assert_eq!(result.latex, r"\frac{3 \cdot x - 26}{x - 4}");
    assert_eq!(restrictions(&result), "x != -3, 2, 4");
    assert!(result.diagnostic.is_none());
}

#[test]
fn pipeline_stage2_normalizes_negative_exponents() {
    let normalized = normalize_notation(&parse(r"x^{-2} + z^{-1}"));
    assert_eq!(serializer::expression(&normalized), r"\frac{1}{x^{2}} + \frac{1}{z}");

    let result = run(r"x^{-2} + \frac{1}{x}");
    assert_eq!(result.steps[0].rule, Some(RuleId::NormalizeExpression));
    assert_eq!(result.latex, r"\frac{x + 1}{x^{2}}");
    assert_eq!(restrictions(&result), "x != 0");
}

#[test]
fn pipeline_stage2_removes_double_negation() {
    let normalized = normalize_notation(&parse(r"-(-x)"));
    assert_eq!(serializer::expression(&normalized), "x");
}

#[test]
fn pipeline_stage3_restrictions_include_reciprocal_denominators() {
    // The numerator x + 3 of the inner divisor becomes a denominator.
    let result = run(r"\frac{1}{\frac{x + 3}{x - 2}}");
    assert_eq!(restrictions(&result), "x != -3, 2");
}

#[test]
fn pipeline_restrictions_found_after_combining_inner_sums() {
    // 1 - 1/x only becomes the polynomial denominator (x - 1)/x after combining.
    let input = r"\frac{1 + \frac{1}{x}}{1 - \frac{1}{x}}";
    let result = run(input);
    assert_steps_chain(&parse(input), &result);
    assert_eq!(result.latex, r"\frac{x + 1}{x - 1}");
    assert_eq!(restrictions(&result), "x != 0, 1");
}

#[test]
fn pipeline_stage5_multiplies_fractions() {
    let (after, rule, _) =
        normalize_product_at(&parse(r"\frac{x}{x + 1} \cdot \frac{x + 1}{x}")).unwrap();
    assert_eq!(rule, RuleId::NormalizeExpression);
    assert_eq!(
        serializer::expression(&after),
        r"\frac{x \cdot (x + 1)}{(x + 1) \cdot x}"
    );

    let result = run(r"\frac{x}{x + 1} \cdot \frac{x + 1}{x}");
    assert_eq!(result.latex, "1");
    assert_eq!(restrictions(&result), "x != -1, 0");
}

#[test]
fn pipeline_stage5_distributes_powers_over_fractions() {
    let input = r"\left(\frac{x}{x + 1}\right)^2 \cdot \frac{x + 1}{x}";
    let result = run(input);
    assert_steps_chain(&parse(input), &result);
    assert_eq!(result.latex, r"\frac{x}{x + 1}");
    assert_eq!(restrictions(&result), "x != -1, 0");
    // Expanding x^2(x + 1) only to factor it back is a detour that gets rolled back (5.2).
    assert_eq!(
        rules(&result),
        vec![
            RuleId::NormalizeExpression,
            RuleId::NormalizeExpression,
            RuleId::CancelCommonFactors
        ]
    );
    assert!(result.diagnostic.is_none());
}

#[test]
fn pipeline_stage6_factors_denominators_before_combining() {
    let result = run(r"\frac{1}{x^2 - 9} + \frac{1}{x + 3}");
    assert_eq!(
        rules(&result),
        vec![
            RuleId::FactorDenominators,
            RuleId::FindCommonDenominator,
            RuleId::CombineFractions,
            RuleId::CollectLikeTerms,
        ]
    );
    assert_eq!(result.latex, r"\frac{x - 2}{(x - 3) \cdot (x + 3)}");
    assert_eq!(restrictions(&result), "x != -3, 3");
}

#[test]
fn pipeline_stage7_rewrites_over_lcd_with_repeated_factors() {
    let node = parse(r"\frac{1}{(x + 1)^2} + \frac{1}{(x + 1) \cdot (x - 1)}");
    let (after, rule, _) = common_denominator_at(&node).unwrap();
    assert_eq!(rule, RuleId::FindCommonDenominator);
    assert_eq!(
        serializer::expression(&after),
        r"\frac{x - 1}{(x - 1) \cdot (x + 1)^{2}} + \frac{x + 1}{(x - 1) \cdot (x + 1)^{2}}"
    );

    // Identical denominators need no rewrite.
    assert!(common_denominator_at(&parse(r"\frac{2}{x + 1} + \frac{3}{x + 1}")).is_none());

    let result = run(r"\frac{1}{(x + 1)^2} + \frac{1}{(x + 1) \cdot (x - 1)}");
    assert_eq!(result.latex, r"\frac{2 \cdot x}{(x - 1) \cdot (x + 1)^{2}}");
}

#[test]
fn pipeline_handles_subtraction_and_negative_coefficients() {
    let result = run(r"\frac{2}{x + 1} - \frac{3}{x - 1}");
    assert_eq!(result.latex, r"\frac{-x - 5}{(x - 1) \cdot (x + 1)}");
    assert_eq!(restrictions(&result), "x != -1, 1");
}

#[test]
fn pipeline_expands_and_collects_polynomials() {
    let result = run(r"(x + 1)^2 - (x - 1)^2");
    assert_eq!(
        rules(&result),
        vec![RuleId::ExpandBinomial, RuleId::ExpandBinomial, RuleId::CollectLikeTerms]
    );
    assert_eq!(result.latex, r"4 \cdot x");
    assert!(result.domain_restrictions.is_empty());
}

#[test]
fn pipeline_cancels_and_preserves_restrictions() {
    let result = run(r"\frac{x^2 - 1}{x - 1}");
    assert_eq!(result.latex, "x + 1");
    assert_eq!(restrictions(&result), "x != 1");

    let result = run(r"\frac{x^2 - 9}{x^2 + 6x + 9}");
    assert_eq!(result.latex, r"\frac{x - 3}{x + 3}");
    assert_eq!(restrictions(&result), "x != -3");
}

#[test]
fn pipeline_never_cancels_additive_terms() {
    let result = run(r"\frac{x + 2}{x}");
    assert!(result.steps.is_empty());
    assert_eq!(result.latex, r"\frac{x + 2}{x}");
    assert_eq!(restrictions(&result), "x != 0");
}

#[test]
fn pipeline_zero_numerator() {
    let result = run(r"\frac{x - x}{x + 1}");
    assert_eq!(result.latex, "0");
    assert_eq!(restrictions(&result), "x != -1");
}

#[test]
fn pipeline_constant_expressions() {
    let result = run(r"\frac{1}{2} + \frac{1}{3}");
    assert_eq!(result.latex, r"\frac{5}{6}");
    assert!(result.domain_restrictions.is_empty());
}

#[test]
fn pipeline_division_by_zero_is_an_error() {
    assert!(simplify(r"\frac{x}{x - x}").is_err());
    assert!(simplify(r"\frac{1}{0}").is_err());
}

#[test]
fn pipeline_stage13_canonical_form() {
    // Expanded numerator, factored denominator, positive leading denominator coefficient.
    let node = parse(r"\frac{(x + 1) \cdot (x + 2)}{-x^2 + 1}");
    let canonical = canonical_form(&node).unwrap();
    assert_eq!(
        serializer::expression(&canonical),
        r"\frac{-x^{2} - 3 \cdot x - 2}{(x - 1) \cdot (x + 1)}"
    );
    // Constant denominators are divided out.
    let canonical = canonical_form(&parse(r"\frac{2x + 4}{2}")).unwrap();
    assert_eq!(serializer::expression(&canonical), "x + 2");
}

#[test]
fn pipeline_is_deterministic() {
    let input = r"\frac{3}{\frac{x + 3}{x - 2}} + \frac{\frac{x - 102}{x - 4}}{x + 3}";
    let a = run(input);
    let b = simplify_node(&parse(input)).unwrap();
    assert_eq!(a, b);
}

// ---------------------------------------------------------------------------------------------
// 5.2 Termination & oscillation prevention
// ---------------------------------------------------------------------------------------------

#[test]
fn pipeline_runs_another_pass_when_a_new_nested_fraction_appears() {
    // Stage 5 turns x * 1/(x + 1) into x/(x + 1), creating a nested fraction only after stage 4
    // has run; the second pass flattens it.
    let input = r"\frac{1}{x \cdot \frac{1}{x + 1}}";
    let result = run(input);
    assert_steps_chain(&parse(input), &result);
    assert!(is_rational_normal_form(&result.node));
    assert_eq!(result.latex, r"\frac{x + 1}{x}");
    assert_eq!(restrictions(&result), "x != -1, 0");
    assert!(result.diagnostic.is_none());
}

#[test]
fn pipeline_never_revisits_a_state() {
    for input in [
        r"\frac{3}{\frac{x + 3}{x - 2}} + \frac{\frac{x - 102}{x - 4}}{x + 3}",
        r"\frac{x^2 - 9}{x^2 + 6x + 9}",
        r"(x + 1)^3 - (x - 1)^3",
        r"\frac{1 + \frac{1}{x}}{1 - \frac{1}{x}}",
        r"\frac{1}{x \cdot \frac{1}{x + 1}}",
    ] {
        let result = run(input);
        let mut seen = std::collections::HashSet::new();
        seen.insert(serializer::expression(&parse(input)));
        for step in &result.steps {
            assert!(seen.insert(step.after_latex.clone()), "{input}: revisited {}", step.after_latex);
        }
        assert!(result.steps.len() <= PipelineConfig::default().max_total_steps);
    }
}

#[test]
fn pipeline_step_limit_stops_with_a_diagnostic() {
    let input = r"\frac{3}{\frac{x + 3}{x - 2}} + \frac{\frac{x - 102}{x - 4}}{x + 3}";
    let config = PipelineConfig {
        max_total_steps: 3,
        ..PipelineConfig::default()
    };
    let result = simplify_node_with(&parse(input), config).unwrap();
    assert_eq!(result.steps.len(), 3);
    assert!(result.diagnostic.as_deref().unwrap().contains("step limit"));
    // Restrictions are complete even when stopping early.
    assert_eq!(restrictions(&result), "x != -3, 2, 4");
}

#[test]
fn pipeline_pass_limit_stops_with_a_diagnostic() {
    let config = PipelineConfig {
        max_passes: 1,
        ..PipelineConfig::default()
    };
    let result = simplify_node_with(&parse(r"\frac{1}{x \cdot \frac{1}{x + 1}}"), config).unwrap();
    assert!(!is_rational_normal_form(&result.node));
    assert!(result.diagnostic.as_deref().unwrap().contains("pass limit"));
}

#[test]
fn pipeline_terminates_on_large_powers() {
    let result = run(r"\frac{(x + 1)^6}{(x + 1)^4}");
    assert_eq!(result.latex, r"x^{2} + 2 \cdot x + 1");
    assert_eq!(restrictions(&result), "x != -1");
}

// ---------------------------------------------------------------------------------------------
// 5.3 Mathematical Equivalence Verification
// ---------------------------------------------------------------------------------------------

#[test]
fn test_equivalence_verification_success() {
    use quantus_core::algebra::verify::verify_equivalence;
    use quantus_core::algebra::domain::DomainRestriction;

    let orig = parse(r"\frac{x^2 - 1}{x - 1}");
    let simp = parse(r"x + 1");
    let restrs = vec![DomainRestriction::var_not_equal("x", 1)];
    let res = verify_equivalence(&orig, &simp, &restrs, 5);
    assert!(res.is_equivalent(), "Should verify as equivalent");
}

#[test]
fn test_equivalence_verification_detects_mismatch() {
    use quantus_core::algebra::verify::VerificationResult;
    use quantus_core::algebra::verify::verify_equivalence;

    let orig = parse(r"x + 2");
    let wrong_simp = parse(r"x + 3");
    let res = verify_equivalence(&orig, &wrong_simp, &[], 5);
    match res {
        VerificationResult::Mismatch { original_val, simplified_val, .. } => {
            assert_ne!(original_val, simplified_val);
        }
        other => panic!("Expected mismatch, got {:?}", other),
    }
}

#[test]
fn test_equivalence_verification_multivariate() {
    use quantus_core::algebra::verify::verify_equivalence;

    let orig = parse(r"(x + z)^2");
    let simp = parse(r"x^2 + 2 \cdot x \cdot z + z^2");
    let res = verify_equivalence(&orig, &simp, &[], 5);
    assert!(res.is_equivalent());
}

#[test]
fn test_verification_fallback_on_mismatch() {
    use quantus_core::algebra::pipeline::simplify_node;

    // A valid simplification preserves equivalence.
    let res = simplify_node(&parse(r"\frac{x^2 - 1}{x - 1}")).unwrap();
    assert_eq!(res.latex, "x + 1");
    assert!(res.diagnostic.is_none());
}
