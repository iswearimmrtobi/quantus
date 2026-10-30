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

`firmware/esp32`: the engine on a classic ESP32 (WROOM, no PSRAM), running fixed inputs and
printing results, time, heap and stack over serial. See its [README](../firmware/esp32/README.md).

- [x] Builds unchanged for `xtensa-esp32-espidf`; the release app is 1.9 MB of a 4 MB flash.
- [x] Flash it and record heap, stack and time per input. First run on a WROOM: 254 KB heap at
  start. Arithmetic, units, equations and the derivative work in 11–76 ms, with at most 14 KB of
  stack. Building the unit table (`units/lookup.rs`, ~1000 owned strings in two hash maps)
  takes 154 KB. The integrals take 7–17 s and keep another ~80 KB; after them the heap is gone
  and `e^{4i\pi/2}` aborts. `ln` and float formatting fail softly first (NaN, "could not read a
  computed number") because `astro-float` reports failed allocations instead of aborting.
- [x] Shrink the unit table: `units/lookup.rs` now replays the registrations on every lookup
  (last match wins, same answers for all 2345 tried spellings). Units then cost no heap; all
  three integrals are right (7–20 s each) and `x^3 - 3x` sketches in 2.1 s. The price on the
  desktop: ~34 µs per lookup in a debug build, so `units_oracle` (190 000 lookups) takes 6.5 s
  instead of 0.16 s.
- [ ] Each integral still keeps ~44 KB, mostly the quadrature node cache (`NODES` in
  `numeric.rs`; also `RATIONAL_CACHE` in `analysis/simplify.rs`, `CACHE` in
  `algebra/rational_roots.rs`), and the second curve sketch runs out of memory. A way to drop
  the caches between calculations would likely fix that.
- [ ] If so: a keypad that writes LaTeX and a display that renders the result.

## 5. Later

- Physical constants (c, h, G) and the remaining gaps in [unsupported-inputs.md](../design/unsupported-inputs.md).
- History and persistence of calculations.
- Partial derivatives and `dx` notation: the largest group of failing cases in the generated test
  set (`test/results/failures.jsonl`).
- Optional: a thin `quantus-server` crate (axum) over `quantus-core`, if self-hosting is wanted.
- Optional: PKGBUILD or Flatpak, signing and updates.
