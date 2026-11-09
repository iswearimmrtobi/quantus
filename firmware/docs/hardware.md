# Calculator hardware

Requirements for a handheld calculator that runs `quantus-core` on an ESP32-P4. It follows the
ESP32 spike (`firmware/esp32`, [plans.md](../../plans/plans.md) §4), which ran out of RAM on a
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
- **R8** Touch input in addition to the keys. Nothing may need touch: every function is
  reachable from the keys.
- **R9** Runs from a LiPo cell for about a week of normal use, with good power saving (§7).
- **R10** Two sizes, one firmware: 77 mm wide (as the Casio) with a 2.8" display, and about 97 mm
  wide with a 3.5" display (§4). The UI adapts to the screen.

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
  the cable, and a datasheet for its backlight. The prototype's controller, an ST7796S, is the common one for these.
  The exact panel is still to be chosen.
  - Why SPI and not DPI (RGB parallel): the panel keeps the picture in its own RAM, so a static
    screen costs the P4 nothing and a keypress only sends the bands that changed (about 9 ms
    each). A DPI panel needs a framebuffer scanned out about 60 times a second from PSRAM. That
    costs current all the time and needs about 20 GPIOs instead of 5. For a bigger or sharper
    screen, MIPI-DSI would be the better step, not DPI.
  - Pick a panel whose flex brings out `TE` (sometimes called `FMARK`), and wire it to a GPIO,
    so full redraws can wait for it and don't tear. The prototype's board doesn't bring it
    out, so the prototype can tear.
  - The prototype's ST7796S already runs at 40 MHz; whether more is possible needs measuring.
- **Display, prototype**: the Waveshare 3.5" Capacitive Touch LCD: an ST7796S over 4-wire SPI
  (plain 8-bit commands), 320×480 turned to landscape, with its own backlight pin, or any
  display with a separate backlight pin. The display RAM cannot be read back.
- **Touch**: on the prototype an FT6336U, capacitive, on I²C (0x38), polled, with a one-time
  calibration. The product uses what the chosen panel brings: a capacitive controller on I²C, or
  a resistive panel with an XPT2046 on the display's SPI bus.
- **Dev board**: until our PCB exists, a Waveshare ESP32-P4-Module-DEV-KIT; its pinout and the
  display's pins on it are in [dev-kit.md](dev-kit.md).
- **Display pins, prototype**: see [dev-kit.md](dev-kit.md#the-35-capacitive-touch-display-on-the-header)
  (SCLK, MOSI, LCD_CS, LCD_DC, LCD_RST, LCD_BL on SPI3 and GPIOs, TP_SDA and TP_SCL on the
  board's I²C, TP_RST).

- **Keypad**: low-profile SMD tactile switches under printed keycaps, in an 8 × 10 matrix (80
  positions) read by three MCP23017 (16-bit I²C port expanders). The ON key is the same switch
  on the same PCB, but wired to the power latch, not the matrix (§7).
  - **Bus**: the three chips sit on an I²C bus of their own, so a scan does not hold up touch,
    RTC and fuel gauge. Their address pins give three addresses, and one P4 GPIO drives the
    `RESET` of all three.
  - **Chip 1**: the 8 rows, inputs with pull-ups, on port A, with interrupt-on-change (INT to a
    P4 GPIO). **Chip 2**: the 10 columns, outputs, on port A and two pins of port B. **Chip 3**:
    general I/O. With the matrix on those pins, port B of chip 1 (8 pins) and 6 pins of chip 2
    stay free as well.
  - **Scanning**: at rest all columns are held low, a press raises INT, and only then does the
    firmware scan (one column write and one row read per step, about 2–3 ms for ten columns at
    400 kHz, to be measured) until every key is released. The chips have no debounce and no
    event FIFO, the firmware does both.
  - **No diodes**: SHIFT, ALPHA and UNIT latch, so at most two keys are held together.
  - **Why not a scanner chip**: the TCA8418 was planned first but is out of stock or has long
    lead times. Alternatives found, not checked against their datasheets: ADP5588 (8 × 10
    keys), ADP5589 (11 × 8), STMPE1801 (10 × 8 and 4 dedicated keys), LM8330 (TI again). The
    MCP23S17 is the SPI version with the same registers, for a faster scan.
- **Clock**: a small RTC, e.g. RV-3028, on I²C. The 3.3 V rail is off when the calculator is, so
  the P4 cannot keep the time. Main supply from 3.3 V, backup supply from the cell (§7).

## 4. Keypad layout

Modelled on the Casio fx-DE line, not a copy. The two sizes run the same firmware and share one
set of logical keys (`firmware/ui/src/key.rs`). The big one adds shortcuts for what the small one
reaches through the soft keys or a menu, so no function exists on one of them only.
[keypad-sizes.html](../../design/keypad-sizes.html) draws both to scale from one table of sizes.

| | Small | Big |
|---|---|---|
| Width | 77 mm, as the Casio | about 97 mm |
| Display | 2.8" (panel not chosen; the sketch assumes 320×240) | 3.5", 480×320 |
| Length (sketch) | about 171 mm | about 182 mm |
| Soft keys | 2 × 5 | 2 × 6 |
| Keys | 68 | 76, of which 6 are extras |

All keys but ON sit in the 8 × 10 matrix (§3): 67 and 75 keys, 13 and 5 positions spare.

Top to bottom:

- **Soft keys**: two rows of plain keys directly below the screen, 5 mm high, spaced to the
  width of the screen. Their labels are drawn in two strips at the bottom of the screen: the top
  key row in the upper strip, the second row in the lower one, so F1 is top left on both, tinted
  differently on screen and on the keycaps. A mode with no use for them (linear systems, graph)
  hides the strips; the keys are then inert, never a hidden function. The labels follow the
  screen: units and symbols, algebra commands, result forms ([unit-keys.md](unit-keys.md)).
- **Control row**: round keys in one row: SHIFT, ALPHA, UNIT, OPTN, MODE, ON.
- **Arrow pad**: four arrows (smaller than the other keys) and OK, with SYMB, CALC and VARS at
  its corners (placeholder positions).
- **Function block**: 6 columns × 4 rows, keys 5 mm high.
- **Number block**: 5 columns × 4 rows, larger keys. EXE sits at the bottom right where the
  Casio has `=`. The `=` of equations is a key of the function block (`Key::Equals`).
- **Big unit only**: UNDO, REDO, ▶unit, x³, 10ˣ, Σ and ∫.

| Key (mm) | Small | Big |
|---|---|---|
| Function | 9.3 × 5 | 10.6 × 5 |
| Soft | 10.3 × 5 | 10.4 × 5.5 |
| Control (round) | Ø 8 | Ø 8.5 |
| Arrows | 7 × 5.5 | 7.5 × 5.5 |
| Number | 11.4 × 8.5 | 15.6 × 9 |

These sizes are a first guess, to be tried on real keycaps.

Legends: SHIFT yellow, ALPHA red, UNIT lavender, at most three per key. One theme per modifier:
SHIFT the second function (inverse, rare command) on the function block and a few control keys,
UNIT the units on the function block and the prefixes on the digits, ALPHA the variables A–F and M
on the digits (`7 8 9 → A B C`, `4 5 6 → D E F`, `1 → M`), so no key carries four legends. ALPHA
does not carry the alphabet: names are rarely typed, and touch is never the way in. A few keys
also have an ALPHA legend of the Casio's: `(` types %, `,` types RanInt#. Which legend sits
under which modifier is the table in [handheld-menus.md](../../design/handheld-menus.md) §7.4.1. SYMB opens Greek letters and math symbols on the soft keys. Typing units fast and the
soft keys' context are in [unit-keys.md](unit-keys.md).

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
- **Case length**: the sketch comes out at about 171 mm (small) and 182 mm (big), against the
  Casio fx-991's 162 mm, because of the taller display and the two soft-key rows. Shorter rows or
  a smaller pitch would cut it.
- **Small display**: the 2.8" panel is not chosen. Its size and resolution decide the UI profile
  (the UI draws 480-wide bands today).
- **Keypad chips**: confirm against the MCP23017 datasheet the I²C speed, the internal pull-up
  value, the standby current and the scan time; the ADP5588 is the fallback. SYMB, CALC and VARS
  have no fixed place yet.

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
  survive goes to the SD card ([handheld-menus.md](../../design/handheld-menus.md) §6).
