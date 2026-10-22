//! Exact unit conversions as `y = factor · x + offset`.
//!
//! Indriya computes with decimal128 and would cut `km/h` to `m/s` (5/18) after 34 digits; here
//! the factors stay rational and fold into the exact value unchanged.

use dashu::rational::RBig;

use super::unit::Unit;
use crate::error::{QuantusError, Result};

/// An exact affine map. Purely multiplicative conversions have `offset = 0`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AffineConversion {
    pub factor: RBig,
    pub offset: RBig,
}

impl AffineConversion {
    pub const IDENTITY: AffineConversion = AffineConversion {
        factor: RBig::ONE,
        offset: RBig::ZERO,
    };

    pub fn is_identity(&self) -> bool {
        self.factor == RBig::ONE && self.offset == RBig::ZERO
    }

    pub fn apply(&self, value: &RBig) -> RBig {
        &self.factor * value + &self.offset
    }
}

/// Exact conversion between two compatible units, folded from the atoms' factors; no
/// converter chain is needed. `km/h` to `m/s` is exactly 5/18, `°C` to `K` adds 5463/20.
pub fn between(from: &Unit, to: &Unit) -> Result<AffineConversion> {
    if from == to {
        return Ok(AffineConversion::IDENTITY);
    }
    if !from.is_compatible(to) {
        return Err(QuantusError::DimensionMismatch(format!("{from} is not compatible with {to}")));
    }
    let from = from.system_conversion()?;
    let to = to.system_conversion()?;
    // x -> system unit -> back out of the target.
    let factor = &from.factor / &to.factor;
    let offset = (&from.offset - &to.offset) / &to.factor;
    Ok(AffineConversion { factor, offset })
}

impl Unit {
    /// Conversion from this unit to the coherent system unit of its dimension.
    ///
    /// Products only compose linear atoms with whole exponents, as in Indriya: `°C/s` or
    /// `m^(1/2)` cannot be converted.
    pub fn system_conversion(&self) -> Result<AffineConversion> {
        match self {
            Unit::Atom(atom) => Ok(atom.system_conversion()),
            Unit::Product(factors) => {
                let mut factor = RBig::ONE;
                for e in factors {
                    let atom = e.item.system_conversion();
                    if atom.offset != RBig::ZERO {
                        return Err(QuantusError::UnsupportedConversion(
                            "cannot compose a unit with an offset into a product converter, because it is not a linear transformation (a linear function without offset)".into(),
                        ));
                    }
                    if e.root != 1 {
                        return Err(QuantusError::UnsupportedConversion(
                            "cannot compose a unit into a product converter, because it holds a base unit with exponent not equal to 1".into(),
                        ));
                    }
                    factor *= atom.factor.pow(e.pow as isize);
                }
                Ok(AffineConversion { factor, offset: RBig::ZERO })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::lookup::find;

    fn unit(symbols: &[(&str, i32)]) -> Unit {
        symbols
            .iter()
            .fold(Unit::ONE, |acc, (symbol, pow)| acc.multiply(&find(symbol).unwrap().pow(*pow)))
    }

    fn rat(text: &str) -> RBig {
        text.parse().unwrap()
    }

    fn convert(from: &Unit, to: &Unit) -> AffineConversion {
        between(from, to).unwrap()
    }

    #[test]
    fn kilometres_per_hour_to_metres_per_second_is_exactly_five_eighteenths() {
        let conversion = convert(&unit(&[("km", 1), ("h", -1)]), &unit(&[("m", 1), ("s", -1)]));
        assert_eq!(conversion.factor, rat("5/18"));
        assert_eq!(conversion.offset, RBig::ZERO);
    }

    #[test]
    fn prefixes_are_powers_of_ten_and_binary_prefixes_powers_of_two() {
        assert_eq!(convert(&unit(&[("km", 1)]), &unit(&[("m", 1)])).factor, rat("1000"));
        assert_eq!(convert(&unit(&[("mm", 1)]), &unit(&[("km", 1)])).factor, rat("1/1000000"));
        assert_eq!(convert(&unit(&[("Kil", 1)]), &unit(&[("l", 1)])).factor, rat("1024"));
    }

    #[test]
    fn celsius_carries_an_exact_offset() {
        let to_kelvin = convert(&unit(&[("°C", 1)]), &unit(&[("K", 1)]));
        assert_eq!(to_kelvin.factor, RBig::ONE);
        assert_eq!(to_kelvin.offset, rat("5463/20"));
        assert_eq!(to_kelvin.apply(&rat("100")), rat("37315/100"));

        let back = convert(&unit(&[("K", 1)]), &unit(&[("°C", 1)]));
        assert_eq!(back.apply(&rat("37315/100")), rat("100"));

        // 1000 x + 273.15: the prefix scales the argument, the offset is not scaled.
        let kilo = convert(&unit(&[("k°C", 1)]), &unit(&[("K", 1)]));
        assert_eq!((kilo.factor, kilo.offset), (rat("1000"), rat("5463/20")));
    }

    #[test]
    fn units_of_equal_dimension_but_different_system_unit_convert_with_factor_one() {
        let force = unit(&[("kg", 1), ("m", 1), ("s", -2)]);
        assert!(convert(&force, &unit(&[("N", 1)])).is_identity());
        assert!(convert(&unit(&[("Hz", 1)]), &unit(&[("Bq", 1)])).is_identity());
        assert!(convert(&unit(&[("J", 1)]), &unit(&[("N", 1), ("m", 1)])).is_identity());
    }

    #[test]
    fn identical_units_convert_by_identity() {
        assert!(convert(&unit(&[("km", 1)]), &unit(&[("km", 1)])).is_identity());
    }

    #[test]
    fn incompatible_units_are_rejected() {
        let error = between(&unit(&[("m", 1)]), &unit(&[("s", 1)])).unwrap_err();
        assert!(matches!(error, QuantusError::DimensionMismatch(_)), "{error}");
    }

    #[test]
    fn offset_units_cannot_sit_in_a_product() {
        let per_second = unit(&[("°C", 1), ("s", -1)]);
        assert!(matches!(
            per_second.system_conversion(),
            Err(QuantusError::UnsupportedConversion(_))
        ));
        let sqrt = find("m").unwrap().root(2).unwrap();
        assert!(matches!(sqrt.system_conversion(), Err(QuantusError::UnsupportedConversion(_))));
    }

    #[test]
    fn minutes_hours_and_years() {
        assert_eq!(convert(&unit(&[("h", 1)]), &unit(&[("min", 1)])).factor, rat("60"));
        assert_eq!(convert(&unit(&[("a", 1)]), &unit(&[("d", 1)])).factor, rat("3652425/10000"));
        assert_eq!(convert(&unit(&[("g", 1)]), &unit(&[("kg", 1)])).factor, rat("1/1000"));
    }
}
