//! Checks of `locales/`: every text in every language and no text English lacks, the same
//! placeholders, no text nobody uses.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every language's code and texts, English first.
fn locales() -> Vec<(String, Map<String, Value>)> {
    let mut locales: Vec<(String, Map<String, Value>)> = std::fs::read_dir(root().join("locales"))
        .unwrap()
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            let code = path.file_stem()?.to_str()?.to_string();
            (path.extension()? == "json").then_some(())?;
            let json: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            Some((code, json["texts"].as_object().unwrap().clone()))
        })
        .collect();
    locales.sort_by_key(|(code, _)| (code != "en", code.clone()));
    assert_eq!(locales[0].0, "en");
    locales
}

#[test]
fn every_text_is_in_every_language() {
    let locales = locales();
    let english = &locales[0].1;
    let mut wrong = Vec::new();
    for (code, texts) in &locales[1..] {
        wrong.extend(english.keys().filter(|key| texts.get(*key).and_then(Value::as_str).is_none_or(str::is_empty)).map(|key| format!("{code} lacks {key}")));
        wrong.extend(texts.keys().filter(|key| !english.contains_key(*key)).map(|key| format!("{code} has {key}, which English lacks")));
    }
    assert!(wrong.is_empty(), "{} texts are missing or extra:\n{}", wrong.len(), wrong.join("\n"));
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
    let locales = locales();
    let mut wrong = Vec::new();
    for (key, english) in &locales[0].1 {
        let wanted = placeholders(english.as_str().unwrap());
        for (code, texts) in &locales[1..] {
            if texts.get(key).and_then(Value::as_str).is_some_and(|text| placeholders(text) != wanted) {
                wrong.push(format!("{key}: {code}"));
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
    let texts = locales().swap_remove(0).1;
    let mut code = String::new();
    for dir in ["crates", "desktop/src", "desktop/src-tauri/src", "firmware"] {
        sources(&root().join(dir), &mut code);
    }
    // Every quoted word that could be a key, `"core.x"`, `'history'` or `` `copy` ``.
    let mut quoted = std::collections::HashSet::new();
    for quote in ['"', '\'', '`'] {
        // Apostrophes in comments throw the pairs off, so any piece made of key characters counts.
        quoted.extend(code.split(quote).filter(|part| part.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_')));
    }
    // The frontend builds some keys, `t(`mode.${mode}`)`: what precedes `${` stands for them all.
    let built: Vec<&str> = code.split("${").filter_map(|before| before.rsplit('`').next()).filter(|p| p.ends_with('.')).collect();
    let used = |short: &str| quoted.contains(short) || built.iter().any(|prefix| short.starts_with(prefix));
    let unused: Vec<&String> = texts
        .keys()
        .filter(|key| match key.strip_prefix("app.") {
            // The frontend's `t('history')` adds the prefix itself; `.hint` goes with its label.
            Some(short) => !used(short) && !short.strip_suffix(".hint").is_some_and(used),
            None => !quoted.contains(key.as_str()),
        })
        .collect();
    assert!(unused.is_empty(), "{} texts are not used anywhere:\n{unused:?}", unused.len());
}
