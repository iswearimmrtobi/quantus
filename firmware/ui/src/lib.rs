//! The handheld calculator's user interface (design/calculator-hardware.md): the input editor,
//! the 2D layout of math and the screen, drawn to any embedded-graphics target. No hardware and
//! no engine; the simulator (`firmware/sim`) and the board (`firmware/handheld`) feed it keys.

pub mod app;
pub mod backend;
pub mod editor;
pub mod font;
pub mod key;
pub mod latex;
pub mod layout;
pub mod render;
