---
sidebar_position: 8
title: Kirchhoff
---

# Kirchhoff

**Kirchhoff** mode works out a resistor network the way it is done on paper: you draw the
circuit, and the page sets up the junction and loop rules, writes them as a matrix, solves it with
the Gauss steps and gives every current, voltage and power with a power balance. Values are read
as in [Physics](physics/index.md): `10V`, `2k\Omega`, `500mA`.

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
`.json` format, so a circuit drawn in one mode opens in the other.

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
   [Linear systems](linear-systems/solution.md#the-steps).
4. **Currents**: each unknown, exact: $I_1 = \frac{3}{800}\,\mathrm{A}$.
5. **Power balance**: the power the sources deliver equals the power the resistors take.

Hovering a rule highlights its loop or junction on the canvas. **Copy** puts the working on the
clipboard as an `align*` block.

For the circuit with 10 V and 5 V, $R_1 = 1\,\mathrm{k\Omega}$, $R_2 = R_3 = 2\,\mathrm{k\Omega}$
(`test/circuits/two-sources.json`):

| | |
|---|---|
| Junction rule | $N_1: I_1 - I_2 - I_3 = 0$ |
| Loop rules | $L_1: R_1 I_1 + R_3 I_3 = U_1$, $L_2: -R_3 I_3 + R_2 I_2 = -U_2$ |
| Currents | $I_1 = \frac{3}{800}\,\mathrm{A}$, $I_2 = \frac{1}{1600}\,\mathrm{A}$, $I_3 = \frac{1}{320}\,\mathrm{A}$ |
| Power balance | $\sum P_{\mathrm{gen}} = 34{,}375\,\mathrm{mW} = \sum P_{\mathrm{cons}}$ |

The current through $R_2$ runs into the `+` of the 5 V source, so its arrow points there, and the
source shows $P = -3{,}125\,\mathrm{mW}$: it is charged rather than delivering. Click $I_2$ to see the
same circuit with $I_2 = -\frac{1}{1600}\,\mathrm{A}$.

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

## Resistance between two points

To find the resistance of a network between two points, draw only resistors and wires, and
place the parts **Terminal A/B** from the palette: the first one is A, the second B. Each sits
on a grid point, the end of a resistor or a point on a wire. While only one is placed, the
working waits for the other.

The working then reduces the network the way it is done by hand, one merge per line:

- **In parallel**: resistors between the same two nodes, $R_{23} = \frac{R_2 \cdot R_3}{R_2 + R_3}$,
  for three or more as the reciprocal of the sum of reciprocals.
- **In series**: two resistors that meet at a node where nothing else branches off,
  $R_{123} = R_1 + R_{23}$.

Parallel resistors are merged first, then series ones, until one resistor between A and B is
left: that is $R_{AB}$, exact. A fraction is followed by its decimal, with $=$ if the decimal ends
within the precision and $\approx$ if not: $R_{AB} = \frac{2}{3}\,\Omega \approx 666{,}667\,\mathrm{m}\Omega$.
The currents of the junction and loop rules are written the same way. The merged resistor is named after the parts it stands for. While
the pointer is on a line, the canvas shows the network after that merge: the merged resistor sits
in the place of the first one it stands for, a resistor in series with it becomes a wire, and one
in parallel goes away. Moving the pointer off brings the drawing back. The step detail decides how much each line
shows: the rule only (Brief), the formula (Standard), or also the values put in (Detailed).

Parts that carry no current are left out, each with a note: a resistor with an open end, one
whose terminals are joined by a wire, and parts not connected to A and B. If A and B are joined
by a wire, $R_{AB} = 0\,\Omega$.

Some networks, such as a Wheatstone bridge, can't be reduced by series and parallel alone. After
the merges that are possible, the rest is solved with the node rule: a test current
$I = 1\,\mathrm{A}$ flows in at A and out at B, B is at $0\,\mathrm{V}$, the potentials
$\varphi$ of the other nodes come from the node equations and their Gauss steps, and
$R_{AB} = \varphi_A / I$.

With the setting **Resistance between A and B** set to a star–delta conversion, such a network
is reduced further instead. **Delta → star** takes three resistors that form a ring over three
nodes and puts a star around a new node in their place. Each arm is the product of the two
resistors at its corner over the sum of all three:
$R_{\mathrm{Y}1} = \frac{R_1 \cdot R_2}{R_1 + R_2 + R_3}$.
**Star → delta** takes three resistors that meet at a node where nothing else is connected and
puts a ring between their outer ends. Each side is the sum of the products of two arms over the
arm opposite it: $R_{\Delta 1} = \frac{R_1 R_2 + R_2 R_3 + R_3 R_1}{R_3}$.
If the chosen form isn't in the network, the other one is used. The merging then goes on as
before, and the new resistors count like drawn ones: $R_{5,\mathrm{Y}2} = R_{\mathrm{Y}2} + R_5$.
A conversion is one line of the working. While the pointer is on that line, the canvas shows the
three new resistors in place of the old ones, across, down or at 45°, whichever reaches their
nodes best. A new star's centre is put where its arms reach the corners with the least wire. A network that the conversions don't reduce
either is finished with the node rule as above.

Sources can't be part of such a drawing: remove them, or remove the terminals to work out the
currents.

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

See [Settings](../reference/settings.md).

## See also

- [Linear systems](linear-systems/index.md): the Gauss steps in detail
- [Working and output](../basics/working-and-output.md)
