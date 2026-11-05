---
sidebar_position: 1
title: Entering formulas
---

# Entering formulas

There are two ways to type: the **formula editor**, which is quick, and the **LaTeX source**,
which is exact. Both feed the same engine, and you can switch at any time without losing the
input.

## The formula editor

The input starts in the formula editor (the button at its right end shows **f(x)**). It sets
the formula as you type:

| You type | You get |
|---|---|
| `/` | a fraction; the next keys go into the denominator |
| `*` | $\cdot$ |
| `^`, `_` | an exponent, an index |
| <kbd>→</kbd> | leaves the exponent, index or bracket you are in |
| `2kg` | a number with a unit |
| <kbd>Enter</kbd> | evaluate |

So `m_1 = 2kg` is typed as <kbd>m</kbd> <kbd>_</kbd> <kbd>1</kbd> <kbd>→</kbd> <kbd>=</kbd>
<kbd>2</kbd> <kbd>k</kbd> <kbd>g</kbd>. Without the <kbd>→</kbd>, `= 2kg` would land in the
index.

Under the input you always see the LaTeX source of what you typed, with a **copy** button. While
the input is empty, the same line shows an example that fits the current mode, such as
`F = 2kg \cdot 10\frac{m}{s^2}` in Physics.

## The LaTeX source

Press <kbd>\\</kbd> (or <kbd>Ctrl</kbd>+<kbd>L</kbd>, or click **f(x)**) to switch to the source.
The current formula is carried over as LaTeX, and the backslash you pressed starts a command, so
`\` `sqrt` goes straight on. From then on what you write is what you get:

```latex
\qty{12}{cm} + 3mm
```

Underneath, a live preview sets the formula; it grows with the formula's height. Use
<kbd>Shift</kbd>+<kbd>Enter</kbd> for a line break in long inputs, <kbd>Enter</kbd> to evaluate
and <kbd>Esc</kbd> to go back to the formula editor, which takes the source with it.

:::tip
Use the source for anything the editor makes awkward: `\sqrt[3]{27}`, `\binom{5}{2}`,
`\left.\frac{d}{dx} x^3\right|_{x=2}`, matrices.
:::

## Errors

When the input cannot be read, the result area shows **ERROR** with the message and, for parser
errors, the **Position** in the input:

```latex
72\frac{km}{h} \to \frac{m}{s}
```

> **ERROR** Unsupported command '\to'. (at position 16: '\to \frac{m}…')

Most messages say what to write instead:

> An integral needs limits, as in \int_{0}^{1} x \, dx.

## See also

- [Syntax](../reference/syntax.md): every command the input understands
- [Keyboard shortcuts](../reference/shortcuts.md)
