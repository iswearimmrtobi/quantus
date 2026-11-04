//! Matrix mode: the result is a `pmatrix` whose entries are read back as exact numbers.

use super::*;

/// `expect.rows` are the entries, row by row, as `p/q`.
pub(super) fn check(settings: &Settings, expect: &Value, calc: &Calculation) -> Outcome {
    let shown = calc.result.clone();
    let Some(body) = shown.strip_prefix("\\begin{pmatrix}").and_then(|s| s.strip_suffix("\\end{pmatrix}")) else {
        return Outcome::fail("wrong-value", "expected a matrix".into(), shown);
    };
    let rows: Vec<Vec<&str>> = body.split("\\\\").map(|row| row.split('&').map(str::trim).collect()).collect();
    let want: Vec<Vec<&str>> = expect["rows"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|row| row.as_array().into_iter().flatten().filter_map(Value::as_str).collect())
        .collect();
    let size = |m: &[Vec<&str>]| format!("{}×{}", m.len(), m.first().map_or(0, Vec::len));
    if rows.len() != want.len() || rows.iter().zip(&want).any(|(a, b)| a.len() != b.len()) {
        return Outcome::fail("wrong-size", format!("expected {}, got {}", size(&want), size(&rows)), shown);
    }
    let empty = Variables::new();
    for (r, (got, want)) in rows.iter().zip(&want).enumerate() {
        for (c, (cell, want)) in got.iter().zip(want).enumerate() {
            let Some(want) = parse_rational(want) else { return generator("unparsable entry") };
            let value = match latex_complex(cell, settings, &empty) {
                Ok(v) => v,
                Err(e) => return Outcome::fail("result-unparsable", e, shown),
            };
            if !close_complex(&value, &(want.clone(), RBig::ZERO)) {
                return Outcome::fail("wrong-value", format!("entry ({}, {}): expected {}, got {cell}", r + 1, c + 1, show(&want)), shown);
            }
        }
    }
    Outcome::pass(shown)
}
