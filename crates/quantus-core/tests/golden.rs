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
        if let Some(v) = text("warningContains") {
            let warning = calculation.warning.as_deref().unwrap_or_else(|| panic!("{name}: expected a warning"));
            assert!(warning.contains(v), "{name}: {warning:?} should contain {v:?}");
        }
        if let Some(v) = text("resultStartsWith") {
            assert!(calculation.result.starts_with(v), "{name}: {}", calculation.result);
        }
        if let Some(digits) = expect["valueSignificantDigits"].as_u64() {
            assert_eq!(significant_digits(&calculation.value) as u64, digits, "{name}: digits");
        }
    }
}

#[test]
fn session_scenarios_match_the_kotlin_implementation() {
    use quantus_core::session::{SessionStore, Variable};

    fn check_variable(name: &str, variable: &Variable, expect: &Json) {
        let text = |key: &str| expect[key].as_str();
        if let Some(v) = text("key") {
            assert_eq!(variable.key, v, "{name}: variable.key");
        }
        if let Some(v) = text("symbol") {
            assert_eq!(variable.symbol, v, "{name}: variable.symbol");
        }
        if let Some(v) = text("latex") {
            assert_eq!(variable.latex, v, "{name}: variable.latex");
        }
        if let Some(v) = text("value") {
            assert_eq!(variable.value.to_rational(), Decimal::parse(v).unwrap().to_rational(), "{name}: variable.value");
        }
    }

    for scenario in golden("sessions.json")["scenarios"].as_array().unwrap() {
        let name = scenario["name"].as_str().unwrap();
        let mut store = SessionStore::new();
        for op in scenario["ops"].as_array().unwrap() {
            let session = store.session(op["session"].as_str());
            let latex = op["latex"].as_str().unwrap_or_default();
            match op["op"].as_str().unwrap() {
                "evaluate" | "define" => {
                    let defining = op["op"] == "define";
                    let outcome = if defining {
                        session.define_from(latex, None).map(|variable| (None, Some(variable)))
                    } else {
                        session.evaluate(latex, None).map(|(c, v)| (Some(c), v))
                    };
                    if let Some(part) = op["errorContains"].as_str() {
                        let message = outcome.err().unwrap_or_else(|| panic!("{name}: expected an error")).to_string();
                        assert!(message.contains(part), "{name}: {message:?} should contain {part:?}");
                        continue;
                    }
                    let (calculation, variable) = outcome.unwrap_or_else(|e| panic!("{name}: {e}"));
                    let expect = &op["expect"];
                    if let Some(c) = &calculation {
                        if let Some(v) = expect["latex"].as_str() {
                            assert_eq!(c.latex, v, "{name}: latex");
                        }
                        if let Some(v) = expect["result"].as_str() {
                            assert_eq!(c.result, v, "{name}: result");
                        }
                        if let Some(v) = expect["alignBlock"].as_str() {
                            assert_eq!(c.align_block(), v, "{name}: alignBlock");
                        }
                        if let Some(kinds) = expect["stepKinds"].as_array() {
                            let got: Vec<_> = c.steps.iter().map(|s| s.kind.name()).collect();
                            let want: Vec<_> = kinds.iter().map(|k| k.as_str().unwrap()).collect();
                            assert_eq!(got, want, "{name}: stepKinds");
                        }
                        if let Some(steps) = expect["steps"].as_array() {
                            let got: Vec<_> = c.steps.iter().map(|s| (s.kind.name(), s.latex.as_str())).collect();
                            let want: Vec<_> = steps
                                .iter()
                                .map(|s| (s["kind"].as_str().unwrap(), s["latex"].as_str().unwrap()))
                                .collect();
                            assert_eq!(got, want, "{name}: steps");
                        }
                    }
                    if expect["noVariable"] == true {
                        assert!(variable.is_none(), "{name}: noVariable");
                    }
                    if expect["variable"].is_object() {
                        let variable = variable.unwrap_or_else(|| panic!("{name}: expected a variable"));
                        check_variable(name, &variable, &expect["variable"]);
                    }
                }
                "list_variables" => {
                    let got: Vec<_> = session.variables().iter().map(|v| v.key.as_str()).collect();
                    let want: Vec<_> = op["keys"].as_array().unwrap().iter().map(|k| k.as_str().unwrap()).collect();
                    assert_eq!(got, want, "{name}: keys");
                }
                "delete" => {
                    session.remove(op["key"].as_str().unwrap());
                }
                other => panic!("{name}: unknown op {other}"),
            }
        }
    }
}
