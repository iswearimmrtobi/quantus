# Linear systems

Three ideas around systems of linear equations. Today (`crates/quantus-core/src/eval/system.rs`,
`linear.rs`) the unknowns are the free variables **without** a subscript, sorted alphabetically,
and the working is a plain list of row operations from `linear::eliminate`.

## 1. Indexed variables and the order of the unknowns

**Idea.** `x_1, x_2, x_3` are valid unknowns, and the unknowns are ordered `x, y, z, a, b, c, … w`
instead of `a … z`.

**Check.** Sensible; it matches how textbooks write systems. Details to decide:
- Order key: `x, y, z` first, then `a … w` without `x, y, z`. Letters with a subscript sort by
  letter, then by index numerically (`x_2` before `x_10`). Greek letters and multi-letter names go
  last, alphabetically.
- Subscripts are currently excluded on purpose, because `v_1`, `t_1` in physics mode are
  quantities, not unknowns. Rule proposal: a subscripted symbol is an unknown only in a *system*
  (two or more equations), and only if it has no value assigned. Single equations keep today's
  behaviour. Needs a test for the physics case (`v_1 = s_1/t_1` plus `s_1 = 4 m`).
- The same order is used for the columns of the matrix, the displayed solution and the free
  parameters of an infinite solution set.

**Plan.** Replace `unknowns.sort()` in `system.rs` by a `variable_order` key function, relax the
subscript filter in `collect_unknowns` for systems, add golden cases (`x_1+x_2=3`, mixed `x, a, b`,
`x_10` vs `x_2`).

## 2. Better input for linear equations

**Idea.** Entering a system as separate lines of LaTeX is clumsy.

**Check.** Sensible, but "better" is not yet defined. Candidates, cheapest first:
1. Several equations in one input, separated by `;` or a line break, without needing `\begin{cases}`.
2. A matrix input mode: dimension picker, grid of cells (MathLive has a matrix environment) for
   `[A | b]`, solved with the same `linear` code.
3. Paste of plain text (`2x + 3y = 5` per line, also `2x+3y-5=0`), converted to LaTeX.

**Plan.** Start with 1 and 3 (parser only, no new UI), then decide on 2. Ask the user which of the
three hurts most before building 2.

## 3. Step-by-step Gauss with more detailed explanations

**Idea.** Explain every row operation in words, not only list it.

**Check.** Sensible and fits [step-detail.md](../design/step-detail.md), which already defines
detail levels; this should plug into them instead of being a separate switch.

**What a step should say.**
- Why this pivot (first non-zero entry of the column; why a swap was needed).
- The operation as `R2 := R2 - 3·R1` plus the one-line reason ("eliminate x from row 2").
- The matrix after the step, with the pivot and the changed row marked.
- The ending: back-substitution reading of the reduced form, and for `Inconsistent` / `Infinite`
  which row or free column caused it.
- Levels: Brief = today's list; Standard = + reason sentence; Detailed = + the arithmetic of each
  entry that changed (`5 - 3·2 = -1`).

**Plan.** Extend `RowOperation` with a reason enum (so the text is built in the `text` layer, in
both languages, not in `linear.rs`), render through the existing step machinery, add a section to
`step-detail.md` and the German strings.

## Order

2.1 (ordering) first, it is small and unblocks 3-variable names. Then 3 (detailed steps), then the
input work.
