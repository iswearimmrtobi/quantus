//! Compares the Rust unit model with the dumps taken from Indriya (`tests/golden/units_*.json`).

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use dashu::rational::RBig;
use quantus_core::units::{find, latex, registry, Kind, Unit};
use serde_json::Value;

fn golden(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(name);
    serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap()
}

fn rational(text: &str) -> RBig {
    text.parse().unwrap_or_else(|_| panic!("not a rational: {text}"))
}

/// The dump tried every one to three character string over this alphabet.
fn alphabet() -> Vec<char> {
    ('a'..='z')
        .chain('A'..='Z')
        // micro sign, greek mu, greek capital omega, degree sign, degree celsius sign
        .chain(['\u{b5}', '\u{3bc}', '\u{3a9}', '°', '\u{2103}'])
        .collect()
}

fn candidates() -> Vec<String> {
    let alphabet = alphabet();
    let mut all: Vec<String> = alphabet.iter().map(|c| c.to_string()).collect();
    let mut previous = all.clone();
    for _ in 0..2 {
        previous = previous
            .iter()
            .flat_map(|prefix| alphabet.iter().map(move |c| format!("{prefix}{c}")))
            .collect();
        all.extend(previous.iter().cloned());
    }
    all
}

/// `{"[L]": "2"}` as sorted (symbol, exponent) pairs.
fn expected_vector(entry: &Value) -> Vec<(String, i32)> {
    let mut vector: Vec<_> = entry["dimensionVector"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), v.as_str().unwrap().parse().unwrap()))
        .collect();
    vector.sort();
    vector
}

fn actual_vector(unit: &Unit) -> Vec<(String, i32)> {
    let mut vector: Vec<_> = unit
        .dimension()
        .exponents()
        .into_iter()
        .map(|(base, pow)| (base.symbol().to_string(), pow))
        .collect();
    vector.sort();
    vector
}

/// The kinds Quantus adds to Indriya's units.
const EXTENSIONS: [Kind; 15] = [
    Kind::Electronvolt,
    Kind::Atmosphere,
    Kind::Bar,
    Kind::Torr,
    Kind::Calorie,
    Kind::WattHour,
    Kind::Hectare,
    Kind::Angstrom,
    Kind::MilePerHour,
    Kind::LightYear,
    Kind::AstronomicalUnit,
    Kind::Dalton,
    Kind::Percent,
    Kind::Tonne,
    Kind::Inch,
];

#[test]
fn lookup_matches_indriya_for_every_short_symbol() {
    let dump = golden("units_symbols.json");
    let entries = dump["symbols"].as_array().unwrap();
    let unknown_count = entries.last().unwrap()["unknownCount"].as_u64().unwrap() as usize;
    let known: HashMap<&str, &Value> = entries
        .iter()
        .filter_map(|e| e["symbol"].as_str().map(|s| (s, e)))
        .collect();
    assert_eq!(known.len(), 715);

    let mut unknown = 0;
    for symbol in candidates() {
        let found = find(&symbol);
        match known.get(symbol.as_str()) {
            None => {
                unknown += 1;
                // Units Quantus adds beyond Indriya's table (eV, bar, kWh, …) are the only exceptions.
                let extension = matches!(&found, Some(Unit::Atom(atom)) if EXTENSIONS.contains(&atom.kind()));
                assert!(found.is_none() || extension, "{symbol:?} should be unknown, found {found:?}");
            }
            Some(entry) => {
                let unit = found.unwrap_or_else(|| panic!("{symbol:?} should be known"));
                check_symbol(&symbol, entry, &unit);
            }
        }
    }
    // The dump also tried 18 longer spellings; 3 of them are known (checked below), 15 are not.
    assert_eq!(unknown_count, unknown + 15, "the alphabet differs from the dump's");

    for symbol in known.keys().filter(|s| s.chars().count() > 3) {
        let unit = find(symbol).unwrap_or_else(|| panic!("{symbol:?} should be known"));
        check_symbol(symbol, known[symbol], &unit);
    }
}

fn check_symbol(symbol: &str, entry: &Value, unit: &Unit) {
    // `one` is the empty product; the registry prints it as `1`, Indriya's raw format as `one`.
    if symbol != "one" {
        assert_eq!(unit.to_string(), entry["format"].as_str().unwrap(), "format of {symbol:?}");
    }
    assert_eq!(unit.dimension().to_string(), entry["dimension"].as_str().unwrap(), "dimension of {symbol:?}");
    assert_eq!(actual_vector(unit), expected_vector(entry), "dimension vector of {symbol:?}");
    let conversion = unit.system_conversion().unwrap();
    assert_eq!(conversion.factor, rational(entry["factor"].as_str().unwrap()), "factor of {symbol:?}");
    assert_eq!(conversion.offset, rational(entry["offset"].as_str().unwrap()), "offset of {symbol:?}");
}

/// The dump builds each unit from `[symbol, power, root]` factors multiplied in order from `1`.
fn build(entry: &Value) -> Unit {
    entry["factors"].as_array().unwrap().iter().fold(Unit::ONE, |acc, factor| {
        let symbol = factor[0].as_str().unwrap();
        let power = factor[1].as_i64().unwrap() as i32;
        let root = factor[2].as_i64().unwrap() as i32;
        let atom = find(symbol).unwrap_or_else(|| panic!("{symbol:?} should be known"));
        acc.multiply(&atom.pow(power).root(root).unwrap())
    })
}

fn optional(entry: &Value, key: &str) -> Option<String> {
    entry[key].as_str().map(str::to_string)
}

#[test]
fn format_registry_and_latex_match_indriya() {
    let dump = golden("units_format.json");
    let units = dump["units"].as_array().unwrap();
    assert_eq!(units.len(), 79);

    for entry in units {
        let spec = entry["spec"].as_str().unwrap();
        let unit = build(entry);

        assert_eq!(unit.is_product(), entry["isProduct"].as_bool().unwrap(), "{spec}: product or atom");
        assert_eq!(unit.to_string(), entry["format"].as_str().unwrap(), "{spec}: format");
        assert_eq!(latex::format(&unit), entry["latex"].as_str().unwrap(), "{spec}: latex");

        let dimension = unit.dimension();
        assert_eq!(dimension.to_string(), entry["dimension"].as_str().unwrap(), "{spec}: dimension");
        assert_eq!(actual_vector(&unit), expected_vector(entry), "{spec}: dimension vector");
        assert_eq!(dimension.is_none(), entry["isDimensionless"].as_bool().unwrap(), "{spec}: dimensionless");

        assert_eq!(registry::quantity_name(&dimension).map(str::to_string), optional(entry, "quantityName"), "{spec}: quantity name");
        assert_eq!(registry::describe(&unit), entry["describe"].as_str().unwrap(), "{spec}: describe");
        assert_eq!(registry::format(&unit), entry["format"].as_str().unwrap(), "{spec}: registry format");
        assert_eq!(
            registry::simplification_target(&unit).map(|u| u.to_string()),
            optional(entry, "simplificationTarget"),
            "{spec}: simplification target"
        );
        assert_eq!(
            registry::preferred_unit(&dimension).map(|u| u.to_string()),
            optional(entry, "preferredUnit"),
            "{spec}: preferred unit"
        );
    }
}
