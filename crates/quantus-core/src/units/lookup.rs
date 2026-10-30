//! Resolving a bare unit symbol: the table behind F2's uniqueness rule.
//!
//! Indriya's `SimpleUnitFormat` answers this with two hash maps filled at start-up
//! (`nameToUnit` for parsing, `unitToName` for printing). Here the same registrations are
//! replayed in the same order on every lookup and the last match wins, so a later registration
//! overrides an earlier one exactly as it does there. That is where the known ambiguities are
//! decided: `mN` is millinewton, `min` the minute, `Pa` the pascal. Nothing is split at lookup
//! time, so `m·i·n` or `P·a` never come up.
//!
//! Only plain symbols (letters and `°`) are accepted. `m2`, `m²`, `km/h` and `℃` exist in
//! Indriya's table too but are rejected up front, as `UnitLookup` does; composition is the
//! parser's job. Units Indriya registers that are not atoms (`km/h`, `m²` with prefixes,
//! percent) are left out for the same reason.

use super::atom::{Atom, Kind, Prefix};
use super::unit::Unit;

/// Greek small letter mu, the second spelling Indriya accepts for the micro prefix.
const GREEK_MU: &str = "\u{3bc}";

/// The metric units that take every prefix, in the order `SimpleUnitFormat` registers them.
const METRIC_UNITS: [Kind; 27] = [
    Kind::Ampere,
    Kind::Becquerel,
    Kind::Candela,
    Kind::Coulomb,
    Kind::Farad,
    Kind::Gray,
    Kind::Henry,
    Kind::Hertz,
    Kind::Joule,
    Kind::Katal,
    Kind::Kelvin,
    Kind::Lumen,
    Kind::Lux,
    Kind::Metre,
    Kind::Mole,
    Kind::Newton,
    Kind::Ohm,
    Kind::Pascal,
    Kind::Radian,
    Kind::Second,
    Kind::Siemens,
    Kind::Sievert,
    Kind::Steradian,
    Kind::Tesla,
    Kind::Volt,
    Kind::Watt,
    Kind::Weber,
];

/// A registered symbol: `prefix` followed by `base`, kept apart so nothing is allocated.
#[derive(Clone, Copy)]
struct Symbol {
    prefix: &'static str,
    base: &'static str,
}

impl Symbol {
    fn prefixed(prefix: &'static str, base: &'static str) -> Symbol {
        Symbol { prefix, base }
    }

    fn is(self, text: &str) -> bool {
        text.strip_prefix(self.prefix) == Some(self.base)
    }
}

impl From<&'static str> for Symbol {
    fn from(base: &'static str) -> Symbol {
        Symbol { prefix: "", base }
    }
}

#[derive(Clone, Copy)]
enum Target {
    Atom(Atom),
    One,
}

/// One entry of `DefaultFormat.init()`. A label goes into both of Indriya's maps, an alias only
/// into `nameToUnit`.
struct Registration {
    symbol: Symbol,
    target: Target,
    label: bool,
}

/// Replays the registrations in order instead of storing them: the two maps hold about a
/// thousand strings, too much for a microcontroller, and a lookup that keeps the last match
/// gives the same answer as a map in which later insertions override earlier ones.
struct Registrar<F: FnMut(Registration)> {
    visit: F,
}

impl<F: FnMut(Registration)> Registrar<F> {
    fn label(&mut self, atom: Atom, symbol: impl Into<Symbol>) {
        (self.visit)(Registration { symbol: symbol.into(), target: Target::Atom(atom), label: true });
    }

    fn alias_atom(&mut self, atom: Atom, alias: impl Into<Symbol>) {
        (self.visit)(Registration { symbol: alias.into(), target: Target::Atom(atom), label: false });
    }

    fn label_with_prefixes(&mut self, kind: Kind, label: &'static str) {
        self.label(Atom::plain(kind), label);
        for prefix in Prefix::METRIC.iter().chain(&Prefix::BINARY) {
            self.label(Atom::prefixed(kind, *prefix), Symbol::prefixed(prefix.symbol, label));
        }
    }

    fn alias_with_prefixes(&mut self, kind: Kind, alias: &'static str) {
        self.alias_atom(Atom::plain(kind), alias);
        for prefix in Prefix::METRIC.iter().chain(&Prefix::BINARY) {
            self.alias_atom(Atom::prefixed(kind, *prefix), Symbol::prefixed(prefix.symbol, alias));
        }
    }

    /// `DefaultFormat.init()`, minus the registrations that are not atoms.
    fn register_all(mut self) {
        let t = &mut self;

        for kind in METRIC_UNITS {
            let symbol = kind.symbol();
            t.label(Atom::plain(kind), symbol);
            for prefix in Prefix::METRIC {
                let atom = Atom::prefixed(kind, prefix);
                t.label(atom, Symbol::prefixed(prefix.symbol, symbol));
                if prefix == Prefix::MICRO {
                    t.label(atom, Symbol::prefixed(GREEK_MU, symbol));
                }
            }
        }

        let gram = Atom::plain(Kind::Gram);
        t.label(gram, "g");
        for prefix in Prefix::METRIC {
            if prefix == Prefix::KILO {
                t.label(Atom::plain(Kind::Kilogram), "kg");
            } else {
                t.label(Atom::prefixed(Kind::Gram, prefix), Symbol::prefixed(prefix.symbol, "g"));
            }
        }
        // `symbolToUnit`: the Greek spelling of microgram has no label of its own.
        t.alias_atom(Atom::prefixed(Kind::Gram, Prefix::MICRO), Symbol::prefixed(GREEK_MU, "g"));

        t.alias_with_prefixes(Kind::Ohm, "Ohm");
        t.label_with_prefixes(Kind::Celsius, Kind::Celsius.symbol());
        t.alias_with_prefixes(Kind::Celsius, "°C");

        (t.visit)(Registration { symbol: "one".into(), target: Target::One, label: false });

        let plain = Atom::plain;
        t.label(plain(Kind::Minute), "min");
        t.label(plain(Kind::Hour), "h");
        t.label(plain(Kind::Day), "d");
        t.alias_atom(plain(Kind::Day), "day");
        t.label(plain(Kind::Week), "wk");
        t.alias_atom(plain(Kind::Week), "week");
        t.label(plain(Kind::Year), "yr");
        for alias in ["y", "year", "days365", "a"] {
            t.alias_atom(plain(Kind::Year), alias);
        }
        t.label(plain(Kind::Month), "mo");
        for alias in ["mon", "month"] {
            t.alias_atom(plain(Kind::Month), alias);
        }

        t.label_with_prefixes(Kind::Litre, "l");

        t.label_with_prefixes(Kind::Electronvolt, "eV");
        t.label(plain(Kind::Atmosphere), "atm");
        t.label_with_prefixes(Kind::Bar, "bar");
        t.label(plain(Kind::Torr), "Torr");
        t.label_with_prefixes(Kind::Calorie, "cal");
        t.label_with_prefixes(Kind::WattHour, "Wh");
        t.label(plain(Kind::Hectare), "ha");
        t.label(plain(Kind::Angstrom), "\u{c5}");
        t.label(plain(Kind::MilePerHour), "mph");
        t.label(plain(Kind::LightYear), "ly");
        t.label(plain(Kind::AstronomicalUnit), "AU");
        t.label(plain(Kind::Dalton), "Da");
        t.label(plain(Kind::Percent), "%");
        // The parser reads these three as variables unless `extra_unit_letters` is on.
        t.label(plain(Kind::Tonne), "t");
        t.alias_atom(plain(Kind::Dalton), "u");
        t.label(plain(Kind::Inch), "in");
    }
}

fn each_registration(visit: impl FnMut(Registration)) {
    Registrar { visit }.register_all();
}

fn is_plain_symbol(symbol: &str) -> bool {
    symbol == "%" || !symbol.is_empty() && symbol.chars().all(|c| c.is_alphabetic() || c == '°')
}

/// The unit for `symbol`, or `None` if it is not a known unit.
pub fn find(symbol: &str) -> Option<Unit> {
    if !is_plain_symbol(symbol) {
        return None;
    }
    let mut found = None;
    each_registration(|r| {
        if r.symbol.is(symbol) {
            found = Some(r.target);
        }
    });
    found.map(|target| match target {
        Target::Atom(atom) => Unit::from(atom),
        Target::One => Unit::ONE,
    })
}

/// The symbol `SimpleUnitFormat` prints for an atom, if it registered one.
pub fn label(atom: &Atom) -> Option<String> {
    let mut found = None;
    each_registration(|r| {
        if r.label && matches!(r.target, Target::Atom(a) if a == *atom) {
            found = Some(r.symbol);
        }
    });
    found.map(|symbol| format!("{}{}", symbol.prefix, symbol.base))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atom(kind: Kind) -> Unit {
        Unit::from(Atom::plain(kind))
    }

    fn prefixed(kind: Kind, symbol: &str) -> Unit {
        let prefix = Prefix::METRIC.iter().find(|p| p.symbol == symbol).unwrap();
        Unit::from(Atom::prefixed(kind, *prefix))
    }

    #[test]
    fn ambiguous_symbols_resolve_the_way_the_spec_wants() {
        assert_eq!(find("mN"), Some(prefixed(Kind::Newton, "m")), "mN is millinewton, never m·N");
        assert_eq!(find("min"), Some(atom(Kind::Minute)), "min is the minute, never m·i·n");
        assert_eq!(find("Pa"), Some(atom(Kind::Pascal)), "Pa is the pascal, never peta-year");
        assert_eq!(find("ms"), Some(prefixed(Kind::Second, "m")), "ms is millisecond, never m·s");
        assert_eq!(find("kg"), Some(atom(Kind::Kilogram)));
        assert_eq!(find("g"), Some(atom(Kind::Gram)));
        assert_eq!(find("h"), Some(atom(Kind::Hour)));
        assert_eq!(find("a"), Some(atom(Kind::Year)));
        assert_eq!(find("d"), Some(atom(Kind::Day)));
    }

    #[test]
    fn composition_is_not_the_lookups_job() {
        for symbol in ["m2", "m²", "m/s", "m·s", "m s", "", "e", "i", "x", "℃", "km/h", "%%"] {
            assert_eq!(find(symbol), None, "{symbol:?}");
        }
    }

    #[test]
    fn both_micro_spellings_name_the_same_unit() {
        assert_eq!(find("\u{b5}m"), find("\u{3bc}m"));
        assert_eq!(find("\u{b5}g"), find("\u{3bc}g"));
        assert!(find("\u{b5}m").is_some());
    }

    #[test]
    fn spellings_longer_than_three_characters_follow_the_same_table() {
        assert_eq!(find("mmol"), Some(prefixed(Kind::Mole, "m")));
        assert_eq!(find("kOhm"), Some(prefixed(Kind::Ohm, "k")));
        assert_eq!(find("month"), Some(atom(Kind::Month)));
        assert_eq!(find("dacd"), Some(prefixed(Kind::Candela, "da")));
        assert_eq!(find("k°C"), Some(prefixed(Kind::Celsius, "k")));
        assert_eq!(find("one"), Some(Unit::ONE));
    }
}
