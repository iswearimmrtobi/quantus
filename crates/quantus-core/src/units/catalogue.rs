//! The units a conversion `value \blacktriangleright unit` can aim at, by the categories of a
//! Casio fx-991DE X's CONV menu, for a menu or a picker. Only units the engine knows are listed.

use serde::{Deserialize, Serialize};

use crate::settings::Language;

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
    pub category_en: &'static str,
    pub category_de: &'static str,
    pub latex: &'static str,
    pub en: &'static str,
    pub de: &'static str,
}

/// Every unit of the catalogue in its order; a unit in two categories comes twice.
pub(crate) fn entries() -> impl Iterator<Item = Entry> {
    CATEGORIES.iter().flat_map(|(category_en, category_de, units)| {
        units.iter().map(|(latex, en, de)| Entry { category_en, category_de, latex, en, de })
    })
}

#[rustfmt::skip]
const CATEGORIES: [(&str, &str, &[(&str, &str, &str)]); 19] = [
    ("Length", "Länge", &[(r"\mathrm{m}", "metre", "Meter"), (r"\mathrm{km}", "kilometre", "Kilometer"), (r"\mathrm{cm}", "centimetre", "Zentimeter"), (r"\mathrm{mm}", "millimetre", "Millimeter"), (r"\mathrm{fm}", "femtometre", "Femtometer"), (r"\mathrm{in}", "inch", "Zoll"), (r"\mathrm{ft}", "foot", "Fuß"), (r"\mathrm{yd}", "yard", "Yard"), (r"\mathrm{mi}", "mile", "Meile"), (r"\mathrm{nmi}", "nautical mile", "Seemeile"), (r"\mathrm{pc}", "parsec", "Parsec"), (r"\mathrm{ly}", "light-year", "Lichtjahr"), (r"\mathrm{AU}", "astronomical unit", "Astronomische Einheit"), (r"\mathrm{Å}", "ångström", "Ångström"), (r"\mathrm{ch}", "chain", "Chain"), (r"\mathrm{mil}", "mil", "Mil"), (r"\mathrm{fath}", "fathom", "Faden")]),
    ("Area", "Fläche", &[(r"\mathrm{m}^{2}", "square metre", "Quadratmeter"), (r"\mathrm{km}^{2}", "square kilometre", "Quadratkilometer"), (r"\mathrm{cm}^{2}", "square centimetre", "Quadratzentimeter"), (r"\mathrm{ha}", "hectare", "Hektar"), (r"\mathrm{acre}", "acre", "Acre"), (r"\mathrm{ft}^{2}", "square foot", "Quadratfuß"), (r"\mathrm{in}^{2}", "square inch", "Quadratzoll"), (r"\mathrm{mi}^{2}", "square mile", "Quadratmeile")]),
    ("Volume", "Volumen", &[(r"\mathrm{m}^{3}", "cubic metre", "Kubikmeter"), (r"\mathrm{l}", "litre", "Liter"), (r"\mathrm{ml}", "millilitre", "Milliliter"), (r"\mathrm{gal}", "gallon", "Gallone"), (r"\mathrm{bu}", "bushel", "Bushel"), (r"\mathrm{bbl}", "barrel", "Barrel"), (r"\mathrm{ft}^{3}", "cubic foot", "Kubikfuß"), (r"\mathrm{in}^{3}", "cubic inch", "Kubikzoll")]),
    ("Angle", "Winkel", &[(r"\mathrm{rad}", "radian", "Radiant")]),
    ("Mass", "Masse", &[(r"\mathrm{kg}", "kilogram", "Kilogramm"), (r"\mathrm{g}", "gram", "Gramm"), (r"\mathrm{t}", "tonne", "Tonne"), (r"\mathrm{lb}", "pound", "Pfund"), (r"\mathrm{oz}", "ounce", "Unze"), (r"\mathrm{ton}", "short ton", "Short ton"), (r"\mathrm{ct}", "carat", "Karat"), (r"\mathrm{Da}", "dalton", "Dalton")]),
    ("Time", "Zeit", &[(r"\mathrm{s}", "second", "Sekunde"), (r"\mathrm{min}", "minute", "Minute"), (r"\mathrm{h}", "hour", "Stunde"), (r"\mathrm{d}", "day", "Tag"), (r"\mathrm{yr}", "year", "Jahr")]),
    ("Speed", "Geschwindigkeit", &[(r"\frac{\mathrm{m}}{\mathrm{s}}", "metre per second", "Meter pro Sekunde"), (r"\frac{\mathrm{km}}{\mathrm{h}}", "kilometre per hour", "Kilometer pro Stunde"), (r"\mathrm{mph}", "mile per hour", "Meile pro Stunde"), (r"\mathrm{kn}", "knot", "Knoten")]),
    ("Acceleration", "Beschleunigung", &[(r"\frac{\mathrm{m}}{\mathrm{s}^{2}}", "metre per second squared", "Meter pro Quadratsekunde"), (r"\mathrm{Gal}", "gal", "Gal")]),
    ("Torque", "Drehmoment", &[(r"\mathrm{N} \cdot \mathrm{m}", "newton metre", "Newtonmeter"), (r"\mathrm{dyn} \cdot \mathrm{cm}", "dyne centimetre", "Dyn-Zentimeter")]),
    ("Force", "Kraft", &[(r"\mathrm{N}", "newton", "Newton"), (r"\mathrm{dyn}", "dyne", "Dyn"), (r"\mathrm{lbf}", "pound-force", "Pound-force"), (r"\mathrm{kgf}", "kilogram-force", "Kilopond")]),
    ("Pressure", "Druck", &[(r"\mathrm{Pa}", "pascal", "Pascal"), (r"\mathrm{kPa}", "kilopascal", "Kilopascal"), (r"\mathrm{bar}", "bar", "Bar"), (r"\mathrm{atm}", "atmosphere", "Atmosphäre"), (r"\mathrm{mmHg}", "millimetre of mercury", "Millimeter Quecksilbersäule"), (r"\mathrm{inHg}", "inch of mercury", "Zoll Quecksilbersäule"), (r"\mathrm{psi}", "pound per square inch", "Pfund pro Quadratzoll"), (r"\mathrm{Torr}", "torr", "Torr"), (r"\frac{\mathrm{kgf}}{\mathrm{cm}^{2}}", "kilogram-force per square centimetre", "Kilopond pro Quadratzentimeter"), (r"\frac{\mathrm{dyn}}{\mathrm{cm}^{2}}", "dyne per square centimetre", "Dyn pro Quadratzentimeter")]),
    ("Energy", "Energie", &[(r"\mathrm{J}", "joule", "Joule"), (r"\mathrm{kJ}", "kilojoule", "Kilojoule"), (r"\mathrm{kWh}", "kilowatt hour", "Kilowattstunde"), (r"\mathrm{eV}", "electronvolt", "Elektronenvolt"), (r"\mathrm{cal}", "calorie", "Kalorie"), (r"\mathrm{kcal}", "kilocalorie", "Kilokalorie"), (r"\mathrm{erg}", "erg", "Erg"), (r"\mathrm{Btu}", "British thermal unit", "British thermal unit"), (r"\mathrm{kgf} \cdot \mathrm{m}", "kilogram-force metre", "Kilopondmeter")]),
    ("Power", "Leistung", &[(r"\mathrm{W}", "watt", "Watt"), (r"\mathrm{kW}", "kilowatt", "Kilowatt"), (r"\mathrm{hp}", "horsepower", "Pferdestärke")]),
    ("Heat flow", "Wärmedurchfluss", &[(r"\mathrm{W}", "watt", "Watt"), (r"\frac{\mathrm{Btu}}{\mathrm{min}}", "BTU per minute", "BTU pro Minute")]),
    ("Temperature", "Temperatur", &[(r"\mathrm{K}", "kelvin", "Kelvin"), (r"^\circ\mathrm{C}", "degree Celsius", "Grad Celsius"), (r"^\circ\mathrm{F}", "degree Fahrenheit", "Grad Fahrenheit")]),
    ("Specific heat", "Spezifische Wärme", &[(r"\frac{\mathrm{J}}{\mathrm{kg} \cdot \mathrm{K}}", "joule per kilogram kelvin", "Joule pro Kilogramm und Kelvin"), (r"\frac{\mathrm{cal}}{\mathrm{g} \cdot \mathrm{K}}", "calorie per gram kelvin", "Kalorie pro Gramm und Kelvin")]),
    ("Kinematic viscosity", "Kinematische Viskosität", &[(r"\frac{\mathrm{m}^{2}}{\mathrm{s}}", "square metre per second", "Quadratmeter pro Sekunde"), (r"\mathrm{St}", "stokes", "Stokes")]),
    ("Magnetism", "Magnetismus", &[(r"\mathrm{Wb}", "weber", "Weber"), (r"\mathrm{Mx}", "maxwell", "Maxwell"), (r"\mathrm{T}", "tesla", "Tesla")]),
    ("Radioactivity", "Radioaktivität", &[(r"\mathrm{Bq}", "becquerel", "Becquerel"), (r"\mathrm{Ci}", "curie", "Curie"), (r"\mathrm{Gy}", "gray", "Gray"), (r"\mathrm{Sv}", "sievert", "Sievert"), (r"\mathrm{rem}", "rem", "Rem")]),
];

/// The target units by category, named in `language`.
pub fn catalogue(language: Language) -> Vec<UnitCategory> {
    CATEGORIES
        .iter()
        .map(|(en, de, units)| UnitCategory {
            name: if language == Language::De { de } else { en }.to_string(),
            units: units
                .iter()
                .map(|(latex, en, de)| UnitInfo {
                    latex: latex.to_string(),
                    name: if language == Language::De { de } else { en }.to_string(),
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
            assert!(!entry.en.is_empty() && !entry.de.is_empty(), "{}", entry.latex);
        }
    }
}
