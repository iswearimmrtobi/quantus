//! Distributions (Verteilungen): `request` goes to `api::distribution` as the mode sends it.

use super::*;
use quantus_core::api::DistributionRequest;

/// `expect.values` are the results in order (one, or one per k), each `[value, rel_tol]`.
pub(super) fn run(case: &Value) -> Outcome {
    let expect = &case["expect"];
    let Ok(request) = serde_json::from_value::<DistributionRequest>(case["request"].clone()) else {
        return generator("unreadable request");
    };
    let result = api::distribution(&settings_of(case), &request);
    if expect["kind"] == "error" {
        return expect_error(result, expect, |r| r.results.iter().map(|x| x.latex.clone()).collect::<Vec<_>>().join(", "));
    }
    let response = match result {
        Ok(r) => r,
        Err(e) => return Outcome::error(e.error),
    };
    let shown = response.results.iter().map(|x| x.latex.clone()).collect::<Vec<_>>().join(", ");
    let want = expect["values"].as_array().cloned().unwrap_or_default();
    if want.len() != response.results.len() {
        return Outcome::fail("solutions", format!("expected {} results, got {}", want.len(), response.results.len()), shown);
    }
    for (i, (got, pair)) in response.results.iter().zip(&want).enumerate() {
        let (Some(value), Some(tol)) = (pair[0].as_str().and_then(parse_rational), pair[1].as_str().and_then(parse_rational)) else {
            return generator("unparsable expectation");
        };
        let Some(actual) = parse_rational(&got.value) else { return Outcome::fail("result-unparsable", got.value.clone(), shown) };
        if !close(&actual, &value, &tol) {
            return Outcome::fail("wrong-value", format!("result {}: expected {}, got {}", i + 1, show(&value), got.value), shown);
        }
    }
    Outcome::pass(shown)
}
