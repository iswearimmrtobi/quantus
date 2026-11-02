# Quantus handheld firmware

The handheld's UI (`firmware/ui`) and the engine (`quantus-core`, through `firmware/engine`) on
the Waveshare ESP32-P4-Module-DEV-KIT with the 3.5" RPi display, with keys typed into the serial
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

`--no-default-features` builds the stub instead of the engine, which builds and flashes much
faster for work on the UI: it answers with the input itself, and an input with a `?` gives an
error.

The same UI runs in a window on the PC: [firmware/sim](../sim/README.md).

```sh
. ~/export-esp.sh
cd firmware/handheld
cargo run --release          # flash and open the serial monitor
cargo run --release --no-default-features   # the same with the stub instead of the engine
```

The hardware, setup and wiring are in [firmware/docs](../docs/README.md):

- [Setup, build and flash](../docs/setup.md), including the serial port on Linux
- [The dev kit and the display's pins](../docs/dev-kit.md#the-35-rpi-display-on-the-header)
- [Display and touch](../docs/display-and-touch.md): first start, colours, calibration, test binaries
- [SD card](../docs/sd-card.md): formatting and wallpapers
- [Mirror](../docs/mirror.md): the board as the simulator's screen

## Keys

Type into the serial monitor; the keys are those of the simulator, see its
[key table](../sim/README.md#keys) (Ctrl+F then Tab for the setup). The arrows work as arrows,
Delete clears like AC. Ctrl+C and Ctrl+R are the monitor's own (exit, reset) and not used as keys.

A USB keyboard in the board's USB-A port types the same keys, with a German (QWERTZ) layout as
in the simulator: F1–F4 are SHIFT, ALPHA, S⇔D and OPTN, Tab is MODE, the keypad works, AltGr
types nothing. The arrows and Backspace repeat while held. It is a HID boot keyboard
(`src/keyboard.rs`, the layout in `firmware/ui/src/hid.rs`); one at a time, plugged in before or
after the start, and the serial monitor keeps working beside it. Its hookup is in
[dev-kit.md](../docs/dev-kit.md#usb-keyboard).

## Touch

Every menu works by touch, as in the simulator: a tap opens a line or picks a tile, ‹ at the
left of the top bar goes back, a list follows the finger up and down and turns its page when let
go, the mode tiles do the same sideways, and a number is set with − and + and OK. Until the
keypad exists, the calculation screen has MENU, SETUP and OPTN buttons along the bottom. The
panel is calibrated once ([display-and-touch.md](../docs/display-and-touch.md#calibration)).
