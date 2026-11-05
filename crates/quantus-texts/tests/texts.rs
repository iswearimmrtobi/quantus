//! Checks of `texts/texts.json`: every text in every language, the same placeholders, no text
//! nobody uses.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn file() -> (Vec<String>, Map<String, Value>) {
    let text = std::fs::read_to_string(root().join("texts/texts.json")).unwrap();
    let json: Value = serde_json::from_str(&text).unwrap();
    let languages = json["languages"].as_object().unwrap().keys().cloned().collect();
    (languages, json["texts"].as_object().unwrap().clone())
}

#[test]
fn every_text_is_in_every_language() {
    let (languages, texts) = file();
    let gaps: Vec<String> = texts
        .iter()
        .flat_map(|(key, entry)| {
            languages
                .iter()
                .filter(|l| entry.get(l.as_str()).and_then(Value::as_str).is_none_or(str::is_empty))
                .map(move |l| format!("{key}: {l}"))
        })
        .collect();
    assert!(gaps.is_empty(), "{} texts are missing a language:\n{}", gaps.len(), gaps.join("\n"));
}

/// The `{name}` and `{}` placeholders of a text; `{{` is a brace.
fn placeholders(text: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut positional = 0;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            if chars.peek() == Some(&'{') {
                chars.next();
                continue;
            }
            let inner: String = std::iter::from_fn(|| chars.next_if(|c| *c != '}')).collect();
            let name = inner.split(':').next().unwrap().trim().to_string();
            if name.is_empty() {
                positional += 1;
                found.insert(format!("#{positional}"));
            } else {
                found.insert(name);
            }
        }
    }
    found
}

#[test]
fn every_language_has_the_same_placeholders() {
    let (_, texts) = file();
    let mut wrong = Vec::new();
    for (key, entry) in &texts {
        let english = placeholders(entry["en"].as_str().unwrap());
        for (language, text) in entry.as_object().unwrap() {
            if let Some(text) = text.as_str().filter(|t| !t.is_empty()) {
                if placeholders(text) != english {
                    wrong.push(format!("{key}: {language}"));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "placeholders differ from English:\n{}", wrong.join("\n"));
}

fn sources(dir: &Path, out: &mut String) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().unwrap().to_string_lossy();
        if path.is_dir() {
            if !matches!(name.as_ref(), "target" | "node_modules" | "dist" | "gen") {
                sources(&path, out);
            }
        } else if name.ends_with(".rs") || name.ends_with(".ts") || name.ends_with(".tsx") {
            out.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
        }
    }
}

#[test]
fn every_text_is_used() {
    let (_, texts) = file();
    let mut code = String::new();
    for dir in ["crates", "desktop/src", "desktop/src-tauri/src", "firmware"] {
        sources(&root().join(dir), &mut code);
    }
    let unused: Vec<&String> = texts
        .keys()
        .filter(|key| match key.strip_prefix("app.") {
            // The frontend's `t('history')` adds the prefix itself.
            Some(short) => ![format!("'{short}'"), format!("\"{short}\""), format!("`{short}`")].iter().any(|k| code.contains(k)),
            None => !code.contains(&format!("\"{key}\"")),
        })
        .collect();
    assert!(unused.is_empty(), "{} texts are not used anywhere:\n{unused:?}", unused.len());
}
