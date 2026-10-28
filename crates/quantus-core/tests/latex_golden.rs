//! Runs the LaTeX cases extracted from the Kotlin tests: parse errors and parse/serialise round trips.

use std::fs;
use std::path::PathBuf;

use quantus_core::latex::parser::parse;
use quantus_core::latex::serializer;
use quantus_core::QuantusError;
use serde_json::Value as Json;

mod common;

fn cases(file: &str) -> Vec<Json> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(file);
    let root: Json = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    root["cases"].as_array().unwrap().clone()
}

fn case_name(case: &Json) -> String {
    case["name"].as_str().unwrap().to_string()
}

#[test]
fn parse_errors_match_the_kotlin_implementation() {
    common::check_all(&cases("parse_errors.json"), case_name, check_parse_error);
}

fn check_parse_error(case: &Json) {
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

#[test]
fn round_trips_match_the_kotlin_implementation() {
    common::check_all(&cases("roundtrip.json"), case_name, |case| {
        let name = case["name"].as_str().unwrap();
        let formula = parse(case["input"].as_str().unwrap()).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(serializer::formula(&formula), case["expected"].as_str().unwrap(), "{name}");
    });
}

#[test]
fn labels_are_split_into_name_and_index() {
    let root: Json = {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/roundtrip.json");
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
    };
    let input = |case: &Json| case["input"].as_str().unwrap().to_string();
    common::check_all(root["labels"].as_array().unwrap(), input, |case| {
        let input = case["input"].as_str().unwrap();
        let label = parse(input).unwrap().label;
        match (&case["label"], label) {
            (Json::Null, None) => {}
            (expected, Some(label)) => {
                assert_eq!(label.name, expected["name"].as_str().unwrap(), "{input}");
                assert_eq!(label.index.as_deref(), expected["index"].as_str(), "{input}");
            }
            (expected, None) => panic!("{input}: expected label {expected}"),
        }
    });
}
