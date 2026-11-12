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
    // At 32 and 64 bits, the pattern read as a float: its class, exact value, rounding error.
    if let Some(want) = expect.get("ieee") {
        let Some(got) = calc.bits.as_ref().and_then(|bits| bits.ieee.as_ref()) else {
            return Outcome::fail("ieee", "no IEEE view".into(), calc.result.clone());
        };
        let got = serde_json::to_value(got).unwrap_or_default();
        for key in ["class", "exact", "error"] {
            if let Some(want) = want.get(key) {
                if got.get(key) != Some(want) {
                    return Outcome::fail("ieee", format!("{key}: expected {want}, got {}", got.get(key).unwrap_or(&Value::Null)), calc.result.clone());
                }
            }
        }
    }
    Outcome::pass(calc.result.clone())
}
