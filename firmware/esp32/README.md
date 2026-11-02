# Quantus on an ESP32

A feasibility spike: does `quantus-core` build, fit and compute on a microcontroller? The firmware
is a prompt over serial: each line typed into the monitor is evaluated like an input in the app's
current mode, and the result is printed with its time, the heap and the engine thread's stack use. There is no
display and no keypad yet. `quantus-core` is built with its `small-memory` feature.

Target: the Waveshare ESP32-P4-Module-DEV-KIT (16 MB flash, 32 MB PSRAM,
[dev-kit.md](../docs/dev-kit.md)), the same board as the [handheld](../handheld/README.md).
Toolchain, flashing and the serial port: [setup.md](../docs/setup.md).

Until the P4 arrived this ran on a classic ESP32 (WROOM, 4 MB flash, 520 KB SRAM, no PSRAM,
target `xtensa-esp32-espidf`); its numbers are in [plans.md](../../plans/plans.md) §4 and git
history has that build setup. To compare with them, `:mem` and each result also print the
internal RAM alone.

```sh
. ~/export-esp.sh
cd firmware/esp32
cargo run --release          # flash and open the serial monitor
```

## Using the prompt

```text
physics> r_1 = 5\,\mathrm{cm}
  20.6ms  r_1 = 5\,\mathrm{cm}  =>  5\,\mathrm{cm}
          stack 8500 of 32768 B; heap 248424 B free, 247284 B lowest, 118784 B largest block
physics> :mode imaginary
mode: imaginary
imaginary> e^{4i\pi/2}
  24.3ms  e^{4i\pi/2}  =>  1
```

(Output from the classic ESP32; the P4 adds a line with the internal RAM.)

Inputs are LaTeX, as the app's editor writes them. Variables need a subscript and stay defined
until the board is reset. Commands:

- `:mode` shows the mode, `:mode <name>` switches it: `algebra`, `physics` (the default),
  `imaginary`, `analysis` (each line is a curve sketch, `f(x) = …`) or `linear` (each line is a
  system, `2x + y = 5; x - y = 1`). The electronics mode is a circuit editor and is left out.
- `:sketch f(x) = …` runs the curve sketching in any mode.
- `:bench` runs a fixed list of inputs, one or two per category of `test/cases`, in `physics`.
- `:mem` prints heap and stack.
- `:help` lists the commands.

The largest free block matters as much as the free heap: big numbers need contiguous memory.
When an allocation fails the board prints `memory allocation of … failed` and halts instead of
restarting; press RESET or run `espflash reset`.
