use std::collections::HashMap;

use quantus_core::eval::LatexCalculator;
use quantus_core::settings::{Mode, Settings};

fn solve(latex: &str, exact_roots: bool, mode: Mode) -> String {
    let settings = Settings { mode, exact_roots, ..Settings::default() };
    LatexCalculator::new(10)
        .with_settings(settings)
        .evaluate(latex, &HashMap::new())
        .map(|c| c.result)
        .unwrap_or_else(|e| format!("error: {e}"))
}

#[test]
fn cubics_and_quartics_are_exact() {
    assert_eq!(solve("x^3 - 7x + 6 = 0", true, Mode::Algebra), r"x = -3 \lor x = 1 \lor x = 2");
    assert_eq!(solve("2x^3 - x^2 - 4x + 2 = 0", true, Mode::Algebra), r"x = -\sqrt{2} \lor x = 0{,}5 \lor x = \sqrt{2}");
    assert_eq!(solve("x^3 = 2", false, Mode::Algebra), r"x = 1{,}25992105");
    assert_eq!(solve("x^4 - 5x^2 + 4 = 0", true, Mode::Algebra), r"x = -2 \lor x = -1 \lor x = 1 \lor x = 2");
    assert_eq!(solve("x^4 = 4", true, Mode::Algebra), r"x = -\sqrt{2} \lor x = \sqrt{2}");
}

#[test]
fn three_real_irrational_roots_come_as_decimals() {
    assert_eq!(solve("x^3 - 3x + 1 = 0", true, Mode::Algebra), r"x = -1{,}879385242 \lor x = 0{,}3472963553 \lor x = 1{,}532088886");
}

#[test]
fn complex_roots_of_a_cubic() {
    assert_eq!(solve("x^3 = 1", true, Mode::Imaginary), r"x = -\frac{1}{2} - \frac{\sqrt{3}}{2}i \lor x = -\frac{1}{2} + \frac{\sqrt{3}}{2}i \lor x = 1");
}

#[test]
fn polynomials_have_no_search_interval() {
    assert_eq!(solve("x^5 - 1000x^4 = 1", false, Mode::Algebra), "x = 1000");
    assert_eq!(solve("x^3 = 1000000", true, Mode::Algebra), "x = 100");
    assert_eq!(solve("x^3 - 300x^2 = 0", true, Mode::Algebra), r"x = 0 \lor x = 300");
    assert!(solve("x^4 + 1 = 0", false, Mode::Algebra).contains("no real solution"));
}

#[test]
fn the_golden_cubic_cases_still_hold() {
    let settings = Settings { mode: Mode::Physics, ..Settings::default() };
    let calculator = LatexCalculator::new(50).with_settings(settings);
    assert_eq!(calculator.evaluate("x^3 = 8", &HashMap::new()).unwrap().result, "x = 2");
    assert!(calculator.evaluate("x^3 - 2 = 0", &HashMap::new()).unwrap().result.starts_with("x = 1{,}25992104989487316476"));
}
