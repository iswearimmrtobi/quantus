# One shared schematic editor

## Context

The circuit editors of Electronics/Kirchhoff (`components/CircuitEditor.tsx`, 643 lines) and Logic
(`components/LogicEditor.tsx`, 777 lines) already share the low level: `src/schematic.ts`
(grid, points, rotate, connectivity, move/turn, ids), `components/Schematic.tsx` (`useCanvasView`,
`useSchematicEditing`, `useLabelSizes`, `ZoomControls`, `GridBackground`, `WirePart`, `DragGhost`,
`InspectorPopover`, `InspectorHead`, `useFullscreen`), `labelLayout.ts` and `picture.ts`.

Still duplicated almost line for line: the palette column, the stage with the `<svg>` canvas and
its handlers (pointer, drop, keys, context menu), the label pipeline (requests → `placeLabels` →
placements frozen while dragging → `fitAll`), junction dots, leader lines, the HTML label layer,
the empty-canvas text, the popover wiring, the free spot of a new part, the part group (transform +
symbol + hit area + pin circles), the `.field` inputs, the turned-box helpers (`circuit.ts`
`turned` vs LogicEditor `turnedBox`), pins from local points (`terminals` vs `partPins`), label
sides from rotation (`partPlace` vs `labelPlace`), ids (`freshId` vs `freshIdOf`), the new wire,
and net numbering (`numberNodes` alias chains vs `numberNets` union-find). Small needless
differences: status note over vs under the canvas, fit only in Logic, export only in Logic, pin
radius, who owns full screen, `circuit-*` vs `logic-*` classes. Neither has undo or copy/paste.

Decided with the user: one shared editor shell that each mode drives through a *part kit*; shared
code in `src/schematic/`, the editor parts of each mode in `src/circuit/` and `src/logic/`;
editors aligned (status bar under the canvas, fit on open/load, SVG/PNG export everywhere, same
details); new in the shell for all three modes: undo/redo, copy/paste/duplicate, arrow keys;
vitest unit tests for the pure shared modules. Frontend only; the Rust engine is untouched.

## Target layout

```
desktop/src/schematic/            shared, no mode knowledge
  geometry.ts    ← schematic.ts; + placeLocal, turnedBox, wireBetween, freeSpot, sidesFrom
  nets.ts        joinPoints: union-find over pins, wire ends, wire middles
  labels.ts      ← labelLayout.ts; + CanvasLabel, useLabelSizes, useLabelPlacements
  picture.ts     ← picture.ts; + PictureColors, EXPORT_COLORS, appColors, schematicPicture
  history.ts     useHistory: undo/redo of `elements`, coalescing drags and typing
  clipboard.ts   copy/paste of a selection, fresh ids, offset
  keys.ts        KeyBinding table: default bindings, matcher, hint rows
  kit.ts         PartKit<E, K, S>
  view.tsx       useCanvasView, ZOOM_STEP, useFullscreen, useFitOnRequest, ZoomControls, GridBackground
  editing.ts     useSchematicEditing, Drag
  parts.tsx      PlacedPart, WirePart, JunctionDots, LeaderLines, DragGhost
  inspector.tsx  InspectorPopover, InspectorHead, TextField, SelectField, CheckedTextField
  StatusBar.tsx  corner + messages (error / warning / info) under the canvas
  ExportButtons.tsx  SVG/PNG buttons and error (picture.ts + shell.saveBytes)
  Editor.tsx     SchematicEditor<E, K, S>: the shell
  *.test.ts      vitest
desktop/src/circuit/   model.ts (← circuit.ts minus reduction), reduce.ts (reducedDrawing…),
                       kit.tsx (symbols, label places, inspector, picture style), CircuitEditor.tsx (wrapper)
desktop/src/logic/     circuit.ts (← logicCircuit.ts), kit.tsx, LogicEditor.tsx (wrapper)
```
The Logic views (`LogicView`, `KvMap`, `LogicTable`, …) and `logic.ts` stay where they are; only
editor files move.

## The part kit (schematic/kit.ts)

Parts are described once in their own coordinates; the shell derives the rest.

```ts
interface PartKit<E extends Placeable, K extends string, S = never> {
  id: string;                                         // clipboard: paste only into the same kit
  palette: K[]; paletteLabel(kind: K, t): string; paletteIcon(kind: K): ReactNode;
  newPart(elements: E[], kind: K, at: Point): E; spacing: number; canAdd?(elements: E[], kind: K): boolean;
  pins(e: E): Point[];                                // placed, by the model's own placeLocal-based pins
  body(e: E): Rect; hit(e: E): Rect; obstacles(e: E): Rect[];   // hit in local coordinates
  joining(elements: E[]): E[];                        // wires that join in their middle (ideal / all)
  dotsIgnore?(e: E): boolean;                         // terminals A/B are marks, not lines
  turned?(rotation: number): number;
  symbol(e: E, state?: S): ReactNode;                 // unturned
  pinPress?(e: E): 'draw' | 'move';
  tapOnly?(e: E): boolean; onTap?(e: E): void;
  pasted(e: E, into: E[]): E | null;                  // fresh id, free name (Logic), null drops it (3rd terminal)
  keys?: KeyBinding[];                                // Shift+R tilt
  pictureStyle(colors: PictureColors): string; exportName: TextKey;
}
```
`circuit/model.ts` `terminals()` (named, for `toRequest`) and `logic/circuit.ts` `partPins()` are
rebuilt on `placeLocal`, so the editor and the netlist cannot drift apart.

Labels are `CanvasLabel { id, anchor, sides, gap, corner?, leaderFrom?: Rect, className,
content: ReactNode, svg?: ReactNode }`, built by each mode (part labels, Kirchhoff extras, node
potentials, Logic names/ids); the shell lays them out, draws leaders and the HTML layer; the
picture draws `svg`.

`SchematicEditor` props: `elements, onChange, kit, labels, reserved?, overlay?, preview?,
partClass?(e), wireClass?(e), partState?(e): S, toolbar?, paletteExtras?, emptyCanvas,
stageExtras?` (U/I/P/R toggle), `status?, below?, fitKey?` (also resets the history),
`fullscreen?, inspector(e, {update, replace, remove, close})`.

## Steps (one commit each on `dev`, tick here; typecheck + vitest after each)

- [x] 1. **Vitest, baseline tests.** `vitest` dev dependency, `npm test` (= `vitest run`), licenses
  regenerated. Tests on today's code: `schematic.ts` (rotate, inside, corner, connected/junction
  points, movedBy, rotatedQuarter, inRectangle), `labelLayout.ts`, and `numberNodes`/`numberNets`
  on sample circuits (a source loop, a potentiometer, a 45° resistor, a
  T-junction, ground vs no ground). These pin the behaviour the refactor must keep.
- [x] 2. **Move files, no behaviour change.** Into `schematic/`, `circuit/`, `logic/` as above;
  `components/Schematic.tsx` split into `view.tsx`, `editing.ts`, `parts.tsx`, `inspector.tsx`,
  `labels.ts`. Imports fixed (App, ElectronicsView, KirchhoffView, KirchhoffOverlay, LogicCircuit,
  LogicGates, LogicView, the tests).
- [x] 3. **Shared geometry.** `placeLocal(part, p)` (quarter turns; 45°: eighth turn × √2), `turnedBox`
  (exact for 45°) replacing `turned` and `turnedBox`; `wireBetween(id, from, to)` for both
  `newWire`s and `newPart/newElement('wire')`; Electronics ids via `freshIdOf` (A/B kept);
  `sidesFrom(rotation, first)` for both label places; `freeSpot(view, count, spacing)`.
- [x] 4. **Shared nets.** `nets.ts` `joinPoints(points, joinEnds, joinMiddles)`; `numberNets` and
  `numberNodes` (ground = 0) on top. A point inside two ideal wires now joins both (the alias map
  took only the first): a fix, with a test and a changelog line.
- [x] 5. **Shared inspector fields.** `TextField`, `SelectField`, `CheckedTextField` (NameField with
  `check(text) → value | null`); both inspectors use them.
- [x] 6. **The shell, Logic on it.** `kit.ts`, `keys.ts` (one table drives `keyDown` and the palette
  hints; a "Keys" disclosure lists all), `useLabelPlacements`, `useFitOnRequest` (LogicEditor's
  pendingFit loop), `Editor.tsx`, `PlacedPart`, `JunctionDots`, `LeaderLines`. The shell owns the
  portal (`fullscreen` prop or its own); `LogicCircuit` only passes `fullscreen`.
  `logic/kit.tsx` + `LogicEditor` wrapper; `schematicPicture` replaces `circuitPicture`
  (`staticPicture` kept for LogicGates). CSS: shared classes `schematic-*`; one pin radius (5).
- [x] 7. **Electronics and Kirchhoff on the shell.** `circuit/kit.tsx` (`circuitKit(fields)`);
  `CircuitEditor` keeps its props, so `ElectronicsView`/`KirchhoffView` stay as they are; preview,
  highlight, overlay, reserved, Shift+R tilt as a kit key.
- [x] 8. **One status bar** under the canvas; Electronics' failure / `circuit.noGround` and Kirchhoff's
  note move there from the `.circuit-note` overlay.
- [x] 9. **Fit on open/load** for Electronics and Kirchhoff: load counter in `App.tsx` (like
  `logicLoads`) → `fitKey`.
- [x] 10. **SVG/PNG export** for Electronics and Kirchhoff via `ExportButtons` in the palette column;
  labels' `svg` = id + shown values as plain text (`value` `unit`); the Kirchhoff overlay is not
  exported. Texts `logic.export.*` → `schematic.export.*`, `circuit.export.name`.
- [x] 11. **Undo / redo.** `useHistory` in the shell wraps `onChange`: a drag is one entry (snapshot at
  start, pushed on drop if changed), typing in one inspector field is one entry, everything else
  one each; Ctrl+Z, Ctrl+Shift+Z / Ctrl+Y on the canvas; ↶ ↷ buttons by the zoom controls;
  limit 100; reset by `fitKey` (load, Logic "Draw from"). Logic: an undo is an edit, so the
  circuit counts as hand-drawn afterwards, as with any edit.
- [x] 12. **Copy / paste / duplicate.** Ctrl+C, Ctrl+V, Ctrl+D on the canvas: the selected parts (and
  wires among them) via `kit.pasted` (fresh ids; Logic switches/lamps get free names; a third
  terminal is dropped), offset two grid steps each paste, pasted parts selected. In-app
  clipboard keyed by `kit.id`.
- [x] 13. **Arrow keys** move the selection one grid step (Shift: five); a burst is one undo entry.
- [x] 14. **Docs.** `CHANGELOG.md` [Unreleased]; user guide pages of Electronics, Kirchhoff, Logic
  (keys, undo, copy/paste, export, status bar); `design/digital-logic.md` §10 file references,
  `plans/ux-rework.md`, `plans/kirchhoff.md`; `CLAUDE.md` and `docs/docs/contributing/testing.md`
  (`npm test`), `building.md` if it lists files. Every new text in all `locales/*.json`.

## Verification

Still open: the manual run through the checklist below in the real app (`npm run dev`); the
unit tests, typecheck and a renderer build pass. Remove this plan and its entry in `plans.md` once
it is done.


- Each step: `cd desktop && npm run typecheck && npm test`; `cargo test -p quantus-texts` when
  locales change; licenses regenerated in step 1. No full workspace test runs.
- Steps 2–7 must keep the step‑1 tests green unchanged (except the documented junction fix).
- Manual in `npm run dev` after steps 6, 7, 10, 13, in all three modes: palette click and drop;
  move/turn/delete/marquee; L wire and T-junction; pull a wire end; inspector beside the part, Esc
  closes then clears; Logic switch click/double click; Shift+R tilt; zoom keys/buttons/Fit/Home;
  full screen and Esc; labels clear of parts and still while dragging; Electronics values and
  potentials unchanged on `circuit.json`; Kirchhoff loop highlight and merge preview; Logic
  simulation colours, critical path, errors/warnings, Gates card picture; export SVG/PNG in each
  mode; undo/redo across drag, typing, add, delete, paste; copy/paste between two Logic circuits
  keeps names unique; arrows move.
