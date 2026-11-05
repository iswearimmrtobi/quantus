//! One search over the constants and the conversion units, for a quick-search line: `steff`
//! finds the Stefan–Boltzmann constant, `zoll` the inch, `c` the speed of light.
//!
//! An entry matches when every word of the query matches a word of its name (in the set
//! language or in English) or its symbol. The hits are ranked by how well they match: an exact
//! symbol, then a name that starts with the query, word prefixes, substrings, and last word
//! prefixes with a typo or two.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::settings::Language;

/// What a hit is; a unit goes in after `\blacktriangleright`, a constant as it is.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum HitKind {
    Constant,
    Unit,
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
    /// The value with unit for a constant, the category for a unit.
    pub detail: String,
}

/// An entry with everything the search compares, already normalised.
struct Entry {
    hit: LibraryHit,
    /// The name in the set language and, if different, in English.
    names: Vec<String>,
    /// The words of all names.
    words: Vec<String>,
    /// The symbol as typed (`c0`, `km/h`), case kept.
    symbols: Vec<String>,
}

/// How well an entry matches; smaller is better.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Tier {
    Symbol,
    SymbolIgnoringCase,
    SymbolBase,
    NamePrefix,
    WordPrefix,
    Substring,
    Fuzzy,
}

/// The best `limit` hits for `query`, best first; an empty query finds nothing.
pub fn search(query: &str, language: Language, limit: usize) -> Vec<LibraryHit> {
    let raw = query.trim();
    let needle = normalise(raw);
    let words = split_words(&needle);
    if words.is_empty() {
        return Vec::new();
    }
    let mut ranked: Vec<(Tier, usize, usize, &Entry)> = entries(language)
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| rank(entry, raw, &needle, &words).map(|tier| (tier, entry.hit.name.chars().count(), index, entry)))
        .collect();
    ranked.sort_by_key(|(tier, length, index, _)| (*tier, *length, *index));
    ranked.into_iter().take(limit).map(|(.., entry)| entry.hit.clone()).collect()
}

fn rank(entry: &Entry, raw: &str, needle: &str, words: &[String]) -> Option<Tier> {
    let plain = raw.replace(' ', "");
    if entry.symbols.contains(&plain) {
        return Some(Tier::Symbol);
    }
    if entry.symbols.iter().any(|symbol| symbol.to_lowercase() == plain.to_lowercase()) {
        return Some(Tier::SymbolIgnoringCase);
    }
    if entry.symbols.iter().any(|symbol| symbol.split('_').next() == Some(plain.as_str())) {
        return Some(Tier::SymbolBase);
    }
    if entry.names.iter().any(|name| name.starts_with(needle)) {
        return Some(Tier::NamePrefix);
    }
    // Each word of the query on its own; the entry is as good as its worst word.
    words
        .iter()
        .map(|word| {
            if entry.words.iter().any(|w| w.starts_with(word.as_str())) {
                Some(Tier::WordPrefix)
            } else if entry.names.iter().any(|name| name.contains(word.as_str()))
                || entry.symbols.iter().any(|symbol| symbol.to_lowercase().contains(word.as_str()))
            {
                Some(Tier::Substring)
            } else if entry.words.iter().any(|w| fuzzy_prefix(word, w)) {
                Some(Tier::Fuzzy)
            } else {
                None
            }
        })
        .try_fold(Tier::WordPrefix, |worst, tier| tier.map(|tier| worst.max(tier)))
}

/// Whether `word` is a prefix of `candidate` with a few typos: one for four to six letters,
/// two from seven on, none for shorter words. Prefixes one letter shorter or longer count, so a
/// letter too many or too few is a single typo.
fn fuzzy_prefix(word: &str, candidate: &str) -> bool {
    let word: Vec<char> = word.chars().collect();
    let allowed = match word.len() {
        0..=3 => return false,
        4..=6 => 1,
        _ => 2,
    };
    let candidate: Vec<char> = candidate.chars().collect();
    (word.len() - 1..=word.len() + 1)
        .filter(|&length| length <= candidate.len())
        .any(|length| distance(&word, &candidate[..length]) <= allowed)
}

/// The optimal string alignment distance: insertions, deletions, substitutions and swaps of
/// two neighbouring letters.
fn distance(a: &[char], b: &[char]) -> usize {
    let mut rows = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in rows.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in rows[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut best = (rows[i - 1][j] + 1).min(rows[i][j - 1] + 1).min(rows[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                best = best.min(rows[i - 2][j - 2] + 1);
            }
            rows[i][j] = best;
        }
    }
    rows[a.len()][b.len()]
}

/// Lower case, without accents, `ß` as `ss`.
fn normalise(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars().flat_map(char::to_lowercase) {
        match c {
            'ä' | 'á' | 'à' | 'â' | 'å' => out.push('a'),
            'ö' | 'ó' | 'ò' | 'ô' => out.push('o'),
            'ü' | 'ú' | 'ù' | 'û' => out.push('u'),
            'é' | 'è' | 'ê' | 'ë' => out.push('e'),
            'í' | 'ì' | 'î' | 'ï' => out.push('i'),
            'ß' => out.push_str("ss"),
            _ => out.push(c),
        }
    }
    out
}

/// Words are split at everything that is not a letter or digit, dashes included.
fn split_words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_string).collect()
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

fn entry(hit: LibraryHit, english: &str, symbols: Vec<String>) -> Entry {
    let mut names = vec![normalise(&hit.name)];
    let english = normalise(english);
    if english != names[0] {
        names.push(english);
    }
    let words = names.iter().flat_map(|name| split_words(name)).collect();
    Entry { hit, names, words, symbols }
}

fn build(language: Language) -> Vec<Entry> {
    let mut entries = Vec::new();
    let english = crate::constants::catalogue(Language::En);
    for (category, english) in crate::constants::catalogue(language).into_iter().zip(english) {
        for (constant, english) in category.constants.into_iter().zip(english.constants) {
            let symbols = vec![constant.key.trim_start_matches('\\').to_string(), typed_symbol(&constant.latex)];
            let hit = LibraryHit { kind: HitKind::Constant, latex: constant.latex, name: constant.name, detail: constant.value };
            entries.push(entry(hit, &english.name, symbols));
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
        entries.push(entry(hit, unit.name.get(Language::En), vec![typed_symbol(unit.latex)]));
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

    #[test]
    fn a_typo_or_two_is_forgiven() {
        assert!(fuzzy_prefix("steff", "stefan"));
        assert!(fuzzy_prefix("boltzman", "boltzmann"));
        assert!(fuzzy_prefix("avogardo", "avogadro"));
        assert!(!fuzzy_prefix("abc", "abd"));
        assert!(!fuzzy_prefix("steff", "planck"));
    }
}
