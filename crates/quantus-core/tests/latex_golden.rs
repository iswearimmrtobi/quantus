//! Runs the LaTeX cases extracted from the Kotlin tests: parse errors and parse/serialise round trips.

use std::fs;
use std::path::PathBuf;

use quantus_core::latex::parser::parse;
use quantus_core::QuantusError;
use serde_json::Value as Json;

fn cases(file: &str) -> Vec<Json> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(file);
    let root: Json = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    root["cases"].as_array().unwrap().clone()
}

#[test]
fn parse_errors_match_the_kotlin_implementation() {
    for case in cases("parse_errors.json") {
        let name = case["name"].as_str().unwrap();
        let error = parse(case["input"].as_str().unwrap()).expect_err(name);
        let QuantusError::Parse { reason, position, .. } = &error else {
            panic!("{name}: expected a parse error, got {error:?}");
        };
        if let Some(expected) = case["position"].as_u64() {
            assert_eq!(*position as u64, expected, "{name}: position");
        }
        if let Some(part) = case["reasonContains"].as_str() {
            assert!(reason.contains(part), "{name}: {reason:?} should contain {part:?}");
        }
        if let Some(part) = case["messageContains"].as_str() {
            assert!(error.to_string().contains(part), "{name}: {error}");
        }
        if case["hasPosition"].as_bool() == Some(true) {
            assert!(error.position().is_some(), "{name}: position");
        }
    }
}
