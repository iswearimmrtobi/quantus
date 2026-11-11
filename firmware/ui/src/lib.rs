//! The handheld calculator's user interface (firmware/docs/hardware.md): the input editor,
//! the 2D layout of math and the screen, drawn to any embedded-graphics target. No hardware and
//! no engine; the simulator (`firmware/sim`) and the board (`firmware/handheld`) feed it keys
//! and touches.

pub mod app;
pub mod backend;
pub mod clock;
pub mod editor;
pub mod font;
pub mod hid;
pub mod key;
pub mod layer;
pub mod latex;
pub mod layout;
pub mod menu;
pub mod render;
pub mod search;
pub mod settings;
pub mod sheet;
pub mod soft;
pub mod text;
pub mod touch;
pub mod tour;
pub mod tree;
pub mod wallpaper;
