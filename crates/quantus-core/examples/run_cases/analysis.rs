//! Analysis mode: the curve sketch (`api::analyze`), tangent and normal (`api::tangent`), the area
//! (`api::area`) and the table of values (`api::value_table`), each with the request the mode sends.

use super::*;
use quantus_core::api::{AnalysisRequest, AreaRequest, TableRequest, TangentRequest};

/// The point kinds a curve sketch is checked for: each must be exactly the expected list.
const KINDS: [&str; 7] = ["zero", "yIntercept", "max", "min", "saddle", "inflection", "hole"];

pub(super) fn run(api: &str, case: &Value) -> Outcome {
    let settings = settings_of(case);
    let request = case["request"].clone();
    let expect = &case["expect"];
    macro_rules! call {
        ($request:ty, $function:path) => {{
            let Ok(request) = serde_json::from_value::<$request>(request) else { return generator("unreadable request") };
            $function(&settings, &request).map(|r| serde_json::to_value(r).expect("response to JSON"))
        }};
    }
    let result = match api {
        "analyze" => call!(AnalysisRequest, api::analyze),
        "tangent" => call!(TangentRequest, api::tangent),
        "area" => call!(AreaRequest, api::area),
        _ => call!(TableRequest, api::value_table),
    };
    if expect["kind"] == "error" {
        return expect_error(result, expect, |r| r.to_string().chars().take(200).collect());
    }
    let response = match result {
        Ok(r) => r,
        Err(e) => return Outcome::error(e.error),
    };
    let checked = match expect["kind"].as_str().unwrap_or_default() {
        "analysis" => sketch(&response, expect),
        "tangent" => tangent(&response, expect),
        "area" => area(&response, expect, &settings),
        "table" => table(&response, expect, &settings),
        _ => return generator("unknown expectation kind"),
    };
    let shown = match api {
        "analyze" => response["points"].as_array().into_iter().flatten().filter_map(|p| p["latex"].as_str()).collect::<Vec<_>>().join(", "),
        "tangent" => format!("{}, {}", response["tangent"]["latex"], response["normal"]["latex"]),
        "area" => format!("A = {}, signed {}", response["area"], response["signed"]),
        _ => format!("{} rows", response["rows"].as_array().map_or(0, Vec::len)),
    };
    match checked {
        Ok(()) => Outcome::pass(shown),
        Err((reason, detail)) => Outcome::fail(reason, detail, shown),
    }
}

type Checked = Result<(), (&'static str, String)>;

fn tol(expect: &Value) -> f64 {
    expect["tol"].as_str().and_then(|t| t.parse().ok()).unwrap_or(1e-9)
}

fn near(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol * b.abs().max(1.0)
}

fn number(v: &Value) -> Option<f64> {
    v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

fn sketch(response: &Value, expect: &Value) -> Checked {
    let tol = tol(expect);
    let points = response["points"].as_array().cloned().unwrap_or_default();
    for kind in KINDS {
        let mut got: Vec<(f64, f64)> = points
            .iter()
            .filter(|p| p["kind"] == kind)
            .filter_map(|p| Some((p["x"].as_f64()?, p["y"].as_f64()?)))
            .collect();
        let want: Vec<(f64, f64)> = expect["points"][kind]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|p| Some((number(&p[0])?, number(&p[1])?)))
            .collect();
        got.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let same = got.len() == want.len() && got.iter().zip(&want).all(|(g, w)| near(g.0, w.0, tol) && near(g.1, w.1, tol));
        if !same {
            return Err(("points", format!("{kind}: expected {want:?}, got {got:?}")));
        }
    }
    let got = response["asymptotes"].as_array().cloned().unwrap_or_default();
    let want = expect["asymptotes"].as_array().cloned().unwrap_or_default();
    let matches = |g: &Value, w: &Value| {
        g["kind"] == w["kind"]
            && ["x", "slope", "intercept"].iter().all(|key| match number(&w[*key]) {
                Some(v) => g[*key].as_f64().is_some_and(|a| near(a, v, tol)),
                None => true,
            })
            && (w["side"].is_null() || g["side"] == w["side"])
    };
    let mut free: Vec<&Value> = got.iter().collect();
    for w in &want {
        let Some(i) = free.iter().position(|g| matches(g, w)) else {
            return Err(("asymptotes", format!("expected {w}, got {}", Value::Array(got.clone()))));
        };
        free.remove(i);
    }
    if !free.is_empty() {
        return Err(("asymptotes", format!("unexpected {}", Value::Array(free.into_iter().cloned().collect()))));
    }
    Ok(())
}

fn line(got: &Value, want: &Value, tol: f64, name: &str) -> Checked {
    for key in ["x", "slope", "intercept"] {
        if let Some(v) = number(&want[key]) {
            if !got[key].as_f64().is_some_and(|a| near(a, v, tol)) {
                return Err(("wrong-value", format!("{name} {key}: expected {v}, got {}", got[key])));
            }
        }
    }
    Ok(())
}

fn tangent(response: &Value, expect: &Value) -> Checked {
    let tol = tol(expect);
    let (x, y) = (number(&expect["point"][0]).unwrap_or_default(), number(&expect["point"][1]).unwrap_or_default());
    let point = &response["point"];
    if !(point["x"].as_f64().is_some_and(|a| near(a, x, tol)) && point["y"].as_f64().is_some_and(|a| near(a, y, tol))) {
        return Err(("wrong-value", format!("point: expected ({x}, {y}), got ({}, {})", point["x"], point["y"])));
    }
    line(&response["tangent"], &expect["tangent"], tol, "tangent")?;
    line(&response["normal"], &expect["normal"], tol, "normal")
}

/// A LaTeX value of a response read back as a number; `\approx` in front is dropped.
fn read_back(latex: &str, settings: &Settings) -> Result<RBig, (&'static str, String)> {
    let bare = latex.trim().trim_start_matches("\\approx").trim();
    let value = latex_complex(bare, settings, &Variables::new()).map_err(|e| ("result-unparsable", e))?;
    if !value.1.is_zero() {
        return Err(("result-unparsable", format!("{latex} is not real")));
    }
    Ok(value.0)
}

fn area(response: &Value, expect: &Value, settings: &Settings) -> Checked {
    for key in ["area", "signed"] {
        let want = expect[key].as_str().and_then(parse_rational).ok_or(("wrong-value", format!("no expected {key}")))?;
        let got = read_back(response[key].as_str().unwrap_or_default(), settings)?;
        if !close(&got, &want, &RBig::from_parts(IBig::ONE, UBig::from(10u8).pow(40))) {
            return Err(("wrong-value", format!("{key}: expected {}, got {}", show(&want), response[key])));
        }
    }
    Ok(())
}

fn table(response: &Value, expect: &Value, settings: &Settings) -> Checked {
    let got = response["rows"].as_array().cloned().unwrap_or_default();
    let want = expect["rows"].as_array().cloned().unwrap_or_default();
    if got.len() != want.len() {
        return Err(("rows", format!("expected {} rows, got {}", want.len(), got.len())));
    }
    let exact = exact_tolerance();
    for (i, (g, w)) in got.iter().zip(&want).enumerate() {
        let x = read_back(g["x"].as_str().unwrap_or_default(), settings)?;
        if Some(&x) != parse_rational(w[0].as_str().unwrap_or_default()).as_ref() {
            return Err(("rows", format!("row {}: expected x = {}, got {}", i + 1, w[0], g["x"])));
        }
        match (w[1].as_str().and_then(parse_rational), g["y"].as_str()) {
            (None, None) => {}
            (Some(want), Some(latex)) => {
                let y = read_back(latex, settings)?;
                if !close(&y, &want, &exact) {
                    return Err(("wrong-value", format!("row {}: expected y = {}, got {latex}", i + 1, show(&want))));
                }
            }
            (want, got) => return Err(("wrong-value", format!("row {}: expected y = {want:?}, got {got:?}", i + 1))),
        }
    }
    Ok(())
}
