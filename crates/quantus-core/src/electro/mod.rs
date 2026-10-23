//! Electronics mode: passive DC circuits. UI-independent, no state of its own.

pub mod materials;
pub mod wire;

pub use materials::Material;
pub use wire::{Warning, Wire};
