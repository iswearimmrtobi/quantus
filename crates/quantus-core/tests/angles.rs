use std::collections::HashMap;

use quantus_core::eval::LatexCalculator;
use quantus_core::forms::FormKind;
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

#[test]
fn a_degree_sign_is_degrees_in_either_unit() {
    assert_eq!(degrees(r"30^\circ"), "30");
    assert_eq!(radians(r"180^\circ"), r"\pi");
    assert_eq!(degrees(r"\sin 30°"), "0{,}5");
    assert_eq!(radians(r"\cos 60°"), "0{,}5");
}

#[test]
fn degrees_minutes_and_seconds_are_one_angle() {
    assert_eq!(degrees(r"2^\circ 30'"), "2{,}5");
    assert_eq!(degrees(r"2°20′30″"), radians(r"2{,}341666667"));
    assert_eq!(degrees(r"1^{\circ}0^{\prime}36^{\prime\prime}"), "1{,}01");
    assert_eq!(degrees(r"10^\circ 30' + 0^\circ 30'"), "11");
    let input = LatexCalculator::new(10)
        .with_settings(Settings { angle_unit: AngleUnit::Degree, ..Settings::default() })
        .evaluate(r"2^\circ 20' 30''", &HashMap::new())
        .unwrap()
        .input;
    assert_eq!(input, r"2^{\circ}20'30''");
}

fn forms(angle_unit: AngleUnit, latex: &str) -> Vec<(FormKind, String)> {
    let settings = Settings { mode: Mode::Physics, angle_unit, ..Settings::default() };
    let calculation = LatexCalculator::new(10).with_settings(settings).evaluate(latex, &HashMap::new()).unwrap();
    calculation.forms.into_iter().map(|f| (f.kind, f.latex)).collect()
}

#[test]
fn an_angle_in_degrees_has_a_sexagesimal_form() {
    assert_eq!(forms(AngleUnit::Degree, r"2^\circ 20' 30''"), [(FormKind::Sexagesimal, "2^{\\circ}20'30''".to_string())]);
    assert_eq!(forms(AngleUnit::Degree, r"\arctan\frac{1}{2}")[0].1, "26^{\\circ}33'54{,}1842''");
    assert!(forms(AngleUnit::Radian, r"\arctan 1").is_empty());
    assert!(forms(AngleUnit::Degree, r"2+3").is_empty());
}
