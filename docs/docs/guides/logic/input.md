---
sidebar_position: 2
title: Input
---

# Input

An output gets its function in one of three places: a **function line**, the **truth table**, or
the **circuit** (see [Circuit editor](editor.md)). Whichever you edit last is that output's
source, and the others are worked out from it.

## Function lines

Each line is one output: an expression or a minterm list. **+ Function** adds a line, the cross
beside a line removes its output. Errors show under the line, naming the symbol: *\sin is not a
logic operator*, *2 is not a logic value: only 0 and 1 are.*

| Line | Output | Inputs |
|---|---|---|
| `f = A \land B \lor C` | $f$ | $A$, $B$, $C$ |
| `f(C, B, A) = A \lor \overline{B}` | $f$ | $C$, $B$, $A$, in this order |
| `A \oplus B` | the next free name of $f$, $g$, $h$, … | $A$, $B$ |
| `y = \sum m(3, 5, 6, 7)` | $y$ | $A$, $B$, $C$ |

### Expressions

| Meaning | Typed as |
|---|---|
| NOT | `\overline{A}`, `\bar{A}`, `\neg A`, `\lnot A`, `!A`, `A'`, `not A` |
| AND | `A \land B`, `A \wedge B`, `A \cdot B`, `A * B`, `A \& B`, `A and B`, or `AB` |
| OR | `A \lor B`, `A \vee B`, `A + B`, `A \mid B`, `A or B` |
| XOR | `A \oplus B`, `A \veebar B`, `A xor B` |
| XNOR | `A \odot B`, `A \leftrightarrow B`, `A \Leftrightarrow B`, `A \equiv B`, `A xnor B` |
| NAND | `A \uparrow B`, `A \barwedge B`, `A nand B` |
| NOR | `A \downarrow B`, `A nor B` |
| Constants | `0`, `1`, `\top`, `\bot` |

- **Precedence**, highest first: NOT; AND (also `AB`); NAND; XOR and XNOR; OR; NOR. So
  `A \lor B \land C` is $A \lor (B \land C)$.
- NAND and NOR are not associative: `A \uparrow B \uparrow C` is an error asking for brackets.
- **Variables** are a letter with an optional index: `A`, `x_1`, `Y_{12}`. `AB` is two variables,
  as in every textbook. A longer name needs `\mathrm{}`: `\mathrm{Start}`. The words `and`,
  `or`, `not`, … are operators, not variables.
- Without `f(…)`, the inputs are in the order the table or the circuit gives them; new ones are
  added alphabetically, indices by number (`x_2` before `x_{10}`).

### Minterm lists

| Line | Meaning |
|---|---|
| `f = \sum m(1, 3, 7)` | 1 in the rows 1, 3, 7, 0 in the others |
| `f = \sum m(1, 3, 7) + d(5)` | as above, row 5 a don't care (–) |
| `f = \prod M(0, 2, 4, 5, 6)` | 0 in the rows 0, 2, 4, 5, 6, 1 in the others |
| `f = \prod M(0, 2) \cdot d(5)` | the product of maxterms with a don't care |

`\sum_m`, `\Sigma m`, `m(…)` and `\sum(…)` are read as `\sum m(…)`. `f(A, B, C) = …` sets the
variables; without them, a list has as many variables as its highest index needs (at least the
scene's inputs), named $A$, $B$, $C$, … . An index twice, or beyond the last row, is an error:
*9 is too large for 3 variables: the rows go from 0 to 7.*

The first variable is the most significant bit of the row number: for $f(A, B, C)$, row 6 is
$A = 1$, $B = 1$, $C = 0$.

## Truth table

The table has the row number $i$ in a grey first column, the inputs, a line, then one column per
output. The rows are always all $2^n$, in binary order. With no outputs yet, **+ Truth table**
starts one.

- Click an output cell, or press <kbd>Space</kbd>, to cycle it 0 → 1 → – → 0. The keys
  <kbd>0</kbd>, <kbd>1</kbd> and <kbd>-</kbd> set it and move down; the arrow keys move.
- **+** beside the inputs adds an input, and the table doubles; the cross on an input removes it
  and the table halves (it asks first when an output's table depends on it). An input a
  function line uses cannot be removed there.
- **+** beside the outputs adds an output; click a header to rename it.
- A new output column is empty, and each empty cell is an error (*Row 5 of f has no value*).
  **Fill the rest with 0** makes the empty cells of that output 0.

Editing a cell of an output that came from a function line or the circuit makes the table its
source: the line then shows *from the table*, and typing a function there and pressing
<kbd>Enter</kbd> takes it back.

The cells of the KV diagram can be edited the same way: a click or the keys <kbd>0</kbd>,
<kbd>1</kbd>, <kbd>-</kbd>.

## Don't cares

A don't care (–) is a row whose value does not matter, such as the codes 10 to 15 of a BCD
digit. The minimisation may use it as a 1 or a 0, whichever gives the smaller form. The
canonical forms leave it out and list it as $d(\ldots)$.

## What is refused

- More than 8 variables: *A function has at most 8 variables, not 9.*
- A variable named like its output (`f = f \land A`), or not among the declared inputs
  (`f(A, B) = A \land C`).
- A missing operand (`A \land`), an unclosed bracket, a number other than 0 and 1.
- A table cell without a value.

## See also

- [Results](results.md)
- [Circuit editor](editor.md)
- [Syntax](../../reference/syntax.md#logic)
