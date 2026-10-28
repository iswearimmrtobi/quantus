# Testing

Quantus has two kinds of tests:

1. **`cargo test`**: 366 tests that run in a few seconds. Run them before every commit.
2. **The generated test set** in `test/`: 103,000 cases run by a separate program, for broad
   coverage of the engine. A full run takes about 4 minutes, so run it when you change the engine's
   behaviour, not on every commit.

The frontend (`desktop/src/`) has no tests of its own; `npm run typecheck` is its only check. The
renderer's calls into the engine are covered by the Tauri IPC tests below.

## Running

```sh
cargo test --workspace                          # everything below, about 10 s plus build time
cargo test -p quantus-core                      # only the engine
cargo test -p quantus-desktop                   # only the Tauri commands
cargo test -p quantus-core --test golden        # one integration test file
cargo test -p quantus-core binomial             # every test whose name contains "binomial"
cargo test -p quantus-core --test golden -- --nocapture   # show println! output

cd desktop && npm run typecheck                 # TypeScript of the renderer
```

The number crates are built with `opt-level = 3` even in debug builds (see the root `Cargo.toml`);
without that, the integral cases run several times slower.

No single test should take longer than a second. Today `tests/golden.rs` breaks that rule (about
5 s, see [plans.md](plans.md)); everything else finishes in under 0.2 s per file.

## Unit tests (195)

`#[cfg(test)] mod tests` blocks inside the source files. They test one module at a time and live
next to the code they test, so the file you change is also where its tests are.

| Where | Tests | What |
|---|---:|---|
| `crates/quantus-core/src/analysis/` | 21 | Analysis mode: expression tree, conversion from and to LaTeX, evaluation, simplifier, exact radicals and trig values |
| `crates/quantus-core/src/electro/` | 36 | Circuit solver, wire resistance, reverse solving, material table |
| `crates/quantus-core/src/num.rs` | 15 | Exact numbers, rounding, powers, π and roots |
| `crates/quantus-core/src/quantity.rs` | 11 | `PhysicalQuantity`: arithmetic, conversion, dimension errors |
| `crates/quantus-core/src/api.rs` | 11 | The JSON shape the app receives (`CalculationResponse`) |
| `crates/quantus-core/src/eval/` | 20 | Calculator steps, evaluator, substitution, numeric root scan |
| `crates/quantus-core/src/latex/` | 28 | Lexer, number formats, serializer, exact forms (`\frac{\sqrt{2}}{2}`, `\frac{\pi}{2}`) |
| `crates/quantus-core/src/units/` | 17 | Dimensions, unit lookup, conversions |
| `crates/quantus-core/src/` (rest) | 20 | Sessions, settings, error messages, message language, algebra pipeline, rational roots |
| `desktop/src-tauri/src/tests.rs` | 16 | The Tauri commands through Tauri's own IPC layer with a mock runtime, as the renderer's `invoke` reaches them: evaluation, variables, error positions, settings and circuit files, the menu |

## Integration tests (171)

`crates/quantus-core/tests/*.rs`. Each file is its own test binary and uses only the public API of
`quantus-core`.

### Golden tests: parity with the old Kotlin engine

The engine was ported from Kotlin. Before the Kotlin code was removed, its tests and the behaviour
of the libraries it used (Symja, Indriya) were dumped to JSON in `crates/quantus-core/tests/golden/`.
These files replay those dumps, so the Rust engine is checked against what the old program did.

| File | Data | What |
|---|---|---|
| `golden.rs` | `evaluate.json`, `sessions.json` | 254 stateless calculations (result, LaTeX, unit, value, warnings, errors) and session scenarios with variables |
| `latex_golden.rs` | `roundtrip.json`, `parse_errors.json` | LaTeX parse/serialise round trips, and parse errors with their position |
| `quantity_golden.rs` | `quantity_cases.json` | 81 `PhysicalQuantity` operations |
| `units_oracle.rs` | `units_symbols.json`, `units_format.json` | Every one- to three-character unit symbol (188,499 candidates) against Indriya: known or not, dimension, factor, offset, printed form; and 79 formatted units |
| `electro.rs` | `electro.json` | The circuit calculator's JSON interface: request in, the fields of the response that must match |

To add a case, add a line to the JSON file; the runner picks it up. `golden.rs` stops at the first
case that fails, so a broken case hides every case after it in the file.

### Algebra

The symbolic simplification of rational expressions, built milestone by milestone:

| File | Tests | What |
|---|---:|---|
| `algebra_milestone1.rs` | 10 | Polynomials: monomial order, arithmetic, division with remainder, content, GCD and LCM |
| `algebra_milestone2.rs` | 11 | Finding the denominators of an expression, for domain restrictions |
| `algebra_milestone3.rs` | 14 | Binomial formulas, exact roots, factoring |
| `algebra_milestone4.rs` | 22 | Nested fractions, common denominators, expansion |
| `algebra_milestone5.rs` | 28 | The full simplification pipeline and checking that input and output are equivalent |
| `algebra_milestone6.rs` | 4 | Wiring into parser, serializer, calculator and API |
| `algebra_reference_integration.rs` | 5 | One reference expression, `3 / ((x + 3)/(x - 2)) + ((x - 102)/(x - 4))/(x + 3)`, end to end |
| `algebra_simplification.rs` | 37 | The 14 categories of simplification the engine has to handle |
| `algebra_binomial_direction.rs` | 8 | When results are written as a binomial and when they are expanded |

### Other

| File | Tests | What |
|---|---:|---|
| `complex.rs` | 10 | Complex results in imaginary mode |
| `settings.rs` | 10 | Each user setting switches its reading or writing rule |

## The generated test set (103,000 cases)

`test/` holds a large set of generated cases across arithmetic, functions, equations, derivatives,
integrals, limits, algebra, physics with units, complex numbers and syntax errors. Every expected
value is computed by the Python generator, independently of the engine.

```sh
cargo run --release -p quantus-core --example run_cases                    # all cases, ~4 min
cargo run --release -p quantus-core --example run_cases -- --filter physics --fail-only
python3 test/generator/generate.py --check                                 # regenerate test/cases
```

The runner writes `test/results/summary.md` and `test/results/failures.jsonl`. Commit both after a
full run, so the repository records the last result. Case format, pass criteria, runner options and
the latest numbers are in [`test/README.md`](../test/README.md).

`test/algebra-simplification-cases.md` is a table of 2,500 hand-checked algebra cases. It is not run
directly; it defines the conventions the generator's algebra cases follow.

## Before committing

```sh
cargo test --workspace && (cd desktop && npm run typecheck)
```
