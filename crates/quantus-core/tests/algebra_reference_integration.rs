//! Step 7.1: Mandatory Reference Integration Test
//!
//! Validates the reference algebraic expression:
//!     3 / ((x + 3)/(x - 2)) + ((x - 102)/(x - 4))/(x + 3)
//!
//! Verifies:
//! 1. Step-by-step simplification sequence:
//!    - First complex fraction: 3 / ((x + 3)/(x - 2)) -> 3(x - 2)/(x + 3)
//!    - Second complex fraction: ((x - 102)/(x - 4))/(x + 3) -> (x - 102)/((x - 4)(x + 3))
//!    - Common denominator & combine: [3(x - 2)(x - 4) + (x - 102)] / [(x - 4)(x + 3)]
//!    - Expand products: 3(x^2 - 6x + 8) + x - 102 and 3x^2 - 18x + 24 + x - 102 over (x - 4)(x + 3)
//!    - Combine like terms: (3x^2 - 17x - 78) / ((x - 4)(x + 3))
//!    - Factor & cancel: (3x - 26)(x + 3) / ((x - 4)(x + 3)) -> (3x - 26) / (x - 4)
//! 2. Domain restrictions: x ≠ -3, 2, 4
//! 3. Equivalence verification: original and simplified expressions evaluate to identical values.
//! 4. Integration across LatexCalculator and API endpoint layers.

use std::collections::HashMap;
use dashu::rational::RBig;
use quantus_core::algebra::domain::{
    format_restrictions_latex, format_restrictions_plain, DomainRestriction,
};
use quantus_core::algebra::steps::RuleId;
use quantus_core::algebra::verify_equivalence;
use quantus_core::algebra::pipeline::{simplify_node_with, PipelineConfig};
use quantus_core::algebra::SimplificationResult;
use quantus_core::settings::StepDetail;
use quantus_core::api::{evaluate as api_evaluate, CalculationRequest};
use quantus_core::eval::LatexCalculator;
use quantus_core::latex::ast::Node;
use quantus_core::latex::{parser, serializer};
use quantus_core::session::SessionStore;

/// One step per rule, as the rules record them.
fn simplify(input: &str) -> quantus_core::Result<SimplificationResult> {
    simplify_node_with(&parse(input), PipelineConfig { detail: StepDetail::Brief, ..PipelineConfig::default() })
}

fn parse(input: &str) -> Node {
    parser::parse(input)
        .unwrap_or_else(|e| panic!("Parse error for '{input}': {e}"))
        .expression
}

#[test]
fn test_reference_expression_slash_notation_full_step_by_step() {
    let input = r"3 / ((x + 3)/(x - 2)) + ((x - 102)/(x - 4))/(x + 3)";
    let parsed_ast = parse(input);
    let result = simplify(input).expect("Simplification of slash reference input must succeed");

    // 1. Chaining check: each step starts where the previous one ended
    let mut current_latex = serializer::expression(&parsed_ast);
    for (i, step) in result.steps.iter().enumerate() {
        assert_eq!(
            step.before_latex, current_latex,
            "Step {i} does not start where the previous step ended"
        );
        assert_ne!(
            step.before_latex, step.after_latex,
            "Step {i} produced no visible LaTeX change"
        );
        assert!(step.rule.is_some(), "Step {i} must have an identified RuleId");
        assert!(
            !step.explanation.trim().is_empty(),
            "Step {i} must have a non-empty explanation"
        );
        current_latex = step.after_latex.clone();
    }
    assert_eq!(current_latex, result.latex);

    // 2. Verify the 5 key sequence points specified in Milestone 7.1:
    // Step sequence contains:
    // (a) Simplify first complex fraction: 3 / ((x + 3)/(x - 2)) -> 3(x - 2)/(x + 3)
    let has_step1 = result.steps.iter().any(|s| {
        s.rule == Some(RuleId::ApplyReciprocalRule)
            && s.after_latex.contains(r"\frac{3 \cdot (x - 2)}{x + 3}")
    });
    assert!(has_step1, "Must contain reciprocal rule step simplifying 3 / ((x + 3)/(x - 2))");

    // (b) Simplify second complex fraction: ((x - 102)/(x - 4))/(x + 3) -> (x - 102)/((x - 4)(x + 3))
    let has_step2 = result.steps.iter().any(|s| {
        s.rule == Some(RuleId::SimplifyNestedFraction)
            && s.after_latex.contains(r"\frac{x - 102}{(x - 4) \cdot (x + 3)}")
    });
    assert!(has_step2, "Must contain nested fraction step simplifying ((x - 102)/(x - 4))/(x + 3)");

    // (c) Common denominator and combine: [3(x - 2)(x - 4) + (x - 102)] / [(x - 4)(x + 3)]
    let has_step3 = result.steps.iter().any(|s| {
        s.rule == Some(RuleId::CombineFractions)
            && s.after_latex.contains(r"3 \cdot (x - 2) \cdot (x - 4) + x - 102")
            && s.after_latex.contains(r"(x - 4) \cdot (x + 3)")
    });
    assert!(has_step3, "Must contain fraction combining step with common denominator (x - 4)(x + 3)");

    // (d) Expand polynomial products:
    //     First: 3(x^2 - 6x + 8) + x - 102
    //     Second: 3x^2 - 18x + 24 + x - 102
    let has_expand1 = result.steps.iter().any(|s| {
        s.rule == Some(RuleId::ExpandPolynomial)
            && s.after_latex.contains(r"3 \cdot (x^{2} - 6 \cdot x + 8)")
    });
    assert!(has_expand1, "Must contain expansion to 3(x^2 - 6x + 8)");

    let has_expand2 = result.steps.iter().any(|s| {
        s.rule == Some(RuleId::ExpandPolynomial)
            && s.after_latex.contains(r"3 \cdot x^{2} - 18 \cdot x + 24")
    });
    assert!(has_expand2, "Must contain expansion to 3x^2 - 18x + 24");

    // (e) Combine like terms: (3x^2 - 17x - 78) / ((x - 4)(x + 3))
    let has_collect = result.steps.iter().any(|s| {
        s.rule == Some(RuleId::CollectLikeTerms)
            && s.after_latex == r"\frac{3 \cdot x^{2} - 17 \cdot x - 78}{(x - 4) \cdot (x + 3)}"
    });
    assert!(has_collect, "Must contain combining like terms to 3x^2 - 17x - 78");

    // (f) Factor and cancel: (3x - 26)/(x - 4)
    let has_factor = result.steps.iter().any(|s| s.rule == Some(RuleId::FactorPolynomial));
    assert!(has_factor, "Must contain factoring step for 3x^2 - 17x - 78");

    let has_cancel = result.steps.iter().any(|s| s.rule == Some(RuleId::CancelCommonFactors));
    assert!(has_cancel, "Must contain cancellation step of common factor (x + 3)");

    // Final result
    assert_eq!(result.latex, r"\frac{3 \cdot x - 26}{x - 4}");
    assert_eq!(
        serializer::algebraic_expression(&result.node),
        r"\frac{3x - 26}{x - 4}"
    );

    // 3. Domain restrictions: x ≠ -3, 2, 4
    let plain_restrictions = format_restrictions_plain(&result.domain_restrictions);
    assert_eq!(plain_restrictions, "x != -3, 2, 4");

    let latex_restrictions = format_restrictions_latex(&result.domain_restrictions);
    assert_eq!(latex_restrictions, "x \\neq -3, 2, 4");

    assert_eq!(result.domain_restrictions.len(), 3);
    assert_eq!(result.domain_restrictions[0], DomainRestriction::var_not_equal("x", RBig::from(-3)));
    assert_eq!(result.domain_restrictions[1], DomainRestriction::var_not_equal("x", RBig::from(2)));
    assert_eq!(result.domain_restrictions[2], DomainRestriction::var_not_equal("x", RBig::from(4)));

    // 4. Mathematical equivalence verification
    assert!(result.diagnostic.is_none());
    assert!(
        verify_equivalence(&parsed_ast, &result.node, &result.domain_restrictions, 5).is_equivalent(),
        "Original and simplified AST must be mathematically equivalent"
    );
}

#[test]
fn test_reference_expression_frac_notation_full_step_by_step() {
    let input = r"\frac{3}{\frac{x + 3}{x - 2}} + \frac{\frac{x - 102}{x - 4}}{x + 3}";
    let parsed_ast = parse(input);
    let result = simplify(input).expect("Simplification of frac reference input must succeed");

    let rules: Vec<RuleId> = result.steps.iter().filter_map(|s| s.rule).collect();
    assert_eq!(
        rules,
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
        ],
        "Exact rule execution order must match the reference pipeline"
    );

    assert_eq!(result.latex, r"\frac{3 \cdot x - 26}{x - 4}");
    assert_eq!(
        serializer::algebraic_expression(&result.node),
        r"\frac{3x - 26}{x - 4}"
    );

    assert_eq!(format_restrictions_plain(&result.domain_restrictions), "x != -3, 2, 4");
    assert_eq!(format_restrictions_latex(&result.domain_restrictions), "x \\neq -3, 2, 4");
    assert!(verify_equivalence(&parsed_ast, &result.node, &result.domain_restrictions, 5).is_equivalent());
}

#[test]
fn test_reference_expression_latex_calculator_integration() {
    let calc = LatexCalculator::default();
    let vars = HashMap::new();

    let input = r"3 / ((x + 3)/(x - 2)) + ((x - 102)/(x - 4))/(x + 3)";
    let calc_res = calc
        .evaluate(input, &vars)
        .expect("LatexCalculator should evaluate algebraic reference input");

    // Result uses clean algebraic notation: \frac{3x - 26}{x - 4}
    assert_eq!(calc_res.result, r"\frac{3x - 26}{x - 4}");

    // Domain restrictions formatted for display
    assert_eq!(
        calc_res.domain_restrictions.as_deref(),
        Some(&["x \\neq -3, 2, 4".to_string()][..])
    );

    // Steps contain rich explanations and rule IDs
    assert!(calc_res.steps.len() >= 5);
    for step in &calc_res.steps {
        assert!(!step.latex.is_empty());
        if let Some(ref rule) = step.rule {
            assert!(!rule.is_empty());
            assert!(step.explanation.is_some());
            assert!(step.before.is_some());
            assert!(step.after.is_some());
        }
    }
}

#[test]
fn test_reference_expression_api_evaluate_endpoint() {
    let mut store = SessionStore::new();
    let request = CalculationRequest {
        latex: r"3 / ((x + 3)/(x - 2)) + ((x - 102)/(x - 4))/(x + 3)".to_string(),
        ..Default::default()
    };

    let response = api_evaluate(&mut store, &request).expect("API evaluate must succeed");

    assert_eq!(response.result, r"\frac{3x - 26}{x - 4}");
    assert_eq!(
        response.domain_restrictions.as_deref(),
        Some(&["x \\neq -3, 2, 4".to_string()][..])
    );

    let rules: Vec<&str> = response
        .steps
        .iter()
        .filter_map(|s| s.rule.as_deref())
        .collect();
    assert!(rules.contains(&"APPLY_RECIPROCAL_RULE"));
    assert!(rules.contains(&"SIMPLIFY_NESTED_FRACTION"));
    assert!(rules.contains(&"COMBINE_FRACTIONS"));
    assert!(rules.contains(&"EXPAND_POLYNOMIAL"));
    assert!(rules.contains(&"COLLECT_LIKE_TERMS"));
    assert!(rules.contains(&"FACTOR_POLYNOMIAL"));
    assert!(rules.contains(&"CANCEL_COMMON_FACTORS"));
}

#[test]
fn test_reference_expression_numerical_evaluation_outside_domain() {
    // Original function: f(x) = 3 / ((x + 3)/(x - 2)) + ((x - 102)/(x - 4))/(x + 3)
    // Simplified function: g(x) = (3x - 26) / (x - 4)
    fn f(x: f64) -> f64 {
        let t1 = 3.0 / ((x + 3.0) / (x - 2.0));
        let t2 = ((x - 102.0) / (x - 4.0)) / (x + 3.0);
        t1 + t2
    }

    fn g(x: f64) -> f64 {
        (3.0 * x - 26.0) / (x - 4.0)
    }

    let sample_points = [-100.0, -10.0, -5.0, -2.0, 0.0, 1.0, 3.0, 5.0, 10.0, 50.0, 102.0];
    for &x in &sample_points {
        let fx = f(x);
        let gx = g(x);
        let diff = (fx - gx).abs();
        assert!(
            diff < 1e-10,
            "Values differ at x={x}: f(x)={fx}, g(x)={gx}, diff={diff}"
        );
    }
}
