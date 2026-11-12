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
| `n!` | factorial; for an $n$ that is not whole $\Gamma(n + 1)$, so $(\frac{1}{2})! = \frac{\sqrt{\pi}}{2}$ |
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
| `\Gamma(x)` | the Gamma function, with a bracket right after it (`\Gamma` alone is a letter): exact at whole numbers and halves ($\Gamma(\frac{5}{2}) = \frac{3}{4}\sqrt{\pi}$), otherwise to the full precision; poles at $0, -1, -2, \ldots$ |
| `\operatorname{nPr}(n; r)` | permutations, $\frac{n!}{(n-r)!}$ |
| `\gcd(a; b)`, `\operatorname{ggT}(a; b)` | greatest common divisor of whole numbers, any number of arguments |
| `\operatorname{lcm}(a; b)`, `\operatorname{kgV}(a; b)` | least common multiple |
| `\operatorname{Int}(x)`, `\operatorname{Intg}(x)` | the whole part toward zero; the floor, like `\lfloor x \rfloor` |
| `\operatorname{Rnd}(x)` | rounded to what the [number format](settings.md#numbers) shows: Fix and Sci digits, ten digits otherwise |
| `\operatorname{RndFix}(x; n)` | rounded to n decimal places |
| `\operatorname{divR}(a; b)` | whole-number division; as the whole input `Q = 3,\; R = 1`, inside a calculation the quotient |
| `\det\begin{pmatrix} … \end{pmatrix}`, `vmatrix` | determinant, exact, up to 8×8 |
| `\Re`, `\Im`, `\arg`, `\overline{z}` | Imaginary mode: real part, imaginary part, argument (in the angle unit), conjugate |
| `r \angle \theta` | Imaginary mode: the complex number with modulus r and angle θ, in the angle unit unless marked (`2\angle 60^\circ`); the angle runs to the next operator |

## Calculus

| Input | Notes |
|---|---|
| `\sum_{k=1}^{10} k` | exact, up to 1,000,000 terms. The body is one term: `\sum k + 1` is $(\sum k) + 1$ |
| `\sum_{k=1}^{\infty} \frac{1}{k^2}` | to infinity, numerical with at most 30 digits: terms that are a rational function of $k$, alternating terms, or terms that fall at least geometrically; a divergent series is an error |
| `\prod_{k=1}^{5} k` | the product of the terms, like `\sum`; an empty product is 1 |
| `\int_0^1 x^2\,dx` | numerical, at most 30 digits. Differentials `dx`, `d x`, `\mathrm{d}x` |
| `\int_1^{\infty} \frac{1}{x^2}\,dx` | improper integrals with `\infty` bounds |
| `\left[\int x^2\,dx\right]_0^2` | an antiderivative evaluated between limits: the integral from 0 to 2 |
| `\int x^2\,dx` | the antiderivative, $\frac{1}{3}x^{3} + C$, with its working; also with both limits left empty, `\int_{}^{}`, as the ∫ key leaves them |
| `\lim_{x \to 0} f`, `\lim_{x \to 0^+} f`, `\lim_{x \to \infty} f` | two-sided, one-sided, at infinity; `\rightarrow` works too |
| `\left.\frac{d}{dx} x^3\right\|_{x=2}` | derivative at a point, symbolic; also `\mathrm{d}` and `\partial` |
| `\frac{d}{dx} x^3`, `(x^3)'`, `x^3\frac{d}{dx}` | the derivative as a function, $3x^{2}$, with its working; written at the end, `\frac{d}{dx}` applies to everything before it (after an `=`) |
| `\left.\frac{d^2}{dx^2} x^3\right\|_{x=2}` | higher derivatives, up to the 9th |
| `\left.(x^3 - x)'\right\|_{x=2}` | prime notation for the derivative of a bracket, `''` for the second |
| `\left.\frac{\partial}{\partial x} (x^2 z)\right\|_{x=2, z=3}` | partial derivative at a point in several variables, in any order; a letter the point names is a variable in the body, even a unit such as `t` |
| `\left.\frac{\partial^2}{\partial x \partial z} f\right\|_{x=1, z=2}` | mixed partials, also `\partial^3 / \partial x^2 \partial z` |

An integral without bounds and a derivative without a point have to be the whole input, and a
function of one variable without units; inside a calculation they need bounds or a point.
`\oint` is refused.

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

An **inequality** with one unknown, polynomial or rational, is solved by a sign chart; the
solution set is written like the roots of an equation:

| Input | Result |
|---|---|
| `x^2 - 4 < 0` | $-2 < x < 2$ |
| `\frac{x+1}{x-2} \ge 0` | $x \le -1 \lor x > 2$ |
| `1 < 2x + 1 \le 5` | $0 < x \le 2$ |

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
The mode `verify` (**Verify**, Prüfen) checks every input this way, an equation too, and
reads `a_1 = 3` as a comparison rather than a definition.

## Labels and variables

| Input | Meaning |
|---|---|
| `m_1 = 2kg` | defines the session variable $m_1$ |
| `F = m_1 \cdot g_n` | names the line $F$ |
| `\omega_0`, `v_{max}` | any letter or Greek letter with an index is a variable name |
| `\mathrm{Ans}`, `\mathrm{PreAns}` | the last numeric result of the session and the one before it, in every mode (also in `\operatorname{}`) |

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
| `5\,\mathrm{cm} \blacktriangleright \mathrm{in}`, `▶` | the value written in the unit after the arrow, in every mode; the dimensions must match. Temperatures convert with their offset: `50°F ▶ ^\circ\mathrm{C}` is $10\,^\circ\mathrm{C}$; see [Converting units](../guides/physics/convert.md) |

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

The mode `baseN` works with whole numbers in the [number base](settings.md#outside-the-window) set,
in a word of 8, 16, 32 or 64 bits (`wordSize`, default 32), signed (two's complement) or unsigned
(`signed`). A result outside the word wraps modulo 2ⁿ and the response carries a `warning`. A
number typed must fit the word: in decimal up to the top of the range (signed: up to 2ⁿ⁻¹ after a
minus), in the other bases any pattern of at most n bits (`FF` is −1 in 8 bit signed). The result
comes in the other three bases and as its bits as its forms. In the app, switches above the input
set the base, the word size and the signedness, and keys put in the logic, the shifts, the floats
and the prefixes.

| Input | Meaning |
|---|---|
| `1F`, `\mathrm{1F}` | a number in the set base; letters A–F in hexadecimal |
| `\mathrm{h}1F`, `\mathrm{d}10`, `\mathrm{b}101`, `\mathrm{o}17` | one number in another base; plain `h1F` works where the letter is no digit of the set base |
| `+`, `-`, `\cdot`, `\div`, `\frac{a}{b}` | arithmetic; division truncates toward zero, signed or unsigned as the word |
| `\operatorname{and}`, `or`, `xor`, `xnor` | bitwise operations, written plain or in `\operatorname{}` |
| `\operatorname{Not}(x)`, `\operatorname{Neg}(x)` | bitwise not (one's complement), negation (two's complement) |
| `x << n`, `x \ll n`; `x >> n`, `x \gg n` | shift left; shift right, arithmetic when signed, logical when unsigned |
| `\operatorname{shl}(x, n)`, `shr`, `sar` | shift left, logical right, arithmetic right; `,` or `;` between the arguments |
| `\operatorname{rol}(x, n)`, `ror` | rotate left, right within the word |
| `\operatorname{f32}(x)`, `f64(x)` | the IEEE-754 pattern of the decimal number `x` (`0{,}1`, `1.5e3`, `2 \cdot 10^{-3}`) |

Signs, `Not`, `Neg` and the functions bind first, then products, then sums, then `<<` and `>>`,
then `and`, then `or`, `xor` and `xnor`. A shift by the width or more leaves 0 (`sar`: all sign
bits); a rotation turns by n mod width; a negative count is an error. `f32` and `f64` need a word
of 32 or 64 bits; `F32` in capitals is a hexadecimal number.

## Ignored layout

`\displaystyle`, `\textstyle`, `\limits`, `\quad`, `\qquad` and `\,` are accepted and change
nothing.
