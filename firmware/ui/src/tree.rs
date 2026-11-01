//! The menu tree of design/handheld-menus.md §7: SETUP (§7.2). Which entries work and which
//! wait for the engine or the firmware is noted there.

use crate::menu::{opt, Action, Entry, Info, Kind, Opt, Page, ON_OFF};
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
        entry(
            t("Automatisch aus", "Auto power off"),
            Kind::Choice(S::AutoOff, &[opt(same("10 min"), 10), opt(same("60 min"), 60)]),
        ),
        entry(t("Zurücksetzen", "Reset"), Kind::Page(&RESET)),
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
        pending(t("Netzwerke", "Networks")),
        pending(t("Gespeicherte Netzwerke", "Saved networks")),
        pending(same("Status")),
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
        entry(
            t("Quelle", "Source"),
            Kind::Choice(S::TimeFromNetwork, &[opt(t("Netzwerk (NTP)", "Network (NTP)"), 1), opt(t("Von Hand", "By hand"), 0)]),
        ),
        entry(t("Stunde", "Hour"), Kind::Number(S::Hour, 0, 23)),
        entry(same("Minute"), Kind::Number(S::Minute, 0, 59)),
        entry(t("Zeitzone (UTC)", "Time zone (UTC)"), Kind::Number(S::TimeZone, -12, 14)),
    ],
};

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
