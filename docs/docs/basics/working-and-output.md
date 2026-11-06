---
sidebar_position: 2
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

## Forms of a result

Under the result, buttons switch between the forms it has: a fraction as $\frac{7}{3}$, mixed
as $2\frac{1}{3}$, as the recurring decimal $2{,}\overline{3}$ and as the decimal; a whole
number in prime factors (**FACT**); a root or π exact and as the decimal; a complex number as
$a + bi$ and $r \angle \theta$; an angle in degrees, minutes and seconds; in Basis-N the other
three bases. <kbd>Ctrl</kbd>+<kbd>D</kbd> steps to the next one.
Which form comes first is set in [Settings](../reference/settings.md#numbers).

## ENG

**ENG** (<kbd>Ctrl</kbd>+<kbd>E</kbd>) writes the result in engineering notation: with an
exponent that is a multiple of three, as on a Casio calculator. Each further press moves the
exponent up by three and the decimal point to the left. **←ENG**
(<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>E</kbd>) moves it the other way; pressed first, it starts
one step below ENG. Both buttons are next to the forms under the result.

For the result $12345$:

| Keys | Shows |
|---|---|
| ENG | $12{,}345 \cdot 10^{3}$ |
| ENG, ENG | $0{,}012345 \cdot 10^{6}$ |
| ←ENG | $12345 \cdot 10^{0}$ |
| ←ENG, ←ENG | $12345000 \cdot 10^{-3}$ |

ENG works on the decimal value of every result, also a fraction or a root, but not in Base-N or
Verify. A unit stays behind the number ($4{,}7 \cdot 10^{3}\,\mathrm{\Omega}$), and a complex
number gets each part stepped on its own:
$1{,}5 \cdot 10^{3} + (2{,}5 \cdot 10^{3})i$. A new result, another form or <kbd>Ctrl</kbd>+<kbd>D</kbd>
ends ENG.

With **Settings → Decimal prefixes** on, a result without a unit gets a prefix instead of the
power of ten: $12{,}345\,\mathrm{k}$, $47\,\mathrm{\mu}$, and $12345$ for the exponent 0.
A result with a unit keeps the power of ten, so the unit you see is still the one of the result.

## Copy

| Key | Copies |
|---|---|
| **copy** under the input | the input as LaTeX |
| <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>C</kbd> | the result as LaTeX, in the form chosen |
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
