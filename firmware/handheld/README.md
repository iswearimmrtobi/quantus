# Quantus handheld firmware

The handheld's UI (`firmware/ui`) on a board with the 3.5" RPi display
([calculator-hardware.md](../../design/calculator-hardware.md)), here the Waveshare
ESP32-P4-Module-DEV-KIT, with keys typed into the serial monitor or the touch
panel, and a stub instead of the engine: the stub answers with the input itself, and an input
with a `?` gives an error. Each calculation is also printed over serial, with the LaTeX the editor wrote.

The same UI runs in a window on the PC: [firmware/sim](../sim/README.md).

## Wiring

The dev kit's 40-pin header has the Raspberry Pi layout, so the display plugs onto it pin for
pin (all 40 pins: [esp32-p4-dev-kit.md](../../design/esp32-p4-dev-kit.md)). The pins are set in
`open_display!` in `src/display.rs`.

| Display signal | Pi header | ESP32-P4 |
|---|---|---|
| SCLK | GPIO 11 (pin 23) | GPIO 0 |
| MOSI | GPIO 10 (pin 19) | GPIO 3 |
| MISO | GPIO 9 (pin 21) | GPIO 2 |
| LCD_CS | GPIO 8 (pin 24) | GPIO 36 |
| TP_CS | GPIO 7 (pin 26) | GPIO 32 |
| LCD_RS | GPIO 24 (pin 18) | GPIO 4 |
| RST | GPIO 25 (pin 22) | GPIO 1 |
| 5 V | pin 2 | 5V (pin 2) |
| 3.3 V | pin 1 | 3V3 (pin 1) |
| GND | pin 6 | GND (pin 6) |

Flash and monitor over the port marked **USB TO UART** (UART0, the console).

Until the P4 arrived this ran on a classic ESP32 (WROOM, VSPI: SCLK 18, MOSI 23, MISO 19,
LCD_CS 5, TP_CS 4, LCD_RS 21, RST 22); git history has that build setup.

The touch controller (XPT2046) shares SCLK, MOSI and MISO with the display and runs at 1 MHz.
It is polled and a press is told by its pressure, so TP_IRQ is not needed. A loose MISO wire
shows as readings of 0; the firmware skips them, but touch then feels hesitant.

## Build and flash

Setup as for the engine spike ([firmware/esp32](../esp32/README.md)). The ESP-IDF tools go to
`~/.espressif` and are shared.

```sh
. ~/export-esp.sh
cd firmware/handheld
cargo run --release          # flash and open the serial monitor
```

At boot the display shows red, green, blue and white bars with their names, and a black square
in the top left corner, for 1.5 s. Wrong colours (red and blue swapped) or a turned picture: change
`MADCTL` in `src/display.rs` (0x28, 0x20 without BGR, 0xE8 turned by 180°). The serial monitor
prints when the display was ready and how long the first frame took.

## Keys

Type into the serial monitor; the keys are those of the simulator, see its
[key table](../sim/README.md#keys) (Ctrl+F then Tab for the setup). The arrows work as arrows, Delete clears like AC. Ctrl+C
and Ctrl+R are the monitor's own (exit, reset) and not used as keys.

## Touch

Every menu works by touch, as in the simulator ([firmware/sim](../sim/README.md)): a tap opens a
line or picks a tile, ‹ at the left of the top bar goes back, a list follows the finger up and
down and turns its page when let go, the mode tiles do the same sideways, and a number is set
with − and + and OK. Until the keypad exists, the calculation screen has MENU, SETUP and
OPTN buttons along the bottom.

The panel needs calibrating once: at the first start the calibration screen comes up by itself,
later it is in SETUP → Gerät → Touch kalibrieren. Touch each cross and hold it briefly; if the
result is more than 20 px off, it starts again. The calibration is kept in flash (NVS) and
survives flashing new firmware. Each press prints its raw reading and where it landed
(`touch: raw x …, y … at …`). `QUANTUS_TOUCH_DEBUG=1 cargo run --release` also prints the raw
readings four times a second, pressed or not.

## Mirror (testing only)

`cargo run --release --bin mirror` flashes a second binary that only shows what the simulator
on the PC sends: the UI and the real engine run there, and the board is its screen. See
[On the board](../sim/README.md#on-the-board-testing-only). `cargo run --release` flashes the
normal firmware again.
