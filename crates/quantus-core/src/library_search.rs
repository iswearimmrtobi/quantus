//! One search over the constants, the conversion units, the functions and the metric prefixes,
//! for a quick-search line: `steff` finds the Stefan–Boltzmann constant, `zoll` the inch, `c` the
//! speed of light, `wurz` the square root, `kilo` the prefix k.
//!
//! An entry matches when every word of the query matches a word of its name (in the set
//! language or in English) or its symbol. The hits are ranked by how well they match: an exact
//! symbol, then a name that starts with the query, word prefixes, substrings, and last word
//! prefixes with a typo or two.

use std::sync::OnceLock;

use quantus_search::{rank, Candidate, Query, Tier};

use serde::{Deserialize, Serialize};

use crate::settings::Language;

/// What a hit is; a unit goes in after `\blacktriangleright`, the others as they are.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum HitKind {
    Constant,
    Unit,
    /// A function or operation; its LaTeX has empty slots (`\sin()`, `\binom{}{}`).
    Function,
    /// A metric prefix, to go before a unit.
    Prefix,
}

/// A hit of the search, ready for a list.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryHit {
    pub kind: HitKind,
    /// The LaTeX to insert; also the symbol to show.
    pub latex: String,
    /// The name in the set language.
    pub name: String,
    /// The value with unit for a constant (LaTeX), the category for a unit or function, the
    /// power of ten for a prefix (LaTeX).
    pub detail: String,
}

/// An entry of the index with what it is found by, for a shell that ranks it together with
/// entries of its own (the handheld's menus) through `quantus_search`.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryEntry {
    pub hit: LibraryHit,
    /// The name in the set language, in English and the aliases, as written.
    pub names: Vec<String>,
    /// The symbols as typed (`c0`, `km/h`, `sqrt`).
    pub symbols: Vec<String>,
}

/// An entry with the hit and what the search compares.
struct Entry {
    entry: LibraryEntry,
    candidate: Candidate,
}

/// Every entry, in the order ties are broken: constants, units, functions, prefixes.
pub fn index(language: Language) -> Vec<LibraryEntry> {
    entries(language).iter().map(|entry| entry.entry.clone()).collect()
}

/// The best `limit` hits for `query`, best first; an empty query finds nothing.
pub fn search(query: &str, language: Language, limit: usize) -> Vec<LibraryHit> {
    let query = Query::new(query);
    if query.is_empty() {
        return Vec::new();
    }
    let mut ranked: Vec<(Tier, usize, usize, &Entry)> = entries(language)
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| rank(&entry.candidate, &query).map(|tier| (tier, entry.entry.hit.name.chars().count(), index, entry)))
        .collect();
    ranked.sort_by_key(|(tier, length, index, _)| (*tier, *length, *index));
    ranked.into_iter().take(limit).map(|(.., entry)| entry.entry.hit.clone()).collect()
}

/// A symbol as one would type it: `\frac{\mathrm{km}}{\mathrm{h}}` is `km/h`, `h_{P}` is
/// `h_P`, `\varepsilon_{0}` is `varepsilon_0`, `^\circ\mathrm{C}` is `°C`.
fn typed_symbol(latex: &str) -> String {
    let mut text = latex.replace(r"^\circ", "°").replace(r"\cdot", "*");
    if let Some(rest) = text.strip_prefix(r"\frac") {
        // `\frac{a}{b}` with a and b free of further fractions.
        if let Some((numerator, denominator)) = rest.split_once("}{") {
            text = format!("{}/{}", &numerator[1..], &denominator[..denominator.len() - 1]);
        }
    }
    let text = text.replace(r"\mathrm", "").replace(['\\', '{', '}', ' '], "");
    text.replace('^', "")
}

fn entries(language: Language) -> &'static [Entry] {
    static ENTRIES: [OnceLock<Vec<Entry>>; Language::ALL.len()] = [const { OnceLock::new() }; Language::ALL.len()];
    ENTRIES[language as usize].get_or_init(|| build(language))
}

fn entry(hit: LibraryHit, english: &str, aliases: &[&str], symbols: Vec<String>) -> Entry {
    let mut names = vec![hit.name.clone()];
    for name in std::iter::once(&english).chain(aliases) {
        if !names.iter().any(|known| known == name) {
            names.push(name.to_string());
        }
    }
    let candidate = Candidate::new(names.iter().map(String::as_str), symbols.clone());
    Entry { entry: LibraryEntry { hit, names, symbols }, candidate }
}

fn build(language: Language) -> Vec<Entry> {
    let mut entries = Vec::new();
    let english = crate::constants::catalogue(Language::En);
    for (category, english) in crate::constants::catalogue(language).into_iter().zip(english) {
        for (constant, english) in category.constants.into_iter().zip(english.constants) {
            let symbols = vec![constant.key.trim_start_matches('\\').to_string(), typed_symbol(&constant.latex)];
            let hit = LibraryHit { kind: HitKind::Constant, latex: constant.latex, name: constant.name, detail: constant.value };
            entries.push(entry(hit, &english.name, &[], symbols));
        }
    }
    let mut seen = std::collections::HashSet::new();
    for unit in crate::units::catalogue::entries() {
        // A unit in two categories is listed once, under the first.
        if !seen.insert(unit.latex) {
            continue;
        }
        let hit = LibraryHit {
            kind: HitKind::Unit,
            latex: unit.latex.to_string(),
            name: unit.name.get(language).to_string(),
            detail: unit.category.get(language).to_string(),
        };
        entries.push(entry(hit, unit.name.get(Language::En), &[], vec![typed_symbol(unit.latex)]));
    }
    for function in crate::functions::FUNCTIONS {
        let hit = LibraryHit {
            kind: HitKind::Function,
            latex: function.latex.to_string(),
            name: function.name.get(language).to_string(),
            detail: function.category.get(language).to_string(),
        };
        let symbols = function.symbols.iter().map(|symbol| symbol.to_string()).collect();
        entries.push(entry(hit, function.name.get(Language::En), function.aliases, symbols));
    }
    // Without symbols: `c` is the speed of light, not centi; `kilo` and `k` find it by name.
    for prefix in &crate::functions::PREFIXES {
        let hit = LibraryHit {
            kind: HitKind::Prefix,
            latex: prefix.latex.to_string(),
            name: prefix.name.get(language).to_string(),
            detail: format!("10^{{{}}}", prefix.exponent),
        };
        entries.push(entry(hit, prefix.name.get(Language::En), &[], Vec::new()));
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbols_read_as_typed() {
        assert_eq!(typed_symbol(r"\frac{\mathrm{km}}{\mathrm{h}}"), "km/h");
        assert_eq!(typed_symbol(r"\mathrm{m}^{2}"), "m2");
        assert_eq!(typed_symbol(r"^\circ\mathrm{C}"), "°C");
        assert_eq!(typed_symbol(r"h_{P}"), "h_P");
        assert_eq!(typed_symbol(r"\mathrm{N} \cdot \mathrm{m}"), "N*m");
    }
}
