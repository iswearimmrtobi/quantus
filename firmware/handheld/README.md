# Quantus handheld firmware

The handheld's UI (`firmware/ui`) and the engine (`quantus-core`, through `firmware/engine`) on
the Waveshare ESP32-P4-Module-DEV-KIT with the 3.5" capacitive touch display, with keys typed into the serial
monitor or on a USB keyboard, or the touch panel.

## The engine

The engine runs on a thread of its own with a 64 KB stack, so the screen, the keys and the
touch panel stay live while it calculates; the spinner turns in the top bar meanwhile. AC drops
the calculation: the input stays, the answer is not shown. The engine still finishes it, and a
calculation sent after the AC waits for that. The other keys wait until the answer is there;
MODE, SETUP and OPTN still open.

Each calculation is printed over serial: the mode, the LaTeX the editor wrote, the result and
the time, then the most stack the engine thread has used and the heap (all of it and the
internal RAM). The engine keeps its unit table, in PSRAM (no `small-memory`).

Exact, a rational result is a fraction, improper or mixed as SETUP says, as long as it has at
most ten digits (`\frac{5}{6}`, `\frac{458}{675}\,\mathrm{J}`); a longer one is a decimal.
S⇔D switches to the decimal and back. The simulator does the same.

`--no-default-features` builds the stub instead of the engine, which builds and flashes much
faster for work on the UI: it answers with the input itself, and an input with a `?` gives an
error.

WLAN goes through the C6 on a thread of its own ([wlan.md](../docs/wlan.md)): SETUP →
Verbindungen → WLAN switches it, finds networks, keeps their passwords and shows the connection;
the clock comes from the network (SETUP → Verbindungen → Uhrzeit), or is set there by hand. The
settings are kept in flash.

Apps and games are Lua scripts ([apps.md](../docs/apps.md)): `build.rs` compiles Lua from
`lua/` with the ESP toolchain, and the script's thread takes its stack and memory from PSRAM.
`QUANTUS_APPS_DEBUG=1 cargo run --release` logs every callback of an app with its time, the
script's stack and the heap.

`QUANTUS_SCREEN_DEBUG=1` prints a line after every change on the screen: the mode, the Basis-N
settings, the menu entry selected, and the input and the entry shown with its result and view.
With the keys sent over the serial line, the board can be tested without looking at it.

`QUANTUS_USB_DEBUG=1` logs every report of the USB keyboard (its eight bytes, with the time in
ms) and every key it gives, repeats included: to tell a keyboard that sends a key twice from the
key repeat.

The same UI runs in a window on the PC: [firmware/sim](../sim/README.md).

```sh
. ~/export-esp.sh
cd firmware/handheld
cargo run --release          # flash and open the serial monitor
cargo run --release --no-default-features   # the same with the stub instead of the engine
```

The hardware, setup and wiring are in [firmware/docs](../docs/README.md):

- [Setup, build and flash](../docs/setup.md), including the serial port on Linux
- [The dev kit and the display's pins](../docs/dev-kit.md#the-35-capacitive-touch-display-on-the-header)
- [Display and touch](../docs/display-and-touch.md): first start, colours, calibration, test binaries
- [SD card](../docs/sd-card.md): formatting, wallpapers and sessions
- [WLAN](../docs/wlan.md): Wi-Fi through the C6, saved networks, network time
- [Mirror](../docs/mirror.md): the board as the simulator's screen

## Keys

Type into the serial monitor; the keys are those of the simulator, see its
[key table](../sim/README.md#keys) (Ctrl+F then Tab for the setup). The arrows work as arrows,
Delete clears like AC, F5 is OK and Shift+F1–F12 are the soft keys (as a terminal sends them,
`ESC [ 1 ; 2 P` and so on, which an app needs for its soft keys). Ctrl+C and Ctrl+R are the monitor's own (exit, reset) and not used as keys.

A USB keyboard in the board's USB-A port types the same keys, with a German (QWERTZ) layout as
in the simulator, F1–F12 and Shift+F1–F12 (the soft keys) included: F1–F4 are SHIFT, ALPHA, S⇔D
and OPTN, F9 UNIT, F10 ENG, Tab is MODE, the keypad works, AltGr
types `@`, braces, brackets, `\`, `~` and `|` for a WLAN password. The arrows and Backspace repeat while held (after 600 ms, then every 100 ms). It is a HID boot keyboard
(`src/keyboard.rs`, the layout in `firmware/ui/src/hid.rs`); one at a time, plugged in before or
after the start, and the serial monitor keeps working beside it. Its hookup is in
[dev-kit.md](../docs/dev-kit.md#usb-keyboard).

## Touch

Every menu works by touch, as in the simulator: a tap opens a line or picks a tile, ‹ at the
left of the top bar goes back, a list follows the finger up and down and turns its page when let
go, the mode tiles do the same sideways, and a number is set with − and + and OK. The
panel is calibrated once ([display-and-touch.md](../docs/display-and-touch.md#calibration)).
