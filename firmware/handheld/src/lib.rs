//! The board's drivers, shared by the firmware (`src/main.rs`) and the test binaries in `src/bin`:
//! the display and its touch controller on one SPI bus, the SD card, a USB keyboard, and WLAN;
//! with the `duo` feature also the Duo's main screen on MIPI-DSI and its touch.

pub mod c6;
pub mod display;
#[cfg(feature = "duo")]
pub mod dsi;
#[cfg(feature = "duo")]
pub mod goodix;
#[cfg(feature = "duo")]
pub mod jd9365;
pub mod keyboard;
pub mod sd;
pub mod store;
pub mod touch;
pub mod wlan;
