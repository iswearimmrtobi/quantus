//! Geometry mode: a scene and a task go to `api::geometry`; the answer is its `relation` and the
//! result lines `name = … = value`, read back as numbers or points `(x|y|z)`.

use super::*;
use quantus_core::api::GeometryRequest;
use quantus_core::settings::Mode;

pub(super) fn run(case: &Value) -> Outcome {
    let expect = &case["expect"];
    let Ok(request) = serde_json::from_value::<GeometryRequest>(case["request"].clone()) else {
        return generator("unreadable request");
    };
    let result = api::geometry(&settings_of(case), &request);
    // A failed task is an answer with `error`, not an `Err`; a scene that cannot be built is either.
    let result = match result {
        Ok(response) => match response.task.as_ref().and_then(|t| t.error.clone()) {
            Some(error) => Err(api::ApiError { error, position: None }),
            None => Ok(response),
        },
        Err(e) => Err(e),
    };
    if expect["kind"] == "error" {
        return expect_error(result, expect, |r| r.task.as_ref().map(|t| t.result.join("; ")).unwrap_or_default());
    }
    let response = match result {
        Ok(r) => r,
        Err(e) => return Outcome::error(e.error),
    };
    if let Some(error) = response.objects.iter().find_map(|o| o.error.clone()) {
        return Outcome::error(error);
    }
    let Some(task) = response.task else { return Outcome::error("no answer to the task".into()) };
    let shown = task.result.join("; ");
    if let Some(relation) = expect["relation"].as_str() {
        if task.relation.as_deref() != Some(relation) {
            return Outcome::fail("relation", format!("expected {relation}, got {:?}", task.relation), shown);
        }
    }
    // `name = … = value`: the name before the first `=`, the value after the last.
    let lines: Vec<(&str, &str)> =
        task.result.iter().filter_map(|l| Some((l.split(" = ").next()?.trim(), l.rsplit(" = ").next()?.trim()))).collect();
    let find = |name: &str| lines.iter().find(|(n, _)| *n == name).map(|(_, v)| *v);
    let numbers = Settings { mode: Mode::Algebra, ..Settings::default() };
    for item in expect["values"].as_array().into_iter().flatten() {
        let name = item[0].as_str().unwrap_or_default();
        let (Some(want), Some(tol)) = (item[1].as_str().and_then(parse_rational), item[2].as_str().and_then(parse_rational)) else {
            return generator("unparsable expectation");
        };
        let Some(text) = find(name) else { return Outcome::fail("solutions", format!("no line {name} = …"), shown) };
        // An angle is shown exactly and then `\approx 54{,}7356^\circ`: the decimal is compared.
        let text = text.rsplit("\\approx").next().unwrap_or(text).replace("^\\circ", "");
        let got = match latex_complex(text.trim(), &numbers, &Variables::new()) {
            Ok(v) => v.0,
            Err(e) => return Outcome::fail("result-unparsable", e, shown),
        };
        if !close(&got, &want, &tol) {
            return Outcome::fail("wrong-value", format!("{name}: expected {}, got {text}", show(&want)), shown);
        }
    }
    for item in expect["points"].as_array().into_iter().flatten() {
        let name = item[0].as_str().unwrap_or_default();
        let Some(text) = find(name) else { return Outcome::fail("solutions", format!("no point {name} = …"), shown) };
        let inner = text.trim().trim_start_matches('(').trim_end_matches(')');
        let coordinates: Vec<&str> = inner.split('|').collect();
        let want: Vec<RBig> = item[1].as_array().into_iter().flatten().filter_map(|v| v.as_str().and_then(parse_rational)).collect();
        if coordinates.len() != want.len() {
            return Outcome::fail("wrong-value", format!("{name}: expected {} coordinates, got {text}", want.len()), shown);
        }
        for (cell, want) in coordinates.iter().zip(&want) {
            match latex_complex(cell.trim(), &numbers, &Variables::new()) {
                Ok(v) if close(&v.0, want, &exact_tolerance()) && v.1.is_zero() => {}
                Ok(_) => return Outcome::fail("wrong-value", format!("{name}: expected {}, got {text}", show(want)), shown),
                Err(e) => return Outcome::fail("result-unparsable", e, shown),
            }
        }
    }
    Outcome::pass(shown)
}
