//! The language of the texts the electronics module writes itself. The solver builds its
//! messages deep inside, so the language is set once per call instead of being passed along.

use std::cell::Cell;

use crate::settings::Language;

thread_local! {
    static LANGUAGE: Cell<Language> = const { Cell::new(Language::En) };
}

/// The language of the call in progress; English outside of one.
pub fn language() -> Language {
    LANGUAGE.with(Cell::get)
}

/// Keeps `language` current until it is dropped.
pub struct Scope(Language);

impl Scope {
    pub fn new(language: Language) -> Self {
        Scope(LANGUAGE.with(|current| current.replace(language)))
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        LANGUAGE.with(|current| current.set(self.0));
    }
}

/// `format!` in the current language: the English text first, then the German one, both with
/// the same arguments.
macro_rules! msg {
    ($en:literal, $de:literal $(, $arg:expr)* $(,)?) => {
        match $crate::electro::text::language() {
            $crate::settings::Language::En => format!($en $(, $arg)*),
            $crate::settings::Language::De => format!($de $(, $arg)*),
        }
    };
}
pub(crate) use msg;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scope_sets_and_restores_the_language() {
        assert_eq!(msg!("one {}", "eins {}", 1), "one 1");
        {
            let _german = Scope::new(Language::De);
            assert_eq!(msg!("one {}", "eins {}", 1), "eins 1");
        }
        assert_eq!(language(), Language::En);
    }
}
