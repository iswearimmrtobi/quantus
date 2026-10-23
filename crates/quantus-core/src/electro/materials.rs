//! Resistivity ρ at 20 °C and temperature coefficient α of common conductors.
//!
//! Both are exact rationals, so they carry no rounding error into the exact solver.

use crate::num::Value;
use crate::quantity::PhysicalQuantity;
use crate::units::{find, Unit};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Material {
    Copper,
    Aluminium,
    Silver,
    Gold,
    Iron,
    Constantan,
    Brass,
}

/// A rational as numerator and denominator.
type Ratio = (i64, u64);

struct Entry {
    material: Material,
    id: &'static str,
    german: &'static str,
    /// ρ in Ω·mm²/m at 20 °C.
    resistivity: Ratio,
    /// α in 1/K.
    alpha: Ratio,
}

const TABLE: [Entry; 7] = [
    Entry { material: Material::Copper, id: "copper", german: "Kupfer", resistivity: (172, 10_000), alpha: (39, 10_000) },
    Entry { material: Material::Aluminium, id: "aluminium", german: "Aluminium", resistivity: (282, 10_000), alpha: (39, 10_000) },
    Entry { material: Material::Silver, id: "silver", german: "Silber", resistivity: (159, 10_000), alpha: (38, 10_000) },
    Entry { material: Material::Gold, id: "gold", german: "Gold", resistivity: (244, 10_000), alpha: (34, 10_000) },
    Entry { material: Material::Iron, id: "iron", german: "Eisen", resistivity: (10, 100), alpha: (56, 10_000) },
    Entry { material: Material::Constantan, id: "constantan", german: "Konstantan", resistivity: (49, 100), alpha: (1, 100_000) },
    Entry { material: Material::Brass, id: "brass", german: "Messing", resistivity: (7, 100), alpha: (15, 10_000) },
];

fn ratio((numerator, denominator): Ratio) -> Value {
    Value::ratio(numerator, denominator)
}

impl Material {
    pub const ALL: [Material; 7] = [
        Material::Copper,
        Material::Aluminium,
        Material::Silver,
        Material::Gold,
        Material::Iron,
        Material::Constantan,
        Material::Brass,
    ];

    fn entry(self) -> &'static Entry {
        TABLE.iter().find(|e| e.material == self).expect("every material has a table entry")
    }

    /// The English identifier used in the JSON interface.
    pub fn id(self) -> &'static str {
        self.entry().id
    }

    pub fn german_name(self) -> &'static str {
        self.entry().german
    }

    /// Looks a material up by its identifier or German name, ignoring case.
    pub fn from_name(name: &str) -> Option<Material> {
        let name = name.trim().to_lowercase();
        TABLE
            .iter()
            .find(|e| e.id == name || e.german.to_lowercase() == name)
            .map(|e| e.material)
    }

    /// ρ at 20 °C, in Ω·mm²/m.
    pub fn resistivity(self) -> PhysicalQuantity {
        let unit = |symbol| find(symbol).expect("unit is registered");
        let unit: Unit = unit("Ω").multiply(&unit("mm").pow(2)).divide(&unit("m"));
        PhysicalQuantity::new(ratio(self.entry().resistivity), unit)
    }

    /// α in 1/K.
    pub fn temperature_coefficient(self) -> PhysicalQuantity {
        let kelvin = find("K").expect("unit is registered");
        PhysicalQuantity::new(ratio(self.entry().alpha), kelvin.inverse())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copper_is_0_0172_ohm_mm2_per_m() {
        let rho = Material::Copper.resistivity();
        assert_eq!(rho.value, Value::ratio(43, 2500));
        assert_eq!(rho.unit.to_string(), "Ω·mm²/m");
    }

    #[test]
    fn materials_are_found_by_either_name() {
        assert_eq!(Material::from_name("Kupfer"), Some(Material::Copper));
        assert_eq!(Material::from_name(" copper "), Some(Material::Copper));
        assert_eq!(Material::from_name("wood"), None);
        for material in Material::ALL {
            assert_eq!(Material::from_name(material.id()), Some(material));
            assert_eq!(Material::from_name(material.german_name()), Some(material));
        }
    }
}
