//! The menu tree of design/handheld-menus.md §7: SETUP (§7.2), OPTN per mode (§7.3) and the
//! Sitzungen tile (§6). Which entries work and which
//! wait for the engine or the firmware is noted there.

use crate::app::Mode;
use crate::backend::Catalogue;
use crate::menu::{opt, Action, Entry, Info, Kind, Opt, Page, TimeField, ON_OFF};
use crate::settings::Setting as S;
use crate::text::{same, t, Text};

const fn entry(label: Text, kind: Kind) -> Entry {
    Entry { label, kind }
}

const fn switch(label: Text, setting: S) -> Entry {
    entry(label, Kind::Choice(setting, &ON_OFF))
}

const fn pending(label: Text) -> Entry {
    entry(label, Kind::Pending)
}

const AUTO: Text = same("Auto");
const EXPANDED: Text = t("Ausmultipliziert", "Expanded");

pub static SETUP: Page = Page {
    title: crate::text::SETUP,
    entries: &[
        entry(t("Allgemein", "General"), Kind::Page(&GENERAL)),
        entry(t("Ergebnis", "Results"), Kind::Page(&RESULTS)),
        entry(t("Rechenweg", "Working"), Kind::Page(&WORKING)),
        entry(same("Algebra"), Kind::Page(&ALGEBRA)),
        entry(t("Eingabe", "Input"), Kind::Page(&INPUT)),
        entry(t("Gerät", "Device"), Kind::Page(&DEVICE)),
        entry(t("Verbindungen", "Connections"), Kind::Page(&CONNECTIONS)),
    ],
};

static GENERAL: Page = Page {
    title: t("Allgemein", "General"),
    entries: &[
        entry(
            t("Sprache", "Language"),
            Kind::Choice(S::Language, &[opt(same("Deutsch"), 0), opt(same("English"), 1)]),
        ),
        entry(
            t("Winkeleinheit", "Angle unit"),
            Kind::Choice(S::Angle, &[opt(t("Grad", "Degrees"), 0), opt(t("Bogenmaß", "Radians"), 1)]),
        ),
        entry(t("Signifikante Stellen", "Significant digits"), Kind::Number(S::Digits, 1, 15)),
        entry(t("Zahlenformat", "Number format"), Kind::Choice(S::Format, &FORMATS)),
        switch(t("Dezimalpräfixe", "Decimal prefixes"), S::Prefixes),
        switch(t("1000er-Trennung", "Digit grouping"), S::Grouping),
    ],
};

static FORMATS: [Opt; 4] = [
    opt(same("Norm 1"), 0),
    opt(same("Norm 2"), 1),
    Opt { text: same("Fix"), value: 2, then: Some((S::FormatDigits, 0, 9)) },
    Opt { text: same("Sci"), value: 3, then: Some((S::FormatDigits, 0, 9)) },
];

static RESULTS: Page = Page {
    title: t("Ergebnis", "Results"),
    entries: &[
        entry(
            t("Exakt oder dezimal", "Exact or decimal"),
            Kind::Choice(S::Decimal, &[opt(t("Exakt", "Exact"), 0), opt(t("Dezimal", "Decimal"), 1)]),
        ),
        entry(
            t("Brüche", "Fractions"),
            Kind::Choice(S::Fraction, &[opt(same("d/c"), 0), opt(same("ab/c"), 1)]),
        ),
        switch(t("Periodische Dezimalzahlen", "Recurring decimals"), S::Recurring),
        entry(
            t("Komplexe Zahlen", "Complex numbers"),
            Kind::Choice(S::Complex, &[opt(same("a+bi"), 0), opt(same("Polar (r, θ)"), 1)]),
        ),
        entry(
            t("Quadratische Gleichungen", "Quadratic equations"),
            Kind::Choice(S::QuadraticFormula, &[opt(AUTO, 0), opt(same("pq"), 1), opt(same("abc"), 2)]),
        ),
    ],
};

static WORKING: Page = Page {
    title: t("Rechenweg", "Working"),
    entries: &[
        entry(
            t("Rechenweg", "Working"),
            Kind::Choice(
                S::StepDetail,
                &[
                    opt(t("Keiner", "None"), 0),
                    opt(t("Kurz", "Brief"), 1),
                    opt(same("Standard"), 2),
                    opt(t("Ausführlich", "Detailed"), 3),
                    opt(same("Maximal"), 4),
                ],
            ),
        ),
        switch(t("Kürzungen durchstreichen", "Strike out cancelling"), S::CancelMarks),
        entry(
            t("Gleichungssysteme", "Linear systems"),
            Kind::Choice(S::Elimination, &[opt(same("Gauß"), 0), opt(same("Gauß-Jordan"), 1)]),
        ),
    ],
};

static ALGEBRA: Page = Page {
    title: same("Algebra"),
    entries: &[
        entry(
            t("Binomformeln", "Binomial formulas"),
            Kind::Choice(S::BinomialForm, &[opt(AUTO, 0), opt(EXPANDED, 1), opt(t("Binom", "Binomial"), 2)]),
        ),
        switch(t("Quadratische Ergänzung", "Completing the square"), S::CompleteSquare),
        entry(
            t("Nenner", "Denominators"),
            Kind::Choice(S::DenominatorForm, &[opt(AUTO, 0), opt(EXPANDED, 1), opt(t("Faktorisiert", "Factored"), 2)]),
        ),
        entry(
            t("Identitäten", "Identities"),
            Kind::Choice(
                S::ConditionalIdentities,
                &[opt(t("Mit Bedingung", "With condition"), 0), opt(t("Nur immer gültige", "Always valid"), 1)],
            ),
        ),
    ],
};

static INPUT: Page = Page {
    title: t("Eingabe", "Input"),
    entries: &[
        switch(t("a und y als Variablen", "a and y as variables"), S::AAndYAsVariables),
        switch(t("Buchstaben als Produkt", "Letters as a product"), S::LetterProducts),
        switch(t("t, u, in als Einheiten", "t, u, in as units"), S::ExtraUnitLetters),
        entry(
            t("Unbekannte (LGS)", "Unknowns (systems)"),
            Kind::Choice(S::UnknownNames, &[opt(t("Indiziert (x_1)", "Indexed (x_1)"), 0), opt(t("Buchstaben (x, y)", "Letters (x, y)"), 1)]),
        ),
    ],
};

static DEVICE: Page = Page {
    title: t("Gerät", "Device"),
    entries: &[
        entry(t("Helligkeit", "Brightness"), Kind::Number(S::Brightness, 1, 10)),
        entry(
            t("Schriftgröße", "Font size"),
            Kind::Choice(S::FontSize, &[opt(same("Normal"), 0), opt(t("Klein", "Small"), 1)]),
        ),
        entry(t("Hintergrundbild", "Wallpaper"), Kind::Page(&WALLPAPER)),
        entry(t("Speicherplatz", "Storage"), Kind::Storage),
        entry(
            t("Automatisch aus", "Auto power off"),
            Kind::Choice(S::AutoOff, &[opt(same("10 min"), 10), opt(same("60 min"), 60)]),
        ),
        entry(t("Zurücksetzen", "Reset"), Kind::Page(&RESET)),
        entry(t("Touch kalibrieren", "Calibrate touch"), Kind::Do(Action::Calibrate)),
    ],
};

/// A picture from the SD card behind the screens (design/handheld-menus.md §7.6).
static WALLPAPER: Page = Page {
    title: t("Hintergrundbild", "Wallpaper"),
    entries: &[
        entry(t("Bild", "Picture"), Kind::Wallpapers),
        entry(
            t("Anzeigen", "Show on"),
            Kind::Choice(
                S::WallpaperOn,
                &[opt(t("Hauptmenü", "Main menu"), 0), opt(t("Menüs", "Menus"), 1), opt(t("Überall", "Everywhere"), 2)],
            ),
        ),
    ],
};

static RESET: Page = Page {
    title: t("Zurücksetzen", "Reset"),
    entries: &[
        entry(t("Einstellungen", "Settings"), Kind::Confirm(Action::ResetSettings)),
        entry(t("Speicher", "Memory"), Kind::Confirm(Action::ResetMemory)),
        entry(t("Alles", "Everything"), Kind::Confirm(Action::ResetAll)),
    ],
};

static CONNECTIONS: Page = Page {
    title: t("Verbindungen", "Connections"),
    entries: &[
        switch(t("Prüfungsmodus", "Exam mode"), S::Exam),
        entry(t("WLAN", "Wi-Fi"), Kind::Page(&WLAN)),
        entry(t("Rechnen im Netz", "Remote engine"), Kind::Page(&REMOTE)),
        entry(t("Sitzungen abgleichen", "Sync sessions"), Kind::Page(&SYNC)),
        entry(t("Uhrzeit", "Clock"), Kind::Page(&CLOCK)),
        entry(same("Firmware-Update"), Kind::Page(&FIRMWARE)),
        entry(same("Bluetooth"), Kind::Page(&BLUETOOTH)),
    ],
};

static WLAN: Page = Page {
    title: t("WLAN", "Wi-Fi"),
    entries: &[
        switch(t("WLAN", "Wi-Fi"), S::Wlan),
        entry(t("Netzwerke", "Networks"), Kind::Networks),
        entry(t("Gespeicherte Netzwerke", "Saved networks"), Kind::SavedNetworks),
        entry(same("Status"), Kind::WlanStatus),
    ],
};

static REMOTE: Page = Page {
    title: t("Rechnen im Netz", "Remote engine"),
    entries: &[
        entry(
            t("Rechnen im Netz", "Remote engine"),
            Kind::Choice(S::Remote, &[opt(crate::text::OFF, 0), opt(t("Ab 500 ms", "Over 500 ms"), 1)]),
        ),
        pending(t("Adresse", "Address")),
    ],
};

static SYNC: Page = Page {
    title: t("Sitzungen abgleichen", "Sync sessions"),
    entries: &[pending(t("Jetzt", "Now")), switch(t("Automatisch", "Automatically"), S::SyncAuto)],
};

static CLOCK: Page = Page {
    title: t("Uhrzeit", "Clock"),
    entries: &[
        entry(t("Datum", "Date"), Kind::Time(TimeField::Date)),
        entry(t("Uhrzeit", "Time"), Kind::Time(TimeField::Clock)),
        entry(
            t("Quelle", "Source"),
            Kind::Choice(S::TimeFromNetwork, &[opt(t("Netzwerk (NTP)", "Network (NTP)"), 1), opt(t("Von Hand", "By hand"), 0)]),
        ),
        entry(t("Zeitzone", "Time zone"), Kind::Choice(S::Zone, &ZONES)),
    ],
};

/// In the order of [`crate::settings::Zone::ALL`].
const ZONES: [Opt; 14] = [
    opt(same("UTC"), 0),
    opt(t("London (UTC+0)", "London (UTC+0)"), 1),
    opt(t("Berlin (UTC+1)", "Berlin (UTC+1)"), 2),
    opt(t("Helsinki (UTC+2)", "Helsinki (UTC+2)"), 3),
    opt(t("Moskau (UTC+3)", "Moscow (UTC+3)"), 4),
    opt(t("Dubai (UTC+4)", "Dubai (UTC+4)"), 5),
    opt(t("Indien (UTC+5:30)", "India (UTC+5:30)"), 6),
    opt(t("China (UTC+8)", "China (UTC+8)"), 7),
    opt(t("Tokio (UTC+9)", "Tokyo (UTC+9)"), 8),
    opt(t("Sydney (UTC+10)", "Sydney (UTC+10)"), 9),
    opt(t("New York (UTC−5)", "New York (UTC−5)"), 10),
    opt(t("Chicago (UTC−6)", "Chicago (UTC−6)"), 11),
    opt(t("Denver (UTC−7)", "Denver (UTC−7)"), 12),
    opt(t("Los Angeles (UTC−8)", "Los Angeles (UTC−8)"), 13),
];

static FIRMWARE: Page = Page {
    title: same("Firmware-Update"),
    entries: &[
        entry(t("Installierte Version", "Installed version"), Kind::Info(Info::Version)),
        pending(t("Suchen", "Check for updates")),
        pending(t("Installieren", "Install")),
    ],
};

static BLUETOOTH: Page = Page {
    title: same("Bluetooth"),
    entries: &[
        switch(same("Bluetooth"), S::Bluetooth),
        pending(t("Koppeln", "Pair")),
        pending(t("Gekoppelte Geräte", "Paired devices")),
        switch(t("Sichtbar", "Visible"), S::Visible),
    ],
};

/// The Sitzungen tile of MODE (§6). Renaming, duplicating and deleting act on the open session;
/// the backend keeps them, on the SD card when there is one.
pub static SESSIONS: Page = Page {
    title: t("Sitzungen", "Sessions"),
    entries: &[
        entry(t("Neu", "New"), Kind::Do(Action::NewSession)),
        entry(t("Öffnen", "Open"), Kind::Sessions),
        entry(t("Umbenennen", "Rename"), Kind::Rename),
        entry(t("Duplizieren", "Duplicate"), Kind::Do(Action::DuplicateSession)),
        entry(t("Löschen", "Delete"), Kind::Confirm(Action::DeleteSession)),
    ],
};

// OPTN (§7.3): the mode's page of groups. An entry the editor can write inserts its LaTeX; one
// that waits for the engine ([eng]) or for a template the editor lacks says so.

const fn ins(label: Text, latex: &'static str) -> Entry {
    entry(label, Kind::Insert(latex))
}

const fn group(label: Text, page: &'static Page) -> Entry {
    entry(label, Kind::Page(page))
}

/// The OPTN page of a mode.
pub fn optn(mode: Mode) -> &'static Page {
    match mode {
        Mode::Physics => &OPTN_PHYSICS,
        Mode::Algebra => &OPTN_ALGEBRA,
        Mode::Imaginary => &OPTN_COMPLEX,
        Mode::Analysis => &OPTN_ANALYSIS,
        Mode::Linear => &OPTN_LINEAR,
        Mode::Table => &OPTN_TABLE,
        Mode::Matrix => &OPTN_MATRIX,
        Mode::BaseN => &OPTN_BASE_N,
        Mode::Statistics => &OPTN_STATISTICS,
        Mode::Distributions => &OPTN_DISTRIBUTIONS,
        Mode::Verify => &OPTN_VERIFY,
        // The tile opens the sessions menu; OPTN never sees it as the mode.
        Mode::Sessions => &OPTN_PHYSICS,
    }
}

const ANALYSIS_GROUP: Entry = group(same("Analysis"), &ANALYSIS);
const NUMBER_GROUP: Entry = group(t("Zahl", "Number"), &NUMBER);
const HYPERBOLIC_GROUP: Entry = group(t("Hyperbol. Fkt.", "Hyperbolic"), &HYPERBOLIC);
const ANGLE_GROUP: Entry = group(t("Winkel", "Angle"), &ANGLE);
const GREEK_GROUP: Entry = group(t("Griechisch", "Greek"), &GREEK);
const TOOLS_GROUP: Entry = group(t("Werkzeuge", "Tools"), &TOOLS);

static ANALYSIS: Page = Page {
    title: same("Analysis"),
    entries: &[
        ins(t("∫ Integral", "∫ Integral"), r"\int_{}^{}"),
        ins(t("d/dx Ableitung", "d/dx Derivative"), r"\frac{d}{dx}"),
        ins(t("∑ Summe", "∑ Sum"), r"\sum_{}^{}"),
        ins(t("∏ Produkt", "∏ Product"), r"\prod_{}^{}"),
        ins(t("lim Grenzwert", "lim Limit"), r"\lim_{x\to}"),
    ],
};

static NUMBER: Page = Page {
    title: t("Zahl", "Number"),
    entries: &[
        ins(t("|x| Betrag", "|x| Absolute value"), "||"),
        ins(t("n! Fakultät", "n! Factorial"), "!"),
        ins(same("nCr"), r"\binom{}{}"),
        ins(same("nPr"), r"\operatorname{nPr}(;)"),
        ins(t("ggT", "gcd"), r"\operatorname{ggT}(;)"),
        ins(t("kgV", "lcm"), r"\operatorname{kgV}(;)"),
        ins(same("Int"), r"\operatorname{Int}()"),
        ins(same("Intg"), r"\operatorname{Intg}()"),
        ins(same("Rnd"), r"\operatorname{Rnd}()"),
        ins(same("RndFix"), r"\operatorname{RndFix}(;)"),
        ins(t("÷R Rest", "÷R Remainder"), r"\operatorname{divR}(;)"),
        ins(same("mod"), r"\bmod "),
        ins(t("Abrunden (floor)", "Floor"), r"\lfloor\rfloor"),
        ins(t("Aufrunden (ceil)", "Ceiling"), r"\lceil\rceil"),
        ins(same("max"), r"\max(;)"),
        ins(same("min"), r"\min(;)"),
        ins(same("%"), r"\%"),
    ],
};

static HYPERBOLIC: Page = Page {
    title: t("Hyperbol. Fkt.", "Hyperbolic"),
    entries: &[
        ins(same("sinh"), r"\sinh()"),
        ins(same("cosh"), r"\cosh()"),
        ins(same("tanh"), r"\tanh()"),
        ins(same("arsinh"), r"\operatorname{arsinh}()"),
        ins(same("arcosh"), r"\operatorname{arcosh}()"),
        ins(same("artanh"), r"\operatorname{artanh}()"),
        ins(same("coth"), r"\coth()"),
    ],
};

static ANGLE: Page = Page {
    title: t("Winkel", "Angle"),
    entries: &[
        ins(t("° Grad", "° Degrees"), "°"),
        // ′ and ″ are not in the fonts; the engine reads ' and '' alike.
        ins(t("' Minuten", "' Minutes"), "'"),
        ins(t("'' Sekunden", "'' Seconds"), "''"),
        ins(t("rad Bogenmaß", "rad Radians"), r"\,\mathrm{rad}"),
    ],
};

static GREEK: Page = Page {
    title: t("Griechisch", "Greek"),
    entries: &[
        ins(same("α"), r"\alpha"),
        ins(same("β"), r"\beta"),
        ins(same("γ"), r"\gamma"),
        ins(same("δ"), r"\delta"),
        ins(same("ε"), r"\varepsilon"),
        ins(same("θ"), r"\theta"),
        ins(same("λ"), r"\lambda"),
        ins(same("μ"), r"\mu"),
        ins(same("ρ"), r"\rho"),
        ins(same("σ"), r"\sigma"),
        ins(same("τ"), r"\tau"),
        ins(same("φ"), r"\varphi"),
        ins(same("ω"), r"\omega"),
        ins(same("Δ"), r"\Delta"),
        ins(same("Ω"), r"\Omega"),
    ],
};

static TOOLS: Page = Page {
    title: t("Werkzeuge", "Tools"),
    entries: &[entry(same("CALC"), Kind::Do(Action::Calc)), entry(same("SOLVE"), Kind::Do(Action::Solve)), CONVERSIONS_ENTRY],
};

const CONSTANTS_ENTRY: Entry = entry(t("Konstanten", "Constants"), Kind::Catalogue(Catalogue::Constants));

/// What the CONST key opens: the constants by category; a message outside Physik.
pub static CONSTANTS: Page = Page { title: t("Konstanten", "Constants"), entries: &[CONSTANTS_ENTRY] };

/// What STO and VARS open over.
pub static VARIABLES: Page = Page { title: t("Variablen", "Variables"), entries: &[] };

/// A value converted to the unit chosen: `▶ unit`.
const CONVERSIONS_ENTRY: Entry = entry(t("In Einheit", "To unit"), Kind::Catalogue(Catalogue::Units));

/// What the CONV key opens: the units by category.
pub static CONVERSIONS: Page = Page { title: t("Umrechnen", "Convert"), entries: &[CONVERSIONS_ENTRY] };

static OPTN_PHYSICS: Page = Page {
    title: t("Physik", "Physics"),
    entries: &[
        group(t("Einheiten", "Units"), &UNITS),
        group(t("Präfixe", "Prefixes"), &PREFIXES),
        CONSTANTS_ENTRY,
        ANALYSIS_GROUP,
        NUMBER_GROUP,
        ANGLE_GROUP,
        HYPERBOLIC_GROUP,
        TOOLS_GROUP,
    ],
};

/// A page of units for one quantity, inserted as letters: Physik reads letters after a number
/// as units, and a prefix from [`PREFIXES`] goes in front.
macro_rules! units {
    ($name:ident, $title:expr, [$($unit:literal),+ $(,)?]) => {
        static $name: Page = Page { title: $title, entries: &[$(ins(same($unit), $unit)),+] };
    };
}

units!(LENGTH, t("Länge", "Length"), ["m", "Å", "AU", "ly"]);
units!(MASS, t("Masse", "Mass"), ["g", "t", "Da"]);
units!(TIME, t("Zeit", "Time"), ["s", "min", "h", "d", "wk", "mo", "yr"]);
units!(CURRENT, t("Strom", "Current"), ["A"]);
units!(TEMPERATURE, t("Temperatur", "Temperature"), ["K", "°C"]);
units!(FORCE, t("Kraft", "Force"), ["N"]);
units!(PRESSURE, t("Druck", "Pressure"), ["Pa", "bar", "atm", "Torr"]);
units!(ENERGY, t("Energie", "Energy"), ["J", "eV", "cal", "Wh"]);
units!(POWER, t("Leistung", "Power"), ["W"]);
units!(CHARGE, t("Ladung", "Charge"), ["C"]);
units!(VOLTAGE, t("Spannung", "Voltage"), ["V"]);
units!(RESISTANCE, t("Widerstand", "Resistance"), ["Ω"]);
units!(CAPACITANCE, t("Kapazität", "Capacitance"), ["F"]);
units!(AREA_VOLUME, t("Fläche, Volumen", "Area, volume"), ["ha", "l"]);
units!(SPEED, t("Geschwindigkeit", "Speed"), ["mph"]);

static UNITS: Page = Page {
    title: t("Einheiten", "Units"),
    entries: &[
        group(t("Länge", "Length"), &LENGTH),
        group(t("Masse", "Mass"), &MASS),
        group(t("Zeit", "Time"), &TIME),
        group(t("Strom", "Current"), &CURRENT),
        group(t("Temperatur", "Temperature"), &TEMPERATURE),
        group(t("Kraft", "Force"), &FORCE),
        group(t("Druck", "Pressure"), &PRESSURE),
        group(t("Energie", "Energy"), &ENERGY),
        group(t("Leistung", "Power"), &POWER),
        group(t("Ladung", "Charge"), &CHARGE),
        group(t("Spannung", "Voltage"), &VOLTAGE),
        group(t("Widerstand", "Resistance"), &RESISTANCE),
        group(t("Kapazität", "Capacitance"), &CAPACITANCE),
        group(t("Fläche, Volumen", "Area, volume"), &AREA_VOLUME),
        group(t("Geschwindigkeit", "Speed"), &SPEED),
    ],
};

/// All 24 prefixes, q to Q; micro as `\mu`, since `u` is the dalton.
static PREFIXES: Page = Page {
    title: t("Präfixe", "Prefixes"),
    entries: &[
        ins(same("Q  quetta"), "Q"),
        ins(same("R  ronna"), "R"),
        ins(same("Y  yotta"), "Y"),
        ins(same("Z  zetta"), "Z"),
        ins(same("E  exa"), "E"),
        ins(same("P  peta"), "P"),
        ins(same("T  tera"), "T"),
        ins(same("G  giga"), "G"),
        ins(same("M  mega"), "M"),
        ins(same("k  kilo"), "k"),
        ins(same("h  hekto"), "h"),
        ins(same("da  deka"), "da"),
        ins(same("d  dezi"), "d"),
        ins(same("c  zenti"), "c"),
        ins(same("m  milli"), "m"),
        ins(same("μ  mikro"), r"\mu "),
        ins(same("n  nano"), "n"),
        ins(same("p  piko"), "p"),
        ins(same("f  femto"), "f"),
        ins(same("a  atto"), "a"),
        ins(same("z  zepto"), "z"),
        ins(same("y  yokto"), "y"),
        ins(same("r  ronto"), "r"),
        ins(same("q  quekto"), "q"),
    ],
};

static OPTN_ALGEBRA: Page = Page {
    title: same("Algebra"),
    entries: &[
        group(t("Gleichungssystem", "System of equations"), &SYSTEM),
        GREEK_GROUP,
        ANALYSIS_GROUP,
        NUMBER_GROUP,
        HYPERBOLIC_GROUP,
        TOOLS_GROUP,
    ],
};

static SYSTEM: Page = Page {
    title: t("Gleichungssystem", "System of equations"),
    entries: &[ins(t("; zwischen Gleichungen", "; between equations"), ";"), pending(same("cases"))],
};

static OPTN_COMPLEX: Page = Page {
    title: t("Komplex", "Complex"),
    entries: &[
        ins(t("Argument", "Argument"), r"\arg()"),
        pending(t("Konjugierte", "Conjugate")),
        ins(t("Realteil", "Real part"), r"\Re()"),
        ins(t("Imaginärteil", "Imaginary part"), r"\Im()"),
        ins(t("Betrag", "Absolute value"), "||"),
        pending(t("In Polarform", "To polar form")),
        pending(t("In a+bi", "To a+bi")),
        pending(t("Winkel-Eingabe", "Angle input")),
        ANALYSIS_GROUP,
        NUMBER_GROUP,
    ],
};

/// The pages of the last sketch, and the forms of the three tasks on the function typed.
static OPTN_ANALYSIS: Page = Page {
    title: t("Kurvendiskussion", "Curve sketching"),
    entries: &[
        entry(t("Abschnitte", "Sections"), Kind::Sections),
        entry(t("Tangente, Normale", "Tangent, normal"), Kind::Do(Action::TangentForm)),
        entry(t("Fläche", "Area"), Kind::Do(Action::AreaForm)),
        entry(t("Wertetabelle", "Table"), Kind::Do(Action::PointsForm)),
        ANALYSIS_GROUP,
        NUMBER_GROUP,
        HYPERBOLIC_GROUP,
    ],
};

/// f(x), g(x), von, bis and Schritt are the rows of the form; OPTN clears it and brings the
/// functions.
static OPTN_TABLE: Page = Page {
    title: t("Wertetabelle", "Table"),
    entries: &[
        entry(CLEAR_ALL, Kind::Do(Action::ClearInput)),
        NUMBER_GROUP,
        HYPERBOLIC_GROUP,
    ],
};

const CLEAR_ALL: Text = t("Alles löschen", "Clear all");

static OPTN_LINEAR: Page = Page {
    title: t("LGS", "Linear systems"),
    entries: &[
        entry(
            t("Eingabe", "Input"),
            Kind::Choice(S::SystemInput, &[opt(t("Gleichungen", "Equations"), 0), opt(same("Matrix [A|b]"), 1)]),
        ),
        entry(t("Zeilen", "Rows"), Kind::Number(S::SystemRows, 1, 10)),
        entry(t("Spalten", "Columns"), Kind::Number(S::SystemColumns, 1, 10)),
        switch(t("Imaginäre Einheit i", "Imaginary unit i"), S::LinearComplex),
        entry(CLEAR_ALL, Kind::Do(Action::ClearInput)),
    ],
};

/// A matrix is defined under a name in a grid of the size set here, and edited in VARS; the
/// last result is the Ans key's.
static OPTN_MATRIX: Page = Page {
    title: t("Matrizen", "Matrices"),
    entries: &[
        entry(t("Definieren", "Define"), Kind::Do(Action::DefineMatrix)),
        entry(t("Bearbeiten", "Edit"), Kind::Do(Action::EditMatrix)),
        entry(t("Zeilen", "Rows"), Kind::Number(S::MatrixRows, 1, 10)),
        entry(t("Spalten", "Columns"), Kind::Number(S::MatrixColumns, 1, 10)),
        pending(t("Determinante", "Determinant")),
        pending(t("Transponierte", "Transpose")),
        pending(t("Inverse", "Inverse")),
        pending(t("Einheitsmatrix", "Identity")),
        pending(same("REF")),
        pending(same("RREF")),
    ],
};

/// The base numbers are typed and shown in, a prefix for one number in another, and the
/// bitwise logic; A–F are letters.
static OPTN_BASE_N: Page = Page {
    title: same("Basis-N"),
    entries: &[
        entry(
            t("Basis", "Base"),
            Kind::Choice(S::NumberBase, &[opt(same("DEC"), 0), opt(same("HEX"), 1), opt(same("BIN"), 2), opt(same("OCT"), 3)]),
        ),
        group(t("Präfixe d, h, b, o", "Prefixes d, h, b, o"), &BASE_PREFIXES),
        ins(same("and"), r"\operatorname{and}"),
        ins(same("or"), r"\operatorname{or}"),
        ins(same("xor"), r"\operatorname{xor}"),
        ins(same("xnor"), r"\operatorname{xnor}"),
        ins(same("Not"), r"\operatorname{Not}()"),
        ins(same("Neg"), r"\operatorname{Neg}()"),
    ],
};

static BASE_PREFIXES: Page = Page {
    title: t("Präfixe", "Prefixes"),
    entries: &[
        ins(t("d dezimal", "d decimal"), r"\mathrm{d}"),
        ins(t("h hexadezimal", "h hexadecimal"), r"\mathrm{h}"),
        ins(t("b binär", "b binary"), r"\mathrm{b}"),
        ins(t("o oktal", "o octal"), r"\mathrm{o}"),
    ],
};

/// The sums, the measures, min, max, the quartiles and the regression are all on the answer
/// EXE brings.
static OPTN_STATISTICS: Page = Page {
    title: t("Statistik", "Statistics"),
    entries: &[
        entry(t("Typ", "Type"), Kind::Choice(S::StatisticsType, &STATISTICS_TYPES)),
        switch(t("Häufigkeit", "Frequency"), S::StatisticsFreq),
        entry(t("Zeile einfügen", "Insert row"), Kind::Do(Action::InsertRow)),
        entry(t("Zeile löschen", "Delete row"), Kind::Do(Action::DeleteRow)),
        entry(CLEAR_ALL, Kind::Do(Action::ClearInput)),
    ],
};

static STATISTICS_TYPES: [Opt; 8] = [
    opt(t("1 Variable", "1 variable"), 0),
    opt(same("y = a + bx"), 1),
    opt(same("y = a + bx + cx²"), 2),
    opt(same("y = a + b·ln(x)"), 3),
    opt(same("y = a·e^(bx)"), 4),
    opt(same("y = a·b^x"), 5),
    opt(same("y = a·x^b"), 6),
    opt(same("y = a + b/x"), 7),
];

/// k takes one value or several, separated by `;`: there is no list to switch to.
static OPTN_DISTRIBUTIONS: Page = Page {
    title: t("Verteilungen", "Distributions"),
    entries: &[
        entry(t("Typ", "Type"), Kind::Choice(S::Distribution, &DISTRIBUTIONS)),
        entry(CLEAR_ALL, Kind::Do(Action::ClearInput)),
        GREEK_GROUP,
    ],
};

static DISTRIBUTIONS: [Opt; 7] = [
    opt(t("Normal-Dichte", "Normal PD"), 0),
    opt(t("Kumul. Normal-V.", "Normal CD"), 1),
    opt(t("Inv. Normal-V.", "Inverse normal"), 2),
    opt(t("Binomial-Dichte", "Binomial PD"), 3),
    opt(t("Kumul. Binom.-V.", "Binomial CD"), 4),
    opt(t("Poisson-Dichte", "Poisson PD"), 5),
    opt(t("Kumul. Poisson-V.", "Poisson CD"), 6),
];

static OPTN_VERIFY: Page = Page {
    title: t("Prüfen", "Verify"),
    entries: &[
        ins(same("="), "="),
        ins(same("≠"), r"\neq "),
        ins(same("<"), "<"),
        ins(same(">"), ">"),
        ins(same("≤"), r"\le "),
        ins(same("≥"), r"\ge "),
    ],
};
