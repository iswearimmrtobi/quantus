---
sidebar_position: 1
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
[Known limits](../../reference/limits.md#analysis-mode) for what is rejected.

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

## On the handheld

Choose **Kurvendiskussion** (tile 4) and type the function on the input line; **EXE** sketches
it. The first page is the graph, with the points named as above; each section follows on a page
of its own. **←**/**→** turn the pages, **AC** goes back to the input.

Each section opens on its result alone. **↓** shows the working from its start, with the result
again at its end; **↑** goes back to the result.

**OK** on the graph starts moving it: the arrows move it, **+** and **−** zoom in and out, **0**
goes back to the first view, and **OK** or **AC** stop. A finger drags the graph at any time.

**OPTN** has the rest:

- **Abschnitte** lists the pages of the last sketch, to jump to one.
- **Tangente, Normale**, **Fläche** and **Wertetabelle** open a small form under the function:
  x₀; a and b (both empty for the outer zeros); von, bis, Schritt. **EXE** on its last field
  works it out. After a sketch of the same function, → shows the tangent and normal, or the
  area, on its graph.

The session keeps the function, so it is back on the input line when the session is opened again.

## Settings that matter here

- **Quadratic equations**: pq formula or abc formula in the working (*Automatic*: pq in German).
- **Step detail**: how much working each section shows.

## See also

- [Graph and tools](graph.md): the graph, tangent and normal, area, value table
- [Calculus](../../basics/calculus.md): a derivative, integral or limit at a point, in any mode
- [Known limits](../../reference/limits.md#analysis-mode)
