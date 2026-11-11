//! A form's description from a script's map (apps.md §4.7), with what is wrong in it named by
//! where it is.

use quantus_ui::form::{Field, Form, Outcome};

use crate::script::Value;

/// The form a script's `description` describes, or what is wrong with it.
pub fn read(description: &Value) -> Result<Form, String> {
    let Value::Map(_) = description else { return Err("form: the description is not a map".into()) };
    let text = |value: &Value, at: &str, name: &str| -> Result<Option<String>, String> {
        match value.get(name) {
            None | Some(Value::Unit) => Ok(None),
            Some(Value::Str(s)) => Ok(Some(s.clone())),
            Some(Value::Int(i)) => Ok(Some(i.to_string())),
            Some(Value::Float(f)) => Ok(Some(f.to_string())),
            Some(_) => Err(format!("{at}{name}: text was expected")),
        }
    };
    let title = text(description, "form.", "title")?.unwrap_or_default();
    let Some(Value::Array(fields)) = description.get("fields") else { return Err("form.fields: a list of fields is needed".into()) };
    if fields.is_empty() {
        return Err("form.fields: the list is empty".into());
    }
    let mut read_fields = Vec::new();
    for (index, field) in fields.iter().enumerate() {
        let at = format!("fields[{index}].");
        if !matches!(field, Value::Map(_)) {
            return Err(format!("fields[{index}]: a map was expected"));
        }
        let id = text(field, &at, "id")?.ok_or_else(|| format!("{at}id is missing"))?;
        let plain = id.starts_with(|c: char| c.is_ascii_alphabetic()) && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
        if !plain {
            return Err(format!("{at}id {id:?}: letters, digits and _, a letter first"));
        }
        if read_fields.iter().any(|other: &Field| other.id == id) {
            return Err(format!("{at}id {id:?} is there twice"));
        }
        let choices = match field.get("choices") {
            None | Some(Value::Unit) => Vec::new(),
            Some(Value::Array(items)) if !items.is_empty() => items
                .iter()
                .map(|item| match item {
                    Value::Str(s) => Ok(s.clone()),
                    _ => Err(format!("{at}choices: texts were expected")),
                })
                .collect::<Result<_, _>>()?,
            Some(_) => return Err(format!("{at}choices: a list of texts was expected")),
        };
        read_fields.push(Field {
            label: text(field, &at, "label")?.unwrap_or_else(|| id.clone()),
            id,
            unit: text(field, &at, "unit")?,
            value: text(field, &at, "value")?,
            hint: text(field, &at, "hint")?,
            choices,
        });
    }
    let submit = description.get("submit").is_some_and(|submit| !matches!(submit, Value::Unit));
    let mut results = Vec::new();
    match description.get("results") {
        Some(Value::Array(items)) => {
            for (index, item) in items.iter().enumerate() {
                let at = format!("results[{index}].");
                if !matches!(item, Value::Map(_)) {
                    return Err(format!("results[{index}]: a map was expected"));
                }
                let formula = text(item, &at, "formula")?.ok_or_else(|| format!("{at}formula is missing"))?;
                results.push(Outcome { label: text(item, &at, "label")?.unwrap_or_default(), formula, unit: text(item, &at, "unit")? });
            }
        }
        None | Some(Value::Unit) if submit => {}
        None | Some(Value::Unit) => return Err("form: results or submit is needed".into()),
        Some(_) => return Err("form.results: a list of results was expected".into()),
    }
    Ok(Form { title, fields: read_fields, results, submit })
}
