//! Limits at ∞ whose body has a root of variable degree, also with complex values.

use std::collections::HashMap;

use quantus_core::eval::LatexCalculator;
use quantus_core::settings::{Mode, Settings};

fn evaluate(mode: Mode, latex: &str) -> Result<String, String> {
    LatexCalculator::new(20)
        .with_settings(Settings { mode, ..Settings::default() })
        .evaluate(latex, &HashMap::new())
        .map(|calculation| calculation.result)
        .map_err(|error| error.to_string())
}

#[test]
fn even_roots_of_negative_numbers_are_principal_roots_in_imaginary_mode() {
    assert_eq!(evaluate(Mode::Imaginary, r"\sqrt{-4}").unwrap(), "2i");
    assert_eq!(evaluate(Mode::Imaginary, r"\sqrt[3]{-8}").unwrap(), "-2");
    assert_eq!(evaluate(Mode::Imaginary, r"\sqrt[4]{-16}").unwrap(), "1{,}4142135623730950488 + 1{,}4142135623730950488i");
    assert!(evaluate(Mode::Algebra, r"\sqrt[4]{-16}").unwrap_err().contains("not a real number"));
}

#[test]
fn roots_of_growing_degree_tend_to_one() {
    assert_eq!(evaluate(Mode::Imaginary, r"\lim_{x\to\infty}\left(\sqrt[x]{i}\right)").unwrap(), "1");
    assert_eq!(evaluate(Mode::Imaginary, r"\lim_{x\to\infin}\left(\sqrt[x]{-1}\right)").unwrap(), "1");
    assert_eq!(evaluate(Mode::Algebra, r"\lim_{x\to\infty}\sqrt[x]{2}").unwrap(), "1");
    assert!(evaluate(Mode::Algebra, r"\lim_{x\to\infin}\left(\sqrt[x]{-1}\right)").unwrap_err().contains("not a real number"));
}

#[test]
fn limits_can_be_complex_in_imaginary_mode() {
    assert_eq!(evaluate(Mode::Imaginary, r"\lim_{x\to 0}\frac{\sin x}{x} + i").unwrap(), "1 + i");
    assert_eq!(evaluate(Mode::Imaginary, r"\lim_{x\to\infty}\left(2 + \frac{3i}{x} + i\right)").unwrap(), "2 + i");
    assert_eq!(evaluate(Mode::Algebra, r"\lim_{x\to 0}\frac{\sin x}{x}").unwrap(), "1");
}
