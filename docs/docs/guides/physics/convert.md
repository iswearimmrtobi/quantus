---
sidebar_position: 3
title: Converting units
---

# Converting units

The arrow `▶` writes a value in the unit you name after it:

```latex
72\frac{km}{h} \blacktriangleright \frac{m}{s}
```

$$
20\,\frac{\mathrm{m}}{\mathrm{s}}
$$

## Write the arrow

- In the LaTeX source (<kbd>\\</kbd> or <kbd>Ctrl</kbd>+<kbd>L</kbd>), type `\blacktriangleright`,
  `\triangleright` or the character `▶` itself.
- Or pick the unit in the **Convert** tab of the **Library** window (<kbd>Ctrl</kbd>+<kbd>K</kbd>):
  a click puts in `▶` and the unit at the cursor.
- Or use the **quick search** (<kbd>Ctrl</kbd>+<kbd>F</kbd>): type part of the unit's name, such as
  `mile` or `zoll`, and <kbd>Enter</kbd> puts in `▶` and the unit.

In the Library and the quick search, <kbd>Alt</kbd> puts in the unit alone, without the arrow.
The **Convert** tab sorts the units into the categories of the Casio fx-991DE X's CONV menu
(length, area, volume, mass, speed, pressure, energy, power, temperature, …).

## Examples

| Input | Result |
|---|---|
| `5cm \blacktriangleright in` | $1{,}968503937\ldots\,\mathrm{in}$ |
| `100km \blacktriangleright mi` | $62{,}137119223\ldots\,\mathrm{mi}$ |
| `1kWh \blacktriangleright \mathrm{J}` | $3600000\,\mathrm{J}$ |
| `2h \blacktriangleright \mathrm{min}` | $120\,\mathrm{min}$ |
| `25°C \blacktriangleright \mathrm{K}` | $298{,}15\,\mathrm{K}$ |
| `50°F \blacktriangleright ^\circ\mathrm{C}` | $10\,^\circ\mathrm{C}$ |
| `1N \blacktriangleright \mathrm{kg}\cdot\frac{\mathrm{m}}{\mathrm{s}^2}` | $1\,\frac{\mathrm{kg} \cdot \mathrm{m}}{\mathrm{s}^{2}}$ |

The target can be a compound unit such as $\mathrm{km/h}$ or $\mathrm{kg \cdot m/s^2}$.
Temperatures are converted with their zero point, so $50\,^\circ\mathrm{F}$ is $10\,^\circ\mathrm{C}$
and not a difference of $27{,}8\,\mathrm{K}$. The letters after the arrow are always read as a unit,
so `in` is the inch there even when **t, u and in as units** is off.

The value before the arrow can be anything that has a unit: a variable, `\mathrm{Ans}` or a whole
calculation.

```latex
s_1 = 3km
s_1 \blacktriangleright \mathrm{m}
```

$$
3000\,\mathrm{m}
$$

## In the other modes

The arrow works in every mode. Outside Physics, units are only known in `\mathrm{…}`, on both sides
of the arrow: `5\mathrm{cm} \blacktriangleright \mathrm{in}`.

## What is refused

| Input | Message |
|---|---|
| `3m \blacktriangleright \mathrm{s}` | Cannot convert m (length) to s (time). |
| `2 \blacktriangleright \mathrm{m}` | Cannot convert 1 to m (length). |
| `3m \blacktriangleright 2` | After ▶ comes a unit alone, such as $\mathrm{in}$ or $\frac{\mathrm{km}}{\mathrm{h}}$. |

## See also

- [Units and quantities](index.md): how results get their unit without an arrow
- [Units and constants](../../reference/units-and-constants.md): every unit and its symbol
- [Syntax](../../reference/syntax.md)
