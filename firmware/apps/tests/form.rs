//! A form's description (apps.md §4.7): read, and what is wrong named by where.

use quantus_apps::form::read;
use quantus_apps::Value;

fn s(text: &str) -> Value {
    Value::Str(text.into())
}

fn map(entries: &[(&str, Value)]) -> Value {
    Value::Map(entries.iter().map(|(key, value)| (key.to_string(), value.clone())).collect())
}

fn divider() -> Value {
    map(&[
        ("title", s("Spannungsteiler")),
        (
            "fields",
            Value::Array(vec![
                map(&[("id", s("U")), ("label", s("Eingangsspannung")), ("unit", s("V"))]),
                map(&[("id", s("R1")), ("label", s("R₁")), ("unit", s("Ω"))]),
                map(&[("id", s("R2")), ("label", s("R₂")), ("unit", s("Ω")), ("value", s("10 kΩ")), ("hint", s("Last"))]),
                map(&[("id", s("m")), ("choices", Value::Array(vec![s("Stahl"), s("Alu")]))]),
            ]),
        ),
        (
            "results",
            Value::Array(vec![
                map(&[("label", s("U₂")), ("formula", s("U*R2/(R1+R2)")), ("unit", s("V"))]),
                map(&[("label", s("I")), ("formula", s("U/(R1+R2)")), ("unit", s("mA"))]),
            ]),
        ),
    ])
}

#[test]
fn a_good_description_is_read() {
    let form = read(&divider()).expect("a form");
    assert_eq!(form.title, "Spannungsteiler");
    assert_eq!(form.fields.len(), 4);
    assert_eq!(form.fields[2].value.as_deref(), Some("10 kΩ"));
    assert_eq!(form.fields[2].hint.as_deref(), Some("Last"));
    assert_eq!(form.fields[3].label, "m");
    assert_eq!(form.fields[3].choices, ["Stahl", "Alu"]);
    assert_eq!(form.results[1].unit.as_deref(), Some("mA"));
    assert!(!form.submit);
}

/// The description with `edit` done to it, read: the error.
fn error(edit: impl Fn(&mut Vec<(String, Value)>)) -> String {
    let Value::Map(mut entries) = divider() else { unreachable!() };
    edit(&mut entries);
    read(&Value::Map(entries)).expect_err("an error")
}

fn field(entries: &mut [(String, Value)], index: usize) -> &mut Vec<(String, Value)> {
    let Some((_, Value::Array(fields))) = entries.iter_mut().find(|(key, _)| key == "fields") else { unreachable!() };
    let Value::Map(field) = &mut fields[index] else { unreachable!() };
    field
}

#[test]
fn what_is_wrong_is_named() {
    assert_eq!(read(&s("x")).unwrap_err(), "form: the description is not a map");
    assert!(error(|e| e.retain(|(key, _)| key != "fields")).starts_with("form.fields"));
    assert_eq!(error(|e| field(e, 1).retain(|(key, _)| key != "id")), "fields[1].id is missing");
    assert!(error(|e| field(e, 2)[0].1 = s("R 2")).starts_with("fields[2].id \"R 2\""));
    assert!(error(|e| field(e, 2)[0].1 = s("U")).contains("twice"));
    assert_eq!(error(|e| field(e, 0)[1].1 = Value::Array(vec![])), "fields[0].label: text was expected");
    assert!(error(|e| field(e, 3)[1].1 = s("Stahl")).starts_with("fields[3].choices"));
    assert_eq!(error(|e| e.retain(|(key, _)| key != "results")), "form: results or submit is needed");
    let message = error(|e| {
        let Some((_, Value::Array(results))) = e.iter_mut().find(|(key, _)| key == "results") else { unreachable!() };
        results[1] = map(&[("label", s("I"))]);
    });
    assert_eq!(message, "results[1].formula is missing");
    // With submit, no results are needed.
    let Value::Map(mut entries) = divider() else { unreachable!() };
    entries.retain(|(key, _)| key != "results");
    entries.push(("submit".into(), s("Fn(submit)")));
    assert!(read(&Value::Map(entries)).expect("a form").submit);
}
