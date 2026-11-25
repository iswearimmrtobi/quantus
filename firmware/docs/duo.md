# Quantus Duo

A second handheld with two screens in a clamshell: a large screen on the lid that shows the
calculation, and a small touch screen in the base that takes the place of the keypad. It runs
the same UI and engine as the handheld ([hardware.md](hardware.md)); what differs is the
screens, the input and the case. State on 2026-11-25: specified, not built.

## 1. Concept

- **Main screen** (lid): shows the calculation, results, graphs and menus, as the handheld's
  screen does. Touch on it is for tapping what one sees (a line, a tile, a cell), not for typing.
- **Input screen** (base): a 3.5" touch screen that draws the keys. Each mode can draw the keys
  it needs, so the layout suits what is on the main screen: the calculator's keys, a hex
  keypad in Base-N, the soft keys of the mode as real buttons rather than labels over physical
  keys.
- **Buttons**: a few hardware buttons for what must work without looking: EXE, MENU, the four
  arrows and power. Everything else is on the input screen.

## 2. Main screen

The Waveshare 5-DSI-TOUCH-A, <https://www.waveshare.com/wiki/5-DSI-TOUCH-A>:

| | |
|---|---|
| Panel | 5" IPS, 720×1280, portrait native; used landscape as 1280×720, about 294 ppi |
| Controller | HX8394, MIPI-DSI 2 lanes (ESP-IDF component `waveshare/esp_lcd_hx8394`) |
| Touch | GT911 capacitive, I²C, address 0x5D or 0x14 (set at reset) |
| Backlight | over I²C: device 0x45, register 0x96, 0x00–0xFF |
| Supply | 5 V, about 0.5 A |
| Size | outline and active area in Waveshare's drawing (PDF and DWG on the wiki), about 110 × 70 mm |

The panel has no frame memory of its own: the P4 keeps a framebuffer in PSRAM (720 × 1280 × 2
bytes, 1.8 MB) and its DSI controller sends it to the panel about 60 times a second. Because the
panel is portrait, each picture is turned by 90° on its way into the framebuffer: the UI still
draws bands, and each band is written turned (by the PPA, the P4's 2D unit, or in software as a
first step). The band renderer's checks for unchanged bands stay: only what changed is drawn and
turned, and the DSI scanout repeats the rest by itself.

The UI draws at the native 1280×720 with larger fonts, not a 640×360 picture doubled: at about
11.6 px/mm against the 3.5" screen's 6.5, the same physical layout needs about 1.75× the
pixels, and text is sharper for it.

## 3. Input screen

The handheld's display: a 3.5" 480×320 ST7796S over SPI with the FT6336U touch controller on
I²C at 0x38 ([display-and-touch.md](display-and-touch.md)). Its driver is the handheld's and does
not change. Its touch shares the I²C bus with the main screen's GT911 and backlight; the three
addresses do not collide.

What it shows:

- The mode's soft keys as a row of buttons (they leave the bottom of the main screen).
- A grid of keys in pages: first the handheld's big keypad
  ([keypad-sizes.html](../../design/keypad-sizes.html)), later layouts of each mode's own.
- The SHIFT, ALPHA and UNIT layers: the keys show the layer in force.

A tap sends the same `Key` a physical key would, so the UI's logic is the handheld's.

## 4. Shared with the handheld

The ESP32-P4 module with the C6 for Wi-Fi, the SD card, the engine, the UI and its menus,
sessions, apps and settings. Power follows [hardware.md](hardware.md) §7, with a 5 V rail
added for the main screen.

## 5. Open questions

- **DSI timings**: the wiki gives no lane rate, pixel clock or porches for this panel; take
  them from Waveshare's `esp_lcd_hx8394` component or BSP.
- **Size**: outline, active area, the FPC's length and where it leaves the panel, from the
  drawing.
- **Power**: the main screen's real draw at usual brightness, measured; the 5 V boost from the
  cell; how long the cell lasts with both screens.
- **Hinge**: the flex cables through it (DSI and touch to the lid), and whether the lid holds
  only the panel.
- **Buttons**: where they sit and how many; whether power also wakes from the input screen.
- **Touch on the main screen**: GT911 interrupt or polled, as the FT6336U is.
