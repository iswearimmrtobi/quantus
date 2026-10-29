---
sidebar_position: 2
title: Units and quantities
---

# Units and quantities

In **Physics** mode, numbers carry units. Quantus checks that they fit, converts between them
and names the result.

## Calculate with units

Write the unit straight after the number:

```latex
\frac{1}{2} \cdot 3kg \cdot (4\frac{m}{s})^2
```

| | |
|---|---|
| SYMBOLIC | $\frac{1}{2} \cdot 3\,\mathrm{kg} \cdot (4\,\frac{\mathrm{m}}{\mathrm{s}})^{2}$ |
| STEP | $24\,\frac{\mathrm{kg} \cdot \mathrm{m}^{2}}{\mathrm{s}^{2}}$ |
| RESULT | $24\,\mathrm{J}$ |

Next to the result Quantus shows the name of the quantity (*energy*) and its dimension.

## Units are converted to a common one

```latex
3km + 200m
```

| | |
|---|---|
| SYMBOLIC | $3\,\mathrm{km} + 200\,\mathrm{m}$ |
| STEP | $3\,\mathrm{km} + 0{,}2\,\mathrm{km}$ |
| RESULT | $3{,}2\,\mathrm{km}$ |

The first unit sets the target. Units you typed yourself are kept: `72\frac{km}{h}` stays in
$\mathrm{km/h}$. Only a product the arithmetic made, such as $\mathrm{kg \cdot m^2/s^2}$, is
replaced by its named unit.

:::note
There is no command to convert into a unit of your choice yet. Divide by that unit instead:
`\frac{72\,\mathrm{km/h}}{1\,\mathrm{m/s}}` gives $20$.
:::

## Work with variables

Define the quantities once with an indexed name, then use them:

```latex
U_1 = 230V
R_1 = 4{,}7k\Omega
P = U_1 \cdot \frac{U_1}{R_1}
```

| | |
|---|---|
| SYMBOLIC | $P = U_{1} \cdot \frac{U_{1}}{R_{1}}$ |
| SUBSTITUTED | $P = 230\,\mathrm{V} \cdot \frac{230\,\mathrm{V}}{4{,}7\,\mathrm{k\Omega}}$ |
| STEP | $P = 11255{,}319\ldots\,\frac{\mathrm{V}^{2}}{\mathrm{k\Omega}}$ |
| RESULT | $P = 11{,}255319\ldots\,\mathrm{W}$ |

Lower **Settings → Precision** if 50 digits are more than you need.

## Physical constants

The CODATA constants have reserved names, such as `c_0`, `g_n`, `k_B` or `\hbar`:

```latex
E = m_e \cdot c_0^2
```

$$
E = 8{,}18710577682388596779369446 \cdot 10^{-14}\,\mathrm{J}
$$

The full list is in [Units and constants](../reference/units-and-constants.md#physical-constants).

## Dimension errors

Quantus refuses what does not fit physically, and says why:

| Input | Message |
|---|---|
| `3m + 2s` | Incompatible units: m (length) and s (time) cannot be added. |
| `2^{3m}` | The exponent must be dimensionless but carries the unit m (length). |
| `25°C + 1K` | ℃ (temperature) and K (temperature) cannot be added directly because one of them has a zero point of its own. |

## Angles

`30^\circ` is in degrees, everything else in radians: `\sin(30^\circ) \cdot 10N` gives
$5\,\mathrm{N}$.

## See also

- [Units and constants](../reference/units-and-constants.md)
- [Concepts](../getting-started/concepts.md): why letters become units
