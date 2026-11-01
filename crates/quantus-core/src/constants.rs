//! The physical constants (CODATA 2018): the 47 of the Casio fx-991DE X, in its six categories.
//! They are known in Physics mode (and the circuit modes) by their key; a session variable of
//! the same name wins. [`catalogue`] lists them for a CONST menu or a picker.

use serde::{Deserialize, Serialize};

use crate::latex::numbers;
use crate::num::{Decimal, Value};
use crate::quantity::PhysicalQuantity;
use crate::settings::Language;
use crate::units::{latex as unit_latex, Unit};

/// The Casio's categories.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Category {
    Universal,
    Electromagnetic,
    AtomicNuclear,
    PhysicoChemical,
    Adopted,
    Other,
}

impl Category {
    const ALL: [Category; 6] = [
        Category::Universal,
        Category::Electromagnetic,
        Category::AtomicNuclear,
        Category::PhysicoChemical,
        Category::Adopted,
        Category::Other,
    ];

    fn name(self, language: Language) -> &'static str {
        let (en, de) = match self {
            Category::Universal => ("Universal", "Universelle Konstanten"),
            Category::Electromagnetic => ("Electromagnetic", "Elektromagnetische Konstanten"),
            Category::AtomicNuclear => ("Atomic and nuclear", "Atom- und Kernkonstanten"),
            Category::PhysicoChemical => ("Physico-chemical", "Physikalisch-chemische Konstanten"),
            Category::Adopted => ("Adopted values", "Übernommene Werte"),
            Category::Other => ("Other", "Andere"),
        };
        if language == Language::De { de } else { en }
    }
}

/// One constant: how it is typed, its value and unit (a product of unit-table atoms written
/// `a*b/c/d`), and its names.
struct Constant {
    /// The session key, as the parser builds it from `latex`.
    key: &'static str,
    /// What a menu inserts.
    latex: &'static str,
    value: &'static str,
    unit: &'static str,
    category: Category,
    en: &'static str,
    de: &'static str,
}

const fn c(
    key: &'static str,
    latex: &'static str,
    value: &'static str,
    unit: &'static str,
    category: Category,
    en: &'static str,
    de: &'static str,
) -> Constant {
    Constant { key, latex, value, unit, category, en, de }
}

use Category::*;

/// h/2π, computed rather than rounded.
const HBAR: &str = "\\hbar";

#[rustfmt::skip]
const CONSTANTS: [Constant; 47] = [
    c("h_P", "h_{P}", "6.62607015e-34", "J*s", Universal, "Planck constant", "Planck-Konstante"),
    c(HBAR, "\\hbar", "", "J*s", Universal, "reduced Planck constant", "reduzierte Planck-Konstante"),
    c("c_0", "c_{0}", "299792458", "m/s", Universal, "speed of light in vacuum", "Lichtgeschwindigkeit im Vakuum"),
    c("\\varepsilon_0", "\\varepsilon_{0}", "8.8541878128e-12", "F/m", Universal, "vacuum permittivity", "elektrische Feldkonstante"),
    c("\\mu_0", "\\mu_{0}", "1.25663706212e-6", "N/A/A", Universal, "vacuum permeability", "magnetische Feldkonstante"),
    c("Z_0", "Z_{0}", "376.730313668", "Ohm", Universal, "characteristic impedance of vacuum", "Wellenwiderstand des Vakuums"),
    c("G_N", "G_{N}", "6.67430e-11", "m*m*m/kg/s/s", Universal, "gravitational constant", "Gravitationskonstante"),
    c("l_P", "l_{P}", "1.616255e-35", "m", Universal, "Planck length", "Planck-Länge"),
    c("t_P", "t_{P}", "5.391247e-44", "s", Universal, "Planck time", "Planck-Zeit"),
    c("\\mu_N", "\\mu_{N}", "5.0507837461e-27", "J/T", Electromagnetic, "nuclear magneton", "Kernmagneton"),
    c("\\mu_B", "\\mu_{B}", "9.2740100783e-24", "J/T", Electromagnetic, "Bohr magneton", "Bohrsches Magneton"),
    c("e_0", "e_{0}", "1.602176634e-19", "C", Electromagnetic, "elementary charge", "Elementarladung"),
    c("\\Phi_0", "\\Phi_{0}", "2.067833848e-15", "Wb", Electromagnetic, "magnetic flux quantum", "magnetisches Flussquant"),
    c("G_0", "G_{0}", "7.748091729e-5", "S", Electromagnetic, "conductance quantum", "Leitwertquant"),
    c("K_J", "K_{J}", "483597.8484e9", "Hz/V", Electromagnetic, "Josephson constant", "Josephson-Konstante"),
    c("R_K", "R_{K}", "25812.80745", "Ohm", Electromagnetic, "von Klitzing constant", "von-Klitzing-Konstante"),
    c("m_p", "m_{p}", "1.67262192369e-27", "kg", AtomicNuclear, "proton mass", "Protonenmasse"),
    c("m_n", "m_{n}", "1.67492749804e-27", "kg", AtomicNuclear, "neutron mass", "Neutronenmasse"),
    c("m_e", "m_{e}", "9.1093837015e-31", "kg", AtomicNuclear, "electron mass", "Elektronenmasse"),
    c("m_\\mu", "m_{\\mu}", "1.883531627e-28", "kg", AtomicNuclear, "muon mass", "Myonmasse"),
    c("a_0", "a_{0}", "5.29177210903e-11", "m", AtomicNuclear, "Bohr radius", "Bohrscher Radius"),
    c("\\alpha_f", "\\alpha_{f}", "7.2973525693e-3", "", AtomicNuclear, "fine-structure constant", "Feinstrukturkonstante"),
    c("r_e", "r_{e}", "2.8179403262e-15", "m", AtomicNuclear, "classical electron radius", "klassischer Elektronenradius"),
    c("\\lambda_C", "\\lambda_{C}", "2.42631023867e-12", "m", AtomicNuclear, "Compton wavelength", "Compton-Wellenlänge"),
    c("\\gamma_p", "\\gamma_{p}", "2.6752218744e8", "/s/T", AtomicNuclear, "proton gyromagnetic ratio", "gyromagnetisches Verhältnis des Protons"),
    c("\\lambda_Cp", "\\lambda_{Cp}", "1.32140985539e-15", "m", AtomicNuclear, "proton Compton wavelength", "Compton-Wellenlänge des Protons"),
    c("\\lambda_Cn", "\\lambda_{Cn}", "1.31959090581e-15", "m", AtomicNuclear, "neutron Compton wavelength", "Compton-Wellenlänge des Neutrons"),
    c("R_inf", "R_{inf}", "10973731.568160", "/m", AtomicNuclear, "Rydberg constant", "Rydberg-Konstante"),
    c("\\mu_p", "\\mu_{p}", "1.41060679736e-26", "J/T", AtomicNuclear, "proton magnetic moment", "magnetisches Moment des Protons"),
    c("\\mu_e", "\\mu_{e}", "-9.2847647043e-24", "J/T", AtomicNuclear, "electron magnetic moment", "magnetisches Moment des Elektrons"),
    c("\\mu_n", "\\mu_{n}", "-9.6623651e-27", "J/T", AtomicNuclear, "neutron magnetic moment", "magnetisches Moment des Neutrons"),
    c("\\mu_\\mu", "\\mu_{\\mu}", "-4.49044830e-26", "J/T", AtomicNuclear, "muon magnetic moment", "magnetisches Moment des Myons"),
    c("m_\\tau", "m_{\\tau}", "3.16754e-27", "kg", AtomicNuclear, "tau mass", "Taumasse"),
    c("m_u", "m_{u}", "1.66053906660e-27", "kg", PhysicoChemical, "atomic mass constant", "atomare Masseneinheit"),
    c("F_Fa", "F_{Fa}", "96485.33212", "C/mol", PhysicoChemical, "Faraday constant", "Faraday-Konstante"),
    c("N_A", "N_{A}", "6.02214076e23", "/mol", PhysicoChemical, "Avogadro constant", "Avogadro-Konstante"),
    c("k_B", "k_{B}", "1.380649e-23", "J/K", PhysicoChemical, "Boltzmann constant", "Boltzmann-Konstante"),
    c("V_m", "V_{m}", "22.41396954e-3", "m*m*m/mol", PhysicoChemical, "molar volume of an ideal gas (273,15 K, 101,325 kPa)", "molares Volumen eines idealen Gases (273,15 K, 101,325 kPa)"),
    // N_A k_B, exactly
    c("R_m", "R_{m}", "8.31446261815324", "J/mol/K", PhysicoChemical, "molar gas constant", "universelle Gaskonstante"),
    c("c_1", "c_{1}", "3.741771852e-16", "W*m*m", PhysicoChemical, "first radiation constant", "erste Strahlungskonstante"),
    c("c_2", "c_{2}", "1.438776877e-2", "m*K", PhysicoChemical, "second radiation constant", "zweite Strahlungskonstante"),
    c("\\sigma_SB", "\\sigma_{SB}", "5.670374419e-8", "W/m/m/K/K/K/K", PhysicoChemical, "Stefan–Boltzmann constant", "Stefan-Boltzmann-Konstante"),
    c("g_n", "g_{n}", "9.80665", "m/s/s", Adopted, "standard acceleration of gravity", "Normfallbeschleunigung"),
    c("p_atm", "p_{atm}", "101325", "Pa", Adopted, "standard atmosphere", "Normatmosphäre"),
    c("R_K90", "R_{K90}", "25812.807", "Ohm", Adopted, "conventional von Klitzing constant (1990)", "konventionelle von-Klitzing-Konstante (1990)"),
    c("K_J90", "K_{J90}", "483597.9e9", "Hz/V", Adopted, "conventional Josephson constant (1990)", "konventionelle Josephson-Konstante (1990)"),
    c("T_0", "T_{0}", "273.15", "K", Other, "0 °C in kelvin", "0 °C in Kelvin"),
];

/// Spellings that name the same constant.
const ALIASES: [(&str, &str); 1] = [("\\epsilon_0", "\\varepsilon_0")];

/// The constant of that key as a quantity.
pub fn find(key: &str) -> Option<PhysicalQuantity> {
    let key = ALIASES.iter().find(|(alias, _)| *alias == key).map_or(key, |(_, name)| *name);
    let constant = CONSTANTS.iter().find(|c| c.key == key)?;
    Some(quantity(constant))
}

fn quantity(constant: &Constant) -> PhysicalQuantity {
    if constant.key == HBAR {
        let h = CONSTANTS.iter().find(|c| c.key == "h_P").expect("h is in the table");
        return quantity(h).div(&PhysicalQuantity::dimensionless(Value::int(2).mul(&Value::Pi))).expect("2π is not zero");
    }
    let value = Decimal::parse(constant.value).expect("a valid decimal");
    PhysicalQuantity::new(Value::from_decimal(&value), unit(constant.unit))
}

/// The unit written as `a*b/c/d`, from atoms of the unit table.
fn unit(written: &str) -> Unit {
    let mut parts = written.split('/');
    let mut result = Unit::ONE;
    for factor in parts.next().unwrap_or_default().split('*').filter(|s| !s.is_empty()) {
        result = result.multiply(&crate::units::find(factor).expect("a known unit"));
    }
    for divisor in parts {
        result = result.divide(&crate::units::find(divisor).expect("a known unit"));
    }
    result
}

/// A category of the catalogue, with its constants.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConstantCategory {
    pub category: Category,
    pub name: String,
    pub constants: Vec<ConstantInfo>,
}

/// A constant for a menu: what to insert, its name, and its value with unit in LaTeX.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConstantInfo {
    pub key: String,
    pub latex: String,
    pub name: String,
    pub value: String,
}

/// All constants by category, named in `language`.
pub fn catalogue(language: Language) -> Vec<ConstantCategory> {
    Category::ALL
        .into_iter()
        .map(|category| ConstantCategory {
            category,
            name: category.name(language).to_string(),
            constants: CONSTANTS
                .iter()
                .filter(|c| c.category == category)
                .map(|c| {
                    let q = quantity(c);
                    let decimal = q.value.decimal(12).expect("a constant has a decimal value");
                    let number = numbers::format(&decimal);
                    let value = if q.unit.is_one() { number } else { format!("{number}\\,{}", unit_latex::format(&q.unit)) };
                    ConstantInfo {
                        key: c.key.to_string(),
                        latex: c.latex.to_string(),
                        name: if language == Language::De { c.de } else { c.en }.to_string(),
                        value,
                    }
                })
                .collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::latex::{parser, Node};

    #[test]
    fn every_constant_parses_to_its_key_and_has_its_unit() {
        assert_eq!(CONSTANTS.len(), 47);
        for constant in &CONSTANTS {
            let node = parser::parse(constant.latex).unwrap().expression;
            let Node::Variable { name, index, .. } = node else { panic!("{} is not a variable", constant.latex) };
            assert_eq!(Node::variable_key(&name, &index), constant.key, "{}", constant.latex);
            assert!(find(constant.key).is_some());
        }
    }

    #[test]
    fn keys_are_unique() {
        for (i, a) in CONSTANTS.iter().enumerate() {
            assert!(CONSTANTS[i + 1..].iter().all(|b| b.key != a.key), "{}", a.key);
        }
    }

    #[test]
    fn the_catalogue_has_six_categories() {
        let catalogue = catalogue(Language::De);
        assert_eq!(catalogue.len(), 6);
        assert_eq!(catalogue.iter().map(|c| c.constants.len()).sum::<usize>(), 47);
        let rydberg = catalogue[2].constants.iter().find(|c| c.key == "R_inf").unwrap();
        assert_eq!(rydberg.value, "10973731{,}5682\\,\\frac{1}{\\mathrm{m}}");
    }
}
