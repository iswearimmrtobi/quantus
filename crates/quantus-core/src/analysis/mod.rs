//! Analysis mode (Kurvendiskussion): a function of one variable in, the full curve sketch with
//! its working out. See `docs/analysis-mode.md`.

pub mod exact;
pub mod expr;
pub mod simplify;

pub use expr::{Expr, Fn};
