# Quantus – Electronics Mode (Specification)

A fourth mode for Quantus ("Elektronik" in the UI): a graphical circuit calculator for purely passive DC circuits (no diodes): resistor networks, voltage drop, cable calculations.

Target example: *100 m copper cable with 2.0 mm² and a 10 Ω load resistor – what is the voltage drop across the cable (material x)?*

---

## 1. Core Principles

1. **Nothing is guessed.** If an input is missing, the result is `Unknown(reason)`, e.g. "supply voltage missing". There are no silent defaults (exception: temperature, see 4.3).
2. **Wires are ideal (R = 0) by default.** As soon as material, length and cross-section are given, the wire resistance is applied automatically. There is no switch and no mode (not to be confused with the application mode of section 3).
3. **The return conductor emerges from the topology.** If the current path is source → wire → load → wire → source, there are two wires in the network. There is no "outbound and return" flag.
4. **L and C are 0 by default** (wire inductance and capacitance). They only take effect once specified, and only in AC (phase 6).
5. **Units and exactness** go through the existing `PhysicalQuantity` / `Value` types of `quantus-core`. The linear solver is exact (rational arithmetic); decimals only appear at the output boundary, like everywhere else in Quantus.

---

## 2. Decisions

| Topic | Decision |
|---|---|
| Missing inputs | `Unknown(reason)` |
| Wire | R = 0 until material, length and cross-section are complete |
| L and C | 0 by default |
| Temperature | Optional; 20 °C if not given (with a note on the result) |
| Scope v1 | DC only; data model allows AC later |
| Components v1 | Resistor, wire, voltage source, current source, potentiometer, constant-power load |
| Application mode | New `Mode::Electronics` in `Settings`, switched in the top bar next to Algebra / Physik / Imaginär |
| Input | Graphical schematic editor, hand-written SVG in React (no new frontend dependency) |
| Display | All values live on elements and nodes, `Unknown` greyed out |
| Reverse solving | One unknown, e.g. "max. 3 % drop" yields the required cross-section |
| Linear solver | Exact: Gaussian elimination on `Value` (no `nalgebra`, no floats) |
| Power load | Iterative (Newton-Raphson) in high-precision decimals |
| Code location | Module `quantus_core::electro` in the existing crate (no new crate) |
| Integration | Tauri commands calling `quantus_core`, circuit sent as JSON; no server |
| Units | Existing `PhysicalQuantity` / `Unit` (Ω, V, A, W, S, K and °C already exist) |
| Several unknowns | A result is `Known` only if it is exactly unchanged when each unknown is varied; otherwise `Unknown("hängt von R1, R2 ab")`. No partial or symbolic relations |
| Saving circuits | v1 has load and save of a circuit as a JSON file; no autosave |

---

## 3. Architecture

- **`quantus_core::electro`**, a module of the existing `quantus-core` crate. It is UI-independent, keeps no thread-local state, and depends only on the existing `num`, `quantity` and `units` modules. No new crate and no new Rust dependency.
- **Application mode:** `Mode::Electronics` is added to the `Mode` enum in `settings.rs` (serialised as `"electronics"`) and to the `Mode` type in `desktop/src/api.ts`. In the parser it behaves like `Physics` (units and constants known, `i` a variable), see `Settings::parse_options`. In the UI, selecting it replaces the calculator and history view by the schematic editor; the other modes are unchanged.
- **Interface to the frontend:** JSON in (circuit), JSON out (results per element/node ID), as serde structs in the style of `api.rs` (camelCase, absent values as `null`). `api.rs` gets a `solve_circuit` function that takes a `CircuitRequest` and returns a `CircuitResponse`.
- **Tauri command:** `solve_circuit` in `desktop/src-tauri/src/commands.rs` wraps `api::solve_circuit`. The circuit is recalculated on every change in the editor. The solver is stateless and does not touch the session store, so the command needs no lock; the precision setting is read from the store like in `evaluate`.
- The solver ignores editor data (positions); this is only passed through or kept by the frontend.
- Because the engine runs natively inside Tauri, a WASM build is not needed. If recalculation while dragging feels laggy, the command can be debounced in the frontend.

---

## 4. Data Model

### 4.1 Circuit

- `Circuit { nodes, elements }`, node 0 = ground/reference potential.
- All elements and nodes have stable IDs (to map results back to the editor).
- Editor metadata (x, y, rotation) lives on the element but is irrelevant to the solver.

### 4.2 Elements (v1)

| Element | Parameters | Note |
|---|---|---|
| `Resistor` | R | Missing value → `Unknown` |
| `Wire` | material, length, area, temp, r_override, l_per_m, c_per_m | see 4.3 |
| `VoltageSource` | U, optional internal resistance | ideal DC source |
| `CurrentSource` | I, optional internal conductance | ideal DC source |
| `Potentiometer` | R_total, wiper position (0..1) | modelled as two partial resistors |
| `PowerLoad` | P | nonlinear, see 6.2 |

Later: `Capacitor`, `Inductor` (DC: open / short circuit, only relevant in AC).

### 4.3 Wire

```rust
struct Wire {
    material:   Option<Material>,
    length:     Option<PhysicalQuantity>,   // dimension [L]
    area:       Option<PhysicalQuantity>,   // dimension [L]²
    temp:       Option<PhysicalQuantity>,   // °C or K; if absent: 20 °C
    r_override: Option<PhysicalQuantity>,   // Ω, e.g. from a datasheet
    l_per_m:    Option<PhysicalQuantity>,   // H/m, default 0
    c_per_m:    Option<PhysicalQuantity>,   // F/m, default 0
}
```

There are no dedicated `Length`, `Area` or `Resistance` types: every parameter is a `PhysicalQuantity`, and the dimension is checked on input (a length given in `V` is an error with the usual unit message). Units are converted through the existing unit system, so `2mm^2`, `2\,\mathrm{mm}^2` and `0{,}000002m^2` are the same cross-section.

Rules for the resistance:

- `r_override` set → this value applies.
- Material, length and cross-section complete → `R = ρ · (1 + α · (T − 20 °C)) · L / A`. Without `temp`, T = 20 °C and the result carries the note "at 20 °C". Only the temperature *difference* enters the formula, so `temp` in °C or K is converted to a difference to 20 °C before use (the unit system treats °C as an affine unit).
- Otherwise → R = 0. With partial input (e.g. length without material) a note is attached to the result ("wire W1 idealised: material missing"), so it is clear why the drop is 0.

```rust
fn resistance(&self) -> Result<(PhysicalQuantity, Vec<Warning>)>
```

`Result` is the crate's `quantus_core::Result` (a unit mismatch is a `QuantusError`).

L' and C' depend on geometry (spacing between outbound and return conductor), not on material, so they are never derived, only taken over if specified.

---

## 5. Material Database

Static table in Rust (a `const` array in `electro/materials.rs`, in the style of the unit registry), with resistivity ρ at 20 °C and temperature coefficient α. Both are stored as exact rationals (e.g. copper ρ = 172/10000 Ω·mm²/m), so they carry no rounding error into the exact solver.

Copper, aluminium, silver, gold, iron, constantan, brass, etc.

Reference for checking: copper ρ ≈ 0.0172 Ω·mm²/m.

---

## 6. Solver

All arithmetic uses `Value`, so the linear parts are exact. Results are formatted by the existing output pipeline (decimal at the user's precision; `exactRoots` and exact π apply as in the calculator).

### 6.1 Linear DC Solver (MNA)

- Modified Nodal Analysis: `A · x = z`, where x = node voltages plus currents through voltage sources. Solved by Gaussian elimination with pivoting on `Value` (`Value::add/mul/div` are exact; zero is detected exactly, so singular matrices are recognised without a tolerance).
- One stamp function per element (R, V, I).
- **Ideal wire (R = 0):** stamped as a 0 V source (equation `V_a − V_b = 0`, current as an extra variable). The current through the wire comes out directly.
- Results: node voltages, branch currents, power dissipation and voltage drop per element.
- Error cases with clear messages (as `QuantusError`, never NaN): floating nodes, short circuit (e.g. an ideal loop or an ideal wire in parallel with a voltage source), singular matrix.
- Preparation for AC: `Value` already has the imaginary unit `I`, so a complex MNA only needs complex stamps, not a new number type.

### 6.2 Power Load (nonlinear)

- Newton-Raphson wrapped around the linear solver. In each iteration the load is linearised at the current operating point (equivalent conductance plus equivalent current) and one MNA solve is performed.
- The iteration runs on `Decimal` values at working precision (a few digits above the requested precision), because the iterates are generally irrational. The converged result is turned back into a `Value` at the boundary.
- Error cases: "no solution" (the load demands more than the network can deliver, voltage collapses) and "did not converge". Both with their own message.

### 6.3 Unknown Logic

- Every parameter is `Option<PhysicalQuantity>`, every result is `Known(q)` or `Unknown(reason)`.
- Pragmatic dependency check: the network is solved twice with different (exact rational) test values for the unknown parameter. If a quantity is exactly equal in both runs, it does not depend on it and is `Known`. Exact arithmetic means no tolerance is involved.
- **Exploiting linearity (purely linear networks only):** if the source is unknown, the solver internally uses 1 V and outputs ratios (e.g. relative voltage drop 14.7 % in the example). Absolute values stay `Unknown`. If any other quantity is specified (e.g. "10 A through the load"), it scales the whole network and all values become absolutely computable.
- **With a power load** the network no longer scales proportionally. In that case absolute values are `Unknown` and the scaling trick does not apply.

### 6.4 Reverse Solving

- Exactly one unknown plus one target value, e.g. "voltage drop ≤ 3 %" → required cross-section or maximum length.
- Implemented via bisection or secant method over the forward solver, on `Decimal` values. Robust, no special-case logic.
- Error cases: target not reachable, more than one unknown.

---

## 7. Interface (JSON)

Same conventions as `api.rs`: camelCase field names, absent values as `null`, errors as `ApiError { error, position }`.

- **Input (`CircuitRequest`):** list of nodes and elements with IDs, parameters and connections; optionally a reverse-solve request (unknown, target quantity, target value). Parameters are given as the text the user typed (value plus unit, e.g. `"2mm^2"`) and parsed by the existing quantity parser, so the editor accepts exactly what the calculator accepts. `precision` is optional, as in `CalculationRequest`.
- **Output (`CircuitResponse`):** per element and node ID the computed values as `Known` or `Unknown(reason)`. A known value carries what a calculation response carries (`result` as LaTeX, plain `value`, `unit`). Plus notes (warnings) and errors.
- The frontend maps results to the corresponding element via its ID. The TypeScript types go to `desktop/src/api.ts`, next to `Calculation`.

---

## 8. Frontend (Tauri/React)

- Schematic editor as a hand-written SVG component under `desktop/src/components/` (no new dependency, in line with the small dependency list); the circuit is built purely by drag and drop:
  - Components are dragged from a palette onto the canvas, moved by dragging, rotated and deleted from the keyboard or a context menu.
  - Cables are drawn by dragging from a component's terminal to another terminal (or to an existing cable, which creates a junction node). A drawn cable is a `Wire` element; dragging its ends reconnects it.
  - Terminals snap to each other, so connections are made by dropping onto a terminal, never by typing node numbers. Nodes and their IDs are created by the editor.
  - Selecting an element opens its parameters for entry. The existing `MathField`/`LatexInput` can be reused for value entry.
- The mode switch in the top bar gets a fourth entry "Elektronik". The UI language is German, so all labels, notes and error texts shown in the editor are German.
- **Live display:** every element and node immediately shows its computed values (rendered with `Katex`), `Unknown` is greyed out, notes (e.g. "idealised") are visible.
- Recalculation on every change via the `solve_circuit` Tauri command.
- **Load / save:** two buttons ("Laden", "Speichern") read and write the circuit as a `.json` file. The path comes from the native file dialog of `tauri-plugin-dialog` (the only new plugin; added in `Cargo.toml`, `main.rs`, `package.json` as `@tauri-apps/plugin-dialog`, and `dialog:allow-open` / `dialog:allow-save` in `capabilities/default.json`, like the clipboard plugin). The file itself is read and written by two Tauri commands (`load_circuit`, `save_circuit`) with `std::fs`, as `commands.rs` already does for `settings.json`, so `tauri-plugin-fs` and its path scopes are not needed. The file holds the circuit as the editor keeps it: nodes, elements with their parameter texts, and the editor metadata (x, y, rotation), in the same structure as the `CircuitRequest`, plus a `version` field for later format changes. Loading validates the file and reports a broken one as an error instead of replacing the circuit. There is no autosave; the circuit is lost when the app closes unless it was saved.
- **German texts** (labels, `Unknown` reasons, notes, errors):
  - Missing input: "Unbekannt: Versorgungsspannung fehlt"
  - Depends on an unknown: "Unbekannt: hängt von R1 ab"
  - Idealised wire: "Leitung W1 idealisiert: Material fehlt"
  - Default temperature: "bei 20 °C"
  - Ratio without a source: "relativ, 14,7 % (Spannung unbekannt)"
  - Solver errors: "Kurzschluss", "Knoten nicht verbunden", "keine Lösung: Last zu groß", "nicht konvergiert"

---

## 9. Tests

In the style of the existing test suites (unit tests in the module, integration tests and JSON golden files under `crates/quantus-core/tests/`):

- Analytical references: voltage divider, balanced Wheatstone bridge, series and parallel networks, copper cable by hand calculation. Because the solver is exact, these compare exactly (as rationals), not within a tolerance.
- Property tests: Kirchhoff (sum of currents at each node = 0), power balance (generated = consumed power). Both hold exactly.
- Unknown logic: partial inputs, missing source, ratio output, scaling via a specified quantity.
- Power load: convergence, "no solution" case, comparison with hand calculation (to the requested precision).
- Reverse solving: sanity check by feeding the result back through the forward solver.
- Golden file `electro.json` for the `CircuitRequest` → `CircuitResponse` JSON shape, plus a command test in `desktop/src-tauri/src/tests.rs`.

### Reference Example

Copper, 2 × 100 m (out and back), 2.0 mm², 10 Ω load, 230 V:

- R_wire = 0.0172 · 200 / 2 ≈ **1.72 Ω**
- I = 230 / 11.72 ≈ **19.6 A**
- Voltage drop across the wire ≈ **33.7 V** (≈ 14.7 %)

Without a supply voltage: the relative drop of 14.7 % is known, absolute values are `Unknown`.

---

## 10. Implementation Order

Each step is one commit on `master`, built and tested before the next.

1. Material table, `Wire` and integration with `PhysicalQuantity` (the example works without a solver).
2. Linear exact MNA with `Known/Unknown` logic and tests.
3. Power load (Newton) and reverse solving.
4. `Mode::Electronics`, JSON interface (`api.rs`) and Tauri command.
5. Schematic editor (React/SVG) with live display, then load and save as JSON.
6. Later: AC (complex impedances via `Value::I`, capacitor and inductor, π-model of the wire with `R + jωL` in series and C/2 at both ends in shunt, output as magnitude and phase).

---

## 11. Open Points

None at the moment.
