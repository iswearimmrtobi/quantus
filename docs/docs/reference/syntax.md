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
| `0{,}1\overline{6}`, `0{,}\overline{3}` | a recurring decimal, the period overlined; exact | $\frac{1}{6}$, $\frac{1}{3}$ |
| `30^\circ`, `30°` | an angle in degrees | $30$ with degrees, $\frac{\pi}{6}$ with radians |
| `2^\circ 20' 30''`, `2°20′30″` | degrees, minutes, seconds (also `^{\prime}`, `^{\prime\prime}`) | $2{,}341\overline{6}$ with degrees |

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
| `\sin`, `\cos`, `\tan`, `\cot`, `\sec`, `\csc` | in the [angle unit](settings.md#general), or marked: `30^\circ` in degrees, `2\,\mathrm{rad}` in radians; exact where the value is rational ($\sin 30^\circ = 0{,}5$) |
| `\arcsin`, `\arccos`, `\arctan`, `\sin^{-1}` | result in the angle unit |
| `\sinh`, `\cosh`, `\tanh`, `\coth` | |
| `\operatorname{arsinh}`, `\operatorname{arcosh}`, `\operatorname{artanh}`, `\sinh^{-1}` | the area functions; also `arcsinh`, `asinh` and so on |
| `\exp`, `e^{x}` | |
| `\ln`, `\log` (base 10), `\lg`, `\log_b` | arguments must be positive; $\log_2 8 = 3$ |
| `\max(a; b)`, `\min(a; b)` | separate the arguments with `;`, since `,` is the decimal comma |
| `\binom{n}{k}`, `\dbinom`, `\tbinom` | exact; non-negative whole numbers |
| `\operatorname{nPr}(n; r)` | permutations, $\frac{n!}{(n-r)!}$ |
| `\gcd(a; b)`, `\operatorname{ggT}(a; b)` | greatest common divisor of whole numbers, any number of arguments |
| `\operatorname{lcm}(a; b)`, `\operatorname{kgV}(a; b)` | least common multiple |
| `\operatorname{Int}(x)`, `\operatorname{Intg}(x)` | the whole part toward zero; the floor, like `\lfloor x \rfloor` |
| `\operatorname{Rnd}(x)` | rounded to what the [number format](settings.md#how-results-are-written) shows: Fix and Sci digits, ten digits otherwise |
| `\operatorname{RndFix}(x; n)` | rounded to n decimal places |
| `\operatorname{divR}(a; b)` | whole-number division; as the whole input `Q = 3,\; R = 1`, inside a calculation the quotient |
| `\det\begin{pmatrix} … \end{pmatrix}`, `vmatrix` | determinant, exact, up to 8×8 |
| `\Re`, `\Im`, `\arg`, `\overline{z}` | Imaginary mode: real part, imaginary part, argument (in the angle unit), conjugate |
| `r \angle \theta` | Imaginary mode: the complex number with modulus r and angle θ, in the angle unit unless marked (`2\angle 60^\circ`); the angle runs to the next operator |

## Calculus

| Input | Notes |
|---|---|
| `\sum_{k=1}^{10} k` | exact, up to 1,000,000 terms. The body is one term: `\sum k + 1` is $(\sum k) + 1$ |
| `\prod_{k=1}^{5} k` | the product of the terms, like `\sum`; an empty product is 1 |
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
| `x^3 - 2x - 5 = 0` | $x = 2{,}0945514815\ldots$ (by Cardano) |
| `x^4 - 5x^2 + 4 = 0` | $x = -2 \lor x = -1 \lor x = 1 \lor x = 2$ |

One unknown per equation. Polynomials with rational coefficients are solved exactly up to degree
4 and numerically on the whole real line beyond; other equations numerically on
$[-100, 100]$.

A **system of linear equations** has several unknowns and is the whole input:

| Input | Result |
|---|---|
| `2x + y = 5; x - y = 1` | $x = 2 \land y = 1$ |
| `\begin{cases} 2x + y = 5 \\ x - y = 1 \end{cases}` | the same |

`&` for alignment (`2x + y &= 5`) is allowed. A comma cannot separate equations, it is the
decimal separator.

An equation **without an unknown** is checked instead: `2 + 3 = 5` gives $\text{true}$ (German
$\text{wahr}$, value 1), `2 + 3 = 6` $\text{false}$ (0). Comparisons work the same way and chain:

| Input | Result |
|---|---|
| `1 < 2`, `\pi > 3{,}2` | $\text{true}$, $\text{false}$ |
| `\le`, `\leq`, `≤`, `\ge`, `≥`, `\neq`, `≠`, `\lt`, `\gt` | the other comparisons |
| `1 < 2 \le 2 \neq 3`, `a = b = c` | a chain is true when every link is |

Equal means equal to 40 digits, so $\sqrt{2}^2 = 2$ holds; both sides need the same dimension.
The mode `verify` (Prüfen, engine only for now) checks every input this way, an equation too, and
reads `a_1 = 3` as a comparison rather than a definition.

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
| `25°C`, `77°F`, `30^\circ` | degree Celsius, degree Fahrenheit, degree of angle |
| `5\,\mathrm{cm} \blacktriangleright \mathrm{in}`, `▶` | the value written in the unit after the arrow, in every mode; the dimensions must match. Temperatures convert with their offset: `50°F ▶ ^\circ\mathrm{C}` is $10\,^\circ\mathrm{C}$ |

The full list is in [Units and constants](units-and-constants.md).

## Matrices

The mode `matrix` (engine only for now) computes with matrices as values, exactly, up to 10 × 10.
`A = \begin{pmatrix} 1 & 2 \\ 3 & 4 \end{pmatrix}` stores a matrix under any name (`A`, `M_1`);
later inputs use it by that name.

| Input | Meaning |
|---|---|
| `A + B`, `A - B` | sum and difference of matrices of the same size |
| `A \cdot B`, `AB`, `2A`, `\frac{A}{2}` | matrix product, a number times a matrix |
| `A^{2}`, `A^{0}`, `A^{-2}` | powers of a square matrix; a negative power is one of the inverse |
| `A^{-1}` | the inverse, by Gauss–Jordan on $[A \mid I]$, with its steps |
| `A^{T}`, `A^{\top}` | the transpose |
| `\det A` | the determinant, as the product of the pivots, with its steps |
| `\operatorname{Identity}(n)` | the $n \times n$ identity matrix |
| `\operatorname{ref}(A)`, `\operatorname{rref}(A)` | a row echelon form (by the school method, without fractions) and the reduced one, with their steps |
| `\|A\|` | the absolute value of each entry |

## Basis-N

The mode `baseN` works with whole numbers in the [number base](settings.md#general) set, as
32-bit two's complement: from −2147483648 to 2147483647 in decimal, any 32-bit pattern in the
other bases (`FFFFFFFF` is −1). The result comes in the other three bases as its forms.

| Input | Meaning |
|---|---|
| `1F`, `\mathrm{1F}` | a number in the set base; letters A–F in hexadecimal |
| `\mathrm{h}1F`, `\mathrm{d}10`, `\mathrm{b}101`, `\mathrm{o}17` | one number in another base; plain `h1F` works where the letter is no digit of the set base |
| `+`, `-`, `\cdot`, `\div`, `\frac{a}{b}` | arithmetic; division truncates toward zero |
| `\operatorname{and}`, `or`, `xor`, `xnor` | bitwise operations, written plain or in `\operatorname{}` |
| `\operatorname{Not}(x)`, `\operatorname{Neg}(x)` | bitwise not (one's complement), negation (two's complement) |

Signs, `Not` and `Neg` bind first, then products, then sums, then `and`, then `or`, `xor` and
`xnor`.

## Ignored layout

`\displaystyle`, `\textstyle`, `\limits`, `\quad`, `\qquad` and `\,` are accepted and change
nothing.
