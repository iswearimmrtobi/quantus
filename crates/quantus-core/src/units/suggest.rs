//! The units a handheld's soft keys offer while a Physik input is typed (firmware/docs/unit-keys.md
//! §2): read from the input before the cursor, by rules, without learning.
//!
//! The strongest signal comes first:
//!
//! 1. after `+`, `−` or `=`, the units of the left side's dimension;
//! 2. after `·` or `÷`, the units that combine with the left side into a named quantity;
//! 3. a lone letter left of `=` (`P =`, `F =`), the unit of the quantity it usually names;
//! 4. otherwise the units used last, then the common ones of mechanics, electricity and heat.
//!
//! The input is unfinished (`5 N ·` does not parse), so it is cut at its last operator and only
//! the left side is read.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use super::Dimension;
use crate::eval::Evaluator;
use crate::latex::parser::{parse_with, ParseOptions};
use crate::settings::Mode;

/// A unit for a soft key: what it inserts, and what its label shows.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UnitSuggestion {
    pub latex: String,
    pub label: String,
}

/// The units to offer, best first: `best` for the first row of keys, `more` for the second and
/// further pages.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UnitSuggestions {
    pub best: Vec<UnitSuggestion>,
    pub more: Vec<UnitSuggestion>,
}

/// One unit of the tables: its label (also its name in the list of recent units) and its LaTeX.
type Entry = (&'static str, &'static str);

/// Units of one dimension, the common ones first.
#[rustfmt::skip]
const FAMILIES: &[&[Entry]] = &[
    &[("m", r"\mathrm{m}"), ("km", r"\mathrm{km}"), ("cm", r"\mathrm{cm}"), ("mm", r"\mathrm{mm}"), ("µm", r"\mathrm{\mu m}"), ("nm", r"\mathrm{nm}")],
    &[("kg", r"\mathrm{kg}"), ("g", r"\mathrm{g}"), ("mg", r"\mathrm{mg}"), ("t", r"\mathrm{t}")],
    &[("s", r"\mathrm{s}"), ("min", r"\mathrm{min}"), ("h", r"\mathrm{h}"), ("ms", r"\mathrm{ms}"), ("µs", r"\mathrm{\mu s}"), ("d", r"\mathrm{d}")],
    &[("A", r"\mathrm{A}"), ("mA", r"\mathrm{mA}"), ("µA", r"\mathrm{\mu A}"), ("kA", r"\mathrm{kA}")],
    &[("K", r"\mathrm{K}"), ("°C", r"^\circ\mathrm{C}")],
    &[("mol", r"\mathrm{mol}"), ("mmol", r"\mathrm{mmol}")],
    &[("N", r"\mathrm{N}"), ("kN", r"\mathrm{kN}"), ("mN", r"\mathrm{mN}"), ("MN", r"\mathrm{MN}")],
    &[("J", r"\mathrm{J}"), ("kJ", r"\mathrm{kJ}"), ("MJ", r"\mathrm{MJ}"), ("eV", r"\mathrm{eV}"), ("Wh", r"\mathrm{Wh}"), ("kWh", r"\mathrm{kWh}"), ("cal", r"\mathrm{cal}"), ("kcal", r"\mathrm{kcal}")],
    &[("W", r"\mathrm{W}"), ("kW", r"\mathrm{kW}"), ("MW", r"\mathrm{MW}"), ("mW", r"\mathrm{mW}"), ("hp", r"\mathrm{hp}")],
    &[("Pa", r"\mathrm{Pa}"), ("kPa", r"\mathrm{kPa}"), ("hPa", r"\mathrm{hPa}"), ("bar", r"\mathrm{bar}"), ("MPa", r"\mathrm{MPa}"), ("mbar", r"\mathrm{mbar}"), ("atm", r"\mathrm{atm}")],
    &[("Hz", r"\mathrm{Hz}"), ("kHz", r"\mathrm{kHz}"), ("MHz", r"\mathrm{MHz}"), ("GHz", r"\mathrm{GHz}")],
    &[("C", r"\mathrm{C}"), ("mC", r"\mathrm{mC}"), ("µC", r"\mathrm{\mu C}"), ("nC", r"\mathrm{nC}")],
    &[("V", r"\mathrm{V}"), ("kV", r"\mathrm{kV}"), ("mV", r"\mathrm{mV}"), ("µV", r"\mathrm{\mu V}")],
    &[("Ω", r"\mathrm{\Omega}"), ("kΩ", r"\mathrm{k\Omega}"), ("MΩ", r"\mathrm{M\Omega}"), ("mΩ", r"\mathrm{m\Omega}")],
    &[("F", r"\mathrm{F}"), ("µF", r"\mathrm{\mu F}"), ("nF", r"\mathrm{nF}"), ("pF", r"\mathrm{pF}"), ("mF", r"\mathrm{mF}")],
    &[("H", r"\mathrm{H}"), ("mH", r"\mathrm{mH}"), ("µH", r"\mathrm{\mu H}")],
    &[("T", r"\mathrm{T}"), ("mT", r"\mathrm{mT}"), ("µT", r"\mathrm{\mu T}")],
    &[("Wb", r"\mathrm{Wb}")],
    &[("m/s", r"\frac{\mathrm{m}}{\mathrm{s}}"), ("km/h", r"\frac{\mathrm{km}}{\mathrm{h}}")],
    &[("m/s²", r"\frac{\mathrm{m}}{\mathrm{s}^{2}}")],
    &[("m²", r"\mathrm{m}^{2}"), ("cm²", r"\mathrm{cm}^{2}"), ("mm²", r"\mathrm{mm}^{2}"), ("km²", r"\mathrm{km}^{2}"), ("ha", r"\mathrm{ha}")],
    &[("m³", r"\mathrm{m}^{3}"), ("l", r"\mathrm{l}"), ("ml", r"\mathrm{ml}"), ("cm³", r"\mathrm{cm}^{3}"), ("dm³", r"\mathrm{dm}^{3}")],
    &[("kg/m³", r"\frac{\mathrm{kg}}{\mathrm{m}^{3}}"), ("g/cm³", r"\frac{\mathrm{g}}{\mathrm{cm}^{3}}")],
    &[("kg·m/s", r"\mathrm{kg} \cdot \frac{\mathrm{m}}{\mathrm{s}}"), ("N·s", r"\mathrm{N} \cdot \mathrm{s}")],
];

/// What the soft keys offer with nothing to go by: mechanics, electricity, heat.
const DEFAULTS: &[&str] = &["m", "s", "kg", "N", "J", "W", "V", "A", "Ω", "Pa", "K", "Hz", "km", "h", "kW", "kN", "kJ", "mA", "°C", "m/s"];

/// After `·`: the left side's unit, and what makes a named quantity with it (`N·m`, `W·h`).
#[rustfmt::skip]
const TIMES: &[(&str, &[&str])] = &[
    ("N", &["m", "s"]),
    ("W", &["h", "s", "min"]),
    ("V", &["A", "s", "C"]),
    ("A", &["s", "V", "Ω", "h"]),
    ("kg", &["m/s²", "m/s", "m"]),
    ("m", &["N", "m", "kg"]),
    ("s", &["W", "A", "V", "m/s", "m/s²"]),
    ("Ω", &["A"]),
    ("Pa", &["m²", "m³"]),
    ("C", &["V"]),
    ("F", &["V"]),
    ("m/s", &["s", "kg"]),
    ("m/s²", &["kg", "s"]),
    ("m²", &["m", "Pa"]),
    ("m³", &["kg/m³", "Pa"]),
    ("kg/m³", &["m³"]),
    ("T", &["m²", "A"]),
    ("Hz", &["m", "s"]),
];

/// After `÷`: the left side's unit, and what it is usually divided by (`m/s`, `J/s`).
#[rustfmt::skip]
const DIVIDE: &[(&str, &[&str])] = &[
    ("m", &["s", "m"]),
    ("J", &["s", "kg", "K", "mol"]),
    ("V", &["A", "Ω"]),
    ("N", &["m²", "kg", "m"]),
    ("kg", &["m³", "mol", "s"]),
    ("W", &["A", "V", "m²"]),
    ("C", &["s", "V"]),
    ("m/s", &["s"]),
    ("A", &["V", "s"]),
    ("m³", &["s", "kg"]),
    ("Wb", &["s", "m²"]),
];

/// A lone letter left of `=`: the unit of the quantity it usually names. Below the units already
/// in the line, since a letter can stand for more than one quantity (`P` is also a pressure).
#[rustfmt::skip]
const LETTERS: &[(&str, &str)] = &[
    ("P", "W"), ("F", "N"), ("U", "V"), ("I", "A"), ("R", "Ω"), ("E", "J"), ("W", "J"),
    ("Q", "C"), ("C", "F"), ("m", "kg"), ("s", "m"), ("t", "s"), ("v", "m/s"), ("a", "m/s²"),
    ("p", "Pa"), ("T", "K"), ("f", "Hz"), ("\\rho", "kg/m³"), ("V", "m³"), ("L", "H"),
    ("B", "T"), ("h", "m"), ("l", "m"), ("\\lambda", "m"), ("x", "m"), ("d", "m"), ("r", "m"),
    ("g", "m/s²"), ("A", "m²"), ("\\vartheta", "°C"), ("\\theta", "°C"), ("n", "mol"),
];

/// The table entry labelled `label`.
fn entry(label: &str) -> Option<Entry> {
    FAMILIES.iter().flat_map(|family| family.iter()).find(|(l, _)| *l == label).copied()
}

/// The dimension of each family, computed once from its first unit.
fn family_dimensions() -> &'static [Option<Dimension>] {
    static DIMENSIONS: OnceLock<Vec<Option<Dimension>>> = OnceLock::new();
    DIMENSIONS.get_or_init(|| FAMILIES.iter().map(|family| unit_dimension(family[0].1)).collect())
}

/// The dimension of a unit of the tables; `°C` reads only after a number.
fn unit_dimension(latex: &str) -> Option<Dimension> {
    dimension(latex).or_else(|| dimension(&format!("1{latex}")))
}

/// The family of units with `dimension`.
fn family_of(dimension: &Dimension) -> Option<&'static [Entry]> {
    let index = family_dimensions().iter().position(|d| d.as_ref() == Some(dimension))?;
    Some(FAMILIES[index])
}

/// The family `label` belongs to.
fn family_with(label: &str) -> Option<&'static [Entry]> {
    FAMILIES.iter().copied().find(|family| family.iter().any(|(l, _)| *l == label))
}

/// The dimension of `latex` read in Physik, if it reads and is not a plain number.
pub fn dimension(latex: &str) -> Option<Dimension> {
    let formula = parse_with(latex, &ParseOptions::default()).ok()?;
    let variables = Default::default();
    let quantity = Evaluator::new(&variables).with_mode(Mode::Physics).evaluate(&formula.expression).ok()?;
    let dimension = quantity.dimension();
    (!dimension.is_none()).then_some(dimension)
}

/// The kind of the last operator before the cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operator {
    /// `+`, `−` and the relations: the same dimension follows.
    Same,
    /// `=`: like [`Operator::Same`], and a lone letter before it names a quantity.
    Equals,
    Times,
    Divide,
}

/// The operators that cut the input, as they are written in LaTeX.
const OPERATORS: &[(&str, Operator)] = &[
    ("+", Operator::Same),
    ("-", Operator::Same),
    ("=", Operator::Equals),
    ("<", Operator::Same),
    (">", Operator::Same),
    (r"\le", Operator::Same),
    (r"\ge", Operator::Same),
    (r"\neq", Operator::Same),
    (r"\cdot", Operator::Times),
    (r"\times", Operator::Times),
    ("*", Operator::Times),
    (r"\div", Operator::Divide),
    ("/", Operator::Divide),
    (":", Operator::Divide),
];

/// An operator found in the input: where it starts and ends, and its kind.
type Found = (usize, usize, Operator);

/// The last operator of `before` outside braces and brackets that are closed, with the left
/// side it applies to: the whole term before it, back to a sum, a relation or an open bracket.
fn last_operator(before: &str) -> Option<(Operator, &str)> {
    // The operators at the level of the cursor.
    let mut found: Vec<Found> = Vec::new();
    // The levels around it: where each starts, and its operators.
    let mut starts: Vec<(usize, Vec<Found>)> = Vec::new();
    let mut start = 0;
    let mut braces = 0usize;
    let mut i = 0;
    while i < before.len() {
        let rest = &before[i..];
        let c = rest.chars().next().expect("inside the string");
        match c {
            '{' => braces += 1,
            '}' => braces = braces.saturating_sub(1),
            '(' if braces == 0 => {
                starts.push((start, std::mem::take(&mut found)));
                start = i + 1;
            }
            ')' if braces == 0 => {
                if let Some((outer, operators)) = starts.pop() {
                    start = outer;
                    found = operators;
                }
            }
            _ if braces == 0 => {
                // A command whose name goes on (`\left`, `\leq`) is not the shorter one.
                let hit = OPERATORS.iter().find(|(text, _)| {
                    rest.starts_with(text)
                        && !(text.starts_with('\\') && rest[text.len()..].starts_with(|c: char| c.is_ascii_alphabetic()))
                });
                if let Some((text, operator)) = hit {
                    found.push((i, i + text.len(), *operator));
                    i += text.len();
                    continue;
                }
            }
            _ => {}
        }
        i += c.len_utf8();
    }
    let &(at, _, operator) = found.last()?;
    // The left side is the whole term: back past `·` and `÷` to the last sum or relation.
    let from = found[..found.len() - 1]
        .iter()
        .rev()
        .find(|(_, _, other)| matches!(other, Operator::Same | Operator::Equals))
        .map_or(start, |&(_, end, _)| end);
    Some((operator, before[from..at].trim()))
}

/// A lone letter, perhaps Greek, perhaps with an index: what a quantity is named.
fn lone_letter(left: &str) -> Option<&str> {
    let name = left.split('_').next()?.trim();
    let single = name.chars().count() == 1 && name.chars().all(|c| c.is_alphabetic());
    let greek = name.starts_with('\\') && name[1..].chars().all(|c| c.is_ascii_alphabetic()) && name.len() > 1;
    (single || greek).then_some(name)
}

/// The table's unit whose dimension is the left side's, given its LaTeX.
fn unit_of(left: &str) -> Option<&'static str> {
    let dimension = dimension(left)?;
    let family = family_of(&dimension)?;
    Some(family[0].0)
}

/// The units to offer for the input `before` the cursor; `recent` are the units used last,
/// newest first, by their labels. With `result`, the ▶unit targets of a shown result instead.
pub fn suggest(before: &str, recent: &[String], result: Option<&str>) -> UnitSuggestions {
    let recent: Vec<&str> = recent.iter().map(String::as_str).filter(|label| entry(label).is_some()).collect();
    let (best, more) = match result {
        Some(result) => match dimension(result).as_ref().and_then(family_of) {
            Some(family) => (by_recent(family, &recent), Vec::new()),
            None => (Vec::new(), Vec::new()),
        },
        None => from_input(before, &recent),
    };
    let mut seen: Vec<&'static str> = Vec::new();
    let mut take = |labels: Vec<&'static str>| -> Vec<UnitSuggestion> {
        labels
            .into_iter()
            .filter(|label| {
                let new = !seen.contains(label);
                seen.push(label);
                new
            })
            .filter_map(entry)
            .map(|(label, latex)| UnitSuggestion { latex: latex.to_string(), label: label.to_string() })
            .collect()
    };
    let best = take(best);
    let more = take(more);
    UnitSuggestions { best, more }
}

/// The family's labels, the ones used recently first.
fn by_recent(family: &[Entry], recent: &[&str]) -> Vec<&'static str> {
    let mut labels: Vec<&'static str> = family.iter().map(|(label, _)| *label).collect();
    labels.sort_by_key(|label| recent.iter().position(|r| r == label).unwrap_or(usize::MAX));
    labels
}

/// The rest of the families of `labels`, for the second row.
fn relatives(labels: &[&str], recent: &[&str]) -> Vec<&'static str> {
    labels.iter().filter_map(|label| family_with(label)).flat_map(|family| by_recent(family, recent)).collect()
}

fn from_input(before: &str, recent: &[&str]) -> (Vec<&'static str>, Vec<&'static str>) {
    let defaults = || -> (Vec<&'static str>, Vec<&'static str>) {
        let mut best: Vec<&'static str> = recent.iter().filter_map(|label| entry(label).map(|(l, _)| l)).collect();
        best.extend(DEFAULTS);
        (best, Vec::new())
    };
    let Some((operator, left)) = last_operator(before) else { return defaults() };
    match operator {
        Operator::Same | Operator::Equals => {
            // A lone letter before `=` is a name, though `F` and `V` would read as units.
            let letter = lone_letter(left).filter(|_| operator == Operator::Equals);
            if let Some((_, unit)) = letter.and_then(|letter| LETTERS.iter().find(|(l, _)| *l == letter)) {
                let family = family_with(unit).expect("every letter's unit is in a family");
                let mut best = vec![*unit];
                best.extend(by_recent(family, recent));
                return (best, Vec::new());
            }
            match dimension(left).as_ref().and_then(family_of) {
                Some(family) => (by_recent(family, recent), Vec::new()),
                None => defaults(),
            }
        }
        Operator::Times | Operator::Divide => {
            let table = if operator == Operator::Times { TIMES } else { DIVIDE };
            let Some(unit) = unit_of(left) else { return defaults() };
            let Some((_, partners)) = table.iter().find(|(u, _)| *u == unit) else { return defaults() };
            let best = partners.to_vec();
            let more = relatives(&best, recent);
            (best, more)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_unit_of_the_tables_reads() {
        for (label, latex) in FAMILIES.iter().flat_map(|family| family.iter()) {
            assert!(unit_dimension(latex).is_some(), "{label}: {latex} does not read as a unit");
        }
        for (index, family) in FAMILIES.iter().enumerate() {
            let first = family_dimensions()[index].as_ref();
            for (label, latex) in family.iter() {
                assert_eq!(unit_dimension(latex).as_ref(), first, "{label} is not of its family's dimension");
            }
        }
        let labels = DEFAULTS.iter().chain(TIMES.iter().chain(DIVIDE).flat_map(|(u, p)| std::iter::once(u).chain(p.iter())));
        for label in labels.chain(LETTERS.iter().map(|(_, u)| u)) {
            assert!(entry(label).is_some(), "{label} is in no family");
        }
    }

    #[test]
    fn the_left_side_ends_at_the_operator_before_it() {
        assert_eq!(last_operator(r"3\,\mathrm{kW}+"), Some((Operator::Same, r"3\,\mathrm{kW}")));
        assert_eq!(last_operator(r"2+5N\cdot "), Some((Operator::Times, "5N")));
        assert_eq!(last_operator(r"2\cdot 5N+"), Some((Operator::Same, r"2\cdot 5N")));
        assert_eq!(last_operator(r"(3kW+"), Some((Operator::Same, "3kW")));
        assert_eq!(last_operator(r"1+(3"), None);
        assert_eq!(last_operator(r"\frac{1+2}{3}\cdot "), Some((Operator::Times, r"\frac{1+2}{3}")));
        assert_eq!(last_operator(r"x\leq "), None);
    }
}
