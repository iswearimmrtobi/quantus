//! The circuit as the solver sees it: nodes and two-terminal elements. Editor data (positions,
//! rotation) is not part of it.

use crate::num::Value;
use crate::quantity::PhysicalQuantity;

use super::wire::Wire;

/// Node 0 is ground, the reference potential.
pub const GROUND: u32 = 0;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Circuit {
    /// Every node by ID; must contain [`GROUND`].
    pub nodes: Vec<u32>,
    pub elements: Vec<Element>,
}

/// Currents and voltages are counted from terminal `a` to terminal `b`: the voltage of an element
/// is `V(a) − V(b)`, its current flows through it from `a` to `b`.
#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    /// Stable ID that results are reported under, e.g. "R1".
    pub id: String,
    pub a: u32,
    pub b: u32,
    pub kind: ElementKind,
}

/// A parameter that is `None` is unknown, never guessed.
#[derive(Clone, Debug, PartialEq)]
pub enum ElementKind {
    Resistor {
        r: Option<PhysicalQuantity>,
    },
    Wire(Box<Wire>),
    /// Ideal source with `V(a) − V(b) = U`, optionally with a series resistance.
    VoltageSource {
        u: Option<PhysicalQuantity>,
        internal_resistance: Option<PhysicalQuantity>,
    },
    /// Ideal source driving `I` through itself from `a` to `b`, optionally with a parallel conductance.
    CurrentSource {
        i: Option<PhysicalQuantity>,
        internal_conductance: Option<PhysicalQuantity>,
    },
    /// Terminals `a` and `b` are the ends of the track. The wiper at `position` (0 at `a`, 1 at `b`)
    /// splits it into `R·position` between `a` and the wiper and `R·(1 − position)` after it.
    Potentiometer {
        wiper: u32,
        r_total: Option<PhysicalQuantity>,
        position: Option<Value>,
    },
    /// A consumer that draws the constant power `P` whatever its voltage: `V·I = P` with the
    /// voltage `V(a) − V(b)` positive. Nonlinear, so the solver iterates.
    PowerLoad {
        p: Option<PhysicalQuantity>,
    },
}
