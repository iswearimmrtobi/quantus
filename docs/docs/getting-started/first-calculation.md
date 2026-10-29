---
sidebar_position: 2
title: Your first calculation
---

# Your first calculation

This tutorial takes five minutes. You define two quantities, calculate with them and copy the
result. Quantus starts in **Physics** mode, which is what you need here; the mode is shown in the
top bar.

## 1. Define a mass

Click into the input and type:

```latex
m_1 = 2kg
```

In the formula editor, `_` starts the index. Press <kbd>→</kbd> to leave the index before you
type `= 2kg`. Press <kbd>Enter</kbd>. Quantus answers with

$$
m_{1} = 2\,\mathrm{kg}
$$

and lists $m_1$ in the **Variables** panel (<kbd>Ctrl</kbd>+<kbd>,</kbd>). Because the label
`m_1` carries an index, the input is a **definition**: $m_1$ keeps its value for the whole
session.

## 2. Calculate with it

```latex
F = m_1 \cdot 9.81\frac{m}{s^2}
```

`*` gives a $\cdot$ and `/` a fraction. Quantus shows the working:

| | |
|---|---|
| SYMBOLIC | $F = m_{1} \cdot 9{,}81\,\frac{\mathrm{m}}{\mathrm{s}^{2}}$ |
| SUBSTITUTED | $F = 2\,\mathrm{kg} \cdot 9{,}81\,\frac{\mathrm{m}}{\mathrm{s}^{2}}$ |
| STEP | $F = 19{,}62\,\frac{\mathrm{kg} \cdot \mathrm{m}}{\mathrm{s}^{2}}$ |
| RESULT | $F = 19{,}62\,\mathrm{N}$ |

The units are multiplied with the numbers, and $\mathrm{kg \cdot m/s^2}$ is recognised as the
newton. `F` has no index, so it only names the line; it is not stored.

:::tip
You can type a decimal point or a decimal comma (`9{,}81`). Results are always written with a
decimal comma.
:::

## 3. Units are converted for you

```latex
v_1 = 72\frac{km}{h}
s_1 = 100m
t = \frac{s_1}{v_1}
```

Enter the three lines one after the other. The last one gives

$$
t = 5\,\mathrm{s}
$$

Quantus cancels $\mathrm{m}$ against $\mathrm{km}$ and $\mathrm{h}$ against $\mathrm{s}$ and
names the result in base units.

## 4. Mistakes are caught

```latex
3m + 2s
```

> **ERROR** Incompatible units: m (length) and s (time) cannot be added.

## 5. Copy the result

- <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>C</kbd> copies the result as LaTeX.
- <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>C</kbd> copies the whole calculation as an `align*` block,
  ready for your document.

## Next

- [Concepts](concepts.md) explains modes, letters and variables.
- [Units and quantities](../guides/physics.md) goes deeper into Physics mode.
