//! Kirchhoff mode: a resistor network solved the way a student does it, with junction and loop
//! rules, the linear system and its Gauss steps. UI-independent, no state of its own.

pub mod graph;

pub use graph::{Edge, Graph, Point};
