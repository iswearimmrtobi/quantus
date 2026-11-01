use std::collections::HashMap;

use quantus_core::eval::LatexCalculator;
use quantus_core::settings::{AngleUnit, ComplexForm, Mode, Settings};

fn settings(complex_form: ComplexForm, angle_unit: AngleUnit, exact_roots: bool) -> Settings {
    Settings { mode: Mode::Imaginary, complex_form, angle_unit, exact_roots, ..Settings::default() }
}

fn evaluate(settings: Settings, latex: &str) -> String {
    LatexCalculator::new(10)
        .with_settings(settings)
        .evaluate(latex, &HashMap::new())
        .map(|c| c.result)
        .unwrap_or_else(|e| format!("error: {e}"))
}

#[test]
fn polar_results_in_either_angle_unit() {
    assert_eq!(evaluate(settings(ComplexForm::Polar, AngleUnit::Degree, false), "3+4i"), r"5 \angle 53{,}13010235^{\circ}");
    assert_eq!(evaluate(settings(ComplexForm::Polar, AngleUnit::Degree, true), "1+i"), r"\sqrt{2} \angle 45^{\circ}");
    assert_eq!(evaluate(settings(ComplexForm::Polar, AngleUnit::Radian, true), "1+i"), r"\sqrt{2} \angle \frac{\pi}{4}");
    assert_eq!(evaluate(settings(ComplexForm::Polar, AngleUnit::Radian, false), "-2i"), r"2 \angle -1{,}570796327");
}

#[test]
fn the_angle_sign_types_a_polar_value() {
    let degrees = settings(ComplexForm::Rectangular, AngleUnit::Degree, true);
    assert_eq!(evaluate(degrees.clone(), r"2\angle 90"), "2i");
    assert_eq!(evaluate(degrees.clone(), r"\sqrt{2}\angle 45 + 1"), "2 + i");
    assert_eq!(evaluate(degrees, r"2\angle\pi\,\mathrm{rad}"), "-2");
    let radians = settings(ComplexForm::Rectangular, AngleUnit::Radian, true);
    assert_eq!(evaluate(radians.clone(), r"2\angle\frac{\pi}{2}"), "2i");
    assert_eq!(evaluate(radians, r"2\angle 60^\circ"), r"1 + \sqrt{3}i");
    let physics = Settings { mode: Mode::Physics, ..Settings::default() };
    assert!(evaluate(physics, r"2\angle 90").starts_with("error"));
}

#[test]
fn a_complex_result_offers_the_other_form() {
    let calculation = LatexCalculator::new(10)
        .with_settings(settings(ComplexForm::Rectangular, AngleUnit::Degree, true))
        .evaluate("1+i", &HashMap::new())
        .unwrap();
    let forms: Vec<_> = calculation.forms.iter().map(|f| f.latex.as_str()).collect();
    assert_eq!(forms, [r"\sqrt{2} \angle 45^{\circ}"]);
}
