# Quantus on an ESP32

A feasibility spike: does `quantus-core` build, fit and compute on a microcontroller? The firmware
runs a fixed list of inputs (one or two per category of `test/cases`) plus two curve sketches,
and prints each result with its time, the heap and the engine thread's stack use over serial.
There is no display and no keypad yet.

Target: a classic ESP32 (WROOM, 4 MB flash, 520 KB SRAM, no PSRAM), on Rust `std` via ESP-IDF.
For an ESP32-S3, change the target and `MCU` in `.cargo/config.toml`.

This crate is its own Cargo workspace, so `cargo test --workspace` and the desktop build don't
change.

## Setup (once)

```sh
cargo install espup espflash ldproxy --locked
espup install --targets esp32
```

## Build and flash

```sh
. ~/export-esp.sh            # every new shell
cd firmware/esp32
cargo build --release        # first build downloads and compiles ESP-IDF (~1 GB, several minutes)
cargo run --release          # flash and open the serial monitor
```
