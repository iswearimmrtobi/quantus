//! The lookup tables behind F1 and F11.
//!
//! Units do not rename themselves: `kg × m/s²` stays `kg·m/s²` and is explicitly *not* `N`.
//! The dimensions do match though, and that is the lever: dimension to preferred named unit,
//! then convert.
//!
//! Dimensions are **not unique** (`m/s` and `km/h` share one, as do Hz and Bq, or Gy and Sv),
//! hence a hand-curated, prioritised table: the first match in [`PREFERRED_UNITS`] wins.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::atom::{Atom, Kind};
use super::dimension::Dimension;
use super::unit::Unit;

/// The coherent SI units that carry a name of their own, in priority order.
///
/// Deliberately *only* named units: `m²` or `m/s` have no name of their own, so `km·km` stays
/// `km²` and `36 km/h` stays `km/h`. Simplification only happens where a real name exists
/// (F1: "kg·m/s² → N").
///
/// Conflicts this ordering decides:
/// - `1/s`: hertz (frequency) over becquerel (activity)
/// - `m²/s²`: gray (absorbed dose) over sievert (dose equivalent)
/// - `cd`: the base unit over lumen (cd·sr, dimensionally identical)
/// - `K`: kelvin over degree celsius (celsius carries an offset, unfit as a target form)
const PREFERRED_UNITS: [Kind; 23] = [
    // base units first
    Kind::Metre,
    Kind::Kilogram,
    Kind::Second,
    Kind::Ampere,
    Kind::Kelvin,
    Kind::Mole,
    Kind::Candela,
    // named derived units
    Kind::Newton,
    Kind::Pascal,
    Kind::Joule,
    Kind::Watt,
    Kind::Coulomb,
    Kind::Volt,
    Kind::Farad,
    Kind::Ohm,
    Kind::Siemens,
    Kind::Weber,
    Kind::Tesla,
    Kind::Henry,
    Kind::Lux,
    Kind::Hertz,
    Kind::Gray,
    Kind::Katal,
];

fn atom(kind: Kind) -> Unit {
    Unit::from(Atom::plain(kind))
}

fn preferred_by_dimension() -> &'static HashMap<Dimension, Unit> {
    static TABLE: OnceLock<HashMap<Dimension, Unit>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = HashMap::new();
        for kind in PREFERRED_UNITS {
            table.entry(kind.dimension().clone()).or_insert_with(|| atom(kind));
        }
        table
    })
}

/// F11: purely informational quantity names. Complements the unit table where no named SI unit
/// exists (`m/s` is "velocity"), and doubles as the source for readable error messages
/// ("m (length) and s (time) cannot be added").
///
/// Ambiguous dimensions (J = N·m = torque, Hz = Bq) carry the more common name; the first
/// entry for a dimension wins.
fn quantity_names() -> &'static HashMap<Dimension, &'static str> {
    static TABLE: OnceLock<HashMap<Dimension, &'static str>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let m = atom(Kind::Metre);
        let kg = atom(Kind::Kilogram);
        let s = atom(Kind::Second);
        let metre_per_second = m.divide(&s);
        let cubic_metre = m.pow(3);

        let names: [(Unit, &str); 32] = [
            (m.clone(), "length"),
            (m.pow(2), "area"),
            (cubic_metre.clone(), "volume"),
            (kg.clone(), "mass"),
            (s.clone(), "time"),
            (atom(Kind::Ampere), "electric current"),
            (atom(Kind::Kelvin), "temperature"),
            (atom(Kind::Mole), "amount of substance"),
            (atom(Kind::Candela), "luminous intensity"),
            (metre_per_second.clone(), "velocity"),
            (metre_per_second.divide(&s), "acceleration"),
            (kg.divide(&cubic_metre), "density"),
            (kg.multiply(&metre_per_second), "momentum"),
            (cubic_metre.divide(&s), "volumetric flow rate"),
            (atom(Kind::Hertz), "frequency"),
            (atom(Kind::Newton), "force"),
            (atom(Kind::Pascal), "pressure"),
            (atom(Kind::Joule), "energy"),
            (atom(Kind::Watt), "power"),
            (atom(Kind::Coulomb), "electric charge"),
            (atom(Kind::Volt), "voltage"),
            (atom(Kind::Farad), "capacitance"),
            (atom(Kind::Ohm), "resistance"),
            (atom(Kind::Siemens), "conductance"),
            (atom(Kind::Weber), "magnetic flux"),
            (atom(Kind::Tesla), "magnetic flux density"),
            (atom(Kind::Henry), "inductance"),
            (atom(Kind::Lux), "illuminance"),
            (atom(Kind::Gray), "absorbed dose"),
            (atom(Kind::Katal), "catalytic activity"),
            (atom(Kind::Joule).divide(&atom(Kind::Kelvin)), "heat capacity"),
            (
                atom(Kind::Joule).divide(&kg.multiply(&atom(Kind::Kelvin))),
                "specific heat capacity",
            ),
        ];

        // Dimensions are derived from units rather than written out by hand, so no dimensional
        // algebra can go wrong here.
        let mut table = HashMap::new();
        for (unit, name) in names {
            table.entry(unit.dimension()).or_insert(name);
        }
        table
    })
}

/// The preferred named unit for a dimension, if there is one.
pub fn preferred_unit(dimension: &Dimension) -> Option<Unit> {
    preferred_by_dimension().get(dimension).cloned()
}

/// Target unit for automatic simplification (F1), or `None` if `unit` should stay as it is.
///
/// Only what arithmetic produced gets simplified: a product such as `kg·m/s²`. Units the user
/// picked (`kJ`, `km/h`, `N`) are left alone: converting the result to a requested unit
/// ("give me kWh") is a separate, explicit operation.
pub fn simplification_target(unit: &Unit) -> Option<Unit> {
    if !unit.is_product() {
        return None;
    }
    let target = preferred_unit(&unit.dimension())
        .or_else(|| unit.is_compatible(&Unit::ONE).then_some(Unit::ONE))?;
    (target != *unit).then_some(target)
}

/// F11: the common physical name of the quantity, purely informational.
pub fn quantity_name(dimension: &Dimension) -> Option<&'static str> {
    quantity_names().get(dimension).copied()
}

/// Display form of a unit, for error messages and debug output (`1` when dimensionless).
pub fn format(unit: &Unit) -> String {
    unit.to_string()
}

/// Unit plus quantity name, e.g. `m/s² (acceleration)`, for error messages.
pub fn describe(unit: &Unit) -> String {
    match quantity_name(&unit.dimension()) {
        Some(name) => format!("{unit} ({name})"),
        None => unit.to_string(),
    }
}
