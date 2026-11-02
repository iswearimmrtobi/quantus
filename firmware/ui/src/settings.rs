//! Everything SETUP (and OPTN in LGS) can change. The settings the engine has
//! (`quantus-core/src/settings.rs`) go to the backend with every calculation; the others are
//! kept here until the engine or the firmware can use them (design/handheld-menus.md §7.2 says
//! which is which).

use crate::text::Lang;

/// An enum of choices, with the list of them in menu order.
macro_rules! choice {
    ($(#[$meta:meta])* $name:ident { $($(#[$variant_meta:meta])* $variant:ident),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum $name {
            $($(#[$variant_meta])* $variant),+
        }

        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            fn index(self) -> i32 {
                Self::ALL.iter().position(|&value| value == self).unwrap_or(0) as i32
            }

            fn from_index(index: i32) -> Self {
                Self::ALL[(index.max(0) as usize).min(Self::ALL.len() - 1)]
            }
        }
    };
}

choice!(Angle { Degrees, Radians });
choice!(
    /// The Casio's number formats; Fix and Sci take [`Settings::format_digits`].
    NumberFormat { Norm1, Norm2, Fix, Sci }
);
choice!(
    /// d/c or ab/c.
    FractionForm { Improper, Mixed }
);
choice!(
    /// a+bi or r∠θ.
    ComplexForm { Cartesian, Polar }
);
choice!(QuadraticFormula { Auto, Pq, Abc });
choice!(StepDetail { None, Brief, Standard, Detailed, Maximal });
choice!(Elimination { Gauss, GaussJordan });
choice!(BinomialForm { Auto, Expanded, Binomial });
choice!(DenominatorForm { Auto, Expanded, Factored });
choice!(ConditionalIdentities { WithCondition, AlwaysValid });
choice!(UnknownNames { Indexed, Letters });
choice!(
    /// How a linear system is typed in LGS.
    SystemInput { Equations, Matrix }
);
choice!(FontSize { Normal, Small });
choice!(
    /// The time zones the clock knows, each with its summer time (`clock.rs`).
    Zone { Utc, London, Berlin, Helsinki, Moscow, Dubai, India, China, Tokyo, Sydney, NewYork, Chicago, Denver, LosAngeles }
);
choice!(
    /// Where the wallpaper shows: behind the mode tiles, also behind the menus, or also behind
    /// the calculation.
    WallpaperOn { Modes, Menus, Everywhere }
);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    // Allgemein
    pub language: Lang,
    pub angle: Angle,
    /// Significant digits of decimal results, 1 to [`Settings::MAX_DIGITS`].
    pub digits: u8,
    pub format: NumberFormat,
    /// The n of Fix n and Sci n, 0 to 9.
    pub format_digits: u8,
    pub prefixes: bool,
    pub grouping: bool,
    // Ergebnis
    /// Decimals instead of exact fractions and roots.
    pub decimal: bool,
    pub fraction: FractionForm,
    pub recurring: bool,
    pub complex: ComplexForm,
    pub quadratic_formula: QuadraticFormula,
    // Rechenweg
    pub step_detail: StepDetail,
    pub cancel_marks: bool,
    pub elimination: Elimination,
    // Algebra
    pub binomial_form: BinomialForm,
    pub complete_square: bool,
    pub denominator_form: DenominatorForm,
    pub conditional_identities: ConditionalIdentities,
    // Eingabe
    pub a_and_y_as_variables: bool,
    pub letter_products: bool,
    pub extra_unit_letters: bool,
    pub unknown_names: UnknownNames,
    // LGS (in its OPTN)
    pub linear_complex: bool,
    pub system_input: SystemInput,
    pub system_rows: u8,
    pub system_columns: u8,
    // Gerät
    pub brightness: u8,
    pub font_size: FontSize,
    pub wallpaper_on: WallpaperOn,
    /// Minutes until the handheld switches itself off.
    pub auto_off: u8,
    // Verbindungen
    pub exam: bool,
    pub wlan: bool,
    /// Hand calculations over 500 ms to a remote engine.
    pub remote: bool,
    pub sync_auto: bool,
    /// The clock from the network (NTP) instead of set by hand.
    pub time_from_network: bool,
    pub zone: Zone,
    pub bluetooth: bool,
    pub visible: bool,
}

impl Settings {
    pub const MAX_DIGITS: u8 = 15;
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            language: Lang::De,
            angle: Angle::Degrees,
            digits: 10,
            format: NumberFormat::Norm1,
            format_digits: 2,
            prefixes: false,
            grouping: false,
            decimal: false,
            fraction: FractionForm::Improper,
            recurring: false,
            complex: ComplexForm::Cartesian,
            quadratic_formula: QuadraticFormula::Auto,
            step_detail: StepDetail::Standard,
            cancel_marks: true,
            elimination: Elimination::Gauss,
            binomial_form: BinomialForm::Auto,
            complete_square: true,
            denominator_form: DenominatorForm::Auto,
            conditional_identities: ConditionalIdentities::WithCondition,
            a_and_y_as_variables: true,
            letter_products: true,
            extra_unit_letters: false,
            unknown_names: UnknownNames::Indexed,
            linear_complex: false,
            system_input: SystemInput::Equations,
            system_rows: 2,
            system_columns: 2,
            brightness: 8,
            font_size: FontSize::Normal,
            wallpaper_on: WallpaperOn::Modes,
            auto_off: 10,
            exam: false,
            wlan: false,
            remote: false,
            sync_auto: false,
            time_from_network: true,
            zone: Zone::Berlin,
            bluetooth: false,
            visible: false,
        }
    }
}

/// A setting, for the menus: its value is a number, an index into its choices or 0/1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Setting {
    Language,
    Angle,
    Digits,
    Format,
    FormatDigits,
    Prefixes,
    Grouping,
    Decimal,
    Fraction,
    Recurring,
    Complex,
    QuadraticFormula,
    StepDetail,
    CancelMarks,
    Elimination,
    BinomialForm,
    CompleteSquare,
    DenominatorForm,
    ConditionalIdentities,
    AAndYAsVariables,
    LetterProducts,
    ExtraUnitLetters,
    UnknownNames,
    LinearComplex,
    SystemInput,
    SystemRows,
    SystemColumns,
    Brightness,
    FontSize,
    WallpaperOn,
    AutoOff,
    Exam,
    Wlan,
    Remote,
    SyncAuto,
    TimeFromNetwork,
    Zone,
    Bluetooth,
    Visible,
}

impl Settings {
    pub fn get(&self, setting: Setting) -> i32 {
        use Setting as S;
        match setting {
            S::Language => self.language as i32,
            S::Angle => self.angle.index(),
            S::Digits => self.digits.into(),
            S::Format => self.format.index(),
            S::FormatDigits => self.format_digits.into(),
            S::Prefixes => self.prefixes.into(),
            S::Grouping => self.grouping.into(),
            S::Decimal => self.decimal.into(),
            S::Fraction => self.fraction.index(),
            S::Recurring => self.recurring.into(),
            S::Complex => self.complex.index(),
            S::QuadraticFormula => self.quadratic_formula.index(),
            S::StepDetail => self.step_detail.index(),
            S::CancelMarks => self.cancel_marks.into(),
            S::Elimination => self.elimination.index(),
            S::BinomialForm => self.binomial_form.index(),
            S::CompleteSquare => self.complete_square.into(),
            S::DenominatorForm => self.denominator_form.index(),
            S::ConditionalIdentities => self.conditional_identities.index(),
            S::AAndYAsVariables => self.a_and_y_as_variables.into(),
            S::LetterProducts => self.letter_products.into(),
            S::ExtraUnitLetters => self.extra_unit_letters.into(),
            S::UnknownNames => self.unknown_names.index(),
            S::LinearComplex => self.linear_complex.into(),
            S::SystemInput => self.system_input.index(),
            S::SystemRows => self.system_rows.into(),
            S::SystemColumns => self.system_columns.into(),
            S::Brightness => self.brightness.into(),
            S::FontSize => self.font_size.index(),
            S::WallpaperOn => self.wallpaper_on.index(),
            S::AutoOff => self.auto_off.into(),
            S::Exam => self.exam.into(),
            S::Wlan => self.wlan.into(),
            S::Remote => self.remote.into(),
            S::SyncAuto => self.sync_auto.into(),
            S::TimeFromNetwork => self.time_from_network.into(),
            S::Zone => self.zone.index(),
            S::Bluetooth => self.bluetooth.into(),
            S::Visible => self.visible.into(),
        }
    }

    /// Sets `setting` to `value`; the menus check the range first.
    pub fn set(&mut self, setting: Setting, value: i32) {
        use Setting as S;
        let on = value != 0;
        let small = value.clamp(0, i32::from(u8::MAX)) as u8;
        match setting {
            S::Language => self.language = if value == Lang::En as i32 { Lang::En } else { Lang::De },
            S::Angle => self.angle = Angle::from_index(value),
            S::Digits => self.digits = small.clamp(1, Self::MAX_DIGITS),
            S::Format => self.format = NumberFormat::from_index(value),
            S::FormatDigits => self.format_digits = small.min(9),
            S::Prefixes => self.prefixes = on,
            S::Grouping => self.grouping = on,
            S::Decimal => self.decimal = on,
            S::Fraction => self.fraction = FractionForm::from_index(value),
            S::Recurring => self.recurring = on,
            S::Complex => self.complex = ComplexForm::from_index(value),
            S::QuadraticFormula => self.quadratic_formula = QuadraticFormula::from_index(value),
            S::StepDetail => self.step_detail = StepDetail::from_index(value),
            S::CancelMarks => self.cancel_marks = on,
            S::Elimination => self.elimination = Elimination::from_index(value),
            S::BinomialForm => self.binomial_form = BinomialForm::from_index(value),
            S::CompleteSquare => self.complete_square = on,
            S::DenominatorForm => self.denominator_form = DenominatorForm::from_index(value),
            S::ConditionalIdentities => {
                self.conditional_identities = ConditionalIdentities::from_index(value)
            }
            S::AAndYAsVariables => self.a_and_y_as_variables = on,
            S::LetterProducts => self.letter_products = on,
            S::ExtraUnitLetters => self.extra_unit_letters = on,
            S::UnknownNames => self.unknown_names = UnknownNames::from_index(value),
            S::LinearComplex => self.linear_complex = on,
            S::SystemInput => self.system_input = SystemInput::from_index(value),
            S::SystemRows => self.system_rows = small,
            S::SystemColumns => self.system_columns = small,
            S::Brightness => self.brightness = small,
            S::FontSize => self.font_size = FontSize::from_index(value),
            S::WallpaperOn => self.wallpaper_on = WallpaperOn::from_index(value),
            S::AutoOff => self.auto_off = small,
            S::Exam => {
                self.exam = on;
                // The exam mode switches both radios off; the menus keep them off.
                if on {
                    self.wlan = false;
                    self.bluetooth = false;
                }
            }
            S::Wlan => self.wlan = on && !self.exam,
            S::Remote => self.remote = on,
            S::SyncAuto => self.sync_auto = on,
            S::TimeFromNetwork => self.time_from_network = on,
            S::Zone => self.zone = Zone::from_index(value),
            S::Bluetooth => self.bluetooth = on && !self.exam,
            S::Visible => self.visible = on,
        }
    }

    /// Whether the exam mode keeps `setting` from being switched on.
    pub fn exam_blocks(&self, setting: Setting) -> bool {
        self.exam && matches!(setting, Setting::Wlan | Setting::Bluetooth)
    }
}

/// The first line of [`Settings::to_text`]; a later format gets a new number.
const HEADER: &str = "quantus-settings 1";

/// Every setting with the key it is kept under. The exam mode comes before the radios, so a
/// saved exam mode keeps them off.
const KEYS: &[(&str, Setting)] = {
    use Setting as S;
    &[
        ("language", S::Language),
        ("angle", S::Angle),
        ("digits", S::Digits),
        ("format", S::Format),
        ("format_digits", S::FormatDigits),
        ("prefixes", S::Prefixes),
        ("grouping", S::Grouping),
        ("decimal", S::Decimal),
        ("fraction", S::Fraction),
        ("recurring", S::Recurring),
        ("complex", S::Complex),
        ("quadratic_formula", S::QuadraticFormula),
        ("step_detail", S::StepDetail),
        ("cancel_marks", S::CancelMarks),
        ("elimination", S::Elimination),
        ("binomial_form", S::BinomialForm),
        ("complete_square", S::CompleteSquare),
        ("denominator_form", S::DenominatorForm),
        ("conditional_identities", S::ConditionalIdentities),
        ("a_and_y_as_variables", S::AAndYAsVariables),
        ("letter_products", S::LetterProducts),
        ("extra_unit_letters", S::ExtraUnitLetters),
        ("unknown_names", S::UnknownNames),
        ("linear_complex", S::LinearComplex),
        ("system_input", S::SystemInput),
        ("system_rows", S::SystemRows),
        ("system_columns", S::SystemColumns),
        ("brightness", S::Brightness),
        ("font_size", S::FontSize),
        ("wallpaper_on", S::WallpaperOn),
        ("auto_off", S::AutoOff),
        ("exam", S::Exam),
        ("wlan", S::Wlan),
        ("remote", S::Remote),
        ("sync_auto", S::SyncAuto),
        ("time_from_network", S::TimeFromNetwork),
        ("zone", S::Zone),
        ("bluetooth", S::Bluetooth),
        ("visible", S::Visible),
    ]
};

impl Settings {
    /// The settings as lines of `key=value`, for the board's NVS and the simulator's SD folder.
    pub fn to_text(&self) -> String {
        let mut text = format!("{HEADER}\n");
        for &(key, setting) in KEYS {
            text += &format!("{key}={}\n", self.get(setting));
        }
        text
    }

    /// The settings [`to_text`](Settings::to_text) wrote. A key that is missing, unknown or not
    /// a number keeps its default, so an older or newer file still loads.
    pub fn from_text(text: &str) -> Settings {
        let mut settings = Settings::default();
        let mut lines = text.lines();
        if lines.next().map(str::trim) != Some(HEADER) {
            return settings;
        }
        for line in lines {
            let Some((key, value)) = line.split_once('=') else { continue };
            let Some(&(_, setting)) = KEYS.iter().find(|(name, _)| *name == key.trim()) else { continue };
            if let Ok(value) = value.trim().parse() {
                settings.set(setting, value);
            }
        }
        settings
    }
}
