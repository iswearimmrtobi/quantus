//! Systems of linear equations: the golden cases of `systems.json`, the result and every step
//! with both elimination methods and at each detail level.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use quantus_core::eval::LatexCalculator;
use quantus_core::settings::Settings;
use serde_json::{json, Value as Json};

mod common;

#[test]
fn system_cases_match_their_steps() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/systems.json");
    let golden: Json = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    common::check_all(golden["cases"].as_array().unwrap(), |case| case["name"].as_str().unwrap().to_string(), check_case);
}

fn check_case(case: &Json) {
    let name = case["name"].as_str().unwrap();
    let settings: Settings = serde_json::from_value(json!({
        "mode": "algebra",
        "elimination": case["elimination"],
        "stepDetail": case["stepDetail"],
    }))
    .unwrap();
    let calculation = LatexCalculator::default()
        .with_settings(settings)
        .evaluate(case["latex"].as_str().unwrap(), &HashMap::new())
        .unwrap_or_else(|e| panic!("{name}: {e}"));
    assert_eq!(calculation.result, case["expect"]["result"].as_str().unwrap(), "{name}: result");
    let steps: Vec<Json> =
        calculation.steps.iter().map(|s| json!([s.latex, s.explanation.clone().unwrap_or_default()])).collect();
    let expected = case["expect"]["steps"].as_array().unwrap();
    for (i, (step, want)) in steps.iter().zip(expected).enumerate() {
        assert_eq!(step, want, "{name}: step {i}");
    }
    assert_eq!(steps.len(), expected.len(), "{name}: number of steps");
}
