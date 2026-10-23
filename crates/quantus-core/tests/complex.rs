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

#[test]
fn euler_and_the_functions_of_a_complex_argument() {
    assert_eq!(imaginary(r"e^{i\pi}"), "-1");
    assert_eq!(imaginary(r"e^{i\pi/2}"), "i");
    assert!(imaginary(r"e^{i}").starts_with("0{,}54030230586813971740"));
    assert!(imaginary(r"e^{i}").contains(" + 0{,}84147098480789650665"));
    assert!(imaginary(r"\sin(i)").starts_with("1{,}17520119364380145688"));
    assert!(imaginary(r"\cos(1+i)").contains(" - 0{,}98889770576286509638"));
    assert!(imaginary(r"i^i").starts_with("0{,}2078795763507619085"));
}

#[test]
fn logarithms_roots_and_powers_with_complex_values() {
    let ln = |latex: &str| imaginary(latex);
    assert!(ln(r"\ln(-1)").starts_with("3{,}14159265358979323846") && ln(r"\ln(-1)").ends_with('i'));
    assert!(ln(r"\ln(i)").starts_with("1{,}5707963267948966192") && ln(r"\ln(i)").ends_with('i'));
    let parts = |latex: &str| imaginary(latex).split(" + ").map(str::to_string).collect::<Vec<_>>();
    let root_i = parts(r"\sqrt{i}");
    assert!(root_i[0].starts_with("0{,}70710678118654752440") && root_i[1].starts_with("0{,}70710678118654752440"));
    let root = parts(r"(1+i)^{1/2}");
    assert!(root[0].starts_with("1{,}0986841134678099660") && root[1].starts_with("0{,}45508986056222734130"));
}

#[test]
fn modulus_argument_conjugate_and_parts() {
    assert_eq!(imaginary(r"|3+4i|"), "5");
    assert_eq!(imaginary(r"\overline{3+4i}"), "3 - 4i");
    assert_eq!(imaginary(r"\Re(3+4i)"), "3");
    assert_eq!(imaginary(r"\Im(3+4i)"), "4");
    assert!(imaginary(r"\arg(i)").starts_with("1{,}5707963267948966192"));
}

#[test]
fn the_other_modes_keep_rejecting_a_logarithm_of_a_negative_number() {
    for mode in [Mode::Physics, Mode::Algebra] {
        assert!(evaluate(mode, r"\ln(-1)").unwrap_err().contains("only defined for positive"), "{mode:?}");
    }
}

#[test]
fn quadratics_with_a_negative_discriminant_have_complex_roots() {
    assert_eq!(imaginary(r"x^2+1=0"), r"x = -i \lor x = i");
    assert_eq!(imaginary(r"x^2+2x+5=0"), r"x = -1 - 2i \lor x = -1 + 2i");
    assert_eq!(imaginary(r"2x^2+x+1=0").matches("i").count(), 2);
    assert!(evaluate(Mode::Physics, r"x^2+1=0").unwrap_err().contains("no real solution"));
}

#[test]
fn a_variable_can_hold_a_complex_number() {
    use quantus_core::session::Session;

    let settings = Settings { mode: Mode::Imaginary, ..Settings::default() };
    let mut session = Session::new("complex");
    let (_, variable) = session.evaluate_with("z_1 = 3+4i", None, &settings).unwrap();
    assert_eq!(variable.unwrap().latex, "3 + 4i");
    let (calculation, _) = session.evaluate_with(r"z_1 \cdot z_1", None, &settings).unwrap();
    assert_eq!(calculation.result, "-7 + 24i");
}
