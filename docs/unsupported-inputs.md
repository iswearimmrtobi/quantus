# Test inputs: what the engine supports

On 2026-10-22 a list of 48 test inputs was run through `LatexCalculator::evaluate` (precision 50,
no session variables). 28 worked; 20 failed on a command the engine did not know. The commands
behind those failures have since been implemented. Each is covered by golden cases in
`crates/quantus-core/tests/golden`.

The 48 inputs themselves are not in the repository, so the original IDs below have **not** been
re-run as a set. What has been checked is the feature each one was waiting for.

## Was unsupported, now supported

| Feature | Original IDs | Notes |
|---|---|---|
| `\sin`, `\cos`, `\tan` | P2, P3, P5, U3, U5, M11 | Radians, or degrees with `30^\circ`. Exact for rational multiples of π where the value is rational (`\sin 30^\circ` is exactly 1/2). Argument must be a plain number. |
| `\ln`, `\log` | M3 | `\log` is base 10. Arguments must be positive. No `\log_b`; write `\ln x / \ln b`. |
| `\|x\|` | M8 | Also `\left\|…\right\|`, `\lvert…\rvert`. Keeps the unit. |
| `\binom{n}{k}` | M7 | Exact; needs non-negative whole numbers. |
| `\sum_{i=a}^{b}` | M6 | Exact, up to 1,000,000 terms. The body is a whole term: `\sum k + 1` is `(\sum k) + 1`. |
| `\int_a^b f\,dx` | M9, P6, P7, P8, U7, U8 | Numerical (tanh-sinh), at most 30 digits. The variable carries the unit of the limits. Nested integrals work but are slow. |
| `\lim_{x \to a}` | M11 | Numerical and two-sided. Reports an error if the sides disagree or the values grow without bound. |
| `\det`, `pmatrix` | M12 | Exact, up to 8×8. `vmatrix` is a determinant. A matrix on its own is an error. |
| Variables without an index | M4, M5 | A single letter that is not a unit or constant (`x`, `\theta`). |
| Derivative `\frac{d}{dx}` | M10 | Symbolic; needs a point: `\left.\frac{d}{dx} f\right\|_{x=2}`. |
| Equations | M4, M5 | `lhs = rhs` with one unknown. Degree ≤ 2 exactly, otherwise numerically on [-100, 100]. |

## Limits worth knowing

- **Free variables cannot be defined in a session.** `x = 3` is a line named `x`, not a
  definition; only subscripted names (`x_1 = 3`) are stored. Letters get their value from
  `\sum`, `\int`, `\lim`, `\frac{d}{dx}` or by being the unknown of an equation.
- **Units win over letters.** `m`, `s`, `g`, `y`, `t` and so on are units, so they cannot be the
  unknown of an equation. Inside `\sum`, `\int`, `\lim` and `\frac{d}{dx}` the bound letter
  takes precedence in the body.
- **The unknown of an equation is a plain number.** `3x = 6m` is a dimension error; `3x\,m = 6m`
  is fine.
- **Integral differentials** (`dx`, `d x`, `\mathrm{d}x`) must be at the top level of the
  integrand, not inside `\frac{…}{…}`.
- **Numerical results are marked by their digits.** An integral shows at most 30 digits, a root
  found numerically as many as the precision.
- **Not implemented:** one-sided limits, limits at infinity, indefinite integrals, higher
  derivatives other than by nesting, `\arcsin` and friends, `\sin^{-1}`, matrix results
  (only `\det`), equations with several unknowns.

## Inputs that already worked

M1, M2, P1, P4, P9–P15, U1, U2, U4, U6, U9–U19 (arithmetic, powers, roots, fractions, units,
constants, scientific notation). U18 (`5 m + 3 s`) correctly errors with
`Incompatible units: m (length) and s (time) cannot be added.`

`M` is the general maths list, `P` the physics list without units, `U` the physics list with
units.
