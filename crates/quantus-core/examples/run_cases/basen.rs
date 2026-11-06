//! Base-N mode (Basis-N): 32-bit whole numbers in the set base, the other bases as forms.

use super::*;

/// `expect.value` is the signed value, `result` how the set base writes it, and `forms` the other
/// three bases by their form kind (`dec`, `hex`, `bin`, `oct`).
pub(super) fn check(expect: &Value, calc: &Calculation) -> Outcome {
    let Some(value) = expect["value"].as_str().and_then(parse_rational) else { return generator("basen without a value") };
    let result = expect["result"].as_str().unwrap_or_default();
    if calc.value.to_rational() != value || calc.result != result {
        return Outcome::fail("wrong-value", format!("expected {result} ({})", show(&value)), calc.result.clone());
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
