//! Runs the stateless evaluation cases (`evaluate.json`) extracted from the Kotlin tests.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use quantus_core::eval::LatexCalculator;
use quantus_core::num::Decimal;
use serde_json::Value as Json;

fn golden(file: &str) -> Json {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(file);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn significant_digits(value: &Decimal) -> usize {
    value.normalized().precision()
}

#[test]
fn evaluate_cases_match_the_kotlin_implementation() {
    for case in golden("evaluate.json")["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let latex = case["latex"].as_str().unwrap();
        let precision = case["precision"].as_u64().unwrap_or(50) as usize;
        let outcome = LatexCalculator::new(precision).evaluate(latex, &HashMap::new());

        if case.get("error").is_some() || case.get("errorContains").is_some() {
            let message = outcome.expect_err(name).to_string();
            if let Some(exact) = case["error"].as_str() {
                assert_eq!(message, exact, "{name}");
            }
            if let Some(part) = case["errorContains"].as_str() {
                assert!(message.contains(part), "{name}: {message:?} should contain {part:?}");
            }
            continue;
        }

        let calculation = outcome.unwrap_or_else(|e| panic!("{name}: {e}"));
        let expect = &case["expect"];
        let text = |key: &str| expect[key].as_str();
        if let Some(v) = text("latex") {
            assert_eq!(calculation.latex, v, "{name}: latex");
        }
        if let Some(v) = text("result") {
            assert_eq!(calculation.result, v, "{name}: result");
        }
        if let Some(v) = text("input") {
            assert_eq!(calculation.input, v, "{name}: input");
        }
        if let Some(v) = text("unit") {
            assert_eq!(calculation.unit, v, "{name}: unit");
        }
        if let Some(v) = text("quantityName") {
            assert_eq!(calculation.quantity_name.as_deref(), Some(v), "{name}: quantityName");
        }
        if let Some(v) = text("value") {
            let want = Decimal::parse(v).unwrap().to_rational();
            assert_eq!(calculation.value.to_rational(), want, "{name}: value");
        }
        if let Some(v) = text("resultStartsWith") {
            assert!(calculation.result.starts_with(v), "{name}: {}", calculation.result);
        }
        if let Some(digits) = expect["valueSignificantDigits"].as_u64() {
            assert_eq!(significant_digits(&calculation.value) as u64, digits, "{name}: digits");
        }
    }
}
