---
sidebar_position: 2
title: Given, wanted, solution
---

# Given, wanted, solution

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

## Laws

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

## Names

- **Variables keep their names.** With `U_1 = 10V`, `I_1 = 0{,}5A` and
  `R_1 = \frac{U_1}{I_1}`, the rearranged law reads $R_1 = \frac{U_1}{I_1}$.
- **Typed-in values get the law's symbol.** If a symbol is already taken by a different value,
  the new value gets an index ($U_2$). The same value used again keeps its name, so it is listed
  only once.
- **A label names the result.** Without a label, the result takes the law's symbol.

## Several steps

- **Earlier results:** a result that a later line uses, by its variable or as `\mathrm{Ans}`, is
  a step on the way. It gets its own block but is not listed as wanted.
- **Sums:** a sum is worked out first. $\frac{230V}{10\Omega + 20\Omega}$ gives
  $R_{ges} = R_1 + R_2$, then $I = \frac{U}{R_{ges}}$.
- **Parts with a law of their own:** when no law fits the whole input but a part of it does, that
  part comes first. $\frac{2kg \cdot 9{,}81\frac{m}{s^2}}{0{,}5m^2}$ is a weight first, then a
  pressure.

## Copy as LaTeX or Markdown

**Copy as LaTeX** copies the whole writeup. It compiles in any document that loads `amsmath`.
**Copy as Markdown** uses simpler LaTeX for notes that render math, such as Obsidian, GitHub or
Typora. Given and wanted become a table. Each solution block is one `$$…$$` formula with an
`aligned` environment, so the equations line up at their `=`, and the sentences are `\text`
rows. It has no `align*` and no `\left( \right)`. In both copies the result is underlined
twice, as in the window.
The unit check always uses SI units, even when the values have prefixes such as kV or mA.

## See also

- [Units and quantities](index.md)
- [Units and constants](../../reference/units-and-constants.md)
