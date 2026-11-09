//! Antiderivatives (`\int f\,dx`) and derivatives without a point (`\frac{d}{dx} f`) in the
//! calculator: symbolic results from the analysis engine.

use std::collections::HashMap;

use quantus_core::eval::LatexCalculator;
use quantus_core::latex::{parser, serializer, Node};
use quantus_core::settings::{Mode, Settings};

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
