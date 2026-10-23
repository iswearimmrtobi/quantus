//! A wire: ideal (R = 0) until material, length and cross-section are all given.

use crate::error::{QuantusError, Result};
use crate::num::Value;
use crate::quantity::PhysicalQuantity;
use crate::units::find;

use super::materials::Material;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Wire {
    pub material: Option<Material>,
    /// Dimension length.
    pub length: Option<PhysicalQuantity>,
    /// Dimension length².
    pub area: Option<PhysicalQuantity>,
    /// °C or K; 20 °C if absent.
    pub temp: Option<PhysicalQuantity>,
    /// Ω, e.g. from a datasheet. Takes precedence over everything else.
    pub r_override: Option<PhysicalQuantity>,
    /// H/m, 0 if absent. Only taken over, never derived.
    pub l_per_m: Option<PhysicalQuantity>,
    /// F/m, 0 if absent. Only taken over, never derived.
    pub c_per_m: Option<PhysicalQuantity>,
}

/// What the user should know about a resistance that was computed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Warning {
    /// No temperature given: the resistance holds at 20 °C.
    DefaultTemperature,
    /// Some but not all of material, length and cross-section are given, so R = 0.
    Idealised { missing: Vec<Missing> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Missing {
    Material,
    Length,
    Area,
}

impl Warning {
    /// The German note shown in the editor; `wire` is the wire's name, e.g. "W1".
    pub fn german(&self, wire: &str) -> String {
        match self {
            Warning::DefaultTemperature => "bei 20 °C".to_string(),
            Warning::Idealised { missing } => {
                let names: Vec<&str> = missing
                    .iter()
                    .map(|m| match m {
                        Missing::Material => "Material",
                        Missing::Length => "Länge",
                        Missing::Area => "Querschnitt",
                    })
                    .collect();
                let verb = if names.len() == 1 { "fehlt" } else { "fehlen" };
                format!("Leitung {wire} idealisiert: {} {verb}", names.join(", "))
            }
        }
    }
}

impl Wire {
    /// The wire's resistance in Ω, with the notes that explain how it came about.
    ///
    /// `r_override` wins. With material, length and cross-section, `R = ρ·(1 + α·(T − 20 °C))·L/A`.
    /// Otherwise the wire is ideal.
    pub fn resistance(&self) -> Result<(PhysicalQuantity, Vec<Warning>)> {
        let ohm = find("Ω").expect("unit is registered");
        let zero = PhysicalQuantity::new(Value::zero(), ohm.clone());

        if let Some(r) = &self.r_override {
            let r = r.to(&ohm)?;
            if r.value.is_negative()? {
                return Err(QuantusError::other("The resistance must not be negative."));
            }
            return Ok((r, vec![]));
        }

        let mut missing = Vec::new();
        if self.material.is_none() {
            missing.push(Missing::Material);
        }
        if self.length.is_none() {
            missing.push(Missing::Length);
        }
        if self.area.is_none() {
            missing.push(Missing::Area);
        }
        let (Some(material), Some(length), Some(area)) = (self.material, &self.length, &self.area) else {
            let warnings = if missing.len() == 3 { vec![] } else { vec![Warning::Idealised { missing }] };
            return Ok((zero, warnings));
        };

        let length = length.to(&find("m").expect("unit is registered"))?;
        let area = area.to(&find("m").expect("unit is registered").pow(2))?;
        if length.value.is_negative()? {
            return Err(QuantusError::other("The length must not be negative."));
        }
        if area.value.is_zero() || area.value.is_negative()? {
            return Err(QuantusError::other("The cross-section must be positive."));
        }

        let (difference, warnings) = self.temperature_difference()?;
        let factor = PhysicalQuantity::dimensionless(Value::one())
            .add(&material.temperature_coefficient().mul(&difference))?;
        let resistance = material.resistivity().mul(&factor).mul(&length).div(&area)?.to(&ohm)?;
        Ok((resistance, warnings))
    }

    /// T − 20 °C in K. Only the difference enters the formula; °C is an offset unit, so the
    /// temperature goes through K first.
    fn temperature_difference(&self) -> Result<(PhysicalQuantity, Vec<Warning>)> {
        let kelvin = find("K").expect("unit is registered");
        let reference = PhysicalQuantity::new(Value::ratio(5863, 20), kelvin.clone());
        match &self.temp {
            None => Ok((PhysicalQuantity::new(Value::zero(), kelvin), vec![Warning::DefaultTemperature])),
            Some(temp) => Ok((temp.to(&kelvin)?.sub(&reference)?, vec![])),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(value: Value, symbol: &str) -> PhysicalQuantity {
        PhysicalQuantity::new(value, find(symbol).unwrap())
    }

    fn cable(length: i64, area_mm2: i64) -> Wire {
        Wire {
            material: Some(Material::Copper),
            length: Some(q(Value::int(length), "m")),
            area: Some(PhysicalQuantity::new(Value::int(area_mm2), find("mm").unwrap().pow(2))),
            ..Wire::default()
        }
    }

    #[test]
    fn the_reference_cable_is_1_72_ohm() {
        // 2 × 100 m of 2.0 mm² copper: 0.0172 · 200 / 2
        let (r, warnings) = cable(200, 2).resistance().unwrap();
        assert_eq!(r.value, Value::ratio(43, 25));
        assert_eq!(r.unit.to_string(), "Ω");
        assert_eq!(warnings, vec![Warning::DefaultTemperature]);
        assert_eq!(warnings[0].german("W1"), "bei 20 °C");
    }

    #[test]
    fn units_are_converted_exactly() {
        let mut wire = cable(200, 2);
        wire.length = Some(q(Value::ratio(1, 5), "km"));
        // 0.000002 m² = 2 mm²
        wire.area = Some(PhysicalQuantity::new(Value::ratio(2, 1_000_000), find("m").unwrap().pow(2)));
        assert_eq!(wire.resistance().unwrap().0.value, Value::ratio(43, 25));
    }

    #[test]
    fn the_temperature_enters_as_a_difference() {
        let mut wire = cable(200, 2);
        wire.temp = Some(q(Value::int(120), "°C"));
        let (r, warnings) = wire.resistance().unwrap();
        // 1.72 · (1 + 0.0039 · 100)
        assert_eq!(r.value, Value::ratio(43, 25).mul(&Value::ratio(139, 100)));
        assert!(warnings.is_empty());

        wire.temp = Some(q(Value::ratio(5863, 20).add(&Value::int(100)), "K"));
        assert_eq!(wire.resistance().unwrap().0.value, r.value);

        wire.temp = Some(q(Value::int(20), "°C"));
        assert_eq!(wire.resistance().unwrap().0.value, Value::ratio(43, 25));
    }

    #[test]
    fn r_override_wins() {
        let mut wire = cable(200, 2);
        wire.r_override = Some(q(Value::int(3), "Ω"));
        let (r, warnings) = wire.resistance().unwrap();
        assert_eq!(r.value, Value::int(3));
        assert!(warnings.is_empty());
    }

    #[test]
    fn incomplete_wires_are_ideal_with_a_note() {
        let (r, warnings) = Wire::default().resistance().unwrap();
        assert!(r.value.is_zero());
        assert!(warnings.is_empty());

        let mut wire = cable(200, 2);
        wire.material = None;
        let (r, warnings) = wire.resistance().unwrap();
        assert!(r.value.is_zero());
        assert_eq!(warnings[0].german("W1"), "Leitung W1 idealisiert: Material fehlt");

        wire.length = None;
        let (_, warnings) = wire.resistance().unwrap();
        assert_eq!(warnings[0].german("W2"), "Leitung W2 idealisiert: Material, Länge fehlen");
    }

    #[test]
    fn wrong_dimensions_are_errors() {
        let mut wire = cable(200, 2);
        wire.length = Some(q(Value::int(200), "V"));
        assert_eq!(wire.resistance().unwrap_err().to_string(), "Cannot convert V (voltage) to m (length).");
        wire.length = Some(q(Value::int(200), "m"));
        wire.area = Some(q(Value::int(2), "mm"));
        assert!(wire.resistance().is_err());
        wire.area = Some(PhysicalQuantity::new(Value::int(0), find("mm").unwrap().pow(2)));
        assert!(wire.resistance().is_err());
    }

    #[test]
    fn the_reference_example_works_without_a_solver() {
        // 230 V over the wire (1.72 Ω) and a 10 Ω load in series
        let (wire, _) = cable(200, 2).resistance().unwrap();
        let total = wire.add(&q(Value::int(10), "Ω")).unwrap();
        let current = q(Value::int(230), "V").div(&total).unwrap();
        let drop = current.mul(&wire).simplify().unwrap();
        assert_eq!(drop.unit.to_string(), "V");
        assert_eq!(drop.decimal(4).unwrap().to_plain_string(), "33.75");
        let relative = wire.div(&total).unwrap().dimensionless_value().unwrap();
        assert_eq!(relative.decimal(3).unwrap().to_plain_string(), "0.147");
    }
}
