//! Resolving a bare unit symbol: the table behind F2's uniqueness rule.
//!
//! Indriya's `SimpleUnitFormat` answers this with two hash maps filled at start-up
//! (`nameToUnit` for parsing, `unitToName` for printing); the same maps are rebuilt here in the
//! same order, so a later registration overrides an earlier one exactly as it does there. That
//! is where the known ambiguities are decided: `mN` is millinewton, `min` the minute, `Pa` the
//! pascal. Nothing is split at lookup time, so `m·i·n` or `P·a` never come up.
//!
//! Only plain symbols (letters and `°`) are accepted. `m2`, `m²`, `km/h` and `℃` exist in
//! Indriya's table too but are rejected up front, as `UnitLookup` does; composition is the
//! parser's job. Units Indriya registers that are not atoms (`km/h`, `m²` with prefixes,
//! percent) are left out for the same reason.

use std::collections::HashMap;
use std::sync::OnceLock;

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

#[derive(Default)]
struct Tables {
    /// Symbol to unit (`nameToUnit`): labels and aliases.
    names: HashMap<String, Unit>,
    /// Atom to its preferred symbol (`unitToName`): the last label registered.
    labels: HashMap<Atom, String>,
}

impl Tables {
    fn label(&mut self, atom: Atom, label: &str) {
        self.names.insert(label.to_string(), Unit::from(atom));
        self.labels.insert(atom, label.to_string());
    }

    fn alias(&mut self, unit: Unit, alias: &str) {
        self.names.insert(alias.to_string(), unit);
    }

    fn alias_atom(&mut self, atom: Atom, alias: &str) {
        self.alias(Unit::from(atom), alias);
    }

    fn label_with_prefixes(&mut self, kind: Kind, label: &str) {
        self.label(Atom::plain(kind), label);
        for prefix in Prefix::METRIC.iter().chain(&Prefix::BINARY) {
            self.label(Atom::prefixed(kind, *prefix), &format!("{}{label}", prefix.symbol));
        }
    }

    fn alias_with_prefixes(&mut self, kind: Kind, alias: &str) {
        self.alias_atom(Atom::plain(kind), alias);
        for prefix in Prefix::METRIC.iter().chain(&Prefix::BINARY) {
            self.alias_atom(Atom::prefixed(kind, *prefix), &format!("{}{alias}", prefix.symbol));
        }
    }

    /// `DefaultFormat.init()`, minus the registrations that are not atoms.
    fn build() -> Tables {
        let mut t = Tables::default();

        for kind in METRIC_UNITS {
            let symbol = kind.symbol();
            t.label(Atom::plain(kind), symbol);
            for prefix in Prefix::METRIC {
                let atom = Atom::prefixed(kind, prefix);
                t.label(atom, &format!("{}{symbol}", prefix.symbol));
                if prefix == Prefix::MICRO {
                    t.label(atom, &format!("{GREEK_MU}{symbol}"));
                }
            }
        }

        let gram = Atom::plain(Kind::Gram);
        t.label(gram, "g");
        for prefix in Prefix::METRIC {
            if prefix == Prefix::KILO {
                t.label(Atom::plain(Kind::Kilogram), "kg");
            } else {
                t.label(Atom::prefixed(Kind::Gram, prefix), &format!("{}g", prefix.symbol));
            }
        }
        // `symbolToUnit`: the Greek spelling of microgram has no label of its own.
        t.alias_atom(Atom::prefixed(Kind::Gram, Prefix::MICRO), &format!("{GREEK_MU}g"));

        t.alias_with_prefixes(Kind::Ohm, "Ohm");
        t.label_with_prefixes(Kind::Celsius, Kind::Celsius.symbol());
        t.alias_with_prefixes(Kind::Celsius, "°C");

        t.names.insert("one".to_string(), Unit::ONE);

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
        t
    }
}

fn tables() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(Tables::build)
}

fn is_plain_symbol(symbol: &str) -> bool {
    symbol == "%" || !symbol.is_empty() && symbol.chars().all(|c| c.is_alphabetic() || c == '°')
}

/// The unit for `symbol`, or `None` if it is not a known unit.
pub fn find(symbol: &str) -> Option<Unit> {
    if !is_plain_symbol(symbol) {
        return None;
    }
    tables().names.get(symbol).cloned()
}

/// The symbol `SimpleUnitFormat` prints for an atom, if it registered one.
pub fn label(atom: &Atom) -> Option<&'static str> {
    tables().labels.get(atom).map(String::as_str)
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
