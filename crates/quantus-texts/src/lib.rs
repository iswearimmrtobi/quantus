//! Every text of Quantus in every language. They live in `locales/`, one file per language
//! (`en.json`, `de.json`), which the macros read while compiling: the engine, the desktop shell
//! and the handheld get string literals and `format!` calls, the desktop frontend imports the
//! same files.
//!
//! - [`tr!`]`(language, "key", arguments…)`: a text through `format!`.
//! - [`texts!`]`("key")`: a text in every language, for a [`Text`] in a `const`.
//!
//! A text missing in a language falls back to English; `tests/texts.rs` lists every gap.

// The macros name `::quantus_texts`, also in this crate's own tests.
extern crate self as quantus_texts;

pub use quantus_texts_macros::{texts, tr};

quantus_texts_macros::language_enum!();

impl Language {
    /// Its code, the name of its file in `locales/` and what the settings store: `en`, `de`.
    pub const fn code(self) -> &'static str {
        Self::CODES[self as usize]
    }

    pub fn from_code(code: &str) -> Option<Language> {
        Language::ALL.into_iter().find(|l| l.code() == code)
    }
}

/// A text in every language, from [`texts!`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Text(pub [&'static str; Language::ALL.len()]);

impl Text {
    /// The same in every language: a symbol, a name, a unit.
    pub const fn same(text: &'static str) -> Text {
        Text([text; Language::ALL.len()])
    }

    pub const fn get(self, language: Language) -> &'static str {
        self.0[language as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texts_by_language() {
        assert_eq!(tr!(Language::En, "test.one", 1), "one 1");
        assert_eq!(tr!(Language::De, "test.one", 1), "eins 1");
        let (n, total) = (2, 3);
        assert_eq!(tr!(Language::De, "test.named"), "2 von 3");
        assert_eq!(tr!(Language::En, "test.named", total = 4), "2 of 4");
        const ONE: Text = Text(texts!("test.one"));
        assert_eq!(ONE.get(Language::De), "eins {}");
        assert_eq!(Language::from_code("de"), Some(Language::De));
        assert_eq!((Language::De.name(), Language::De.decimal(), Language::default()), ("Deutsch", ",", Language::En));
    }
}
