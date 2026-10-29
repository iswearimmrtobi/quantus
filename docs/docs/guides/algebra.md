---
sidebar_position: 3
title: Algebra
---

# Algebra

In **Algebra** mode every letter is a variable. Quantus simplifies, factors and solves, and
shows each rule it uses. Switch to it in the top bar.

## Simplify

```latex
(a+b)^2 - 2ab
```

| | |
|---|---|
| SYMBOLIC | $(a + b)^{2} - 2 \cdot a \cdot b$ |
| STEP | $a^{2} + 2 \cdot a \cdot b + b^{2} - 2a \cdot b$ — *Expand binomial power using Square of a sum* |
| STEP | $a^{2} + b^{2}$ — *Combine like terms* |

Powers, roots, logarithms and the trigonometric identities are simplified too:

| Input | Result | Rule |
|---|---|---|
| `a^3 \cdot a^4` | $a^{7}$ | same base: add the exponents |
| `\sqrt{x^2}` | $\left\|x\right\|$ | take powers out of the root |
| `\ln(e^3)` | $3$ | |
| `\sin^2 x + \cos^2 x` | $1$ | Pythagorean identity |

## Factor

```latex
2x^2 - 8
```

$$
2 \cdot (x - 2)(x + 2)
$$

*Write as a binomial: Difference of squares.* A quadratic without a binomial form is written by
completing the square: `x^2 - 6x + 3` becomes $(x - 3)^{2} - 6$. The settings
**Binomial formulas** and **Completing the square** decide which form you get, see
[Settings](../reference/settings.md#how-results-are-written).

## Fractions and their domain

```latex
\frac{x^2 - 1}{x + 1}
```

| | |
|---|---|
| SYMBOLIC | $\frac{x^{2} - 1}{x + 1}$ |
| STEP | $\frac{(x - 1)(x + 1)}{x + 1}$ — *Factor numerator* |
| STEP | $x - 1$ — *Cancel common factor $x + 1$* |
| DOMAIN | $x \neq -1$ |

When a factor is cancelled, the value that made it zero is kept under **DOMAIN**, so the result
says exactly where it equals the input. Sums of fractions are put over a common denominator:

```latex
\frac{1}{x} + \frac{1}{x+1}
```

| | |
|---|---|
| STEP | $\frac{1}{x} \cdot \frac{x + 1}{x + 1} + \frac{1}{x + 1} \cdot \frac{x}{x}$ — *Extend each fraction* |
| STEP | $\frac{x + 1}{x(x + 1)} + \frac{x}{x(x + 1)}$ — *Multiply the fractions* |
| STEP | $\frac{x + 1 + x}{x(x + 1)}$ — *Add the numerators* |
| STEP | $\frac{2x + 1}{x(x + 1)}$ |
| DOMAIN | $x \neq -1, 0$ |

Identities that hold only under a condition, such as $\sqrt{a}\sqrt{b} = \sqrt{ab}$ for
$a, b \geq 0$, are applied with the condition added to the domain. Set
**Identities with a condition** to *Only always valid* to skip them instead.

## Solve

Write an equation with one unknown:

| Input | Result |
|---|---|
| `x^2 - 5x + 6 = 0` | $x = 2 \lor x = 3$ |
| `\frac{x+1}{x-2} = 3` | $x = 3{,}5$ |
| `\|x - 1\| = 2` | $x = -1 \lor x = 3$ |
| `\sqrt{x+3} = x - 3` | $x = 6$ (squaring would also give $x = 1$, which does not fit) |
| `x^2 + 1 = 0` | *This equation has no real solution.* — try Imaginary mode |

## Systems of linear equations

Write several equations separated by `;`, or as `\begin{cases} … \end{cases}` in the LaTeX
field (open it with `\`):

| Input | Result |
|---|---|
| `2x + y = 5; x - y = 1` | $x = 2 \land y = 1$ |
| `\begin{cases} x + y + z = 6 \\ x - y = 0 \\ 2z = 6 \end{cases}` | $x = 1{,}5 \land y = 1{,}5 \land z = 3$ |
| `x + y = 1; 2x + 2y = 2` | $x = 1 - y \land y \in \mathbb{R}$ |
| `x + y = 1; x + y = 2` | *The system has no solution: its equations contradict each other.* |

The unknowns are the letters without a value, in alphabetical order. The system is solved
exactly by Gauss–Jordan elimination: at *Standard* detail the steps show the augmented matrix
and its reduced form, at *Detailed* every row operation ($\text{II} - 2 \cdot \text{I}$).
With infinitely many solutions the unknowns without a pivot stay free and the others are given
in terms of them. Coefficients may be fractions, $\pi$ or stored variables (`k_1 x + y = 7`);
the unknowns themselves must appear linearly.

## When nothing happens

An expression that is already as simple as Quantus can make it is shown as typed, with a note:

> The variable x has no value, so the expression is shown as typed.

## See also

- [Complex numbers](imaginary.md) for equations without real solutions
- [Working and output](working-and-output.md) for more or fewer steps
- [Known limits](../reference/limits.md)
