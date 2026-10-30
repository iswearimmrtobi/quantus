---
sidebar_position: 8
title: Kirchhoff
---

# Kirchhoff

**Kirchhoff** mode works out a resistor network the way it is done on paper: you draw the
circuit, and the page sets up the junction and loop rules, writes them as a matrix, solves it with
the Gauss steps and gives every current, voltage and power with a power balance. Values are read
as in [Physics](physics.md): `10V`, `2k\Omega`, `500mA`.

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
<kbd>Space</kbd> rotates the selection and <kbd>Del</kbd> deletes it. Two terminals are connected
when they sit on the same grid point, or one ends in the middle of a wire. The canvas moves with
the right or middle mouse button or by scrolling, and zooms with <kbd>Ctrl</kbd> and scrolling.

The source's `+` terminal is the end with the `+` mark. Every value must be given, and every
terminal must be connected to something.

The circuit is kept when you switch to another mode and back. It is separate from the Electronics
circuit, but both use the same files: **Load** and **Save** under *File* read and write the same
`.json` format, so a circuit drawn in one mode opens in the other.

## What the canvas shows

Once the circuit can be solved, the canvas gets the names the equations use:

- **Junctions** $N_1, N_2, \dots$ (German $K_1, K_2, \dots$): a dot where three or more
  connections meet. Junctions are numbered from top to bottom, then left to right.
- **Branch currents** $I_1, I_2, \dots$: a green arrow beside one part of each branch. Parts in
  series between two junctions form one branch and share its current.
- **Loops** $L_1, L_2, \dots$ (German $M_1, M_2, \dots$): a circular arrow in every window of the
  drawing, always clockwise. Hovering one highlights its parts.

The buttons **U I P R** at the top right choose which values the parts show, as in Electronics.

### Reference directions

The current arrows are chosen for you: out of a source's `+` terminal, and through a resistor
from its first terminal to its second. **Click an arrow to reverse it.** The equations, the matrix
and the results change with it; a current that comes out negative simply flows against its arrow.
The direction is saved with the circuit.

## The working

The panel under the canvas holds the whole solution. Drag the bar above it to make it taller or
shorter.

1. **Junction rules**: at each junction the currents flowing in equal those flowing out,
   $I_1 + I_2 - I_3 = 0$. In each connected part of the circuit, the rule at the last junction
   follows from the others; it is shown greyed and left out.
2. **Loop rules**: around each loop, clockwise, the voltages across the resistors add up to the
   source voltages. Each rule is written with names first, $R_1 I_1 + R_3 I_3 = U_1$, then with
   the values, $1000\,\Omega \cdot I_1 + 2000\,\Omega \cdot I_3 = 10\,\mathrm{V}$.
3. **Matrix**: the rules as $[A \mid b]$ in the unknowns, and the Gauss steps that solve it, as in
   [Linear systems](linear-systems.md#the-steps).
4. **Currents**: each unknown, exact: $I_1 = \frac{3}{800}\,\mathrm{A}$.
5. **Power balance**: the power the sources deliver equals the power the resistors take.

Hovering a rule highlights its loop or junction on the canvas. **Copy** puts the working on the
clipboard as an `align*` block.

For the circuit with 10 V and 5 V, $R_1 = 1\,\mathrm{k\Omega}$, $R_2 = R_3 = 2\,\mathrm{k\Omega}$
(`test/circuits/two-sources.json`):

| | |
|---|---|
| Junction rule | $N_1: I_1 + I_2 - I_3 = 0$ |
| Loop rules | $L_1: R_1 I_1 + R_3 I_3 = U_1$, $L_2: -R_3 I_3 - R_2 I_2 = -U_2$ |
| Currents | $I_1 = \frac{3}{800}\,\mathrm{A}$, $I_2 = -\frac{1}{1600}\,\mathrm{A}$, $I_3 = \frac{1}{320}\,\mathrm{A}$ |
| Power balance | $\sum P_{\mathrm{gen}} = 34{,}375\,\mathrm{mW} = \sum P_{\mathrm{cons}}$ |

$I_2$ is negative: the current through $R_2$ runs into the 5 V source, against its arrow.

### Current sources

A current source fixes the current of its branch, so that current is not an unknown. The voltage
across the source, $U_{q,k}$, takes its place. The loop rules of the loops through it contain
that voltage.

### Internal resistance and conductance

A voltage source's internal resistance is a resistor in series with it, named
$R_{\mathrm{i,V1}}$ in the equations. A current source's internal conductance is a branch of its
own in parallel with the source, with $R = 1/G$. Neither appears as a part on the canvas.

### Crossing wires

When wires cross without being connected, the windows of the drawing are not the loops of the
circuit. The loops then come from a spanning tree, one for each branch outside the tree, and a
note above the working says so. The loop arrows are drawn as usual.

## What is refused

| Input | Message |
|---|---|
| A potentiometer, a power load, or a wire with a material | *P1 can't be used in the Kirchhoff mode: …* |
| A part without its value | *The value … of R1 is missing.* |
| A terminal connected to nothing | *R1 has an open end: nothing is connected to one of its terminals.* |
| Voltage sources alone in a loop, or current sources alone at a junction | *The equations contradict each other: …* or *The equations leave the currents open: …* |

The last kind still shows the equations and the steps, up to the row that shows the
contradiction. Ground symbols in a loaded Electronics file are ignored.

## Settings that matter here

- **Step detail**: how much of the Gauss working is shown.
- **Systems of equations**: Gauss or Gauss–Jordan steps.
- **Precision**: the significant digits of the values on the canvas. The currents in the working
  are exact.
- **Language**: the names of junctions and loops and the notes.

See [Settings](../reference/settings.md).

## See also

- [Linear systems](linear-systems.md): the Gauss steps in detail
- [Working and output](working-and-output.md)
