//! Runs the PhysicalQuantity cases dumped from the Kotlin implementation (`quantity_cases.json`).

use std::fs;
use std::path::PathBuf;

use quantus_core::num::{Decimal, Value};
use quantus_core::units::{find, latex, registry, Unit};
use quantus_core::{PhysicalQuantity, QuantusError, Result};
use serde_json::Value as Json;

fn cases() -> Vec<Json> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/quantity_cases.json");
    let root: Json = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    root["cases"].as_array().unwrap().clone()
}

/// Operand values are a decimal string, `a/b`, `Pi`, `E` or `Sqrt2`.
fn value(text: &str) -> Value {
    match text {
        "Pi" => Value::Pi,
        "E" => Value::E,
        "Sqrt2" => Value::int(2).pow(&Value::ratio(1, 2)).unwrap(),
        _ => match text.split_once('/') {
            Some((n, d)) => Value::int(n.parse().unwrap()).div(&Value::int(d.parse().unwrap())).unwrap(),
            None => Value::from_decimal(&Decimal::parse(text).unwrap_or_else(|| panic!("number {text:?}"))),
        },
    }
}

fn unit(factors: &Json) -> Unit {
    factors.as_array().unwrap().iter().fold(Unit::ONE, |acc, factor| {
        let atom = find(factor[0].as_str().unwrap()).unwrap();
        let (power, root) = (factor[1].as_i64().unwrap() as i32, factor[2].as_i64().unwrap() as i32);
        acc.multiply(&atom.pow(power).root(root).unwrap())
    })
}

fn quantity(operand: &Json) -> PhysicalQuantity {
    PhysicalQuantity::new(value(operand["value"].as_str().unwrap()), unit(&operand["unit"]))
}

fn run(case: &Json) -> Result<PhysicalQuantity> {
    let a = quantity(&case["a"]);
    let b = || quantity(&case["b"]);
    let n = || case["n"].as_i64().unwrap() as i32;
    let result = match case["op"].as_str().unwrap() {
        "add" => a.add(&b())?,
        "sub" => a.sub(&b())?,
        "mul" => a.mul(&b()),
        "div" => a.div(&b())?,
        "neg" => a.neg(),
        "pow_int" => a.pow_int(n())?,
        "pow" => a.pow(&b())?,
        "root" => a.root(n() as u32)?,
        "to" => a.to(&unit(&case["target"]))?,
        // The dump left `simplify` out of its operator switch, so these cases only record the
        // operand; the real behaviour is covered by unit tests in `quantity.rs` and `registry.rs`.
        "simplify" => a,
        other => panic!("unknown op {other}"),
    };
    if case["simplify"].as_bool().unwrap() {
        result.simplify()
    } else {
        Ok(result)
    }
}

fn exact(value: &Value) -> Option<String> {
    value.as_rational().map(value_text)
}

fn value_text(r: &dashu::rational::RBig) -> String {
    Value::Rat(r.clone()).to_string()
}

#[test]
fn quantity_cases_match_the_kotlin_implementation() {
    for case in cases() {
        let name = case["name"].as_str().unwrap();
        let expect = &case["expect"];
        let outcome = run(&case);

        if let Some(message) = expect["error"].as_str() {
            let error = outcome.expect_err(name);
            assert_eq!(error.to_string(), message, "{name}");
            let kind_ok = match expect["errorKind"].as_str().unwrap() {
                "DimensionMismatchException" => matches!(error, QuantusError::DimensionMismatch(_)),
                _ => matches!(error, QuantusError::Other(_)),
            };
            assert!(kind_ok, "{name}: error kind of {error:?}");
            continue;
        }

        // Kotlin divides by zero into Symja's ComplexInfinity, which is merely not real. Here
        // the division is refused up front (see the plan's findings for M3).
        if expect["exact"].is_null() && expect["decimal"].is_null() && case["op"] == "div" {
            let error = outcome.expect_err(name);
            assert_eq!(error.to_string(), "Division by zero.", "{name}");
            continue;
        }

        let result = outcome.unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(registry::format(&result.unit), expect["unit"].as_str().unwrap(), "{name}: unit");
        assert_eq!(latex::format(&result.unit), expect["unitLatex"].as_str().unwrap(), "{name}: latex");
        assert_eq!(result.quantity_name(), expect["quantityName"].as_str(), "{name}: quantity name");
        assert_eq!(result.dimension().to_string(), expect["dimension"].as_str().unwrap(), "{name}: dimension");
        assert_eq!(result.is_dimensionless(), expect["isDimensionless"].as_bool().unwrap(), "{name}: dimensionless");
        assert_eq!(exact(&result.value).as_deref(), expect["exact"].as_str(), "{name}: exact");
        match expect["decimal"].as_str() {
            Some(digits) => {
                let decimal = result.decimal(50).unwrap_or_else(|e| panic!("{name}: {e}"));
                assert_eq!(decimal.normalized().to_plain_string(), digits, "{name}: decimal");
            }
            None => assert!(matches!(result.decimal(50), Err(QuantusError::NotReal(_))), "{name}: decimal"),
        }
    }
}
