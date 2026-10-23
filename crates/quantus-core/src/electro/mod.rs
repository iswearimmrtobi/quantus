//! Electronics mode: passive DC circuits. UI-independent, no state of its own.

pub mod circuit;
pub mod interface;
pub mod materials;
pub mod reverse;
pub mod solve;
pub mod text;
pub mod wire;

pub use circuit::{Circuit, Element, ElementKind, GROUND};
pub use materials::Material;
pub use reverse::{solve_for, Measure, Parameter, Reverse, Target};
pub use solve::{solve, solve_at, ElementResult, NodeResult, Output, Reason, Solution};
pub use wire::{Warning, Wire};
pub use interface::{CircuitRequest, CircuitResponse};
