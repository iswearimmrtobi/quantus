---
sidebar_position: 6
title: FAQ
---

# FAQ

### Why is my `m` a metre?

In Physics mode letters that match a unit are units. Give the quantity an index (`m_1`), or
switch to **Algebra** if you do not need units. See
[Concepts](../getting-started/concepts.md#variables-carry-an-index).

### Why does `2kg` give $2 \cdot g \cdot k$?

You are in Algebra, Imaginary or Analysis mode, where every letter is a variable. Write
`2\,\mathrm{kg}` or switch to **Physics**.

### Why was my variable not stored?

Only names with an index are stored: `m_1 = 2kg` defines $m_1$, `m = 2kg` only names the line.

### *The variable x_1 has no value yet*

You used $x_1$ before defining it. Define it first (`x_1 = 5m`), or check the **Variables**
panel (<kbd>Ctrl</kbd>+<kbd>,</kbd>) for the right name.

### Why is `1.5e3` not 1500?

`e` is Euler's number, so this is $1{,}5 \cdot e \cdot 3$. Write `1.5 \times 10^3`.

### Why does `\max(2,5)` give 2,5?

The comma is a decimal comma. Separate arguments with a semicolon: `\max(2; 5)`.

### *cannot be added directly because one of them has a zero point of its own*

Degrees Celsius have an offset, so $25\,°\mathrm{C} + 1\,\mathrm{K}$ is ambiguous. Use the same
unit on both sides.

### How do I convert to another unit?

There is no conversion command yet. Divide by the unit you want:
`\frac{72\,\mathrm{km/h}}{1\,\mathrm{m/s}}`. See [Known limits](limits.md).

### Why is the result a long decimal?

Quantus writes 50 significant digits by default. Lower **Settings → Precision**, or turn on
**Exact as a root** to keep $\sqrt{3}$ and $\pi$ symbolic.

### Why is a section of the curve sketch marked *PARTIAL*?

The time limit ran out or a numeric method was used. See
[Known limits](limits.md#analysis-mode).

### How do I get the working into my LaTeX document?

<kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>C</kbd> copies the whole calculation as an `align*` block.
