use std::collections::HashMap;

use quantus_core::eval::LatexCalculator;
use quantus_core::settings::{AngleUnit, Mode, Settings};

fn evaluate(mode: Mode, angle_unit: AngleUnit, latex: &str) -> String {
    let settings = Settings { mode, angle_unit, exact_roots: true, precision: 10, ..Settings::default() };
    LatexCalculator::new(10)
        .with_settings(settings)
        .evaluate(latex, &HashMap::new())
        .map(|c| c.result)
        .unwrap_or_else(|e| format!("error: {e}"))
}

fn degrees(latex: &str) -> String {
    evaluate(Mode::Physics, AngleUnit::Degree, latex)
}

fn radians(latex: &str) -> String {
    evaluate(Mode::Physics, AngleUnit::Radian, latex)
}

#[test]
fn plain_numbers_are_read_in_the_angle_unit() {
    assert_eq!(degrees(r"\sin 30"), "0{,}5");
    assert_eq!(degrees(r"\cos(60)"), "0{,}5");
    assert_eq!(degrees(r"\tan 45"), "1");
    assert_eq!(radians(r"\sin 0"), "0");
    assert_eq!(radians(r"\cos\pi"), "-1");
}

#[test]
fn inverse_functions_answer_in_the_angle_unit() {
    assert_eq!(degrees(r"\arcsin\frac{1}{2}"), "30");
    assert_eq!(degrees(r"\arctan 1"), "45");
    assert_eq!(degrees(r"\sin^{-1}(1)"), "90");
    assert_eq!(radians(r"\arctan 1"), r"\frac{\pi}{4}");
}

#[test]
fn a_marked_value_keeps_its_unit() {
    assert_eq!(radians(r"\sin 30^\circ"), "0{,}5");
    assert_eq!(degrees(r"\sin 30^\circ"), "0{,}5");
    assert_eq!(degrees(r"\sin(\pi\,\mathrm{rad})"), "0");
    assert_eq!(evaluate(Mode::Algebra, AngleUnit::Degree, r"\cos(\pi\,\mathrm{rad})"), "-1");
}

#[test]
fn the_unit_applies_inside_other_constructs() {
    assert_eq!(evaluate(Mode::Algebra, AngleUnit::Degree, r"\sin 30 + \cos 60"), "1");
    // d/dx sin x at 0, with x in degrees, is π/180.
    assert_eq!(degrees(r"\left.\frac{d}{dx}\sin x\right|_{x=0}"), radians(r"\frac{\pi}{180}"));
}

#[test]
fn the_argument_of_a_complex_number_follows_the_unit() {
    assert_eq!(evaluate(Mode::Imaginary, AngleUnit::Degree, r"\arg(1+i)"), "45");
    assert_eq!(evaluate(Mode::Imaginary, AngleUnit::Radian, r"\arg(1+i)"), r"\frac{\pi}{4}");
}
