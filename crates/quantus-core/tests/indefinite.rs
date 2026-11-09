//! Antiderivatives (`\int f\,dx`) and derivatives without a point (`\frac{d}{dx} f`) in the
//! calculator: symbolic results from the analysis engine.

use std::collections::HashMap;

use quantus_core::eval::LatexCalculator;
use quantus_core::latex::{parser, serializer, Node};
use quantus_core::num::Value;
use quantus_core::quantity::PhysicalQuantity;
use quantus_core::settings::{Mode, Settings, StepDetail};

fn calc_in(latex: &str, mode: Mode) -> String {
    let settings = Settings { mode, ..Settings::default() };
    LatexCalculator::new(10)
        .with_settings(settings)
        .evaluate(latex, &HashMap::new())
        .map(|c| c.latex)
        .unwrap_or_else(|e| format!("error: {e}"))
}

fn calc(latex: &str) -> String {
    calc_in(latex, Mode::Algebra)
}

#[test]
fn an_integral_without_limits_parses_as_an_antiderivative() {
    let formula = parser::parse(r"\int x^2\,dx").unwrap();
    assert!(matches!(formula.expression, Node::Antiderivative { ref variable, .. } if variable == "x"));
    assert_eq!(serializer::formula(&formula), r"\int x^{2} \,dx");
    assert!(parser::parse(r"\int^1_0 x\,dx").unwrap_err().to_string().contains("needs limits"));
}

#[test]
fn an_antiderivative_inside_a_calculation_is_refused() {
    assert!(calc(r"2 + \int x\,dx").contains("without limits"));
}

#[test]
fn integrals_without_limits_give_the_antiderivative_plus_c() {
    assert_eq!(calc(r"\int x^2\,dx"), r"\frac{1}{3}x^{3} + C");
    assert_eq!(calc(r"\int x e^x\,dx"), r"(x - 1)e^{x} + C");
    assert_eq!(calc(r"\int \frac{1}{x}\,dx"), r"\ln\left|x\right| + C");
    assert_eq!(calc(r"F = \int 2x\,dx"), r"F = x^{2} + C");
    // The keypad's and MathLive's template with its limits left empty.
    assert_eq!(calc(r"\int_{}^{}x^{2}\,dx"), r"\frac{1}{3}x^{3} + C");
    assert!(calc(r"\int_{}^{1}x\,dx").starts_with("error"));
    assert_eq!(calc_in(r"\int t^2\,dt", Mode::Physics), r"\frac{1}{3}t^{3} + C");
}

#[test]
fn derivatives_without_a_point_give_the_derivative() {
    assert_eq!(calc(r"\frac{d}{dx} x^3"), r"3x^{2}");
    assert_eq!(calc(r"\frac{d}{dx}(x^3 - 2x)"), r"3x^{2} - 2");
    assert_eq!(calc(r"\frac{d^2}{dx^2} \sin x"), r"-\sin(x)");
    assert_eq!(calc(r"\frac{d}{dx}\frac{d}{dx} x^4"), r"12x^{2}");
    assert_eq!(calc(r"(x^3 - x)''"), r"6x");
    assert_eq!(calc_in(r"\frac{d}{dt} t^3", Mode::Physics), r"3t^{2}");
    // Written after the function, as the handheld types it.
    assert_eq!(calc(r"x^{2}\frac{d}{dx}"), r"2x");
    assert_eq!(calc(r"x^{3}-2x\frac{d}{dx}"), r"3x^{2} - 2");
    assert_eq!(calc(r"y = x^{3}\frac{d}{dx}"), r"y = 3x^{2}");
    assert_eq!(calc(r"\sin(x)\frac{d^2}{dx^2}"), r"-\sin(x)");
}

#[test]
fn the_working_names_the_function_and_ends_in_the_result() {
    let settings = Settings { mode: Mode::Algebra, step_detail: StepDetail::Detailed, ..Settings::default() };
    let c = LatexCalculator::new(10).with_settings(settings).evaluate(r"\int x e^x\,dx", &HashMap::new()).unwrap();
    let lines: Vec<&str> = c.steps.iter().map(|s| s.latex.as_str()).collect();
    assert_eq!(lines.first(), Some(&r"\int x \cdot e^{x} \,dx"));
    assert!(lines.contains(&"f(x) = xe^{x}"));
    assert!(c.steps.iter().any(|s| s.rule.as_deref() == Some("INTEGRATION_BY_PARTS")));
    assert_eq!(lines.last(), Some(&r"(x - 1)e^{x} + C"));
}

#[test]
fn a_derivative_whose_variable_has_a_value_is_taken_there() {
    let settings = Settings { mode: Mode::Algebra, ..Settings::default() };
    let variables = HashMap::from([("x".to_string(), PhysicalQuantity::dimensionless(Value::int(2)))]);
    let c = LatexCalculator::new(10).with_settings(settings).evaluate(r"\frac{d}{dx} x^3", &variables).unwrap();
    assert_eq!(c.result, "12");
}

#[test]
fn what_has_no_symbolic_result_says_why() {
    assert!(calc(r"\int e^{x^2}\,dx").contains("No elementary antiderivative"));
    assert!(calc(r"\int a x\,dx").contains("also contains a"));
    assert!(calc(r"\frac{\partial}{\partial x} (x^2 z)").contains("also contains z"));
    assert!(calc_in(r"\int 3\,\mathrm{m}\,x\,dx", Mode::Physics).contains("without units"));
    assert!(calc(r"\frac{d}{dx} x^3 + 1").contains("has to be the whole input"));
}
