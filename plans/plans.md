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
Documented in the README and the user docs (`docs/docs/guides/electronics.md`).

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
- [x] Kurvendiskussion on the P4 (2026-11-03): the sketch of `x^3-3x` in 0.3–0.7 s, `\sin(x)`
  0.4 s, `xe^{-x}` 0.8 s, `\frac{x^2-1}{x-2}` 1.9 s (all sections ok, under the 3 s budget);
  the tangent at 2 in 0.23 s, the area between the outer zeros in 0.2 s. At most 25 KB of
  stack, 33 MB heap free throughout, internal RAM down to 116 KB free at its lowest. Eight
  sketches and tasks in a row did not run short (the WROOM's second sketch did). The graph page
  first took 7 s to draw (a 2 px polyline of 600 points across all ten bands); drawn in short
  1 px pieces a full screen takes at most 218 ms.
- [x] Moving the graph on the P4 (2026-11-03): the curve for a new window (480 samples) in
  33–41 ms for `\sin(x)`, 59–63 ms for `x^3-3x`; a full redraw of the moved graph 134–204 ms.
  Once, right after a boot (WLAN connecting, clock being set), keys sent over serial came in
  late and bunched, so OK closed the sketch instead of starting the move; not seen again.
- [ ] A handheld on an ESP32-P4: hardware requirements in
  [hardware.md](../firmware/docs/hardware.md).
- [ ] Its UI, on the ESP32-P4 dev kit (before that a classic ESP32) with the display, keys over
  serial and no engine. Rust and embedded-graphics, shared by a PC simulator and the board:
  - [x] `firmware/ui`: the input editor (a tree with a cursor, Casio-like templates) that writes
    LaTeX, and a reader of LaTeX results.
  - [x] 2D layout of math, the screen, band-wise redraws, a stub backend.
  - [x] `firmware/sim`: the UI in a window, PC keys.
  - [x] `firmware/handheld`: the ST7796S over SPI and its FT6336U touch over I²C on the ESP32-P4 (before: an ILI9486 RPi display).
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
  - [ ] The keys still missing in §7.4 (ENG, PreAns, UNDO …).
  - [ ] Try both on the board: colours, orientation, SPI clock, time to the first frame.
  - [ ] On a bare ST7796S panel, measure `bench` at 40, 60 and 80 MHz and wire `TE` against
    tearing ([hardware.md](../firmware/docs/hardware.md) §3).
  - [x] Brackets that grow with what they hold.
  - [x] For testing: the simulator with the real engine, and its picture mirrored to the board
    over serial (`--mirror`, the handheld's `mirror` binary; not yet tried on the board).
  - [x] A wallpaper from the SD card behind the mode tiles, the menus or everything, picked in
    SETUP → Gerät → Hintergrundbild (§7.6; the simulator reads `sd/wallpapers/`).
  - [x] The SD card on the board: `wallpapers()` and `wallpaper()` of the backend from
    `/wallpapers/`, and the picture set kept across restarts (its name in NVS).
  - [x] Sessions on the card in `/sessions/` (§6), the Sitzungen menu, the last one reopened.
  - [x] WLAN through the C6 (ESP-Hosted; [wlan.md](../firmware/docs/wlan.md)): networks,
    passwords, saved networks, status, off in the exam mode. On the board: joining, NTP and
    the reconnect after a restart work. The C6's firmware is updated by the handheld itself
    (`firmware/c6`, now ESP-Hosted 3.0.10 on both sides).
  - [x] The clock: NTP or by hand, named time zones with summer time, session names and
    timestamps from it; the settings kept across a restart.
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
- [ ] Matrices: matrix values with + − ×, inverse, transpose, identity, det, powers, REF and
  RREF, exact and with steps, reusing the LGS Gauss code; any variable name instead of MatA–D.
  The handheld has the mode with its editor; the desktop's input and output are missing.
- [ ] Stochastik: lists, one-variable statistics, the regressions, the distributions (normal,
  binomial, Poisson; density, cumulative, inverse normal). Two tiles on the handheld, a mode
  with a data editor on the desktop.
- [x] Sessions: history, mode inputs, a name and timestamps in `session.rs`, serialised to JSON
  with a format version. The desktop keeps them in its config directory with a session menu and
  opens the last one on start; the handheld keeps them on the SD card.
- [ ] PreAns on a key of the handheld (the session has it; STO and VARS are done).
- [ ] The link between handheld and desktop: the request and response format (the JSON of
  `api.rs`) and the session files from the core; on the desktop a listener reachable over WLAN
  and Bluetooth, with pairing; on the handheld the client (remote engine, session sync).
- [ ] Firmware updates: an updater in the desktop app that takes the image from the GitHub
  releases and uploads it to the handheld (USB or the link); the handheld can fetch it over
  WLAN itself; the release workflow ([RELEASING.md](../RELEASING.md)) attaches the image.
- (Vectors went into the Geometry mode, built in engine and desktop; its handheld part is in
  [geometry.md](geometry.md) § Later.)

The handheld's own part (menus, keys, the purple palette, WLAN and Bluetooth, the exam mode) is
in [handheld-menus.md](../design/handheld-menus.md) §7.

## 6. Search everywhere

From [feature-ideas.md](../firmware/docs/feature-ideas.md) (was §1): a few typed letters find
functions, units, prefixes, constants, settings and menu pages together. The handheld types into
CATALOG (SHIFT+OPTN); the desktop's quick search and library dialog get the larger index. Ranking
by how often an entry was used comes later.

- [x] `crates/quantus-search`: the matching and ranking of `library_search`, shared by the engine
  and the handheld, which does not depend on the engine.
- [x] Core: functions and prefixes with names in every language and aliases (`wurzel`, `sqrt`) in
  the library search, and the whole index for the handheld (`api::library_index`).
- [x] Handheld: typing in CATALOG lists the hits of the index and of the SETUP and OPTN menus;
  a function, unit or constant is inserted, a menu entry opens where it lives.
- [x] Desktop: the quick search lists functions and prefixes too.
- [ ] Usage counts that rank frequent entries first.

## 7. Apps and games on the handheld

Games (tic tac toe, 2048) and user-made UIs for special calculations, as scripts that the handheld
runs: built in, or from `/apps/` on the card. Handheld only; the core does not change. The design
and the steps, with their checkboxes, are in [apps.md](../firmware/docs/apps.md) §8; the first
step is a spike of the interpreter on the board.

## 8. Later

- The remaining gaps in [unsupported-inputs.md](../design/unsupported-inputs.md) (constants and the
  history are now in §5).
- Optional: a thin `quantus-server` crate (axum) over `quantus-core`, if self-hosting is wanted.
- Optional: PKGBUILD or Flatpak, signing and updates.
