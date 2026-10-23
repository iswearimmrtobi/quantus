//! Atoms: the units a symbol can name, with or without a prefix.
//!
//! The set is what Indriya's `SimpleUnitFormat` registers and `UnitLookup` can reach: the seven
//! base units, the named SI units, gram, litre, degree celsius and the common time units; plus the
//! common units outside the SI (electronvolt, bar, calorie, watt hour, …). Everything else is a
//! product of these.

use std::sync::OnceLock;

use dashu::integer::{IBig, UBig};
use dashu::rational::RBig;

use super::conversions::AffineConversion;
use super::dimension::{BaseDim, Dimension};
use super::unit::Unit;

/// The unprefixed units.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    // base units
    Metre,
    Kilogram,
    Second,
    Ampere,
    Kelvin,
    Mole,
    Candela,
    // named SI units, defined by Indriya as alternates of a product
    Radian,
    Steradian,
    Hertz,
    Newton,
    Pascal,
    Joule,
    Watt,
    Coulomb,
    Volt,
    Farad,
    Ohm,
    Siemens,
    Weber,
    Tesla,
    Henry,
    Lumen,
    Lux,
    Becquerel,
    Gray,
    Sievert,
    Katal,
    // scaled or shifted units
    Gram,
    Litre,
    Celsius,
    Minute,
    Hour,
    Day,
    Week,
    Year,
    Month,
    // common units outside the SI, by their exact definitions (CODATA 2018 for eV and Da)
    Electronvolt,
    Atmosphere,
    Bar,
    Torr,
    Calorie,
    WattHour,
    Hectare,
    Angstrom,
    MilePerHour,
    LightYear,
    AstronomicalUnit,
    Dalton,
    Percent,
    // read as units only with `extra_unit_letters`, since the letters are common variables
    Tonne,
    Inch,
}

const KIND_COUNT: usize = Kind::Inch as usize + 1;

/// How a kind relates to the rest.
enum Origin {
    Base(BaseDim),
    /// Same dimension as this unit (alternate units and scaled units).
    Of(Unit),
}

impl Kind {
    /// The symbol Indriya gives the unit itself (celsius is `℃`, U+2103).
    pub fn symbol(self) -> &'static str {
        match self {
            Kind::Metre => "m",
            Kind::Kilogram => "kg",
            Kind::Second => "s",
            Kind::Ampere => "A",
            Kind::Kelvin => "K",
            Kind::Mole => "mol",
            Kind::Candela => "cd",
            Kind::Radian => "rad",
            Kind::Steradian => "sr",
            Kind::Hertz => "Hz",
            Kind::Newton => "N",
            Kind::Pascal => "Pa",
            Kind::Joule => "J",
            Kind::Watt => "W",
            Kind::Coulomb => "C",
            Kind::Volt => "V",
            Kind::Farad => "F",
            Kind::Ohm => "\u{3a9}",
            Kind::Siemens => "S",
            Kind::Weber => "Wb",
            Kind::Tesla => "T",
            Kind::Henry => "H",
            Kind::Lumen => "lm",
            Kind::Lux => "lx",
            Kind::Becquerel => "Bq",
            Kind::Gray => "Gy",
            Kind::Sievert => "Sv",
            Kind::Katal => "kat",
            Kind::Gram => "g",
            Kind::Litre => "l",
            Kind::Celsius => "\u{2103}",
            Kind::Minute => "min",
            Kind::Hour => "h",
            Kind::Day => "d",
            Kind::Week => "wk",
            Kind::Year => "yr",
            Kind::Month => "mo",
            Kind::Electronvolt => "eV",
            Kind::Atmosphere => "atm",
            Kind::Bar => "bar",
            Kind::Torr => "Torr",
            Kind::Calorie => "cal",
            Kind::WattHour => "Wh",
            Kind::Hectare => "ha",
            Kind::Angstrom => "\u{c5}",
            Kind::MilePerHour => "mph",
            Kind::LightYear => "ly",
            Kind::AstronomicalUnit => "AU",
            Kind::Dalton => "Da",
            Kind::Percent => "%",
            Kind::Tonne => "t",
            Kind::Inch => "in",
        }
    }

    fn unit(self) -> Unit {
        Unit::from(Atom::plain(self))
    }

    /// Mirrors the definitions in Indriya's `Units`; the order of the factors decides how the
    /// dimension prints (`N` is `m·kg/s²`, so `[L]·[M]/[T]²`).
    fn origin(self) -> Origin {
        let m = || Kind::Metre.unit();
        let s = || Kind::Second.unit();
        let one = Unit::ONE;
        match self {
            Kind::Metre => Origin::Base(BaseDim::Length),
            Kind::Kilogram => Origin::Base(BaseDim::Mass),
            Kind::Second => Origin::Base(BaseDim::Time),
            Kind::Ampere => Origin::Base(BaseDim::Current),
            Kind::Kelvin => Origin::Base(BaseDim::Temperature),
            Kind::Mole => Origin::Base(BaseDim::Amount),
            Kind::Candela => Origin::Base(BaseDim::Luminous),
            Kind::Radian | Kind::Steradian => Origin::Of(one),
            Kind::Hertz | Kind::Becquerel => Origin::Of(one.divide(&s())),
            Kind::Newton => Origin::Of(m().multiply(&Kind::Kilogram.unit()).divide(&s().pow(2))),
            Kind::Pascal => Origin::Of(Kind::Newton.unit().divide(&m().pow(2))),
            Kind::Joule => Origin::Of(Kind::Newton.unit().multiply(&m())),
            Kind::Watt => Origin::Of(Kind::Joule.unit().divide(&s())),
            Kind::Coulomb => Origin::Of(s().multiply(&Kind::Ampere.unit())),
            Kind::Volt => Origin::Of(Kind::Watt.unit().divide(&Kind::Ampere.unit())),
            Kind::Farad => Origin::Of(Kind::Coulomb.unit().divide(&Kind::Volt.unit())),
            Kind::Ohm => Origin::Of(Kind::Volt.unit().divide(&Kind::Ampere.unit())),
            Kind::Siemens => Origin::Of(Kind::Ampere.unit().divide(&Kind::Volt.unit())),
            Kind::Weber => Origin::Of(Kind::Volt.unit().multiply(&s())),
            Kind::Tesla => Origin::Of(Kind::Weber.unit().divide(&m().pow(2))),
            Kind::Henry => Origin::Of(Kind::Weber.unit().divide(&Kind::Ampere.unit())),
            Kind::Lumen => Origin::Of(Kind::Candela.unit().multiply(&Kind::Steradian.unit())),
            Kind::Lux => Origin::Of(Kind::Lumen.unit().divide(&m().pow(2))),
            Kind::Gray | Kind::Sievert => Origin::Of(Kind::Joule.unit().divide(&Kind::Kilogram.unit())),
            Kind::Katal => Origin::Of(Kind::Mole.unit().divide(&s())),
            Kind::Gram => Origin::Of(Kind::Kilogram.unit()),
            Kind::Litre => Origin::Of(m().pow(3)),
            Kind::Celsius => Origin::Of(Kind::Kelvin.unit()),
            Kind::Minute | Kind::Hour | Kind::Day | Kind::Week | Kind::Year | Kind::Month => Origin::Of(s()),
            Kind::Electronvolt | Kind::Calorie | Kind::WattHour => Origin::Of(Kind::Joule.unit()),
            Kind::Atmosphere | Kind::Bar | Kind::Torr => Origin::Of(Kind::Pascal.unit()),
            Kind::Hectare => Origin::Of(m().pow(2)),
            Kind::Angstrom | Kind::LightYear | Kind::AstronomicalUnit | Kind::Inch => Origin::Of(m()),
            Kind::MilePerHour => Origin::Of(m().divide(&s())),
            Kind::Dalton | Kind::Tonne => Origin::Of(Kind::Kilogram.unit()),
            Kind::Percent => Origin::Of(one),
        }
    }

    pub fn dimension(self) -> &'static Dimension {
        static DIMENSIONS: [OnceLock<Dimension>; KIND_COUNT] = [const { OnceLock::new() }; KIND_COUNT];
        DIMENSIONS[self as usize].get_or_init(|| match self.origin() {
            Origin::Base(base) => Dimension::base(base),
            Origin::Of(unit) => unit.dimension(),
        })
    }

    /// Exact conversion to the coherent system unit of the same dimension, e.g. minutes to seconds.
    fn system_conversion(self) -> AffineConversion {
        let (factor, offset) = match self {
            Kind::Gram | Kind::Litre => (rat(1, 1000), RBig::ZERO),
            // Indriya's AddConverter(273.15) as the exact decimal.
            Kind::Celsius => (RBig::ONE, rat(5463, 20)),
            Kind::Minute => (rat(60, 1), RBig::ZERO),
            Kind::Hour => (rat(3600, 1), RBig::ZERO),
            Kind::Day => (rat(86_400, 1), RBig::ZERO),
            Kind::Week => (rat(604_800, 1), RBig::ZERO),
            // 365.2425 days, the Gregorian year; a month is a twelfth of it.
            Kind::Year => (rat(31_556_952, 1), RBig::ZERO),
            Kind::Month => (rat(2_629_746, 1), RBig::ZERO),
            Kind::Electronvolt => (decimal("1.602176634e-19"), RBig::ZERO),
            Kind::Atmosphere => (rat(101_325, 1), RBig::ZERO),
            Kind::Bar => (rat(100_000, 1), RBig::ZERO),
            // 1 Torr is 1/760 of a standard atmosphere.
            Kind::Torr => (rat(101_325, 760), RBig::ZERO),
            // The thermochemical calorie.
            Kind::Calorie => (rat(4184, 1000), RBig::ZERO),
            Kind::WattHour => (rat(3600, 1), RBig::ZERO),
            Kind::Hectare => (rat(10_000, 1), RBig::ZERO),
            Kind::Angstrom => (decimal("1e-10"), RBig::ZERO),
            Kind::MilePerHour => (rat(44_704, 100_000), RBig::ZERO),
            // Julian year times c.
            Kind::LightYear => (rat(9_460_730_472_580_800, 1), RBig::ZERO),
            Kind::AstronomicalUnit => (rat(149_597_870_700, 1), RBig::ZERO),
            Kind::Dalton => (decimal("1.66053906660e-27"), RBig::ZERO),
            Kind::Percent => (rat(1, 100), RBig::ZERO),
            Kind::Tonne => (rat(1000, 1), RBig::ZERO),
            Kind::Inch => (rat(254, 10_000), RBig::ZERO),
            _ => (RBig::ONE, RBig::ZERO),
        };
        AffineConversion { factor, offset }
    }
}

fn decimal(text: &str) -> RBig {
    crate::num::Decimal::parse(text).expect("a valid decimal").to_rational()
}

fn rat(numerator: i64, denominator: u64) -> RBig {
    RBig::from_parts(IBig::from(numerator), UBig::from(denominator))
}

/// A metric (power of ten) or binary (power of two) prefix.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Prefix {
    pub symbol: &'static str,
    base: u8,
    exponent: i8,
}

const fn prefix(symbol: &'static str, base: u8, exponent: i8) -> Prefix {
    Prefix { symbol, base, exponent }
}

impl Prefix {
    /// In the order of Indriya's `MetricPrefix` (the micro sign is U+00B5).
    pub const METRIC: [Prefix; 24] = [
        prefix("Q", 10, 30),
        prefix("R", 10, 27),
        prefix("Y", 10, 24),
        prefix("Z", 10, 21),
        prefix("E", 10, 18),
        prefix("P", 10, 15),
        prefix("T", 10, 12),
        prefix("G", 10, 9),
        prefix("M", 10, 6),
        prefix("k", 10, 3),
        prefix("h", 10, 2),
        prefix("da", 10, 1),
        prefix("d", 10, -1),
        prefix("c", 10, -2),
        prefix("m", 10, -3),
        prefix("\u{b5}", 10, -6),
        prefix("n", 10, -9),
        prefix("p", 10, -12),
        prefix("f", 10, -15),
        prefix("a", 10, -18),
        prefix("z", 10, -21),
        prefix("y", 10, -24),
        prefix("r", 10, -27),
        prefix("q", 10, -30),
    ];

    /// In the order of Indriya's `BinaryPrefix`.
    pub const BINARY: [Prefix; 8] = [
        prefix("Ki", 2, 10),
        prefix("Mi", 2, 20),
        prefix("Gi", 2, 30),
        prefix("Ti", 2, 40),
        prefix("Pi", 2, 50),
        prefix("Ei", 2, 60),
        prefix("Zi", 2, 70),
        prefix("Yi", 2, 80),
    ];

    pub const MICRO: Prefix = Prefix::METRIC[15];
    pub const KILO: Prefix = Prefix::METRIC[9];

    pub fn factor(self) -> RBig {
        let base = UBig::from(self.base);
        let power = base.pow(self.exponent.unsigned_abs() as usize);
        if self.exponent >= 0 {
            RBig::from(IBig::from(power))
        } else {
            RBig::from_parts(IBig::ONE, power)
        }
    }
}

/// A unit symbol's meaning: a kind, optionally with a prefix (`km`, `mN`, `µm`).
///
/// Equality is structural like Indriya's `TransformedUnit.equals` (same parent, same converter),
/// so `kg` (the base unit) is not the same atom as a kilo-prefixed gram.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Atom {
    kind: Kind,
    prefix: Option<Prefix>,
}

impl Atom {
    pub fn plain(kind: Kind) -> Atom {
        Atom { kind, prefix: None }
    }

    pub fn prefixed(kind: Kind, prefix: Prefix) -> Atom {
        Atom { kind, prefix: Some(prefix) }
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    pub fn prefix(&self) -> Option<Prefix> {
        self.prefix
    }

    pub fn dimension(&self) -> &'static Dimension {
        self.kind.dimension()
    }

    /// Exact conversion to the system unit. A prefix scales the argument first, the kind's own
    /// conversion comes after: `k°C` is `1000·x + 273.15` kelvin.
    pub fn system_conversion(&self) -> AffineConversion {
        let own = self.kind.system_conversion();
        match self.prefix {
            None => own,
            Some(prefix) => AffineConversion {
                factor: own.factor * prefix.factor(),
                offset: own.offset,
            },
        }
    }
}
