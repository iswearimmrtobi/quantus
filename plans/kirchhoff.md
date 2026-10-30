# Kirchhoff mode

## Context

`plans/physics-and-circuits.md` §3 sketches a Kirchhoff calculator: draw a resistor network and get
the junction and loop rules, the linear system, its Gauss steps and all currents and voltages, worked
out the way a student does it. The parts it needs exist now. The electronics solver
(`crates/quantus-core/src/electro/`), the circuit editor (`desktop/src/components/CircuitEditor.tsx`),
indexed unknowns and the Gauss steps of the LGS mode (`linear.rs`, `eval/system.rs`,
`eval/system_steps.rs`, `eval/linear_system.rs`) are all done. The electronics MNA solver gives
the answer but not the *explained* route. This mode adds that route.

Decisions (from the questions):

| Topic | Decision |
|---|---|
| Placement | Own mode **Kirchhoff** in the mode switch, own page |
| Circuit | Its own schematic, separate from Electronics; same file format for load/save |
| Method | Branch-current method: (n−1) junction rules + (b−n+1) loop rules per connected part |
| Input | Circuit editor only; equations are generated, not editable |
| Parts | Resistor, voltage source, current source, ideal wire. Ground symbols are ignored. Potentiometer, power load and cable wires (material/length/area) are an error naming the part |
| Branches | A path between two junctions; series parts share one current `I_k` |
| Loops | The windows of the drawing (planar faces), clockwise, drawn on the canvas. Spanning-tree loops if the drawing has crossings, with a note |
| Directions | Automatic; clicking a branch's current arrow flips it. Negative results mean the opposite direction |
| Values | Every value must be given; a missing one is an error naming the part |
| Equations | First symbolic (`R_1 I_1 − R_2 I_2 = U_1`), then with values and units, then the matrix and the Gauss steps |
| Results | U, I, P per part on the canvas (the existing U/I/P/R toggles), plus a power balance |
| Copy | The full working as an `align*` block |
| Layout | Palette · canvas · inspector as in Electronics, plus a working panel under the canvas with a splitter |
| Names | Junctions `N_1…`, loops `L_1…` (en); `K_1…`, `M_1…` (de). Currents `I_1…` |

## Engine (`crates/quantus-core/src/kirchhoff/`)

A new module beside `electro`, reusing its parsing and value formatting.

- **Request** `KirchhoffRequest { elements, precision }`. Each element carries the
  `electro::interface::ElementRequest` fields (`#[serde(flatten)]`), plus the grid coordinates of its
  terminals `pa`, `pb` (integers, drawing units / `GRID`) and `reversed: bool`. The backend builds
  the nodes itself from the coordinates; node numbers are not sent.
- **`graph.rs`**: builds the drawing graph (vertices are grid points, edges are parts and wires).
  - Planarity check: any two segments that cross or overlap away from a shared endpoint mean the
    drawing is not planar. A terminal lying in the middle of a wire counts as a crossing too.
    Exact integer cross products throughout.
  - Faces by half-edge traversal: outgoing edges are sorted by angle exactly (quadrant plus cross
    product). The outer face of each connected part is the one with the opposite signed area.
  - Contraction: ideal wires merge their end points into one electrical node (union-find). A face
    with no edge other than wires disappears. A face's loop is its non-wire edges in order.
  - Fallback: if the drawing isn't planar, a BFS spanning tree on the contracted graph gives one
    fundamental loop per link, and the response carries a note.
  - Open ends (a node with one connection) are an error naming the part, in the style of the
    electro errors.
- **`branches.rs`**: junctions are contracted nodes with three or more connections. A branch is a
  maximal chain through nodes with two connections. A connected part with no junction is a single
  cycle branch.
  - Lead element: the first of the branch by kind (voltage source, current source, resistor), then by
    ID. Its `reversed` flag sets the reference direction. By default the current runs out of a
    source's + terminal (`a`), and through a resistor from `a` to `b`.
  - Numbering: branches `I_k` in lead order. Junctions are numbered by their drawing position
    (top to bottom, left to right). Loops are numbered the same way, by their anchor.
  - A voltage source's internal resistance is a series `R_{i,V1}` in its branch. A current source's
    internal conductance is a parallel branch of its own, with `R = 1/G`.
- **`equations.rs`**: the unknowns are the branch currents, except that a branch with a current
  source has a known current, and the voltage `U_{q,k}` across the source is the unknown instead.
  Equation count = unknown count per connected part.
  - Junction rule at each junction but the last per part (incoming +, outgoing −). The response
    notes that the dropped rule follows from the others.
  - Loop rule per loop: clockwise, `Σ ±R·I = Σ ±U_q`.
  - Each equation is written twice: symbolic, with part names, and with values and units. Values are
    converted exactly to Ω, V and A, then to rows of `[A | b]` as `Value`.
- **Solving**: `eval::system::solve_matrix(keys, matrix, settings.elimination)`, then
  `calculator.system_steps` and `calculator.system_result`, as in `eval/linear_system.rs::respond`.
  Those helpers are `pub(super)` in `eval`; widen them to `pub(crate)`. Settings `step_detail` and
  `elimination` apply as in LGS. A singular or inconsistent system, such as a loop of only voltage
  sources or two current sources in series, gives a clear error. The steps up to that point are
  still returned.
- **Results**: U, I and P per part from the branch currents. They use `ValueResponse` through
  electro's `known`/`with_prefix` (make them `pub(crate)`), so the canvas labels work unchanged.
  The power balance compares generated and consumed power, exactly.
- **Response** `KirchhoffResponse`:
  - `junctions`: name, anchor point, symbolic and numeric equation, and whether it was dropped.
  - `loops`: name, anchor (an interior grid point far from the edges), element IDs in order,
    symbolic and numeric equation.
  - `branches`: name, element IDs, direction per element, and the lead ID that a flip toggles.
  - Also: `unknowns`, `matrix`, `steps`, `result`, `elements: Vec<ElementResponse>`,
    `powerBalance`, `notes`, `alignBlock`.
- `Mode::Kirchhoff` in `settings.rs`, serialised as `"kirchhoff"`. It reads input like Physics, with
  units known and `i` not imaginary. `api::kirchhoff(settings, request)`.

## App

- Tauri command `solve_kirchhoff` in `desktop/src-tauri/src/commands.rs`, plus the wrapper and types
  in `desktop/src/api.ts`. `validate_circuit` accepts the optional `reversed` field.
- **Refactor `CircuitEditor`** so both modes use it:
  - Props `palette: EditorKind[]` and `fields` (Kirchhoff hides the wire's material fields).
  - Results come in as props (`results`, `voltages`, `failure`) instead of the editor solving by
    itself. Electronics moves its `solveCircuit` effect into a small `ElectronicsView`, unchanged in
    behaviour.
  - An `overlay` slot (SVG in the drawing transform) and extra labels for the layout in
    `labelLayout.ts`.
- `circuit.ts`: `toKirchhoffRequest(elements)`, which sends terminal points and `reversed`. The
  element ID prefix for an internal resistance doesn't apply; it is engine-only.
- **`components/KirchhoffView.tsx`**:
  - The editor with palette `voltageSource, currentSource, resistor, wire`.
  - Overlay: junction dots and names; current arrows on the lead part, which flip when clicked; a
    circular loop arrow with its name at each loop anchor. Hovering a loop or equation highlights
    its parts.
  - A working panel under the canvas, with a splitter: junction rules, loop rules (symbolic, then
    numeric), the matrix, the Gauss steps (reusing the steps rendering of `LinearSystemView`), the
    results and the power balance, and a copy button for `alignBlock`.
- `App.tsx` holds a separate `kirchhoffCircuit` state, so it survives mode switches, and shares the
  load/save buttons. `modeChoices` gets `mode.kirchhoff` and its hint.
- All strings go in `i18n/en.ts` and `de.ts`. Engine messages and labels use `msg!`.

## Implementation plan

Each phase is one or more commits on `dev`. Tick a box when its commit lands.

### Phase 1: graph

- [x] `graph.rs`: the drawing graph, crossings, faces, contraction, spanning-tree fallback.
- [x] Unit tests: single loop, ladder, Wheatstone (planar, 3 inner faces), crossing wires
  (fallback), wire-only loop, open end.

### Phase 2: branches and equations

- [x] `branches.rs`: junctions, branches, lead elements, directions, numbering.
- [x] `equations.rs`: junction and loop rules, symbolic and numeric, exact rows of `[A | b]`.
- [x] Tests: equation count and symbolic text for the reference circuits.

### Phase 3: solve and respond

- [ ] `Mode::Kirchhoff`, `api::kirchhoff`, steps, results, power balance, copy block.
- [ ] `tests/kirchhoff.rs`: hand-worked references (two sources with three resistors,
  Wheatstone); errors (unsupported part, missing value, open end, source loop).
- [ ] Property test against `electro::solve`: the R/U/I circuits in `test/circuits/` and generated
  planar grid ladders give exactly the same currents; a flipped branch negates its current.

### Phase 4: command and API

- [ ] `solve_kirchhoff` in `commands.rs`, types and wrapper in `api.ts`, `reversed` accepted by
  `validate_circuit`.
- [ ] Command test in `desktop/src-tauri/src/tests.rs`.

### Phase 5: editor refactor

- [ ] Props-driven `CircuitEditor` (palette, fields, results, overlay) and `ElectronicsView`;
  Electronics behaves exactly as before.

### Phase 6: Kirchhoff page

- [ ] Mode switch entry, `KirchhoffView`, separate circuit state in `App.tsx`, load and save.
- [ ] Overlay: junction names, current arrows that flip on click, loop arrows, highlight on hover.
- [ ] Working panel under the canvas with a splitter, copy, all strings in `en.ts` and `de.ts`.
- [ ] Check in the running app (`npm run dev`).

### Phase 7: docs

- [ ] `docs/docs/guides/kirchhoff.md`, the mode in the settings reference, the README mode list.
- [ ] CHANGELOG entry, `test/circuits/kirchhoff-*.json` for manual checks.

## Verification


Targeted only (no full runs):

- `cargo test -p quantus-core --test kirchhoff`, plus the unit tests in the module.
- `cargo test -p quantus-core --test electro` after the helper visibility changes.
- `cargo test -p quantus-desktop` for the command test (or the crate name in `src-tauri`).
- `cargo clippy --workspace --all-targets` and `cd desktop && npm run typecheck`.
- Manual in `npm run dev`:
  - Load `test/circuits/two-sources.json` and `wheatstone.json` in Kirchhoff mode. Check the loop
    windows, the names, that a flip negates the current, and that the steps change with step detail
    and Gauss/Gauss-Jordan. Check that copy pastes a compiling `align*` block.
  - Check that the values match the Electronics mode for the same file.
  - Check that Electronics still works unchanged.
