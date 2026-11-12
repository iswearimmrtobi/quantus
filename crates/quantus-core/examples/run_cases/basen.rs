//! Base-N mode (Basis-N): whole numbers in the word set (`wordSize`, `signed` among the case's
//! settings), written in the set base, the other bases as forms.

use super::*;

/// `expect.value` is the value as the word reads it, `result` how the set base writes it, `forms`
/// the other three bases by their form kind (`dec`, `hex`, `bin`, `oct`), and `wrapped` whether
/// the result warns that it was wrapped into the word.
pub(super) fn check(expect: &Value, calc: &Calculation) -> Outcome {
    let Some(value) = expect["value"].as_str().and_then(parse_rational) else { return generator("basen without a value") };
    let result = expect["result"].as_str().unwrap_or_default();
    if calc.value.to_rational() != value || calc.result != result {
        return Outcome::fail("wrong-value", format!("expected {result} ({})", show(&value)), calc.result.clone());
    }
    let wrapped = expect["wrapped"].as_bool().unwrap_or(false);
    if calc.warning.is_some() != wrapped {
        let got = calc.warning.clone().unwrap_or_else(|| "no warning".into());
        return Outcome::fail("wrapped", format!("expected wrapped = {wrapped}"), got);
    }
    for (kind, want) in expect["forms"].as_object().into_iter().flatten() {
        let got = calc.forms.iter().find(|f| serde_json::to_value(f.kind).ok().as_ref().and_then(Value::as_str) == Some(kind));
        match got {
            Some(form) if Some(form.latex.as_str()) == want.as_str() => {}
            Some(form) => return Outcome::fail("forms", format!("{kind}: expected {want}, got {}", form.latex), calc.result.clone()),
            None => return Outcome::fail("forms", format!("no {kind} form"), calc.result.clone()),
        }
    }
    Outcome::pass(calc.result.clone())
}
