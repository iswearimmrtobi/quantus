//! The function as typed: `f(x) = …`, `f\left(x\right) = …` (as MathLive writes it), `y = …` or
//! just the expression. The prefix is blanked out with spaces rather than cut off, so a parse
//! error still points at the right place in what the user typed.

use super::expr::Expr;
use crate::algebra::pipeline::collect_variables;
use crate::latex::{parser, Node};
use crate::settings::{Mode, Settings};
use crate::text::msg;
use crate::{QuantusError, Result};

/// The function and the name of its variable.
pub fn parse_function(latex: &str, variable: Option<&str>, settings: &Settings) -> Result<(Expr, String)> {
    let (source, named) = blank_prefix(latex);
    let options = Settings { mode: Mode::Algebra, ..settings.clone() }.parse_options();
    let formula = parser::parse_with(&source, &options)?;
    if let Node::Equation { .. } = formula.expression {
        return Err(QuantusError::other(msg!("Enter a function, not an equation.", "Gib eine Funktion ein, keine Gleichung.")));
    }
    let var = match (variable, named) {
        (Some(asked), Some(named)) if asked != named => {
            return Err(QuantusError::other(msg!(
                "The function is written in {named}, but {asked} was asked for.",
                "Die Funktion ist in {named} geschrieben, gefragt war aber {asked}."
            )))
        }
        (Some(asked), _) => asked.to_string(),
        (None, Some(named)) => named,
        (None, None) => {
            let found = collect_variables(&formula.expression);
            match found.len() {
                0 => "x".to_string(),
                1 => found.into_iter().next().unwrap(),
                _ => {
                    let listed = found.into_iter().collect::<Vec<_>>().join(", ");
                    return Err(QuantusError::other(msg!(
                        "f may depend on one variable only, but it contains {listed}; function families are not supported yet.",
                        "f darf nur von einer Variablen abhängen, enthält aber {listed}; Funktionenscharen werden noch nicht unterstützt."
                    )));
                }
            }
        }
    };
    Ok((Expr::from_node(&formula.expression, &var)?, var))
}

/// The input with a leading `f(x) =` or `y =` replaced by spaces, and the variable it named.
fn blank_prefix(latex: &str) -> (String, Option<String>) {
    let bytes = latex.as_bytes();
    let mut i = 0;
    let skip_spaces = |mut i: usize| {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        i
    };
    let eat = |i: usize, text: &str| latex[i..].starts_with(text).then(|| i + text.len());
    i = skip_spaces(i);
    let Some(&name) = bytes.get(i) else { return (latex.to_string(), None) };
    if !name.is_ascii_alphabetic() {
        return (latex.to_string(), None);
    }
    let after_name = i + 1;
    let blank = |end: usize| format!("{}{}", " ".repeat(end), &latex[end..]);

    // y = …
    if name == b'y' {
        if let Some(end) = eat(skip_spaces(after_name), "=") {
            return (blank(end), None);
        }
    }
    // f(x) = …, f_{1}(x) = …, f\left(x\right) = …
    let mut j = after_name;
    if let Some(k) = eat(j, "_") {
        j = if let Some(k) = eat(k, "{") {
            match latex[k..].find('}') {
                Some(close) => k + close + 1,
                None => return (latex.to_string(), None),
            }
        } else {
            k + 1
        };
    }
    let Some(j) = eat(j, "\\left(").or_else(|| eat(j, "(")) else { return (latex.to_string(), None) };
    let j = skip_spaces(j);
    let Some(&var) = bytes.get(j) else { return (latex.to_string(), None) };
    if !var.is_ascii_alphabetic() {
        return (latex.to_string(), None);
    }
    let k = skip_spaces(j + 1);
    let Some(k) = eat(k, "\\right)").or_else(|| eat(k, ")")) else { return (latex.to_string(), None) };
    let Some(end) = eat(skip_spaces(k), "=") else { return (latex.to_string(), None) };
    (blank(end), Some((var as char).to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(latex: &str) -> (String, String) {
        let (f, var) = parse_function(latex, None, &Settings::default()).unwrap();
        (f.to_latex(&var), var)
    }

    #[test]
    fn prefixes_are_accepted_and_name_the_variable() {
        assert_eq!(parsed("f(x) = x^2"), ("x^{2}".into(), "x".into()));
        assert_eq!(parsed("f\\left(t\\right)=t^2+1"), ("t^{2} + 1".into(), "t".into()));
        assert_eq!(parsed("g_{1}(x) = 2x"), ("2x".into(), "x".into()));
        assert_eq!(parsed("y = x^3"), ("x^{3}".into(), "x".into()));
        assert_eq!(parsed("x^3 - 3x"), ("x^{3} - 3x".into(), "x".into()));
        assert_eq!(parsed("t e^t"), ("t \\cdot e^{t}".into(), "t".into()));
        assert_eq!(parsed("5"), ("5".into(), "x".into()));
    }

    #[test]
    fn a_parse_error_keeps_its_position() {
        let error = parse_function("f(x) = x^2 +", None, &Settings::default()).unwrap_err();
        let bare = parse_function("x^2 +", None, &Settings::default()).unwrap_err();
        assert_eq!(error.position(), bare.position().map(|p| p + 7));
    }

    #[test]
    fn families_and_equations_are_errors() {
        assert!(parse_function("x^2 + a", None, &Settings::default()).unwrap_err().to_string().contains("function families"));
        assert!(parse_function("x^2 = 4", None, &Settings::default()).unwrap_err().to_string().contains("not an equation"));
        assert!(parse_function("f(x) = t^2", None, &Settings::default()).unwrap_err().to_string().contains("function families"));
        assert!(parse_function("f(x) = x^2", Some("t"), &Settings::default()).is_err());
    }
}
