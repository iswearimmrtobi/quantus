//! The modes added after the linear systems mode. A case of a mode with its own API names it in
//! `api` and carries the `request` that mode's shell sends; the others go through
//! `LatexCalculator::evaluate` like the first categories and only add an `expect.kind`.

use super::*;
use quantus_core::eval::Calculation;

#[path = "basen.rs"]
mod basen;
#[path = "distributions.rs"]
mod distributions;
#[path = "matrix.rs"]
mod matrix;
#[path = "statistics.rs"]
mod statistics;
#[path = "verify.rs"]
mod verify;

/// A case with `api`: the request goes to that API function.
pub(super) fn run_api_case(api: &str, case: &Value) -> Outcome {
    match api {
        "statistics" => statistics::run(case),
        "distribution" => distributions::run(case),
        _ => generator(&format!("unknown api {api:?}")),
    }
}

/// The expectation kinds of the new modes that `evaluate` answers.
pub(super) fn check_calculation(kind: &str, settings: &Settings, expect: &Value, calc: &Calculation) -> Outcome {
    match kind {
        "truth" => verify::check(settings, expect, calc),
        "basen" => basen::check(expect, calc),
        "matrix" => matrix::check(settings, expect, calc),
        _ => generator("unknown expectation kind"),
    }
}

/// The outcome of an API call that should have failed with a message containing `contains`.
pub(super) fn expect_error<T>(result: Result<T, api::ApiError>, expect: &Value, shown: impl Fn(&T) -> String) -> Outcome {
    let contains = expect["contains"].as_str().unwrap_or_default();
    match result {
        Err(e) if e.error.contains(contains) => Outcome::pass(format!("error: {}", e.error)),
        Err(e) => Outcome::fail("wrong-error", format!("expected an error containing '{contains}'"), format!("error: {}", e.error)),
        Ok(response) => Outcome {
            status: "UNEXPECTED_OK",
            reason: String::new(),
            detail: "expected an error".into(),
            actual: shown(&response),
            note: String::new(),
        },
    }
}
