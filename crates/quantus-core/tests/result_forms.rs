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

fn forms(settings: Settings, latex: &str) -> Vec<(String, String)> {
    calculate(settings, latex)
        .forms
        .into_iter()
        .map(|form| (serde_json::to_value(form.kind).unwrap().as_str().unwrap().to_string(), form.latex))
        .collect()
}

fn kinds(settings: Settings, latex: &str) -> Vec<String> {
    forms(settings, latex).into_iter().map(|(kind, _)| kind).collect()
}

#[test]
fn a_fraction_comes_improper_mixed_recurring_and_factored() {
    assert_eq!(
        forms(Settings::default(), r"\frac{7}{3}"),
        [
            ("fraction".to_string(), r"\frac{7}{3}".to_string()),
            ("mixedFraction".to_string(), r"2\frac{1}{3}".to_string()),
            ("recurring".to_string(), r"2{,}\overline{3}".to_string()),
        ]
    );
    let mixed = Settings { fraction_form: quantus_core::settings::FractionForm::Mixed, ..Settings::default() };
    assert_eq!(kinds(mixed, r"\frac{7}{3}"), ["mixedFraction", "fraction", "recurring"]);
    let no_recurring = Settings { recurring: false, ..Settings::default() };
    assert_eq!(kinds(no_recurring, r"\frac{1}{3}"), ["fraction"]);
}

#[test]
fn whole_numbers_offer_their_prime_factors() {
    assert_eq!(forms(Settings::default(), "120"), [("primeFactors".to_string(), r"2^{3} \cdot 3 \cdot 5".to_string())]);
    assert_eq!(kinds(Settings::default(), r"\frac{12}{35}"), ["fraction", "recurring", "primeFactors"]);
    assert!(forms(Settings::default(), "1").is_empty());
}

#[test]
fn units_stay_with_the_fraction() {
    assert_eq!(forms(Settings::default(), r"\frac{1}{4}\,\mathrm{m}"), [("fraction".to_string(), r"\frac{1}{4}\,\mathrm{m}".to_string())]);
}

#[test]
fn roots_switch_between_exact_and_decimal() {
    assert_eq!(kinds(Settings::default(), r"\sqrt{12}"), ["exact"]);
    assert_eq!(forms(Settings::default(), r"\sqrt{12}")[0].1, r"2\sqrt{3}");
    let exact = Settings { exact_roots: true, ..Settings::default() };
    assert_eq!(kinds(exact, r"\sqrt{12}"), ["decimal"]);
}

#[test]
fn a_base_n_result_has_its_bits_as_a_form() {
    use quantus_core::settings::{NumberBase, WordSize};
    let settings = Settings { mode: Mode::BaseN, number_base: NumberBase::Hex, word_size: WordSize::W16, ..Settings::default() };
    let forms = forms(settings, "F0F");
    assert_eq!(forms.last().unwrap(), &("bits".to_string(), r"\mathtt{0000\,1111\,0000\,1111}".to_string()));
    assert_eq!(forms.iter().map(|(kind, _)| kind.as_str()).collect::<Vec<_>>(), ["dec", "bin", "oct", "bits"]);
}
