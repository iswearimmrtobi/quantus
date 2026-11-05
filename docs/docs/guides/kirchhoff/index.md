---
sidebar_position: 1
title: Kirchhoff
---

# Kirchhoff

**Kirchhoff** mode works out a resistor network the way it is done on paper: you draw the
circuit, and the page sets up the junction and loop rules, writes them as a matrix, solves it with
the Gauss steps and gives every current, voltage and power with a power balance. Values are read
as in [Physics](../physics/index.md): `10V`, `2k\Omega`, `500mA`.

## Drawing the circuit

Switch to **Kirchhoff** in the top bar. The page has the parts on the left, the canvas in the
middle, the selected part's values on the right, and the working under the canvas.

| Part | Field | Example |
|---|---|---|
| **Voltage source** | Voltage, internal resistance (optional) | `10V`, `0.5\Omega` |
| **Current source** | Current, internal conductance (optional) | `2A`, `1mS` |
| **Resistor** | Resistance | `1k\Omega` |
| **Wire** | none: wires are ideal | |

Drag a part onto the canvas or click it in the list. Dragging from a terminal draws a wire;
<kbd>Space</kbd> rotates the selection, <kbd>Shift</kbd>+<kbd>R</kbd> turns a resistor by 45° and
<kbd>Del</kbd> deletes it. Two terminals are connected
when they sit on the same grid point, or one ends in the middle of a wire. The canvas moves with
the right or middle mouse button or by scrolling, and zooms with <kbd>Ctrl</kbd> and scrolling.

The source's `+` terminal is the end with the `+` mark. Every value must be given, and every
terminal must be connected to something.

The circuit is kept when you switch to another mode and back. It is separate from the Electronics
circuit, but both use the same files: **Load** and **Save** under *File* read and write the same
`.json` format, so a circuit drawn in one mode opens in the other, see
[Circuit files](../circuit-files.md).

## What the canvas shows

Once the circuit can be solved, the canvas gets the names the equations use:

- **Junctions** $N_1, N_2, \dots$ (German $K_1, K_2, \dots$): a dot where three or more
  connections meet. Junctions are numbered from top to bottom, then left to right.
- **Branch currents** $I_1, I_2, \dots$: a green arrow beside one part of each branch, its first
  resistor if it has one. Parts in series between two junctions form one branch and share its
  current.
- **Voltages**: a blue arrow beside every part while **U** is on. A resistor's runs with its
  current (consumer convention), a voltage source's from `+` to `−` and a current source's against
  the current it drives (generator convention).
- **Loops** $L_1, L_2, \dots$ (German $M_1, M_2, \dots$): a circular arrow in every window of the
  drawing, always clockwise. Hovering one highlights its parts.

The buttons **U I P R** at the top right choose which values the parts show, as in Electronics.
Here the values follow the arrows: a resistor's $U$ and $I$ along its arrows, so $P = U \cdot I$ is
what it takes, and a source's $I$ is the current it delivers, so $P$ is what it gives. Their powers
add up to the power balance below. (Electronics counts every part from its first terminal to its
second instead.)

### Reference directions

The current arrows point the way the current really flows, from `+` to `−` outside the
sources, so every current comes out positive. **Click an arrow to reverse it**, as if you had
guessed that direction on paper: the equations, the matrix and the results change with it, and
the current comes out negative, because it flows against its arrow. The reversal is saved with
the circuit.

## What is refused

| Input | Message |
|---|---|
| A potentiometer, a power load, or a wire with a material | *P1 can't be used in the Kirchhoff mode: …* |
| A part without its value | *The value … of R1 is missing.* |
| A terminal connected to nothing | *R1 has an open end: nothing is connected to one of its terminals.* |
| Voltage sources alone in a loop, or current sources alone at a junction | *The equations contradict each other: …* or *The equations leave the currents open: …* |
| A source together with the terminals A and B | *V1 is a source; the resistance between A and B is worked out for resistors and wires only, …* |
| A and B with no resistor path between them | *No resistor path leads from A to B, so no current can flow: R_AB is infinite.* |
| A terminal on no part and no wire | *B is not connected: put it on the terminal of a resistor or on a wire.* |

The last kind still shows the equations and the steps, up to the row that shows the
contradiction. Ground symbols in a loaded Electronics file are ignored.

## Settings that matter here

- **Step detail**: how much of the Gauss working is shown.
- **Systems of equations**: Gauss or Gauss–Jordan steps.
- **Resistance between A and B**: the node rule, or a star–delta conversion where series and
  parallel get stuck.
- **Precision**: the significant digits of the values on the canvas. The currents in the working
  are exact.
- **Language**: the names of junctions and loops and the notes.

See [Settings](../../reference/settings.md).

## See also

- [The working](working.md): junction and loop rules, the matrix and the currents
- [Resistance between two points](resistance.md)
- [Linear systems](../linear-systems/solution.md#the-steps): the Gauss steps in detail
- [Working and output](../../basics/working-and-output.md)
