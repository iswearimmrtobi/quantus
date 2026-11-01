use std::collections::HashMap;

use quantus_core::eval::LatexCalculator;
use quantus_core::settings::{Mode, Settings};

fn calculate(settings: Settings, latex: &str) -> quantus_core::eval::Calculation {
    LatexCalculator::new(20).with_settings(settings).evaluate(latex, &HashMap::new()).unwrap()
}

fn result(mode: Mode, latex: &str) -> String {
    calculate(Settings { mode, ..Settings::default() }, latex).result
}

#[test]
fn a_period_after_a_decimal_is_typed_with_overline() {
    assert_eq!(result(Mode::Physics, r"0{,}\overline{3} \cdot 3"), "1");
    assert_eq!(result(Mode::Physics, r"0{,}1\overline{6} \cdot 6"), "1");
    assert_eq!(result(Mode::Physics, r"1.\overline{142857} \cdot 7"), "8");
    assert_eq!(result(Mode::Algebra, r"0{,}\overline{9}"), "1");
    // Not after a decimal, \overline stays the conjugate.
    assert_eq!(result(Mode::Imaginary, r"\overline{3+4i}"), "3 - 4i");
}
