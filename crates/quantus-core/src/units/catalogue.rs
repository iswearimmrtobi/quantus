//! The units a conversion `value \blacktriangleright unit` can aim at, by the categories of a
//! Casio fx-991DE X's CONV menu, for a menu or a picker. Only units the engine knows are listed.

use serde::{Deserialize, Serialize};

use crate::settings::Language;

/// A category with its target units.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UnitCategory {
    pub name: String,
    /// What to insert after `\blacktriangleright`.
    pub units: Vec<String>,
}

#[rustfmt::skip]
const CATEGORIES: [(&str, &str, &[&str]); 19] = [
    ("Length", "Länge", &[r"\mathrm{m}", r"\mathrm{km}", r"\mathrm{cm}", r"\mathrm{mm}", r"\mathrm{fm}", r"\mathrm{in}", r"\mathrm{ft}", r"\mathrm{yd}", r"\mathrm{mi}", r"\mathrm{nmi}", r"\mathrm{pc}", r"\mathrm{ly}", r"\mathrm{AU}", r"\mathrm{Å}", r"\mathrm{ch}", r"\mathrm{mil}", r"\mathrm{fath}"]),
    ("Area", "Fläche", &[r"\mathrm{m}^{2}", r"\mathrm{km}^{2}", r"\mathrm{cm}^{2}", r"\mathrm{ha}", r"\mathrm{acre}", r"\mathrm{ft}^{2}", r"\mathrm{in}^{2}", r"\mathrm{mi}^{2}"]),
    ("Volume", "Volumen", &[r"\mathrm{m}^{3}", r"\mathrm{l}", r"\mathrm{ml}", r"\mathrm{gal}", r"\mathrm{bu}", r"\mathrm{bbl}", r"\mathrm{ft}^{3}", r"\mathrm{in}^{3}"]),
    ("Angle", "Winkel", &[r"\mathrm{rad}"]),
    ("Mass", "Masse", &[r"\mathrm{kg}", r"\mathrm{g}", r"\mathrm{t}", r"\mathrm{lb}", r"\mathrm{oz}", r"\mathrm{ton}", r"\mathrm{ct}", r"\mathrm{Da}"]),
    ("Time", "Zeit", &[r"\mathrm{s}", r"\mathrm{min}", r"\mathrm{h}", r"\mathrm{d}", r"\mathrm{yr}"]),
    ("Speed", "Geschwindigkeit", &[r"\frac{\mathrm{m}}{\mathrm{s}}", r"\frac{\mathrm{km}}{\mathrm{h}}", r"\mathrm{mph}", r"\mathrm{kn}"]),
    ("Acceleration", "Beschleunigung", &[r"\frac{\mathrm{m}}{\mathrm{s}^{2}}", r"\mathrm{Gal}"]),
    ("Torque", "Drehmoment", &[r"\mathrm{N} \cdot \mathrm{m}", r"\mathrm{dyn} \cdot \mathrm{cm}"]),
    ("Force", "Kraft", &[r"\mathrm{N}", r"\mathrm{dyn}", r"\mathrm{lbf}", r"\mathrm{kgf}"]),
    ("Pressure", "Druck", &[r"\mathrm{Pa}", r"\mathrm{kPa}", r"\mathrm{bar}", r"\mathrm{atm}", r"\mathrm{mmHg}", r"\mathrm{inHg}", r"\mathrm{psi}", r"\mathrm{Torr}", r"\frac{\mathrm{kgf}}{\mathrm{cm}^{2}}", r"\frac{\mathrm{dyn}}{\mathrm{cm}^{2}}"]),
    ("Energy", "Energie", &[r"\mathrm{J}", r"\mathrm{kJ}", r"\mathrm{kWh}", r"\mathrm{eV}", r"\mathrm{cal}", r"\mathrm{kcal}", r"\mathrm{erg}", r"\mathrm{Btu}", r"\mathrm{kgf} \cdot \mathrm{m}"]),
    ("Power", "Leistung", &[r"\mathrm{W}", r"\mathrm{kW}", r"\mathrm{hp}"]),
    ("Heat flow", "Wärmedurchfluss", &[r"\mathrm{W}", r"\frac{\mathrm{Btu}}{\mathrm{min}}"]),
    ("Temperature", "Temperatur", &[r"\mathrm{K}", r"^\circ\mathrm{C}", r"^\circ\mathrm{F}"]),
    ("Specific heat", "Spezifische Wärme", &[r"\frac{\mathrm{J}}{\mathrm{kg} \cdot \mathrm{K}}", r"\frac{\mathrm{cal}}{\mathrm{g} \cdot \mathrm{K}}"]),
    ("Kinematic viscosity", "Kinematische Viskosität", &[r"\frac{\mathrm{m}^{2}}{\mathrm{s}}", r"\mathrm{St}"]),
    ("Magnetism", "Magnetismus", &[r"\mathrm{Wb}", r"\mathrm{Mx}", r"\mathrm{T}"]),
    ("Radioactivity", "Radioaktivität", &[r"\mathrm{Bq}", r"\mathrm{Ci}", r"\mathrm{Gy}", r"\mathrm{Sv}", r"\mathrm{rem}"]),
];

/// The target units by category, named in `language`.
pub fn catalogue(language: Language) -> Vec<UnitCategory> {
    CATEGORIES
        .iter()
        .map(|(en, de, units)| UnitCategory {
            name: if language == Language::De { de } else { en }.to_string(),
            units: units.iter().map(|unit| unit.to_string()).collect(),
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
            for unit in &category.units {
                // Magnetism holds flux and flux density, so each unit converts only to itself here.
                let input = format!(r"1\,{unit} \blacktriangleright {unit}");
                let result = calculator.evaluate(&input, &std::collections::HashMap::new());
                assert!(result.is_ok(), "{}: {input}: {:?}", category.name, result.err());
            }
        }
    }
}
