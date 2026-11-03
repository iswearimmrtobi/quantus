//! The step texts in German Algebra mode are German: every explanation of the algebra pipeline
//! and the solver has a German text, picked by the settings' language.

use quantus_core::eval::{LatexCalculator, Variables};
use quantus_core::settings::{Language, Mode, Settings, StepDetail};

/// The explanations of `input` in Algebra mode, at the most detail, in `language`.
fn explanations(input: &str, language: Language) -> Vec<String> {
    let settings = Settings { mode: Mode::Algebra, language, step_detail: StepDetail::Maximal, ..Settings::default() };
    let calculation = LatexCalculator::default().with_settings(settings).evaluate(input, &Variables::new()).unwrap();
    calculation.steps.into_iter().filter_map(|step| step.explanation).collect()
}

/// Words that give an English text away.
const ENGLISH: &[&str] = &[" the ", "Combine", "Expand", "Distribute", "Factor", "Cancel", "Order ", "Add ", "Write ", "Multiply"];

#[allow(dead_code)]
fn assert_german(input: &str) -> Vec<String> {
    let texts = explanations(input, Language::De);
    assert!(!texts.is_empty(), "{input} has steps");
    for text in &texts {
        assert!(!ENGLISH.iter().any(|word| text.contains(word)), "{input}: English step '{text}' in {texts:?}");
    }
    texts
}

#[test]
fn expanding_and_collecting_is_german() {
    let texts = explanations("2(x+3) + (x+1)(x-1)", Language::De);
    assert!(texts.iter().any(|text| text.contains("ausmultiplizieren")), "{texts:?}");
    // And English stays English.
    assert!(explanations("2(x+3) + (x+1)(x-1)", Language::En).iter().any(|text| text.contains("Distribute") || text.contains("Expand")));
}

#[test]
fn cancelling_a_fraction_is_german() {
    let texts = explanations(r"\frac{x^2-1}{x-1}", Language::De);
    assert!(texts.iter().any(|text| text.contains("kürzen")), "{texts:?}");
}

#[test]
fn adding_fractions_is_german() {
    let texts = explanations(r"\frac{1}{x} + \frac{1}{x+1}", Language::De);
    assert!(texts.iter().any(|text| text == "Die Zähler über dem gemeinsamen Nenner addieren"), "{texts:?}");
}

#[test]
fn binomials_roots_and_powers_are_german() {
    let texts = explanations("(x+1)^2", Language::De);
    assert!(texts.iter().any(|text| text.ends_with("mit a = x, b = 1")) && texts.contains(&"Die Terme ausrechnen".to_string()), "{texts:?}");
    let texts = explanations(r"\sqrt{8a}", Language::De);
    assert!(texts[0].starts_with("Potenzen aus der Wurzel ziehen"), "{texts:?}");
    let texts = explanations(r"\frac{1}{\sqrt{a}}", Language::De);
    assert!(texts.contains(&r"Mit \sqrt{a} erweitern".to_string()), "{texts:?}");
    let texts = explanations(r"\sqrt{a}\sqrt{b}", Language::De);
    assert!(texts.iter().any(|text| text.starts_with("Unter einer Wurzel multiplizieren") && text.contains(", für ")), "{texts:?}");
}
