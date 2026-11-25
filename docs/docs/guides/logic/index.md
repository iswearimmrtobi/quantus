---
sidebar_position: 1
title: Logic
---

# Logic

**Logic** mode (German **Logik**) is for combinational digital logic. Type a function, fill in
a truth table or draw a circuit of gates. The page then shows the truth table, the canonical and
minimal forms, the KV diagram, the function as AND–OR, OR–AND, NAND and NOR gates (also from
2-input gates only) and a circuit drawn from any of them, each with its working.

**Example.** The majority of three: *the output is 1 when at least two of A, B, C are 1.* Type
`f = \sum m(3, 5, 6, 7)`, or fill in the column of $f$ in the table. The KV diagram shows three
blocks, the minimal form is $f = B \land C \lor A \land C \lor A \land B$, and the **Gates**
card shows it as 4 NANDs over 2 levels, or 6 from 2-input NANDs over 4 levels. **Open in
editor** puts that circuit into the editor. Click its input switches and the matching row of the table and cell of the KV
diagram light up.

## The page

Switch to **Logic** in the top bar. From top to bottom the page has:

1. **Load** and **Save** for the scene,
2. the **function lines**, one per output,
3. a switch **Table & KV | Circuit**, with a tab per output beside it when there are several,
4. under it the **truth table** and the **KV diagram** side by side, for one output at a time, or
   the **circuit** editor,
5. the **results** of that output, each with its steps and **Copy**, under either.

The function lines, the table and the circuit are three ways to give an output its function.
Each output has one **source**, the view it was edited in last; the other views follow from it.
A small marker beside each function line names its source: *expression*, *minterms*, *table* or
*circuit*. Editing a view makes it the source of the outputs it touches. See
[Input](input.md) for the details.

The scene is kept when you switch to another mode and back, and saved with the session.

## Limits

- Up to **8 inputs** (256 rows); any number of outputs.
- KV diagrams for 2 to 6 inputs. With 7 or 8, the Quine–McCluskey tables take the diagram's
  place.
- Combinational logic only: a circuit with a feedback loop is refused (*sequential circuits are
  not supported yet*). Flip-flops, timing diagrams and hazards are not part of the mode.
- Every output is minimised on its own, and with several outputs also all together, with shared
  terms (see [Results](results.md#all-outputs-together)).
- Minimisation is exact (Quine–McCluskey and Petrick), so it never gives a form that might not
  be minimal. For random tables of 7 or 8 inputs the comparison can get too large; the mode then
  says *Too many solutions to compare* instead of guessing.

## Files

**Save** writes the scene (the function lines, the table, the circuit and its layout) as a
`.quantus-logic.json` file, **Load** reads one back and replaces the scene. A file that cannot
be read is reported, and the scene stays as it was. The file is
`{ "version": 1, "scene": … }`, the scene as the [engine API](../../contributing/api.md) takes it.

## Write-up

<kbd>Ctrl</kbd>+<kbd>G</kbd> writes the scene up as *Given*, *Wanted*, *Solution*: each output's
function under *Given* (as typed, traced from the circuit, or its minterms), and per output its
KV diagram, the chosen minimal DNF as the result (*one of n* when there are several), the minimal
KNF and the NAND and NOR forms (and the 2-input ones where they differ) with their gates and levels. An output with an error is left out.
See [Given, wanted, solution](../physics/writeup.md).

## Settings that matter here

The **Logic** section of the settings:

- **Logic notation**: results, KV labels and Copy in $A \land \lnot B \lor C$ or in
  $A \cdot \overline{B} + C$. The input reads both.
- **Gate symbols**: IEC 60617 rectangles (`&`, `≥1`, `=1`, `1`) or the ANSI distinctive shapes.
- **Show 0s in the truth table and KV diagram**: off leaves the 0 cells empty, as many
  textbooks draw them.

**Step detail** decides how much of each working is shown, and **Language** the texts of the
steps. See [Settings](../../reference/settings.md#logic).

## See also

- [Input](input.md): function lines, minterm lists and the truth table
- [Operators](operators.md): each operator's meaning, rules, spellings and precedence
- [Results](results.md): the forms, the KV diagram and its blocks, the Gates card, Copy
- [Circuit editor](editor.md): drawing gates, simulating, drawing a circuit from a form, delay
- [Syntax](../../reference/syntax.md#logic): every operator spelling
- [Base-N](../base-n.md): bitwise logic on whole numbers
