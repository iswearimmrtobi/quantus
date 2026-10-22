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

    pub fn pow_int(&self, exponent: i32) -> Result<PhysicalQuantity> {
        Ok(PhysicalQuantity::new(
            self.value.pow(&Value::int(exponent.into()))?,
            self.unit.pow(exponent),
        ))
    }

    /// Raising to an arbitrary power.
    ///
    /// The exponent must be dimensionless. If the base carries a unit, the exponent must also
    /// be rational: only then can the unit be raised along with it (`m²` to the ½ is `m`). A
    /// dimensionless base may be raised to anything.
    pub fn pow(&self, exponent: &PhysicalQuantity) -> Result<PhysicalQuantity> {
        if !exponent.is_dimensionless() {
            return Err(QuantusError::DimensionMismatch(format!(
                "The exponent must be dimensionless but carries the unit {}.",
                registry::describe(&exponent.unit)
            )));
        }
        let powered = self.value.pow(&exponent.value)?;
        if self.is_dimensionless() {
            return Ok(PhysicalQuantity::dimensionless(powered));
        }
        let rational = exponent.value.as_rational().ok_or_else(|| {
            QuantusError::other(format!(
                "A quantity with a unit can only be raised to a rational power, but the exponent \
                 here is '{}'.",
                exponent.value
            ))
        })?;
        let (numerator, denominator) = (
            i32::try_from(rational.numerator().clone()),
            i32::try_from(rational.denominator().clone()),
        );
        let (Ok(numerator), Ok(denominator)) = (numerator, denominator) else {
            return Err(QuantusError::other(format!("The exponent '{}' is too large.", exponent.value)));
        };
        let unit = self.unit.pow(numerator);
        let unit = if denominator == 1 { unit } else { unit.root(denominator)? };
        Ok(PhysicalQuantity::new(powered, unit))
    }

    /// `n`-th root of both value and unit (`m²` to `m`).
    pub fn root(&self, n: u32) -> Result<PhysicalQuantity> {
        if n == 0 {
            return Err(QuantusError::other("root exponent must be positive, was: 0"));
        }
        self.pow(&PhysicalQuantity::dimensionless(Value::ratio(1, n.into())))
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
    fn integer_powers_raise_the_unit_along() {
        let area = q(3, &[("m", 1)]).pow_int(2).unwrap();
        assert_eq!((area.value, area.unit.to_string()), (Value::int(9), "m²".to_string()));
        let inverse = q(3, &[("m", 1)]).pow_int(-2).unwrap();
        assert_eq!((inverse.value, inverse.unit.to_string()), (Value::ratio(1, 9), "1/m²".to_string()));
        let zero = q(3, &[("m", 1)]).pow_int(0).unwrap();
        assert_eq!((zero.value, zero.unit), (Value::one(), Unit::ONE));
    }

    #[test]
    fn roots_take_the_unit_along() {
        let length = q(9, &[("m", 2)]).root(2).unwrap();
        assert_eq!((length.value, length.unit.to_string()), (Value::int(3), "m".to_string()));
        let half = q(9, &[("m", 1)]).root(2).unwrap();
        assert_eq!(half.unit.to_string(), "m^1:2");
        let irrational = q(2, &[]).root(2).unwrap();
        assert!(irrational.value.as_rational().is_none());
        assert!(q(2, &[]).root(0).is_err());
    }

    #[test]
    fn exponents_must_be_dimensionless_and_rational_for_unit_bearing_bases() {
        assert_eq!(
            message(q(2, &[("m", 1)]).pow(&q(2, &[("s", 1)]))),
            "The exponent must be dimensionless but carries the unit s (time)."
        );
        let sqrt2 = PhysicalQuantity::dimensionless(Value::int(2).pow(&Value::ratio(1, 2)).unwrap());
        assert_eq!(
            message(q(2, &[("m", 1)]).pow(&sqrt2)),
            "A quantity with a unit can only be raised to a rational power, but the exponent here is 'Sqrt(2)'."
        );
        let half = PhysicalQuantity::dimensionless(Value::ratio(1, 2));
        let root = q(4, &[("m", 2)]).pow(&half).unwrap();
        assert_eq!((root.value, root.unit.to_string()), (Value::int(2), "m".to_string()));
    }

    #[test]
    fn negation_keeps_the_unit() {
        let negated = q(3, &[("K", 1)]).neg();
        assert_eq!((negated.value, negated.unit.to_string()), (Value::int(-3), "K".to_string()));
    }
}
