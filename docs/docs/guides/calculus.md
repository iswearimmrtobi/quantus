---
sidebar_position: 5
title: Calculus
---

# Calculus

Derivatives at a point, definite integrals, limits and sums work in every calculator mode, and
with units in Physics mode. For a function as a whole (its derivative, antiderivative, zeros and
graph) use [Curve sketching](analysis.md).

These are easiest to type in the [LaTeX source](input.md#the-latex-source).

## Derivative at a point

```latex
\left.\frac{d}{dx} (3x^2+1)^4\right|_{x=1}
```

$$
1536
$$

The derivative is taken symbolically, then the point is put in. A derivative without a point is
an error that tells you how to add one.

## Definite integrals

| Input | Result |
|---|---|
| `\int_0^{\pi} \sin(x)\,dx` | $2$ |
| `\int_1^{\infty} \frac{1}{x^2}\,dx` | $1$ |
| `\int_{-\infty}^{\infty} e^{-x^2}\,dx` | $1{,}77245385090551602729816748334$ |

Integrals are computed numerically, to at most 30 digits; exact values such as $2$ are
recognised. The differential can be written `dx`, `d x` or `\mathrm{d}x`. Bounds with units give
the integration variable that unit:

```latex
\int_0^{2s} 3\frac{m}{s}\,dt
```

$$
6\,\mathrm{m}
$$

## Limits

| Input | Result |
|---|---|
| `\lim_{x \to 0} \frac{\sin x}{x}` | $1$ |
| `\lim_{x \to \infty} (1+\frac{1}{x})^x` | $2{,}718281828\ldots$ |
| `\lim_{x \to 0^+} \frac{\|x\|}{x}` | $1$ |
| `\lim_{x \to 0^-} \frac{\|x\|}{x}` | $-1$ |

When the two sides disagree, the two-sided limit is an error; ask for one side with `0^+` or
`0^-`.

## Sums and binomials

| Input | Result |
|---|---|
| `\sum_{k=1}^{100} k^2` | $338350$ |
| `\sum_{k=0}^{10} \binom{10}{k}` | $1024$ |
| `5!` | $120$ |

Sums are exact and allow up to 1,000,000 terms. The body of a sum is one term: write
`\sum_{k=1}^{3} (k + 1)` for $\sum (k+1)$.

## Determinants

```latex
\det\begin{pmatrix}1&2\\3&4\end{pmatrix}
```

$$
-2
$$

`\begin{vmatrix}…\end{vmatrix}` is the same. Exact, up to 8×8.

## See also

- [Syntax](../reference/syntax.md#calculus)
- [Known limits](../reference/limits.md): no higher or partial derivatives, no indefinite integrals
  in the calculator
