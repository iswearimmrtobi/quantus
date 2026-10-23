//! Electronics mode: passive DC circuits. UI-independent, no state of its own.

pub mod circuit;
pub mod materials;
pub mod solve;
pub mod wire;

pub use circuit::{Circuit, Element, ElementKind, GROUND};
pub use materials::Material;
pub use solve::{solve, solve_at, ElementResult, NodeResult, Output, Reason, Solution};
pub use wire::{Warning, Wire};
