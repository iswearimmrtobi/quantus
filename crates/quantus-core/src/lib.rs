//! Quantus calculation engine: LaTeX in, exact physical quantities, LaTeX out.

pub mod algebra;
pub mod analysis;
pub mod api;
pub mod basen;
pub mod error;
pub mod forms;
pub mod electro;
pub mod eval;
pub mod kirchhoff;
pub mod latex;
pub mod matrix;
pub(crate) mod linear;
pub mod num;
pub(crate) mod numeric;
pub mod quantity;
pub mod session;
pub mod settings;
pub mod stochastics;
pub mod text;
pub mod units;

pub use error::{QuantusError, Result};
pub use quantity::PhysicalQuantity;
