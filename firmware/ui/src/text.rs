//! The hui's texts in German and English. SETUP → Allgemein → Sprache picks one; it also sets
//! the language of the engine's own texts.

/// The language of the texts, one of `texts/texts.json`. The hui's settings start in German:
/// the handheld follows the Casio fx-991DE X.
pub use quantus_texts::Language as Lang;
/// A text in every language.
pub use quantus_texts::Text;

/// The text `key` of `texts/texts.json`, a [`Text`] usable in a `const`.
macro_rules! tx {
    ($key:tt) => {
        $crate::text::Text(::quantus_texts::texts!($key))
    };
}
pub(crate) use tx;

/// A text that is the same in every language: a symbol, a name, a unit.
pub const fn same(text: &'static str) -> Text {
    Text::same(text)
}

pub const SETUP: Text = tx!("hui.setup");
pub const OPTIONS: Text = tx!("hui.options");
pub const ON: Text = tx!("hui.on");
pub const OFF: Text = tx!("hui.off");
pub const YES: Text = tx!("hui.yes");
pub const NOT_AVAILABLE: Text = tx!("hui.not_available");
pub const RANGE: Text = tx!("hui.range");
pub const TO: Text = tx!("hui.to");
pub const EXAM_BLOCKS: Text = tx!("hui.exam_blocks");
pub const DONE: Text = tx!("hui.done");
pub const NO_ENTRIES: Text = tx!("hui.no_entries");
pub const EXAM: Text = tx!("hui.exam");
pub const EXACT_SHORT: Text = tx!("hui.exact_short");
pub const NONE: Text = tx!("hui.none");
pub const NO_PICTURES: Text = tx!("hui.no_pictures");
pub const BAD_PICTURE: Text = tx!("hui.bad_picture");
pub const NO_CARD: Text = tx!("hui.no_card");
pub const SD_CARD: Text = tx!("hui.sd_card");
pub const USED: Text = tx!("hui.used");
pub const FREE: Text = tx!("hui.free");
pub const TOTAL: Text = tx!("hui.total");
pub const DECIMAL_SHORT: Text = tx!("hui.decimal_short");
pub const SIGNIFICANT_SHORT: Text = tx!("hui.significant_short");
pub const CALIBRATE: Text = tx!("hui.calibrate");
pub const CALIBRATE_HINT: Text = tx!("hui.calibrate_hint");
pub const CALIBRATE_RETRY: Text = tx!("hui.calibrate_retry");
pub const EMPTY_NAME: Text = tx!("hui.empty_name");
pub const NAME_HINT: Text = tx!("hui.name_hint");
pub const NOT_SET: Text = tx!("hui.not_set");
pub const DATE_HINT: Text = tx!("hui.date_hint");
pub const CLOCK_HINT: Text = tx!("hui.clock_hint");
pub const BAD_DATE: Text = tx!("hui.bad_date");
pub const BAD_CLOCK: Text = tx!("hui.bad_clock");
pub const DATE_FIRST: Text = tx!("hui.date_first");
/// Monday first.
pub const WEEKDAYS: [Text; 7] =
    [tx!("hui.weekday.mon"), tx!("hui.weekday.tue"), tx!("hui.weekday.wed"), tx!("hui.weekday.thu"), tx!("hui.weekday.fri"), tx!("hui.weekday.sat"), tx!("hui.weekday.sun")];
pub const WLAN_OFF: Text = tx!("hui.wlan_off");
pub const SEARCHING: Text = tx!("hui.searching");
pub const NO_NETWORKS: Text = tx!("hui.no_networks");
pub const NO_SAVED_NETWORKS: Text = tx!("hui.no_saved_networks");
pub const FORGET: Text = tx!("hui.forget");
pub const PASSWORD_HINT: Text = tx!("hui.password_hint");
pub const PASSWORD_LENGTH: Text = tx!("hui.password_length");
pub const NETWORK: Text = tx!("hui.network");
pub const ADDRESS: Text = tx!("hui.address");
pub const SIGNAL: Text = tx!("hui.signal");
pub const WLAN_IS_OFF: Text = tx!("hui.wlan_is_off");
pub const WLAN_SEARCHING: Text = tx!("hui.wlan_searching");
pub const CONNECTING: Text = tx!("hui.connecting");
pub const FAILED: Text = tx!("hui.failed");
pub const SECTION_PARTIAL: Text = tx!("hui.section_partial");
pub const SECTION_FAILED: Text = tx!("hui.section_failed");
pub const SKETCH_FIRST: Text = tx!("hui.sketch_first");
pub const GRAPH: Text = tx!("hui.graph");
pub const WORKING: Text = tx!("hui.working");
pub const NO_UNKNOWN: Text = tx!("hui.no_unknown");
pub const SOLVE_HINT: Text = tx!("hui.solve_hint");
pub const STO: Text = same("STO");
pub const VARS: Text = same("VARS");
pub const MATRIX: Text = tx!("hui.matrix");
pub const MATRIX_HINT: Text = tx!("hui.matrix_hint");
pub const MATRIX_NAME: Text = tx!("hui.matrix_name");
pub const NO_VARIABLES: Text = tx!("hui.no_variables");
pub const STORE_HINT: Text = tx!("hui.store_hint");
pub const BAD_VARIABLE: Text = tx!("hui.bad_variable");
pub const NO_RESULT: Text = tx!("hui.no_result");
pub const STORED: Text = tx!("hui.stored");
pub const PHYSICS_ONLY: Text = tx!("hui.physics_only");
pub const MOVING: Text = tx!("hui.moving");
