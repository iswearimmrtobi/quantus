# Engine test set

112,540 generated cases that put the engine through arithmetic, elementary functions, equations,
derivatives, integrals, limits, algebraic simplification, physics with units, parser robustness,
complex numbers, the linear systems mode and the modes added after it (from `verify` on in the
table below).
The 2500 hand-checked algebra cases from before are in `algebra-simplification-cases.md`.
The [test inventory](../docs/docs/contributing/test-set.md) counts every test and case, by category and subcategory.

| Path | What |
|---|---|
| `generator/generate.py` | Writes `cases/`. Standard library only, deterministic (seed 20261023). |
| `generator/gens/*.py` | One module per area; every expected value is computed there, independently of the engine. |
| `cases/<category>/<category>-NNN.json` | The cases, 1000 per file. |
| `results/summary.md` | The last full run. `results/results.jsonl` (one line per case) is not committed. |
| `results/failures.jsonl` | Every case of the last full run that did not pass, one JSON line each. Committed. |
| `../crates/quantus-core/examples/run_cases.rs` | The runner; the checks of the newer modes are in `run_cases/`, one file per mode. |
| `circuits/` | Circuit files for the electronics mode, with their expected values ([README](circuits/README.md)). |

## Running

```sh
python3 test/generator/generate.py --check          # regenerate test/cases (about 80 s)
cargo run --release -p quantus-core --example run_cases                       # all cases
cargo run --release -p quantus-core --example run_cases -- --filter physics/decay --fail-only
```

Runner options: `--cases DIR`, `--out DIR`, `--filter TEXT` (id, category or `category/subcategory`
prefix; repeatable), `--jobs N` (default: cores, at most 8), `--timeout SECS` (default 5), `--limit N`, `--fail-only`.

Each worker is a child process limited to 4 GiB of address space; a case that hangs or runs out of
memory is killed and reported as `TIMEOUT` or `ERROR` without affecting the others. To be safe on a
shared desktop, wrap a full run in `systemd-run --user --scope -p MemoryMax=12G`.

`--check` asserts unique ids, the number of cases per category and parseable expectations, and compares the generator's unit
factors and dimensions with the oracle dump in `crates/quantus-core/tests/golden/units_symbols.json`.

## Case format

Each file is `{"category", "generator_version", "seed", "count", "cases": [...]}`, one case per line:

```json
{"id": "phys-000412", "category": "physics", "subcategory": "ideal-gas", "difficulty": "medium",
 "gap": false, "latex": "p = \\frac{2\\,\\mathrm{mol} \\cdot 8{,}314\\,\\frac{\\mathrm{J}}{\\mathrm{mol} \\cdot \\mathrm{K}} \\cdot 300\\,\\mathrm{K}}{5\\,\\mathrm{l}}",
 "expect": {"kind": "quantity", "value": "997680", "rel_tol": "1e-45", "dim": {"L": -1, "M": 1, "T": -2}},
 "description": "p = n R T / V"}
```

- `difficulty`: `easy`, `medium`, `hard` or `very-hard`, in a 40/30/20/10 split.
- `gap: true` marks inputs that were not implemented when the case was written, such as one-sided
  limits, `\infty`, `\arcsin`, `\log_b` or units like `eV`. Most of them have been implemented
  since and pass. Gap cases make up about 1.4% of the set (1,592). They count like any other case,
  but the summary reports them separately. The `open-features` category was written as gaps for
  what `plans/plans.md` still listed and is implemented now; to track a new open feature, add
  cases with `gap=True` there.
- `settings` (optional) overrides `Settings` fields (`a_and_y_as_variables`, `letter_products`, `extra_unit_letters`, `mode`: `algebra`, `physics` (the default), `imaginary` or `linear`, and for `linear` also `linear_complex`, `elimination` and `unknown_names`). Any other key is a field of `Settings` under its JSON name, such as `mode: "verify"`, `language`, `angleUnit` or `numberBase`.
- `api` (optional) names the API function of a mode with its own request, such as `statistics`; `request` is then what that mode's shell sends to it.
- `request` (linear systems only) is what the mode sends to `api::linear_system`: `{"equations": latex}` or `{"matrix": {"rows": [[latex, …], …], "unknowns"?: [latex, …]}}`. `latex` then shows the system for the report.
- Values are exact rationals `p/q` or decimal strings. `rel_tol` is relative. If the expected value
  is 0 the comparison is absolute, and with the exact tolerance `1e-45` it must be exactly 0.

| `expect.kind` | Passes when |
|---|---|
| `number` | The result is dimensionless and `value` matches within `rel_tol`. |
| `quantity` | The result's dimension equals `dim` (base exponents `L M T I Θ N J`) and its value, converted to the coherent SI unit (°C becomes K), matches `value`. `unit`, if given, is only compared for a note. |
| `solutions` | The solver's real solutions equal `values` as a set, with SI values and `dim`. |
| `inequality` | The result is read as the solver writes an equation's roots: pieces joined by `\lor` (after a `\Leftrightarrow`, if the input is restated), each a chain such as `-2 < x \le 3`. Each piece is evaluated in the Verify mode with `var` replaced by sample points: every bound in `intervals` (`[lo, hi, lo_closed, hi_closed]`, `null` for an infinite end), a thousandth either side of it, the midpoints and one beyond each end. A point must be in the result's set exactly when it is in `intervals`. |
| `algebra` | The result equals `simplified` at 8 sample points (exact rationals), is fully reduced (`Poly::gcd` of numerator and denominator is constant), and its restrictions equal `restrictions` (values per variable) plus `poly_restrictions`. |
| `complex` | The result's real part (`value`) and imaginary part match `re` and `im` within `rel_tol`; a real result has the imaginary part 0. Used with `settings.mode` set to `imaginary`. |
| `complex-solutions` | The solver's roots, each split into real and imaginary part, equal `values` (`{re, im}` pairs) as a set. |
| `linear` | The unknowns equal `unknowns` (LaTeX, in column order), every matrix entry read equals `rows` (`[A \| b]`, a value or `{re, im}` each), and the outcome matches: for `unique` each unknown equals `values`, for `none` the result is "no solution", for `infinite` there are `free` parameters and the solution satisfies every row at two choices of them. |
| `truth` | Verify mode: the value is 1 for `true` and 0 for `false`, and the result is `\text{true}` or `\text{false}` (`\text{wahr}`, `\text{falsch}` with `language: "de"`). |
| `basen` | Base-N mode: the value equals `value` (as the word reads it, signed or unsigned), the result is `result` as written in the set base, the three other bases are the result's forms (`forms`, by `dec`, `hex`, `bin`, `oct`), and the result warns that it wrapped exactly when `wrapped` is true. With `ieee` (32 and 64 bits), the response's float view has that `class`, `exact` value and rounding `error`. The word comes from the settings `wordSize` and `signed` (default 32, signed). |
| `matrix` | Matrix mode: the result is a `pmatrix` of the size of `rows`, and each entry read back equals the one in `rows` exactly. A determinant is a `number`. |
| `statistics` | Statistics: `request` goes to `api::statistics`. The keys of the one- or two-variable values are exactly those of `values`; each value, and each of `regression` (a, b, c, r) and `estimates` (`xHat`, `yHat`), matches its `[value, rel_tol]`. |
| `distribution` | Distributions: `request` goes to `api::distribution`; the results, one or one per k, match `values` (`[value, rel_tol]` each) in order. |
| `analysis` | Analysis mode, `api::analyze`: for each point kind (`zero`, `yIntercept`, `max`, `min`, `saddle`, `inflection`, `hole`) the points equal `points[kind]` (none if absent), and the asymptotes equal `asymptotes` (kind, x or slope and intercept, and `side` if given), all to `tol` (the engine's points are floats). |
| `tangent` | `api::tangent`: the point, the tangent's slope and intercept, and the normal's, or its `x` when it is vertical, to `tol`. |
| `area` | `api::area`: `area` and `signed`, read back from the LaTeX (after a `\approx`), match exactly (1e-40). |
| `table` | `api::value_table`: as many rows as `rows`, each x exact and y exact or empty (`null`, outside the domain). |
| `geometry` | Geometry mode, `api::geometry`: the task's `relation` equals the expected one (if given), and for each of `values` (`[name, value, rel_tol]`) and `points` (`[name, [x, y, z]]`, exact) there is a result line `name = … = value` that matches. An angle is compared by its decimal after `\approx`. A task answered with `error` counts as refused. |
| `circuit` | Electronics (`api::solve_circuit`) and Kirchhoff (`api::kirchhoff`): for each part in `elements`, its voltage, current and power (`[U, I, P]` in SI units), read back from the LaTeX with its unit, match to 1e-40. In the Electronics mode a source counts from a to b, so a delivering one has a negative current; in the Kirchhoff mode a source counts as what it delivers. A resistor counts along its current in both. |
| `equivalent` | Kirchhoff mode with `terminals`: the resistance between A and B, read back from its LaTeX, matches `resistance` to 1e-40. |
| `error` | The engine refuses the input with a message containing `contains`. |

The tolerances are `1e-45` for exact results (the engine prints 50 digits) and `1e-40` for exact
input with a transcendental result. Numerical methods get looser ones: `1e-25` for integrals,
`1e-30` for roots found numerically and `1e-12` for limits.

Algebra restrictions follow the convention of `algebra-simplification-cases.md`. Every real value
that makes a denominator of the *input* zero is listed. A factor without a real zero (`x^2 + 4`) is
listed only when the input writes it as a factor of a denominator, not when it only appears after
factoring.

## Outcomes

`PASS`; `FAIL(reason)` with reason `wrong-value`, `wrong-dimension`, `not-reduced`, `restrictions`,
`solutions`, `inequality`, `wrong-error`, `not-evaluated` (returned as typed with free variables),
`unit-not-convertible` (the result's unit has no SI conversion) or `result-unparsable`; `ERROR`
(engine error); `UNEXPECTED_OK` (an error was expected); `PANIC`; `TIMEOUT`; and `GENERATOR`. A
`GENERATOR` outcome means the algebra input and its expected form disagree at a sample point, so
the case itself is wrong.

## Categories

| Category | Cases | Covers |
|---|---:|---|
| arithmetic | 15,000 | Integer sums and products, numbers with up to 200 digits, fractions, nested fractions, decimals with `{,}` and `.`, powers, roots, scientific notation, precedence traps, absolute values, long chains |
| functions | 10,000 | Trig at special angles (degrees and multiples of π) and in radians, identities, `\ln`, `\log`, `\binom`, `\sum` (closed forms, telescoping, nested), determinants up to 8×8, domain errors |
| equations | 8,000 | Linear, quadratic (rational, irrational, double and no real roots), degree 3–6 with integer roots, a quintic, fractional, root, exponential, logarithmic and absolute-value equations, unknowns next to units |
| derivatives | 7,000 | Polynomials, product and quotient rule, chain rule with sin, cos, exp, ln and roots, `x^x`, higher derivatives by nesting, as `d^n/dx^n` and in prime notation, `\partial` |
| integrals | 7,000 | Polynomials, trig, exp, `1/x`, arctan, by parts, singular ends, kinks, reversed and equal bounds, double integrals, integrals with units, evaluation brackets `[\int f\,dx]_a^b`, `\oint` refused |
| limits | 8,000 | Removable singularities, standard limits (`sin x / x`, `(1 + kx)^{1/x}`, …), continuous points, divergent limits |
| algebra | 20,000 | Rational functions of order 1–40 in categories like those of the 2500-case table, plus continued fractions, binomials, geometric sums, multivariate and negative exponents |
| physics | 22,000 | Mechanics, gravitation, fluids, thermodynamics (°C), waves and optics, circuits, modern and nuclear physics (Bq, Gy, Sv, decay), photometry (cd, lm, lx, sr), chemistry (mol, kat), and niche cases: extreme prefixes (ym … Ym), calendar units, mixed prefixes, named-unit collapse, radians, `\qty`, dimension errors |
| complex | 3,000 | Imaginary mode: sums, products, quotients and integer powers of Gaussian rationals, powers of `i`, roots of negative numbers, modulus, `\Re`, `\Im`, `\overline`, Euler's formula, `\exp` of a complex number, quadratics with a negative discriminant, and the errors that the other modes keep giving |
| linear-systems | 2,000 | Linear systems mode, typed and as a matrix: unique, inconsistent and underdetermined systems of 1–7 equations in 2–6 unknowns built around a chosen solution, over- and underdetermined shapes, unknown names in every order (`x_{10}` after `x_2`, `x, y, z` before `a`), both elimination methods, Gaussian rational coefficients with the `i` toggle, and the input errors |
| syntax | 3,000 | Malformed input that must be refused, mathematical errors, notation variants (`\left`, `\dfrac`, `\times`, `*`, unicode operators, spacing) |
| verify | 1,000 | Verify mode: equations of fractions, sums and products, recurring decimals against their fraction, powers and roots, inequalities, chains of up to five relations, sin, cos and tan at special angles in radians and degrees, π, e, √2 and √3 against nearby fractions, logarithms, German answers, and a free variable as the error |
| base-n | 1,000 | Base-N mode in all four bases, half in the default word (32 bit, signed) and half in one of 8, 16, 32, 64 bits, signed or unsigned: single numbers up to the ends of the range and full patterns (`FF` = −1 in 8 bit signed), + − · ÷ with truncating division, and/or/xor/xnor written plain, as `\operatorname` or `\land`/`\lor`/`\oplus`, `Not` and `Neg`, mixed precedence levels, numbers with a prefix in another base (`\mathrm{h}1F`), shifts and rotations (`shl`, `shr`, `sar`, `rol`, `ror`, `<<`, `>>`) with counts past the width, `f32(x)`/`f64(x)` of decimal numbers (normal, subnormal, zero, infinity) with the exact value and rounding error from an exact Python reference, results outside the word that wrap (with the warning), and literals too big for the word and division by zero as errors |
| matrix | 1,000 | Matrix mode with inline `pmatrix` and `bmatrix` up to 5×5 and integer, fraction and decimal entries: sums and differences, a number times or over a matrix (`\cdot`, juxtaposed, `\div`, `\frac`), products and chains of them, transposes (`T`, `\top`), powers, inverses up to 4×4 and `A \cdot A^{-1}`, determinants up to 5×5 (also 0), `rref` with dependent rows, `A - k\,\operatorname{Identity}(n)`, combined inputs, and the errors: singular, sizes that do not fit for + and ·, matrix + number, det of a non-square matrix |
| statistics | 1,000 | Statistics through `api::statistics`: one variable (1–200 values, integers, decimals, thirds, empty cells skipped) with n, mean, sums, both variances and standard deviations, min, the Casio quartiles, median and max; frequency columns (0 drops a row); two-variable values; all seven regressions on data near their curve with a, b, c, r and the estimates x̂ and ŷ; and the errors: regression without pairs, a frequency that is not whole, an empty list, equal x values, x = 0 for y = a + b/x, a short y column, a unit in a cell |
| distributions | 1,000 | Distributions through `api::distribution`: the normal density, the cumulative between two bounds or with an open one (empty or `\infty`), the inverse normal, all with the standard or a given σ and μ; binomial density and cumulative lists up to N = 400 with exact fractions (and k > N); Poisson density and cumulative lists; and the errors for σ, the area, p, μ, k, N and a missing N. Φ is computed from a 120-digit erf series |
| analysis | 1,000 | Analysis mode through `api::analyze`, `tangent`, `area` and `value_table`: polynomials of degree 2–5 from rational roots (double roots, irrational extrema, saddle points), rational functions with poles, holes and horizontal or oblique asymptotes, (ax + b)e^{cx}, a ln(x)/x^k and a x^k ln(x) with their asymptotes and sides, tangent and normal (also vertical), the area and signed area between bounds or the outermost zeros, tables of values with a pole, and the errors (an equation, two variables, unparsable input). Expected points come from Sturm sequences and a square-free decomposition |
| geometry | 1,000 | Geometry mode through `api::geometry`, angles in degrees, every object in one of its forms (points as cells, `(x\|y\|z)` or typed `A(…)`, lines by point and direction or two points, planes in coordinate, normal, three-point or parameter form, spheres by centre and radius or a point on them): distance and midpoint of two points; distance, foot, image and lying on for a point and a line or a plane; two lines identical, parallel (distance), intersecting (point, angle) or skew (distance), and their angle; line and plane intersecting (point, angle), parallel (distance) or contained; two planes identical, parallel (distance) or intersecting (angle); a sphere and a point (inside, on, outside, distance), a plane (circle, tangent, apart) or a line (secant, tangent, passant); volume, surface, circumference and area; and the errors |
| electronics | 20 | Electronics mode through `api::solve_circuit`, one circuit per template with drawn E12-style values and prefixes (`2{,}2k\Omega`, `500mA`): series chains of 2–4 resistors, 2–4 in parallel, a loaded divider, a current source, two sources, an unbalanced and a balanced bridge, a source with inner resistance, a three-stage ladder, and a short circuit as the error. Solved by nodal analysis over fractions |
| kirchhoff | 20 | Kirchhoff mode through `api::kirchhoff`, the parts placed on grid points where no three nodes lie on a line: eight source circuits (series, parallel, loaded divider, two sources, bridge, ladder, current source) with every part's U, I and P along the arrows; ten resistances between A and B for series-parallel, ladder and bridge networks under each `starDelta` setting (node rule, Δ → Y, Y → Δ), all giving the same value; and the errors: two voltage sources in parallel, a part without a value |
| open-features | 500 | The features that were open in `plans/plans.md` until 2026-11-04, written as gaps first and implemented since: linear, quadratic and cubic inequalities and rational ones with `<`, `>`, `\le`, `\ge` and rational bounds; in imaginary mode, the complex roots of `x^4 + a`, of biquadratics without a real root and of `x^n + a` up to degree 8, `\arcsin`, `\arccos` and `\arctan` of a complex number and on the branch cuts, and integrals of complex functions (`x + bi`, `e^{kix}`, `1/(x + bi)`); partial derivatives at a point in two variables, mixed and second partials; sums to `\infty` (ζ(2), ζ(4), geometric, telescoping, exponential, alternating and divergent series); the Gamma function at whole numbers, halves and fractions, its poles, and factorials of halves. Values at 80 digits (Γ by the Stirling series), checked against `cmath`, numerical quadrature and finite differences when written |

## Latest results

Full run (2026-11-05, 4 workers, 5 s limit, 337 s): **all 112,540 cases pass**, including the
1,592 gap cases written before the features they test were implemented and the 500 of
`open-features`. The slowest are the double integrals, about 2 s each.

The full breakdown of the full run is in `test/results/summary.md`.
