---
sidebar_position: 4
title: Circuit editor
---

# Circuit editor

The circuit editor draws a circuit of gates. A drawn circuit is the source of every output that
has a lamp in it: its function is traced from the lamp back to the switches, and the table, the
forms and the KV diagram follow. The other way round, **Draw from** puts a circuit drawn from a
minimal form into the editor.

Switch **Table & KV | Circuit** to *Circuit* to see the editor in place of the table and the KV
diagram; the results stay under it. The page opens on *Circuit* when the scene has a circuit, and
**Draw from** or **Open in editor** switch to it. It has the parts on the left and the canvas beside them, as in the
[Electronics](../electronics.md) editor; clicking a part opens its settings in a box beside it (a switch: double-click, since a click flips it).

## Parts

| Part | Pins | Settings |
|---|---|---|
| **Input switch** | 1 out | its name: `A`, `x_1`, `Start` |
| **Output lamp** | 1 in | its name, the output's name: `f`, `Y_2` |
| **Constant** | 1 out | 0 or 1 |
| **NOT**, **buffer** | 1 in, 1 out | |
| **AND**, **OR**, **NAND**, **NOR** | 2 to 8 in, 1 out | the number of inputs |
| **XOR**, **XNOR** | 2 to 8 in, 1 out | the number of inputs; with more than 2, odd and even parity |
| **Wire** | | |

A gate's type can be changed in the box beside it; it keeps its place and its wires. The
symbols are the IEC 60617 rectangles (`&`, `≥1`, `=1`, `1`) or the ANSI shapes, as set under
**Gate symbols**; NAND, NOR, NOT and XNOR have the inversion mark.

Drag a part onto the canvas or click it in the list. Dragging from a pin, or from a point on a
wire, draws a wire; dragging on empty canvas selects several parts. As in
[Electronics](../electronics.md#drawing-the-circuit), a wire between two pins turns halfway with
equal stubs (once between a sideways and an upright pin), two wires that meet end to end become one, and the wires on a moved part's pins go
with it. <kbd>Space</kbd> or
<kbd>R</kbd> rotates the selection by 90°, <kbd>Del</kbd> deletes it, <kbd>Esc</kbd> clears it.
<kbd>Ctrl</kbd>+<kbd>Z</kbd> undoes a change and <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Z</kbd> makes it again
(also ↶ and ↷ beside the zoom buttons); a drag or the typing in one field is one step.
<kbd>Ctrl</kbd>+<kbd>C</kbd> and <kbd>Ctrl</kbd>+<kbd>V</kbd> copy and paste the chosen parts,
<kbd>Ctrl</kbd>+<kbd>D</kbd> duplicates them, and the arrow keys move them by a grid step. *Keys*
under the parts lists every key; see [Keyboard shortcuts](../../reference/shortcuts.md#in-the-circuit-editors).
A pasted switch or lamp keeps its name only while no other has it.
Scrolling zooms about the pointer; the canvas moves with the right or middle mouse button or
with <kbd>Shift</kbd> and scrolling. The buttons in the canvas's top left corner zoom out, back to
100 % and in, and **Fit** shows the whole circuit; on the canvas <kbd>+</kbd>, <kbd>-</kbd> and
<kbd>0</kbd> do the same, and so does <kbd>Home</kbd> for the whole circuit.

The last of those buttons, ⤢, lets the editor fill the window, with the output tabs in the bar
above the canvas; ⤡ or <kbd>Esc</kbd> (when nothing is selected) goes back.

## Checks

Nothing is guessed: an open pin is not silently 0. The parts at fault turn red, with the message
under the canvas.

| Circuit | Message |
|---|---|
| A gate input with no wire | *G3, input 2 is not connected.* |
| Two outputs on one net | *G2 and G5 drive the same net.* |
| A loop back into a gate | *Feedback loop over G2 → G5 → G2: sequential circuits are not supported yet.* |
| A switch or lamp without a name, two with the same name, more than 8 switches | |

A gate whose output goes nowhere is only a warning: *G7 drives nothing.*

## Simulation

The circuit is live. Clicking an input switch toggles it between 0 and 1; every wire is drawn in
the colour of its value (1 bright, 0 dim) and the lamps light. The row of the table and the cell
of the KV diagram that match the switches are highlighted. The other way round, a click on a row
number in the table, or <kbd>Ctrl</kbd>+click on a KV cell, sets the switches to that row.

## Draw from

**Draw from**, in the bar above the canvas, replaces the circuit with one drawn from a form of
the function. While the canvas is empty and there is a function, the same six buttons stand in
its middle.

- **minimal DNF**, **minimal KNF**, **NAND only**, **2-input NAND only**, **NOR only** or
  **2-input NOR only**;
- for **all outputs** at once (the default), or for one. Several outputs share their identical
  gates, so a truth table with several output columns gives one circuit.

With several outputs, the tabs beside **Table & KV | Circuit** read **All**, f, g, … in the
circuit view. On a drawn circuit a tab draws it again at once, in the same form, for that output
or for all; on a circuit you made or edited by hand it only chooses what the next **Draw from**
draws, and leaves your circuit as it is.

The circuit is laid out on the grid: the switches on the left, each with a bus line down (the
buses in the order that makes the fewest wires cross), the inverters by the buses, a column per level of gates, the lamps on the right. A circuit drawn this
way follows its function: it is drawn again from the same form when the outputs or the inputs
change, until you edit it. **Open in editor** in the [Gates](results.md#gates) card draws the same
way, and can also draw a NAND or NOR form rewritten from the expression. From then on it is a drawn circuit like any other.

**Arrange** lays the circuit out afresh in the same way, keeping its gates.

### Out of date

When an output's source is a function line or the table and a circuit you drew no longer gives
the same values, the circuit is not replaced silently. A note under the canvas says *Out of date:
the circuit gives other values for f in rows 3, 5*, with the button **Draw from the minimal form**.

## Gates and delay

The bar under the canvas shows the circuit's figures, with the note or the errors beside them:

- *gates*, *gate inputs* and *levels*: the most gates on any path from a switch to a lamp
  (switches, lamps and constants count 0);
- the **delay**: the levels times $t_{pd}$, or a time when the field $t_{pd}$ has one (`10ns`
  gives $40\,\mathrm{ns}$ for 4 levels);
- the **critical path**, the path with the most delay, such as $A$ → G1 → G4 → G6 → $f$.
  Click it to mark it in the circuit, dashed in red; click again to hide it. It starts hidden.

## Picture

**SVG** and **PNG** under *Picture* save the circuit as a picture: the parts, wires, junctions
and names as the editor shows them, black on white, without the grid, the pins, the selection or
the simulation. The PNG is at twice the size.

## See also

- [Logic](index.md): the page, files and the write-up
- [Results](results.md): the Gates card with the forms the circuits are drawn from
- [Electronics](../electronics.md): the same editor for DC circuits
