//! Verify mode (Prüfen): an (in)equality or a chain of them is true or false.

use super::*;
use quantus_core::settings::Language;

/// `expect.value` is `true` or `false`: the value must be 1 or 0 and the result the word for it
/// in the set language.
pub(super) fn check(settings: &Settings, expect: &Value, calc: &Calculation) -> Outcome {
    let Some(holds) = expect["value"].as_bool() else { return generator("truth without a value") };
    let word = match (settings.language, holds) {
        (Language::De, true) => "wahr",
        (Language::De, false) => "falsch",
        (_, true) => "true",
        (_, false) => "false",
    };
    let want = format!("\\text{{{word}}}");
    let value = calc.value.to_rational();
    if value != RBig::from(u8::from(holds)) || calc.result != want {
        return Outcome::fail("wrong-value", format!("expected {want} ({})", u8::from(holds)), calc.result.clone());
    }
    Outcome::pass(calc.result.clone())
}
