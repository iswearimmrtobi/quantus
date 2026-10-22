//! Unit symbols as `SimpleUnitFormat` prints them: `N`, `km/h`, `kg·m/s²`, `℃`, `1/s`.

use std::fmt;

use super::atom::Atom;
use super::lookup;
use super::product;
use super::unit::Unit;

impl Atom {
    /// The registered symbol (`μm` with a Greek mu, `℃`), else prefix and kind symbol.
    pub fn name(&self) -> String {
        match lookup::label(self) {
            Some(label) => label.to_string(),
            None => format!("{}{}", self.prefix().map_or("", |p| p.symbol), self.kind().symbol()),
        }
    }
}

/// The dimensionless unit prints as `1`.
impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Unit::Atom(atom) => f.write_str(&atom.name()),
            Unit::Product(factors) if factors.is_empty() => f.write_str("1"),
            Unit::Product(factors) => f.write_str(&product::format(factors, Atom::name)),
        }
    }
}
