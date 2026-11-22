# Plans

Open work, most important first. Remove an item once it is done; git keeps the history.

## 1. Make nested integrals fast

**Problem.** `\int_0^1\int_0^2 x y \, dx\, dy` takes 1.9 s in a release build (2.9 s in a debug
one), because every sample of the outer integral runs the inner integral again, about 1,100 times,
at 1.7 ms each. `tests/golden.rs` lists it in `KNOWN_SLOW`; remove it from there when this is done.

**Ideas.** A polynomial integrand needs no tanh-sinh: integrate it exactly by its antiderivative
(the curve-sketching code already has one for `\int x e^x`). Or cache the inner integral's nodes and
evaluation per outer sample, or lower the inner precision to what the outer one needs.
Target: under 0.2 s.

**Improper integrals of exponentials.** 0.2–0.3 s per case in a release build now.
- [ ] Stop sampling a tail earlier than `integrate`'s two negligible nodes per level. About 1,100
  samples at 0.2 ms (an `exp` at 78 digits) remain.

## 4. Quantus on an ESP32 (spike)

`firmware/esp32`: the engine driven over serial, printing results, time, heap and stack. It
started on a classic ESP32 (WROOM, no PSRAM; its numbers in the README) and now builds for the ESP32-P4
dev kit. See its [README](../firmware/esp32/README.md).

- [ ] Each integral still keeps ~44 KB, mostly the quadrature node cache (`NODES` in
  `numeric.rs`; also `RATIONAL_CACHE` in `analysis/simplify.rs`, `CACHE` in
  `algebra/rational_roots.rs`), and the second curve sketch runs out of memory. A way to drop
  the caches between calculations would likely fix that.
- [ ] Run `:bench` on the P4 and record time, heap and internal RAM against the WROOM numbers.
- [ ] A handheld on an ESP32-P4: hardware requirements in
  [hardware.md](../firmware/docs/hardware.md).
- [ ] Its UI (`firmware/ui`, shared by the simulator and the board), what is left:
  - [ ] ALPHA letters on the keys, once the keypad's legend is fixed.
  - [ ] The keys still missing in §7.4 (ENG, UNDO …).
  - [ ] Try both on the board: colours, orientation, SPI clock, time to the first frame.
  - [ ] On a bare ST7796S panel, measure `bench` at 40, 60 and 80 MHz and wire `TE` against
    tearing ([hardware.md](../firmware/docs/hardware.md) §3).
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
- [ ] The link between handheld and desktop: the request and response format (the JSON of
  `api.rs`) and the session files from the core; on the desktop a listener reachable over WLAN
  and Bluetooth, with pairing; on the handheld the client (remote engine, session sync).
- [ ] Firmware updates: an updater in the desktop app that takes the image from the GitHub
  releases and uploads it to the handheld (USB or the link); the handheld can fetch it over
  WLAN itself; the release workflow ([RELEASING.md](../RELEASING.md)) attaches the image.
- (Vectors went into the Geometry mode, built in engine and desktop; its handheld part is in
  §9.)

The handheld's own part (menus, keys, the purple palette, WLAN and Bluetooth, the exam mode) is
in [handheld-menus.md](../design/handheld-menus.md) §7.

## 6. Search everywhere

From [feature-ideas.md](feature-ideas.md) (was its §1): a few typed letters find
functions, units, prefixes, constants, settings and menu pages together. The handheld types into
CATALOG (SHIFT+OPTN); the desktop's quick search and library dialog get the larger index. Ranking
by how often an entry was used comes later.

- [ ] Usage counts that rank frequent entries first.

## 9. Later

- The Geometrie tile on the handheld: the engine and its 3D view are shared, the tile, the
  input and the drawing (short 1 px lines, keys rotate) are missing; the approach is in
  [geometry.md](geometry.md) § 3D view and the handheld.
- The remaining gaps in [unsupported-inputs.md](../design/unsupported-inputs.md) (constants and the
  history are now in §5).
- The unit bar on an app's input line on the handheld (apps-roadmap.md F9); the apps themselves
  are done ([apps.md](../firmware/docs/apps.md)).
- Optional: a thin `quantus-server` crate (axum) over `quantus-core`, if self-hosting is wanted.
- Optional: PKGBUILD or Flatpak, signing and updates.
