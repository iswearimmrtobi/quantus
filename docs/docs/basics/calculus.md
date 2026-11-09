---
sidebar_position: 4
title: Calculus
---

# Calculus

Derivatives, integrals, limits and sums work in every calculator mode, and with units in Physics
mode. For a function as a whole (its zeros, extrema and graph) use
[Curve sketching](../guides/analysis/index.md).

These are easiest to type in the [LaTeX source](input.md#the-latex-source).

## Derivative at a point

```latex
\left.\frac{d}{dx} (3x^2+1)^4\right|_{x=1}
```

$$
1536
$$

The derivative is taken symbolically, then the point is put in. When the variable has a value in
the session, a derivative without a point is taken there.

## Derivatives and antiderivatives as functions

Without a point or limits, the derivative or the integral is worked out as a function, with
the working the steps show:

| Input | Result |
|---|---|
| `\frac{d}{dx} (x^3 - 2x)` | $3x^{2} - 2$ |
| `\frac{d^2}{dx^2} \sin x` | $-\sin(x)$ |
| `(x^3 - x)'` | $3x^{2} - 1$ |
| `\int x e^x\,dx` | $(x - 1)e^{x} + C$ |
| `\int \frac{1}{x}\,dx` | $\ln\left\|x\right\| + C$ |

The rules are those of [Curve sketching](../guides/analysis/index.md): derivatives by the sum,
product, quotient and chain rules; antiderivatives from a table, by partial fractions,
substitution and integration by parts, each checked by differentiating it back. Integrands without
an elementary antiderivative, such as $e^{x^2}$, give *No elementary antiderivative found*.

Such a derivative or integral is the whole input (`\frac{d}{dx} x^3 + 1` is an error) and a
function of its variable alone: other letters (`\int a x\,dx`) and units are refused.

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

A sum can run to `\infty`. Like an integral it is numerical, with at most 30 digits:

| Input | Result |
|---|---|
| `\sum_{k=1}^{\infty} \frac{1}{k^2}` | $1{,}644934066848\ldots$ $\left(= \frac{\pi^2}{6}\right)$ |
| `\sum_{k=0}^{\infty} \frac{1}{2^k}` | $2$ |
| `\sum_{k=1}^{\infty} \frac{(-1)^{k+1}}{k}` | $0{,}693147180559\ldots$ $(= \ln 2)$ |
| `\sum_{k=1}^{\infty} \frac{1}{k}` | *The series diverges: its terms fall like 1/k, as in the harmonic series.* |

Three kinds of terms are summed: a fraction of polynomials in $k$ (decided exactly: it converges
when the denominator's degree is at least two above the numerator's), alternating terms that
fall towards 0, and terms that fall at least geometrically, such as $\frac{k}{2^k}$ or
$\frac{1}{k!}$. Other series, such as $\sum \frac{1}{\sqrt{k}}$, are refused instead of guessed: a
series that diverges slowly looks convergent for millions of terms.

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
- [Known limits](../reference/limits.md)
