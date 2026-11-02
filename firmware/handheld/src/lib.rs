//! The board's drivers, shared by the firmware (`src/main.rs`) and the test binaries in `src/bin`:
//! the display and its touch controller on one SPI bus, the SD card, and a USB keyboard.

pub mod display;
pub mod keyboard;
pub mod sd;
pub mod touch;
