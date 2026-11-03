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
const ENGLISH: &[&str] = &[
    " the ", " as ", " and ", "Combine", "Expand", "Distribute", "Factor", "Cancel", "Order ", "Add ", "Write ", "Multiply", "Use ",
    "Take ", "Apply", "Simplify", "Note:", "Solve", "Subtract", "Divide", "Try ", "Split", "Rationalize", "Evaluate", "Recognise",
    "Group", "Raise", "Rewrite", "Extend", "Find ", "Complete", "Half", "Negative", "Power of", "Same base", "Square", "Cube",
];

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

#[test]
fn completing_the_square_and_fractions_are_german() {
    for (input, wanted) in [
        (r"\frac{1}{x} + \frac{1}{x+1}", "Hauptnenner"),
        (r"\frac{x^2-1}{x-1}", "Hinweis: x ≠ 1"),
        (r"\sin^2(x) + \cos^2(x)", "Pythagoras"),
    ] {
        let texts = explanations(input, Language::De);
        assert!(texts.iter().any(|text| text.contains(wanted)), "{input}: {wanted} in {texts:?}");
    }
}

/// Inputs across the pipeline and the solver: none of their steps is English.
#[test]
fn no_step_is_english() {
    for input in [
        "2(x+3) + (x+1)(x-1)",
        r"\frac{x^2-1}{x-1}",
        r"\frac{1}{x} + \frac{1}{x+1}",
        "(x+1)^2",
        "(x-2)^3",
        "x^2 + 4x + 1",
        r"\sqrt{8a}",
        r"\frac{1}{\sqrt{a}}",
        r"\sqrt{a}\sqrt{b}",
        r"\frac{\frac{1}{x}}{\frac{2}{x^2}}",
        r"\sin^2(x) + \cos^2(x)",
        r"\ln(a) + \ln(b)",
        "|a||b|",
        "a^2 a^3",
        "2x+3=7",
        "x^2-5x+6=0",
        "x^3-6x^2+11x-6=0",
        r"\frac{x}{x-1} = 2",
        "2x + y = 5; x - y = 1",
        r"\sqrt{x+1} = 3",
    ] {
        let texts = explanations(input, Language::De);
        if std::env::var("SHOW_STEPS").is_ok() {
            for text in &texts {
                eprintln!("STEP {input} | {text}");
            }
            continue;
        }
        for text in &texts {
            assert!(!ENGLISH.iter().any(|word| text.contains(word)), "{input}: English step '{text}'");
        }
    }
}
