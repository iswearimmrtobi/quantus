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
- **R2** An SD card, in a slot on our PCB. The slot must be reachable from outside the case.
- **R3** The ESP32-P4 module is soldered directly to the PCB.
- **R4** The display is fixed directly to the PCB: its flex cable is soldered or plugged in.
- **R5** One USB-C port on our PCB is exposed, for flashing and power.
- **R6** A custom boot screen.
- **R7** Near-instant boot: the first screen within a few hundred milliseconds.
- **R8** Touch input in addition to the keys.
- **R9** Runs from a LiPo cell for about a week of normal use, with good power saving (§7).

## 3. Components

- **MCU**: Waveshare ESP32-P4-Module, `-U` version: an ESP32-P4 with an ESP32-C6 for Wi-Fi
  (ESP-Hosted) and 32 MB PSRAM in the package, 25 × 25 mm with 1 mm castellated edges, fed at
  3.3 V. The `-U` version has an IPEX connector for the antenna. Our PCB adds:
  - the SD slot; if it uses GPIO39–48, firmware sets that domain's LDO to 3.3 V (default 1.8 V),
  - USB to GPIO24/25, which should be the USB Serial/JTAG pins (to verify, Waveshare does not
    label them so),
  - BOOT (GPIO35 to GND) and reset (`ESP_EN`) buttons inside the case, a 10 kΩ pull-up on
    GPIO36,
  - `VBAT` tied to 3.3 V, `ESP_EN` pulled high, 10 µF at the supply input.
- **Display, product**: a bare 3.5" 480×320 SPI panel on a flex cable with `LED+` and `LED−` on
  the cable, and a datasheet for its backlight. Many of these use an ST7796S, not an ILI9486,
  which is a small driver change. The exact panel is still to be chosen.
- **Display, prototype**: an RPi display with an ILI9486 over SPI, no HDMI, or any display with
  a separate backlight pin. These boards often put shift registers in front of the controller,
  so commands and data go out as 16-bit words. The display RAM cannot be read back.
- **Touch**: on the prototype an XPT2046, resistive, on the same SPI bus as the display with its
  own chip select, at most ~2 MHz. Polled only while `TP_IRQ` is low. Needs a one-time
  calibration. The product uses what the chosen panel brings: the same chip on our PCB for a
  resistive panel, or a capacitive controller on I²C.
- **Dev board**: until our PCB exists, a Waveshare ESP32-P4-Module-DEV-KIT; its pinout and the
  display's pins on it are in [esp32-p4-dev-kit.md](esp32-p4-dev-kit.md).
- **Display pins, prototype** (Waveshare (A) layout, to verify on the board):

  | Signal | Pi header | Purpose |
  |---|---|---|
  | SCLK, MOSI, MISO | GPIO 11, 10, 9 | Shared SPI bus |
  | LCD_CS | GPIO 8 (CE0) | Display select |
  | TP_CS | GPIO 7 (CE1) | Touch select |
  | LCD_RS | GPIO 24 | Command or data |
  | RST | GPIO 25 | Display reset |
  | TP_IRQ | GPIO 17 | Low while touched |

- **Keypad**: low-profile SMD tactile switches under printed keycaps, read by a TCA8418
  (I²C, up to 80 keys, debounced, with an interrupt line). The ON key is the same switch on the
  same PCB, but wired to the power latch, not the matrix (§7).
- **Clock**: a small RTC, e.g. RV-3028, on I²C. The 3.3 V rail is off when the calculator is, so
  the P4 cannot keep the time. Main supply from 3.3 V, backup supply from the cell (§7).

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
- Off cuts the 3.3 V rail, so switching on is a cold boot (§7). Deep sleep is not used: it loses
  SRAM and PSRAM as well, so it would save little, and it needs more current when off. The
  bootloader raises `KILL` first, and R7 is measured from a cold start.

## 6. Open questions

- **Power**: the real capacity of the cell and the real currents of the module, panel and
  backlight, to be measured (§7).
- **C6**: no enable pin is listed for it on the module. Check the schematic for how to keep it
  quiet during normal use. This no longer affects the current when off.
- **Panel**: which one, its backlight's voltage and current, and resistive or capacitive touch.
- **Parts to confirm**: which LTC2955 variant has the active-high EN output, the LTC2955's
  window for `KILL` after power-on, and the MAX17048 and RV-3028 figures, against their
  datasheets.
- **Enclosure and keycaps**: how they are made.

## 7. Power

### Cell

One LiPo pouch cell, 606090 (6 × 60 × 90 mm), 3.7 V, rated 4000 mAh, with its own protection
board (cut-off at 3.0 V and 4.2 V, at most 1.5 A). 4000 mAh is a lot for that volume (about
450 Wh/l); plan with about 3000 mAh until the cell is measured. Pouch cells swell with age, so
the case leaves about 10 % room in thickness, and the cell sits away from the regulators and
the P4. It connects with its leads and a connector (JST-PH), not soldered at the tabs.

### Power chain

There is no BMS chip. One cell needs no balancing, so the work is shared: the cell's protection
board handles over-voltage, under-voltage and over-current, the charger handles charging and the
power path, the gauge reports the level, and the firmware shuts down cleanly when the cell is low.

```
USB-C ── VBUS ─► charger with power path ── VSYS ─┬─► buck-boost 3.3 V ─► P4 module, display
  │                    │                          │        ▲ EN         logic, keypad, SD, RTC
  │                LiPo cell ── fuel gauge        │        │
  │                                               ├─► power latch ◄─ ON key
  │                                               │
  │                                               └─► boost 5 V ─► backlight ─► MOSFET (PWM)
  └── D+/D− ─► ESD ─► P4 USB
```

VBUS goes only to the charger, never to the module. Charging continues while the calculator is
off.

- **USB-C**: receptacle on our PCB, 5.1 kΩ on CC1 and CC2, USBLC6-2 ESD protection on D+/D−.
- **Charger** BQ24074, with a power path, so the calculator runs from USB while charging and
  with a flat cell. 0.6 A charge current, as the cell's maker recommends; a linear charger at
  1 A would put about 1.3 W of heat into a thin case.

  | Pin | Value | Result |
  |---|---|---|
  | `ISET` | 1.5 kΩ | about 0.59 A charge current |
  | `ILIM`, EN2 to OUT, EN1 to GND | 1.6 kΩ | about 1 A input limit, independent of the 3.3 V rail |
  | `TMR` | 72 kΩ | 9.6 h safety timer (7.2–12 h); the 5 h default would stop a 4000 mAh charge |
  | `CE` | GND | charging always enabled |
  | `ITERM` | open | stops at 10 % (about 60 mA) |
  | `TS` | 10 kΩ NTC (103AT-2) | blocks charging outside the safe temperature range |
  | IN, OUT, BAT | 1 µF, 10 µF, 4.7 µF | decoupling |
  | `CHG` | LED | shows charging while the screen is off, and also read by the P4 |
  | `PGOOD` | P4 GPIO | USB present |

  The NTC comes from a three-wire pack, or is glued to the cell. A fixed 10 kΩ would disable the
  check, and then nothing stops charging below 0 °C.
- **Buck-boost to 3.3 V** TPS63802, so the cell is used down to its end. 0.47 µH inductor (at
  least about 4 A saturation), 10 µF in, 2 × 22 µF out (effective values under DC bias), divider
  560 k / 100 k from the 500 mV reference. 2 A output, against about 0.7 A estimated worst case.
  The module is fed at 3.3 V, not boosted to 5 V and regulated down again.
- **Power latch** LTC2955 from VSYS, about 1.2 µA, driving the buck-boost's `EN` (pull-down,
  off by default). The ON key goes to `PB`. `INT` goes to a P4 GPIO, so firmware sees the key,
  and `KILL` comes from a P4 GPIO with a pull-down.
  - **On**: the key enables the regulator, the P4 boots and holds `KILL` high.
  - **Off**: the key raises `INT`, firmware saves to the SD card and drops `KILL`.
  - **Hang**: holding the key forces power off, which is also the hard reset. Unsaved input is
    lost.
- **Boost to 5 V** TPS61240 from VSYS, only for the backlight, with true load disconnect. `EN`
  from a P4 GPIO with a pull-down, so it is off whenever the P4 is.
- **Backlight**: run at its full voltage and dimmed only by PWM, so power follows the duty cycle.
  Low-side N-MOSFET (e.g. AO3400), about 20 kHz, 100 Ω gate resistor, 100 kΩ pull-down. Series
  resistor from the panel's datasheet: about 15 Ω for 6 parallel LEDs at 3.2 V and 120 mA.
- **Jumper** between the 3.3 V rail and the module's `ESP_3V3`, at least 1 A, inside the case.
  Display and keypad stay on our rail. It is also the link for measuring the module's current;
  a second link in series with the cell measures the total.
- **Fuel gauge** MAX17048 on I²C, directly at the cell, no sense resistor. 0.1 µF on VDD, QSTRT
  and CTG to GND, `ALRT` to a P4 GPIO. For the battery level on screen. Below about 3.3–3.4 V the
  firmware saves its state to the SD card and drops `KILL`, before the cell's protection board
  cuts the power.
- **RTC** RV-3028 or similar, main supply from 3.3 V, backup supply from the cell, so the time
  survives switching off.

### PCB

- **Placement**: display and keys on the front, the module behind the display, the cell behind
  the keys. The power parts go in a strip beside the cell, in the order USB-C, charger,
  regulator, close to the cell connector and off the cell's footprint: the charger can
  dissipate close to 1 W at mid-charge.
- **Buck-boost**: input and output capacitors and inductor tight on the same layer as the chip,
  nothing routed under the inductor, the divider at the chip and away from the inductor.
- **Charger**: several vias in the thermal pad to the ground plane, VSS connected by its pin as
  well, and `IN`, `OUT` and `BAT` traces wide enough for at least 1.5 A.
- **Gauge**: a separate trace from the cell connector, so load current does not skew it.
- **Cell connector**: check the lead's polarity before plugging in, it is not standardized
  between suppliers.

### Budget (estimates, to be measured)

| State | Current |
|---|---|
| P4 computing, Wi-Fi off | 100–200 mA |
| Backlight, full brightness | about 180 mA from the cell (120 mA at 5 V, example panel) |
| Backlight, 30 % duty | about 55 mA from the cell |
| In use, total, 30–40 % brightness | 200–250 mA |
| Off | about 15–35 µA: charger, gauge, cell protection, latch, regulators in shutdown |

About 12–15 h of active use from 3000 mAh, about a week of normal use. This needs a brightness
around 30–40 % as the normal level; at full brightness the total is about 300–380 mA. Peaks
stay under the cell's 1.5 A: about 0.7 A at 3.3 V with Wi-Fi and the SD card, about 0.85 A from
the cell at 3.0 V. When off, the cell's own self-discharge matters more than these currents.

### Power saving

- **Backlight**: the largest single load. PWM through the MOSFET: a brightness setting, dimmed
  after some seconds without input, off after a longer pause with the 5 V boost shut down, back
  on with any key or touch.
- **Display**: sent to sleep (`SLPIN`) when the screen is off.
- **P4**: ESP-IDF power management with frequency scaling and automatic light sleep, so it idles
  between key presses and only runs at full speed during a calculation.
- **Wi-Fi**: the C6 stays off unless the remote engine or an update needs it.
- **Off**: the latch cuts the 3.3 V rail (§5), by the ON key or the auto-off setting. The module,
  display, keypad and SD card all lose power, so no load switches are needed, and what must
  survive goes to the SD card ([handheld-menus.md](handheld-menus.md) §6).
