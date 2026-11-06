//! LaTeX of the values, points and vectors in a geometry working.

use crate::eval::calculator::fraction_node;
use crate::latex::{exact, serializer};
use crate::linear;
use crate::num::{Decimal, Value};
use crate::text;

use super::{Angle, Point, Vector3};
use crate::settings::AngleUnit;

/// A value exactly: a whole number or fraction, a root or multiple of `π`; anything else as a
/// short decimal.
pub(crate) fn value(value: &Value) -> String {
    let mut value = value.clone();
    linear::settle(&mut value);
    if let Some(rational) = value.as_rational() {
        return serializer::expression(&fraction_node(rational));
    }
    if let Some(exact) = exact::format(&value) {
        return exact;
    }
    match value.decimal(12) {
        Ok(d) => decimal(&d),
        Err(_) => "?".into(),
    }
}

/// A decimal with six digits, with a decimal comma in German.
pub(crate) fn decimal(d: &Decimal) -> String {
    let plain = d.round(6).to_string();
    match text::language().decimal() {
        "." => plain,
        separator => plain.replace('.', &format!("{{{separator}}}")),
    }
}

fn is_sum(latex: &str) -> bool {
    latex.get(1..).is_some_and(|rest| rest.contains(" + ") || rest.contains(" - "))
}

/// A value as a factor or a power base: in brackets if it is a sum or negative.
pub(crate) fn factor(v: &Value) -> String {
    let shown = value(v);
    if is_sum(&shown) || shown.starts_with('-') { format!("\\left({shown}\\right)") } else { shown }
}

/// The coordinates that are written: two for a planar object.
fn coordinates(coords: &[Value; 3], planar: bool) -> &[Value] {
    if planar { &coords[..2] } else { coords }
}

/// A vector as a column.
pub(crate) fn vector(v: &Vector3) -> String {
    let entries: Vec<String> = coordinates(&v.coords, v.planar).iter().map(value).collect();
    format!("\\begin{{pmatrix}} {} \\end{{pmatrix}}", entries.join(" \\\\ "))
}

/// A point in school notation, `(1|2|3)`.
pub(crate) fn point(p: &Point) -> String {
    let entries: Vec<String> = coordinates(&p.coords, p.planar).iter().map(value).collect();
    format!("({})", entries.join("|"))
}

/// A sum of multiples, `2x - y + \frac{1}{2}z - 3`; a term with an empty symbol is a constant.
/// Zero terms are left out, a coefficient 1 is not written.
pub(crate) fn combination(terms: &[(&Value, &str)]) -> String {
    let mut out = String::new();
    for (coefficient, symbol) in terms {
        let mut coefficient = (*coefficient).clone();
        linear::settle(&mut coefficient);
        if coefficient.is_zero() {
            continue;
        }
        let mut shown = value(&coefficient);
        let negative = shown.starts_with('-') && !is_sum(&shown);
        if negative {
            shown.remove(0);
        }
        if !symbol.is_empty() {
            if shown == "1" {
                shown.clear();
            } else if is_sum(&shown) {
                shown = format!("\\left({shown}\\right)");
            }
        }
        let term = format!("{shown}{symbol}");
        out = match (out.is_empty(), negative) {
            (true, false) => term,
            (true, true) => format!("-{term}"),
            (false, false) => format!("{out} + {term}"),
            (false, true) => format!("{out} - {term}"),
        };
    }
    if out.is_empty() { "0".into() } else { out }
}

/// The length of `v` written out, `\sqrt{1^2 + (-2)^2 + 2^2}`.
pub(crate) fn length(v: &Vector3) -> String {
    let squares: Vec<String> = coordinates(&v.coords, v.planar).iter().map(|c| format!("{}^2", factor(c))).collect();
    format!("\\sqrt{{{}}}", squares.join(" + "))
}

/// The dot product of `a` and `b` written out, `1 \cdot 3 + 2 \cdot (-1) + 0 \cdot 4`.
pub(crate) fn dot(a: &Vector3, b: &Vector3) -> String {
    let planar = a.planar && b.planar;
    let products: Vec<String> = coordinates(&a.coords, planar)
        .iter()
        .zip(coordinates(&b.coords, planar))
        .enumerate()
        .map(|(i, (x, y))| format!("{} \\cdot {}", if i == 0 { factor_first(x) } else { factor(x) }, factor(y)))
        .collect();
    products.join(" + ")
}

/// The cross product of `a` and `b` written out by its components,
/// `\begin{pmatrix} 2 \cdot 4 - 3 \cdot 0 \\ … \end{pmatrix}`.
pub(crate) fn cross(a: &Vector3, b: &Vector3) -> String {
    let entry = |i: usize, j: usize| {
        format!("{} \\cdot {} - {} \\cdot {}", factor_first(&a.coords[i]), factor(&b.coords[j]), factor(&a.coords[j]), factor(&b.coords[i]))
    };
    format!("\\begin{{pmatrix}} {} \\\\ {} \\\\ {} \\end{{pmatrix}}", entry(1, 2), entry(2, 0), entry(0, 1))
}

/// A value as the first factor of a product: brackets only around a sum.
fn factor_first(v: &Value) -> String {
    let shown = value(v);
    if is_sum(&shown) { format!("\\left({shown}\\right)") } else { shown }
}

/// An angle, `\varphi = 60^\circ` where it is exact, else `\varphi = \arccos\left(\frac{11}{15}\right) \approx 42.8334^\circ`.
pub(crate) fn angle(angle: &Angle) -> String {
    let degree = if angle.unit == AngleUnit::Degree { "^\\circ" } else { "" };
    let exact = match angle.value.as_rational() {
        Some(rational) => Some(serializer::expression(&fraction_node(rational))),
        None => exact::format(&angle.value),
    };
    let approx = format!("\\approx {}{degree}", decimal(&angle.decimal));
    match exact {
        Some(exact) if angle.value.to_i64().is_some() => format!("\\varphi = {exact}{degree}"),
        Some(exact) => format!("\\varphi = {exact}{degree} {approx}"),
        None => format!("\\varphi = \\{}\\left({}\\right) {approx}", angle.function(), value(&angle.ratio)),
    }
}
