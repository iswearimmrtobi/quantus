---
sidebar_position: 2
title: Graph and tools
---

# Graph and tools

Beside the curve sketch of [Analysis](index.md) mode are the graph and three tools that work on
the same function: tangent and normal, area, and a value table.

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

## See also

- [Curve sketching](index.md)
- [Known limits](../../reference/limits.md#analysis-mode)
