# Physics mode and circuits

Ideas for physics mode, ending in a Kirchhoff calculator that reuses the electronics mode
(implemented but hidden, see [plans.md](plans.md) item 3, spec in
[electronics-module.md](../design/electronics-module.md)).

## 1. Formula collection in physics mode

**Idea.** A browsable list of formulas (`v = s/t`, `F = m·a`, `U = R·I`, …) that can be inserted
into the input and solved for any variable.

**Check.** Sensible, and cheap to start: the engine already rearranges equations and handles units.
Risks: scope creep (a formula collection has no end) and correctness (every entry is content that
must be right).

**Plan.**
- Data, not code: one file (`crates/quantus-core/src/physics/formulas.*` or JSON) with id, name
  (en/de), LaTeX, variable meanings with default units, topic. A test parses every formula and
  checks dimensional consistency with the unit system, so a typo cannot ship.
- Start small: mechanics, electricity, thermodynamics, optics, ~40 formulas.
- UI: searchable side panel; click inserts the formula; variables with a value in the session are
  substituted, the rest stays unknown.
- Later: a user-defined formula list.

## 2. Ready-to-copy step-by-step solving

**Idea.** The working of a physics task (formula, rearranged, numbers put in, result with unit) can
be copied as a finished block.

**Check.** Sensible. Copying already puts an `align*` block on the clipboard, so this is mostly
making the *physics* steps complete and tidy: given values, formula, rearranging, substitution
with units, result. The calculator part of [step-detail.md](../design/step-detail.md) ("numbers and
units written apart, rearranged equation") is exactly the missing piece.

**Plan.** Finish those rows first. Then copy formats: LaTeX `align*` (today), plain text, and
optionally Markdown. Check that the copied block compiles standalone in a LaTeX document.

## 3. Kirchhoff calculator

**Idea.** Enter a circuit (or its Kirchhoff equations); get node and mesh equations, the linear
system, the Gauss working, and all currents and voltages. It combines three existing parts:
physics (units), Gauss ([linear-systems.md](linear-systems.md)) and the electronics solver.

**Check.** Sensible, and the best fit of all the ideas: the circuit solver already builds and
solves `A·x = z` with exact arithmetic (`electro/solve.rs`, `linear.rs`). What is missing is the
*explained* route, as a student does it: choose a tree, write the junction rule per node, the loop
rule per mesh, then solve. Note the name: **Kirchhoff**.

**Plan.**
1. Generate the Kirchhoff equations from a `Circuit` as text (junction and loop rules with the
   unknown currents `I_1, I_2, …`). Needs the indexed unknowns of linear-systems.md §1.
2. Feed them into the system solver, so the steps are the same Gauss steps.
3. Show it next to the circuit editor; result values written back to the elements.
4. Property test against the existing MNA result: both must agree exactly.

**Depends on.** Electronics mode being un-hidden (or at least its solver exposed), indexed unknowns,
detailed Gauss steps. Do it last.

## Order

Formula collection and copyable steps are independent and can go first; the Kirchhoff calculator
comes after the linear-systems work.
