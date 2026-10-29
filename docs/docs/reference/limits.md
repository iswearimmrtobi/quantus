---
sidebar_position: 5
title: Known limits
---

# Known limits

What Quantus does not do yet, or does only in part. Each item gives a clear error message in the
app.

## Calculator (Physics, Algebra, Imaginary)

- **No conversion to a unit of your choice.** Results stay in the units you typed, or are named
  when the arithmetic produced a product of units. To get a number in another unit, divide by
  it: `\frac{1\,\mathrm{yr}}{1\,\mathrm{d}}` gives $365{,}2425$.
- **Plain letters cannot be defined.** `x = 3` names a line; only subscripted names
  (`x_1 = 3`) are stored. Plain letters get a value from `\sum`, `\int`, `\lim`, `\frac{d}{dx}`
  or as the unknown of an equation.
- **Units win over letters** in Physics mode: `m`, `s`, `g`, `h` and so on cannot be the
  unknown of an equation. Inside `\sum`, `\int`, `\lim` and `\frac{d}{dx}` the bound letter
  wins in the body.
- **The unknown of an equation is a plain number.** `3x = 6m` is a dimension error;
  `3x\,\mathrm{m} = 6\,\mathrm{m}` works.
- **One unknown per equation.** Degree up to 2 is solved exactly, higher degrees numerically on
  $[-100, 100]$. Several unknowns need a system of equations.
- **Systems must be linear.** `x^2 + y = 1; x - y = 0` is refused; there is no numerical
  solver for non-linear systems. A system is always the whole input and cannot be part of a
  calculation, and its unknowns are plain numbers like those of a single equation.
- **Integrals need bounds** and are numerical (tanh-sinh), with at most 30 digits. Nested
  integrals work but are slow; improper integrals of exponentials take about a second.
  Differentials must stand at the top level of the integrand, not inside `\frac{…}{…}`.
- **Derivatives need a point**: `\left.\frac{d}{dx} f\right|_{x=2}`. No `\frac{d^2}{dx^2}`, no
  partial derivatives.
- **Limits** are numerical; a limit whose two sides disagree is an error, use `0^+` or `0^-`.
- **Matrices** only inside `\det`, up to 8×8.
- **No `\operatorname{…}`** functions beyond those in [Syntax](syntax.md).
- **siunitx macros** such as `\kilo\metre` are not supported; write `\qty{3}{km}`.

## Complex numbers (Imaginary mode)

Exact: `i^n`, `\sqrt{-a}` (principal root $i\sqrt{a}$), sums, products, quotients and integer
powers of $a + bi$, so $(3+4i)(1-2i) = 11 - 2i$. Also supported: `e^{a+bi}`, `z^w` (principal
branch), `\ln` and `\log` of negative or complex numbers, the trigonometric and hyperbolic
functions of complex arguments, `|z|`, `\Re`, `\Im`, `\arg` (in $(-\pi, \pi]$) and
`\overline{z}`. Quadratic equations with a negative discriminant have complex roots, and a
variable can hold a complex number (`z_1 = 3+4i`).

Not covered:

- `\arcsin`, `\arccos`, `\arctan` of a complex number
- equations of degree above two with complex roots
- derivatives and integrals of complex functions
- complex units: `2i\,\mathrm{m}` works, but $\mathrm{m}$ is not a complex quantity
- odd roots of negative numbers keep their real value: $\sqrt[3]{-8} = -2$
- the simplifier leaves $i$ alone, so `(x+i)(x-i)` with a free $x$ is shown as typed

## Analysis mode

Accepted: a function of one real variable built from numbers, $\pi$, $e$, $+ - \cdot /$,
powers, roots, $e^u$, $a^u$, `\exp`, `\ln`, `\log`, `\log_b`, the six trigonometric functions,
their inverses `\arcsin`, `\arccos`, `\arctan`, `\sinh`, `\cosh`, `\tanh`, $|u|$ and degrees,
nested in any way.

Rejected with an error that names the reason:

- **function families** with a second free letter ($x^2 + a$)
- piecewise functions, implicit or parametric curves, complex results
- units, sums, integrals, limits, derivatives, matrices, $n!$ and the other integer functions
- **domains made of periodic intervals**, such as $\ln(\sin x)$ (periodic gaps like those of
  $\tan x$ are fine)
- in the area task: **improper integrals** and areas between two curves

Results that are partial or approximate:

- Equations no school method solves are **scanned numerically** on $[-100, 100]$ (or in one
  period); the section is then marked *PARTIAL*. Such roots are accurate to about 15 digits,
  polynomial roots to the full precision.
- **Antiderivatives** come from a table, partial fractions, substitution and integration by
  parts (up to six nested rounds). Repeated irreducible quadratic factors
  ($\frac{1}{(x^2+1)^2}$) and integrands without an elementary antiderivative ($e^{x^2}$,
  $\frac{\sin x}{x}$) give *no elementary antiderivative found*. Every antiderivative shown has
  been checked by differentiation.
- **Limits** that no rule decides are sampled and shown with $\approx$, or as unknown.
- A sketch is cut off after **3 seconds**, each task after 2 seconds; the missing sections are
  marked *PARTIAL*.
- Simplification treats $\ln x$ or $\sqrt{3}$ as independent symbols, so a result can be correct
  without being the shortest form.
- Periodic families are listed in $[-P, P]$ (at most 12 members each), or $[-10, 10]$ when $f$
  itself is not periodic.

## Electronics mode

DC only, passive parts: resistors, wires, voltage and current sources, potentiometers and
constant-power loads. No diodes, transistors, capacitors or inductors, no AC. Reverse solving
finds one unknown at a time. Circuits are saved by hand, there is no autosave.
