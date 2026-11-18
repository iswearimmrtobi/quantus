# UX rework before the next release

## Context

The modes added since v0.2.0 work, but they are hard to use. This list comes from a walk through
the running desktop app on 2026-11-18 (debug build, fresh config, 2360×1560 window): Geometry with
three points, a line, a plane and a sphere; Base-N with `FF·3+1` in HEX, a bit flip and a change to
8 bits; Logic with the majority of three, then its circuit drawn from the minimal DNF. Nothing is
released until the items marked **must** are done.

Effort: S = an hour or two, M = a day, L = several days.

## Geometry (`desktop/src/components/Geometry*.tsx`)

- [ ] **must, M: one long column.** Typed line, then a 3D view about 900 px high, then the Add list
  (six full-width rows), then the objects, then the task at the very bottom. The scene, the objects
  and the task's answer are never on screen together. Proposal: two columns, the 3D view on one
  side and the objects with the task on the other (`GeometryView.tsx`); the view's height follows the
  window.
- [ ] **must, M: the answer is not drawn.** `GeometryPlot` gets only `objects`
  (`GeometryView.tsx:69`), so a distance, a foot point, an intersection or a reflection never shows
  in the scene. Draw the task's result: a segment for a distance, the new point, the line of
  intersection, highlighted, with the objects it uses picked out.
- [ ] **must, S: the Add list is too big.** Six text rows over the full width. Make them a row of
  small buttons (or a menu) next to the typed line. Each inserts a template into that line.
- [ ] **should, S: each object is shown twice.** The editable line plus a rendered copy that is often
  identical (`A(1|2|3)` / `A = (1|2|3)`). Show the built form only when it says more (the vector
  form of a line, a converted plane), or on hover.
- [ ] **should, M: the task is a dropdown and then more dropdowns.** "Choose a task…" plus a
  "choose…" for each operand, listing every object whatever its kind. Proposal: offer the tasks
  that fit the objects you selected (click A and E → distance, foot point, reflection), and list
  only operands of a fitting kind. Several tasks at once, kept as a list, would match how
  exercises are set.
- [ ] **should, S: the result is shown twice.** `d = √6/6` sits above the steps and again as their
  last row (`GeometryTask.tsx`).
- [ ] **could, S: rendering.** `B(4|0|-1)` renders as `(4|0| − 1)` with a binary minus. Labels
  overlap in the view (A and K, g on B). Axis ticks are hard to read.
- [ ] **could, S: delete is a tiny × at the far right**, with low contrast.
- [ ] **check by hand:** typing `A(1|2|3)` key by key in the MathLive field. Automated keystrokes
  garbled `|` and the auto-closed `)`, so a real keyboard needs to confirm it works.

## Base-N (`desktop/src/components/BitView.tsx`, the Base-N keypad in `App.tsx`)

- [ ] **must, S: the hex digits look like variables.** Typed `FF` renders in italics as `F·F` in
  the field and in the Symbolic row; a bit flip writes `\mathrm{2FF}` upright. Render the digits
  upright as you type, the same way everywhere.
- [ ] **must, M: show all bases at once.** A programmer view shows HEX, DEC, OCT and BIN of the
  result together. Now you see one, and the others sit behind the chips `= DEC BIN OCT BITS`
  (and `=` is not clear). Make a fixed block with all four and copy on each.
- [ ] **should, S: 25 chips in two rows above the input.** Operators, functions, IEEE and the
  suffixes are mixed, and the second row is just `f64( d h b o`. Group them: logic (and or xor xnor
  Not Neg), shifts (<< >> shl shr sar rol ror), float (f32 f64), suffixes. Or put the rarer ones
  in a menu.
- [ ] **should, S: a bit flip throws away the expression.** `FF·3+1` becomes `2FF` with no hint
  and no undo. Then switching to 8 bits fails with "'2FF' does not fit 8 bits." Keep the
  expression (flip the result only, as a new line in the history), or at least say what happens
  ("replaces the input"), and let the narrower word truncate with the wrap warning.
- [ ] **should, S: the bit view.** 1s and 0s differ only a little in brightness, the bit indices
  (28, 24 …) do not sit under the bit they name, nothing shows that bits can be clicked except a
  tooltip, and "IEEE 754" wraps onto two lines in its segment button.
- [ ] Closes `programmer-view.md` 4.6 (the manual check) once the above is done.

## Logic (`desktop/src/components/Logic*.tsx`)

- [ ] **must, M: eight full-width result cards.** Expression, canonical DNF/KNF, minimal DNF,
  minimal KNF, NAND only, NAND gate by gate, NOR only, NOR gate by gate. Each holds one short
  formula, a stat line, Copy and Steps, and the page is 2.5 screens long. Proposal: one card with
  tabs or a compact table (form | formula | gates/literals | copy | steps), with the minimal DNF open
  by default.
- [ ] **must, S: the help text shows raw LaTeX.** "Type a function, f = A \land B \lor C or
  f = \sum m(3, 5, 6, 7)". Show it rendered, and give buttons or keys for ∧ ∨ ¬ (what does a
  student press?).
- [ ] **should, S: "expression" badges everywhere.** On the line, the table's column, before the
  results and in the result. Show the source once, on the line.
- [ ] **should, S: Enter adds an empty function line**, and "+ Function" is there as well. Do one
  or the other.
- [ ] **should, S:** the `f` label beside a line that already starts with `f =`. The empty line's
  placeholder sits far from the left edge.
- [ ] **could, S: NAND/NOR forms with three stacked overlines** are hard to read. Offer the
  gate-by-gate form with named intermediate signals instead.
- [ ] **could, S:** the KV diagram is small in a wide panel with empty space on the right; put
  the minimal form next to it.

## Logic circuit editor (`LogicEditor.tsx`, `LogicCircuit.tsx`, `Schematic.tsx`)

- [ ] **must, S: "Draw from" is hidden.** With a function typed, the empty canvas says "Drag input
  switches…" while Draw from the minimal DNF / KNF / NAND / NOR sits under the parts list, out
  of sight unless you scroll the palette. Put it on the empty canvas as the first offer, and in a
  toolbar above the canvas.
- [ ] **must, S: the drawn circuit is not fitted.** G3 is cut off at the bottom, under the
  "Drawn from the minimal DNF…" note that covers the canvas. Fit the circuit (Home) after
  drawing, and move the note out of the canvas.
- [ ] **should, S: colours clash.** The critical path (yellow) colours B's switch, G1, G4 and f as
  if they were on or selected, at row 0 where everything is 0. Use another colour or a dashed
  outline for the path, and show it only on request.
- [ ] **should, S: the panel layout.** The stats box overlaps the canvas at the top right. The
  inspector on the right takes about 440 px for a one-line hint until a part is chosen. The
  editor pushes the table, KV diagram and results about 950 px down. Proposal: the inspector as a
  popover on the chosen part, the stats in a bar under the canvas, the editor as its own tab next
  to the results instead of above them.
- [ ] **could, M: wiring readability.** Many crossings in the drawn circuit, with junction dots
  and crossings close together (A over the B junction). Route the input rails with fewer
  crossings, or put each input's rail with its NOT next to it.

## Order

1. The **must** items, mode by mode: Logic and its editor first (biggest gains for the least
   work), then Geometry, then Base-N.
2. Walk through all four again in the app, then the **should** items.
3. Release (plans.md §2's manual checks, RELEASING.md).
