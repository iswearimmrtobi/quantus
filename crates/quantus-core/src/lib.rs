//! Quantus calculation engine: LaTeX in, exact physical quantities, LaTeX out.

pub mod algebra;
pub mod analysis;
pub mod api;
pub mod basen;
pub mod constants;
pub mod error;
pub mod forms;
pub mod functions;
pub mod geometry;
pub mod electro;
pub mod eval;
pub mod kirchhoff;
pub mod latex;
pub mod library_search;
pub mod matrix;
pub(crate) mod linear;
pub mod num;
pub(crate) mod numeric;
pub mod quantity;
pub mod remote;
pub mod session;
pub mod settings;
pub mod stochastics;
pub mod text;
pub mod units;
pub mod writeup;

pub use error::{QuantusError, Result};
pub use quantity::PhysicalQuantity;
