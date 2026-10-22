//! `Unit`: a single atom or a product of atoms, as in Indriya.
//!
//! The split matters. `kg·m/s²` is a product and may be simplified to `N`; `kJ`, `N` and `km`
//! are atoms and stay as the user wrote them. Products never merge different atoms of the same
//! dimension (`km·m` stays `km·m`), and a product of one factor with exponent 1 *is* its atom.

use std::borrow::Cow;

use super::atom::Atom;
use super::dimension::Dimension;
use super::product::{self, Element};
use crate::error::{QuantusError, Result};

#[derive(Clone, Debug)]
pub enum Unit {
    Atom(Atom),
    /// Factors in insertion order; empty is the dimensionless unit `1`.
    Product(Vec<Element<Atom>>),
}

impl Unit {
    /// The dimensionless unit.
    pub const ONE: Unit = Unit::Product(Vec::new());

    pub fn is_one(&self) -> bool {
        matches!(self, Unit::Product(factors) if factors.is_empty())
    }

    pub fn is_product(&self) -> bool {
        matches!(self, Unit::Product(_))
    }

    pub fn as_atom(&self) -> Option<&Atom> {
        match self {
            Unit::Atom(atom) => Some(atom),
            Unit::Product(_) => None,
        }
    }

    /// The factors, an atom counting as the single factor `atom^1`.
    pub fn factors(&self) -> Cow<'_, [Element<Atom>]> {
        match self {
            Unit::Atom(atom) => Cow::Owned(vec![Element::once(*atom)]),
            Unit::Product(factors) => Cow::Borrowed(factors),
        }
    }

    fn from_factors(factors: Vec<Element<Atom>>) -> Unit {
        match factors.as_slice() {
            [only] if only.pow == 1 && only.root == 1 => Unit::Atom(only.item),
            _ => Unit::Product(factors),
        }
    }

    pub fn multiply(&self, other: &Unit) -> Unit {
        if self.is_one() {
            return other.clone();
        }
        if other.is_one() {
            return self.clone();
        }
        Unit::from_factors(product::merge(&self.factors(), &other.factors()))
    }

    pub fn inverse(&self) -> Unit {
        if self.is_one() {
            return Unit::ONE;
        }
        Unit::from_factors(product::inverse(&self.factors()))
    }

    pub fn divide(&self, other: &Unit) -> Unit {
        self.multiply(&other.inverse())
    }

    pub fn pow(&self, n: i32) -> Unit {
        Unit::from_factors(product::pow(&self.factors(), n))
    }

    /// The `n`-th root; `n` must not be zero.
    pub fn root(&self, n: i32) -> Result<Unit> {
        match n {
            0 => Err(QuantusError::other("Root's order of zero")),
            n if n < 0 => Ok(Unit::ONE.divide(&self.root(-n)?)),
            n => Ok(Unit::from_factors(product::root(&self.factors(), n))),
        }
    }

    /// `ProductUnit.getDimension`: the factors' dimensions multiplied in order.
    pub fn dimension(&self) -> Dimension {
        match self {
            Unit::Atom(atom) => atom.dimension().clone(),
            Unit::Product(factors) => factors.iter().fold(Dimension::NONE, |dimension, e| {
                dimension.multiply(&e.item.dimension().pow(e.pow).root(e.root))
            }),
        }
    }

    /// Whether a quantity in `self` can be converted to `other` (`Unit.isCompatible`).
    pub fn is_compatible(&self, other: &Unit) -> bool {
        self == other || self.dimension().is_compatible(&other.dimension())
    }
}

impl From<Atom> for Unit {
    fn from(atom: Atom) -> Self {
        Unit::Atom(atom)
    }
}

impl PartialEq for Unit {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Unit::Atom(a), Unit::Atom(b)) => a == b,
            (Unit::Product(a), Unit::Product(b)) => product::same_elements(a, b),
            _ => false,
        }
    }
}

impl Eq for Unit {}
