# Display and touch

The prototype display is the Waveshare 3.5" capacitive touch LCD: 320×480, run in landscape as
480×320, with an ST7796S on SPI and an FT6336U touch controller on I²C, wired to the dev kit's header
with jumpers ([dev-kit.md](dev-kit.md) has the pins; [hardware.md](hardware.md) §3 the product's
display). The driver is `firmware/handheld/src/display.rs`, the touch `firmware/handheld/src/touch.rs`.

## First start

At boot the display shows red, green, blue and white bars with their names, and a black square
in the top left corner, for 1.5 s. Wrong colours (red and blue swapped) or a turned picture: change
`MADCTL` in `src/display.rs` (0x28, 0xE8 turned by 180°, 0x68 and 0xA8 mirrored). If the colours look inverted (black
on white), the panel needs the inversion command (0x21) in `INIT` toggled to 0x20. The serial monitor
prints when the display was ready and how long the first frame took.

## SPI clock

The display runs at 40 MHz (80/2). `QUANTUS_SPI_MHZ` at build time sets another; the P4 takes the
fastest whole divisor of 80 MHz not above it. If the picture shows noise or shifted lines
(jumper wires), try 27 (26.67 MHz). Two test binaries help when trying one (testing only;
`cargo run --release` flashes the normal firmware again):

- `cargo run --release --bin bench`: a test picture, then full screens in a loop, printing the
  milliseconds per screen. Wrong colours, shifted lines or noise mean the clock is too fast.
- `cargo run --release --bin tour`: the simulator's UI tour on the board with a wallpaper,
  printing how long each key took until the screen was sent.

## Touch

The FT6336U is polled over I²C (400 kHz, address 0x38): one read says whether a finger is down
and where. TP_INT is not needed. At start the log prints `touch: FT6336U ready`; another chip ID
is a warning; no answer at all is one too, with the addresses that do answer on the bus (check
SDA, SCL, TP_RST and the supply), and touch then stays silent. The
controller reports the panel's own coordinates, whatever way round it is mounted; the
calibration below turns them into screen points.

If touch does not answer, `cargo run --release --bin i2cscan` (testing only) prints the idle
levels of SDA and SCL and how often the audio codec (0x18) and the touch controller (0x38)
answer out of 20 probes. The codec should always answer; if it does not, the bus is the
problem (wiring, supply), not the firmware. Do not scan every address: the failed probes
disturb the bus and make the result look worse than it is.

## Calibration

The panel needs calibrating once (again after switching from the resistive display, whose
calibration is not used): at the first start the calibration screen comes up by itself,
later it is in SETUP → Gerät → Touch kalibrieren. Touch each cross and hold it briefly; if the
result is more than 20 px off, it starts again. The calibration is kept in flash (NVS) and
survives flashing new firmware.

Each press prints its raw reading and where it landed (`touch: raw x …, y … at …`).
`QUANTUS_TOUCH_DEBUG=1 cargo run --release` also prints the controller's registers four times a second,
touched or not.
