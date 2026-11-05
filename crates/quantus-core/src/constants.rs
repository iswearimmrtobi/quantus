//! The physical constants (CODATA 2018): the 47 of the Casio fx-991DE X, in its six categories.
//! They are known in Physics mode (and the circuit modes) by their key; a session variable of
//! the same name wins. [`catalogue`] lists them for a CONST menu or a picker.

use serde::{Deserialize, Serialize};

use crate::latex::numbers;
use crate::num::{Decimal, Value};
use crate::quantity::PhysicalQuantity;
use crate::settings::Language;
use quantus_texts::{texts, Text};
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
        let name = match self {
            Category::Universal => Text(texts!("constant.category.universal")),
            Category::Electromagnetic => Text(texts!("constant.category.electromagnetic")),
            Category::AtomicNuclear => Text(texts!("constant.category.atomic_nuclear")),
            Category::PhysicoChemical => Text(texts!("constant.category.physico_chemical")),
            Category::Adopted => Text(texts!("constant.category.adopted_values")),
            Category::Other => Text(texts!("constant.category.other")),
        };
        name.get(language)
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
    name: Text,
}

const fn c(
    key: &'static str,
    latex: &'static str,
    value: &'static str,
    unit: &'static str,
    category: Category,
    name: Text,
) -> Constant {
    Constant { key, latex, value, unit, category, name }
}

use Category::*;

/// h/2π, computed rather than rounded.
const HBAR: &str = "\\hbar";

#[rustfmt::skip]
const CONSTANTS: [Constant; 47] = [
    c("h_P", "h_{P}", "6.62607015e-34", "J*s", Universal, Text(texts!("constant.planck_constant"))),
    c(HBAR, "\\hbar", "", "J*s", Universal, Text(texts!("constant.reduced_planck_constant"))),
    c("c_0", "c_{0}", "299792458", "m/s", Universal, Text(texts!("constant.speed_light_vacuum"))),
    c("\\varepsilon_0", "\\varepsilon_{0}", "8.8541878128e-12", "F/m", Universal, Text(texts!("constant.vacuum_permittivity"))),
    c("\\mu_0", "\\mu_{0}", "1.25663706212e-6", "N/A/A", Universal, Text(texts!("constant.vacuum_permeability"))),
    c("Z_0", "Z_{0}", "376.730313668", "Ohm", Universal, Text(texts!("constant.characteristic_impedance_vacuum"))),
    c("G_N", "G_{N}", "6.67430e-11", "m*m*m/kg/s/s", Universal, Text(texts!("constant.gravitational_constant"))),
    c("l_P", "l_{P}", "1.616255e-35", "m", Universal, Text(texts!("constant.planck_length"))),
    c("t_P", "t_{P}", "5.391247e-44", "s", Universal, Text(texts!("constant.planck_time"))),
    c("\\mu_N", "\\mu_{N}", "5.0507837461e-27", "J/T", Electromagnetic, Text(texts!("constant.nuclear_magneton"))),
    c("\\mu_B", "\\mu_{B}", "9.2740100783e-24", "J/T", Electromagnetic, Text(texts!("constant.bohr_magneton"))),
    c("e_0", "e_{0}", "1.602176634e-19", "C", Electromagnetic, Text(texts!("constant.elementary_charge"))),
    c("\\Phi_0", "\\Phi_{0}", "2.067833848e-15", "Wb", Electromagnetic, Text(texts!("constant.magnetic_flux_quantum"))),
    c("G_0", "G_{0}", "7.748091729e-5", "S", Electromagnetic, Text(texts!("constant.conductance_quantum"))),
    c("K_J", "K_{J}", "483597.8484e9", "Hz/V", Electromagnetic, Text(texts!("constant.josephson_constant"))),
    c("R_K", "R_{K}", "25812.80745", "Ohm", Electromagnetic, Text(texts!("constant.von_klitzing_constant"))),
    c("m_p", "m_{p}", "1.67262192369e-27", "kg", AtomicNuclear, Text(texts!("constant.proton_mass"))),
    c("m_n", "m_{n}", "1.67492749804e-27", "kg", AtomicNuclear, Text(texts!("constant.neutron_mass"))),
    c("m_e", "m_{e}", "9.1093837015e-31", "kg", AtomicNuclear, Text(texts!("constant.electron_mass"))),
    c("m_\\mu", "m_{\\mu}", "1.883531627e-28", "kg", AtomicNuclear, Text(texts!("constant.muon_mass"))),
    c("a_0", "a_{0}", "5.29177210903e-11", "m", AtomicNuclear, Text(texts!("constant.bohr_radius"))),
    c("\\alpha_f", "\\alpha_{f}", "7.2973525693e-3", "", AtomicNuclear, Text(texts!("constant.fine_structure_constant"))),
    c("r_e", "r_{e}", "2.8179403262e-15", "m", AtomicNuclear, Text(texts!("constant.classical_electron_radius"))),
    c("\\lambda_C", "\\lambda_{C}", "2.42631023867e-12", "m", AtomicNuclear, Text(texts!("constant.compton_wavelength"))),
    c("\\gamma_p", "\\gamma_{p}", "2.6752218744e8", "/s/T", AtomicNuclear, Text(texts!("constant.proton_gyromagnetic_ratio"))),
    c("\\lambda_Cp", "\\lambda_{Cp}", "1.32140985539e-15", "m", AtomicNuclear, Text(texts!("constant.proton_compton_wavelength"))),
    c("\\lambda_Cn", "\\lambda_{Cn}", "1.31959090581e-15", "m", AtomicNuclear, Text(texts!("constant.neutron_compton_wavelength"))),
    c("R_inf", "R_{inf}", "10973731.568160", "/m", AtomicNuclear, Text(texts!("constant.rydberg_constant"))),
    c("\\mu_p", "\\mu_{p}", "1.41060679736e-26", "J/T", AtomicNuclear, Text(texts!("constant.proton_magnetic_moment"))),
    c("\\mu_e", "\\mu_{e}", "-9.2847647043e-24", "J/T", AtomicNuclear, Text(texts!("constant.electron_magnetic_moment"))),
    c("\\mu_n", "\\mu_{n}", "-9.6623651e-27", "J/T", AtomicNuclear, Text(texts!("constant.neutron_magnetic_moment"))),
    c("\\mu_\\mu", "\\mu_{\\mu}", "-4.49044830e-26", "J/T", AtomicNuclear, Text(texts!("constant.muon_magnetic_moment"))),
    c("m_\\tau", "m_{\\tau}", "3.16754e-27", "kg", AtomicNuclear, Text(texts!("constant.tau_mass"))),
    c("m_u", "m_{u}", "1.66053906660e-27", "kg", PhysicoChemical, Text(texts!("constant.atomic_mass_constant"))),
    c("F_Fa", "F_{Fa}", "96485.33212", "C/mol", PhysicoChemical, Text(texts!("constant.faraday_constant"))),
    c("N_A", "N_{A}", "6.02214076e23", "/mol", PhysicoChemical, Text(texts!("constant.avogadro_constant"))),
    c("k_B", "k_{B}", "1.380649e-23", "J/K", PhysicoChemical, Text(texts!("constant.boltzmann_constant"))),
    c("V_m", "V_{m}", "22.41396954e-3", "m*m*m/mol", PhysicoChemical, Text(texts!("constant.molar_volume_stp"))),
    // N_A k_B, exactly
    c("R_m", "R_{m}", "8.31446261815324", "J/mol/K", PhysicoChemical, Text(texts!("constant.molar_gas_constant"))),
    c("c_1", "c_{1}", "3.741771852e-16", "W*m*m", PhysicoChemical, Text(texts!("constant.first_radiation_constant"))),
    c("c_2", "c_{2}", "1.438776877e-2", "m*K", PhysicoChemical, Text(texts!("constant.second_radiation_constant"))),
    c("\\sigma_SB", "\\sigma_{SB}", "5.670374419e-8", "W/m/m/K/K/K/K", PhysicoChemical, Text(texts!("constant.stefan_boltzmann_constant"))),
    c("g_n", "g_{n}", "9.80665", "m/s/s", Adopted, Text(texts!("constant.standard_acceleration_gravity"))),
    c("p_atm", "p_{atm}", "101325", "Pa", Adopted, Text(texts!("constant.standard_atmosphere"))),
    c("R_K90", "R_{K90}", "25812.807", "Ohm", Adopted, Text(texts!("constant.von_klitzing_1990"))),
    c("K_J90", "K_{J90}", "483597.9e9", "Hz/V", Adopted, Text(texts!("constant.josephson_1990"))),
    c("T_0", "T_{0}", "273.15", "K", Other, Text(texts!("constant.zero_celsius"))),
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
                        name: c.name.get(language).to_string(),
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
