# Calculator hardware

Requirements for a handheld calculator that runs `quantus-core` on an ESP32-P4. It follows the
ESP32 spike (`firmware/esp32`, [plans.md](../plans/plans.md) §4), which ran out of RAM on a
classic ESP32.

## 1. Goal

A handheld calculator that computes locally with `quantus-core`. Near-instant boot comes first,
long battery life second. Later, and optional: hand calculations that take longer than about
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
- **R9** Runs from a LiPo cell for about a week of normal use, with good power saving (§7).

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
- The UI is Rust with embedded-graphics (`firmware/ui`), shared with a PC simulator
  (`firmware/sim`). It draws the screen in bands of 480×32 pixels into a 30 KB buffer and sends
  only the bands that changed, so it needs no framebuffer; with PSRAM the bands can grow. The UI
  and touch run on one core, the engine on the other, so a long calculation does not freeze the
  screen.
- Off is deep sleep, entered with a key, so switching on is a wake-up rather than a cold boot.

## 6. Open questions

- **Power**: the real capacity of the cell and the real currents of the board, display and
  backlight, to be measured (§7). Whether the P4 board can be fed at its 3.3 V pin, and what
  the display module's 5 V pin drives.
- **Display mounting**: solder the RPi module by its header (about 10 mm tall, simple), or a bare
  ILI9486 panel on a flex cable with the XPT2046 on our PCB (thinnest, more design work). This
  decides how well R1 is met.
- **P4 board**: its footprint, and where the USB port and SD slot sit relative to the board edge.
  R2 and R5 constrain the PCB outline.
- **Enclosure and keycaps**: how they are made.

## 7. Power

### Cell

One LiPo pouch cell, 606090 (6 × 60 × 90 mm), 3.7 V, rated 4000 mAh, with its own protection
board (cut-off at 3.0 V and 4.2 V, at most 1.5 A). 4000 mAh is a lot for that volume (about
450 Wh/l); plan with about 3000 mAh until the cell is measured. Pouch cells swell with age, so
the case leaves about 10 % room in thickness, and the cell sits away from the regulators and
the P4. It connects with its leads and a connector (JST-PH), not soldered at the tabs.

### Power chain

```
USB-C ──► charger with power path ──► system rail ──► buck-boost 3.3 V ──► P4 board, display, keypad
                    │
                LiPo cell ── fuel gauge
```

- **Charger** with a power path (e.g. BQ24074), so the calculator runs from USB while charging
  and with a flat cell. 0.6 A charge current, as the cell's maker recommends; a linear charger
  at 1 A would put about 1.3 W of heat into a thin case.
- **Buck-boost to 3.3 V** (e.g. TPS63802), so the cell is used down to its end. The board is fed
  at 3.3 V, not boosted to 5 V and regulated down again.
- **Fuel gauge** MAX17048 on I²C, for the battery level on screen. Below about 3.3–3.4 V the
  firmware saves its state to the SD card and goes to deep sleep, before the cell's protection
  board cuts the power.

### Budget (estimates, to be measured)

| State | Current |
|---|---|
| P4 computing, Wi-Fi off | 100–200 mA |
| Backlight, full brightness | 50–80 mA |
| In use, total | 200–250 mA |
| Off (deep sleep) | set by the board's regulator, USB bridge, LEDs and display module |

About 12–15 h of active use from 3000 mAh, about a week of normal use.

### Power saving

- **Backlight**: the largest single load. Switched by a MOSFET with PWM, so it can be dimmed: a
  brightness setting, dimmed after some seconds without input, off after a longer pause, back
  on with any key or touch.
- **Display**: sent to sleep (ILI9486 `SLPIN`) when the screen is off, and its module cut by a
  load switch in deep sleep.
- **P4**: ESP-IDF power management with frequency scaling and automatic light sleep, so it idles
  between key presses and only runs at full speed during a calculation.
- **Wi-Fi**: the C6 stays off unless the remote engine or an update needs it.
- **Off**: deep sleep (§5). Deep sleep turns the SRAM and PSRAM off, so what must survive goes
  to the SD card ([handheld-menus.md](handheld-menus.md) §6). Power LEDs on the boards are
  removed; the SD card is cut by a load switch if the board allows it.
