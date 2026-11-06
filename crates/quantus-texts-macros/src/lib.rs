//! The macros of `quantus-texts`. They read `locales/*.json` while compiling, so the programs
//! (the handheld among them) get plain string literals and `format!` calls: no JSON at run time.
//!
//! `build.rs` lists the files as `include_str!`s, so cargo rebuilds this crate, and everything
//! using it, when a text changes or a language is added.

use std::collections::HashSet;
use std::sync::OnceLock;

use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};
use serde_json::{Map, Value};

include!(concat!(env!("OUT_DIR"), "/locales.rs"));

struct Language {
    code: String,
    variant: String,
    /// The file's `language` object: name, decimal separator, …
    details: Map<String, Value>,
    texts: Map<String, Value>,
}

fn file() -> &'static Result<Vec<Language>, String> {
    static LANGUAGES: OnceLock<Result<Vec<Language>, String>> = OnceLock::new();
    LANGUAGES.get_or_init(|| {
        let languages = LOCALES
            .iter()
            .map(|(code, json)| {
                let root: Value = serde_json::from_str(json).map_err(|e| format!("locales/{code}.json: {e}"))?;
                let object = |name: &str| {
                    root.get(name).and_then(Value::as_object).cloned().ok_or(format!("locales/{code}.json: no `{name}` object"))
                };
                Ok(Language { code: code.to_string(), variant: variant(code), details: object("language")?, texts: object("texts")? })
            })
            .collect::<Result<Vec<_>, String>>()?;
        if languages.first().map(|l| l.code.as_str()) != Some("en") {
            return Err("locales/en.json is missing; English is the fallback".into());
        }
        Ok(languages)
    })
}

/// `en` → `En`, `pt-br` → `PtBr`.
fn variant(code: &str) -> String {
    code.split(['-', '_'])
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map(|c| c.to_ascii_uppercase().to_string() + chars.as_str()).unwrap_or_default()
        })
        .collect()
}

/// The text of `key` in every language, in the order of `LOCALES`; English where one is missing.
fn lookup(key: &str) -> Result<Vec<String>, String> {
    let languages = file().as_ref()?;
    let english = languages[0].texts.get(key).and_then(Value::as_str).ok_or_else(|| format!("no text `{key}` in locales/en.json"))?;
    Ok(languages.iter().map(|language| language.texts.get(key).and_then(Value::as_str).unwrap_or(english).to_string()).collect())
}

fn error(message: &str, span: Span) -> TokenStream {
    let mut literal = Literal::string(message);
    literal.set_span(span);
    let path: TokenStream = "::core::compile_error!".parse().unwrap();
    let mut out = path;
    out.extend([TokenTree::Group(Group::new(Delimiter::Parenthesis, TokenTree::Literal(literal).into()))]);
    out
}

/// The input split at its top-level commas.
fn arguments(input: TokenStream) -> Vec<Vec<TokenTree>> {
    let mut parts = vec![Vec::new()];
    for token in input {
        match &token {
            TokenTree::Punct(p) if p.as_char() == ',' => parts.push(Vec::new()),
            _ => parts.last_mut().unwrap().push(token),
        }
    }
    if parts.last().is_some_and(Vec::is_empty) {
        parts.pop();
    }
    parts
}

/// The key of a string literal argument.
fn key(tokens: &[TokenTree]) -> Result<(String, Span), (String, Span)> {
    match tokens {
        [TokenTree::Literal(literal)] => {
            let text = literal.to_string();
            match text.strip_prefix('"').and_then(|t| t.strip_suffix('"')) {
                Some(key) if !key.contains('\\') => Ok((key.to_string(), literal.span())),
                _ => Err(("the key must be a plain string literal".into(), literal.span())),
            }
        }
        _ => Err(("expected the key of a text, a string literal".into(), tokens.first().map_or(Span::call_site(), TokenTree::span))),
    }
}

/// The names a format string uses, `{name}` or `{name:…}`, in order of first use.
fn named_placeholders(format: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut chars = format.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
            }
            '{' => {
                let name: String = std::iter::from_fn(|| chars.next_if(|c| *c != '}' && *c != ':')).collect();
                let name = name.trim();
                if name.starts_with(|c: char| c.is_alphabetic() || c == '_') && !names.iter().any(|n| n == name) {
                    names.push(name.to_string());
                }
            }
            _ => {}
        }
    }
    names
}

/// The name of a `name = value` argument.
fn argument_name(tokens: &[TokenTree]) -> Option<String> {
    match tokens {
        [TokenTree::Ident(name), TokenTree::Punct(eq), ..] if eq.as_char() == '=' && eq.spacing() == Spacing::Alone => Some(name.to_string()),
        _ => None,
    }
}

/// `tr!(language, "key", arguments…)`: the text `key` in `language`, through `format!` with the
/// arguments. Every language's text is checked against the arguments while compiling.
#[proc_macro]
pub fn tr(input: TokenStream) -> TokenStream {
    let parts = arguments(input);
    if parts.len() < 2 {
        return error("expected `tr!(language, \"key\", arguments…)`", Span::call_site());
    }
    let (key, span) = match key(&parts[1]) {
        Ok(key) => key,
        Err((message, span)) => return error(&message, span),
    };
    let values = match lookup(&key) {
        Ok(values) => values,
        Err(message) => return error(&message, span),
    };
    let languages = file().as_ref().unwrap();
    let given: HashSet<String> = parts[2..].iter().filter_map(|p| argument_name(p)).collect();

    let mut arms = TokenStream::new();
    for (language, value) in languages.iter().zip(&values) {
        let pattern: TokenStream = format!("::quantus_texts::Language::{} =>", language.variant).parse().unwrap();
        arms.extend(pattern);
        let mut literal = Literal::string(value);
        literal.set_span(span);
        let mut args = TokenStream::from(TokenTree::Literal(literal));
        for part in &parts[2..] {
            args.extend([TokenTree::Punct(Punct::new(',', Spacing::Alone))]);
            args.extend(part.iter().cloned());
        }
        // Implicit captures do not work in a format string a macro made, so they are spelled out.
        for name in named_placeholders(value).into_iter().filter(|n| !given.contains(n)) {
            args.extend([
                TokenTree::Punct(Punct::new(',', Spacing::Alone)),
                TokenTree::Ident(Ident::new(&name, span)),
                TokenTree::Punct(Punct::new('=', Spacing::Alone)),
                TokenTree::Ident(Ident::new(&name, span)),
            ]);
        }
        arms.extend("::std::format!".parse::<TokenStream>().unwrap());
        arms.extend([TokenTree::Group(Group::new(Delimiter::Parenthesis, args)), TokenTree::Punct(Punct::new(',', Spacing::Alone))]);
    }
    let mut out: TokenStream = "match".parse().unwrap();
    out.extend([TokenTree::Group(Group::new(Delimiter::Parenthesis, parts[0].iter().cloned().collect()))]);
    out.extend([TokenTree::Group(Group::new(Delimiter::Brace, arms))]);
    out
}

/// `texts!("key")`: the text `key` in every language, a `[&'static str; N]` in the order of
/// `Language`, usable in a `const`.
#[proc_macro]
pub fn texts(input: TokenStream) -> TokenStream {
    let parts = arguments(input);
    let (key, span) = match parts.as_slice() {
        [only] => match key(only) {
            Ok(key) => key,
            Err((message, span)) => return error(&message, span),
        },
        _ => return error("expected `texts!(\"key\")`", Span::call_site()),
    };
    let values = match lookup(&key) {
        Ok(values) => values,
        Err(message) => return error(&message, span),
    };
    let mut items = TokenStream::new();
    for value in values {
        let mut literal = Literal::string(&value);
        literal.set_span(span);
        items.extend([TokenTree::Literal(literal), TokenTree::Punct(Punct::new(',', Spacing::Alone))]);
    }
    TokenTree::Group(Group::new(Delimiter::Bracket, items)).into()
}

/// `language_enum!()`: `pub enum Language` with a variant per file of `locales/`, and its table.
#[proc_macro]
pub fn language_enum(_input: TokenStream) -> TokenStream {
    let languages = match file() {
        Ok(languages) => languages,
        Err(message) => return error(message, Span::call_site()),
    };
    let field = |language: &Language, name: &str| format!("{:?}", language.details.get(name).and_then(Value::as_str).unwrap_or(""));

    let mut code = String::new();
    code.push_str("/// A language of the texts, one per file of `locales/`; English is the fallback.\n");
    code.push_str("#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]\n");
    code.push_str("#[cfg_attr(feature = \"serde\", derive(::serde::Serialize, ::serde::Deserialize))]\n");
    code.push_str("#[repr(usize)]\npub enum Language {\n");
    for (i, language) in languages.iter().enumerate() {
        if i == 0 {
            code.push_str("#[default]\n");
        }
        code.push_str(&format!("#[cfg_attr(feature = \"serde\", serde(rename = {:?}))]\n{},\n", language.code, language.variant));
    }
    code.push_str("}\n");
    let list = |f: &dyn Fn(&Language) -> String| languages.iter().map(f).collect::<Vec<_>>().join(", ");
    code.push_str(&format!("impl Language {{\n/// Every language, in the order of the file.\npub const ALL: [Language; {}] = [{}];\n", languages.len(), list(&|l| format!("Language::{}", l.variant))));
    code.push_str(&format!("const CODES: [&'static str; {}] = [{}];\n", languages.len(), list(&|l| format!("{:?}", l.code))));
    for (name, doc) in [("name", "Its own name for itself, shown the same in every language."), ("decimal", "The decimal separator."), ("intervals", "How it writes intervals: `round` `(0, 1)` or `reversed` `]0; 1[`."), ("quadratic", "The formula its schools teach for quadratic equations: `abc` or `pq`.")] {
        code.push_str(&format!("const {}: [&'static str; {}] = [{}];\n", name.to_uppercase(), languages.len(), list(&|l| field(l, name))));
        code.push_str(&format!("#[doc = {doc:?}]\npub const fn {name}(self) -> &'static str {{ Self::{}[self as usize] }}\n", name.to_uppercase()));
    }
    code.push_str("}\n");
    code.parse().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_and_variants() {
        assert_eq!(named_placeholders("{a} {{b}} {} {c:>5} {0} {a}"), ["a", "c"]);
        assert_eq!(variant("pt-br"), "PtBr");
    }
}
