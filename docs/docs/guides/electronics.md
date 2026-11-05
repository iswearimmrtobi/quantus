---
sidebar_position: 9
title: Electronics
---

# Electronics

**Electronics** mode is a circuit calculator for DC circuits: resistor networks, voltage
dividers, voltage drop on a cable. You draw the circuit, and every part shows its voltage and
current as you go. The values are exact until they are written; inputs are read as in
[Physics](physics/index.md): `230V`, `2k\Omega`, `2.5mm^2`.

## Drawing the circuit

Switch to **Electronics** in the top bar. The page has the parts on the left, the canvas in the
middle and the selected part's values on the right.

| Part | Fields | Example |
|---|---|---|
| **Voltage source** | Voltage, internal resistance (optional) | `230V`, `0.5\Omega` |
| **Current source** | Current, internal conductance (optional) | `2A`, `1mS` |
| **Resistor** | Resistance | `10\Omega` |
| **Potentiometer** | Total resistance, position (0 to 1) | `10k\Omega`, `0.25` |
| **Power load** | Power | `500W` |
| **Wire** | Material, length, cross-section, temperature, or a fixed resistance | `100m`, `2mm^2`, `20°C` |
| **Ground** | none: the reference potential | |

Drag a part onto the canvas or click it in the list. Dragging from a terminal, or from a grid
point on a wire, draws a wire. Wires run only across and down: one drawn on a slant becomes two at
a right angle. <kbd>Space</kbd> or <kbd>R</kbd> rotates the selection, <kbd>Shift</kbd>+<kbd>R</kbd> turns a
resistor by 45° (its terminals then sit on the grid two steps across and two down from its
middle), and <kbd>Del</kbd> deletes it; dragging on empty canvas selects several parts.

Two terminals are connected when they sit on the same grid point, or when one ends in the middle
of an ideal wire (a T-junction). The open dot at a terminal disappears once something is
connected there.

The canvas moves with the right or middle mouse button or by scrolling, and zooms with
<kbd>Ctrl</kbd> and scrolling or a pinch; <kbd>Home</kbd> goes back to the start.

**Load** and **Save** under *File* read and write the circuit as a `.json` file. The Kirchhoff
mode uses the same files, see [Kirchhoff](kirchhoff.md).

## Ground

The voltages of the junctions are measured against ground. Without a **Ground** part, ground is
the negative terminal of the first voltage source, and a note under the canvas says so.

## Wires

A wire is ideal ($R = 0$) until its material, length and cross-section are all given; then its
resistance follows from them:

$$R = \rho_{20} \cdot \frac{l}{A} \cdot \bigl(1 + \alpha\,(\vartheta - 20\,^\circ\mathrm{C})\bigr)$$

| Material | $\rho$ at 20 °C in $\Omega\,\mathrm{mm^2/m}$ | $\alpha$ in 1/K |
|---|---|---|
| Copper | 0,0172 | 0,0039 |
| Aluminium | 0,0282 | 0,0039 |
| Silver | 0,0159 | 0,0038 |
| Gold | 0,0244 | 0,0034 |
| Iron | 0,1 | 0,0056 |
| Constantan | 0,49 | 0,00001 |
| Brass | 0,07 | 0,0015 |

Without a temperature, 20 °C is used and the wire says *at 20 °C*. **Resistance (fixed)** gives
the wire a resistance of its own instead. A cable is drawn as two wires, out and back: there is
no flag for the return conductor.

Ideal wires get no label on the canvas; their values show in the inspector.

## What the canvas shows

Each part has a label beside it with its name and its values. The buttons **U I P R** at the top
right choose which values are shown: voltage, current, power and resistance (voltage and current
to start with). The inspector on the right always shows all of them for the selected part.

- Values get the unit prefix that fits: $4\,\mathrm{mA}$ instead of $0{,}004\,\mathrm{A}$,
  $2\,\mathrm{kW}$ instead of $2000\,\mathrm{W}$.
- Resistors, wires and power loads count voltage and current along the real current, so they
  are positive whichever way the part is drawn. A source counts from its `+` terminal through
  itself, so a source that delivers power shows a negative current and power.
- A power load also reports its resistance, $U/I$.
- A label with no room beside its part moves to the next free spot, with a dashed line to it.

The values follow the **Precision** and the output settings at once. The circuit is solved again
after every change.

## Example: voltage drop on a cable

`test/circuits/cable.json`: a 230 V source feeds a 2 kW load through 100 m of 2,5 mm² copper
out and 100 m back.

| Part | $U$ | $I$ | $R$ |
|---|---|---|---|
| Cable C1 | $6{,}33116\ldots\,\mathrm{V}$ | $9{,}20227\ldots\,\mathrm{A}$ | $688\,\mathrm{m\Omega}$ |
| Load L1 | $217{,}33767\ldots\,\mathrm{V}$ | $9{,}20227\ldots\,\mathrm{A}$ | $23{,}61783\ldots\,\Omega$ |
| Source V1 | $230\,\mathrm{V}$ | $-9{,}20227\ldots\,\mathrm{A}$ | |

The load takes exactly $2\,\mathrm{kW}$; each half of the cable loses $58{,}26\ldots\,\mathrm{W}$.
A power load makes the circuit non-linear, so it is solved iteratively (Newton's method) to the
set precision.

## Values that cannot be found

Nothing is guessed. A value that cannot be worked out shows as **?**; hovering it gives the
reason, such as *the supply voltage is missing* or *depends on R1, R2*.

A circuit whose only source has no value is still solved **relative** to it: voltages as a share
of the source voltage, shown in amber. In `test/circuits/relative.json`, 1 kΩ and 3 kΩ in series
show $U = 25\,\%$ and $U = 75\,\%$, and the current as $250\,\mu\mathrm{S}$, per volt of the
source.

## What is refused

| Circuit | Message |
|---|---|
| A wire straight across a voltage source | *short circuit* |
| A part not connected to the rest | *Node … is not connected* |
| A negative value | *The value of R1 must not be negative.* |
| A power load larger than the source can deliver | *no solution: the load is too large* |

## Settings that matter here

- **Precision**: the significant digits of the values.
- **Exact as a root** and the other output settings: how the values are written.
- **Language**: the notes and the reasons.

See [Settings](../reference/settings.md).

## See also

- [Kirchhoff](kirchhoff.md): the same kind of circuit worked out with the junction and loop rules
- [Units and quantities](physics/index.md)
