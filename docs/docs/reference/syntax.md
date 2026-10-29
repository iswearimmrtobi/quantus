---
sidebar_position: 1
title: Syntax
---

# Syntax

Everything the input understands, written as you would type it in the LaTeX source. The formula
editor produces the same LaTeX. Results in this page are from Physics mode unless noted.

## Numbers

| Input | Meaning | Result |
|---|---|---|
| `2.5` or `2{,}5` | decimal point or decimal comma | $2{,}5$ |
| `1.5 \times 10^3`, `1{,}5 \cdot 10^3` | scientific notation | $1500$ |
| `20\%` | percent, a unit | $20\,\%$ |
| `\pi`, `e` | the constants $\pi$ and $e$ | |
| `i` | the imaginary unit, in Imaginary mode only | |

:::caution
`1.5e3` is **not** scientific notation: it is $1{,}5 \cdot e \cdot 3$. Write `1.5 \times 10^3`.
:::

## Operators

| Input | Meaning |
|---|---|
| `+`, `-` | sum, difference |
| `\cdot`, `\times`, `*`, `\ast`, or nothing (`2x`) | product |
| `/`, `\div`, `\frac{a}{b}`, `\dfrac`, `\tfrac` | quotient |
| `a^b`, `a^{b}` | power |
| `\sqrt{x}`, `\sqrt[n]{x}` | square root, n-th root |
| `n!` | factorial |
| `a \bmod b` | remainder |
| `( )`, `[ ]`, `\left( \right)` | brackets |
| `\|x\|`, `\left\|x\right\|`, `\lvert x \rvert` | absolute value (keeps the unit) |
| `\lfloor x \rfloor`, `\lceil x \rceil` | floor, ceiling |

Operators bind as usual: powers before products, products and quotients from left to right
before sums.

## Functions

| Input | Notes |
|---|---|
| `\sin`, `\cos`, `\tan`, `\cot`, `\sec`, `\csc` | radians, or degrees with `30^\circ`; exact where the value is rational ($\sin 30^\circ = 0{,}5$) |
| `\arcsin`, `\arccos`, `\arctan`, `\sin^{-1}` | result in radians |
| `\sinh`, `\cosh`, `\tanh` | |
| `\exp`, `e^{x}` | |
| `\ln`, `\log` (base 10), `\lg`, `\log_b` | arguments must be positive; $\log_2 8 = 3$ |
| `\max(a; b)`, `\min(a; b)` | separate the arguments with `;`, since `,` is the decimal comma |
| `\binom{n}{k}`, `\dbinom`, `\tbinom` | exact; non-negative whole numbers |
| `\det\begin{pmatrix} … \end{pmatrix}`, `vmatrix` | determinant, exact, up to 8×8 |
| `\Re`, `\Im`, `\arg`, `\overline{z}` | Imaginary mode: real part, imaginary part, argument, conjugate |

## Calculus

| Input | Notes |
|---|---|
| `\sum_{k=1}^{10} k` | exact, up to 1,000,000 terms. The body is one term: `\sum k + 1` is $(\sum k) + 1$ |
| `\int_0^1 x^2\,dx` | numerical, at most 30 digits. Differentials `dx`, `d x`, `\mathrm{d}x` |
| `\int_1^{\infty} \frac{1}{x^2}\,dx` | improper integrals with `\infty` bounds |
| `\lim_{x \to 0} f`, `\lim_{x \to 0^+} f`, `\lim_{x \to \infty} f` | two-sided, one-sided, at infinity; `\rightarrow` works too |
| `\left.\frac{d}{dx} x^3\right\|_{x=2}` | derivative at a point, symbolic |

An integral needs bounds and a derivative needs a point; for antiderivatives and derivatives as
functions use [Curve sketching](../guides/analysis.md).

## Equations

| Input | Result |
|---|---|
| `2x + 3 = 11` | $x = 4$ |
| `x^2 = 4` | $x = -2 \lor x = 2$ |
| `x^3 - 2x - 5 = 0` | $x = 2{,}0945514815\ldots$ (numerical) |

One unknown per equation. Degree up to 2 is solved exactly, higher degrees numerically on
$[-100, 100]$.

## Labels and variables

| Input | Meaning |
|---|---|
| `m_1 = 2kg` | defines the session variable $m_1$ |
| `F = m_1 \cdot g_n` | names the line $F$ |
| `\omega_0`, `v_{max}` | any letter or Greek letter with an index is a variable name |

Greek letters: `\alpha` … `\omega`, the capitals that differ from Latin ones (`\Gamma`,
`\Delta`, `\Theta`, `\Lambda`, `\Xi`, `\Pi`, `\Sigma`, `\Upsilon`, `\Phi`, `\Psi`, `\Omega`)
and the variants `\varepsilon`, `\vartheta`, `\varphi`.

## Units

| Input | Notes |
|---|---|
| `2kg`, `9.81\frac{m}{s^2}` | bare letters after a number are units (Physics mode) |
| `2\,\mathrm{kg}`, `3\text{ m}` | explicit, works in every mode |
| `\qty{12}{cm}`, `\SI{12}{cm}`, `\num{12}`, `\unit{cm}`, `\si{cm}` | siunitx; the unit is written with symbols, not `\kilo\metre` |
| `25°C`, `30^\circ` | degree Celsius, degree of angle |

The full list is in [Units and constants](units-and-constants.md).

## Ignored layout

`\displaystyle`, `\textstyle`, `\limits`, `\quad`, `\qquad` and `\,` are accepted and change
nothing.
