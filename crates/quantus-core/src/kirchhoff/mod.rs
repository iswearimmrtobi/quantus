//! Kirchhoff mode: a resistor network solved the way a student does it, with junction and loop
//! rules, the linear system and its Gauss steps. UI-independent, no state of its own.

pub mod branches;
pub mod graph;

pub use branches::{Branch, Kind, Member, Mesh, Network, Part, Turn};
pub use graph::{Edge, Graph, Point};
