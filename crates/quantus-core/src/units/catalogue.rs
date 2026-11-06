//! The units a conversion `value \blacktriangleright unit` can aim at, by the categories of a
//! Casio fx-991DE X's CONV menu, for a menu or a picker. Only units the engine knows are listed.

use serde::{Deserialize, Serialize};

use crate::settings::Language;
use quantus_texts::{texts, Text};

/// A category with its target units.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UnitCategory {
    pub name: String,
    pub units: Vec<UnitInfo>,
}

/// A target unit: what to insert after `\blacktriangleright`, and its written-out name.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UnitInfo {
    pub latex: String,
    pub name: String,
}

/// A unit of the catalogue with both names, for the search, which matches the English name in
/// every language.
pub(crate) struct Entry {
    pub category: Text,
    pub latex: &'static str,
    pub name: Text,
}

/// Every unit of the catalogue in its order; a unit in two categories comes twice.
pub(crate) fn entries() -> impl Iterator<Item = Entry> {
    CATEGORIES.iter().flat_map(|(category, units)| units.iter().map(|(latex, name)| Entry { category: *category, latex, name: *name }))
}

#[rustfmt::skip]
const CATEGORIES: [(Text, &[(&str, Text)]); 19] = [
    (Text(texts!("unit.category.length")), &[(r"\mathrm{m}", Text(texts!("unit.metre"))), (r"\mathrm{km}", Text(texts!("unit.kilometre"))), (r"\mathrm{cm}", Text(texts!("unit.centimetre"))), (r"\mathrm{mm}", Text(texts!("unit.millimetre"))), (r"\mathrm{fm}", Text(texts!("unit.femtometre"))), (r"\mathrm{in}", Text(texts!("unit.inch"))), (r"\mathrm{ft}", Text(texts!("unit.foot"))), (r"\mathrm{yd}", Text(texts!("unit.yard"))), (r"\mathrm{mi}", Text(texts!("unit.mile"))), (r"\mathrm{nmi}", Text(texts!("unit.nautical_mile"))), (r"\mathrm{pc}", Text(texts!("unit.parsec"))), (r"\mathrm{ly}", Text(texts!("unit.light_year"))), (r"\mathrm{AU}", Text(texts!("unit.astronomical_unit"))), (r"\mathrm{Å}", Text(texts!("unit.ngstr_m"))), (r"\mathrm{ch}", Text(texts!("unit.chain"))), (r"\mathrm{mil}", Text(texts!("unit.mil"))), (r"\mathrm{fath}", Text(texts!("unit.fathom")))]),
    (Text(texts!("unit.category.area")), &[(r"\mathrm{m}^{2}", Text(texts!("unit.square_metre"))), (r"\mathrm{km}^{2}", Text(texts!("unit.square_kilometre"))), (r"\mathrm{cm}^{2}", Text(texts!("unit.square_centimetre"))), (r"\mathrm{ha}", Text(texts!("unit.hectare"))), (r"\mathrm{acre}", Text(texts!("unit.acre"))), (r"\mathrm{ft}^{2}", Text(texts!("unit.square_foot"))), (r"\mathrm{in}^{2}", Text(texts!("unit.square_inch"))), (r"\mathrm{mi}^{2}", Text(texts!("unit.square_mile")))]),
    (Text(texts!("unit.category.volume")), &[(r"\mathrm{m}^{3}", Text(texts!("unit.cubic_metre"))), (r"\mathrm{l}", Text(texts!("unit.litre"))), (r"\mathrm{ml}", Text(texts!("unit.millilitre"))), (r"\mathrm{gal}", Text(texts!("unit.gallon"))), (r"\mathrm{bu}", Text(texts!("unit.bushel"))), (r"\mathrm{bbl}", Text(texts!("unit.barrel"))), (r"\mathrm{ft}^{3}", Text(texts!("unit.cubic_foot"))), (r"\mathrm{in}^{3}", Text(texts!("unit.cubic_inch")))]),
    (Text(texts!("unit.category.angle")), &[(r"\mathrm{rad}", Text(texts!("unit.radian")))]),
    (Text(texts!("unit.category.mass")), &[(r"\mathrm{kg}", Text(texts!("unit.kilogram"))), (r"\mathrm{g}", Text(texts!("unit.gram"))), (r"\mathrm{t}", Text(texts!("unit.tonne"))), (r"\mathrm{lb}", Text(texts!("unit.pound"))), (r"\mathrm{oz}", Text(texts!("unit.ounce"))), (r"\mathrm{ton}", Text(texts!("unit.short_ton"))), (r"\mathrm{ct}", Text(texts!("unit.carat"))), (r"\mathrm{Da}", Text(texts!("unit.dalton")))]),
    (Text(texts!("unit.category.time")), &[(r"\mathrm{s}", Text(texts!("unit.second"))), (r"\mathrm{min}", Text(texts!("unit.minute"))), (r"\mathrm{h}", Text(texts!("unit.hour"))), (r"\mathrm{d}", Text(texts!("unit.day"))), (r"\mathrm{yr}", Text(texts!("unit.year")))]),
    (Text(texts!("unit.category.speed")), &[(r"\frac{\mathrm{m}}{\mathrm{s}}", Text(texts!("unit.metre_per_second"))), (r"\frac{\mathrm{km}}{\mathrm{h}}", Text(texts!("unit.kilometre_per_hour"))), (r"\mathrm{mph}", Text(texts!("unit.mile_per_hour"))), (r"\mathrm{kn}", Text(texts!("unit.knot")))]),
    (Text(texts!("unit.category.acceleration")), &[(r"\frac{\mathrm{m}}{\mathrm{s}^{2}}", Text(texts!("unit.metre_per_second_squared"))), (r"\mathrm{Gal}", Text(texts!("unit.gal")))]),
    (Text(texts!("unit.category.torque")), &[(r"\mathrm{N} \cdot \mathrm{m}", Text(texts!("unit.newton_metre"))), (r"\mathrm{dyn} \cdot \mathrm{cm}", Text(texts!("unit.dyne_centimetre")))]),
    (Text(texts!("unit.category.force")), &[(r"\mathrm{N}", Text(texts!("unit.newton"))), (r"\mathrm{dyn}", Text(texts!("unit.dyne"))), (r"\mathrm{lbf}", Text(texts!("unit.pound_force"))), (r"\mathrm{kgf}", Text(texts!("unit.kilogram_force")))]),
    (Text(texts!("unit.category.pressure")), &[(r"\mathrm{Pa}", Text(texts!("unit.pascal"))), (r"\mathrm{kPa}", Text(texts!("unit.kilopascal"))), (r"\mathrm{bar}", Text(texts!("unit.bar"))), (r"\mathrm{atm}", Text(texts!("unit.atmosphere"))), (r"\mathrm{mmHg}", Text(texts!("unit.millimetre_mercury"))), (r"\mathrm{inHg}", Text(texts!("unit.inch_mercury"))), (r"\mathrm{psi}", Text(texts!("unit.pound_per_square_inch"))), (r"\mathrm{Torr}", Text(texts!("unit.torr"))), (r"\frac{\mathrm{kgf}}{\mathrm{cm}^{2}}", Text(texts!("unit.kilogram_force_per_square_centimetre"))), (r"\frac{\mathrm{dyn}}{\mathrm{cm}^{2}}", Text(texts!("unit.dyne_per_square_centimetre")))]),
    (Text(texts!("unit.category.energy")), &[(r"\mathrm{J}", Text(texts!("unit.joule"))), (r"\mathrm{kJ}", Text(texts!("unit.kilojoule"))), (r"\mathrm{kWh}", Text(texts!("unit.kilowatt_hour"))), (r"\mathrm{eV}", Text(texts!("unit.electronvolt"))), (r"\mathrm{cal}", Text(texts!("unit.calorie"))), (r"\mathrm{kcal}", Text(texts!("unit.kilocalorie"))), (r"\mathrm{erg}", Text(texts!("unit.erg"))), (r"\mathrm{Btu}", Text(texts!("unit.british_thermal_unit"))), (r"\mathrm{kgf} \cdot \mathrm{m}", Text(texts!("unit.kilogram_force_metre")))]),
    (Text(texts!("unit.category.power")), &[(r"\mathrm{W}", Text(texts!("unit.watt"))), (r"\mathrm{kW}", Text(texts!("unit.kilowatt"))), (r"\mathrm{hp}", Text(texts!("unit.horsepower")))]),
    (Text(texts!("unit.category.heat_flow")), &[(r"\mathrm{W}", Text(texts!("unit.watt"))), (r"\frac{\mathrm{Btu}}{\mathrm{min}}", Text(texts!("unit.btu_per_minute")))]),
    (Text(texts!("unit.category.temperature")), &[(r"\mathrm{K}", Text(texts!("unit.kelvin"))), (r"^\circ\mathrm{C}", Text(texts!("unit.degree_celsius"))), (r"^\circ\mathrm{F}", Text(texts!("unit.degree_fahrenheit")))]),
    (Text(texts!("unit.category.specific_heat")), &[(r"\frac{\mathrm{J}}{\mathrm{kg} \cdot \mathrm{K}}", Text(texts!("unit.joule_per_kilogram_kelvin"))), (r"\frac{\mathrm{cal}}{\mathrm{g} \cdot \mathrm{K}}", Text(texts!("unit.calorie_per_gram_kelvin")))]),
    (Text(texts!("unit.category.kinematic_viscosity")), &[(r"\frac{\mathrm{m}^{2}}{\mathrm{s}}", Text(texts!("unit.square_metre_per_second"))), (r"\mathrm{St}", Text(texts!("unit.stokes")))]),
    (Text(texts!("unit.category.magnetism")), &[(r"\mathrm{Wb}", Text(texts!("unit.weber"))), (r"\mathrm{Mx}", Text(texts!("unit.maxwell"))), (r"\mathrm{T}", Text(texts!("unit.tesla")))]),
    (Text(texts!("unit.category.radioactivity")), &[(r"\mathrm{Bq}", Text(texts!("unit.becquerel"))), (r"\mathrm{Ci}", Text(texts!("unit.curie"))), (r"\mathrm{Gy}", Text(texts!("unit.gray"))), (r"\mathrm{Sv}", Text(texts!("unit.sievert"))), (r"\mathrm{rem}", Text(texts!("unit.rem")))]),
];

/// The target units by category, named in `language`.
pub fn catalogue(language: Language) -> Vec<UnitCategory> {
    CATEGORIES
        .iter()
        .map(|(name, units)| UnitCategory {
            name: name.get(language).to_string(),
            units: units
                .iter()
                .map(|(latex, name)| UnitInfo {
                    latex: latex.to_string(),
                    name: name.get(language).to_string(),
                })
                .collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Mode, Settings};

    #[test]
    fn every_target_is_a_unit_the_arrow_takes() {
        let settings = Settings { mode: Mode::Physics, extra_unit_letters: true, ..Settings::default() };
        let calculator = crate::eval::LatexCalculator::new(10).with_settings(settings);
        for category in catalogue(Language::En) {
            for UnitInfo { latex: unit, .. } in &category.units {
                // Magnetism holds flux and flux density, so each unit converts only to itself here.
                let input = format!(r"1\,{unit} \blacktriangleright {unit}");
                let result = calculator.evaluate(&input, &std::collections::HashMap::new());
                assert!(result.is_ok(), "{}: {input}: {:?}", category.name, result.err());
            }
        }
    }

    #[test]
    fn every_unit_has_a_name_in_both_languages() {
        for entry in entries() {
            assert!(entry.name.0.iter().all(|name| !name.is_empty()), "{}", entry.latex);
        }
    }
}
