---
sidebar_position: 4.5
title: Test inventory
---

# Test inventory

Every test Quantus has, and how many of each, as of 2026-11-04. How to run them is on
[Testing](testing.md); the case format and the pass rules of the generated set are in
[`test/README.md`](https://github.com/iswearimmrtobi/quantus/blob/master/test/README.md).

| Kind | Count | Where | Run with |
|---|---:|---|---|
| Unit and integration tests | 1,088 | `#[test]` functions in the workspace | `cargo test --workspace` |
| Generated cases | 112,540 | `test/cases/`, 21 categories | `cargo run --release -p quantus-core --example run_cases` |
| Hand-checked algebra table | 2,500 | `test/algebra-simplification-cases.md` | not run; the generator's algebra cases follow it |
| Circuit files | 13 | `test/circuits/` | solved by two of the integration tests |

The `cargo test` counts are the `#[test]` attributes in each place.

## `cargo test`

| Crate | Tests | What |
|---|---:|---|
| `quantus-core`, unit tests in `src/` | 394 | One module at a time, next to its code |
| `quantus-core`, integration tests in `tests/` | 434 | The public API, one binary per file |
| `quantus-desktop` (`desktop/src-tauri`) | 27 | The Tauri commands through Tauri's IPC layer with a mock runtime |
| `firmware/ui` | 174 | The handheld UI: menus, editor, sheets, settings, sessions, Wi-Fi, the screen |
| `firmware/engine` | 50 | The handheld's UI joined with the engine behind its backend |
| `firmware/mirror` | 9 | Sending the simulator's picture to the board |

### Unit tests in `quantus-core/src`

| Module | Tests |
|---|---:|
| `src/algebra/` | 12 |
| `src/analysis/` | 80 |
| `src/electro/` | 37 |
| `src/eval/` | 64 |
| `src/kirchhoff/` | 27 |
| `src/latex/` | 36 |
| `src/stochastics/` | 10 |
| `src/units/` | 19 |
| `src/writeup/` | 12 |
| the files directly in `src/` (`api`, `num`, `linear`, `quantity`, `settings`, `basen`, …) | 97 |

### Integration tests in `quantus-core/tests`

The golden tests among them (`golden.rs`, `latex_golden.rs`, `quantity_golden.rs`, `units_oracle.rs`,
`electro.rs`) are few functions that each replay a whole JSON dump from `tests/golden/`, such as
254 calculations or 188,499 unit symbols, so their count here understates what they check.

| File | Tests |
|---|---:|
| `algebra_binomial_direction.rs` | 8 |
| `algebra_identities.rs` | 5 |
| `algebra_milestone1.rs` | 10 |
| `algebra_milestone2.rs` | 11 |
| `algebra_milestone3.rs` | 14 |
| `algebra_milestone4.rs` | 22 |
| `algebra_milestone5.rs` | 53 |
| `algebra_milestone6.rs` | 4 |
| `algebra_reference_integration.rs` | 5 |
| `algebra_simplification.rs` | 39 |
| `analysis.rs` | 43 |
| `angles.rs` | 8 |
| `basen.rs` | 2 |
| `complex.rs` | 10 |
| `complex_polar.rs` | 3 |
| `electro.rs` | 4 |
| `functions.rs` | 7 |
| `geometry.rs` | 48 |
| `geometry_view.rs` | 25 |
| `german_steps.rs` | 6 |
| `golden.rs` | 2 |
| `kirchhoff.rs` | 23 |
| `latex_golden.rs` | 3 |
| `library_search.rs` | 7 |
| `linear_systems.rs` | 6 |
| `matrices.rs` | 6 |
| `number_format.rs` | 5 |
| `quantity_golden.rs` | 1 |
| `result_forms.rs` | 5 |
| `sessions.rs` | 6 |
| `settings.rs` | 12 |
| `solving.rs` | 8 |
| `units_conversion.rs` | 4 |
| `units_oracle.rs` | 2 |
| `verify.rs` | 3 |
| `writeup.rs` | 14 |

## The generated test set

112,540 cases in 21 categories. Every expected value is computed by the Python generator in
`test/generator/`, independently of the engine. The first eleven categories go back to the first
version of the engine; the nine from `verify` on cover the modes added since, and two of them,
`electronics` and `kirchhoff`, are kept to a handful of circuits. The last, `open-features`, holds
the features that were open in `plans/plans.md` until 2026-11-04, written as gaps first and
implemented since.

| Category | Cases | Mode | Into the engine through |
|---|---:|---|---|
| arithmetic | 15,000 | Physics | `LatexCalculator::evaluate` |
| functions | 10,000 | Physics | `evaluate` |
| equations | 8,000 | Physics | `evaluate` |
| derivatives | 7,000 | Physics | `evaluate` |
| integrals | 7,000 | Physics | `evaluate` |
| limits | 8,000 | Physics | `evaluate` |
| algebra | 20,000 | Physics | `evaluate`, then checked by the algebra pipeline |
| physics | 22,000 | Physics | `evaluate` |
| syntax | 3,000 | Physics | `evaluate` |
| complex | 3,000 | Imaginary (and the other modes' errors) | `evaluate` |
| linear-systems | 2,000 | Linear systems | `api::linear_system` |
| verify | 1,000 | Verify | `evaluate` |
| base-n | 1,000 | Base-N | `evaluate` |
| matrix | 1,000 | Matrix | `evaluate` |
| statistics | 1,000 | Statistics | `api::statistics` |
| distributions | 1,000 | Distributions | `api::distribution` |
| analysis | 1,000 | Analysis | `api::analyze`, `tangent`, `area`, `value_table` |
| geometry | 1,000 | Geometry | `api::geometry` |
| electronics | 20 | Electronics | `api::solve_circuit` |
| kirchhoff | 20 | Kirchhoff | `api::kirchhoff` |
| open-features | 500 | Physics, Imaginary | `evaluate` |

### Latest results

The first twenty categories were run in full on 2026-11-04: all 112,040 cases pass, including the
1,592 gap cases written before the features they test were implemented. `open-features` was added
after that run; on its own all 500 of its cases pass, now that its features are implemented.

### By category and subcategory

Each case has a subcategory; `--filter category/subcategory` runs just those. "Checked as" lists the
expectation kinds, which `test/README.md` explains.

### arithmetic (15,000)

Checked as: `number` 15,000.

<details>
<summary>Subcategories</summary>

| Subcategory | Cases |
|---|---:|
| integer-sum | 1,748 |
| decimals | 1,457 |
| fraction-sum | 1,457 |
| integer-product | 1,457 |
| powers | 1,310 |
| fraction-product | 1,166 |
| precedence | 1,165 |
| roots | 1,019 |
| big-integer | 874 |
| nested-fraction | 874 |
| scientific | 873 |
| absolute-value | 582 |
| long-chain | 582 |
| identities | 436 |

</details>

### functions (10,000)

Checked as: `number` 9,579, `error` 421. 736 of them are marked as not implemented (`gap`).

<details>
<summary>Subcategories</summary>

| Subcategory | Cases |
|---|---:|
| logarithms | 1,685 |
| trig-special-angle | 1,685 |
| sums | 1,473 |
| trig-combination | 1,053 |
| trig-radians | 1,053 |
| determinant | 1,052 |
| binomial-coefficient | 842 |
| not-implemented | 736 |
| domain-error | 421 |

</details>

### equations (8,000)

Checked as: `solutions` 7,209, `error` 791. 137 of them are marked as not implemented (`gap`).

<details>
<summary>Subcategories</summary>

| Subcategory | Cases |
|---|---:|
| quadratic | 2,400 |
| linear | 2,000 |
| higher-degree | 1,280 |
| fractional | 1,120 |
| special | 800 |
| unknown-with-unit | 263 |
| not-implemented | 137 |

</details>

### derivatives (7,000)

Checked as: `number` 7,000.

<details>
<summary>Subcategories</summary>

| Subcategory | Cases |
|---|---:|
| polynomial | 2,100 |
| transcendental | 1,960 |
| quotient-rule | 907 |
| product-rule | 843 |
| higher-by-nesting | 840 |
| notation | 350 |

</details>

### integrals (7,000)

Checked as: `number` 6,245, `quantity` 700, `error` 55. 271 of them are marked as not implemented (`gap`).

<details>
<summary>Subcategories</summary>

| Subcategory | Cases |
|---|---:|
| polynomial | 2,450 |
| transcendental | 2,450 |
| special | 980 |
| with-units | 700 |
| not-implemented | 271 |
| notation | 149 |

</details>

### limits (8,000)

Checked as: `number` 6,988, `error` 1,012. 448 of them are marked as not implemented (`gap`).

<details>
<summary>Subcategories</summary>

| Subcategory | Cases |
|---|---:|
| standard-limit | 2,800 |
| removable-singularity | 2,000 |
| continuous | 1,600 |
| divergent | 960 |
| not-implemented | 448 |
| notation | 192 |

</details>

### algebra (20,000)

Checked as: `algebra` 20,000.

<details>
<summary>Subcategories</summary>

| Subcategory | Cases |
|---|---:|
| different-denominators | 2,693 |
| common-factor | 2,116 |
| same-denominator | 1,924 |
| nested-fraction | 1,731 |
| polynomial-plus-fraction | 1,731 |
| cancels-to-zero | 1,539 |
| repeated-factors | 1,347 |
| binomial-expansion | 1,153 |
| multivariate | 1,153 |
| continued-fraction | 962 |
| expand-and-collect | 961 |
| irreducible-factor | 961 |
| binomial-cancel | 769 |
| geometric-sum | 576 |
| negative-exponent | 384 |

</details>

### physics (22,000)

Checked as: `quantity` 20,900, `error` 1,100.

<details>
<summary>Subcategories</summary>

| Subcategory | Cases |
|---|---:|
| dimension-mismatch | 686 |
| non-si-unit | 679 |
| named-unit-collapse | 607 |
| celsius | 504 |
| ohm | 499 |
| photon-energy | 466 |
| doppler | 350 |
| period | 350 |
| frequency-of-light | 343 |
| lens | 335 |
| flow-rate | 333 |
| wave-speed | 323 |
| pressure | 315 |
| heat-conduction | 313 |
| buoyancy | 311 |
| density | 304 |
| thermal-energy | 304 |
| ideal-gas | 302 |
| escape-velocity | 286 |
| kepler | 282 |
| stefan-boltzmann | 282 |
| orbital-speed | 281 |
| capacitor-energy | 278 |
| parallel-resistors | 278 |
| hydrostatic-pressure | 277 |
| heat | 275 |
| kinetic-energy | 274 |
| lc-resonance | 273 |
| potential-energy | 265 |
| lorentz-force | 263 |
| work | 260 |
| pendulum | 257 |
| velocity | 256 |
| inclined-plane | 255 |
| newton | 253 |
| wire-field | 253 |
| gravitation | 251 |
| kinematics | 251 |
| momentum | 250 |
| electric-power | 249 |
| rc-time-constant | 249 |
| de-broglie | 247 |
| inductor-energy | 247 |
| spring | 247 |
| coulomb | 245 |
| rotation | 244 |
| series-resistors | 243 |
| mass-energy | 239 |
| offset-unit | 238 |
| circular-motion | 237 |
| free-fall | 236 |
| power | 235 |
| equivalent-dose | 233 |
| decay | 232 |
| activity | 228 |
| electric-energy | 228 |
| electron-energy | 220 |
| decay-count | 216 |
| magnetic-flux | 215 |
| amount-of-substance | 214 |
| nuclear-radius | 208 |
| absorbed-dose | 203 |
| luminous-flux | 203 |
| concentration | 201 |
| gray-vs-sievert | 201 |
| named-constant | 201 |
| catalytic-activity | 196 |
| illuminance | 195 |
| calendar-units | 192 |
| radian | 191 |
| inverse-square | 189 |
| hertz-vs-becquerel | 188 |
| volume-units | 188 |
| extreme-prefix | 187 |
| speed-units | 180 |
| area | 179 |
| unit-power | 179 |
| mixed-prefix-sum | 177 |
| unit-root | 171 |
| lorentz-factor | 148 |
| particle-number | 122 |
| function-of-unit | 89 |
| exponent-with-unit | 87 |
| snell | 59 |

</details>

### syntax (3,000)

Checked as: `error` 2,100, `number` 900.

<details>
<summary>Subcategories</summary>

| Subcategory | Cases |
|---|---:|
| malformed | 1,350 |
| notation | 900 |
| math-error | 750 |

</details>

### complex (3,000)

Checked as: `complex` 2,634, `complex-solutions` 244, `error` 122.

<details>
<summary>Subcategories</summary>

| Subcategory | Cases |
|---|---:|
| power | 307 |
| product | 307 |
| quotient | 307 |
| sum | 307 |
| modulus | 245 |
| sqrt-negative | 245 |
| mixed | 244 |
| quadratic | 244 |
| i-power | 184 |
| euler | 183 |
| parts | 183 |
| exp | 122 |
| other-modes | 122 |

</details>

### linear-systems (2,000)

Checked as: `linear` 1,900, `error` 100.

<details>
<summary>Subcategories</summary>

| Subcategory | Cases |
|---|---:|
| unique | 800 |
| infinite | 400 |
| none | 300 |
| complex-unique | 204 |
| complex-infinite | 107 |
| errors | 100 |
| complex-none | 89 |

</details>

### verify (1,000)

Checked as: `truth` 960, `error` 40.

<details>
<summary>Subcategories</summary>

| Subcategory | Cases |
|---|---:|
| fractions | 188 |
| inequality | 150 |
| chain | 120 |
| irrational | 80 |
| trig | 77 |
| logarithm | 70 |
| trig-degrees | 63 |
| recurring | 62 |
| power-quotient | 41 |
| errors | 40 |
| root | 39 |
| power | 38 |
| root-product | 32 |

</details>

### base-n (1,000)

Checked as: `basen` 907, `error` 93.

<details>
<summary>Subcategories</summary>

| Subcategory | Cases |
|---|---:|
| arithmetic | 220 |
| logic | 200 |
| precedence | 140 |
| literal | 120 |
| prefix | 120 |
| not-neg | 100 |
| overflow | 100 |

</details>

### matrix (1,000)

Checked as: `matrix` 840, `number` 120, `error` 40.

<details>
<summary>Subcategories</summary>

| Subcategory | Cases |
|---|---:|
| product | 140 |
| determinant | 120 |
| inverse | 120 |
| sum | 120 |
| rref | 100 |
| scalar | 100 |
| power | 80 |
| transpose | 80 |
| combined | 50 |
| identity | 50 |
| errors | 40 |

</details>

### statistics (1,000)

Checked as: `statistics` 950, `error` 50.

<details>
<summary>Subcategories</summary>

| Subcategory | Cases |
|---|---:|
| one-variable | 250 |
| frequencies | 150 |
| two-variable | 150 |
| regression-quadratic | 68 |
| regression-power | 65 |
| regression-abExponential | 62 |
| regression-inverse | 59 |
| regression-logarithmic | 53 |
| errors | 50 |
| regression-linear | 47 |
| regression-eExponential | 46 |

</details>

### distributions (1,000)

Checked as: `distribution` 940, `error` 60.

<details>
<summary>Subcategories</summary>

| Subcategory | Cases |
|---|---:|
| normal-cumulative | 200 |
| normal-density | 150 |
| binomial-density | 133 |
| inverse-normal | 120 |
| poisson-density | 119 |
| binomial-cumulative | 117 |
| poisson-cumulative | 101 |
| errors | 60 |

</details>

### analysis (1,000)

Checked as: `analysis` 700, `tangent` 100, `area` 100, `table` 70, `error` 30.

<details>
<summary>Subcategories</summary>

| Subcategory | Cases |
|---|---:|
| rational | 250 |
| polynomial-3 | 122 |
| polynomial-2 | 113 |
| area | 100 |
| tangent | 100 |
| exponential | 80 |
| logarithmic | 70 |
| table | 70 |
| polynomial-4 | 54 |
| errors | 30 |
| polynomial-5 | 11 |

</details>

### geometry (1,000)

Checked as: `geometry` 970, `error` 30.

<details>
<summary>Subcategories</summary>

| Subcategory | Cases |
|---|---:|
| point-line | 140 |
| point-plane | 140 |
| distance-points | 65 |
| line-plane-intersecting | 59 |
| planes-intersecting | 59 |
| lines-intersecting | 58 |
| midpoint | 55 |
| lines-skew | 50 |
| line-plane-contained | 43 |
| lines-identical | 41 |
| line-plane-parallel | 38 |
| sphere-point | 33 |
| errors | 30 |
| lines-parallel | 30 |
| planes-parallel | 22 |
| angle-lines | 21 |
| sphere-line | 20 |
| planes-identical | 19 |
| sphere-plane | 19 |
| circle-circumference | 16 |
| sphere-surface | 15 |
| circle-area | 14 |
| sphere-volume | 13 |

</details>

### electronics (20)

Checked as: `circuit` 19, `error` 1.

<details>
<summary>Subcategories</summary>

| Subcategory | Cases |
|---|---:|
| parallel | 3 |
| series | 3 |
| bridge | 2 |
| current-source | 2 |
| inner-resistance | 2 |
| ladder | 2 |
| loaded-divider | 2 |
| two-sources | 2 |
| balanced-bridge | 1 |
| short-circuit | 1 |

</details>

### kirchhoff (20)

Checked as: `equivalent` 10, `circuit` 8, `error` 2.

<details>
<summary>Subcategories</summary>

| Subcategory | Cases |
|---|---:|
| resistance-bridge | 6 |
| resistance-ladder | 2 |
| resistance-series-parallel | 2 |
| two-sources | 2 |
| bridge | 1 |
| current-source | 1 |
| ladder | 1 |
| loaded-divider | 1 |
| missing-value | 1 |
| parallel | 1 |
| series | 1 |
| source-loop | 1 |

</details>

### open-features (500)

Checked as: `inequality` 175, `number` 137, `complex` 100, `complex-solutions` 75, `error` 13.

<details>
<summary>Subcategories</summary>

| Subcategory | Cases |
|---|---:|
| inequality | 125 |
| complex-roots | 75 |
| complex-integral | 50 |
| complex-inverse-trig | 50 |
| gamma | 50 |
| infinite-sum | 50 |
| partial-derivative | 50 |
| rational-inequality | 50 |

</details>

## Circuit files

`test/circuits/` holds 13 circuit files drawn in the editor, with their expected values in its
[README](https://github.com/iswearimmrtobi/quantus/blob/master/test/circuits/README.md): ten for the
Electronics mode, three only for the Kirchhoff mode. `the_test_circuits_solve_to_their_documented_values`
(desktop) and the two circuit tests in `kirchhoff.rs` solve them.
