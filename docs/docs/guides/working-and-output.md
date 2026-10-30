---
sidebar_position: 9
title: Working and output
---

# Working and output

Every result comes with its working. You choose how much of it you see, and you can copy all of
it into a LaTeX document.

## Reading the working

The steps sit between the input and the result. Each line has a label:

| Label | Line |
|---|---|
| **SYMBOLIC** | the formula as you entered it |
| **SUBSTITUTED** | the variables replaced by their values |
| **STEP** | one rule, with its name |
| **RESULT** | the result |
| **DOMAIN** | where the result equals the input, when a factor was cancelled |
| **NOTE** | a remark, for example why an expression is shown as typed |

<kbd>Ctrl</kbd>+<kbd>J</kbd> hides or shows the steps.

## Step detail

**Settings → Step detail** sets how many steps you see. Each level shows everything the levels
before it show. For `x^2 - 6x + 3` in Algebra mode:

| Level | Steps |
|---|---|
| **None** | only the result $(x - 3)^{2} - 6$ |
| **Brief** | $(x - 3)^{2} - 6$ — *Complete the square: add and subtract 9, then apply the binomial formula* |
| **Standard** *(default)* | $x^{2} - 6x + 9 - 9 + 3$ — *add and subtract 9* <br/> $(x - 3)^{2} - 9 + 3$ — *Second binomial formula* <br/> $(x - 3)^{2} - 6$ — *Add the constants* |
| **Detailed** | adds $(x^{2} - 6x + 9) - 9 + 3$ — *Group the square* |
| **Maximal** | adds the bookkeeping: signs, the order of terms, domain notes, plain arithmetic |

*Standard* is what you would write in a notebook. *Detailed* adds every intermediate line, such
as the formula with the values put in or a factor written out before it is cancelled.

## Cancelled factors

When a factor is cancelled, the step before shows which one. With
**Strike out cancelled factors** on, it is struck through; off, it is shown in colour.

## Copy

| Key | Copies |
|---|---|
| **copy** under the input | the input as LaTeX |
| <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>C</kbd> | the result as LaTeX |
| <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>C</kbd> | the whole calculation as an `align*` block |

The `align*` block of the first calculation in [Your first calculation](../getting-started/first-calculation.md):

```latex
\begin{align*}
  F = m_{1} \cdot 9{,}81\,\frac{\mathrm{m}}{\mathrm{s}^{2}} \\
  F = 2\,\mathrm{kg} \cdot 9{,}81\,\frac{\mathrm{m}}{\mathrm{s}^{2}} \\
  F = 19{,}62\,\frac{\mathrm{kg} \cdot \mathrm{m}}{\mathrm{s}^{2}} \\
  F = 19{,}62\,\mathrm{N}
\end{align*}
```

It needs `amsmath`. The decimal comma is written as `{,}`, so it does not add space.

## Precision

**Settings → Precision** sets the significant digits of decimal results (default 50, up to
10,000). The current value is shown in the status bar. Numerical results, such as integrals,
show at most as many digits as are reliable.

## See also

- [Settings](../reference/settings.md) for how results are written (binomials, denominators,
  roots)
