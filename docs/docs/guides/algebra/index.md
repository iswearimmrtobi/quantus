---
sidebar_position: 1
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
[Settings](../../reference/settings.md#algebra).

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

## When nothing happens

An expression that is already as simple as Quantus can make it is shown as typed, with a note:

> The variable x has no value, so the expression is shown as typed.

## See also

- [Solving](solve.md): equations, inequalities and systems
- [Working and output](../../basics/working-and-output.md) for more or fewer steps
- [Settings](../../reference/settings.md#algebra)
- [Known limits](../../reference/limits.md)
