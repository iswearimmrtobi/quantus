# Setup, build and flash

Both firmwares (`firmware/handheld` and `firmware/esp32`) build for the ESP32-P4 dev kit
([dev-kit.md](dev-kit.md)) on Rust `std` via ESP-IDF. Each is its own Cargo workspace, so
`cargo test --workspace` and the desktop build don't change.

## Toolchain (once)

```sh
cargo install espup espflash ldproxy --locked
espup install --targets esp32p4
```

The ESP-IDF tools go to `~/.espressif` and are shared by both firmwares.

## Build and flash

```sh
. ~/export-esp.sh            # every new shell
cd firmware/handheld         # or firmware/esp32
cargo build --release        # first build downloads and compiles ESP-IDF (~1 GB, several minutes)
cargo run --release          # flash and open the serial monitor
```

Flash and monitor over the port marked **USB TO UART** (UART0, the console). The other port,
**USB**, is the P4's own USB and can flash too.

## Serial port on Linux

The **USB TO UART** port shows up as `/dev/ttyACM0` (a CH343 bridge), owned by the group `uucp`
(Arch) or `dialout` (Debian). Add yourself to that group once and log in again, or grant access
after each replug:

```sh
sudo setfacl -m u:$USER:rw /dev/ttyACM0
```

`espflash` lives in `~/.cargo/bin`. To read the boot log without an interactive monitor:

```sh
timeout 10 espflash monitor --port /dev/ttyACM0 --non-interactive
```

The serial monitor takes the port alone; close it before anything else (such as the
[mirror](mirror.md)) uses it.

## Buttons and resets

- **RESET** restarts the board; `espflash reset` does the same from the PC.
- **BOOT** held while powering on or pressing RESET enters download mode, should flashing fail
  to start by itself.

The dev kit's chip is an ESP32-P4 rev v3.2; ESP-IDF v5.5's defaults already target v3.1 and later.
