# UX rework before the next release

## Context

The modes added since v0.2.0 (Geometry, Base-N, Logic and its circuit editor) work, but they are
hard to use. A walk through the running desktop app on 2026-11-18 (debug build, fresh config,
2360×1560 window) found the problems listed in each phase below. On top of those, two wishes:

1. **Geometry:** a sidebar with all objects, and a 3D view that fills the rest of the window.
2. **Logic:** from a truth table straight to gates, as NAND only, NAND with 2 inputs only, NOR
   only and NOR with 2 inputs only.

Nothing is released until Phases 1–4 are done. One commit per step on `dev`, each step's box
ticked in the same commit, a CHANGELOG line under `[Unreleased]` for every user-visible change,
texts in every file of `locales/`.

Effort: S = an hour or two, M = a day, L = several days.

## Decisions

| Topic | Decision |
|---|---|
| 2-input gates | No longer a setting. Every output gets all four gate forms (NAND, NAND 2-input, NOR, NOR 2-input). `logicTwoInput` goes from the settings, the dialog, the docs and the handheld's engine |
| Where the four forms live | A **Gates** card in the results, a segmented switch over six forms (AND–OR, OR–AND, NAND, NAND 2-in, NOR, NOR 2-in): formula, cost, a picture of the circuit and **Open in editor** |
| Response shape | `OutputResponse.nand / nor / nand_rewrite / nor_rewrite` become one list `gate_forms`, each item tagged with `kind`, `two_input` and `origin` (minimal or rewrite) |
| Draw forms | `DrawForm` gets `Nand2` and `Nor2`; `Nand` and `Nor` mean any number of inputs |
| Geometry layout | Objects in a sidebar on the left (about 380 px, resizable), the 3D view fills the rest of the window's height and width, and the task sits at the foot of the sidebar |
| Object rows | Collapsed by default (swatch, name, the built formula, eye, ×). A click opens the row's editor in place, and only one is open at a time |
| Hidden objects | `SceneObject.hidden` in the core: built, so others can use it, but not drawn |
| Rewrite pictures | `LogicTask::Draw { from_expression }` draws the rewrite of a typed or traced expression (NAND/NOR forms only) |
| The Gates card's choice | Kept in the scene's `editor` data (`gates: { [output]: { form, fromExpression } }`), no core field |
| The task's answer in 3D | Drawn by the core (`ViewRequest.task`), highlighted in the view |
| Docs screenshots | None; the docs have no screenshots anywhere, Phase 3 is described in text |
| Object colours | By the object's place in the scene (`ViewObject.index`), not among those drawn, so hiding one or a broken one recolours none |

---

## Phase 1: Logic, from the truth table to gates (the wish)

### 1.1 Core: all four gate forms per output (M)

`crates/quantus-core/src/logic/interface.rs`, `nand.rs`, `layout.rs`.

- [x] New `GateFormResponse` fields: `kind: GateKind` (`nand`/`nor`, serialised lowercase),
  `two_input: bool`, `origin: "minimal" | "rewrite"`. Keep `latex`, `gates`, `inputs`, `levels`
  and `worked`.
- [x] `OutputResponse`: replace `nand`, `nor`, `nand_rewrite`, `nor_rewrite` with
  `gate_forms: Vec<GateFormResponse>`, in this order: NAND, NAND 2-in, NOR, NOR 2-in (from the
  chosen minimal DNF and KNF), then the rewrites of a typed expression or traced circuit in the
  same order. Build them in `output_response` (around L788 and L822) by calling
  `GateForm::nand`/`nor`/`rewrite` with `two_input` false and true. When nothing is split (every
  gate already has 2 inputs or fewer), leave out the 2-input copy and set `two_input: true` on
  the one kept, so the UI does not show two identical forms. Compare by `GateForm::split.is_empty()`.
- [x] `DrawForm`: add `Nand2` and `Nor2` (serialised `nand2`, `nor2`). In `Generated::add`
  (`layout.rs:76`) the `two_input` parameter goes; it follows from the form.
  `LogicTask::Draw` gets `from_expression` (default false): a NAND or NOR form rewritten from the
  typed or traced expression, so the Gates card can draw the rewrites too.
- [x] `Options.two_input` goes.
- [x] `writeup/logic.rs`: write the NAND and NOR forms from `gate_forms` (the minimal ones, any
  width), plus the 2-input ones when they differ, as two more `step` lines.
- [x] Tests in `tests/logic.rs`: the majority of three has four forms. NAND has 4 gates, NAND
  2-in has more gates and no gate with more than 2 inputs. A 2-variable function has only two
  forms (nothing to split). `Draw { form: Nand2 }` gives a netlist whose gates all have ≤ 2
  inputs. Rewrite the 22 existing NAND/NOR asserts for the new shape. Run only
  `cargo test -p quantus-core --test logic` and `--test writeup`.

### 1.2 Core: the setting goes (S)

- [x] Remove `logic_two_input` from `Settings` (`settings.rs` L288, L327, the serde test at L456).
  Old config files that still hold `logicTwoInput` must still load: check that `Settings` has no
  `deny_unknown_fields`, and add a test that reads a settings JSON with the old key.
- [x] `firmware/engine/src/lib.rs:525`: drop the field. Check with `cargo check -p quantus-engine`.
- [x] `tests/logic.rs:2249`: drop `logic_two_input: false` (that test then asks for the
  non-split form from `gate_forms`).
- [x] `design/digital-logic.md` (L406, L549, L592): the setting replaced by "every output has both".

### 1.3 Desktop: the types (S)

- [x] `desktop/src/api.ts`: `DrawForm = 'dnf' | 'knf' | 'nand' | 'nand2' | 'nor' | 'nor2'`;
  `GateFormResult` gets `kind`, `twoInput` and `origin`; `LogicOutputResult.gateForms` replaces the
  four fields; `Settings.logicTwoInput` goes (L750).
- [x] `SettingsDialog.tsx`: remove the checkbox (L548–551) and the default (L151); remove
  `app.settings.logicTwoInput*` from `locales/*.json`; remove the row in
  `docs/docs/reference/settings.md:157`.
- [x] `LogicView.tsx:57–60` and `LogicCircuit.tsx:48`: the setting goes from the dependency
  lists; `drawnFrom` (`logicCircuit.ts:331`) loses its `twoInput` argument.
- [x] `npm run typecheck`.

### 1.4 Desktop: the Gates card (M)

New component `desktop/src/components/LogicGates.tsx`, used by `LogicResults.tsx` instead of the
four NAND/NOR `Section`s (L240–249).

- [x] A segmented switch with six buttons: **AND–OR**, **OR–AND**, **NAND**, **NAND 2-in**, **NOR**,
  **NOR 2-in**. A 2-in button is left out when the core sent no separate 2-input form. The choice is
  kept per output in the scene's editor data (like `chosen`), so it survives a reload, and is
  NAND by default.
- [x] Under it: the formula (`eq(latex)`), the cost line (`gates · gate inputs · levels`, the
  existing `gates()` helper), **Copy** and **Steps**.
- [x] The rewrite forms (from a typed expression) as a second small switch, "from the minimal
  form / from your expression", shown only when the output has an expression or a circuit.
- [x] Texts: `app.logic.gates.title`, `.andOr`, `.orAnd`, `.nand`, `.nand2`, `.nor`, `.nor2`,
  `.fromMinimal`, `.fromExpression`, `.open`, in `en.json` and `de.json`.

### 1.5 Desktop: the circuit picture in the Gates card (M)

- [x] When the switch changes (and on every new answer), send `{ kind: 'draw', form, output }`
  through the existing `task()` path (move it from `LogicCircuit.tsx` to `logic.ts` so both use
  it). Drop an answer to an older request, as `LogicView.run` does.
- [x] Turn the answer into parts with `fromGenerated` (`logicCircuit.ts`) and draw it read-only
  with `circuitPicture` (`LogicEditor.tsx`, already used for the SVG export), fitted to the
  card's width, in the app's colours instead of black on white (give `circuitPicture` a
  `theme` argument).
- [x] **Open in editor**: calls `draw(form, output)` of `LogicCircuit` (lift it into
  `LogicView` or pass a callback), opens the editor and scrolls to it.
- [ ] Simulation in the picture: the wires coloured by the table's current row, as in the editor
  (`scene.row`). This is optional and can follow later.

### 1.6 Desktop: Draw from with six forms (S)

- [x] `LogicCircuit.tsx` `FORMS` (L20): add `nand2` and `nor2` with texts `logic.draw.nand2`,
  `logic.draw.nor2` ("NAND, 2 inputs only", "NOR, 2 inputs only").
- [x] A circuit drawn from `nand2` is drawn again from `nand2` when the function changes
  (`generated.form`); check this by hand.

### 1.7 Docs and changelog (S)

- [x] `docs/docs/guides/logic/results.md`: the Gates card, the six forms, the picture, Open in
  editor; `editor.md`: Draw from with six forms; `index.md` § settings: `logicTwoInput` gone.
- [x] CHANGELOG `[Unreleased]`: the Gates card with the four NAND/NOR forms in the Logic entry
  under Added. The Logic mode is itself unreleased, so the setting *NAND and NOR from 2-input
  gates only* was taken out of its entry instead of being listed under Removed.

---

## Phase 2: Logic, the rest of the review

### 2.1 Results in less space (M)

- [x] `LogicResults.tsx`: the eight cards become three: **Expression and canonical forms**
  (one card, two columns), **Minimal forms** (DNF and KNF side by side, each with its choice,
  cost, Copy and Steps) and **Gates** (Phase 1). The page shrinks from about 2.5 screens to about one.
- [x] Next to the KV diagram (`logic-views`), the chosen minimal DNF under the map instead of the
  empty space to its right. Check at 1280 px and in the narrow layout.

### 2.2 Input (S)

- [x] `logic.empty` (`locales/*.json`): the example rendered with `Katex`, not as raw
  `\land`/`\lor`/`\sum`. Split the text into words and two formulas.
- [x] Operator keys above the function lines, **∧ ∨ ¬ ⊕ ( )** and **Σm**, inserting into the
  focused MathField (as the Base-N chips do).
- [x] Enter on a line no longer adds an empty line. **+ Function** stays the only way
  (`LogicLines.tsx`).
- [x] The `f` label left of a line goes when the line itself starts with `f =`; the placeholder
  sits at the field's left edge.
- [x] The **expression** badge only on the line, not on the table's column, the results head
  (`SourceMark` in `ResultsOf`) and the Truth table button.

### 2.3 Circuit editor (M)

- [x] `LogicEditor.tsx`: **Draw from** and **Arrange** go into a toolbar above the canvas, out
  of the palette (`toolbar` prop). On an empty canvas with a function typed, show the six Draw
  from buttons in the middle of the canvas instead of "Drag input switches…".
- [x] After a drawing, fit the whole circuit (`fitKey` exists; check that it runs after the
  layout is measured, since G3 was cut off).
- [x] The note "Drawn from the minimal DNF…" and the stats box (`corner`) go into a bar under
  the canvas, not over it.
- [x] The critical path in a colour of its own (not the yellow of "on" and of the selection),
  dashed, and off by default. Its button in the bar turns it on.
- [x] The inspector: as a popover beside the chosen part instead of a fixed 440 px column. The
  canvas gets that width.
- [x] The editor as a tab: **Table & KV | Circuit**, so it no longer pushes the table and
  results 950 px down (`LogicView.tsx`).
- [x] (Could, M) fewer wire crossings in `layout.rs`: each input's rail next to its NOT gate,
  rails sorted by first use.

### 2.4 Zoom in the circuit editors (S)

Found after 2.3: only Ctrl + wheel zoomed, and a touchpad pinch does not reach the page in
WebKitGTK.

- [x] `useCanvasView` (`Schematic.tsx`): the wheel zooms about the pointer (with Ctrl too);
  Shift + wheel and sideways swipes pan. `zoomBy` zooms about the canvas's middle.
- [x] `ZoomControls` in the canvas's top left corner of both editors: −, the zoom in % (a click
  goes back to 100 %), +, **Fit**. Keys on the canvas: `+`/`=`, `-`, `0` (fit). `CircuitEditor`
  gets a fit like `LogicEditor.fitAll`.
- [x] Texts `app.schematic.*`, `app.circuit.pan`; docs (`logic/editor.md`, `electronics.md`,
  `kirchhoff/index.md`); CHANGELOG → Changed.

### 2.5 One circuit for all outputs (M)

A truth table with several output columns gave one circuit per output in the Gates card.

- [x] Core: `Draw { output: None, from_expression: true }` takes the minimal form's gate form for
  an output without an expression, failing only when no output has one. Tests in
  `tests/logic.rs`: two outputs drawn into one netlist with both lamps and a shared NOT; a NAND
  rewrite over one typed and one table output.
- [x] Gates card: with several outputs the picture and **Open in editor** draw all of them in one
  circuit; the cost line adds the whole circuit's figures.
- [x] Editor: output tabs **All | f | g | …** over the canvas instead of the select (the
  Logic view's output tabs, with **All** in the circuit view). On a drawn
  circuit a tab draws the same form for that output (or all) at once; on a hand-made or edited
  one it only chooses what **Draw from** draws.
- [x] Docs (`logic/results.md`, `logic/editor.md`), CHANGELOG (the Logic entry).

### 2.6 The circuit editors in full screen (S)

- [x] `useFullscreen` (`Schematic.tsx`): the editor, put into the page's body, fills the window
  over the app (under its dialogs). Its switch ⤢/⤡ is the last of the zoom controls, in the
  Logic, Electronics and Kirchhoff editors. Esc leaves (it clears a selection first); in Logic the
  output tabs move into the toolbar, and the Table & KV tab or another mode leaves too.
- [x] Texts `app.schematic.fullscreen*`, docs (`logic/editor.md`, `electronics.md`,
  `kirchhoff/index.md`), CHANGELOG.

---

## Phase 3: Geometry, sidebar and full 3D view (the wish)

### 3.1 Core: hidden objects (S)

`crates/quantus-core/src/geometry/interface.rs`, `view.rs`.

- [x] `SceneObject.hidden: bool` with `#[serde(default, skip_serializing_if = "is_false")]`, so
  old sessions load unchanged.
- [x] `Builder::scene`: a hidden object is built (so `g: X = A + …` still finds A) but not pushed
  to `shown`, and it is left out of `Bounds::fit`.
- [x] Tests (`tests/geometry_view.rs`): a line through a hidden point is drawn and the point is not;
  the bounds ignore the hidden point; a session with `hidden` round-trips.

### 3.2 Core: the view says which object a primitive belongs to (S)

- [x] `ViewObject` gets `index`: the object's place in the scene (not among those drawn), so a
  click in the view can select the row in the sidebar.

### 3.3 Desktop: the layout (M)

`GeometryView.tsx`, `styles.css`.

- [x] `GeometryView` returns a two-column grid: the sidebar (`geometry-sidebar`) left, the plot
  (`geometry-stage`) right. It no longer uses `analysis-panel` with its `max-width: 1280px`; the
  workspace does not scroll, and each column scrolls on its own.
- [x] The stage fills the window: `height: 100%` instead of `.geometry-plot .plot-canvas
  { height: 460px }` (`styles.css:2612`). The ResizeObserver in `GeometryPlot` (L76) already
  follows the size.
- [x] The sidebar's width can be dragged (240–600 px) and is kept in `localStorage`. A button
  collapses it to a thin strip, so the view gets the whole window.
- [x] The narrow layout (`@media (max-width: 760px)` of its own): sidebar over the view, the view at least
  60 vh high.

### 3.4 Desktop: the sidebar (M)

New `GeometrySidebar.tsx`. It takes over the typed line and the parts of `GeometryObjects.tsx`.

- [x] Top: the typed line (`MathField`, as now), and beside it a **+** menu with Point, Vector,
  Line, Plane, Sphere, Circle (the old Add list). Each inserts its form as a new row and
  opens it.
- [x] The list: one compact row per object: colour swatch, name, the built formula small
  (`answer.latex`, otherwise the error in red), an eye toggle (`hidden`) and ×. Hidden rows are
  greyed.
- [x] A click on a row opens its editor below it (the current `Form`, `VariantSwitch`,
  name field and steps, laid out for 380 px: the coordinate cells in one row, wrapping). Only
  one row is open; Esc closes it.
- [x] Rows can be reordered by dragging. The order is the build order, so a move that puts an
  object before one it needs shows that object's error. Keep it simple: move up and move down
  buttons in the open row.
- [x] Bottom of the sidebar: the task panel (`GeometryTaskPanel`), with the result and steps
  scrolling inside the sidebar.
- [x] Selection: hovering a row highlights its object in the view (thicker stroke, the others
  faded). A click on a marker or label in the view selects its row (needs 3.2).
- [x] Texts: `app.geometry.sidebar.collapse`, `.expand`, `.hide`, `.show`, `.moveUp`,
  `.moveDown`, `app.geometry.add.menu`.

### 3.5 Desktop: picking a task in the sidebar (M)

- [x] With one or two objects selected (Ctrl + click on rows or in the view), the task panel
  lists the tasks that fit them (point and plane → distance, foot of the perpendicular,
  reflection, relation; two lines → relation, angle, distance; …) as buttons. A click sets the
  task with these operands.
- [x] The operand pickers (`Pick` in `GeometryTask.tsx:48`) list only objects of a fitting
  kind; `kinds` is already passed, so check every task's slots.
- [x] The result shown once: the result lines, then the steps without the final line repeated.
  Does `Steps` take `hidden` per line? If not, drop the last step when it equals `result`.
  (It does not: `stepsBelow` drops the trailing steps that repeat a result line. The pickers
  were checked against the core: relation no longer offers vectors, and the second pick lists
  only kinds that go with the first, from the table `PAIRS`.)

### 3.6 Docs and changelog (S)

- [x] `docs/docs/guides/geometry/` (`index.md`, `objects.md`, `tasks.md`): the sidebar, hiding,
  selecting, picking tasks; `contributing/api.md`: `hidden` and `index`. No screenshots: the docs
  have none anywhere.
- [x] CHANGELOG `[Unreleased]` → Changed: the Geometry mode's layout.

---

## Phase 4: Geometry, the task's answer in the view

### 4.1 Core (M)

- [x] `ViewRequest.task: Option<GeometryTask>`. `geometry_view` answers it as `geometry` does
  and adds its figures as extra `Shown` with a flag `result: true`:
  - distance: the segment between the two closest points (point–plane: point and foot);
  - foot of the perpendicular, midpoint, reflection: the new point, and the perpendicular as a
    dashed segment;
  - relation: the intersection point, line or circle;
  - angle: the two directions from the intersection point and an arc;
  - tangent: the tangent plane or line.
- [x] New stroke `Stroke::Result` and primitive field `result: bool`. The labels are the
  result's name (`F`, `d = …` at the segment's middle).
- [x] Tests: a point–plane distance draws a segment ending on the plane; a reflection draws the
  mirrored point.
  (Done with two answer-only figures, `Figure::Segment` and `Figure::Angle`, and the stroke
  `Guide` for the dashed helper lines. The answer's figures follow the scene's in
  `ViewResponse.objects`, marked `result`.)

### 4.2 Desktop (S)

- [x] `GeometryPlot` gets `task` and sends it with the view request; `.geometry-result` drawn in
  the accent colour, on top. The other objects fade while a result is shown.

---

## Phase 5: Base-N

- [ ] **5.1 Hex digits upright (S).** Typed `FF` shows as italic variables. In Base-N with
  HEX, MathField turns the letters A–F into `\mathrm{…}` as they are typed (the same
  form a bit flip writes). The **Symbolic** row uses the core's LaTeX, which writes them
  upright. Check in `basen.rs::write`.
- [ ] **5.2 All bases at once (M).** Replace the chips `= DEC BIN OCT BITS` with a fixed
  block: HEX, DEC, OCT and BIN of the result, one line each with Copy, BIN grouped in fours.
  The values are already in the result's forms (`FormKind::{Dec,Hex,Bin,Oct}`).
- [ ] **5.3 The keys (S).** The 25 chips (`App.tsx`) in four labelled groups on one line,
  wrapping as a whole: Logic (and or xor xnor Not Neg), Shift (<< >> shl shr sar rol ror), Float
  (f32 f64), Suffix (d h b o).
- [ ] **5.4 A bit flip keeps the expression (S).** A click on a bit enters the flipped value as
  a new calculation (history) instead of overwriting the field, or, if overwriting stays, a
  note "replaces your input" and Ctrl+Z brings it back. When the word size shrinks, a literal
  that no longer fits is truncated with the wrap warning instead of the error "does not fit
  8 bits" (core: `basen.rs::literal`, decide for literals produced by a flip only).
- [ ] **5.5 The bit view (S).** 1s clearly set apart (accent colour, bold), the index under the
  bit it names (`BitView.tsx` `.bit-index`, align right under the group's lowest bit), a cursor
  and hover on bits, "IEEE 754" on one line.
- [x] **5.6** A walk through all widths (the programmer view's check 4.6).

---

## Phase 6: Polish (could)

- [ ] Geometry: `B(4|0|-1)` renders as `(4|0| − 1)`; write the sign as unary in the object's
  LaTeX (core, `geometry/show.rs`).
- [ ] Geometry: labels overlap in the view (A and K, g on B). Nudge labels away from each
  other in `view::project`, as `labelLayout.ts` does for the schematics.
- [ ] Geometry: axis ticks with more contrast.
- [ ] Logic: the NAND/NOR formulas with three stacked overlines. In the Gates card, offer the
  gate-by-gate form with named signals (`G_1 = \overline{A \land B}` …) as the default view.
- [x] Check by hand: typing `A(1|2|3)` key by key in the MathLive field (automated typing
  garbled `|` and the auto-closed `)`).

---

## Verification

- Per step: the targeted tests named in it (`cargo test -p quantus-core --test logic`,
  `--test geometry`, `--test writeup`, `--test settings`), `cargo clippy -p quantus-core`,
  `npm run typecheck`. No full workspace run.
- Per phase: the app through tauri-driver with a throwaway config (as on 2026-11-18): Logic with
  the majority of three, the six forms in the Gates card, Open in editor, then a 2-variable
  function (no 2-in buttons). Geometry with six objects: the sidebar, hiding A (g still drawn),
  the view filling the window at 1280×800 and at full size, a distance drawn in the view. Base-N
  with `FF·3+1` in HEX, all four bases, a bit flip, then 8 bits.
- Before the release: the checks of RELEASING.md.
