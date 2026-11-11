//! How a typed query matches an entry of a search list, shared by the engine's library search
//! and the handheld's CATALOG, so both rank the same way.
//!
//! An entry matches when every word of the query matches a word of one of its names or its
//! symbol. [`rank`] says how well: an exact symbol, then a name that starts with the query, word
//! prefixes, substrings, and last word prefixes with a typo or two. Lists sort by the [`Tier`],
//! then by the shorter name.

/// How well an entry matches; smaller is better.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
    Symbol,
    SymbolIgnoringCase,
    SymbolBase,
    NamePrefix,
    WordPrefix,
    Substring,
    Fuzzy,
}

/// An entry with everything the search compares, already normalised.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Candidate {
    /// The names, normalised, each once.
    pub names: Vec<String>,
    /// The words of all names.
    pub words: Vec<String>,
    /// The symbols as typed (`c0`, `km/h`), case kept.
    pub symbols: Vec<String>,
}

impl Candidate {
    /// An entry with these names (the set language first, then English and aliases; doubles are
    /// dropped) and symbols.
    pub fn new<'a>(names: impl IntoIterator<Item = &'a str>, symbols: Vec<String>) -> Self {
        let mut normalised: Vec<String> = Vec::new();
        for name in names {
            let name = normalise(name);
            if !name.is_empty() && !normalised.contains(&name) {
                normalised.push(name);
            }
        }
        let words = normalised.iter().flat_map(|name| split_words(name)).collect();
        Candidate { names: normalised, words, symbols }
    }
}

/// A query, prepared once for a whole list.
#[derive(Clone, Debug)]
pub struct Query {
    /// Without spaces, case kept, to compare with symbols.
    plain: String,
    /// Normalised.
    needle: String,
    words: Vec<String>,
}

impl Query {
    pub fn new(raw: &str) -> Self {
        let raw = raw.trim();
        let needle = normalise(raw);
        let words = split_words(&needle);
        Query { plain: raw.replace(' ', ""), needle, words }
    }

    /// Whether nothing was typed.
    pub fn is_empty(&self) -> bool {
        self.plain.is_empty()
    }
}

/// How well `entry` matches `query`, or `None`; an empty query matches nothing.
pub fn rank(entry: &Candidate, query: &Query) -> Option<Tier> {
    if query.is_empty() {
        return None;
    }
    let plain = query.plain.as_str();
    if entry.symbols.iter().any(|symbol| symbol == plain) {
        return Some(Tier::Symbol);
    }
    let lower = plain.to_lowercase();
    if entry.symbols.iter().any(|symbol| symbol.to_lowercase() == lower) {
        return Some(Tier::SymbolIgnoringCase);
    }
    if entry.symbols.iter().any(|symbol| symbol.split('_').next() == Some(plain)) {
        return Some(Tier::SymbolBase);
    }
    // Only signs (`√`, `°`) match only a symbol.
    if query.words.is_empty() {
        return None;
    }
    if entry.names.iter().any(|name| name.starts_with(&query.needle)) {
        return Some(Tier::NamePrefix);
    }
    // Each word of the query on its own; the entry is as good as its worst word.
    query
        .words
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
pub fn fuzzy_prefix(word: &str, candidate: &str) -> bool {
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
pub fn normalise(text: &str) -> String {
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
pub fn split_words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_string).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_typo_or_two_is_forgiven() {
        assert!(fuzzy_prefix("steff", "stefan"));
        assert!(fuzzy_prefix("boltzman", "boltzmann"));
        assert!(fuzzy_prefix("avogardo", "avogadro"));
        assert!(!fuzzy_prefix("abc", "abd"));
        assert!(!fuzzy_prefix("steff", "planck"));
    }

    #[test]
    fn names_rank_by_how_they_match() {
        let root = Candidate::new(["Quadratwurzel", "Square root", "sqrt"], vec!["√".into()]);
        assert_eq!(rank(&root, &Query::new("√")), Some(Tier::Symbol));
        assert_eq!(rank(&root, &Query::new("quadr")), Some(Tier::NamePrefix));
        assert_eq!(rank(&root, &Query::new("root")), Some(Tier::WordPrefix));
        assert_eq!(rank(&root, &Query::new("wurz")), Some(Tier::Substring));
        assert_eq!(rank(&root, &Query::new("sqaure")), Some(Tier::Fuzzy));
        assert_eq!(rank(&root, &Query::new("sinus")), None);
        assert_eq!(rank(&root, &Query::new("  ")), None);
    }

    #[test]
    fn accents_and_sharp_s_are_ignored() {
        assert_eq!(normalise("Größe Ä"), "grosse a");
        let size = Candidate::new(["Schriftgröße"], Vec::new());
        assert_eq!(rank(&size, &Query::new("schriftgrosse")), Some(Tier::NamePrefix));
    }
}
