//! Complex results in imaginary mode.

use std::collections::HashMap;

use quantus_core::eval::LatexCalculator;
use quantus_core::settings::{Mode, Settings};

fn evaluate(mode: Mode, latex: &str) -> Result<String, String> {
    LatexCalculator::new(50)
        .with_settings(Settings { mode, ..Settings::default() })
        .evaluate(latex, &HashMap::new())
        .map(|c| c.result)
        .map_err(|e| e.to_string())
}

fn imaginary(latex: &str) -> String {
    evaluate(Mode::Imaginary, latex).unwrap_or_else(|e| panic!("{latex}: {e}"))
}

#[test]
fn products_and_quotients_of_complex_numbers() {
    assert_eq!(imaginary(r"(3+4i)(1-2i)"), "11 - 2i");
    assert_eq!(imaginary(r"(3+4i)^2"), "-7 + 24i");
    assert_eq!(imaginary(r"\frac{3+4i}{1-2i}"), "-1 + 2i");
    assert_eq!(imaginary(r"\frac{1}{i}"), "-i");
}

#[test]
fn powers_of_i_and_roots_of_negative_numbers() {
    assert_eq!(imaginary(r"i^7"), "-i");
    assert_eq!(imaginary(r"i^2"), "-1");
    assert_eq!(imaginary(r"\sqrt{-1}"), "i");
    assert_eq!(imaginary(r"\sqrt{-4}"), "2i");
    assert!(imaginary(r"\sqrt{-2}").starts_with("1{,}41421356"));
    assert!(imaginary(r"\sqrt{-2}").ends_with('i'));
}

#[test]
fn real_results_are_unchanged() {
    assert_eq!(imaginary(r"(1+i)(1-i)"), "2");
    assert_eq!(imaginary(r"2+3"), "5");
}

#[test]
fn complex_results_are_an_error_outside_imaginary_mode() {
    for mode in [Mode::Physics, Mode::Algebra] {
        assert!(evaluate(mode, r"\sqrt{-4}").unwrap_err().contains("not a real number"), "{mode:?}");
    }
}
