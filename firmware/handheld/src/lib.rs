//! The board's drivers, shared by the firmware (`src/main.rs`) and the test binaries in `src/bin`:
//! the display and its touch controller on one SPI bus, and the SD card.

pub mod display;
pub mod sd;
pub mod touch;
