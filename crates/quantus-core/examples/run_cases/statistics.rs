//! Statistics (Statistik): `request` goes to `api::statistics` as the mode sends it.

use super::*;
use quantus_core::api::{StatisticValue, StatisticsRequest};

pub(super) fn run(case: &Value) -> Outcome {
    let expect = &case["expect"];
    let Ok(request) = serde_json::from_value::<StatisticsRequest>(case["request"].clone()) else {
        return generator("unreadable request");
    };
    let result = api::statistics(&settings_of(case), &request);
    if expect["kind"] == "error" {
        return expect_error(result, expect, |r| format!("{} values", r.values.len()));
    }
    let response = match result {
        Ok(r) => r,
        Err(e) => return Outcome::error(e.error),
    };
    let shown = response.values.iter().map(|v| format!("{} = {}", v.key, v.latex)).collect::<Vec<_>>().join(", ");
    // The one- or two-variable values must be exactly the expected keys; the regression and the
    // estimates must contain theirs.
    let keys: BTreeSet<&str> = response.values.iter().map(|v| v.key.as_str()).collect();
    let want: BTreeSet<&str> = expect["values"].as_object().into_iter().flatten().map(|(k, _)| k.as_str()).collect();
    if keys != want {
        return Outcome::fail("keys", format!("expected {want:?}, got {keys:?}"), shown);
    }
    for (group, got) in [("values", &response.values), ("regression", &response.regression), ("estimates", &response.estimates)] {
        if let Err(detail) = compare(got, &expect[group]) {
            return Outcome::fail("wrong-value", format!("{group}: {detail}"), shown);
        }
    }
    Outcome::pass(shown)
}

/// Each `key: [value, rel_tol]` of `want` against the value of that key in `got`.
pub(super) fn compare(got: &[StatisticValue], want: &Value) -> Result<(), String> {
    for (key, pair) in want.as_object().into_iter().flatten() {
        let (Some(value), Some(tol)) = (pair[0].as_str().and_then(parse_rational), pair[1].as_str().and_then(parse_rational)) else {
            return Err(format!("{key}: unparsable expectation"));
        };
        let Some(found) = got.iter().find(|v| &v.key == key) else { return Err(format!("{key} is missing")) };
        let Some(actual) = parse_rational(&found.value) else { return Err(format!("{key}: unparsable {}", found.value)) };
        if !close(&actual, &value, &tol) {
            return Err(format!("{key}: expected {}, got {}", show(&value), found.value));
        }
    }
    Ok(())
}
