//! Physical dimensions, with Indriya's `UnitDimension` semantics.
//!
//! A dimension is a product of the seven base dimensions. Indriya keeps it as a `ProductUnit`
//! of pseudo units, so it remembers the *order* in which it was built: `N` prints as
//! `[L]·[M]/[T]²` while `kg·m/s²` prints as `[M]·[L]/[T]²`, although both are equal.
//! `Display` reproduces `Dimension.toString()`, equality ignores the order.

use std::fmt;
use std::hash::{Hash, Hasher};

use super::product::{self, Element};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BaseDim {
    Length,
    Mass,
    Time,
    Current,
    Temperature,
    Amount,
    Luminous,
}

impl BaseDim {
    /// The pseudo unit symbol Indriya prints, e.g. `[L]`.
    pub fn symbol(self) -> &'static str {
        match self {
            BaseDim::Length => "[L]",
            BaseDim::Mass => "[M]",
            BaseDim::Time => "[T]",
            BaseDim::Current => "[I]",
            BaseDim::Temperature => "[Θ]",
            BaseDim::Amount => "[N]",
            BaseDim::Luminous => "[J]",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Dimension(Vec<Element<BaseDim>>);

impl Dimension {
    /// The dimension of dimensionless quantities.
    pub const NONE: Dimension = Dimension(Vec::new());

    pub fn base(base: BaseDim) -> Self {
        Dimension(vec![Element::once(base)])
    }

    pub fn is_none(&self) -> bool {
        self.0.is_empty()
    }

    pub fn multiply(&self, other: &Dimension) -> Dimension {
        Dimension(product::merge(&self.0, &other.0))
    }

    pub fn divide(&self, other: &Dimension) -> Dimension {
        Dimension(product::merge(&self.0, &product::inverse(&other.0)))
    }

    pub fn pow(&self, n: i32) -> Dimension {
        Dimension(product::pow(&self.0, n))
    }

    /// `n` must be positive.
    pub fn root(&self, n: i32) -> Dimension {
        Dimension(product::root(&self.0, n))
    }

    /// The base dimensions with their integer exponent. Roots are dropped, which is what
    /// `getBaseDimensions()` reports: `[L]^1:2` shows up as `[L]` with exponent 1.
    pub fn exponents(&self) -> Vec<(BaseDim, i32)> {
        let mut exponents: Vec<_> = self.0.iter().map(|e| (e.item, e.pow)).collect();
        exponents.sort();
        exponents
    }

    /// Whether quantities of the two dimensions can be converted into each other.
    ///
    /// Indriya first compares dimensions and then falls back to `DimensionalModel`'s
    /// "fundamental dimension". That fallback folds `(acc · base)^exponent` instead of
    /// `acc · base^exponent` and ignores roots; it is reproduced as is, because the Kotlin
    /// implementation inherits the result.
    pub fn is_compatible(&self, other: &Dimension) -> bool {
        self == other || self.fundamental() == other.fundamental()
    }

    fn fundamental(&self) -> Dimension {
        self.0.iter().fold(Dimension::NONE, |acc, e| {
            acc.multiply(&Dimension::base(e.item)).pow(e.pow)
        })
    }
}

impl PartialEq for Dimension {
    fn eq(&self, other: &Self) -> bool {
        product::same_elements(&self.0, &other.0)
    }
}

impl Eq for Dimension {}

impl Hash for Dimension {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut sorted = self.0.clone();
        sorted.sort_by_key(|e| e.item);
        sorted.hash(state);
    }
}

impl fmt::Display for Dimension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return f.write_str("one");
        }
        f.write_str(&product::format(&self.0, |b| b.symbol().to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use BaseDim::*;

    fn d(base: BaseDim) -> Dimension {
        Dimension::base(base)
    }

    #[test]
    fn display_keeps_the_construction_order() {
        let force = d(Length).multiply(&d(Mass)).divide(&d(Time).pow(2));
        assert_eq!(force.to_string(), "[L]·[M]/[T]²");
        let product = d(Mass).multiply(&d(Length)).divide(&d(Time).pow(2));
        assert_eq!(product.to_string(), "[M]·[L]/[T]²");
        assert_eq!(force, product);
    }

    #[test]
    fn display_of_special_shapes() {
        assert_eq!(Dimension::NONE.to_string(), "one");
        assert_eq!(d(Time).pow(-1).to_string(), "1/[T]");
        assert_eq!(d(Time).pow(4).to_string(), "[T]^4");
        assert_eq!(d(Length).root(2).to_string(), "[L]^1:2");
        let pressure = d(Mass).divide(&d(Length).multiply(&d(Time).pow(2)));
        assert_eq!(pressure.to_string(), "[M]/([L]·[T]²)");
    }

    #[test]
    fn factors_cancel_and_merge_like_a_product_unit() {
        let energy = d(Length).pow(2).multiply(&d(Mass)).divide(&d(Time).pow(2));
        let gray = energy.divide(&d(Mass));
        assert_eq!(gray.to_string(), "[L]²/[T]²");
        assert!(d(Length).divide(&d(Length)).is_none());
        assert_eq!(d(Length).root(2).multiply(&d(Length).root(2)), d(Length));
    }

    #[test]
    fn exponents_lose_the_root() {
        let sqrt = d(Length).root(2);
        assert_eq!(sqrt.exponents(), vec![(Length, 1)]);
        assert_ne!(sqrt, d(Length));
    }

    #[test]
    fn compatibility_falls_back_to_the_lossy_fundamental_dimension() {
        assert!(d(Length).is_compatible(&d(Length)));
        assert!(!d(Length).is_compatible(&d(Time)));
        // Indriya ignores the root in the fallback, so these count as compatible.
        assert!(d(Length).root(2).is_compatible(&d(Length)));
    }
}
