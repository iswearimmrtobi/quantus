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

## Modes and complex numbers

The mode is a setting (`mode`: `algebra`, `physics`, `imaginary`; physics is the default) and is
switched in the top bar.

| | algebra | physics | imaginary |
|---|---|---|---|
| Units (outside `\mathrm{}`) | no, every letter is a variable | yes | no, every letter but `i` is a variable |
| Physical constants (`c_0`, `h_P`, …) | no | yes | no |
| `i` | a variable | a variable | the imaginary unit |
| Complex results | error | error | shown as `a + bi` |

In imaginary mode:

- `i^n`, `\sqrt{-a}` (principal root `i\sqrt{a}`), sums, products, quotients and integer powers
  of `a + bi` are exact. `(3+4i)(1-2i)` is `11 - 2i`.
- `e^{a+bi}`, `z^w` (as `exp(w ln z)`, principal branch), `\ln` and `\log` of a negative or
  complex number, `\sin`, `\cos`, `\tan`, `\sinh`, … of a complex argument, `\exp`.
- `|z|` is the modulus; `\Re`, `\Im`, `\arg` and `\overline` give the real part, the imaginary
  part, the argument in (-π, π] and the conjugate.
- Quadratic equations with a negative discriminant have the roots `(-b ± i√-D) / 2a`.
- A variable can hold a complex number (`z_1 = 3+4i`).
- Both parts of a result are given to the calculator's precision; the API reports the real part
  as `value` and the imaginary part as `imaginary`.

Not covered: `\arcsin`, `\arccos` and `\arctan` of a complex number, equations of degree above
two with complex roots, derivatives and integrals of complex functions, and complex units
(`2i\,\mathrm{m}` works, `\mathrm{m}` is not a complex quantity). Odd roots of negative
numbers keep their real value: `\sqrt[3]{-8}` is -2. The algebraic simplifier leaves `i` alone,
so `(x+i)(x-i)` with a free `x` is shown as typed.

## Analysis mode

The analysis mode (`docs/analysis-mode.md`) takes a function of one real variable built from
numbers, `\pi`, `e`, `+ − · /`, powers, roots, `e^u`, `a^u`, `\exp`, `\ln`, `\log`, `\log_b`,
`\sin`, `\cos`, `\tan`, `\cot`, `\sec`, `\csc`, `\arcsin`, `\arccos`, `\arctan`, `\sinh`,
`\cosh`, `\tanh`, `|u|` and degrees, nested in any way.

Rejected with an error that names the reason:

- **Function families** (a second free letter, `x^2 + a`).
- Piecewise functions, implicit or parametric curves, complex results.
- Units, sums, integrals, limits, derivatives, matrices, `n!` and the other integer functions.
- **Domains made of periodic intervals**, such as `\ln(\sin x)` (periodic gaps like those of
  `\tan x` are fine).
- In the area task: **improper integrals** (a pole, gap or unbounded bound in the interval) and
  areas between two curves.

Limits worth knowing:

- Equations no school method solves are **scanned numerically** on [−100, 100] (or in the
  window of a periodic family); the section is then marked *partial*. Such roots are accurate
  to about 15 digits, polynomial roots to the full precision. A numeric root that is exactly a
  simple rational or rational multiple of π is shown exactly.
- Antiderivatives are found by a table (with completed squares for `\frac{1}{x^2+2x+5}` and
  `\frac{1}{\sqrt{3+2x-x^2}}`), partial fractions (also over quadratics with irrational roots,
  `\frac{1}{x^2-2}`), substitution and integration by parts (up to six nested rounds, so
  `x^5 e^{2x}` works). **Repeated irreducible quadratic factors** in a denominator
  (`\frac{1}{(x^2+1)^2}`) and integrands without an elementary antiderivative (`e^{x^2}`,
  `\frac{\sin x}{x}`) give "no elementary antiderivative found". Every result shown has been
  checked by differentiation.
- Limits that neither degrees, pole orders, growth orders (including `\ln` of a growing
  argument) nor the bounded-factor rule decide are sampled numerically and shown with
  `\approx`, or as unknown. An oblique asymptote found that way is shown with `\approx`.
- The time budget is checked inside the numeric scans, the simplifier and the integration
  methods, so a section that runs out of time is marked partial, not only the ones after it.
  The tasks have 2 seconds each, a resampled view of the graph 200 ms.
- Simplification treats functions such as `\ln x` and `\sqrt{3}` as independent symbols when it
  multiplies out and cancels, so a result can be correct without being the shortest form.
- A sketch is cut off after **3 seconds**; the sections still missing are marked partial.
- Periodic families are listed in the window [−P, P] (at most 12 members each), or [−10, 10]
  when f itself is not periodic. For f = ax + (a periodic part), such as `x + \sin x`, the
  general family is shown with a y that moves along; for other non-periodic f only x is general.

## Inputs that already worked

M1, M2, P1, P4, P9–P15, U1, U2, U4, U6, U9–U19 (arithmetic, powers, roots, fractions, units,
constants, scientific notation). U18 (`5 m + 3 s`) correctly errors with
`Incompatible units: m (length) and s (time) cannot be added.`

`M` is the general maths list, `P` the physics list without units, `U` the physics list with
units.
