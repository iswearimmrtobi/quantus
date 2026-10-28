//! The user settings: each one switches a built-in reading or writing rule.

use quantus_core::latex::parser::{parse, parse_with, ParseOptions};
use quantus_core::latex::Node;

fn options(a_and_y_as_variables: bool, letter_products: bool) -> ParseOptions {
    ParseOptions { a_and_y_as_variables, letter_products, ..ParseOptions::default() }
}

#[test]
fn a_and_y_are_variables_by_default_and_units_when_switched_off() {
    assert!(matches!(parse("a").unwrap().expression, Node::Variable { .. }));
    assert!(matches!(parse("y").unwrap().expression, Node::Variable { .. }));
    let off = options(false, true);
    assert!(matches!(parse_with("a", &off).unwrap().expression, Node::Unit { .. }));
    assert!(matches!(parse_with("y", &off).unwrap().expression, Node::Unit { .. }));
}

#[test]
fn t_u_and_in_are_variables_by_default_and_units_when_switched_on() {
    for symbol in ["t", "u"] {
        assert!(matches!(parse(symbol).unwrap().expression, Node::Variable { .. }), "{symbol}");
    }
    let on = ParseOptions { extra_unit_letters: true, ..ParseOptions::default() };
    for symbol in ["t", "u", "in"] {
        assert!(matches!(parse_with(symbol, &on).unwrap().expression, Node::Unit { .. }), "{symbol}");
    }
    // Explicit unit markup is a unit either way.
    let marked = parse(r"\mathrm{t}").unwrap().expression;
    let marked = if let Node::Group { inner, .. } = marked { *inner } else { marked };
    assert!(matches!(marked, Node::Unit { .. }), "{marked:?}");
    // A product of letters keeps working: 3tx is 3 · t · x.
    assert!(parse("3tx").is_ok());
}

#[test]
fn letter_runs_are_products_by_default_and_errors_when_switched_off() {
    assert!(parse("2ab").is_ok());
    assert!(parse("xz").is_ok());
    let off = options(true, false);
    for input in ["2ab", "xz"] {
        assert!(parse_with(input, &off).is_err(), "{input}");
    }
    // Single letters stay variables either way.
    assert!(parse_with("2 x z", &off).is_ok());
}

#[test]
fn a_letter_run_with_a_needs_a_to_be_a_variable() {
    assert!(parse_with("2ab", &options(false, true)).is_err());
}

// ---------------------------------------------------------------------------------------------
// Output style
// ---------------------------------------------------------------------------------------------

use quantus_core::algebra::pipeline::{simplify_node_with, OutputStyle, PipelineConfig};
use quantus_core::latex::serializer;
use quantus_core::settings::{BinomialForm, DenominatorForm};

fn simplified(input: &str, style: OutputStyle) -> (String, usize) {
    let config = PipelineConfig { style, ..PipelineConfig::default() };
    let result = simplify_node_with(&parse(input).unwrap().expression, config).unwrap();
    (serializer::algebraic_expression(&result.node), result.steps.len())
}

fn binomial(binomial: BinomialForm) -> OutputStyle {
    OutputStyle { binomial, ..OutputStyle::default() }
}

fn denominator(denominator: DenominatorForm) -> OutputStyle {
    OutputStyle { denominator, ..OutputStyle::default() }
}

#[test]
fn binomial_form_auto_expands_compact_inputs_and_compresses_the_rest() {
    let auto = binomial(BinomialForm::Auto);
    assert_eq!(simplified("(a+b)^2", auto).0, r"a^{2} + 2 \cdot a \cdot b + b^{2}");
    assert_eq!(simplified("a^2+2ab+b^2", auto).0, "(a + b)^{2}");
}

#[test]
fn binomial_form_can_be_forced_either_way() {
    assert_eq!(simplified("(a+b)^2", binomial(BinomialForm::Binomial)).0, "(a + b)^{2}");
    let (expanded, steps) = simplified("a^2+2ab+b^2", binomial(BinomialForm::Expanded));
    assert_eq!((expanded.as_str(), steps), (r"a^{2} + 2a \cdot b + b^{2}", 0));
}

#[test]
fn completing_the_square_can_be_switched_off() {
    assert_eq!(simplified("x^2+8x+11", OutputStyle::default()).0, "(x + 4)^{2} - 5");
    let off = OutputStyle { complete_square: false, ..OutputStyle::default() };
    assert_eq!(simplified("x^2+8x+11", off).0, "x^{2} + 8x + 11");
    // A genuine binomial is still written as one.
    assert_eq!(simplified("x^2+2x+1", off).0, "(x + 1)^{2}");
}

#[test]
fn denominator_form_follows_the_setting() {
    let cases = [
        (r"\frac{1}{x^2-1}", DenominatorForm::Auto, r"\frac{1}{(x - 1)(x + 1)}"),
        (r"\frac{1}{x^2-1}", DenominatorForm::Expanded, r"\frac{1}{x^{2} - 1}"),
        (r"\frac{1}{x^2-1}", DenominatorForm::Factored, r"\frac{1}{(x - 1)(x + 1)}"),
        (r"\frac{1}{2x+4}", DenominatorForm::Auto, r"\frac{1}{2x + 4}"),
        (r"\frac{1}{2x+4}", DenominatorForm::Factored, r"\frac{1}{2(x + 2)}"),
    ];
    for (input, form, expected) in cases {
        assert_eq!(simplified(input, denominator(form)).0, expected, "{input} {form:?}");
    }
}

#[test]
fn i_is_a_constant_only_in_imaginary_mode_and_units_vanish_in_algebra_mode() {
    let physics = ParseOptions { imaginary_i: false, ..ParseOptions::default() };
    assert!(matches!(parse_with("i", &physics).unwrap().expression, Node::Variable { .. }));
    assert!(matches!(parse("i").unwrap().expression, Node::Constant { .. }));
    let algebra = ParseOptions { units: false, imaginary_i: false, ..ParseOptions::default() };
    assert!(matches!(parse_with("m", &algebra).unwrap().expression, Node::Variable { .. }));
    assert!(matches!(parse("m").unwrap().expression, Node::Unit { .. }));
}

#[test]
fn physical_constants_are_known_only_in_physics_mode() {
    use quantus_core::eval::LatexCalculator;
    use quantus_core::settings::{Mode, Settings};
    use std::collections::HashMap;

    let calculator = |mode| LatexCalculator::new(50).with_settings(Settings { mode, ..Settings::default() });
    let latex = r"c_0 \cdot 2";
    assert!(calculator(Mode::Physics).evaluate(latex, &HashMap::new()).is_ok());
    assert!(calculator(Mode::Imaginary).evaluate(latex, &HashMap::new()).is_err());
    assert!(calculator(Mode::Algebra).evaluate(latex, &HashMap::new()).is_err());
}


fn calculator_lines(input: &str, detail: quantus_core::settings::StepDetail) -> Vec<(String, String)> {
    let settings = quantus_core::settings::Settings { step_detail: detail, ..Default::default() };
    let c = quantus_core::eval::LatexCalculator::new(50).with_settings(settings).evaluate(input, &Default::default()).unwrap();
    c.steps.into_iter().map(|s| (s.latex, s.explanation.unwrap_or_default())).collect()
}

#[test]
fn the_calculator_shows_the_order_of_operations_at_maximal() {
    use quantus_core::settings::StepDetail;
    let input = r"3 \cdot (4+5)^2 - \frac{6}{4}";
    assert_eq!(calculator_lines(input, StepDetail::Detailed).len(), 2);
    let maximal = calculator_lines(input, StepDetail::Maximal);
    let lines: Vec<&str> = maximal.iter().map(|(latex, _)| latex.as_str()).collect();
    assert_eq!(lines, [r"3 \cdot (4 + 5)^{2} - \frac{6}{4}", r"3 \cdot 9^{2} - \frac{6}{4}", r"3 \cdot 81 - \frac{6}{4}", "243 - 1{,}5", "241{,}5"]);
}

#[test]
fn the_calculator_converts_to_a_common_unit_and_names_the_unit() {
    use quantus_core::settings::StepDetail;
    let sum = calculator_lines(r"5\,\mathrm{km} + 300\,\mathrm{m}", StepDetail::Standard);
    assert_eq!(sum[1], (r"5\,\mathrm{km} + 0{,}3\,\mathrm{km}".to_string(), "Convert to a common unit".to_string()));
    assert_eq!(calculator_lines(r"5\,\mathrm{km} + 300\,\mathrm{m}", StepDetail::Brief).len(), 2);
    let energy = calculator_lines(r"\frac{1}{2} \cdot 3\,\mathrm{kg} \cdot (4\,\mathrm{m/s})^2", StepDetail::Standard);
    assert_eq!(energy[1].0, r"24\,\frac{\mathrm{kg} \cdot \mathrm{m}^{2}}{\mathrm{s}^{2}}");
    assert_eq!(energy[2].0, r"24\,\mathrm{J}");
}
