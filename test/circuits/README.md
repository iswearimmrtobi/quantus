# Test circuits

Circuit files for trying the electronics mode by hand: open one with *Load* in the parts column.
`the_test_circuits_solve_to_their_documented_values` in `desktop/src-tauri/src/tests.rs` solves
every file and checks the values below. The `kirchhoff-*` files are for the Kirchhoff mode, see
below. The voltage of a part is φ(a) − φ(b), its current flows
from `a` to `b`. A source therefore shows a negative current when it delivers power.

| File | What it tests | Expected |
|---|---|---|
| `divider.json` | Series resistors | 12 V over 1 kΩ + 2 kΩ: I = 4 mA, U(R1) = 4 V, U(R2) = 8 V |
| `parallel.json` | Parallel resistors | 10 V over 100 Ω ‖ 400 Ω: I(R1) = 0.1 A, I(R2) = 0.025 A, I(V1) = −0.125 A |
| `precision.json` | Significant digits, exact results | 10 V over 3 Ω: I = 3.333… A, P = 33.33… W; follows the precision and *exact roots* settings |
| `current-source.json` | Current source | 2 A into 5 Ω ‖ 20 Ω: U = 8 V, I(R1) = 1.6 A, I(R2) = 0.4 A |
| `potentiometer.json` | Loaded potentiometer | 10 V, 10 kΩ at 0.25, 10 kΩ load on the wiper: U(R1) = 120/19 V ≈ 6.31579 V |
| `two-sources.json` | Two sources, one inner node | 10 V and 5 V, R1 = 1 kΩ, R2 = R3 = 2 kΩ: φ = 6.25 V, I(R1) = 3.75 mA, I(R2) = 0.625 mA towards V2, I(R3) = 3.125 mA |
| `wheatstone.json` | Balanced bridge | 1 kΩ / 2 kΩ on both sides: I(R5) = 0 A, U(R1) = 3.333 V |
| `cable.json` | Copper wires, power load (Newton iteration) | 230 V, 2 × 100 m of 2.5 mm² copper, 2 kW: P(L1) = 2000 W, U(C1) ≈ 6.331 V, I ≈ 9.202 A |
| `relative.json` | Source without a value | 1 kΩ + 3 kΩ: U(R1) = 25 %, U(R2) = 75 %, currents per volt of the source |
| `short-circuit.json` | Error message | A wire across the source: "short circuit" |

## Kirchhoff mode

The circuits above without a potentiometer, a power load or a cable also work in the Kirchhoff
mode and give the same values (`the_test_circuits_match_the_electronics_mode` in
`crates/quantus-core/tests/kirchhoff.rs`); `two-sources.json` and `wheatstone.json` show the most.
These are for what only the Kirchhoff mode has, and
`the_kirchhoff_test_circuits_solve_to_their_documented_values` in the same file checks them:

| File | What it tests | Expected |
|---|---|---|
| `kirchhoff-current-source.json` | A current source as a known branch current; the voltage across it is an unknown instead | 10 V with 2 Ω, 2 A into the junction, 4 Ω to the bottom: I(R1) = 1/3 A, I(R2) = 7/3 A, U(R2) = 28/3 V |
| `kirchhoff-crossing.json` | Wires that cross without a connection: loops from a spanning tree, with a note | 12 V across 4 Ω ‖ 12 Ω: I(R1) = 3 A, I(R2) = 1 A, I(V1) = −4 A |
| `kirchhoff-source-loop.json` | Two voltage sources in parallel: the equations contradict each other | Error, with the rules and the steps up to the contradiction |

The files were written by the editor's own `toFile` (`desktop/src/circuit.ts`), so the node
numbers in them match the positions. When the editor loads a file it works the nodes out again.
