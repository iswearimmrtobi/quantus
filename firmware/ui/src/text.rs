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
pub const EXACT_SHORT: Text = t("Exakt", "Exact");
pub const NONE: Text = t("Keins", "None");
pub const NO_PICTURES: Text = t("Keine Bilder auf der SD-Karte", "No pictures on the SD card");
pub const BAD_PICTURE: Text = t("Bild muss 480×320 BMP sein", "Picture must be a 480×320 BMP");
pub const NO_CARD: Text = t("Keine SD-Karte", "No SD card");
pub const SD_CARD: Text = t("SD-Karte", "SD card");
pub const USED: Text = t("Belegt", "Used");
pub const FREE: Text = t("Frei", "Free");
pub const TOTAL: Text = t("Gesamt", "Total");
pub const DECIMAL_SHORT: Text = t("Dez", "Dec");
pub const CALIBRATE: Text = t("Touch kalibrieren", "Calibrate touch");
pub const CALIBRATE_HINT: Text = t("Kreuz berühren und kurz halten", "Touch the cross and hold briefly");
pub const CALIBRATE_RETRY: Text = t("Zu ungenau, bitte noch einmal", "Too far off, once more please");
pub const EMPTY_NAME: Text = t("Der Name darf nicht leer sein", "The name must not be empty");
pub const NAME_HINT: Text = t("Namen tippen, OK übernimmt ihn", "Type a name, OK sets it");
pub const NOT_SET: Text = t("Nicht gestellt", "Not set");
pub const DATE_HINT: Text = t("TT.MM.JJJJ tippen, OK stellt die Uhr", "Type DD.MM.YYYY, OK sets the clock");
pub const CLOCK_HINT: Text = t("HH:MM tippen, OK stellt die Uhr", "Type HH:MM, OK sets the clock");
pub const BAD_DATE: Text = t("Kein gültiges Datum", "Not a valid date");
pub const BAD_CLOCK: Text = t("Keine gültige Uhrzeit", "Not a valid time");
pub const DATE_FIRST: Text = t("Erst das Datum stellen", "Set the date first");
/// Monday first.
pub const WEEKDAYS: [Text; 7] =
    [t("Mo", "Mon"), t("Di", "Tue"), t("Mi", "Wed"), t("Do", "Thu"), t("Fr", "Fri"), t("Sa", "Sat"), t("So", "Sun")];
pub const WLAN_OFF: Text = t("Erst WLAN einschalten", "Switch Wi-Fi on first");
pub const SEARCHING: Text = t("Suche Netzwerke ...", "Looking for networks ...");
pub const NO_NETWORKS: Text = t("Keine Netzwerke gefunden", "No networks found");
pub const NO_SAVED_NETWORKS: Text = t("Keine gespeicherten Netzwerke", "No saved networks");
pub const FORGET: Text = t("Vergessen", "Forget");
pub const PASSWORD_HINT: Text = t("Passwort tippen, S-D zeigt es", "Type the password, S-D shows it");
pub const PASSWORD_LENGTH: Text = t("8 bis 63 Zeichen", "8 to 63 characters");
pub const NETWORK: Text = t("Netzwerk", "Network");
pub const ADDRESS: Text = t("Adresse", "Address");
pub const SIGNAL: Text = t("Signal", "Signal");
pub const WLAN_IS_OFF: Text = t("WLAN ist aus", "Wi-Fi is off");
pub const WLAN_SEARCHING: Text = t("Kein gespeichertes Netzwerk in Reichweite", "No saved network in reach");
pub const CONNECTING: Text = t("Verbinde mit", "Connecting to");
pub const FAILED: Text = t("Keine Verbindung mit", "Could not connect to");
pub const SECTION_PARTIAL: Text = t("teilweise", "partial");
pub const SECTION_FAILED: Text = t("fehlgeschlagen", "failed");
pub const SKETCH_FIRST: Text = t("Erst f(x) eingeben und EXE", "Type f(x) and EXE first");
pub const GRAPH: Text = t("Graph", "Graph");
pub const WORKING: Text = t("Rechenweg", "Working");
pub const NO_UNKNOWN: Text = t("Keine Variable zum Auflösen", "No letter to solve for");
pub const SOLVE_HINT: Text = t("Gesucht: das leere Feld", "Solved for: the empty field");
pub const STO: Text = same("STO");
pub const VARS: Text = same("VARS");
pub const NO_VARIABLES: Text = t("Keine Variablen", "No variables");
pub const STORE_HINT: Text = t("Namen tippen (A, r_1), OK speichert", "Type a name (A, r_1), OK stores");
pub const BAD_VARIABLE: Text = t("Ein Buchstabe, dann Buchstaben, Ziffern, _", "A letter, then letters, digits, _");
pub const NO_RESULT: Text = t("Erst etwas ausrechnen", "Calculate something first");
pub const STORED: Text = t("Gespeichert", "Stored");
pub const PHYSICS_ONLY: Text = t("Nur im Modus Physik", "In Physik mode only");
pub const MOVING: Text = t("Bewegen", "Move");
