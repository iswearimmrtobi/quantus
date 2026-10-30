# Engine test set

105,000 generated cases that put the engine through arithmetic, elementary functions, equations,
derivatives, integrals, limits, algebraic simplification, physics with units, parser robustness,
complex numbers and the linear systems mode.
The 2500 hand-checked algebra cases from before are in `algebra-simplification-cases.md`.

| Path | What |
|---|---|
| `generator/generate.py` | Writes `cases/`. Standard library only, deterministic (seed 20261023). |
| `generator/gens/*.py` | One module per area; every expected value is computed there, independently of the engine. |
| `cases/<category>/<category>-NNN.json` | The cases, 1000 per file, 105 files. |
| `results/summary.md` | The last full run. `results/results.jsonl` (one line per case) is not committed. |
| `results/failures.jsonl` | Every case of the last full run that did not pass, one JSON line each. Committed. |
| `../crates/quantus-core/examples/run_cases.rs` | The runner. |
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

`--check` asserts 105,000 unique ids and parseable expectations, and compares the generator's unit
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
- `gap: true` marks inputs that `design/unsupported-inputs.md` lists as not implemented, such as
  one-sided limits, `\infty`, `\arcsin`, `\log_b`, named constants like `c`, and units like `eV` or
  `atm`. These cases make up about 3% of the set. They count like any other case, but the summary
  reports them separately.
- `settings` (optional) overrides `Settings` fields (`a_and_y_as_variables`, `letter_products`, `extra_unit_letters`, `mode`: `algebra`, `physics` (the default), `imaginary` or `linear`, and for `linear` also `linear_complex`, `elimination` and `unknown_names`).
- `request` (linear systems only) is what the mode sends to `api::linear_system`: `{"equations": latex}` or `{"matrix": {"rows": [[latex, …], …], "unknowns"?: [latex, …]}}`. `latex` then shows the system for the report.
- Values are exact rationals `p/q` or decimal strings. `rel_tol` is relative. If the expected value
  is 0 the comparison is absolute, and with the exact tolerance `1e-45` it must be exactly 0.

| `expect.kind` | Passes when |
|---|---|
| `number` | The result is dimensionless and `value` matches within `rel_tol`. |
| `quantity` | The result's dimension equals `dim` (base exponents `L M T I Θ N J`) and its value, converted to the coherent SI unit (°C becomes K), matches `value`. `unit`, if given, is only compared for a note. |
| `solutions` | The solver's real solutions equal `values` as a set, with SI values and `dim`. |
| `algebra` | The result equals `simplified` at 8 sample points (exact rationals), is fully reduced (`Poly::gcd` of numerator and denominator is constant), and its restrictions equal `restrictions` (values per variable) plus `poly_restrictions`. |
| `complex` | The result's real part (`value`) and imaginary part match `re` and `im` within `rel_tol`; a real result has the imaginary part 0. Used with `settings.mode` set to `imaginary`. |
| `complex-solutions` | The solver's roots, each split into real and imaginary part, equal `values` (`{re, im}` pairs) as a set. |
| `linear` | The unknowns equal `unknowns` (LaTeX, in column order), every matrix entry read equals `rows` (`[A \| b]`, a value or `{re, im}` each), and the outcome matches: for `unique` each unknown equals `values`, for `none` the result is "no solution", for `infinite` there are `free` parameters and the solution satisfies every row at two choices of them. |
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
`solutions`, `wrong-error`, `not-evaluated` (returned as typed with free variables),
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
| derivatives | 7,000 | Polynomials, product and quotient rule, chain rule with sin, cos, exp, ln and roots, `x^x`, higher derivatives by nesting |
| integrals | 7,000 | Polynomials, trig, exp, `1/x`, arctan, by parts, singular ends, kinks, reversed and equal bounds, double integrals, integrals with units |
| limits | 8,000 | Removable singularities, standard limits (`sin x / x`, `(1 + kx)^{1/x}`, …), continuous points, divergent limits |
| algebra | 20,000 | Rational functions of order 1–40 in categories like those of the 2500-case table, plus continued fractions, binomials, geometric sums, multivariate and negative exponents |
| physics | 22,000 | Mechanics, gravitation, fluids, thermodynamics (°C), waves and optics, circuits, modern and nuclear physics (Bq, Gy, Sv, decay), photometry (cd, lm, lx, sr), chemistry (mol, kat), and niche cases: extreme prefixes (ym … Ym), calendar units, mixed prefixes, named-unit collapse, radians, `\qty`, dimension errors |
| complex | 3,000 | Imaginary mode: sums, products, quotients and integer powers of Gaussian rationals, powers of `i`, roots of negative numbers, modulus, `\Re`, `\Im`, `\overline`, Euler's formula, `\exp` of a complex number, quadratics with a negative discriminant, and the errors that the other modes keep giving |
| linear-systems | 2,000 | Linear systems mode, typed and as a matrix: unique, inconsistent and underdetermined systems of 1–7 equations in 2–6 unknowns built around a chosen solution, over- and underdetermined shapes, unknown names in every order (`x_{10}` after `x_2`, `x, y, z` before `a`), both elimination methods, Gaussian rational coefficients with the `i` toggle, and the input errors |
| syntax | 3,000 | Malformed input that must be refused, mathematical errors, notation variants (`\left`, `\dfrac`, `\times`, `*`, unicode operators, spacing) |

## Latest results

Full run (2026-10-28, 8 workers, 5 s limit, 80 s): **102,336 of 103,000 pass (99.36%)**. All 100,646
cases for implemented features pass. The 664 that do not are all `not-implemented` cases that the
generator marks as gaps, listed one per line in `test/results/failures.jsonl`:

| Category | Outcome | Cases |
|---|---|---:|
| derivatives | ERROR (partial derivatives, `dx` notation not parsed) | 252 |
| equations | ERROR | 263 |
| integrals | ERROR | 149 |

The full breakdown is in `test/results/summary.md`. The linear-systems category was added after
that run; on its own (2026-10-30) all 2,000 of its cases pass.
