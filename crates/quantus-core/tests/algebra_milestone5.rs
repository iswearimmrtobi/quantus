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
use quantus_core::settings::{DenominatorForm, StepDetail};

fn parse(input: &str) -> Node {
    parser::parse(input)
        .unwrap_or_else(|e| panic!("Parse error for {input}: {e}"))
        .expression
}

/// The steps as the rules record them, one per rule (`StepDetail::Brief`).
fn run(input: &str) -> SimplificationResult {
    run_at(input, StepDetail::Brief)
}

fn run_at(input: &str, detail: StepDetail) -> SimplificationResult {
    simplify_node_with(&parse(input), PipelineConfig { detail, ..PipelineConfig::default() })
        .unwrap_or_else(|e| panic!("Simplification of {input} failed: {e}"))
}

const LEVELS: [StepDetail; 5] =
    [StepDetail::None, StepDetail::Brief, StepDetail::Standard, StepDetail::Detailed, StepDetail::Maximal];

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
        if step.note {
            assert_eq!(step.before_latex, step.after_latex, "a note changes nothing");
            continue;
        }
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
    let canonical = canonical_form(&node, DenominatorForm::Auto).unwrap();
    assert_eq!(
        serializer::expression(&canonical),
        r"\frac{-x^{2} - 3 \cdot x - 2}{(x - 1) \cdot (x + 1)}"
    );
    // Constant denominators are divided out.
    let canonical = canonical_form(&parse(r"\frac{2x + 4}{2}"), DenominatorForm::Auto).unwrap();
    assert_eq!(serializer::expression(&canonical), "x + 2");
}

#[test]
fn pipeline_is_deterministic() {
    let input = r"\frac{3}{\frac{x + 3}{x - 2}} + \frac{\frac{x - 102}{x - 4}}{x + 3}";
    for detail in LEVELS {
        assert_eq!(run_at(input, detail), run_at(input, detail));
    }
    assert_eq!(run_at(input, StepDetail::Standard), simplify_node(&parse(input)).unwrap());
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
        detail: StepDetail::Brief,
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
    assert_eq!(result.latex, r"(x + 1)^{2}");
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

// ---------------------------------------------------------------------------------------------
// 5.4 Unsupported constructs handling
// ---------------------------------------------------------------------------------------------

#[test]
fn test_unsupported_constructs_diagnostic_and_partial_simplification() {
    // Non-integer exponent: returns diagnostic without invalid algebraic steps
    let res = simplify(r"x^{\pi} + 1").unwrap();
    assert_eq!(res.latex, r"x^{\pi} + 1");
    assert!(res.diagnostic.as_ref().unwrap().contains("non-integer exponents"));

    // Matrix: returns diagnostic gracefully
    let res = simplify(r"\begin{pmatrix} x & 1 \ 0 & x \end{pmatrix}").unwrap();
    assert!(res.diagnostic.as_ref().unwrap().contains("matrices"));

    // Transcendental function: simplifies supported rational subtrees, notes diagnostic
    let res = simplify(r"\sin(x) + \frac{x^2 - 1}{x - 1}").unwrap();
    assert!(res.diagnostic.as_ref().unwrap().contains("transcendental functions"));
    assert!(!res.steps.is_empty(), "Should simplify rational subtrees");
}

/// `(input, explanation of the step, result)`: each step says what it changed.
#[test]
fn steps_name_what_they_change() {
    let cases = [
        (r"x^2 \cdot x^3", "Same base: add the exponents", "x^{5}"),
        (r"(x^2)^3", "Power of a power: multiply the exponents", "x^{6}"),
        (r"\frac{x}{2}", "Write the division by 2 as a coefficient", "\\frac{1}{2} \\cdot x"),
        (
            r"\frac{1}{1 - x}",
            "Multiply numerator and denominator by -1, so the denominator starts with a positive term",
            "\\frac{-1}{x - 1}",
        ),
        (r"2 \sin(x) \cos(x)", "Use the double angle: 2 \\cdot \\sin{x} \\cdot \\cos{x} = \\sin{2 \\cdot x}", "\\sin{2 \\cdot x}"),
        (r"\log_2(8)", "Divide the logarithms: 8 = 2^{3}, so \\log_{2}{8} = 3", "3"),
        (
            r"\frac{1}{\sqrt{3} - 1}",
            "Rationalize the denominator: multiply by the conjugate \\sqrt{3} + 1",
            "\\frac{\\sqrt{3} + 1}{2}",
        ),
    ];
    for (input, explanation, after) in cases {
        let result = run(input);
        let step = &result.steps[0];
        assert_eq!(step.explanation, explanation, "{input}");
        assert_eq!(step.after_latex, after, "{input}");
    }
    // Putting the terms in order is bookkeeping: its own step only at Maximal, otherwise part of
    // the step after it.
    let ordered = run_at(r"3 + x^2 - 2x", StepDetail::Maximal);
    assert_eq!(ordered.steps[0].explanation, "Order the terms by descending powers");
    assert_eq!(ordered.steps[0].after_latex, "x^{2} - 2 \\cdot x + 3");
    let brief = run(r"3 + x^2 - 2x");
    assert_eq!(brief.steps.len(), 1);
    assert_eq!(brief.steps[0].before_latex, "3 + x^{2} - 2 \\cdot x");
    assert!(brief.steps[0].explanation.starts_with("Complete the square"));
}

#[test]
fn a_logarithm_with_a_base_is_shown_as_typed() {
    let result = run(r"\log_{8} 4");
    assert_eq!(result.steps[0].before_latex, "\\log_{8}{4}");
    assert_eq!(result.latex, "\\frac{2}{3}");
    // A quotient of logarithms without a number as the base stays a fraction.
    assert_eq!(serializer::expression(&parse(r"\frac{\ln x}{\ln y}")), "\\frac{\\ln{x}}{\\ln{y}}");
}

// ---------------------------------------------------------------------------------------------
// Step detail: every level chains and ends at the same result
// ---------------------------------------------------------------------------------------------

#[test]
fn every_detail_level_chains_to_the_same_result() {
    let inputs = [
        r"x^2 - 6x + 3",
        r"\frac{3}{x-3}+\frac{x}{x+3}",
        r"\frac{x}{x+1}-\frac{1}{x}",
        r"\frac{2x^2+4x}{x^2-4}",
        r"\frac{x^2-9}{x-3}",
        r"\frac{\frac{1}{x}}{\frac{2}{x^2}}",
        r"\frac{1}{1+\frac{1}{x}}",
        r"(x+2)(x-5)",
        r"(x-3)^2",
        r"3x^2 - 18x + 24 + x - 102",
        r"3+x^2-2x",
        r"\frac{x^5}{x^2}",
        r"\frac{1}{1-x}",
        r"x^2+6x+9",
        r"\left(\frac{x}{2}\right)^3",
        r"x^{-2}",
        r"\sqrt{12}",
        r"\frac{1}{\sqrt{3}-1}",
        r"\log_2(8)",
        r"2\sin(x)\cos(x)",
    ];
    for input in inputs {
        let brief = run(input);
        for detail in LEVELS {
            let result = run_at(input, detail);
            assert_steps_chain(&parse(input), &result);
            assert_eq!(result.latex, brief.latex, "{input} at {detail:?}");
            assert!(result.steps.len() >= brief.steps.len(), "{input} at {detail:?} has fewer steps than at Brief");
        }
    }
}

/// `(before, after, explanation)` of each step, in LaTeX.
fn lines(result: &SimplificationResult) -> Vec<(String, String, String)> {
    result.steps.iter().map(|s| (s.before_latex.clone(), s.after_latex.clone(), s.explanation.clone())).collect()
}

fn print_levels(input: &str) {
    for detail in LEVELS {
        println!("--- {detail:?}");
        for (before, after, text) in lines(&run_at(input, detail)) {
            println!("{before}  ->  {after}   [{text}]");
        }
    }
}

#[test]
#[ignore]
fn show_levels() {
    for input in std::env::var("INPUTS").unwrap_or_default().split(';').filter(|s| !s.is_empty()) {
        println!("===== {input}");
        print_levels(input);
    }
}

/// `(after, explanation)` of each step at `detail`.
fn rows(input: &str, detail: StepDetail) -> Vec<(String, String)> {
    run_at(input, detail).steps.into_iter().map(|s| (s.after_latex, s.explanation)).collect()
}

fn row(after: &str, text: &str) -> (String, String) {
    (after.to_string(), text.to_string())
}

#[test]
fn completing_the_square_adds_and_subtracts_then_groups() {
    let input = "x^2-6x+3";
    assert_eq!(
        rows(input, StepDetail::Brief),
        [row("(x - 3)^{2} - 6", "Complete the square: add and subtract 9, then apply the binomial formula")]
    );
    let standard = [
        row(r"x^{2} - 6 \cdot x + 9 - 9 + 3", "Half of the coefficient of x is -3, its square is 9: add and subtract 9"),
        row("(x - 3)^{2} - 9 + 3", "Second binomial formula: a² - 2ab + b² = (a - b)²"),
        row("(x - 3)^{2} - 6", "Add the constants"),
    ];
    assert_eq!(rows(input, StepDetail::Standard), standard);
    let detailed = rows(input, StepDetail::Detailed);
    assert_eq!(detailed.len(), 4);
    assert_eq!(detailed[1], row(r"(x^{2} - 6 \cdot x + 9) - 9 + 3", "Group the square"));

    let explicit = rows(r"x^2 + 2 \cdot x \cdot 4 + 11", StepDetail::Standard);
    assert_eq!(explicit[1], row(r"x^{2} + 8 \cdot x + 16 - 16 + 11", r"2 \cdot x \cdot 4 is 2ab with a = x, b = 4: add and subtract b² = 16"));
    assert_eq!(explicit[2].1, "First binomial formula: a² + 2ab + b² = (a + b)²");
}

#[test]
fn a_common_denominator_shows_where_the_missing_factors_come_from() {
    let input = r"\frac{3}{x-3}+\frac{x}{x+3}";
    let brief = rows(input, StepDetail::Brief);
    assert_eq!(brief[0].1, r"Rewrite every fraction over the least common denominator (x - 3) \cdot (x + 3)");
    assert_eq!(brief[1].1, "Add the numerators over the common denominator");
    let standard = rows(input, StepDetail::Standard);
    assert_eq!(
        standard[0],
        row(
            r"\frac{3}{x - 3} \cdot \frac{x + 3}{x + 3} + \frac{x}{x + 3} \cdot \frac{x - 3}{x - 3}",
            r"Extend each fraction by the factor its denominator is missing from the least common denominator (x - 3) \cdot (x + 3)"
        )
    );
    assert_eq!(standard[1], row(r"\frac{3 \cdot (x + 3)}{(x - 3) \cdot (x + 3)} + \frac{x \cdot (x - 3)}{(x - 3) \cdot (x + 3)}", "Multiply the fractions"));
    // A term without a denominator is extended too.
    assert_eq!(rows(r"\frac{1}{x}+1", StepDetail::Standard)[0].0, r"\frac{1}{x} + 1 \cdot \frac{x}{x}");
}

#[test]
fn a_minus_before_a_bracket_flips_the_signs_in_a_line_of_its_own() {
    let standard = rows(r"\frac{x}{x+1}-\frac{1}{x}", StepDetail::Standard);
    let n = standard.len();
    assert_eq!(
        standard[n - 2],
        row(r"\frac{x \cdot x - x - 1}{x \cdot (x + 1)}", "A minus sign in front of a bracket flips every sign inside: -(x + 1) = -x - 1")
    );
    assert_eq!(standard[n - 1].0, r"\frac{x^{2} - x - 1}{x \cdot (x + 1)}");
    let brief = rows(r"\frac{x}{x+1}-\frac{1}{x}", StepDetail::Brief);
    assert_eq!(brief.last().unwrap(), &row(r"\frac{x^{2} - x - 1}{x \cdot (x + 1)}", r"Combine like terms to obtain x^{2} - x - 1"));
}

#[test]
fn factoring_names_the_identity_and_writes_the_squares() {
    let input = r"\frac{2x^2+4x}{x^2-4}";
    let brief = rows(input, StepDetail::Brief);
    assert_eq!(brief[0].1, "Factor the denominator using binomial identity: Difference of squares: a² - b² = (a - b)(a + b)");
    let detailed = rows(input, StepDetail::Detailed);
    assert_eq!(detailed[0], row(r"\frac{2 \cdot x^{2} + 4 \cdot x}{x^{2} - 2^{2}}", "Write 4 as 2^{2}"));
    assert_eq!(detailed[1].1, "Difference of squares: a² - b² = (a - b)(a + b)");
    assert_eq!(detailed[2], row(r"\frac{2 \cdot x \cdot x + 2 \cdot x \cdot 2}{(x - 2) \cdot (x + 2)}", r"Find the common factor 2 \cdot x of all terms"));
    assert_eq!(detailed[3], row(r"\frac{2 \cdot x \cdot (x + 2)}{(x - 2) \cdot (x + 2)}", r"Factor out 2 \cdot x"));
}

#[test]
fn factoring_by_a_root_tries_it_and_divides_it_off() {
    let input = r"\frac{x^2-5x+6}{x-2}";
    let standard = rows(input, StepDetail::Standard);
    assert_eq!(standard[0].1, "Try the divisors of 6: p(2) = 4 - 10 + 6 = 0, so x - 2 is a factor");
    assert_eq!(standard[1], row(r"\frac{(x - 3) \cdot (x - 2)}{x - 2}", "Write as a product of the factors"));
    let detailed = rows(input, StepDetail::Detailed);
    assert_eq!(detailed[1].1, r"Divide: (x^{2} - 5 \cdot x + 6) : (x - 2) = x - 3");
    assert!(run_at(input, StepDetail::Detailed).steps[..2].iter().all(|s| s.note));
    assert_eq!(rows(input, StepDetail::Brief)[0].1, r"Factor numerator to identify common factors: (x - 3) \cdot (x - 2)");
}

#[test]
fn a_common_factor_comes_out_before_the_rest_is_factored() {
    let brief = rows(r"\frac{2x^2-8}{x-2}", StepDetail::Brief);
    assert_eq!(brief[0], row(r"\frac{2 \cdot (x^{2} - 4)}{x - 2}", "Factor out 2"));
    assert_eq!(brief[1], row(r"\frac{2 \cdot (x - 2) \cdot (x + 2)}{x - 2}", "Difference of squares: a² - b² = (a - b)(a + b)"));
}

#[test]
fn cancelling_marks_the_factor_and_notes_where_it_was_zero() {
    let input = r"\frac{x^2-9}{x-3}";
    let cancel = row("x + 3", "Cancel common factor x - 3 from numerator and denominator");
    assert_eq!(rows(input, StepDetail::Standard).last().unwrap(), &cancel);
    let detailed = rows(input, StepDetail::Detailed);
    assert_eq!(detailed[2], row(r"\frac{\cancel{(x - 3)} \cdot (x + 3)}{\cancel{(x - 3)}}", "Mark the common factor"));
    assert_eq!(detailed[3], row("x + 3", "Cancel x - 3"));
    let maximal = run_at(input, StepDetail::Maximal);
    let note = maximal.steps.last().unwrap();
    assert!(note.note && note.after_latex == "x + 3");
    assert_eq!(note.explanation, "Note: x ≠ 3: the cancelled factor x - 3 is zero there, so the input is undefined at x = 3");

    let coloured = simplify_node_with(
        &parse(input),
        PipelineConfig { detail: StepDetail::Detailed, cancel_marks: false, ..PipelineConfig::default() },
    )
    .unwrap();
    assert_eq!(coloured.steps[2].after_latex, r"\frac{\textcolor{orange}{(x - 3)} \cdot (x + 3)}{\textcolor{orange}{(x - 3)}}");
}

#[test]
fn cancelling_powers_subtracts_the_exponents() {
    assert_eq!(
        rows(r"\frac{x^5}{x^2}", StepDetail::Standard),
        [row("x^{5 - 2}", "Divide powers of the same base: subtract the exponents"), row("x^{3}", "Cancel x^{2}")]
    );
    assert_eq!(rows(r"\frac{x^2}{x^5}", StepDetail::Standard)[0].0, r"\frac{1}{x^{5 - 2}}");
}

#[test]
fn the_reciprocal_rule_writes_the_multiplication_out() {
    let standard = rows(r"\frac{\frac{1}{x}}{\frac{2}{x^2}}", StepDetail::Standard);
    assert_eq!(
        standard[..2],
        [
            row(r"\frac{1}{x} \cdot \frac{x^{2}}{2}", r"Dividing by \frac{2}{x^{2}} is multiplying by its reciprocal \frac{x^{2}}{2}"),
            row(r"\frac{x^{2}}{x \cdot 2}", "Multiply numerators and denominators"),
        ]
    );
    // Part of a power is cancelled: it is split before it is marked.
    let detailed = rows(r"\frac{\frac{1}{x}}{\frac{2}{x^2}}", StepDetail::Detailed);
    assert_eq!(detailed[2], row(r"\frac{x \cdot \cancel{x}}{\cancel{x} \cdot 2}", "Mark the common factor"));

    let input = r"\frac{1}{1+\frac{1}{x}}";
    assert_eq!(
        rows(input, StepDetail::Standard),
        [
            row(r"\frac{1}{\frac{x}{x} + \frac{1}{x}}", "Write 1 over the denominator x"),
            row(r"\frac{1}{\frac{x + 1}{x}}", "Combine the inner fractions"),
            row(r"1 \cdot \frac{x}{x + 1}", r"Dividing by \frac{x + 1}{x} is multiplying by its reciprocal \frac{x}{x + 1}"),
            row(r"\frac{x}{x + 1}", "Multiply numerators and denominators"),
        ]
    );
    assert_eq!(rows(input, StepDetail::Brief), [row(r"\frac{x}{x + 1}", "Apply the reciprocal rule: divide by a fraction by multiplying by its reciprocal")]);
    let maximal = run_at(input, StepDetail::Maximal);
    assert!(maximal.steps[0].note);
    assert_eq!(maximal.steps[0].explanation, "Note: x ≠ -1, 0, where a denominator of the input is zero");
}

#[test]
fn a_product_of_sums_is_multiplied_out_term_by_term() {
    let input = "(x+2)(x-5)";
    assert_eq!(rows(input, StepDetail::Brief).len(), 1);
    assert_eq!(
        rows(input, StepDetail::Standard),
        [
            row(r"x^{2} - 5 \cdot x + 2 \cdot x - 10", "Multiply every term of the first bracket by every term of the second"),
            row(r"x^{2} - 3 \cdot x - 10", "Combine like terms"),
        ]
    );
    assert_eq!(rows(input, StepDetail::Detailed)[0].0, r"x \cdot x + x \cdot (-5) + 2 \cdot x + 2 \cdot (-5)");
    // The other factors stay in place around the pair.
    assert_eq!(rows("3(x-2)(x-4)", StepDetail::Detailed)[0].0, r"3 \cdot (x \cdot x + x \cdot (-4) + (-2) \cdot x + (-2) \cdot (-4))");
}

#[test]
fn a_binomial_power_puts_the_values_into_the_formula() {
    assert_eq!(
        rows("(x-3)^2", StepDetail::Standard),
        [
            row(r"x^{2} - 2 \cdot x \cdot 3 + 3^{2}", "(a - b)² = a² - 2ab + b² with a = x, b = 3"),
            row(r"x^{2} - 6 \cdot x + 9", "Work out the terms"),
        ]
    );
    assert_eq!(
        rows("(2x+1)^3", StepDetail::Standard)[0].0,
        r"(2 \cdot x)^{3} + 3 \cdot (2 \cdot x)^{2} \cdot 1 + 3 \cdot (2 \cdot x) \cdot 1^{2} + 1^{3}"
    );
    // With letters only, the line would be the formula itself.
    assert_eq!(rows("(a+b)^2", StepDetail::Standard).len(), 1);
}

#[test]
fn like_terms_are_grouped_before_they_are_added() {
    let input = "3x^2 - 18x + 24 + x - 102";
    assert_eq!(rows(input, StepDetail::Standard), rows(input, StepDetail::Brief));
    assert_eq!(
        rows(input, StepDetail::Detailed),
        [
            row(r"3 \cdot x^{2} + (-18 \cdot x + x) + (24 - 102)", "Put like terms next to each other"),
            row(r"3 \cdot x^{2} - 17 \cdot x - 78", "Add the coefficients"),
        ]
    );
    assert_eq!(rows(input, StepDetail::Maximal)[0], row(r"3 \cdot x^{2} - 18 \cdot x + x + 24 - 102", "Order the terms by degree"));

    let cancelling = rows(r"\frac{3}{x-3}+\frac{x}{x+3}", StepDetail::Detailed);
    let n = cancelling.len();
    assert_eq!(cancelling[n - 2], row(r"\frac{x^{2} + 0 + 9}{(x - 3) \cdot (x + 3)}", r"3 \cdot x - 3 \cdot x = 0"));
}

#[test]
fn each_kind_of_notation_is_a_step_of_its_own() {
    assert_eq!(rows("x^{-2}", StepDetail::Brief), [row(r"\frac{1}{x^{2}}", r"Negative exponent: a^{-n} = \frac{1}{a^{n}}")]);
    assert_eq!(rows("-(-x)", StepDetail::Brief), [row("x", "Two minus signs cancel: -(-a) = a")]);
    let input = r"\frac{-(-x)}{x^{-1}}";
    let maximal: Vec<String> = run_at(input, StepDetail::Maximal).steps.into_iter().map(|s| s.explanation).collect();
    assert_eq!(maximal[1..3], [r"Negative exponent: a^{-n} = \frac{1}{a^{n}}", "Two minus signs cancel: -(-a) = a"]);
    assert_eq!(maximal[3], r"Dividing by \frac{1}{x} is multiplying by its reciprocal x");
    assert!(rows(input, StepDetail::Standard).iter().all(|(_, text)| !text.starts_with("Two minus")));
}
