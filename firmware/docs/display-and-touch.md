# Display and touch

The prototype display is a 3.5" 480×320 RPi display with an ILI9486 and an XPT2046 resistive
touch controller, plugged onto the dev kit's header ([dev-kit.md](dev-kit.md) has the pins;
[hardware.md](hardware.md) §3 the product's display). The driver is
`firmware/handheld/src/display.rs`, the touch `firmware/handheld/src/touch.rs`.

## First start

At boot the display shows red, green, blue and white bars with their names, and a black square
in the top left corner, for 1.5 s. Wrong colours (red and blue swapped) or a turned picture: change
`MADCTL` in `src/display.rs` (0x28, 0x20 without BGR, 0xE8 turned by 180°). The serial monitor
prints when the display was ready and how long the first frame took.

## SPI clock

The display runs at 26.67 MHz (80/3), the fastest the display's shift registers take cleanly;
40 MHz (80/2) garbles the picture. `QUANTUS_SPI_MHZ` at build time sets another; the P4 takes the
fastest whole divisor of 80 MHz not above it. Two test binaries help when trying one (testing
only; `cargo run --release` flashes the normal firmware again):

- `cargo run --release --bin bench`: a test picture, then full screens in a loop, printing the
  milliseconds per screen. Wrong colours, shifted lines or noise mean the clock is too fast.
- `cargo run --release --bin tour`: the simulator's UI tour on the board with a wallpaper,
  printing how long each key took until the screen was sent.

## Touch

The touch controller shares SCLK, MOSI and MISO with the display and runs at 1 MHz. It is
polled and a press is told by its pressure, so TP_IRQ is not needed. A loose MISO wire shows as
readings of 0; the firmware skips them, but touch then feels hesitant.

## Calibration

The panel needs calibrating once: at the first start the calibration screen comes up by itself,
later it is in SETUP → Gerät → Touch kalibrieren. Touch each cross and hold it briefly; if the
result is more than 20 px off, it starts again. The calibration is kept in flash (NVS) and
survives flashing new firmware.

Each press prints its raw reading and where it landed (`touch: raw x …, y … at …`).
`QUANTUS_TOUCH_DEBUG=1 cargo run --release` also prints the raw readings four times a second,
pressed or not.
