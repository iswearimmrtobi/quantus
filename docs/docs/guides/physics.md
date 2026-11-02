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

## Write a task up: given, wanted, solution

<kbd>Ctrl</kbd>+<kbd>G</kbd> (View → Writeup: given, wanted, solution) writes the whole session up the way a physics
solution is written at school (*Gegeben, Gesucht, Lösung*). The given values are top left, the
wanted quantities top right, and the solution is below them. Take this session:

```latex
R = \frac{10V}{0{,}5A}
P = 10V \cdot 0{,}5A
```

It is written up as:

- **Given:** $U = 10\,\mathrm{V}$, $I = 0{,}5\,\mathrm{A}$
- **Wanted:** $R$, $P$
- **Solution**, one block for each quantity:
  - By Ohm's law: $U = R \cdot I$
  - Solved for R: $R = \frac{U}{I}$
  - Inserting the values: $R = \frac{10\,\mathrm{V}}{0{,}5\,\mathrm{A}}$
  - $R = 20\,\Omega$
  - Unit check: $\left[R\right] = \frac{1\,\mathrm{V}}{1\,\mathrm{A}} = 1\,\Omega$

### Laws

Quantus recognises the law from the units and powers of the values. It knows these laws:

- **Electricity:** Ohm's law, electric power (also as $R I^2$ and $U^2/R$), electric work,
  charge and the resistance of a wire.
- **Mechanics:** speed, acceleration, uniformly accelerated motion, Newton's second law,
  weight, density, pressure, potential and kinetic energy, mechanical work, power and
  momentum.
- **Waves:** frequency and the speed of a wave.
- **Heat:** heat.

An acceleration near $9{,}81\,\mathrm{m/s^2}$ (or `g_n`) makes a weight rather than $F = m \cdot a$.
A formula that is not in the list starts with *It holds:* instead.

### Names

- **Variables keep their names.** With `U_1 = 10V`, `I_1 = 0{,}5A` and
  `R_1 = \frac{U_1}{I_1}`, the rearranged law reads $R_1 = \frac{U_1}{I_1}$.
- **Typed-in values get the law's symbol.** If a symbol is already taken by a different value,
  the new value gets an index ($U_2$). The same value used again keeps its name, so it is listed
  only once.
- **A label names the result.** Without a label, the result takes the law's symbol.

### Several steps

- **Earlier results:** a result that a later line uses, by its variable or as `\mathrm{Ans}`, is
  a step on the way. It gets its own block but is not listed as wanted.
- **Sums:** a sum is worked out first. $\frac{230V}{10\Omega + 20\Omega}$ gives
  $R_{ges} = R_1 + R_2$, then $I = \frac{U}{R_{ges}}$.
- **Parts with a law of their own:** when no law fits the whole input but a part of it does, that
  part comes first. $\frac{2kg \cdot 9{,}81\frac{m}{s^2}}{0{,}5m^2}$ is a weight first, then a
  pressure.

### Copy as LaTeX or Markdown

**Copy as LaTeX** copies the whole writeup. It compiles in any document that loads `amsmath`.
**Copy as Markdown** uses simpler LaTeX for notes that render math, such as Obsidian, GitHub or
Typora. Given and wanted become a table. Each solution block is one `$$…$$` formula with an
`aligned` environment, so the equations line up at their `=`, and the sentences are `\text`
rows. It has no `align*` and no `\left( \right)`. In both copies the result is underlined
twice, as in the window.
The unit check always uses SI units, even when the values have prefixes such as kV or mA.

## See also

- [Units and constants](../reference/units-and-constants.md)
- [Concepts](../getting-started/concepts.md): why letters become units
