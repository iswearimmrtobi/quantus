# Plans

Open work, most important first. Remove an item once it is done; git keeps the history.

## 1. Make improper integrals of exponentials fast

**Problem.** `\int_0^{\infty} e^{-2x}\,dx`, `\int_{-\infty}^{\infty} e^{-x^2}\,dx` and
`\int_{-\infty}^{0} e^{x}\,dx` take 0.8–1.2 s each (measured 2026-10-24), against 0.02–0.07 s for
rational integrands like `\int_1^{\infty} \frac{1}{x^2}\,dx`. They are most of the 5 s that
`tests/golden.rs` takes, and the user waits just as long in the app.

**Cause, as far as measured.** `improper_integral` (`crates/quantus-core/src/eval/evaluator.rs`)
maps the infinite range onto (-1, 1) or [0, 1) and passes the mapped integrand to
`numeric::integrate` (tanh-sinh). A trace showed about 1,100 samples over 8 levels, and samples close
to t = 1 cost up to 130 ms each. There, `x = t/(1-t)` is built as an exact rational whose numerator
and denominator have about 100 digits, and the integrand is evaluated on that exact value. A single
`e^{-x}` on its own is fast (0.01 s even for x = 2·10⁶, thanks to the cut-off in `Value::eval_float`),
so the cost is in building and evaluating the huge rationals, not in `exp`.

**Plan.**
1. Add a timing guard first: a test that fails if any single `evaluate.json` case takes over 1 s.
   It fails today and shows the fix working.
2. In the `sample` closure of `improper_integral`, round `x` and the Jacobian to the working
   precision (`digits + 10`) before evaluating the integrand, instead of using exact rationals.
3. Stop sampling a tail once the integrand has stayed below the precision cut-off. `integrate`
   already stops after two negligible nodes per level. Check whether this happens late here because
   the Jacobian `1/(1-t)²` grows while `e^{-x}` shrinks.
4. Measure again. Target: each case under 0.2 s, with no change in digits (the golden expectations
   are 30 significant digits).

## 2. Manual checks left over from the Rust/Tauri migration

Verified in code and unit tests, but not yet by eye in the running app:

- Accelerators in the real window (Ctrl+0, Ctrl+Plus, Ctrl+-, F11, Ctrl+Shift+I in debug builds)
  and every menu shortcut.
- Starting a second instance focuses the first one.
- External links open in the browser; copy puts the `align*` block on the clipboard.
- `StartupWMClass=dev.tobifrosch.quantus` matches the window, so the taskbar groups it (Wayland
  and X11).
- The rendering of the AppImage, run from a fresh directory.

## 3. Finish the electronics mode

Implemented (`crates/quantus-core/src/electro/`, `tests/electro.rs`, the circuit editor in the
frontend) and selectable again in the mode selector. Spec: [electronics-module.md](../design/electronics-module.md).
Needed to ship it: document it in the README and the user docs.

## 4. Quantus on an ESP32 (spike)

`firmware/esp32`: the engine driven over serial, printing results, time, heap and stack. It
started on a classic ESP32 (WROOM, no PSRAM; the numbers below) and now builds for the ESP32-P4
dev kit. See its [README](../firmware/esp32/README.md).

- [x] Builds unchanged for `xtensa-esp32-espidf`; the release app is 1.9 MB of a 4 MB flash.
- [x] Flash it and record heap, stack and time per input. First run on a WROOM: 254 KB heap at
  start. Arithmetic, units, equations and the derivative work in 11–76 ms, with at most 14 KB of
  stack. Building the unit table (`units/lookup.rs`, ~1000 owned strings in two hash maps)
  takes 154 KB. The integrals take 7–17 s and keep another ~80 KB; after them the heap is gone
  and `e^{4i\pi/2}` aborts. `ln` and float formatting fail softly first (NaN, "could not read a
  computed number") because `astro-float` reports failed allocations instead of aborting.
- [x] Shrink the unit table: with the `small-memory` feature of `quantus-core` (on in the
  spike, off on the handheld, which has PSRAM), `units/lookup.rs` replays the registrations on every lookup instead of keeping
  the two maps (last match wins, same answers for all 2345 tried spellings). Units then cost no
  heap; all three integrals are right (7–20 s each) and `x^3 - 3x` sketches in 2.1 s. The
  desktop keeps the maps; with the feature a lookup costs ~34 µs in a debug build.
- [ ] Each integral still keeps ~44 KB, mostly the quadrature node cache (`NODES` in
  `numeric.rs`; also `RATIONAL_CACHE` in `analysis/simplify.rs`, `CACHE` in
  `algebra/rational_roots.rs`), and the second curve sketch runs out of memory. A way to drop
  the caches between calculations would likely fix that.
- [ ] Run `:bench` on the P4 and record time, heap and internal RAM against the WROOM numbers.
- [x] The engine on the handheld (`firmware/handheld`, through `firmware/engine`): on a thread
  of its own with a 64 KB stack, the spinner turning meanwhile, AC dropping the answer. First
  run on the P4 (2026-11-02): `2+3`, `2^{10}` and S⇔D in 6–15 ms, `2x+3=7` in 43 ms,
  `\int_0^1 \ln x\,dx` in 3.1 s (2.5 s the second time) with at most 16 KB of stack; 33 MB of
  heap free, of it 380–440 KB internal RAM.
- [ ] A handheld on an ESP32-P4: hardware requirements in
  [hardware.md](../firmware/docs/hardware.md).
- [ ] Its UI, on the ESP32-P4 dev kit (before that a classic ESP32) with the display, keys over
  serial and no engine. Rust and embedded-graphics, shared by a PC simulator and the board:
  - [x] `firmware/ui`: the input editor (a tree with a cursor, Casio-like templates) that writes
    LaTeX, and a reader of LaTeX results.
  - [x] 2D layout of math, the screen, band-wise redraws, a stub backend.
  - [x] `firmware/sim`: the UI in a window, PC keys.
  - [x] `firmware/handheld`: the ILI9486 over SPI on the ESP32-P4.
  - [x] Keys over the serial console on the board (built; not yet tried on the board).
  - [x] The screen like a Casio, in the desktop's dark palette: a top bar in the mode's colour,
    one calculation (input top left, result bottom right), the mode menu as coloured tiles,
    a setup page (SHIFT+MODE) for significant digits and exact or decimal, S⇔D.
  - [ ] ALPHA letters on the keys, once the keypad's legend is fixed.
  - [x] The menus of the handheld tree, modelled on the Casio fx-991DE X
    ([handheld-menus.md](../design/handheld-menus.md) §7, which marks each entry as working or
    UI only): MODE with twelve tiles, SETUP with a submenu per entry (a list of values or a typed
    number), OPTN per mode, German and English texts, the top bar's indicators, the purple
    palette. Entries that wait for the engine or the firmware say so.
  - [ ] CONST and CONV, and the keys still missing in §7.4 (ENG, STO, VARS, CALC, SOLVE, UNDO …).
  - [ ] Try both on the board: colours, orientation, SPI clock, time to the first frame.
  - [x] Brackets that grow with what they hold.
  - [x] For testing: the simulator with the real engine, and its picture mirrored to the board
    over serial (`--mirror`, the handheld's `mirror` binary; not yet tried on the board).
  - [x] A wallpaper from the SD card behind the mode tiles, the menus or everything, picked in
    SETUP → Gerät → Hintergrundbild (§7.6; the simulator reads `sd/wallpapers/`).
  - [x] The SD card on the board: `wallpapers()` and `wallpaper()` of the backend from
    `/wallpapers/`, and the picture set kept across restarts (its name in NVS). Sessions on the
    card come with §6.
  - [ ] A larger Greek font than u8g2's bitmap ones (π, μ and Ω are fine); German texts.

## 5. Features modelled on the Casio fx-991DE X

Decided entry by entry in [handheld-menus.md](../design/handheld-menus.md) §4; who does what in
§8 there. The maths, the data and the state go into `quantus-core`, so the desktop gets each item
as well as the handheld: a new setting goes into the desktop's settings dialog, `en.ts`, `de.ts`
and `docs/docs/reference/settings.md`, a new function into the syntax reference.

The engine has every item below but the last; the desktop has its part of all of them except
Matrizen, Stochastik, the link and the updater. What is open per shell is listed in
handheld-menus.md, *What is missing*.

- [ ] Angle unit: a setting Grad/Bogenmaß (degrees by default on the handheld, radians on the
  desktop), ° and rad as markers on a value, °′″ as input and as a result form.
- [ ] Number format: Norm 1/2, Fix 0–9 and Sci 0–9 for the display, with the significant digits
  as the internal precision; decimal prefixes on plain numbers; ENG steps of 3; digit grouping
  with a thin space.
- [ ] Result forms: mixed fractions (setting ab/c, d/c), recurring decimals (and a template to
  type a period), prime factors, complex r∠θ (setting, conversion of one result, ∠ input). Each
  result carries its alternative forms, so S⇔D on the handheld and a form switch on the desktop
  read them from the core.
- [ ] Basis-N: a mode for integers in DEC/HEX/BIN/OCT, the prefixes d, h, b, o and the bitwise
  operations and, or, xor, xnor, Not, Neg.
- [ ] Matrices: matrix values with + − ×, inverse, transpose, identity, det, powers, REF and
  RREF, exact and with steps, reusing the LGS Gauss code; any variable name instead of MatA–D.
- [ ] Stochastik: lists, one-variable statistics, the regressions, the distributions (normal,
  binomial, Poisson; density, cumulative, inverse normal). Two tiles on the handheld, a mode
  with a data editor on the desktop.
- [ ] Functions: arsinh, arcosh, artanh, coth, `\prod`, nPr, ggT, kgV, Int, Intg, Rnd, RndFix,
  ÷R.
- [ ] Constants and units: the 32 constants the Casio has and Quantus lacks (CODATA 2018, the
  Casio's six categories), usable outside Physik when inserted by name; a "▶ unit" operator
  that writes a result in a target unit; the missing imperial units (ft, yd, lb, oz, gal, psi,
  Btu, hp, °F …).
- [ ] Solving: exact cubics and quartics (Cardano, Ferrari) without the [−100, 100] limit; SOLVE
  for a chosen variable; CALC (the free variables of an expression, then substitute);
  wahr/falsch for an equation without unknowns, and a Prüfen mode.
- [ ] Sessions: history, mode inputs, a name and timestamps in `session.rs`, serialised to JSON
  with a format version. The desktop keeps them in its config directory with a session menu and
  opens the last one on start; the handheld keeps them on the SD card.
- [ ] Ans and PreAns in the session; STO and VARS on the handheld.
- [ ] The link between handheld and desktop: the request and response format (the JSON of
  `api.rs`) and the session files from the core; on the desktop a listener reachable over WLAN
  and Bluetooth, with pairing; on the handheld the client (remote engine, session sync).
- [ ] Firmware updates: an updater in the desktop app that takes the image from the GitHub
  releases and uploads it to the handheld (USB or the link); the handheld can fetch it over
  WLAN itself; the release workflow ([RELEASING.md](../RELEASING.md)) attaches the image.
- [ ] Later: polynomial inequalities; vectors with a Geometrie area
  ([new-areas-and-themes.md](new-areas-and-themes.md)).

The handheld's own part (menus, keys, the purple palette, WLAN and Bluetooth, the exam mode) is
in [handheld-menus.md](../design/handheld-menus.md) §7.

## 6. Later

- The remaining gaps in [unsupported-inputs.md](../design/unsupported-inputs.md) (constants and the
  history are now in §5).
- Partial derivatives and `dx` notation: the largest group of failing cases in the generated test
  set (`test/results/failures.jsonl`).
- Optional: a thin `quantus-server` crate (axum) over `quantus-core`, if self-hosting is wanted.
- Optional: PKGBUILD or Flatpak, signing and updates.
