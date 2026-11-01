use std::collections::HashMap;

use quantus_core::eval::LatexCalculator;
use quantus_core::settings::{Language, Mode, Settings};

fn run(mode: Mode, language: Language, latex: &str) -> String {
    let settings = Settings { mode, language, ..Settings::default() };
    LatexCalculator::new(20)
        .with_settings(settings)
        .evaluate(latex, &HashMap::new())
        .map(|c| format!("{} {}", c.result, c.value.to_plain_string()))
        .unwrap_or_else(|e| format!("error: {e}"))
}

#[test]
fn an_equation_without_unknowns_is_true_or_false() {
    assert_eq!(run(Mode::Algebra, Language::De, "2 + 3 = 5"), r"\text{wahr} 1");
    assert_eq!(run(Mode::Algebra, Language::En, "2 + 3 = 6"), r"\text{false} 0");
    assert_eq!(run(Mode::Physics, Language::En, r"\sqrt{2}^2 = 2"), r"\text{true} 1");
    assert_eq!(run(Mode::Physics, Language::En, r"100\,\mathrm{cm} = 1\,\mathrm{m}"), r"\text{true} 1");
    // With an unknown it is still solved.
    assert_eq!(run(Mode::Algebra, Language::En, "2x = 6"), "x = 3 3");
}

#[test]
fn relations_and_chains() {
    assert_eq!(run(Mode::Algebra, Language::En, "1 < 2"), r"\text{true} 1");
    assert_eq!(run(Mode::Algebra, Language::En, r"3 \le 2"), r"\text{false} 0");
    assert_eq!(run(Mode::Algebra, Language::En, r"1 < 2 \le 2 \neq 3"), r"\text{true} 1");
    assert_eq!(run(Mode::Algebra, Language::En, r"\pi > 3{,}2"), r"\text{false} 0");
    assert_eq!(run(Mode::Algebra, Language::En, "5 ≥ 5"), r"\text{true} 1");
    assert_eq!(run(Mode::Algebra, Language::En, r"\frac{1}{3} = \frac{2}{6} = 0{,}\overline{3}"), r"\text{true} 1");
}

#[test]
fn the_verify_mode_checks_rather_than_solves() {
    assert_eq!(run(Mode::Verify, Language::De, "4 = 2^2"), r"\text{wahr} 1");
    assert!(run(Mode::Verify, Language::De, "x = 2").starts_with("error"));
    let settings = Settings { mode: Mode::Verify, ..Settings::default() };
    let input = LatexCalculator::new(10).with_settings(settings).evaluate(r"1 < 2 \le 3", &HashMap::new()).unwrap().input;
    assert_eq!(input, r"1 < 2 \le 3");
}
