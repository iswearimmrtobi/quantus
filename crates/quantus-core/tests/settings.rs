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
