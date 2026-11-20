---
sidebar_position: 1
title: Curve sketching
---

# Curve sketching

**Analysis** mode takes a function of one variable and works through the whole curve sketch,
each part with its working, beside an interactive graph that fills the window's height. Letters are read as in Algebra.

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

### A given domain

To sketch $f$ on less than its largest possible domain, write the domain after the function, or
type it into the **D =** field below the input (leave it empty for the largest one):

```latex
f(x) = x^2, \ x \in [-1, 2]
f(x) = x^2, \ 0 \le x < 3
f(x) = \ln x \text{ für } x > 1
f(x) = x^2 \quad D = [-1, 2]
```

Intervals (`[0, 3)`, `]0; 3]`, `(-\infty, 2]`, joined with `\cup`), inequalities and chains of
two, `x \neq a`, $\mathbb{R}^+$ and $\mathbb{R}_0^+$ are understood. The sketch cuts every
section to the domain: the domain section shows $D = \mathbb{R} \cap [-1, 2] = [-1, 2]$, closed
ends become boundary maxima or minima, open ends get a limit, and the graph stops at the ends. A
section **Global extrema** compares the values at the local and boundary extrema with the limits
at open or unbounded ends: $x^2$ on $[-1, 2]$ has the global maximum $f(2) = 4$ and the global
minimum $f(0) = 0$; $e^{-x}$ for $x \ge 0$ has no global minimum, its infimum 0 is not attained.

### Piecewise functions

Type the pieces as cases, each with its condition, and `\text{otherwise}` (`\text{sonst}`) for
the rest if you like:

```latex
f(x) = \begin{cases} x^2 & x \le 1 \\ 2x - 1 & x > 1 \end{cases}
```

Each piece is sketched on its part of the domain, its working headed *Piece 1*, *Piece 2*, …;
the derivatives come as cases. The section **Continuity at the joins** works out f at each join,
both one-sided limits and, where f is continuous, both one-sided slopes: here f is continuous
and differentiable at $x = 1$ with $f'(1) = 2$. A join where f is lower or higher on both sides
is a maximum or minimum. The antiderivative is joined up with constants where f is continuous.
The graph shows the ends of the pieces as filled or hollow dots; tangent, area and value table
take the piece that holds x, and a tangent at a join needs f to be differentiable there.

### Several functions

**+ Function** below the input adds another function; each gets the next free letter (g, h, k, …)
unless it is typed with its own name (`p(x) = …`). The functions may use each other, also
differentiated:

```latex
g(x) = f(x - 1) + 2
h(x) = f'(x)
```

The reference is put in before the sketch starts, so $g$ above is sketched as
$(x - 1)^2 + 2$ for $f(x) = x^2$. **Sketch** beside a function swaps it into the main field and
sketches it. A function may not use itself, directly or through others, and a piecewise
function or one with a given domain cannot be used inside another.

All functions are drawn in the graph, each in its own colour with its name in the legend. The
boxes $f'$ and $f''$ above the graph draw the derivatives of the sketched function as well.

### A curve through points

Type points instead of a function, and the analysis builds the curve through them:

```latex
(0, 0), (1, 1), (2, 0), (3, 1)
(0 | 0); (1 | 1); (2 | 0)
```

Below the input choose a **natural spline** (no curvature at the ends, the default), a **spline
with end slopes** (type the slopes at the first and last point) or the **interpolating
polynomial**. A card above the sketch shows the working: one cubic
$s_i(x) = a_i + b_i(x - x_i) + c_i(x - x_i)^2 + d_i(x - x_i)^3$ per interval, the conditions
through the points, the same slope and curvature where two pieces meet and at the ends, the
linear system solved exactly, and the pieces multiplied out. The curve is then sketched as a
piecewise function on $[x_0, x_n]$, with the points drawn in the graph. The coordinates must be
rational numbers; up to 30 points.

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

The sections start closed, so the results read as a summary; click a section's title to open its
working. The zeros, for example:

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
