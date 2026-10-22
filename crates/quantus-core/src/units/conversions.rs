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
