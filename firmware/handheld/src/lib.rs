//! The board's drivers, shared by the firmware (`src/main.rs`) and the test binaries in `src/bin`:
//! the display and its touch controller on one SPI bus, the SD card, a USB keyboard, and WLAN.

pub mod c6;
pub mod display;
pub mod keyboard;
pub mod sd;
pub mod store;
pub mod touch;
pub mod wlan;
