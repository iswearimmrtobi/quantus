//! Analysis mode (Kurvendiskussion): a function of one variable in, the full curve sketch with
//! its working out. See `docs/analysis-mode.md`.

pub mod derive;
pub mod domain;
pub mod exact;
pub mod expr;
pub mod interval;
pub mod limits;
pub mod polydiv;
pub mod rules;
pub mod simplify;
pub mod solve;

pub use expr::{Expr, Fn};
