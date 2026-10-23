//! The circuit calculator's JSON interface, from `golden/electro.json`: each case is a
//! `CircuitRequest`, and either the fields of the `CircuitResponse` that must match (the rest
//! is not compared) or the error.

use std::fs;
use std::path::PathBuf;

use quantus_core::api::{solve_circuit, CircuitRequest};
use quantus_core::settings::{Language, Mode, Settings};
use serde_json::{json, Value};

fn golden() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/electro.json");
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn settings() -> Settings {
    Settings { mode: Mode::Electronics, ..Settings::default() }
}

/// Whether everything in `expected` is in `actual`. Objects match by key, arrays element by
/// element, except that an array of objects with an `id` matches by that ID.
fn contains(actual: &Value, expected: &Value, path: &str) -> Result<(), String> {
    match (actual, expected) {
        (Value::Object(a), Value::Object(e)) => {
            for (key, want) in e {
                let have = a.get(key).ok_or_else(|| format!("{path}.{key} is missing"))?;
                contains(have, want, &format!("{path}.{key}"))?;
            }
            Ok(())
        }
        (Value::Array(a), Value::Array(e)) => {
            for (n, want) in e.iter().enumerate() {
                let have = match want.get("id") {
                    Some(id) => a.iter().find(|x| x.get("id") == Some(id)),
                    None => a.get(n),
                };
                let have = have.ok_or_else(|| format!("{path}[{n}] is missing"))?;
                contains(have, want, &format!("{path}[{n}]"))?;
            }
            Ok(())
        }
        _ if actual == expected => Ok(()),
        _ => Err(format!("{path}: {actual} should be {expected}")),
    }
}

#[test]
fn circuit_cases_match() {
    for case in golden()["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let request: CircuitRequest = serde_json::from_value(case["request"].clone()).unwrap();
        let outcome = solve_circuit(&settings(), &request);

        if case.get("error").is_some() || case.get("errorContains").is_some() {
            let message = outcome.expect_err(name).error;
            if let Some(exact) = case["error"].as_str() {
                assert_eq!(message, exact, "{name}");
            }
            if let Some(part) = case["errorContains"].as_str() {
                assert!(message.contains(part), "{name}: {message:?} should contain {part:?}");
            }
            continue;
        }
        let response = serde_json::to_value(outcome.unwrap_or_else(|e| panic!("{name}: {e}"))).unwrap();
        if let Err(problem) = contains(&response, &case["expect"], "") {
            panic!("{name}: {problem}\n{response:#}");
        }
    }
}

#[test]
fn the_reverse_result_meets_the_target_when_fed_back() {
    let mut case = golden()["cases"][3]["request"].clone();
    let found = solve_circuit(&settings(), &serde_json::from_value(case.clone()).unwrap()).unwrap();
    let area = found.reverse.unwrap().value.value.unwrap();
    // The area is in m²; give it back as a plain given parameter.
    case["reverse"] = Value::Null;
    case["elements"][1]["area"] = json!(format!("{area}m^2"));
    let forward = solve_circuit(&settings(), &serde_json::from_value(case).unwrap()).unwrap();
    let r1 = forward.elements.iter().find(|e| e.id == "R1").unwrap();
    let volts: f64 = r1.voltage.value.as_ref().unwrap().parse().unwrap();
    assert!((volts - 200.0).abs() < 1e-3, "{volts}");
}

#[test]
fn a_bad_precision_is_an_error() {
    let request = CircuitRequest { nodes: vec![0], precision: Some(0), ..Default::default() };
    assert!(solve_circuit(&settings(), &request).is_err());
}

#[test]
fn the_texts_follow_the_language_setting() {
    let request = CircuitRequest { nodes: vec![1], ..Default::default() };
    let message = |language| solve_circuit(&Settings { language, ..settings() }, &request).unwrap_err().to_string();
    assert_eq!(message(Language::En), "The circuit has no ground node 0.");
    assert_eq!(message(Language::De), "Die Schaltung hat keinen Masseknoten 0.");
}
