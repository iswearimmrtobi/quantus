---
sidebar_position: 7
title: Circuits
---

# Circuits

**Electronics** mode replaces the calculator with a schematic editor for passive DC circuits.
Every change recalculates the circuit, and every part shows its voltage, current and power. The
solver is exact and never guesses a missing value.

## Build a circuit

1. Switch to **Electronics** in the top bar.
2. Drag parts from the **PARTS** palette onto the canvas, or click them.
3. Drag from one terminal to another to draw a wire.
4. Click a part to enter its values.

| Part | Values |
|---|---|
| **Resistor** | Resistance |
| **Wire** | Material, Length, Cross-section, Temperature, or a fixed Resistance |
| **Voltage source** | Voltage, Internal resistance |
| **Current source** | Current, Internal conductance |
| **Potentiometer** | Total resistance, Position (0 to 1) |
| **Power load** | Power |
| **Ground** | sets the reference potential |

Values are typed as in the calculator, with units: `230V`, `4{,}7k\Omega`, `2mm^2`. A value in the
wrong unit, such as a length in volts, is rejected.

<kbd>Space</kbd> or <kbd>R</kbd> rotates the selected parts, <kbd>Del</kbd> deletes them, and
dragging on empty canvas selects several. Without a **Ground** part, the negative terminal of the
first voltage source is the reference potential.

## Wires with resistance

A wire is ideal (0 Ω) until **Material**, **Length** and **Cross-section** are all given. Then
its resistance is

$$
R = \rho \cdot (1 + \alpha \cdot (T - 20\,°\mathrm{C})) \cdot \frac{L}{A}
$$

with the resistivity $\rho$ and temperature coefficient $\alpha$ of the material: copper,
aluminium, silver, gold, iron, constantan or brass. Without a **Temperature**, 20 °C is used and
the result says *at 20 °C*. A wire with only some of the values stays ideal and gets a note
naming what is missing. **Resistance (fixed)** overrides the calculation, for example with a
value from a datasheet.

There is no "out and back" switch: the return conductor is simply the second wire in the loop.

## Example: voltage drop on a cable

A 230 V source feeds a 10 Ω load through 100 m of copper cable with 2 mm², out and back:

| Part | Values |
|---|---|
| Voltage source | `230V` |
| Wire (out) | Copper, `100m`, `2mm^2` |
| Resistor | `10\Omega` |
| Wire (back) | Copper, `100m`, `2mm^2` |

Results (precision 5):

| | Voltage | Current |
|---|---|---|
| each wire | $16{,}877\,\mathrm{V}$ | $19{,}625\,\mathrm{A}$ |
| load | $196{,}25\,\mathrm{V}$ | $19{,}625\,\mathrm{A}$ |

So about 33.75 V, or 14.7 %, are lost in the cable.

## Unknown values

Nothing is guessed. A value that cannot be determined is shown greyed out as **Unknown**, with
the reason, such as *Unknown: the supply voltage is missing*. Without a source value, a purely linear
circuit still gives the ratios (the relative drop of 14.7 % above); absolute values stay
unknown.

Errors that stop the whole circuit, such as a short circuit, a floating node or a power load the
network cannot supply, are shown with their own message.

## Save and load

**FILE → Save** writes the circuit, with the positions of the parts, as a `.json` file; **Load**
reads one back. A broken file is reported and leaves the current circuit alone. There is no
autosave.

:::note Solving backwards
The engine can solve for one unknown from a target, for example the cross-section that keeps the
drop under 3 %. The schematic editor does not offer this yet; it is available through the
[engine API](../contributing/api.md) (`solve_circuit` with a `reverse` request).
:::

## See also

- [Known limits](../reference/limits.md#electronics-mode)
