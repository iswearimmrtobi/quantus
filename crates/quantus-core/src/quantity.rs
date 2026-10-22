//! A physical quantity: an exact value together with its unit.
//!
//! `Value` keeps the number exact (rationals, π, roots); `Unit` keeps the dimensional books:
//! unit arithmetic for `×`/`÷`, compatibility and conversion for `+`/`−`. Decimal digits only
//! appear at the output boundary, through [`PhysicalQuantity::decimal`].
//!
//! The error texts are the ones the Kotlin implementation produced.

use std::fmt;

use dashu::rational::RBig;

use crate::error::{QuantusError, Result};
use crate::num::{Decimal, Value};
use crate::units::{between, registry, AffineConversion, Dimension, Unit};

#[derive(Clone, Debug, PartialEq)]
pub struct PhysicalQuantity {
    pub value: Value,
    pub unit: Unit,
}

impl PhysicalQuantity {
    pub fn new(value: Value, unit: Unit) -> Self {
        PhysicalQuantity { value, unit }
    }

    /// A plain number without a unit, e.g. the ½ in `E = ½mv²`.
    pub fn dimensionless(value: Value) -> Self {
        PhysicalQuantity::new(value, Unit::ONE)
    }

    pub fn dimension(&self) -> Dimension {
        self.unit.dimension()
    }

    pub fn is_dimensionless(&self) -> bool {
        self.unit.is_compatible(&Unit::ONE)
    }

    /// F11: informational quantity name (`m/s` is "velocity"), if known.
    pub fn quantity_name(&self) -> Option<&'static str> {
        registry::quantity_name(&self.dimension())
    }

    /// Numeric value with `precision` significant digits (F3).
    pub fn decimal(&self, precision: usize) -> Result<Decimal> {
        self.value.decimal(precision)
    }

    /// The right operand is converted into the left operand's unit, which the result keeps.
    pub fn add(&self, other: &PhysicalQuantity) -> Result<PhysicalQuantity> {
        let converted = other.value_in(&self.unit, "added", self)?;
        Ok(PhysicalQuantity::new(self.value.add(&converted), self.unit.clone()))
    }

    pub fn sub(&self, other: &PhysicalQuantity) -> Result<PhysicalQuantity> {
        let converted = other.value_in(&self.unit, "subtracted", self)?;
        Ok(PhysicalQuantity::new(self.value.sub(&converted), self.unit.clone()))
    }

    pub fn mul(&self, other: &PhysicalQuantity) -> PhysicalQuantity {
        PhysicalQuantity::new(self.value.mul(&other.value), self.unit.multiply(&other.unit))
    }

    pub fn div(&self, other: &PhysicalQuantity) -> Result<PhysicalQuantity> {
        Ok(PhysicalQuantity::new(self.value.div(&other.value)?, self.unit.divide(&other.unit)))
    }

    pub fn neg(&self) -> PhysicalQuantity {
        PhysicalQuantity::new(self.value.neg(), self.unit.clone())
    }

    /// Explicit conversion to a target unit (F1: km/h to m/s, prefixes).
    pub fn to(&self, target: &Unit) -> Result<PhysicalQuantity> {
        if !self.unit.is_compatible(target) {
            return Err(QuantusError::DimensionMismatch(format!(
                "Cannot convert {} to {}.",
                registry::describe(&self.unit),
                registry::describe(target)
            )));
        }
        if self.unit == *target {
            return Ok(self.clone());
        }
        let conversion = between(&self.unit, target)?;
        Ok(PhysicalQuantity::new(apply(&conversion, &self.value), target.clone()))
    }

    /// F1: automatic simplification to a named unit, `kg·m/s²` to `N`. Units the user chose
    /// (`kJ`, `km/h`, `N`) are left alone.
    pub fn simplify(&self) -> Result<PhysicalQuantity> {
        match registry::simplification_target(&self.unit) {
            Some(target) => self.to(&target),
            None => Ok(self.clone()),
        }
    }

    /// This quantity's value expressed in `target`, with a dimension check. `context` is the
    /// quantity being converted to; it shows up in the error message.
    fn value_in(&self, target: &Unit, verb: &str, context: &PhysicalQuantity) -> Result<Value> {
        if !self.unit.is_compatible(target) {
            return Err(QuantusError::DimensionMismatch(format!(
                "Incompatible units: {} and {} cannot be {verb}.",
                registry::describe(&context.unit),
                registry::describe(&self.unit)
            )));
        }
        let conversion = between(&self.unit, target)?;
        // Offset units have no meaningful sum: 20 °C + 5 K is 25 °C read as a temperature
        // difference and 298.15 °C read as two absolute temperatures. Rather than pick one
        // silently, say so; converting to K makes the intent explicit.
        if conversion.offset != RBig::ZERO {
            return Err(QuantusError::DimensionMismatch(format!(
                "{} and {} cannot be {verb} directly because one of them has a zero point of its own. \
                 Convert both to the same unit first.",
                registry::describe(&context.unit),
                registry::describe(&self.unit)
            )));
        }
        Ok(apply(&conversion, &self.value))
    }
}

fn apply(conversion: &AffineConversion, value: &Value) -> Value {
    if conversion.is_identity() {
        return value.clone();
    }
    Value::Rat(conversion.factor.clone())
        .mul(value)
        .add(&Value::Rat(conversion.offset.clone()))
}

impl fmt::Display for PhysicalQuantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.value, registry::format(&self.unit))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::find;

    fn q(value: i64, symbols: &[(&str, i32)]) -> PhysicalQuantity {
        let unit = symbols
            .iter()
            .fold(Unit::ONE, |acc, (symbol, pow)| acc.multiply(&find(symbol).unwrap().pow(*pow)));
        PhysicalQuantity::new(Value::int(value), unit)
    }

    fn message<T>(result: Result<T>) -> String {
        match result {
            Ok(_) => panic!("expected an error"),
            Err(e) => e.to_string(),
        }
    }

    #[test]
    fn force_from_mass_times_acceleration_simplifies_to_newton() {
        let product = q(2, &[("kg", 1)]).mul(&q(10, &[("m", 1), ("s", -2)]));
        assert_eq!(product.unit.to_string(), "kg·m/s²");
        let newton = product.simplify().unwrap();
        assert_eq!((newton.value, newton.unit.to_string()), (Value::int(20), "N".to_string()));
    }

    #[test]
    fn the_right_operand_is_converted_into_the_left_unit() {
        let sum = q(1, &[("km", 1), ("h", -1)]).add(&q(1, &[("m", 1), ("s", -1)])).unwrap();
        assert_eq!(sum.value, Value::ratio(23, 5));
        assert_eq!(sum.unit.to_string(), "km/h");
        let difference = q(1, &[("m", 1)]).sub(&q(50, &[("cm", 1)])).unwrap();
        assert_eq!(difference.value, Value::ratio(1, 2));
    }

    #[test]
    fn mismatched_dimensions_name_both_units() {
        assert_eq!(
            message(q(5, &[("m", 1)]).add(&q(3, &[("s", 1)]))),
            "Incompatible units: m (length) and s (time) cannot be added."
        );
        assert_eq!(
            message(q(5, &[("m", 1)]).sub(&q(3, &[("s", 1)]))),
            "Incompatible units: m (length) and s (time) cannot be subtracted."
        );
        assert_eq!(
            message(q(36, &[("km", 1), ("h", -1)]).to(&find("J").unwrap())),
            "Cannot convert km/h (velocity) to J (energy)."
        );
    }

    #[test]
    fn offset_units_do_not_add_across_zero_points() {
        assert_eq!(
            message(q(20, &[("°C", 1)]).add(&q(5, &[("K", 1)]))),
            "℃ (temperature) and K (temperature) cannot be added directly because one of them \
             has a zero point of its own. Convert both to the same unit first."
        );
        let same = q(20, &[("°C", 1)]).add(&q(5, &[("°C", 1)])).unwrap();
        assert_eq!(same.value, Value::int(25));
    }

    #[test]
    fn conversion_is_exact() {
        let speed = q(1, &[("km", 1), ("h", -1)]);
        let target = Unit::ONE.multiply(&find("m").unwrap()).divide(&find("s").unwrap());
        assert_eq!(speed.to(&target).unwrap().value, Value::ratio(5, 18));

        let celsius = q(20, &[("°C", 1)]).to(&find("K").unwrap()).unwrap();
        assert_eq!(celsius.value, Value::ratio(5863, 20));
    }

    #[test]
    fn dividing_by_zero_is_an_error() {
        assert!(q(1, &[("m", 1)]).div(&q(0, &[("s", 1)])).is_err());
    }

    #[test]
    fn negation_keeps_the_unit() {
        let negated = q(3, &[("K", 1)]).neg();
        assert_eq!((negated.value, negated.unit.to_string()), (Value::int(-3), "K".to_string()));
    }
}
