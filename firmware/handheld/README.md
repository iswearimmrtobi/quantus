# Quantus handheld firmware

The handheld's UI (`firmware/ui`) on a board with the 3.5" RPi display
([calculator-hardware.md](../../design/calculator-hardware.md)). Until the ESP32-P4 arrives it
runs on a classic ESP32 (WROOM, no PSRAM), with keys typed into the serial monitor and a stub
instead of the engine: the stub answers with the input itself, and an input with a `?` gives an
error. Each calculation is also printed over serial, with the LaTeX the editor wrote.

The same UI runs in a window on the PC: [firmware/sim](../sim/README.md).

## Wiring

The display's Pi header to the WROOM; the pins are constants at the top of `src/main.rs`.

| Display signal | Pi header | ESP32 |
|---|---|---|
| SCLK | GPIO 11 (pin 23) | GPIO 18 |
| MOSI | GPIO 10 (pin 19) | GPIO 23 |
| MISO | GPIO 9 (pin 21) | GPIO 19 |
| LCD_CS | GPIO 8 (pin 24) | GPIO 5 |
| TP_CS | GPIO 7 (pin 26) | GPIO 4, held high |
| LCD_RS | GPIO 24 (pin 18) | GPIO 21 |
| RST | GPIO 25 (pin 22) | GPIO 22 |
| 5 V | pin 2 | 5 V (VIN) |
| 3.3 V | pin 1 | 3V3 |
| GND | pin 6 | GND |

Touch (TP_IRQ) is not used yet.

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
