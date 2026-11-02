# Quantus handheld firmware

The handheld's UI (`firmware/ui`) on the Waveshare ESP32-P4-Module-DEV-KIT with the 3.5" RPi
display, with keys typed into the serial monitor or the touch panel, and a stub instead of the
engine: the stub answers with the input itself, and an input with a `?` gives an error. Each
calculation is also printed over serial, with the LaTeX the editor wrote.

The same UI runs in a window on the PC: [firmware/sim](../sim/README.md).

```sh
. ~/export-esp.sh
cd firmware/handheld
cargo run --release          # flash and open the serial monitor
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

## Touch

Every menu works by touch, as in the simulator: a tap opens a line or picks a tile, ‹ at the
left of the top bar goes back, a list follows the finger up and down and turns its page when let
go, the mode tiles do the same sideways, and a number is set with − and + and OK. Until the
keypad exists, the calculation screen has MENU, SETUP and OPTN buttons along the bottom. The
panel is calibrated once ([display-and-touch.md](../docs/display-and-touch.md#calibration)).
