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

## 3. Later

- Physical constants (c, h, G) and the remaining gaps in [unsupported-inputs.md](unsupported-inputs.md).
- History and persistence of calculations.
- Partial derivatives and `dx` notation: the largest group of failing cases in the generated test
  set (`test/results/failures.jsonl`).
- Optional: a thin `quantus-server` crate (axum) over `quantus-core`, if self-hosting is wanted.
- Optional: PKGBUILD or Flatpak, signing and updates.
