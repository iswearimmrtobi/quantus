//! The Electronics mode (`api::solve_circuit`) and the Kirchhoff mode (`api::kirchhoff`): each
//! part's voltage, current and power, read back from their LaTeX in SI units.

use super::*;
use quantus_core::api::{CircuitRequest, KirchhoffRequest};
use quantus_core::settings::Mode;

pub(super) fn run(api: &str, case: &Value) -> Outcome {
    let settings = settings_of(case);
    let expect = &case["expect"];
    let result = if api == "solve_circuit" {
        let Ok(request) = serde_json::from_value::<CircuitRequest>(case["request"].clone()) else { return generator("unreadable request") };
        api::solve_circuit(&settings, &request).map(|r| serde_json::to_value(r).expect("response to JSON"))
    } else {
        let Ok(request) = serde_json::from_value::<KirchhoffRequest>(case["request"].clone()) else { return generator("unreadable request") };
        // The equations without a single solution come back as `error`.
        match api::kirchhoff(&settings, &request) {
            Ok(r) if r.error.is_some() => Err(api::ApiError::new(r.error.clone().unwrap_or_default())),
            other => other.map(|r| serde_json::to_value(r).expect("response to JSON")),
        }
    };
    if expect["kind"] == "error" {
        return expect_error(result, expect, |r| r["elements"].to_string().chars().take(200).collect());
    }
    let response = match result {
        Ok(r) => r,
        Err(e) => return Outcome::error(e.error),
    };
    let elements = response["elements"].as_array().cloned().unwrap_or_default();
    let shown = elements
        .iter()
        .map(|e| format!("{}: {}, {}", e["id"].as_str().unwrap_or("?"), e["voltage"]["result"].as_str().unwrap_or("?"), e["current"]["result"].as_str().unwrap_or("?")))
        .collect::<Vec<_>>()
        .join("; ");
    if expect["kind"] == "equivalent" {
        let got = response["equivalent"]["resistance"]["result"].as_str().unwrap_or_default();
        return match compare(got, expect["resistance"].as_str().unwrap_or_default()) {
            Ok(()) => Outcome::pass(response["equivalent"]["latex"].as_str().unwrap_or_default().to_string()),
            Err(detail) => Outcome::fail("wrong-value", format!("R_AB: {detail}"), response["equivalent"]["latex"].to_string()),
        };
    }
    for (id, want) in expect["elements"].as_object().into_iter().flatten() {
        let Some(element) = elements.iter().find(|e| e["id"] == id.as_str()) else {
            return Outcome::fail("solutions", format!("no part {id}"), shown);
        };
        for (k, quantity) in ["voltage", "current", "power"].iter().enumerate() {
            let got = element[*quantity]["result"].as_str().unwrap_or_default();
            if let Err(detail) = compare(got, want[k].as_str().unwrap_or_default()) {
                return Outcome::fail("wrong-value", format!("{id} {quantity}: {detail}"), shown);
            }
        }
    }
    Outcome::pass(shown)
}

/// A value with its unit, as the response writes it (`625\,\mathrm{\mu A}`), against the SI value.
fn compare(latex: &str, want: &str) -> Result<(), String> {
    let Some(want) = parse_rational(want) else { return Err("unparsable expectation".into()) };
    let calculator = LatexCalculator::new(PRECISION).with_settings(Settings { mode: Mode::Physics, ..Settings::default() });
    let calc = calculator.evaluate(latex, &Variables::new()).map_err(|e| format!("{latex}: {e}"))?;
    let got = si_value(&calc.quantity, &calc.value)?;
    let tol = RBig::from_parts(IBig::ONE, UBig::from(10u8).pow(40));
    if close(&got, &want, &tol) { Ok(()) } else { Err(format!("expected {} (SI), got {latex}", show(&want))) }
}
