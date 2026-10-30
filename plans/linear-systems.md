# Linear systems

Three ideas around systems of linear equations. Today (`crates/quantus-core/src/eval/system.rs`,
`linear.rs`) the unknowns are the free variables **without** a subscript, sorted alphabetically,
and the working is a plain list of row operations from `linear::eliminate`.

## Implementation plan

Five phases, each one a commit (or a few) on `dev` that leaves `cargo test --workspace`,
`cargo clippy --workspace --all-targets` and `npm run typecheck` green. Sections 1–3 below are the
specification; this list says in which order to build them and where.

### Phase 1: order of the unknowns and indexed unknowns (section 1)

- [x] `crates/quantus-core/src/eval/system.rs`: a `variable_order` key (`x, y, z`, then `a … w`,
  subscripts numerically, Greek and multi-letter names last) replaces `unknowns.sort()`.
- [x] `eval/solver.rs`, `collect_unknowns`: take a flag for systems; with it, a `Node::Variable` with an
  index counts as an unknown if it has no value. Single equations pass the flag off.
- [x] Tests in `system.rs` and golden cases: `x_1 + x_2 = 3`, mixed `x, a, b`, `x_10` vs `x_2`, the
  physics case `v_1 = s_1/t_1` with `s_1 = 4 m`.
- [x] CHANGELOG entry.

### Phase 2: Gauss steps (section 3)

- [x] Setting `elimination` (`Gauss` default, `GaussJordan`) in `crates/quantus-core/src/settings.rs`
  next to `step_detail`; type in `desktop/src/api.ts`, drop-down in `SettingsPanel.tsx`, strings in
  `en.ts` and `de.ts`.
- [x] `linear.rs`: `gauss(matrix, unknowns) -> Echelon` with the log; new `RowOperation` variants
  `Combine { target, a, source, b }` and `Scale { row, divisor }`, and a reason enum.
  `eliminate` keeps its behaviour for the circuit solver and `solve_square`.
- [x] `system.rs`: back-substitution from the echelon form, the outcome (unique, `0 = c`, free
  unknowns), Greek parameters `λ, μ, ν, …` skipping letters in use. A test that `gauss` and
  `eliminate` agree on every system in the tests. *Check:* how fraction-free steps look with `π`.
  Result: `π·II − I`, no gcd, as intended; but the matrix showed `π` as a 50-digit decimal
  unless *exact roots* is on, so the step block writes entries exactly.
- [x] `eval/calculator.rs`: rewrite the step block (`augmented`, `row_operation`) for both methods:
  row labels with marks (`II'`), the annotation column, the coloured pivot and changed row, the
  levels from the table in section 3 (entry-by-entry arithmetic at Detailed, the check at
  Maximal). *Check:* a KaTeX row background inside `array`, in both themes.
  Result: KaTeX 0.16 has no `\rowcolor`/`\cellcolor` and `\colorbox` is text mode only, so the
  changed row is coloured text (`dodgerblue`, the pivot `orange`), readable in both themes. The
  builder lives in `eval/system_steps.rs`.
- [x] `design/step-detail.md`: a *Linear systems* section with a worked 3×3 example per level.
  Golden cases for unique, inconsistent and infinite, with both settings.
- [x] CHANGELOG entry. Visible now in Algebra with `;`.

### Phase 3: LGS engine and API (section 2, backend)

- [x] `Mode::Linear` in `settings.rs` and `'linear'` in `api.ts`; settings `unknown_names`
  (`Indexed` default, `Letters`) and `linear_complex` (default off).
- [x] `crates/quantus-core/src/api.rs`: `linear_system(settings, LinearSystemRequest)`. The request is
  `Equations(latex)` or `Matrix { unknowns, rows }` (JSON input is parsed into `Matrix` by the
  frontend, the engine validates it). It evaluates without session variables; with
  `linear_complex` off, `i` is an unknown. Empty cells are 0. Cells with units are an error.
- [x] The response: unknowns, matrix cells as LaTeX, the system as equations LaTeX (for the tab
  switch, named by `unknown_names`), steps, the exact result, the copy block (`align*`).
- [x] Tauri command `solve_linear_system` in `desktop/src-tauri/src/commands.rs`, wrapper in `api.ts`.
- [x] Tests: `crates/quantus-core/tests/linear_systems.rs` (equations, matrix, both methods, with and
  without `i`, m ≠ n, the round trip equations → matrix → equations) and LGS cases for
  `run_cases`.
  Result: a `linear-systems` category of 2000 cases (`gens/linear_systems.py`, generated alone
  with `--only`), checked by `run_cases` through `api::linear_system`. It found complex factors
  of parameters written without brackets and Gaussian rationals written as decimals; both fixed.
  Letters as unknown names skip `e` and `i`. Values without an exact form (`\frac{5 + \pi}{1 +
  \sqrt{2}}`) still fall back to a decimal.

### Phase 4: LGS page (section 2, frontend)

- [x] *Check first:* a spike of MathLive editing inside `cases` (Enter → `addRowAfter`, Backspace →
  `removeRow`, paste, undo). It decides between the single field and one field per line.
  Result (MathLive 0.110, headless Chromium): Enter → `addRowAfter`, Up/Down and undo work, but
  Home/End leave the block and the next Backspace deletes the whole system; Backspace at the
  start of a row does nothing, and `removeRow` removes the row the cursor offset belongs to,
  which is the next one after moving right from an empty row; pasting a `cases` block nests it.
  So one field per line, drawn as a single block.
- [x] Mode switch: `modeChoices` gets "Linear systems" / "LGS", shown as "… + $i$" when
  `linear_complex` is on. `App.tsx` renders the new view and holds its state, so the input and
  the last tab survive a mode switch (like `circuit`).
- [x] `components/LinearSystemView.tsx`, built like `AnalysisView`: the tabs, the `i` toggle, `Steps`,
  the error in grey for incomplete input, copy.
- [x] `components/SystemField.tsx`: the multiline field from the spike.
- [x] `components/MatrixGrid.tsx`: n and m (m defaults to n, both ≤ 10), the cell `MathField`s,
  Tab/Shift+Tab/Enter, empty = 0, resizing keeps entries, column headers by `unknown_names`.
- [ ] JSON input: paste and a load button; up to 10 × 10 it fills the grid, above that a read-only
  preview of the matrix.
- [ ] Tab switch converts through the API response. All strings in `en.ts` and `de.ts`.
- [ ] Check in the running app (`npm run dev`).

### Phase 5: docs

- [ ] A user guide for the mode in `docs/docs/guides/`, and the settings in the reference.
- [ ] Update the flowcharts in `docs/docs/contributing/solving.md` for the LGS path.
- [ ] CHANGELOG entry for the mode; `npm run licenses` only if a dependency was added.

## 1. Indexed variables and the order of the unknowns

**Idea.** `x_1, x_2, x_3` are valid unknowns, and the unknowns are ordered `x, y, z, a, b, c, … w`
instead of `a … z`.

**Check.** Sensible; it matches how textbooks write systems. Details to decide:
- Order key: `x, y, z` first, then `a … w` without `x, y, z`. Letters with a subscript sort by
  letter, then by index numerically (`x_2` before `x_10`). Greek letters and multi-letter names go
  last, alphabetically.
- Subscripts are currently excluded on purpose, because `v_1`, `t_1` in physics mode are
  quantities, not unknowns. Rule: a subscripted symbol is an unknown only in a *system*
  (two or more equations), and only if it has no value assigned. Single equations keep today's
  behaviour. Needs a test for the physics case (`v_1 = s_1/t_1` plus `s_1 = 4 m`).
- The same order is used for the columns of the matrix, the displayed solution and the free
  parameters of an infinite solution set.

**Plan.** Replace `unknowns.sort()` in `system.rs` by a `variable_order` key function, relax the
subscript filter in `collect_unknowns` for systems, add golden cases (`x_1+x_2=3`, mixed `x, a, b`,
`x_10` vs `x_2`).

## 2. Input page for linear systems

**Idea.** One line with `;` between the equations stays as it is, but it gets messy with three or
more unknowns. A new mode **Linear systems** (German: **LGS**; next to Analysis in the mode switch)
gets its own page with two tabs, *Equations* and *Matrix*. Both lead to the same solution and the
same steps.

### Equations tab

- One MathLive field that shows the system as rendered maths, one equation per line (a `cases`
  block, so the parser reads it unchanged).
- **Enter** adds a new line below the cursor (MathLive `addRowAfter`) and solves what is there.
  An incomplete system that fails while typing is fine: the error is shown in grey under the field,
  not as a red error row, and goes away with the next valid state.
- **Backspace** on an empty line removes it (`removeRow`); Up/Down move between lines.
- The field starts as an empty one-line block, so the user never types `\begin{cases}`.
- Risk: MathLive's editing inside environments. Spike it first (Enter, Backspace, pasting a line,
  undo). If it is not good enough, fall back to one field per line that looks like a single block.

### Matrix tab

- Two number inputs: **unknowns** n and **equations** m. If m is not set it equals n. Both are at
  most 10; anything larger goes through the equations tab or JSON input (below).
- Below them a grid of m × (n+1) cells, with a vertical bar before the right-hand side and the
  unknowns as column headers.
- **Unknown names** are a setting: `x_1 … x_n` (default), or letters in the order of section 1
  (`x, y, z, a, b, c, … w`), continuing with `x_1, x_2, …` once the letters run out after `w`.
  The names also apply to matrix → equations and to the solution.
- Each cell is a small MathLive field, so `\frac{1}{2}`, `\pi` and `\sqrt{2}` are allowed.
- **Tab** / **Shift+Tab** go through the cells row by row; Tab in the last cell does not leave the
  grid. **Enter** solves. An empty cell counts as 0, so zeros never have to be typed.
- Changing n or m keeps the entries that still fit.
- **JSON input** for any size: paste or load a file with the rows of `[A | b]`, each entry a number
  or a LaTeX string, optionally with names:
  `{"unknowns": ["x_1", "x_2"], "rows": [[2, 1, 5], [1, "\\frac{1}{2}", 1]]}`, or just the `rows`
  array. Up to 10 × 10 it fills the grid; above that the grid is replaced by a read-only matrix
  preview and the system is solved from the JSON.

### Shared

- Switching tabs converts the input: equations → matrix uses the matrix the engine reads
  (`SystemSolution.matrix` and `unknowns`), matrix → equations writes `2x - y = 3` per row (zero
  coefficients left out, 1 and −1 without the number).
- The mode does not use session variables at all: `k_1` is an unknown here, not a stored value.
  Systems that need stored values stay in Algebra with `;`.
- Coefficients are dimensionless; units are an error, as today.
- Copy works as in Algebra: the result and steps as an `align*` block.
- **Imaginary unit** is a toggle: on, `i` is the imaginary unit and coefficients may be complex
  (as in Imaginary mode); off, `i` is an ordinary unknown. While it is on, the mode name in the
  mode switch reads "LGS + $i$" ("Linear systems + $i$").
- The solution is shown exact only (`x_1 = \frac{3}{2}`, `\sqrt{2}`), without a decimal.
- m ≠ n is allowed in both tabs (over- and underdetermined); it goes through the same
  elimination and ends in a contradiction or free parameters.
- Both tabs keep their input when switching to another mode and back (like the circuit editor),
  and the page opens on the tab used last.

**Plan.**
- Engine: `api::linear_system(settings, LinearSystemRequest)` where the request is either
  `Equations(latex)` or `Matrix { unknowns, rows: Vec<Vec<String>> }`. The response holds the
  unknowns, the matrix as LaTeX cells, the system as equations LaTeX, the steps and the result.
  The matrix path parses and evaluates each cell to a `Value` and then uses the same elimination
  and step code as the equation path; nothing is turned into LaTeX equations first.
- `Mode::Linear` in the settings and `'linear'` in `desktop/src/api.ts`, a Tauri command
  `solve_linear_system` in `desktop/src-tauri/src/commands.rs`.
- Frontend: `components/LinearSystemView.tsx` built like `AnalysisView` (tabs, `Steps`, `ErrorRow`),
  `SystemField` (the multiline field) and `MatrixGrid`. Strings go into `en.ts` and `de.ts`.
- Settings: `unknown_names` (`Indexed` default, `Letters`), `linear_complex` (the `i` toggle,
  default off) and `elimination` (see section 3).
- Tests: golden cases for the LGS API (equations, matrix and JSON input, both elimination
  methods, with and without `i`), a test that Gauss and Gauss–Jordan give the same solution, and
  LGS cases in the generated test set (`run_cases`).
- Docs: a user page for the mode, and a CHANGELOG entry.

## 3. Step-by-step Gauss as taught at school

**Idea.** The steps follow the school method: bring the system to triangle form without fractions,
then solve from the bottom up. Each step shows what was done and why, with the pivot and the changed
row marked.

A setting `elimination` chooses the method: **Gauss** (default, below) or **Gauss–Jordan** (today's
steps: every pivot divided to 1, columns cleared above and below, no back-substitution). It applies
to the LGS mode and to systems typed with `;` in Algebra. Both methods get the marks, colours and
levels below.

**Method (Gauss).**
- Pivot: the first non-zero entry of the column, swapping its row up if needed.
- Eliminate below the pivot only, without fractions: `II := a·II − b·I` with `a`, `b` the pivot and
  the entry divided by their gcd (`II := 2·II − 3·I`). Whenever a row's entries share a factor, it
  is divided out (`II : 3`), always, so the numbers stay small. Symbolic entries (`π`) skip the gcd.
- Then back-substitution: the last row gives `x_3 = …`, which is put into the row above, and so on,
  written as equations rather than matrices.
- `Inconsistent`: show the row `0 = c` and say that it contradicts. `Infinite`: name the free
  unknowns, set them to Greek parameters (`λ`, then `μ`, `ν`, …), and solve for the others. A
  letter the system already uses as an unknown is skipped.
- New `linear::gauss(matrix, unknowns) -> Echelon` with its own `log`. `eliminate` stays
  Gauss–Jordan for the circuit solver and `solve_square`. The result is checked against
  `eliminate` in tests.

**How a step looks.**
- A row that was changed gets a mark, one per change: `II` → `II'` → `II''`. The operation names
  the rows as they are at that moment (`III' - 2\cdot\text{II}'`). A swap exchanges the labels
  with the rows.
- The operation is written to the right of the row it changes, as on paper:
  `\left(\begin{array}{cc|c|l} 2 & 1 & 5 & \\ 4 & -1 & 1 & \text{II} - 2\cdot\text{I} \end{array}\right)`.
- The pivot is coloured (`\textcolor`) and the changed row in the next matrix has a background
  (`\colorbox` or a cell colour), in both themes.
- `+3·II` on row I is written out entry by entry: `I_1 + 3\cdot(II_1)`, `I_2 + 3\cdot(II_2)`, …
  with the numbers: `2 + 3\cdot(-1) = -1`.

**Levels** (from [step-detail.md](../design/step-detail.md), no separate switch):

| Level | Shown |
|---|---|
| Brief | Start matrix, triangle form, solution. |
| Standard | Every matrix with the operations next to the rows, pivot coloured, changed row marked, one-line reason ("eliminate x from II"); back-substitution as equations. |
| Detailed | Also the entry-by-entry arithmetic of each changed row (`2 + 3·(−1) = −1`), and the gcd division. |
| Maximal | Also why this pivot (the swap when the entry is zero), and the check of the solution in each original equation. |

**Plan.** `RowOperation` gets the school variants (`Combine { target, a, source, b }`,
`Scale { row, divisor }`, `Swap`) and a reason enum, so the text is built in the calculator/text
layer in both languages and not in `linear.rs`. The step builder (`row_operation` and `augmented`
in `eval/calculator.rs`) renders the annotation column and the colours. Add a *Linear systems*
section to `step-detail.md` with one worked 3×3 example per level, and golden cases for unique,
inconsistent and infinite.
