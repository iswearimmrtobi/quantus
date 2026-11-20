//! Functions that use other functions: with `f(x) = x^2` defined, `g(x) = f(x - 1) + 2` and
//! `h(x) = f'(x)` are put in before anything else reads them, so `g` becomes
//! `\left((x - 1)^{2}\right) + 2`. The other functions come with each request as definitions
//! (`g(x) = …`, one per entry); a reference is a defined name followed by primes for its
//! derivatives and an argument in brackets. Definitions may use each other, but not in a circle.

use super::derive::derivative;
use super::expr::Expr;
use super::input::{definition, parse_function};
use super::simplify::simplify;
use crate::latex::parser;
use crate::settings::{Mode, Settings};
use crate::text::msg;
use crate::{QuantusError, Result};

struct Definition<'a> {
    name: String,
    var: String,
    body: &'a str,
}

/// `latex` with every reference to one of `definitions` replaced by what it stands for.
pub fn resolve(latex: &str, definitions: &[String], settings: &Settings) -> Result<String> {
    let defined: Vec<Definition> = definitions
        .iter()
        .filter_map(|d| {
            let (name, var, body) = definition(d)?;
            Some(Definition { name, var, body: &d[body..] })
        })
        .collect();
    if defined.is_empty() {
        return Ok(latex.to_string());
    }
    // The input's own prefix stays as typed; its name may not refer to itself.
    let (own, var, start) = match definition(latex) {
        Some((name, var, body)) => (Some(name), var, body),
        None => (None, "x".to_string(), 0),
    };
    let mut stack: Vec<String> = own.into_iter().collect();
    let body = replace(&latex[start..], &var, &defined, &mut stack, settings)?;
    Ok(format!("{}{body}", &latex[..start]))
}

/// The text with its references replaced; `var` is the variable the arguments are written in.
fn replace(text: &str, var: &str, defined: &[Definition], stack: &mut Vec<String>, settings: &Settings) -> Result<String> {
    let mut out = String::new();
    let mut i = 0;
    while i < text.len() {
        let rest = &text[i..];
        let c = rest.chars().next().unwrap();
        // A command (`\frac`, `\sin`) is copied whole, so its letters are never names.
        if c == '\\' {
            let length = 1 + rest[1..].chars().take_while(|c| c.is_ascii_alphabetic()).count().max(1);
            let length = rest.char_indices().nth(length).map_or(rest.len(), |(k, _)| k);
            out.push_str(&rest[..length]);
            i += length;
            continue;
        }
        let after_letter = out.chars().last().is_some_and(|p| p.is_ascii_alphabetic());
        let found = (!after_letter)
            .then(|| {
                defined
                    .iter()
                    .filter(|d| rest.starts_with(d.name.as_str()))
                    .max_by_key(|d| d.name.len())
                    .and_then(|d| reference(rest, d).map(|(order, argument, length)| (d, order, argument, length)))
            })
            .flatten();
        match found {
            Some((d, order, argument, length)) => {
                if stack.contains(&d.name) {
                    return Err(QuantusError::other(msg!("core.analysis.references.circle", d.name.clone())));
                }
                let argument = replace(argument, var, defined, stack, settings)?;
                out.push_str(&format!("\\left({}\\right)", put_in(d, order, &argument, var, defined, stack, settings)?));
                i += length;
            }
            None => {
                out.push(c);
                i += c.len_utf8();
            }
        }
    }
    Ok(out)
}

/// A reference to `d` at the start of `text`: how often it is differentiated, the argument and
/// the length it takes. `f(x)`, `f'(2x)`, `f''\left(x\right)`, `f^{\prime}(x)`.
fn reference<'t>(text: &'t str, d: &Definition) -> Option<(usize, &'t str, usize)> {
    let mut i = d.name.len();
    let mut order = 0;
    loop {
        let rest = &text[i..];
        if let Some(r) = rest.strip_prefix('\'') {
            order += 1;
            i = text.len() - r.len();
        } else if let Some(r) = rest.strip_prefix("^{\\prime\\prime}") {
            order += 2;
            i = text.len() - r.len();
        } else if let Some(r) = rest.strip_prefix("^{\\prime}").or_else(|| rest.strip_prefix("^\\prime")) {
            order += 1;
            i = text.len() - r.len();
        } else {
            break;
        }
    }
    let rest = text[i..].trim_start();
    i = text.len() - rest.len();
    let (open, close) = if rest.starts_with("\\left(") { ("\\left(", "\\right)") } else if rest.starts_with('(') { ("(", ")") } else { return None };
    let inner_start = i + open.len();
    // The matching bracket, counting nested ones of either spelling.
    let mut depth = 1;
    let mut j = inner_start;
    while j < text.len() {
        let rest = &text[j..];
        if rest.starts_with("\\left(") {
            depth += 1;
            j += 6;
        } else if rest.starts_with('(') {
            depth += 1;
            j += 1;
        } else if rest.starts_with(close) && (close == "\\right)" || !text[..j].ends_with("\\right")) {
            depth -= 1;
            if depth == 0 {
                return Some((order, &text[inner_start..j], j + close.len()));
            }
            j += close.len();
        } else if rest.starts_with("\\right)") || rest.starts_with(')') {
            depth -= 1;
            j += if rest.starts_with(')') { 1 } else { 7 };
        } else {
            j += rest.chars().next().unwrap().len_utf8();
        }
    }
    None
}

/// The definition `order` times differentiated, at `argument`, in LaTeX.
fn put_in(d: &Definition, order: usize, argument: &str, var: &str, defined: &[Definition], stack: &mut Vec<String>, settings: &Settings) -> Result<String> {
    stack.push(d.name.clone());
    let body = replace(d.body, &d.var, defined, stack, settings)?;
    stack.pop();
    let (function, _) = parse_function(&body, Some(&d.var), None, settings)?;
    if function.pieces.len() != 1 || !function.pieces[0].on.is_reals() {
        return Err(QuantusError::other(msg!("core.analysis.references.pieces", d.name.clone())));
    }
    let mut f = function.pieces[0].expr.clone();
    for _ in 0..order {
        f = simplify(&derivative(&f));
    }
    let options = Settings { mode: Mode::Algebra, ..settings.clone() }.parse_options();
    let node = parser::parse_with(argument, &options)?.expression;
    let at = Expr::from_node(&node, var)?;
    // Put in as written, so `f(x - 1)` reads `(x - 1)^{2}` and not multiplied out.
    Ok(f.substitute(&at).to_latex(var))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolved(latex: &str, definitions: &[&str]) -> String {
        let definitions: Vec<String> = definitions.iter().map(|d| d.to_string()).collect();
        resolve(latex, &definitions, &Settings::default()).unwrap()
    }

    #[test]
    fn references_are_put_in() {
        let f = ["f(x) = x^2"];
        assert_eq!(resolved("g(x) = f(x-1) + 2", &f), "g(x) = \\left((x - 1)^{2}\\right) + 2");
        assert_eq!(resolved("f'(x)", &f), "\\left(2x\\right)");
        assert_eq!(resolved("h(x)=f^{\\prime\\prime}\\left(x\\right)", &f), "h(x)=\\left(2\\right)");
        assert_eq!(resolved("\\frac{f(x)}{x}", &f), "\\frac{\\left(x^{2}\\right)}{x}");
        // Names inside commands and other words stay.
        assert_eq!(resolved("\\sin(x) + f", &f), "\\sin(x) + f");
        assert_eq!(resolved("x^2", &[]), "x^2");
    }

    #[test]
    fn definitions_use_each_other_but_not_in_a_circle() {
        let chain = ["f(x) = x^2", "g(x) = f(x) + 1"];
        assert_eq!(resolved("h(t) = g(2t)", &chain), "h(t) = \\left((2t)^{2} + 1\\right)");
        let circle: Vec<String> = ["f(x) = g(x)", "g(x) = f(x)"].iter().map(|s| s.to_string()).collect();
        assert!(resolve("f(x)", &circle, &Settings::default()).unwrap_err().to_string().contains("itself"));
        let own = vec!["f(x) = x".to_string()];
        assert!(resolve("f(x) = f(x-1)", &own, &Settings::default()).is_err());
    }
}
