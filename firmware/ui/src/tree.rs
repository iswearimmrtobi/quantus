//! The menu tree of design/handheld-menus.md §7: SETUP (§7.2), OPTN per mode (§7.3) and the
//! Sitzungen tile (§6). Which entries work and which
//! wait for the engine or the firmware is noted there.

use crate::app::Mode;
use crate::backend::{Catalogue, Item};
use crate::menu::{opt, Action, Entry, Info, Kind, Opt, Page, TimeField, ON_OFF};
use crate::settings::Setting as S;
use crate::text::{same, tx, Lang, Text};

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
const EXPANDED: Text = tx!("hui.tree.expanded");

pub static SETUP: Page = Page {
    title: crate::text::SETUP,
    entries: &[
        entry(tx!("hui.tree.general"), Kind::Page(&GENERAL)),
        entry(tx!("hui.tree.results"), Kind::Page(&RESULTS)),
        entry(tx!("hui.tree.working"), Kind::Page(&WORKING)),
        entry(same("Algebra"), Kind::Page(&ALGEBRA)),
        entry(tx!("hui.tree.input"), Kind::Page(&INPUT)),
        entry(tx!("hui.tree.device"), Kind::Page(&DEVICE)),
        entry(tx!("hui.tree.connections"), Kind::Page(&CONNECTIONS)),
    ],
};

/// Every language of `locales/`, each by its own name.
const LANGUAGES: [Opt; Lang::ALL.len()] = {
    const NONE: Opt = opt(same(""), 0);
    let mut opts = [NONE; Lang::ALL.len()];
    let mut i = 0;
    while i < opts.len() {
        opts[i] = opt(same(Lang::ALL[i].name()), i as i32);
        i += 1;
    }
    opts
};

static GENERAL: Page = Page {
    title: tx!("hui.tree.general"),
    entries: &[
        entry(
            tx!("hui.tree.language"),
            Kind::Choice(S::Language, &LANGUAGES),
        ),
        entry(
            tx!("hui.tree.angle_unit"),
            Kind::Choice(S::Angle, &[opt(tx!("hui.tree.degrees"), 0), opt(tx!("hui.tree.radians"), 1)]),
        ),
        entry(tx!("hui.tree.significant_digits"), Kind::Number(S::Digits, 1, 15)),
        entry(tx!("hui.tree.number_format"), Kind::Choice(S::Format, &FORMATS)),
        switch(tx!("hui.tree.decimal_prefixes"), S::Prefixes),
        switch(tx!("hui.tree.digit_grouping"), S::Grouping),
    ],
};

static FORMATS: [Opt; 4] = [
    opt(same("Norm 1"), 0),
    opt(same("Norm 2"), 1),
    Opt { text: same("Fix"), value: 2, then: Some((S::FormatDigits, 0, 9)) },
    Opt { text: same("Sci"), value: 3, then: Some((S::FormatDigits, 0, 9)) },
];

static RESULTS: Page = Page {
    title: tx!("hui.tree.results"),
    entries: &[
        entry(
            tx!("hui.tree.exact_decimal"),
            Kind::Choice(S::Decimal, &[opt(tx!("hui.tree.exact"), 0), opt(tx!("hui.tree.decimal"), 1)]),
        ),
        entry(
            tx!("hui.tree.fractions"),
            Kind::Choice(S::Fraction, &[opt(same("d/c"), 0), opt(same("ab/c"), 1)]),
        ),
        switch(tx!("hui.tree.recurring_decimals"), S::Recurring),
        entry(
            tx!("hui.tree.complex_numbers"),
            Kind::Choice(S::Complex, &[opt(same("a+bi"), 0), opt(same("Polar (r, θ)"), 1)]),
        ),
        entry(
            tx!("hui.tree.quadratic_equations"),
            Kind::Choice(S::QuadraticFormula, &[opt(AUTO, 0), opt(same("pq"), 1), opt(same("abc"), 2)]),
        ),
    ],
};

static WORKING: Page = Page {
    title: tx!("hui.tree.working"),
    entries: &[
        entry(
            tx!("hui.tree.working"),
            Kind::Choice(
                S::StepDetail,
                &[
                    opt(tx!("hui.tree.none"), 0),
                    opt(tx!("hui.tree.brief"), 1),
                    opt(same("Standard"), 2),
                    opt(tx!("hui.tree.detailed"), 3),
                    opt(same("Maximal"), 4),
                ],
            ),
        ),
        switch(tx!("hui.tree.strike_out_cancelling"), S::CancelMarks),
        entry(
            tx!("hui.tree.linear_systems"),
            Kind::Choice(S::Elimination, &[opt(same("Gauß"), 0), opt(same("Gauß-Jordan"), 1)]),
        ),
    ],
};

static ALGEBRA: Page = Page {
    title: same("Algebra"),
    entries: &[
        entry(
            tx!("hui.tree.binomial_formulas"),
            Kind::Choice(S::BinomialForm, &[opt(AUTO, 0), opt(EXPANDED, 1), opt(tx!("hui.tree.binomial"), 2)]),
        ),
        switch(tx!("hui.tree.completing_square"), S::CompleteSquare),
        entry(
            tx!("hui.tree.denominators"),
            Kind::Choice(S::DenominatorForm, &[opt(AUTO, 0), opt(EXPANDED, 1), opt(tx!("hui.tree.factored"), 2)]),
        ),
        entry(
            tx!("hui.tree.identities"),
            Kind::Choice(
                S::ConditionalIdentities,
                &[opt(tx!("hui.tree.condition"), 0), opt(tx!("hui.tree.always_valid"), 1)],
            ),
        ),
    ],
};

static INPUT: Page = Page {
    title: tx!("hui.tree.input"),
    entries: &[
        switch(tx!("hui.tree.a_and_y_as_variables"), S::AAndYAsVariables),
        switch(tx!("hui.tree.letters_product"), S::LetterProducts),
        switch(tx!("hui.tree.extra_unit_letters"), S::ExtraUnitLetters),
        entry(
            tx!("hui.tree.unknowns_systems"),
            Kind::Choice(S::UnknownNames, &[opt(tx!("hui.tree.unknowns_indexed"), 0), opt(tx!("hui.tree.unknowns_letters"), 1)]),
        ),
    ],
};

static DEVICE: Page = Page {
    title: tx!("hui.tree.device"),
    entries: &[
        entry(tx!("hui.tree.brightness"), Kind::Number(S::Brightness, 1, 10)),
        entry(
            tx!("hui.tree.font_size"),
            Kind::Choice(S::FontSize, &[opt(same("Normal"), 0), opt(tx!("hui.tree.small"), 1)]),
        ),
        entry(tx!("hui.tree.wallpaper"), Kind::Page(&WALLPAPER)),
        entry(tx!("hui.tree.storage"), Kind::Storage),
        entry(
            tx!("hui.tree.auto_power_off"),
            Kind::Choice(S::AutoOff, &[opt(same("10 min"), 10), opt(same("60 min"), 60)]),
        ),
        entry(tx!("hui.tree.reset"), Kind::Page(&RESET)),
        entry(tx!("hui.tree.calibrate_touch"), Kind::Do(Action::Calibrate)),
    ],
};

/// A picture from the SD card behind the screens (design/handheld-menus.md §7.6).
static WALLPAPER: Page = Page {
    title: tx!("hui.tree.wallpaper"),
    entries: &[
        entry(tx!("hui.tree.picture"), Kind::Wallpapers),
        entry(
            tx!("hui.tree.show"),
            Kind::Choice(
                S::WallpaperOn,
                &[opt(tx!("hui.tree.main_menu"), 0), opt(tx!("hui.tree.menus"), 1), opt(tx!("hui.tree.everywhere"), 2)],
            ),
        ),
    ],
};

static RESET: Page = Page {
    title: tx!("hui.tree.reset"),
    entries: &[
        entry(tx!("hui.tree.settings"), Kind::Confirm(Action::ResetSettings)),
        entry(tx!("hui.tree.memory"), Kind::Confirm(Action::ResetMemory)),
        entry(tx!("hui.tree.everything"), Kind::Confirm(Action::ResetAll)),
    ],
};

static CONNECTIONS: Page = Page {
    title: tx!("hui.tree.connections"),
    entries: &[
        switch(tx!("hui.tree.exam_mode"), S::Exam),
        entry(tx!("hui.tree.wi_fi"), Kind::Page(&WLAN)),
        entry(tx!("hui.tree.remote_engine"), Kind::Page(&REMOTE)),
        entry(tx!("hui.tree.sync_sessions"), Kind::Page(&SYNC)),
        entry(tx!("hui.tree.clock"), Kind::Page(&CLOCK)),
        entry(tx!("hui.tree.firmware_update"), Kind::Page(&FIRMWARE)),
        entry(same("Bluetooth"), Kind::Page(&BLUETOOTH)),
    ],
};

static WLAN: Page = Page {
    title: tx!("hui.tree.wi_fi"),
    entries: &[
        switch(tx!("hui.tree.wi_fi"), S::Wlan),
        entry(tx!("hui.tree.networks"), Kind::Networks),
        entry(tx!("hui.tree.saved_networks"), Kind::SavedNetworks),
        entry(same("Status"), Kind::WlanStatus),
    ],
};

static REMOTE: Page = Page {
    title: tx!("hui.tree.remote_engine"),
    entries: &[
        entry(
            tx!("hui.tree.remote_engine"),
            Kind::Choice(S::Remote, &[opt(crate::text::OFF, 0), opt(tx!("hui.tree.long_press"), 1)]),
        ),
        pending(tx!("hui.tree.address")),
    ],
};

static SYNC: Page = Page {
    title: tx!("hui.tree.sync_sessions"),
    entries: &[pending(tx!("hui.tree.now")), switch(tx!("hui.tree.automatically"), S::SyncAuto)],
};

static CLOCK: Page = Page {
    title: tx!("hui.tree.clock"),
    entries: &[
        entry(tx!("hui.tree.date"), Kind::Time(TimeField::Date)),
        entry(tx!("hui.tree.time"), Kind::Time(TimeField::Clock)),
        entry(
            tx!("hui.tree.source"),
            Kind::Choice(S::TimeFromNetwork, &[opt(tx!("hui.tree.network_ntp"), 1), opt(tx!("hui.tree.hand"), 0)]),
        ),
        entry(tx!("hui.tree.time_zone"), Kind::Choice(S::Zone, &ZONES)),
    ],
};

/// In the order of [`crate::settings::Zone::ALL`].
const ZONES: [Opt; 14] = [
    opt(same("UTC"), 0),
    opt(tx!("hui.tree.zone.london"), 1),
    opt(tx!("hui.tree.zone.berlin"), 2),
    opt(tx!("hui.tree.zone.helsinki"), 3),
    opt(tx!("hui.tree.zone.moscow"), 4),
    opt(tx!("hui.tree.zone.dubai"), 5),
    opt(tx!("hui.tree.zone.india"), 6),
    opt(tx!("hui.tree.zone.china"), 7),
    opt(tx!("hui.tree.zone.tokyo"), 8),
    opt(tx!("hui.tree.zone.sydney"), 9),
    opt(tx!("hui.tree.zone.new_york"), 10),
    opt(tx!("hui.tree.zone.chicago"), 11),
    opt(tx!("hui.tree.zone.denver"), 12),
    opt(tx!("hui.tree.zone.los_angeles"), 13),
];

static FIRMWARE: Page = Page {
    title: tx!("hui.tree.firmware_update"),
    entries: &[
        entry(tx!("hui.tree.installed_version"), Kind::Info(Info::Version)),
        pending(tx!("hui.tree.check_updates")),
        pending(tx!("hui.tree.install")),
    ],
};

static BLUETOOTH: Page = Page {
    title: same("Bluetooth"),
    entries: &[
        switch(same("Bluetooth"), S::Bluetooth),
        pending(tx!("hui.tree.pair")),
        pending(tx!("hui.tree.paired_devices")),
        switch(tx!("hui.tree.visible"), S::Visible),
    ],
};

/// The Sitzungen tile of MODE (§6). Renaming, duplicating and deleting act on the open session;
/// the backend keeps them, on the SD card when there is one.
pub static SESSIONS: Page = Page {
    title: tx!("hui.tree.sessions"),
    entries: &[
        entry(tx!("hui.tree.new"), Kind::Do(Action::NewSession)),
        entry(tx!("hui.tree.open"), Kind::Sessions),
        entry(tx!("hui.tree.rename"), Kind::Rename),
        entry(tx!("hui.tree.duplicate"), Kind::Do(Action::DuplicateSession)),
        entry(tx!("hui.tree.delete"), Kind::Confirm(Action::DeleteSession)),
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

/// Every entry of a mode's OPTN menu that inserts something, by name: the CATALOG.
pub fn functions(mode: Mode, lang: Lang) -> Vec<Item> {
    fn walk(page: &Page, lang: Lang, items: &mut Vec<Item>) {
        for entry in page.entries {
            match &entry.kind {
                Kind::Page(page) => walk(page, lang, items),
                Kind::Insert(latex) => items.push(Item { name: entry.label.get(lang).to_string(), latex: latex.to_string() }),
                _ => {}
            }
        }
    }
    let mut items = Vec::new();
    walk(optn(mode), lang, &mut items);
    items.sort_by_cached_key(|item| item.name.to_lowercase());
    items.dedup_by(|a, b| a.name == b.name);
    items
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
const NUMBER_GROUP: Entry = group(tx!("hui.tree.number"), &NUMBER);
const HYPERBOLIC_GROUP: Entry = group(tx!("hui.tree.hyperbolic"), &HYPERBOLIC);
const ANGLE_GROUP: Entry = group(tx!("hui.tree.angle"), &ANGLE);
const GREEK_GROUP: Entry = group(tx!("hui.tree.greek"), &GREEK);
const TOOLS_GROUP: Entry = group(tx!("hui.tree.tools"), &TOOLS);

static ANALYSIS: Page = Page {
    title: same("Analysis"),
    entries: &[
        ins(tx!("hui.tree.integral"), r"\int_{}^{}"),
        ins(tx!("hui.tree.derivative"), r"\frac{d}{dx}"),
        ins(tx!("hui.tree.sum"), r"\sum_{}^{}"),
        ins(tx!("hui.tree.product"), r"\prod_{}^{}"),
        ins(tx!("hui.tree.lim_limit"), r"\lim_{x\to}"),
    ],
};

static NUMBER: Page = Page {
    title: tx!("hui.tree.number"),
    entries: &[
        ins(tx!("hui.tree.absolute_value_key"), "||"),
        ins(tx!("hui.tree.factorial"), "!"),
        ins(same("nCr"), r"\binom{}{}"),
        ins(same("nPr"), r"\operatorname{nPr}(;)"),
        ins(tx!("hui.tree.gcd"), r"\operatorname{ggT}(;)"),
        ins(tx!("hui.tree.lcm"), r"\operatorname{kgV}(;)"),
        ins(same("Int"), r"\operatorname{Int}()"),
        ins(same("Intg"), r"\operatorname{Intg}()"),
        ins(same("Rnd"), r"\operatorname{Rnd}()"),
        ins(same("RndFix"), r"\operatorname{RndFix}(;)"),
        ins(tx!("hui.tree.remainder"), r"\operatorname{divR}(;)"),
        ins(same("mod"), r"\bmod "),
        ins(tx!("hui.tree.floor"), r"\lfloor\rfloor"),
        ins(tx!("hui.tree.ceiling"), r"\lceil\rceil"),
        ins(same("max"), r"\max(;)"),
        ins(same("min"), r"\min(;)"),
        ins(same("%"), r"\%"),
    ],
};

static HYPERBOLIC: Page = Page {
    title: tx!("hui.tree.hyperbolic"),
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
    title: tx!("hui.tree.angle"),
    entries: &[
        ins(tx!("hui.tree.degrees_symbol"), "°"),
        // ′ and ″ are not in the fonts; the engine reads ' and '' alike.
        ins(tx!("hui.tree.minutes"), "'"),
        ins(tx!("hui.tree.seconds"), "''"),
        ins(tx!("hui.tree.rad_radians"), r"\,\mathrm{rad}"),
    ],
};

static GREEK: Page = Page {
    title: tx!("hui.tree.greek"),
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
    title: tx!("hui.tree.tools"),
    entries: &[entry(same("CALC"), Kind::Do(Action::Calc)), entry(same("SOLVE"), Kind::Do(Action::Solve)), CONVERSIONS_ENTRY],
};

const CONSTANTS_ENTRY: Entry = entry(tx!("hui.tree.constants"), Kind::Catalogue(Catalogue::Constants));

/// What the CONST key opens: the constants by category; a message outside Physik.
pub static CONSTANTS: Page = Page { title: tx!("hui.tree.constants"), entries: &[CONSTANTS_ENTRY] };

/// What STO and VARS open over.
pub static VARIABLES: Page = Page { title: tx!("hui.tree.variables"), entries: &[] };

/// A value converted to the unit chosen: `▶ unit`.
const CONVERSIONS_ENTRY: Entry = entry(tx!("hui.tree.unit"), Kind::Catalogue(Catalogue::Units));

/// What the CONV key opens: the units by category.
pub static CONVERSIONS: Page = Page { title: tx!("hui.tree.convert"), entries: &[CONVERSIONS_ENTRY] };

static OPTN_PHYSICS: Page = Page {
    title: tx!("hui.tree.physics"),
    entries: &[
        group(tx!("hui.tree.units"), &UNITS),
        group(tx!("hui.tree.prefixes"), &PREFIXES),
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

units!(LENGTH, tx!("hui.tree.length"), ["m", "Å", "AU", "ly"]);
units!(MASS, tx!("hui.tree.mass"), ["g", "t", "Da"]);
units!(TIME, tx!("hui.tree.time_units"), ["s", "min", "h", "d", "wk", "mo", "yr"]);
units!(CURRENT, tx!("hui.tree.current"), ["A"]);
units!(TEMPERATURE, tx!("hui.tree.temperature"), ["K", "°C"]);
units!(FORCE, tx!("hui.tree.force"), ["N"]);
units!(PRESSURE, tx!("hui.tree.pressure"), ["Pa", "bar", "atm", "Torr"]);
units!(ENERGY, tx!("hui.tree.energy"), ["J", "eV", "cal", "Wh"]);
units!(POWER, tx!("hui.tree.power"), ["W"]);
units!(CHARGE, tx!("hui.tree.charge"), ["C"]);
units!(VOLTAGE, tx!("hui.tree.voltage"), ["V"]);
units!(RESISTANCE, tx!("hui.tree.resistance"), ["Ω"]);
units!(CAPACITANCE, tx!("hui.tree.capacitance"), ["F"]);
units!(AREA_VOLUME, tx!("hui.tree.area_volume"), ["ha", "l"]);
units!(SPEED, tx!("hui.tree.speed"), ["mph"]);

static UNITS: Page = Page {
    title: tx!("hui.tree.units"),
    entries: &[
        group(tx!("hui.tree.length"), &LENGTH),
        group(tx!("hui.tree.mass"), &MASS),
        group(tx!("hui.tree.time_units"), &TIME),
        group(tx!("hui.tree.current"), &CURRENT),
        group(tx!("hui.tree.temperature"), &TEMPERATURE),
        group(tx!("hui.tree.force"), &FORCE),
        group(tx!("hui.tree.pressure"), &PRESSURE),
        group(tx!("hui.tree.energy"), &ENERGY),
        group(tx!("hui.tree.power"), &POWER),
        group(tx!("hui.tree.charge"), &CHARGE),
        group(tx!("hui.tree.voltage"), &VOLTAGE),
        group(tx!("hui.tree.resistance"), &RESISTANCE),
        group(tx!("hui.tree.capacitance"), &CAPACITANCE),
        group(tx!("hui.tree.area_volume"), &AREA_VOLUME),
        group(tx!("hui.tree.speed"), &SPEED),
    ],
};

/// All 24 prefixes, q to Q; micro as `\mu`, since `u` is the dalton.
static PREFIXES: Page = Page {
    title: tx!("hui.tree.prefixes"),
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
        ins(tx!("hui.tree.prefix.hecto"), "h"),
        ins(tx!("hui.tree.prefix.deca"), "da"),
        ins(tx!("hui.tree.prefix.deci"), "d"),
        ins(tx!("hui.tree.prefix.centi"), "c"),
        ins(same("m  milli"), "m"),
        ins(tx!("hui.tree.prefix.micro"), r"\mu "),
        ins(same("n  nano"), "n"),
        ins(tx!("hui.tree.prefix.pico"), "p"),
        ins(same("f  femto"), "f"),
        ins(same("a  atto"), "a"),
        ins(same("z  zepto"), "z"),
        ins(tx!("hui.tree.prefix.yocto"), "y"),
        ins(same("r  ronto"), "r"),
        ins(tx!("hui.tree.prefix.quecto"), "q"),
    ],
};

static OPTN_ALGEBRA: Page = Page {
    title: same("Algebra"),
    entries: &[
        group(tx!("hui.tree.system_equations"), &SYSTEM),
        GREEK_GROUP,
        ANALYSIS_GROUP,
        NUMBER_GROUP,
        HYPERBOLIC_GROUP,
        TOOLS_GROUP,
    ],
};

static SYSTEM: Page = Page {
    title: tx!("hui.tree.system_equations"),
    entries: &[ins(tx!("hui.tree.between_equations"), ";"), pending(same("cases"))],
};

static OPTN_COMPLEX: Page = Page {
    title: tx!("hui.tree.complex"),
    entries: &[
        ins(tx!("hui.tree.argument"), r"\arg()"),
        pending(tx!("hui.tree.conjugate")),
        ins(tx!("hui.tree.real_part"), r"\Re()"),
        ins(tx!("hui.tree.imaginary_part"), r"\Im()"),
        ins(tx!("hui.tree.absolute_value"), "||"),
        pending(tx!("hui.tree.polar_form")),
        pending(tx!("hui.tree.bi")),
        pending(tx!("hui.tree.angle_input")),
        ANALYSIS_GROUP,
        NUMBER_GROUP,
    ],
};

/// The pages of the last sketch, and the forms of the three tasks on the function typed.
static OPTN_ANALYSIS: Page = Page {
    title: tx!("hui.tree.curve_sketching"),
    entries: &[
        entry(tx!("hui.tree.sections"), Kind::Sections),
        entry(tx!("hui.tree.tangent_normal"), Kind::Do(Action::TangentForm)),
        entry(tx!("hui.tree.area"), Kind::Do(Action::AreaForm)),
        entry(tx!("hui.tree.table"), Kind::Do(Action::PointsForm)),
        ANALYSIS_GROUP,
        NUMBER_GROUP,
        HYPERBOLIC_GROUP,
    ],
};

/// f(x), g(x), von, bis and Schritt are the rows of the form; OPTN clears it and brings the
/// functions.
static OPTN_TABLE: Page = Page {
    title: tx!("hui.tree.table"),
    entries: &[
        entry(CLEAR_ALL, Kind::Do(Action::ClearInput)),
        NUMBER_GROUP,
        HYPERBOLIC_GROUP,
    ],
};

const CLEAR_ALL: Text = tx!("hui.tree.clear_all");

static OPTN_LINEAR: Page = Page {
    title: tx!("hui.tree.linear_systems_page"),
    entries: &[
        entry(
            tx!("hui.tree.input"),
            Kind::Choice(S::SystemInput, &[opt(tx!("hui.tree.equations"), 0), opt(same("Matrix [A|b]"), 1)]),
        ),
        entry(tx!("hui.tree.rows"), Kind::Number(S::SystemRows, 1, 10)),
        entry(tx!("hui.tree.columns"), Kind::Number(S::SystemColumns, 1, 10)),
        switch(tx!("hui.tree.imaginary_unit_i"), S::LinearComplex),
        entry(CLEAR_ALL, Kind::Do(Action::ClearInput)),
    ],
};

/// A matrix is defined under a name in a grid of the size set here, and edited in VARS; the
/// last result is the Ans key's.
static OPTN_MATRIX: Page = Page {
    title: tx!("hui.tree.matrices"),
    entries: &[
        entry(tx!("hui.tree.define"), Kind::Do(Action::DefineMatrix)),
        entry(tx!("hui.tree.edit"), Kind::Do(Action::EditMatrix)),
        entry(tx!("hui.tree.rows"), Kind::Number(S::MatrixRows, 1, 10)),
        entry(tx!("hui.tree.columns"), Kind::Number(S::MatrixColumns, 1, 10)),
        ins(tx!("hui.tree.determinant"), r"\det "),
        ins(tx!("hui.tree.transpose"), "^{T}"),
        ins(tx!("hui.tree.inverse"), "^{-1}"),
        ins(tx!("hui.tree.identity"), r"\operatorname{Identity}()"),
        ins(same("REF"), r"\operatorname{ref}()"),
        ins(same("RREF"), r"\operatorname{rref}()"),
    ],
};

/// The base numbers are typed and shown in, a prefix for one number in another, and the
/// bitwise logic; A–F are letters.
static OPTN_BASE_N: Page = Page {
    title: tx!("hui.base_n"),
    entries: &[
        entry(
            tx!("hui.tree.base"),
            Kind::Choice(S::NumberBase, &[opt(same("DEC"), 0), opt(same("HEX"), 1), opt(same("BIN"), 2), opt(same("OCT"), 3)]),
        ),
        group(tx!("hui.tree.prefixes_d_h_b_o"), &BASE_PREFIXES),
        ins(same("and"), r"\operatorname{and}"),
        ins(same("or"), r"\operatorname{or}"),
        ins(same("xor"), r"\operatorname{xor}"),
        ins(same("xnor"), r"\operatorname{xnor}"),
        ins(same("Not"), r"\operatorname{Not}()"),
        ins(same("Neg"), r"\operatorname{Neg}()"),
    ],
};

static BASE_PREFIXES: Page = Page {
    title: tx!("hui.tree.prefixes"),
    entries: &[
        ins(tx!("hui.tree.base.decimal"), r"\mathrm{d}"),
        ins(tx!("hui.tree.base.hexadecimal"), r"\mathrm{h}"),
        ins(tx!("hui.tree.base.binary"), r"\mathrm{b}"),
        ins(tx!("hui.tree.base.octal"), r"\mathrm{o}"),
    ],
};

/// The sums, the measures, min, max, the quartiles and the regression are all on the answer
/// EXE brings.
static OPTN_STATISTICS: Page = Page {
    title: tx!("hui.tree.statistics"),
    entries: &[
        entry(tx!("hui.tree.type"), Kind::Choice(S::StatisticsType, &STATISTICS_TYPES)),
        switch(tx!("hui.tree.frequency"), S::StatisticsFreq),
        entry(tx!("hui.tree.insert_row"), Kind::Do(Action::InsertRow)),
        entry(tx!("hui.tree.delete_row"), Kind::Do(Action::DeleteRow)),
        entry(CLEAR_ALL, Kind::Do(Action::ClearInput)),
        entry(tx!("hui.tree.estimates"), Kind::Do(Action::Estimates)),
    ],
};

static STATISTICS_TYPES: [Opt; 8] = [
    opt(tx!("hui.tree.one_variable"), 0),
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
    title: tx!("hui.tree.distributions"),
    entries: &[
        entry(tx!("hui.tree.type"), Kind::Choice(S::Distribution, &DISTRIBUTIONS)),
        entry(CLEAR_ALL, Kind::Do(Action::ClearInput)),
        GREEK_GROUP,
    ],
};

static DISTRIBUTIONS: [Opt; 7] = [
    opt(tx!("hui.tree.normal_pd"), 0),
    opt(tx!("hui.tree.normal_cd"), 1),
    opt(tx!("hui.tree.inverse_normal"), 2),
    opt(tx!("hui.tree.binomial_pd"), 3),
    opt(tx!("hui.tree.binomial_cd"), 4),
    opt(tx!("hui.tree.poisson_pd"), 5),
    opt(tx!("hui.tree.poisson_cd"), 6),
];

static OPTN_VERIFY: Page = Page {
    title: tx!("hui.tree.verify"),
    entries: &[
        ins(same("="), "="),
        ins(same("≠"), r"\neq "),
        ins(same("<"), "<"),
        ins(same(">"), ">"),
        ins(same("≤"), r"\le "),
        ins(same("≥"), r"\ge "),
    ],
};

// The soft keys (design/handheld-menus.md §7.4.1): the mode's OPTN commands, one press each.
// Physik's units and the result forms are made by `crate::soft`.

/// A soft key: its short label, LaTeX as the strip draws it, and what it does, as OPTN does it.
#[derive(Debug, PartialEq, Eq)]
pub struct Soft {
    pub label: Text,
    pub entry: Entry,
}

const fn soft(label: Text, entry: Entry) -> Soft {
    Soft { label, entry }
}

/// A soft key that inserts `latex`, labelled with `label`.
const fn soft_ins(label: &'static str, latex: &'static str) -> Soft {
    soft(same(label), ins(same(label), latex))
}

/// What a soft key's command opens over: a message or a list, which closes back to the input.
pub static SOFT_PAGE: Page = Page { title: same(""), entries: &[] };

/// The soft keys of a mode while its input is typed; Physik's are its units.
pub fn soft_keys(mode: Mode) -> &'static [Soft] {
    match mode {
        Mode::Algebra => &SOFT_ALGEBRA,
        Mode::Imaginary => &SOFT_COMPLEX,
        Mode::Analysis => &SOFT_ANALYSIS,
        Mode::Matrix => &SOFT_MATRIX,
        Mode::BaseN => &SOFT_BASE_N,
        Mode::Verify => &SOFT_VERIFY,
        _ => &[],
    }
}

pub static SOFT_ALGEBRA: [Soft; 12] = [
    soft_ins(";", ";"),
    soft_ins("∫", r"\int_{}^{}"),
    soft_ins("d/dx", r"\frac{d}{dx}"),
    soft_ins("Σ", r"\sum_{}^{}"),
    soft_ins("Π", r"\prod_{}^{}"),
    soft_ins("lim", r"\lim_{x\to}"),
    soft_ins("nCr", r"\binom{}{}"),
    soft_ins("|x|", "||"),
    soft_ins("x!", "!"),
    soft(tx!("hui.tree.gcd"), ins(tx!("hui.tree.gcd"), r"\operatorname{ggT}(;)")),
    soft(tx!("hui.tree.lcm"), ins(tx!("hui.tree.lcm"), r"\operatorname{kgV}(;)")),
    soft_ins("mod", r"\bmod "),
];

/// Komplex: the parts of a number, then the forms a result is shown in (they wait for the engine).
pub static SOFT_COMPLEX: [Soft; 8] = [
    soft_ins("arg", r"\arg()"),
    soft(tx!("hui.soft.conjugate"), pending(tx!("hui.tree.conjugate"))),
    soft_ins("Re", r"\Re()"),
    soft_ins("Im", r"\Im()"),
    soft_ins("|z|", "||"),
    soft(same(r"\blacktriangleright r\angle\theta"), pending(tx!("hui.tree.polar_form"))),
    soft(same(r"\blacktriangleright a+bi"), pending(tx!("hui.tree.bi"))),
    soft(same(r"\angle"), pending(tx!("hui.tree.angle_input"))),
];

/// Kurvendiskussion: the pages of the last sketch and the three tasks, then two templates.
pub static SOFT_ANALYSIS: [Soft; 6] = [
    soft(tx!("hui.soft.sections"), entry(tx!("hui.tree.sections"), Kind::Sections)),
    soft(tx!("hui.soft.tangent"), entry(tx!("hui.tree.tangent_normal"), Kind::Do(Action::TangentForm))),
    soft(tx!("hui.tree.area"), entry(tx!("hui.tree.area"), Kind::Do(Action::AreaForm))),
    soft(tx!("hui.soft.table"), entry(tx!("hui.tree.table"), Kind::Do(Action::PointsForm))),
    soft_ins("d/dx", r"\frac{d}{dx}"),
    soft_ins("∫", r"\int_{}^{}"),
];

pub static SOFT_MATRIX: [Soft; 8] = [
    soft(tx!("hui.soft.define"), entry(tx!("hui.tree.define"), Kind::Do(Action::DefineMatrix))),
    soft(tx!("hui.soft.edit"), entry(tx!("hui.tree.edit"), Kind::Do(Action::EditMatrix))),
    soft_ins("det", r"\det "),
    soft(same("A^{T}"), ins(tx!("hui.tree.transpose"), "^{T}")),
    soft(same("A^{-1}"), ins(tx!("hui.tree.inverse"), "^{-1}")),
    soft(tx!("hui.soft.identity"), ins(tx!("hui.tree.identity"), r"\operatorname{Identity}()")),
    soft_ins("REF", r"\operatorname{ref}()"),
    soft_ins("RREF", r"\operatorname{rref}()"),
];

/// Basis-N: the logic; the four bases come first, from `crate::soft`.
pub static SOFT_BASE_N: [Soft; 6] = [
    soft_ins("and", r"\operatorname{and}"),
    soft_ins("or", r"\operatorname{or}"),
    soft_ins("xor", r"\operatorname{xor}"),
    soft_ins("xnor", r"\operatorname{xnor}"),
    soft_ins("Not", r"\operatorname{Not}()"),
    soft_ins("Neg", r"\operatorname{Neg}()"),
];

pub static SOFT_VERIFY: [Soft; 6] = [
    soft_ins("=", "="),
    soft_ins("≠", r"\neq "),
    soft_ins("<", "<"),
    soft_ins(">", ">"),
    soft_ins("≤", r"\le "),
    soft_ins("≥", r"\ge "),
];
