---
sidebar_position: 2
title: The working
---

# The working

The panel under the canvas holds the whole solution. Drag the bar above it to make it taller or
shorter.

1. **Junction rules**: at each junction the currents flowing in equal those flowing out,
   $I_1 + I_2 - I_3 = 0$. In each connected part of the circuit, the rule at the last junction
   follows from the others; it is shown greyed and left out.
2. **Loop rules**: around each loop, clockwise, the voltages across the resistors add up to the
   source voltages. Each rule is written with names first, $R_1 I_1 + R_3 I_3 = U_1$, then with
   the values, $1000\,\Omega \cdot I_1 + 2000\,\Omega \cdot I_3 = 10\,\mathrm{V}$.
3. **Matrix**: the rules as $[A \mid b]$ in the unknowns, and the Gauss steps that solve it, as in
   [Linear systems](../linear-systems/solution.md#the-steps).
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

## Current sources

A current source fixes the current of its branch, so that current is not an unknown. The voltage
across the source, $U_{q,k}$, takes its place. The loop rules of the loops through it contain
that voltage.

## Internal resistance and conductance

A voltage source's internal resistance is a resistor in series with it, named
$R_{\mathrm{i,V1}}$ in the equations. A current source's internal conductance is a branch of its
own in parallel with the source, with $R = 1/G$. Neither appears as a part on the canvas.

## Crossing wires

When wires cross without being connected, the windows of the drawing are not the loops of the
circuit. The loops then come from a spanning tree, one for each branch outside the tree, and a
note above the working says so. The loop arrows are drawn as usual.

## See also

- [Kirchhoff](index.md): drawing the circuit
- [Linear systems](../linear-systems/solution.md#the-steps): the Gauss steps in detail
