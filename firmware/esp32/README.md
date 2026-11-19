# Quantus on an ESP32

A feasibility spike: does `quantus-core` build, fit and compute on a microcontroller? The firmware
is a prompt over serial: each line typed into the monitor is evaluated like an input in the app's
current mode, and the result is printed with its time, the heap and the engine thread's stack use. There is no
display and no keypad yet. `quantus-core` is built with its `small-memory` feature.

Target: the Waveshare ESP32-P4-Module-DEV-KIT (16 MB flash, 32 MB PSRAM,
[dev-kit.md](../docs/dev-kit.md)), the same board as the [handheld](../handheld/README.md).
Toolchain, flashing and the serial port: [setup.md](../docs/setup.md).

Until the P4 arrived this ran on a classic ESP32 (WROOM, 4 MB flash, 520 KB SRAM, no PSRAM,
target `xtensa-esp32-espidf`); its numbers are under [Measurements](#measurements) and git
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

## Measurements

**WROOM** (release app 1.9 MB of a 4 MB flash, 254 KB heap at start). Arithmetic, units,
equations and the derivative in 11–76 ms with at most 14 KB of stack. The unit table
(`units/lookup.rs`, ~1000 owned strings in two hash maps) took 154 KB; the integrals 7–17 s and
another ~80 KB, after which `e^{4i\pi/2}` aborted (`ln` and float formatting failed softly first,
since `astro-float` reports failed allocations). With the `small-memory` feature lookups replay
the registrations instead of keeping the maps (~34 µs each in a debug build): units cost no heap,
all three integrals are right (7–20 s each) and `x^3 - 3x` sketches in 2.1 s; the second sketch
still runs out of memory.

**P4, on the handheld** (`firmware/handheld`, the engine on its own thread with a 64 KB stack;
2026-11-02/03). `2+3`, `2^{10}` and S⇔D in 6–15 ms, `2x+3=7` in 43 ms, `\int_0^1 \ln x\,dx` in
3.1 s (2.5 s the second time); 33 MB heap free, 380–440 KB of it internal. Curve sketches:
`x^3-3x` 0.3–0.7 s, `\sin(x)` 0.4 s, `xe^{-x}` 0.8 s, `\frac{x^2-1}{x-2}` 1.9 s; the tangent
0.23 s, an area 0.2 s; at most 25 KB of stack, internal RAM at least 116 KB free, eight sketches
in a row fine. Moving the graph: a new window (480 samples) in 33–63 ms, the redraw 134–204 ms.
