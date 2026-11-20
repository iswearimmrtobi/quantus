//! The function as typed: `f(x) = …`, `f\left(x\right) = …` (as MathLive writes it), `y = …` or
//! just the expression. The prefix is blanked out with spaces rather than cut off, so a parse
//! error still points at the right place in what the user typed.

use super::expr::Expr;
use super::function::{parse_condition, Condition, Function, Piece};
use super::interval::IntervalSet;
use crate::algebra::pipeline::collect_variables;
use crate::latex::{parser, Node};
use crate::settings::{Mode, Settings};
use crate::text::msg;
use crate::{QuantusError, Result};

/// The function and the name of its variable. A domain clause after the expression
/// (`f(x) = x^2, \ x \in [0, 3]`) and `domain` (from its own field) restrict it; a
/// `\begin{cases}` gives a piecewise function.
pub fn parse_function(latex: &str, variable: Option<&str>, domain: Option<&str>, settings: &Settings) -> Result<(Function, String)> {
    let (source, named) = blank_prefix(latex);
    let options = Settings { mode: Mode::Algebra, ..settings.clone() }.parse_options();
    let (body, clause) = split_clause(&source);
    // Each piece as (the expression padded to its place in the input, its condition).
    let rows: Vec<Row> = match cases_rows(body) {
        Some(rows) => rows?,
        None => vec![(body.to_string(), None)],
    };
    let mut nodes = Vec::new();
    for (expression, _) in &rows {
        let formula = parser::parse_with(expression, &options)?;
        if let Node::Equation { .. } | Node::System { .. } = formula.expression {
            return Err(QuantusError::other(msg!("core.analysis.input.enter_function_not_equation")));
        }
        nodes.push(formula.expression);
    }
    let var = match (variable, named) {
        (Some(asked), Some(named)) if asked != named => {
            return Err(QuantusError::other(msg!(
                "core.analysis.input.wrong_function_name")))
        }
        (Some(asked), _) => asked.to_string(),
        (None, Some(named)) => named,
        (None, None) => {
            let found: std::collections::BTreeSet<String> = nodes.iter().flat_map(collect_variables).collect();
            match found.len() {
                0 => "x".to_string(),
                1 => found.into_iter().next().unwrap(),
                _ => {
                    let listed = found.into_iter().collect::<Vec<_>>().join(", ");
                    return Err(QuantusError::other(msg!(
                        "core.analysis.input.one_variable_only")));
                }
            }
        }
    };
    let mut restriction = IntervalSet::reals();
    for given in [clause.map(|(_, c)| c), domain.filter(|d| !d.trim().is_empty())].into_iter().flatten() {
        match parse_condition(given, &var, settings)? {
            Condition::Set(set) => restriction = restriction.intersect(&set),
            Condition::Otherwise => return Err(QuantusError::other(msg!("core.analysis.input.condition_not_understood", given.trim()))),
        }
    }
    if restriction.is_empty() {
        return Err(QuantusError::other(msg!("core.analysis.input.empty_domain")));
    }
    let mut pieces = Vec::new();
    let mut otherwise = None;
    for ((_, condition), node) in rows.iter().zip(&nodes) {
        let expr = Expr::from_node(node, &var)?;
        let on = match condition {
            None => IntervalSet::reals(),
            Some(condition) => match parse_condition(condition, &var, settings)? {
                Condition::Set(set) => set,
                Condition::Otherwise if otherwise.is_none() && rows.len() > 1 => {
                    otherwise = Some(pieces.len());
                    IntervalSet::empty()
                }
                Condition::Otherwise => return Err(QuantusError::other(msg!("core.analysis.input.piece_needs_condition"))),
            },
        };
        pieces.push(Piece { expr, on });
    }
    if let Some(index) = otherwise {
        let others = pieces.iter().fold(IntervalSet::empty(), |all, p| all.union(&p.on));
        pieces[index].on = others.complement();
    }
    for i in 0..pieces.len() {
        for j in i + 1..pieces.len() {
            let both = pieces[i].on.intersect(&pieces[j].on);
            if !both.is_empty() {
                let at = both.to_latex(super::DEFAULT_DIGITS, settings.language);
                return Err(QuantusError::other(msg!("core.analysis.input.pieces_overlap", i + 1, j + 1, at)));
            }
        }
    }
    for piece in &mut pieces {
        piece.on = piece.on.intersect(&restriction);
    }
    pieces.retain(|p| !p.on.is_empty());
    if pieces.is_empty() {
        return Err(QuantusError::other(msg!("core.analysis.input.empty_domain")));
    }
    Ok((Function { pieces }, var))
}

/// The expression and a domain clause after it: `x^2, \ x \in [0, 3]`, `x^2; \ 0 \le x \le 3`,
/// `x^2 \quad D = [0, 3]` or `x^2 \text{ für } x > 0`. A separator only counts when what
/// follows it is a condition, so a decimal comma (`0,5x`) stays in the expression.
fn split_clause(source: &str) -> (&str, Option<(usize, &str)>) {
    const RELATIONS: [&str; 16] = ["<", ">", "\\le", "\\ge", "\\lt", "\\gt", "\\in", "\\notin", "\\ne", "≤", "≥", "≠", "D=", "D =", "D_", "\\mathbb{D}"];
    let bytes = source.as_bytes();
    let mut depth = 0i32;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' | b'(' => depth += 1,
            b'}' | b')' => depth -= 1,
            _ => {}
        }
        let separator = if depth != 0 || !source.is_char_boundary(i) {
            None
        } else if bytes[i] == b',' || bytes[i] == b';' {
            Some(1)
        } else {
            ["\\qquad", "\\quad", "\\text{"].iter().find(|s| source[i..].starts_with(**s)).map(|s| s.len())
        };
        if let Some(length) = separator {
            let rest = &source[i + length..];
            let is_text = source[i..].starts_with("\\text{");
            let words_only = is_text && {
                let word: String = rest.split('}').next().unwrap_or("").trim().to_lowercase();
                matches!(word.as_str(), "für" | "for" | "if" | "falls" | "wenn" | "mit" | "with" | "on" | "auf")
            };
            // Inside cases a comma belongs to a row.
            let in_cases = source[..i].contains("\\begin{cases}") && !source[..i].contains("\\end{cases}");
            if (!is_text || words_only) && !in_cases && RELATIONS.iter().any(|r| rest.contains(r)) {
                let start = if is_text { i + length + rest.find('}').map_or(0, |k| k + 1) } else { i + length };
                return (&source[..i], Some((start, &source[start..])));
            }
        }
        i += 1;
    }
    (source, None)
}

/// A piece as typed: the expression, padded to its place in the input, and its condition.
type Row = (String, Option<String>);

/// The rows of `\begin{cases} … \end{cases}` as (expression, condition), each expression padded
/// with spaces to where it stands so parse errors point at the right place; `None` when the
/// body is not cases.
fn cases_rows(body: &str) -> Option<Result<Vec<Row>>> {
    const BEGIN: &str = "\\begin{cases}";
    const END: &str = "\\end{cases}";
    let trimmed = body.trim_start();
    let mut start = body.len() - trimmed.len();
    // MathLive may wrap it: \left\{ \begin{cases} … \end{cases} \right.
    for wrapper in ["\\left\\{", "\\left."] {
        if body[start..].starts_with(wrapper) {
            start += wrapper.len();
            start += body[start..].len() - body[start..].trim_start().len();
        }
    }
    if !body[start..].starts_with(BEGIN) {
        return None;
    }
    let inner_start = start + BEGIN.len();
    let Some(end) = body.rfind(END) else { return Some(Err(QuantusError::other(msg!("core.analysis.input.cases_rows")))) };
    let mut rows = Vec::new();
    let mut row_start = inner_start;
    let mut depth = 0i32;
    let mut i = inner_start;
    let bytes = body.as_bytes();
    let cut = |from: usize, to: usize, rows: &mut Vec<Row>| -> Result<()> {
        let row = &body[from..to];
        if row.trim().is_empty() {
            return Ok(());
        }
        // `expr & condition`, or `expr, condition` without the ampersand.
        let (expression, condition) = match top_level_find(row, "&") {
            Some(k) => (&row[..k], Some(row[k + 1..].to_string())),
            None => match split_clause(row) {
                (expression, Some((_, condition))) => (expression, Some(condition.to_string())),
                _ => return Err(QuantusError::other(msg!("core.analysis.input.cases_rows"))),
            },
        };
        rows.push((format!("{}{}", " ".repeat(from), expression), condition));
        Ok(())
    };
    while i < end {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
        if depth == 0 && body.is_char_boundary(i) && body[i..].starts_with("\\\\") {
            if let Err(e) = cut(row_start, i, &mut rows) {
                return Some(Err(e));
            }
            i += 2;
            row_start = i;
            continue;
        }
        i += 1;
    }
    if let Err(e) = cut(row_start, end, &mut rows) {
        return Some(Err(e));
    }
    if rows.is_empty() {
        return Some(Err(QuantusError::other(msg!("core.analysis.input.cases_rows"))));
    }
    Some(Ok(rows))
}

/// The first `needle` outside braces.
fn top_level_find(text: &str, needle: &str) -> Option<usize> {
    let mut depth = 0i32;
    for (i, c) in text.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => depth -= 1,
            _ => {}
        }
        if depth == 0 && text[i..].starts_with(needle) && !text[..i].ends_with('\\') {
            return Some(i);
        }
    }
    None
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
        let (f, var) = parse_function(latex, None, None, &Settings::default()).unwrap();
        (f.to_latex(&var, 6, crate::settings::Language::En), var)
    }

    #[test]
    fn prefixes_are_accepted_and_name_the_variable() {
        assert_eq!(parsed("f(x) = x^2"), ("x^{2}".into(), "x".into()));
        assert_eq!(parsed("f\\left(t\\right)=t^2+1"), ("t^{2} + 1".into(), "t".into()));
        assert_eq!(parsed("g_{1}(x) = 2x"), ("2x".into(), "x".into()));
        assert_eq!(parsed("y = x^3"), ("x^{3}".into(), "x".into()));
        assert_eq!(parsed("x^3 - 3x"), ("x^{3} - 3x".into(), "x".into()));
        assert_eq!(parsed("t e^t"), ("te^{t}".into(), "t".into()));
        assert_eq!(parsed("5"), ("5".into(), "x".into()));
    }

    #[test]
    fn a_parse_error_keeps_its_position() {
        let error = parse_function("f(x) = x^2 +", None, None, &Settings::default()).unwrap_err();
        let bare = parse_function("x^2 +", None, None, &Settings::default()).unwrap_err();
        assert_eq!(error.position(), bare.position().map(|p| p + 7));
    }

    #[test]
    fn families_and_equations_are_errors() {
        assert!(parse_function("x^2 + a", None, None, &Settings::default()).unwrap_err().to_string().contains("function families"));
        assert!(parse_function("x^2 = 4", None, None, &Settings::default()).unwrap_err().to_string().contains("not an equation"));
        assert!(parse_function("f(x) = t^2", None, None, &Settings::default()).unwrap_err().to_string().contains("function families"));
        assert!(parse_function("f(x) = x^2", Some("t"), None, &Settings::default()).is_err());
    }

    #[test]
    fn a_domain_clause_restricts_the_function() {
        assert_eq!(parsed("f(x) = x^2, \\ x \\in [0, 3]").0, "x^{2}, \\ 0 \\le x \\le 3");
        assert_eq!(parsed("x^2;\\ 0\\le x<3").0, "x^{2}, \\ 0 \\le x < 3");
        assert_eq!(parsed("x^2 \\quad D=[-1,1]").0, "x^{2}, \\ -1 \\le x \\le 1");
        assert_eq!(parsed("\\sqrt{x}\\text{ für }x>1").0, "\\sqrt{x}, \\ x > 1");
        let (f, _) = parse_function("x^2", None, Some("x \\ge 2"), &Settings::default()).unwrap();
        assert_eq!(f.to_latex("x", 6, crate::settings::Language::En), "x^{2}, \\ x \\ge 2");
        assert!(parse_function("x^2, x \\in [0,1]", None, Some("x > 2"), &Settings::default()).unwrap_err().to_string().contains("empty"));
        let german = Settings { language: crate::settings::Language::De, ..Settings::default() };
        let (f, _) = parse_function("0,5x^2", None, None, &german).unwrap();
        assert!(!f.is_restricted());
    }

    #[test]
    fn cases_give_a_piecewise_function() {
        let (f, var) = parse_function("f(x)=\\begin{cases}x^2 & x<0\\\\ x & x\\ge 0\\end{cases}", None, None, &Settings::default()).unwrap();
        assert_eq!(var, "x");
        assert_eq!(f.pieces.len(), 2);
        assert_eq!(f.to_latex("x", 6, crate::settings::Language::En), "\\begin{cases} x^{2} & x < 0 \\\\ x & x \\ge 0 \\end{cases}");
        assert_eq!(f.joins().len(), 1);
        let (otherwise, _) = parse_function("\\begin{cases}1 & 0\\le x\\le 1\\\\ 0 & \\text{sonst}\\end{cases}", None, None, &Settings::default()).unwrap();
        assert_eq!(otherwise.pieces[1].on.to_latex(6, crate::settings::Language::En), "(-\\infty, 0) \\cup (1, \\infty)");
        let overlap = parse_function("\\begin{cases}x & x<1\\\\ 2 & x>0\\end{cases}", None, None, &Settings::default()).unwrap_err();
        assert!(overlap.to_string().contains("overlap"), "{overlap}");
        let error = parse_function("\\begin{cases}x+ & x<1\\\\ 2 & x\\ge 1\\end{cases}", None, None, &Settings::default()).unwrap_err();
        assert!(error.position().is_some_and(|p| p >= 14), "{:?}", error.position());
    }
}
