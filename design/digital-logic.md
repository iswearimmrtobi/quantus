# Logic mode (Logik)

A mode for combinational digital logic: draw gates and get the truth table, the minimal forms and
the KV diagram; or go the other way, from a truth table, a typed expression or a minterm list to
the minimal form and a drawn circuit, also in NAND-only or NOR-only form. Every result comes with
its working, and the mode checks a student's own KV blocks and minimal forms. It is the digital
counterpart of the electronics mode ([electronics-module.md](electronics-module.md)); the idea
it grew from is [feature-ideas.md § 16](../plans/feature-ideas.md#16-digital-logic-editor).

**Target example.** The majority of three: *the output is 1 when at least two of A, B, C are 1.*
The user fills in the truth table (or types `\sum m(3,5,6,7)`); the mode shows the KV diagram
with three blocks, the minimal form $f = AB \lor AC \lor BC$, the NAND-only circuit from 2-input
NANDs (6 gates, 4 gate delays) and draws it, editable. Clicking the input switches of the circuit
highlights the matching row of the table and cell of the KV diagram.

Code (planned): `crates/quantus-core/src/logic/`, re-exported in `api.rs`; in the desktop app
`LogicView.tsx`, `LogicEditor.tsx`, `LogicTable.tsx`, `KvMap.tsx` and `logic.ts`. Tests:
`crates/quantus-core/tests/logic.rs`.

## 1. Scope

**In the first version:**

- Combinational logic with up to **8 inputs** (256 rows) and any number of outputs.
- Four ways in, each leading to every result: a drawn circuit, a truth table, a typed
  expression, a minterm list (§5).
- Truth table, canonical DNF and KNF, minimal DNF and KNF (exact, all minimal solutions), KV
  diagrams for 2 to 6 variables, NAND-only and NOR-only forms, a circuit drawn from any form,
  gate count and propagation delay (§6).
- Checks: two functions equal (with a counterexample), the user's own KV blocks, the user's own
  minimal form (§6.8).
- The working at the four step levels (§8).
- Gate symbols IEC 60617 or ANSI, notation ∧ ∨ ¬ or · + overline, as settings (§9).
- Export as LaTeX, the circuit as SVG and PNG, the write-up, the scene as a JSON file (§10.5).

**Not in the first version** (§13): flip-flops and state machines, timing diagrams and hazards,
ready-made blocks (adders, multiplexers, decoders, 7-segment display) and own subcircuits, joint
minimisation of several outputs, more than 8 inputs, random exercises, the handheld, HDL export.

## 2. Core principles

1. **Nothing is guessed.** An unconnected gate input, an output with two drivers or a feedback
   loop is an error that names the gate and the pin; no pin is silently 0. A table row without
   a value is an error too, unless it is marked don't care (–).
2. **Exact and complete.** A truth table has all $2^n$ rows. Minimisation is provably minimal
   (Quine–McCluskey and Petrick, no heuristics), and when several minimal forms exist, all are
   listed.
3. **One function, many views.** Circuit, table, expression, minterm list and KV diagram are views
   of the same function. The view edited last is the source; the others follow (§5.1).
4. **Every result has a working**, at Brief, Standard, Detailed and Maximal, like every mode.
5. **The engine is stateless**; the shell keeps the scene. The core knows no positions: the shell
   sends a netlist, like the electronics mode sends nodes.

## 3. Glossary

The English names are those of the app; the German ones those of `locales/de.json`.

| English | German | Notes |
|---|---|---|
| Truth table | Wahrheitstabelle (Funktionstabelle) | rows in binary order, the first input the MSB |
| Input, output | Eingang, Ausgang | named variables, `A`, `x_1`, `f`, `Y_2` |
| Gate | Gatter | AND, OR, NOT, XOR, XNOR, NAND, NOR, buffer |
| Minterm, maxterm | Minterm, Maxterm | $m_5 = A\bar{B}C$, $M_5 = \bar{A} \lor B \lor \bar{C}$ |
| DNF, KNF | DNF (disjunktive Normalform), KNF (konjunktive Normalform) | sum of products, product of sums |
| Canonical DNF / KNF | DNF / KNF in Vollform (kanonische DNF / KNF) | every term has every variable |
| Minimal form | Minimalform | fewest terms, then fewest literals |
| Literal | Literal | a variable or its negation |
| Don't care | redundant, beliebig (–) | the value does not matter; `d(…)` |
| KV diagram | KV-Diagramm (Karnaugh-Veitch) | the map of the table in Gray code |
| Block | Block (Päckchen, Schleife) | $2^k$ neighbouring cells of 1s or –, one term |
| Implicant, prime implicant | Implikant, Primimplikant | a block that cannot grow |
| Essential prime implicant | wesentlicher Primimplikant (Kernprimimplikant) | the only block over some 1 |
| Prime implicant chart | Primimplikantentafel (Überdeckungstabelle) | minterms × prime implicants |
| NAND-only, NOR-only | nur NAND, nur NOR | the whole circuit from one gate type |
| Gate delay | Gatterlaufzeit ($t_{pd}$) | the time of one gate |
| Critical path | kritischer Pfad | the input-to-output path with the most gates |
| Counterexample | Gegenbeispiel | a row where two functions differ |

## 4. Decisions

| Topic | Decision |
|---|---|
| Mode | `Mode::Logic`, serialised as `"logic"`, *Logic* / *Logik* in the mode menu next to Basis-N and Elektronik. Inputs read as in `Algebra`. |
| Code location | Module `quantus_core::logic`, no new crate, no new dependency |
| Sequential logic | Not in v1; a feedback loop is an error |
| Size | Up to 8 inputs per scene (256 rows); outputs unlimited |
| KV diagrams | 2 to 6 variables; 5 and 6 as two or four 4-variable maps side by side. 7 and 8 variables: no KV diagram, the Quine–McCluskey tables instead |
| KV labels | German school style: a bar with the variable's name over the rows or columns where it is 1; the Gray code as a tooltip |
| Minimisation | Quine–McCluskey for the prime implicants, Petrick for the cover; all minimal solutions |
| Cost | Fewest terms, then fewest literals |
| Several outputs | Each minimised alone; identical gates drawn once. Joint minimisation later |
| NAND / NOR | NAND-only from the minimal DNF, NOR-only from the minimal KNF; the switch *only 2-input gates*, on by default |
| Delay | Gate levels on the critical path × $t_{pd}$; optionally a time per gate type |
| Generated circuits | Laid out automatically once, then editable like a drawn circuit |
| Symbols | IEC 60617 (default) or ANSI distinctive shapes, a setting |
| Notation | ∧ ∨ ¬ (default) or · + overline, a setting; the input accepts both |
| Checks | Equivalence, the user's KV blocks, the user's minimal form |
| Platforms | Desktop first; the handheld later |
| Session | The scene in `ModeInputs.logic`, saved with the session |
| Files | Save and load the scene as JSON, like circuits |

## 5. Input

### 5.1 The scene and the source of truth

A `LogicScene` holds:

- **inputs**: names in order; the first is the most significant bit of the row number;
- **outputs**: names, and for each output its **source**: `circuit`, `table`, `expression` or
  `minterms`, with the data of that source;
- the **circuit** (if any): the gates and the netlist, with positions kept by the shell;
- the **options**: only 2-input gates, gate times, the chosen minimal solution per output.

Each output has one source; the other views are worked out from it. Editing a view makes it the
source of the outputs it touches:

| Edited | Becomes the source | What follows |
|---|---|---|
| Circuit | `circuit` for every output pin of the circuit | table, forms, KV diagram |
| Table cell | `table` for that output | forms, KV diagram; the circuit as below |
| Expression line | `expression` for its output | table, forms, KV diagram; the circuit as below |
| Minterm line | `minterms` for its output | as the expression |
| KV cell | `table` for that output | as a table cell |

When an output's source is no longer `circuit` and a hand-drawn circuit exists, the circuit is
**not** replaced silently: it is marked *out of date* (the circuit's table differs from the
function; the differing rows are shown), with the button **Draw from the minimal form**. With no
circuit yet, nothing is drawn until the user asks. A drawn circuit that was generated and never
edited is redrawn without asking.

The inputs of a scene are the union of the variables of all sources, in the order of the first
declaration (`f(A, B, C) = …`, the table's columns, the circuit's input pins), otherwise
alphabetical with indices numeric (`x_2` before `x_10`).

### 5.2 Typed expressions

One line per output: `f = …`, `f(A, B, C) = …` or just the expression (output `f`, then `g`, `h`).
The line is MathLive LaTeX, read by a parser of its own (like Basis-N, `basen.rs`, because `+`,
juxtaposition and the overline mean other things here).

| Meaning | Accepted |
|---|---|
| NOT | `\overline{…}`, `\neg`, `\lnot`, `!`, postfix `'`, `\bar{A}`, the word `not` |
| AND | `\land`, `\wedge`, `\cdot`, `*`, `\&`, juxtaposition, the word `and` |
| OR | `\lor`, `\vee`, `+`, `\mid`, the word `or` |
| XOR | `\oplus`, `\veebar`, the word `xor` |
| XNOR | `\odot`, `\leftrightarrow`, `\Leftrightarrow`, the word `xnor` |
| NAND | `\uparrow`, `\barwedge`, the word `nand` |
| NOR | `\downarrow`, the word `nor` |
| Constants | `0`, `1` (also `\top`, `\bot`) |
| Grouping | `( )`, `\left( \right)`, the overline's braces |

- **Precedence**, highest first: NOT; AND (also juxtaposition); NAND; XOR, XNOR; OR; NOR.
  NAND and NOR are not associative: `A \uparrow B \uparrow C` is an error asking for brackets.
- **Variables**: a letter with an optional index (`A`, `x_1`, `Y_{12}`); `AB` is two variables
  `A` and `B`, as in every textbook. A longer name (`Start`) needs `\mathrm{Start}`. Words
  `and`, `or`, … are operators, not variables.
- **Errors** through `msg!`, naming the symbol: an unknown command (`\sin is not a logic
  operator`), a number other than 0 and 1, a missing operand, more than 8 variables.
- The expression is kept as typed; the working starts from it (§8).

### 5.3 Minterm list

`f = \sum m(1, 3, 7) + d(5)`, also `\sum_m`, `\Sigma m`, `m(1,3,7)` and `\sum(1,3,7)`;
`f = \prod M(0, 2, 4, 5, 6)` with an optional `\cdot d(…)` for the product of maxterms.
`f(A, B, C) = …` sets the variables; without them, the number of variables is the smallest that
holds the highest index (at least the scene's inputs), named `A`, `B`, `C`, … . An index twice,
in both `m` and `d`, or beyond $2^n - 1$ is an error.

### 5.4 Truth table

A grid: the input columns, a line, the output columns. Rows always in binary order with the row
number $i$ in a first, grey column. An output cell cycles 0 → 1 → – → 0 with click or
Space; the keys `0`, `1`, `-` set it and move down. Buttons add or remove an input (the table
doubles or halves; removing asks when the outputs depend on it) and add or remove an output;
headers are renamed in place. A new output column is empty, which is an error per row until
filled (*Row 5 of f has no value*), with **Fill the rest with 0** as a fix.

### 5.5 Drawn circuit

The editor of the electronics mode, adapted (§10.2): a grid, snap, rotation by 90°, wires with
corners and junction dots.

| Element | Pins | Notes |
|---|---|---|
| Input | 1 out | a switch with the variable's name; clicking it toggles 0/1 (§10.3) |
| Output | 1 in | a lamp with the output's name |
| Constant | 1 out | 0 or 1 |
| NOT, buffer | 1 in, 1 out | |
| AND, OR, NAND, NOR | 2–8 in, 1 out | the number of inputs set in the element's menu |
| XOR, XNOR | 2–8 in, 1 out | with more than 2 inputs: odd parity / even parity |

The shell sends a **netlist**: each element with its kind, its input nets and its output net
(net numbers from the shell's connectivity, like `CircuitRequest`'s nodes). The core checks:

- every gate input and output pin on a net with exactly one driver (an input pin, a constant or
  a gate output) — *NAND 3, input 2 is not connected*, *Net at G4 has two drivers: G2 and G5*;
- no cycle — *Feedback loop over G2 → G5 → G2: sequential circuits are not supported yet*;
- input and output names unique, at most 8 inputs.

A gate whose output drives nothing is a warning (*G7 drives nothing*), not an error. Each error
carries the element ids so the editor marks them red.

## 6. Operations

All operations work on one output at a time, a function $f: \{0,1\}^n \to \{0, 1, –\}$, held as
two bitsets over ≤ 256 rows (ones and don't cares).

### 6.1 Circuit → expression → table

The circuit is traced from each output back to the inputs; each gate becomes one operator, so the
traced expression has the circuit's structure (unsimplified): `\overline{\overline{AB} \cdot
\overline{C}}`. Shared gates are written once per use. The table follows by evaluating all
$2^n$ rows; at Detailed the intermediate value of each gate is a column of its own (§8).

### 6.2 Expression or minterms → table → canonical forms

An expression is evaluated row by row; a minterm list is the table directly. From the table:

- canonical DNF: the OR of the minterms of the 1-rows, `\sum m(…)` beside it;
- canonical KNF: the AND of the maxterms of the 0-rows, `\prod M(…)` beside it;
- don't cares are listed as `d(…)` and left out of both.

A function that is constantly 0 or 1 says so (*f is always 1*) and has no KV blocks.

### 6.3 KV diagram

- **Layout**: for $n$ variables, the first $\lceil n/2 \rceil$ along the columns, the rest
  along the rows, each axis in Gray order (00, 01, 11, 10). 5 variables: two 4-variable maps
  side by side for the first variable = 0 and = 1; 6 variables: four maps in a 2 × 2 grid for the
  first two variables in Gray order. Cells in the same position of neighbouring maps are
  neighbours (the maps are copied, not mirrored).
- **Labels** (German school style): each variable has a bar along the edge, over the rows or
  columns where it is 1, with its name; the first column variable on top, the second at the
  bottom, the first row variable on the left, the second on the right. The Gray code and the row
  number $i$ show as a tooltip on each cell; the cell's corner shows $i$ in small grey digits
  (a setting of the view).
- **Cells**: 1, 0 (or empty, a view setting) and –. Clicking a cell cycles it like the table.
- **Blocks**: the chosen minimal DNF's terms as coloured rounded rectangles, wrapping over the
  edges (drawn as two or four open pieces) and across maps. A switch shows the KNF blocks over
  the 0s instead. Hovering a term in the result highlights its block, and the other way round.

### 6.4 Minimisation

**Prime implicants** by Quine–McCluskey over the 1s and the don't cares: the minterms grouped by
the number of 1s, neighbouring groups combined where they differ in one bit, repeated until
nothing combines; every term that never combined is a prime implicant. Terms are written as
patterns (`1-0-`) and as products.

**Cover**: the prime implicant chart, rows the prime implicants, columns the minterms with value
1 (don't cares need no cover). Essential prime implicants are taken first, then rows dominated
and columns dominating are removed, and what is left is solved with Petrick's method: the
product of sums is multiplied out with absorption, and the products with the lowest cost are the
minimal covers. Cost: first the number of terms, then the number of literals.

**All minimal solutions** are listed; the first (in the order of their terms' first minterms) is
chosen, and the user can pick another, which then drives the KV blocks, the NAND form and the
drawn circuit. A limit guards Petrick: with more than 5 000 products in an intermediate step the
mode stops with *Too many solutions to compare*, which a function of 8 inputs rarely reaches; it
never returns a cover that might not be minimal.

**Minimal KNF**: the same over the 0s and the don't cares, the terms negated by De Morgan into
sums: $\bar f = \bar{A}\bar{B} \lor \dots \Rightarrow f = (A \lor B) \land \dots$.

### 6.5 NAND-only and NOR-only

**NAND-only** from the chosen minimal DNF $f = p_1 \lor \dots \lor p_k$:

1. double negation and De Morgan: $f = \overline{\overline{p_1} \land \dots \land \overline{p_k}}$,
   so each product becomes a NAND and the OR a NAND of the NANDs (NAND–NAND);
2. a negated variable $\bar A$ becomes `NAND(A, A)` (inverters as NANDs with tied inputs); one
   inverter per variable, shared;
3. a product of one literal needs no first-level NAND: $\overline{\overline{A}} = A$ is dropped,
   the literal goes, negated, straight into the output NAND;
4. with **only 2-input gates** (default on), every NAND with $m > 2$ inputs becomes a balanced
   tree: AND-trees of NAND + inverting NAND pairs, the last stage a NAND. A 3-input NAND becomes
   3 gates, a 4-input one 5 (two NANDs, two inverters, one NAND).

**NOR-only** is the dual from the minimal KNF (NOR–NOR, inverters as `NOR(A, A)`).

For a function given only as a circuit or expression that is not in DNF (an XOR, for example), the
conversion goes through the minimal DNF; a gate-by-gate rewrite of the given circuit (each AND as
NAND + inverter, each OR as inverters + NAND, then cancelling pairs of inverters) is a second
result, *NAND from the circuit as drawn*, with its own gate count.

The result: the NAND (NOR) expression, the gate count, the number of gate inputs and the levels;
**Draw** puts it into the editor (§6.6). The NAND form is not guaranteed to be the circuit with the
fewest gates; a search for that is later (§13).

### 6.6 Table → circuit

**Draw from …** the minimal DNF, the minimal KNF, the NAND form or the NOR form, for one output
or all. The core builds the gates and lays them out:

- inputs on the left in order, each with a vertical bus line; inverters next to the buses;
- one column per level of gates, gates in a column ordered to keep wires short (by the mean row
  of their inputs, two sweeps);
- outputs on the right, in order;
- several outputs share identical gates (same kind, same inputs), so the full adder's sum and
  carry share their common terms where they are literally the same.

The layout comes back as positions on the editor's grid. From then on the circuit is an ordinary
drawn circuit (generated, until the user edits it; §5.1).

### 6.7 Delay and cost

For the circuit (drawn or generated): the **gate count** per type, the **gate inputs**, and the
**levels**: the longest path from an input to each output, counted in gates (input pins, outputs
and constants count 0). The **propagation delay** is levels × $t_{pd}$; with times per gate type
set (e.g. NAND 10 ns, XOR 18 ns, NOT 6 ns), it is the longest path in time instead, in `ns` as a
quantity through `quantity.rs`. The **critical path** is highlighted in the editor and listed
(*A → G1 → G4 → G6 → f*). The same figures are shown for each form in the result panel, so the
minimal DNF, the NAND form with and without 2-input gates can be compared.

### 6.8 Checks

- **Equivalence**: two functions of the scene (an output against an expression, the circuit
  against the table, two typed expressions) are equal when they agree on every row where neither
  is –. Otherwise the first row where they differ is the counterexample, with the inputs and
  both values (*Not equal: for A = 1, B = 0, C = 1 the circuit gives 0, the table 1*), and all
  differing rows highlighted in the table.
- **My KV blocks**: in the KV view, **Draw my blocks** lets the user drag rectangles (with
  wrap-around by dragging over an edge, across maps for 5 and 6 variables). Each block is
  checked: its size a power of two, its shape a block in the Gray-code sense, no 0 inside. Then
  the set: 1s not covered, blocks that could be larger (*could grow to `A\bar{C}`*), blocks
  that are redundant (every 1 also in other blocks). The answer: the user's expression from the
  blocks, and whether it is a minimal form (its cost against the minimum).
- **My minimal form**: a typed expression for an output is checked for equality (above); then
  whether it is a DNF (or KNF), and whether its cost equals the minimum. If not minimal, the hint
  names the first improvement (*the term `AB\bar{C}` can lose `\bar{C}`*), not the full answer.

## 7. Engine

### 7.1 Module layout: `crates/quantus-core/src/logic/`

| File | Contents |
|---|---|
| `mod.rs` | `LogicScene`, `Function` (ones and don't-care bitsets, variable names) |
| `expr.rs` | `Expr`: `Var`, `Const`, `Not`, `And`, `Or`, `Xor`, `Xnor`, `Nand`, `Nor` (n-ary where associative); evaluation, LaTeX in both notations |
| `parse.rs` | the expression and minterm-list parser (§5.2, §5.3) |
| `table.rs` | the truth table, canonical forms |
| `circuit.rs` | the netlist, its checks, tracing to `Expr`, evaluation per gate |
| `minimise.rs` | Quine–McCluskey, the prime implicant chart, Petrick, all minimal covers, cost |
| `kmap.rs` | the KV layout, labels, blocks as cell sets and as drawn rectangles |
| `nand.rs` | NAND-only and NOR-only, the 2-input split, the gate-by-gate rewrite |
| `layout.rs` | the automatic layout of a generated circuit |
| `delay.rs` | levels, delay, critical path, gate counts |
| `check.rs` | equivalence, the user's blocks, the user's minimal form |
| `working.rs` | the steps (`Working`, `Note` as in `geometry/working.rs`) |
| `interface.rs` | the serde structs of §7.2, re-exported in `api.rs` |
| `text.rs` | the `msg!` texts |

A pattern is a pair of `u8` (value, mask); a function a pair of `[u64; 4]`. No new dependency.

### 7.2 Interface

| Function | Request | Response |
|---|---|---|
| `api::logic` | `LogicRequest { scene, task?, precision? }` | `LogicResponse { inputs[], outputs[], circuit?, task?, errors[] }` |

- `OutputResponse { name, source, table, canonical { dnf, knf, minterms, maxterms, dontCares },
  minimal { dnf[], knf[], chosen }, kmap?, nand, nor, cost, steps[] }`: each output with every
  form as LaTeX in the chosen notation; `dnf[]` and `knf[]` hold all minimal solutions.
- `KmapResponse { maps[], rowVars[], colVars[], cells[], blocks[] }`: positions and labels; the
  shells only draw.
- `CircuitResponse { values[], delay, criticalPath[], counts, outOfDate[] }`: per net its value
  for the simulated input row (§10.3), the delay figures, and the rows where the circuit and an
  output's source differ.
- `TaskResponse`: the result of a task: `draw` (a generated circuit with positions), `equivalent`,
  `checkBlocks`, `checkForm`, each with its steps and an `alignBlock` for **Copy**.
- Errors carry element ids or table rows (`ErrorResponse { message, elements[], rows[] }`).
- Stateless; the Tauri command `logic` wraps it, reading the language, notation and step detail
  from the store like `geometry`. The page recomputes on every change; an answer to an older
  request is dropped.

## 8. Steps

Each line has a text (`msg!`, English and German), a formula and the lowest level it shows at.

| Working | Brief | Standard | Detailed | Maximal |
|---|---|---|---|---|
| Circuit → expression | the traced expression | the expression gate by gate, output first | each gate's output as a named intermediate ($G_1 = \overline{AB}$) | — |
| Expression → table | the table | — | one column per subexpression | the evaluation of one example row |
| Canonical forms | DNF and KNF | `\sum m(…)`, `\prod M(…)` and the rows they come from | each minterm written from its row | — |
| KV diagram | the minimal form | the blocks and their terms | why each variable stays or goes in a block | the prime implicants not used |
| Quine–McCluskey | the minimal form | the prime implicants | the grouping tables, step by step, with ✓ for combined terms | the pairs tried that did not combine |
| Cover | — | the essential prime implicants | the chart, reductions by dominance | Petrick's product multiplied out |
| NAND / NOR | the NAND form | double negation and De Morgan | the 2-input split, inverters | the cancelled double negations |
| Delay | levels and delay | the critical path | the level of every gate | — |
| Checks | equal or not, minimal or not | the counterexample row, the faults of each block | each block's check | — |

At 7 and 8 variables the Quine–McCluskey tables replace the KV diagram in the working. The
worked examples level by level (majority, a 7-segment segment with don't cares, the NAND form of
an XOR) are to be added to [step-detail.md](step-detail.md) as a section *Logic mode*.

## 9. Settings

| Setting | Values | Default |
|---|---|---|
| `logic_symbols` | `iec` (rectangles with `&`, `≥1`, `=1`, `1`) or `ansi` (distinctive shapes) | `iec` |
| `logic_notation` | `logic` ($A \land \lnot B \lor C$, with the overline for longer negations) or `technical` ($A \cdot \overline{B} + C$) | `logic` |
| `logic_two_input` | NAND / NOR forms only from 2-input gates | `true` |
| `logic_zeros` | KV diagram and table show 0s or leave them empty | show |

The input accepts every notation whatever the setting; output, KV labels and **Copy** follow it.
Settings live in `Settings` (`settings.rs`) and the settings dialog's Logic section.

## 10. Desktop page

### 10.1 Layout

From top to bottom, the panels collapsible:

1. **Function lines**: one MathLive line per output (expression or minterm list), with the
   output's name; a small marker shows which view is each output's source.
2. **Circuit editor** with a toolbar (the elements of §5.5, rotate, delete, **Draw from…**,
   **Arrange**), the gate count and delay in its corner.
3. **Truth table** (§5.4) and the **KV diagram** (§6.3) side by side, one output at a time
   (tabs per output).
4. **Results**: per output the canonical forms, the minimal forms (a choice when there are
   several), NAND and NOR forms with their cost, the checks, each with its steps and **Copy**.

Narrow windows stack the table under the KV diagram.

### 10.2 The editor

`LogicEditor.tsx` reuses the electronics editor's grid, snapping, rotation, terminals, label
placement and wire routing (`desktop/src/circuit.ts`, `desktop/src/labelLayout.ts`, the
interaction of `CircuitEditor.tsx`); what is common moves into a shared module rather than being
copied. New: gate symbols in both styles as SVG, pins on a gate that grow with its number of
inputs, an inversion bubble on NAND/NOR/NOT/XNOR (IEC: the small triangle or circle, as the
standard draws it), nets coloured by value.

### 10.3 Simulation

The editor is live: clicking an input switch toggles it; each net is drawn in the colour of its
value (1 bright, 0 dim), the lamps light, and the matching table row and KV cell are highlighted.
Clicking a table row or KV cell sets the switches to it. The values come from the core
(`CircuitResponse.values`), so the editor has no logic of its own.

### 10.4 Texts

All labels in every file of `locales/` under `mode.logic` and `logic.*`; the gate names stay
English in both languages (AND, NAND), as in German textbooks.

### 10.5 Export

- **Copy** on every result: LaTeX `align*` with the steps; the truth table as a `tabular`; the KV
  diagram as a `tabular` with the bars as `\overline` labels (TikZ is not used).
- The circuit as **SVG** and **PNG** (the editor's SVG, without the grid, in the light theme).
- The **write-up** (`crates/quantus-core/src/writeup`) takes the scene's results like the other
  modes.
- **Save / Load** the scene as a `.quantus-logic.json` file; the electronics mode's file dialog.

## 11. Tests

`crates/quantus-core/tests/logic.rs`, targeted:

- **Parser**: every notation of §5.2, precedence, `AB` as two variables, `\mathrm{Start}`,
  errors; minterm lists with `d(…)`; round trip expression → LaTeX → expression.
- **Reference functions**: majority of three (one minimal DNF, 3 terms); XOR of 3 (4 terms,
  nothing combines, no KV block larger than 1); full adder sum and carry; segment *a* of a BCD
  7-segment decoder with don't cares 10–15; a function with two minimal solutions (cyclic
  chart, e.g. $\sum m(0,1,2,5,6,7)$) listing both; constants 0 and 1.
- **Minimality**: for all functions of 3 variables (256) and a sample of 4 variables, the minimal
  DNF's cost against a brute force over all covers.
- **NAND / NOR**: the NAND form equals the function on every row, with and without the 2-input
  split; gate counts of the majority (4 gates / 6 gates) and of the 3-input NAND split (3 gates).
- **Circuit**: tracing, an undriven pin, two drivers, a cycle, all with the right element ids;
  levels and the critical path; shared gates in a generated full adder.
- **Checks**: a counterexample row; KV blocks too large, wrong shape, over a 0, redundant,
  growable; a non-minimal form with its hint.

## 12. Implementation order

Each step a commit on `dev`, its plan box ticked.

1. `expr.rs`, `parse.rs`, `table.rs`: expressions and minterm lists to truth tables and
   canonical forms; parser tests.
2. `minimise.rs`: Quine–McCluskey, chart, Petrick, all minimal DNFs and KNFs; the brute-force
   test.
3. `working.rs` and the steps of 1–2 at all levels.
4. `kmap.rs`: layout, labels, blocks.
5. `nand.rs`: NAND-only and NOR-only, the 2-input split, steps.
6. `circuit.rs`, `delay.rs`: netlist checks, tracing, evaluation, levels and critical path.
7. `layout.rs`: generated circuits.
8. `check.rs`: equivalence, the user's blocks and form.
9. `interface.rs`, `api::logic`, `Mode::Logic`, `ModeInputs.logic`, the settings, the Tauri
   command.
10. Desktop: the page with function lines, truth table and results.
11. Desktop: the KV diagram, including drawing one's own blocks.
12. Desktop: the editor (shared code with the electronics editor), both symbol sets.
13. Desktop: simulation, **Draw from…**, delay display, export, save and load.
14. Docs pages, the changelog, `step-detail.md` § Logic mode.

## 13. Later

- **Sequential logic**: SR, D, JK and T flip-flops, clocked simulation, then state machines
  (Moore and Mealy: state table → transition table → KV diagrams → circuit).
- **Timing**: a timing diagram per input change, glitches, static hazards found in the KV
  diagram and the term that removes them.
- **Blocks**: half and full adder, multiplexer, decoder, 7-segment display, and own
  subcircuits saved as blocks.
- **Multi-output minimisation** with shared terms.
- **Fewest gates**: a search for the NAND circuit with the fewest gates, free inverted inputs as
  an option.
- **More than 8 inputs** with Espresso, marked *possibly not minimal*.
- **Random exercises**: a function to minimise or to convert, the solution hidden.
- **The handheld**: function lines, table and KV diagram (the editor at most as a view).
- **HDL export**: VHDL and Verilog of a circuit.
