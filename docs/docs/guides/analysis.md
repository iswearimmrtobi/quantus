---
sidebar_position: 6
title: Curve sketching
---

# Curve sketching

**Analysis** mode takes a function of one variable and works through the whole curve sketch,
each part with its working, beside an interactive graph. Letters are read as in Algebra.

## Enter a function

Switch to **Analysis** in the top bar and enter the function in any of these forms:

```latex
f(x) = x^3 - 3x
y = x^3 - 3x
x^3 - 3x
```

Press <kbd>Enter</kbd>. Polynomials, rational functions, $e^x$, $a^x$, logarithms, roots, the
trigonometric functions and their inverses and $|x|$ work, also combined. See
[Known limits](../reference/limits.md#analysis-mode) for what is rejected.

## The curve sketch

The **Curve sketch** tab lists one section per question. For $f(x) = x^3 - 3x$:

| Section | Result |
|---|---|
| Derivatives | $f'(x) = 3x^{2} - 3, \quad f''(x) = 6x, \quad f'''(x) = 6$ |
| Domain | $D = \mathbb{R}$ |
| Symmetry | symmetric about the origin |
| y-intercept | $S_y(0 \mid 0)$ |
| Zeros | $Z_{1}(-\sqrt{3} \mid 0), \; Z_{2}(0 \mid 0), \; Z_{3}(\sqrt{3} \mid 0)$ |
| Extrema | $Max(-1 \mid 2), \; Min(1 \mid -2)$ |
| Inflection points | $I(0 \mid 0)$ |
| Monotonicity | increasing on $(-\infty, -1]$ and $[1, \infty)$, decreasing on $[-1, 1]$ |
| Curvature | convex on $[0, \infty)$, concave on $(-\infty, 0]$ |
| Behaviour at the edges and asymptotes | no asymptotes |
| Antiderivative | $F(x) = \frac{1}{4}x^{4} - \frac{3}{2}x^{2} + C$ |

Each section opens to its working. The zeros, for example:

| | |
|---|---|
| Set $f(x) = 0$ | $x^{3} - 3x = 0$ |
| Factor out $x$ | $x \cdot (x^{2} - 3) = 0$ |
| A product is zero where one of its factors is zero | $x = 0 \lor x^{2} - 3 = 0$ |
| Isolate the square | $x^{2} = 3$ |
| Take the root | $x_{1,2} = \pm \sqrt{3}$ |

Results are exact where school methods reach, with a decimal next to them
($Z_{1}(-\sqrt{3} \mid 0) \approx Z_{1}(-1{,}73205 \mid 0)$). A section that needed a numeric
method or ran out of time is marked **PARTIAL**.

### Poles and asymptotes

For a rational function such as $f(x) = \frac{x^2+1}{x-1}$ the sketch also finds the gaps in the
domain and the asymptotes:

| Section | Result |
|---|---|
| Domain | $D = \mathbb{R} \setminus \{1\}$ |
| Extrema | $Max(1 - \sqrt{2} \mid 2 - 2\sqrt{2}), \; Min(1 + \sqrt{2} \mid 2 + 2\sqrt{2})$ |
| Behaviour at the edges and asymptotes | vertical asymptote $x = 1$, oblique asymptote $y = x + 1$ |
| Antiderivative | $F(x) = \frac{1}{2}x^{2} + x + 2\ln\left\|x - 1\right\| + C$ |

## The graph

The graph shows the curve, its asymptotes and the marked points (zeros, extrema, inflection and
saddle points, removable gaps). Scroll to zoom, drag to move, **Reset view** to go back. Clicking
a point selects it in the list below the graph, and the other way round.

## Tangent and normal

On the **Tangent / normal** tab, enter $x_0$ and press **Compute**. For $x_0 = 2$:

| | |
|---|---|
| Point on the graph | $f(2) = 2^{3} - 3 \cdot 2 = 2$ |
| Slope: the derivative at $x_0$ | $m = f'(2) = 3 \cdot 2^{2} - 3 = 9$ |
| Tangent, values put in | $t(x) = 9 \cdot (x - 2) + 2$ |
| | $t(x) = 9x - 16$ |
| Normal: slope $-1/m$ | $n(x) = -\frac{1}{9}x + \frac{20}{9}$ |

Both lines are drawn into the graph.

## Area

On the **Area** tab, enter the bounds, or leave both empty for the area between the outermost
zeros. Parts below the axis count with their absolute value; the signed **INTEGRAL** is shown
too. For $x^3 - 3x$ without bounds:

| | |
|---|---|
| Zeros inside the interval split it | $x = 0$ |
| Integral over the part | $A_{1} = \left[F(x)\right]_{-\sqrt{3}}^{0} = \frac{9}{4}$ |
| Integral over the part | $A_{2} = \left[F(x)\right]_{0}^{\sqrt{3}} = -\frac{9}{4}$ |
| **AREA** | $A = \left\|A_{1}\right\| + \left\|A_{2}\right\| = \frac{9}{2}$ |
| **INTEGRAL** | $\int_{-\sqrt{3}}^{\sqrt{3}} f(x)\,dx = 0$ |

## Value table

On the **Value table** tab, enter **from**, **to** and **step**. Points outside the domain are
shown as *undef.*

| $x$ | $-2$ | $-1$ | $0$ | $1$ | $2$ |
|---|---|---|---|---|---|
| $f(x)$ | $-2$ | $2$ | $0$ | $-2$ | $2$ |

## Settings that matter here

- **Quadratic equations**: pq formula or abc formula in the working (*Automatic*: pq in German).
- **Step detail**: how much working each section shows.

## See also

- [Known limits](../reference/limits.md#analysis-mode)
