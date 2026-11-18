---
sidebar_position: 4
title: Circuit editor
---

# Circuit editor

The circuit editor draws a circuit of gates. A drawn circuit is the source of every output that
has a lamp in it: its function is traced from the lamp back to the switches, and the table, the
forms and the KV diagram follow. The other way round, **Draw from** puts a circuit drawn from a
minimal form into the editor.

Click **Circuit** above the table to open the editor; it is folded away while there is no
circuit. It has the parts on the left, the canvas in the middle and the selected part on the
right, like the [Electronics](../electronics.md) editor.

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

A gate's type can be changed in the panel on the right; it keeps its place and its wires. The
symbols are the IEC 60617 rectangles (`&`, `≥1`, `=1`, `1`) or the ANSI shapes, as set under
**Gate symbols**; NAND, NOR, NOT and XNOR have the inversion mark.

Drag a part onto the canvas or click it in the list. Dragging from a pin, or from a point on a
wire, draws a wire; dragging on empty canvas selects several parts. <kbd>Space</kbd> or
<kbd>R</kbd> rotates the selection by 90°, <kbd>Del</kbd> deletes it, <kbd>Esc</kbd> clears it.
The canvas moves with the right or middle mouse button or by scrolling, zooms with
<kbd>Ctrl</kbd> and scrolling, and <kbd>Home</kbd> shows the whole circuit.

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
- for one output, or for **all outputs** at once. Several outputs share their identical gates.

The circuit is laid out on the grid: the switches on the left, each with a bus line down, the
inverters by the buses, a column per level of gates, the lamps on the right. A circuit drawn this
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
