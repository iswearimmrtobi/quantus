---
sidebar_position: 3
title: Resistance between two points
---

# Resistance between two points

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

## See also

- [Kirchhoff](index.md): drawing the circuit
- [Settings](../../reference/settings.md): **Resistance between A and B**
