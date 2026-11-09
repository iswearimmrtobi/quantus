//! The angle unit: in degree mode a plain number in `\sin` is read in degrees and `\arcsin`
//! answers in degrees. Each mode computes in its own unit: `30^\circ` is `30` in degree mode and
//! `\frac{\pi}{6}` in radian mode, so the degree sign works in both. A value marked with
//! `\mathrm{rad}` is radians either way, so `\sin(\pi\,\mathrm{rad})` is 0 in both.

use std::cell::Cell;

use crate::latex::Node;
use crate::num::Value;
use crate::units::atom::Kind;
use crate::Result;

/// Functions that take an angle.
pub(crate) fn takes_angle(name: &str) -> bool {
    matches!(name, "sin" | "cos" | "tan" | "cot" | "sec" | "csc")
}

/// Whether `node` gives an angle that the °′″ form suits: it has a degree sign or an inverse
/// trigonometric function, or takes the argument of a complex number.
pub(crate) fn gives_degrees(node: &Node) -> bool {
    match node {
        Node::Degrees { .. } => true,
        Node::Function { name, .. } if gives_angle(name) || name == "arg" => true,
        other => other.children().into_iter().any(gives_degrees),
    }
}

/// Functions that give an angle.
pub(crate) fn gives_angle(name: &str) -> bool {
    matches!(name, "arcsin" | "arccos" | "arctan")
}

/// Whether `node` is marked as radians, by a `\mathrm{rad}` outside any nested function.
pub(crate) fn is_marked(node: &Node) -> bool {
    match node {
        Node::Unit { unit, .. } => unit.factors().iter().any(|factor| factor.item.kind() == Kind::Radian),
        Node::Function { .. } | Node::Call { .. } | Node::Sum { .. } | Node::Integral { .. } | Node::Antiderivative { .. }
        | Node::Limit { .. } | Node::Derivative { .. } => false,
        other => other.children().into_iter().any(is_marked),
    }
}

/// `degrees · π / 180`.
pub(crate) fn to_radians(degrees: &Value) -> Result<Value> {
    degrees.mul(&Value::Pi).div(&Value::int(180))
}

/// `radians · 180 / π`.
pub(crate) fn to_degrees(radians: &Value) -> Result<Value> {
    radians.mul(&Value::int(180)).div(&Value::Pi)
}

thread_local! {
    static DEGREES: Cell<bool> = const { Cell::new(false) };
}

/// Whether the derivative being built reads angles in degrees (see [`DegreeScope`]).
pub(crate) fn in_degrees() -> bool {
    DEGREES.with(Cell::get)
}

/// Sets the angle unit for the symbolic derivative while it lives.
pub(crate) struct DegreeScope(bool);

impl DegreeScope {
    pub(crate) fn new(degrees: bool) -> Self {
        DegreeScope(DEGREES.with(|current| current.replace(degrees)))
    }
}

impl Drop for DegreeScope {
    fn drop(&mut self) {
        DEGREES.with(|current| current.set(self.0));
    }
}
