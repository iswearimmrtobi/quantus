//! The hui's texts in German and English. SETUP → Allgemein → Sprache picks one; it also sets
//! the language of the engine's own texts.

/// The language of the texts. German by default: the handheld follows the Casio fx-991DE X.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lang {
    #[default]
    De,
    En,
}

/// A text in both languages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Text {
    pub de: &'static str,
    pub en: &'static str,
}

/// A text in German and English.
pub const fn t(de: &'static str, en: &'static str) -> Text {
    Text { de, en }
}

/// A text that is the same in both languages: a symbol, a name, a unit.
pub const fn same(text: &'static str) -> Text {
    Text { de: text, en: text }
}

impl Text {
    pub fn get(self, lang: Lang) -> &'static str {
        match lang {
            Lang::De => self.de,
            Lang::En => self.en,
        }
    }
}

pub const SETUP: Text = t("Setup", "Setup");
pub const OPTIONS: Text = t("Optionen", "Options");
pub const ON: Text = t("Ein", "On");
pub const OFF: Text = t("Aus", "Off");
pub const YES: Text = t("Ja", "Yes");
pub const NOT_AVAILABLE: Text = t("Noch nicht verfügbar", "Not available yet");
pub const RANGE: Text = t("Bereich", "Range");
pub const TO: Text = t("bis", "to");
pub const EXAM_BLOCKS: Text = t("Aus im Prüfungsmodus", "Off in exam mode");
pub const DONE: Text = t("Erledigt", "Done");
pub const NO_ENTRIES: Text = t("Keine Einträge", "No entries");
pub const EXAM: Text = t("PRÜFUNG", "EXAM");
pub const WLAN: Text = t("WLAN", "Wi-Fi");
pub const EXACT_SHORT: Text = t("Exakt", "Exact");
pub const DECIMAL_SHORT: Text = t("Dez", "Dec");
