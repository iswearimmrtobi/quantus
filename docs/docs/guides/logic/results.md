---
sidebar_position: 3
title: Results
---

# Results

Under the table and the KV diagram are the results of one output; with several outputs, the
tabs beside **Table & KV | Circuit** choose it. They sit in three cards: the expression beside the canonical
forms, the minimal DNF beside the minimal KNF (one under the other in a narrow window), and the
Gates card. Each result has its steps at the level of **Step detail**
(see [Step detail in the Logic mode](#step-detail)) and a **Copy** button that copies the result
and its steps as a LaTeX `align*` block.

| Result | What it is |
|---|---|
| **Expression** | the line as typed, with the truth table it gives |
| **Expression of the circuit** | for an output from the circuit: the circuit traced from the lamp back to the switches, gate by gate, unsimplified |
| **Canonical DNF and KNF** | $\sum m(\ldots)$ and the OR of the minterms; $\prod M(\ldots)$ and the AND of the maxterms; don't cares as $d(\ldots)$ |
| **Minimal DNF** | the fewest terms, then the fewest literals; the KV path, with **Quine–McCluskey** as a second working |
| **Minimal KNF** | the same over the 0s, each term turned into a sum by De Morgan |
| **Gates** | the function as AND–OR, OR–AND, NAND, NAND 2-in, NOR or NOR 2-in, with gates, gate inputs and levels, and a picture of the circuit (see [Gates](#gates)) |

A function that is always 0 or always 1 says so (*f is always 1*) and has no KV blocks.

## Minimal forms

Minimisation is exact: the prime implicants by Quine–McCluskey, the cover by the prime implicant
chart (essential prime implicants, then row and column dominance) and Petrick's method for the
rest. **All** minimal solutions are found. When there are several, the result says so
(*2 minimal solutions:*) and lets you pick one; the one picked drives the KV blocks, the Gates
card, the write-up and a circuit drawn from the minimal form.

$\sum m(0, 1, 2, 5, 6, 7)$ has two minimal DNFs, each with 3 terms and 6 literals:

$$f = \lnot A \land \lnot B \lor B \land \lnot C \lor A \land C \qquad f = \lnot A \land \lnot C \lor \lnot B \land C \lor A \land B$$

The cost of each form is shown beside it (*terms: 3 · literals: 6*). Hovering a term highlights
its block in the KV diagram, and hovering a block highlights its term.

## All outputs together

With two or more outputs, the card **All outputs, minimised together** minimises them jointly:
one set of terms for all of them, where a term that serves several outputs is built only once.
Its cost is the number of distinct terms, then their literals, then the inputs of the outputs'
ORs (ANDs for the KNF); beside it is the cost of each output minimised alone, identical terms
counted once.

$f = \sum m(1, 3, 7)$ and $g = \sum m(2, 6, 7)$ each need two products alone, four in all.
Together they need three, $A \land B \land C$ serving both:

$$f = \lnot A \land C \lor A \land B \land C \qquad g = B \land \lnot C \lor A \land B \land C$$

A switch shows the joint DNF or the joint KNF (the same over the 0s). A shared term is marked
over and under, keeps its colour in every output, and hovering it marks it everywhere; its
tooltip names the outputs that share it. With several joint minimal solutions you can pick one,
like the minimal forms of one output. The steps show the tagged Quine–McCluskey tables (each term
with the outputs it is an implicant of), the chart with a column per 1 of each output and
Petrick's method. An output whose source has an error is left out, and named.

## The KV diagram

The diagram stands beside the truth table, for 2 to 6 inputs. The first half of the inputs run
along the columns, the rest along the rows, each in Gray code (00, 01, 11, 10). 5 inputs give two
4-input maps side by side, one for each value of the first input; 6 inputs give four maps. Each
variable has a bar beside the rows or columns where it is 1, with its name, as in German
textbooks; hovering a cell shows its row number and its code. The *i* button writes the row
number into each cell's corner.

- **1s · DNF** shows the blocks of the chosen minimal DNF over the 1s, **0s · KNF** those of the
  minimal KNF over the 0s. A block that wraps over an edge is drawn as two or four open pieces.
  The form itself stands under the map, each term underlined in its block's colour.
- A click on a cell cycles it 0 → 1 → – like the table, and makes the table the output's source.
- With a circuit, <kbd>Ctrl</kbd>+click sets the circuit's switches to that cell.

### Draw my blocks

**Draw my blocks** lets you draw your own blocks and have them checked. Drag a rectangle over the
cells; drag past an edge to wrap round, or into the next map to span both. Each block gets an
answer on it:

- *right: a prime implicant that is needed*;
- not a power of two in size, or not the cells of one term;
- covering a 0 (or, for the KNF, a 1);
- *could grow*, with the larger term;
- *not needed*: each of its 1s is in another block.

Under the diagram: the 1s in no block, then the expression your blocks give, and whether it is a
minimal form (*Right, and a minimal form*) or not, with its cost against the minimum. With
**0s · KNF** you draw blocks of 0s and get a KNF. **Clear** removes your blocks.

## Gates

The **Gates** card shows the function built from gates. A switch picks the form:

| Form | What it is |
|---|---|
| **AND–OR** | the chosen minimal DNF: ANDs of the literals into an OR |
| **OR–AND** | the chosen minimal KNF: ORs of the literals into an AND |
| **NAND**, **NOR** | NAND gates only, NOR gates only, of any number of inputs |
| **NAND 2-in**, **NOR 2-in** | the same from gates of 2 inputs only |

The 2-in buttons are there only when they differ: for a function of two variables every gate
already has 2 inputs, so there is one NAND and one NOR form. Under the formula and its cost is
the circuit as **Draw from** would draw it, read-only, simulated in the table's current row as
the editor is: a net at 1 bright, at 0 dim, the switches set and the lamps lit. **Open in editor** draws it in the
[circuit editor](editor.md) (replacing the circuit there) and scrolls to it. With several
outputs, such as a truth table with several output columns, the picture and **Open in editor**
take all of them in one circuit in the form chosen here, built from the
[joint minimal forms](#all-outputs-together) so that the outputs share their terms; the line under
the picture gives the whole circuit's gates, gate inputs and levels. (Drawn from the expression,
an output without one takes its minimal form.) If another output has an error, the picture
shows this output alone. Each output keeps its form, also in a saved session.

For a typed expression or a circuit, a second switch picks where the NAND and NOR forms come
from: **from the minimal form** or **from the expression** (rewritten gate by gate, see below).

A NAND or NOR form is shown **gate by gate** at first: each gate gets a signal of its own,
numbered from the inputs on, and the output is the last one. For the majority of three as NAND:

$$
\begin{aligned} G_1 &= \overline{A \land B} \\ G_2 &= \overline{A \land C} \\ G_3 &= \overline{B \land C} \\ G_4 &= \overline{G_1 \land G_2 \land G_3} \\ f &= G_4 \end{aligned}
$$

That is the circuit of the picture, and easier to read than the formula with its overlines
stacked three deep. A third switch, **gate by gate / as one formula**, shows the formula instead.

**NAND** starts from the chosen minimal DNF $f = p_1 \lor \dots \lor p_k$: double negation
and De Morgan turn it into $\overline{\overline{p_1} \land \dots \land \overline{p_k}}$, a NAND
of NANDs. Each negated variable comes from a NAND with both inputs tied, one per variable; a
term of one literal goes straight into the output NAND. In **NAND 2-in**, a NAND of 3 inputs
becomes a tree of 3 gates, one of 4 inputs 5 gates. **NOR** is the same from the minimal KNF.

For the majority of three, **NAND** has 4 gates, 9 gate inputs and 2 levels; **NAND 2-in** has
6 gates, 12 gate inputs and 4 levels.

**From the expression**, the NAND form rewrites each gate instead: each NOT
an inverter, each AND a NAND and an inverter, each OR a NAND of the inverted inputs, each XOR
four NANDs, and then cancels pairs of inverters. This can be smaller than the form from the
minimal DNF: $A \oplus B$ takes 5 NANDs from its minimal DNF but 4 gate by gate. Neither is
guaranteed to be the circuit with the fewest gates.

## Copy

- **Copy** on a result: the result and its steps as an `align*` block.
- **Copy** in the table's foot: the truth table as a LaTeX `tabular`, all outputs.
- **Copy** in the KV diagram's head: the diagram as a `tabular`, each bar a rule beside its
  columns (`\hrulefill`) or rows (`\vline`) with the variable's name outside it; 5 and 6 inputs
  as maps inside an outer `tabular`.

All three follow **Logic notation**, and leave the 0s empty when **Show 0s** is off.

## Step detail

| Working | Brief | Standard | Detailed | Maximal |
|---|---|---|---|---|
| Expression → table | the table | | a column per part of the expression | one row worked as an example |
| Circuit → expression | the traced expression | the gates from the output back | each gate's output as $G_1$, $G_2$, … | |
| Canonical forms | DNF and KNF | $\sum m$, $\prod M$, the don't cares | each minterm and maxterm from its row | |
| KV diagram | the minimal form | the diagram, the blocks and their terms | why each variable stays or goes in a block | the prime implicants not used |
| Quine–McCluskey | the minimal form | the prime implicants, the essential ones, the covers | the grouping tables with ✓, the chart, dominance | the pairs that did not combine, Petrick multiplied out |
| NAND and NOR | the form with its gates and levels | double negation and De Morgan | the inverters, the 2-input split, every gate with its level | |

**None** hides the steps. At 7 and 8 inputs the Quine–McCluskey tables replace the KV diagram in
the working.

## See also

- [Input](input.md)
- [Circuit editor](editor.md)
- [Working and output](../../basics/working-and-output.md)
