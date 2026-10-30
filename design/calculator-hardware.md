# Calculator hardware

Requirements for a handheld calculator that runs `quantus-core` on an ESP32-P4. It follows the
ESP32 spike (`firmware/esp32`, [plans.md](../plans/plans.md) §4), which ran out of RAM on a
classic ESP32.

## 1. Goal

A handheld calculator that computes locally with `quantus-core`. Near-instant boot comes first;
power use is secondary. Later, and optional: hand calculations that take longer than about
500 ms to a remote engine over Wi-Fi, the same LaTeX in, LaTeX out boundary as the desktop.

## 2. Requirements

- **R1** One PCB only, to keep the profile low.
- **R2** An SD card, using the reader on the ESP board. The slot must be reachable from outside
  the case.
- **R3** The ESP32-P4 board is soldered directly to the PCB.
- **R4** The display is soldered directly to the PCB.
- **R5** The USB port is exposed, for flashing and power.
- **R6** A custom boot screen.
- **R7** Near-instant boot: the first screen within a few hundred milliseconds.
- **R8** Touch input in addition to the keys.

## 3. Components

- **MCU**: an ESP32-P4 board with an ESP32-C6 on board for Wi-Fi (ESP-Hosted). 32 MB PSRAM.
  Exact board name and footprint: to be filled in.
- **Display**: 3.5", 480×320, an RPi display with an ILI9486 over SPI, no HDMI. These boards
  often put shift registers in front of the controller, so commands and data go out as 16-bit
  words. The display RAM cannot be read back.
- **Touch**: XPT2046, resistive, on the same SPI bus as the display with its own chip select,
  at most ~2 MHz. Polled only while `TP_IRQ` is low. Needs a one-time calibration.
- **Display pins** (Waveshare (A) layout, to verify on the board):

  | Signal | Pi header | Purpose |
  |---|---|---|
  | SCLK, MOSI, MISO | GPIO 11, 10, 9 | Shared SPI bus |
  | LCD_CS | GPIO 8 (CE0) | Display select |
  | TP_CS | GPIO 7 (CE1) | Touch select |
  | LCD_RS | GPIO 24 | Command or data |
  | RST | GPIO 25 | Display reset |
  | TP_IRQ | GPIO 17 | Low while touched |

- **Keypad**: low-profile SMD tactile switches under printed keycaps, read by a TCA8418
  (I²C, up to 80 keys, debounced, with an interrupt line).

## 4. Keypad layout

Modelled on the Casio fx-DE line, not a copy:

- a top row of 4 keys,
- a navigation cluster: 4 directions and OK,
- a 4×6 block of function keys,
- a 4×5 block of number and operator keys at the bottom.

About 53 keys in total.

## 5. Firmware requirements that affect the hardware

- The boot screen lives in flash, not on the SD card, and is drawn before the engine is touched.
  `CONFIG_SPIRAM_MEMTEST` is off.
- A full framebuffer in PSRAM with partial redraws (LVGL). The UI and touch run on one core,
  the engine on the other, so a long calculation does not freeze the screen.
- Off is deep sleep, entered with a key, so switching on is a wake-up rather than a cold boot.

## 6. Open questions

- **Power**: USB-C only, or a LiPo with a charger that has a power path (e.g. BQ24074) and a
  battery gauge (e.g. MAX17048).
- **Display mounting**: solder the RPi module by its header (about 10 mm tall, simple), or a bare
  ILI9486 panel on a flex cable with the XPT2046 on our PCB (thinnest, more design work). This
  decides how well R1 is met.
- **P4 board**: its footprint, and where the USB port and SD slot sit relative to the board edge.
  R2 and R5 constrain the PCB outline.
- **Enclosure and keycaps**: how they are made.
