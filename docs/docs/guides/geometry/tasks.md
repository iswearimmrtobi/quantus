---
sidebar_position: 3
title: Tasks
---

# Tasks

Pick two objects of the [Geometry](index.md) page and a task: the answer is worked out exactly, with the steps.
The task sits at the foot of the sidebar.

## Tasks

Choose a task, then the objects it works on. The choices offer only objects that fit; the
second offers only those that go with the first (for a distance from a sphere, only points).

| Task | Objects | Answer |
|---|---|---|
| **Relation** | two of points, lines, planes, spheres and circles (the pairs below) | how they lie, and what comes with it (below) |
| **Distance** | points, lines, planes, spheres | the distance |
| **Angle** | vectors, lines, planes | the angle |
| **Foot of the perpendicular** | a point onto a line or plane | the foot $F$ |
| **Reflection** | a point in a point, line or plane | the image $P'$ |
| **Midpoint** | two points | the midpoint $M$ |
| **Tangent** | a sphere or circle at a point on it | the tangent plane or tangent |
| **Convert a plane** | a plane into parameter, normal or coordinate form | the plane in that form |
| **Length, area, volume** | a vector, sphere or circle | length; surface or volume; circumference or area |
| **Expression** | the scene's points and vectors | the value of an expression |

## Picking a task from the objects

Open an object's row, or select two objects with <kbd>Ctrl</kbd> + click (<kbd>⌘</kbd> on a Mac)
on their rows or in the view, and the task shows the tasks that fit them under **For the
selection:**. One click sets the task with these objects, in the order it takes them: a point
and a plane give **Relation**, **Distance**, **Foot of the perpendicular** and **Reflection**;
two lines **Relation**, **Distance** and **Angle**; a sphere alone **Length, area, volume**.
A third object selected takes the place of the first; <kbd>Ctrl</kbd> + click on a selected one
takes it out.

## The relations

| Objects | Results |
|---|---|
| Two lines | identical, parallel (with the distance), intersecting (point and angle), skew (distance and angle) |
| Line and plane | intersecting (point and angle), parallel (distance), contained |
| Two planes | identical, parallel (distance), intersecting (line and angle) |
| Point and line, plane or circle | on it or not |
| Point and sphere | inside, on or outside, with the distance to the sphere |
| Sphere and line | secant (two points), tangent (the point), no common point |
| Sphere and plane | intersection circle, tangent (the point), no common point (distance) |
| Two spheres | identical, concentric, apart, one inside the other, touching, or intersecting with the circle and the Potenzebene |
| Circle and line | secant, tangent or no common point in the circle's plane; otherwise through the circle or past it |
| Circle and plane | two points, touching, no common point, contained, parallel |

Parallel, skew or no common point are answers, not errors.

### Expressions

**Expression** works with the scene's points and vectors. The types decide what an operator
means:

| Input | Result |
|---|---|
| `B - A` | the vector from $A$ to $B$ |
| `A + \vec{u}` | a point |
| `2 \cdot \vec{u}`, `\frac{\vec{u}}{2}` | a multiple |
| `\vec{a} \cdot \vec{b}` | the dot product, a number |
| `\vec{a} \times \vec{b}` | the cross product, a vector |
| `\|\vec{a}\|`, `\|B - A\|` | the length |
| `\vec{a}^2` | $\vec{a} \cdot \vec{a}$ |
| `(\vec{a} \times \vec{b}) \cdot \vec{c}` | the triple product (Spatprodukt) |

Between numbers, `\times` stays multiplication. A number times a vector is written with `\cdot`;
two points cannot be added, and a point cannot be multiplied (its position vector `A - (0|0|0)`
can).

## The answer

The answer is exact: fractions, roots and $\pi$ stay as they are, so a distance is
$2\sqrt{2}$, not $2{,}83$. An angle follows the setting **Angle unit**; when it has no closed
form it is written as its $\arccos$ or $\arcsin$ with the decimal:

| Task | Answer |
|---|---|
| Lines $g: \vec{x} = (1\|0\|0) + \lambda (1, 1, 0)$ and $h: \vec{x} = (0\|3\|0) + \mu (1, 0, 0)$ | intersecting in $S = (4\|3\|0)$, $\varphi = 45^\circ$ |
| Line $g: \vec{x} = (1\|2\|0) + \lambda (1, 0, -1)$ and plane $E: 2x - y + 3z = 4$ | $S = (-3\|2\|4)$, $\varphi = \arcsin\left(\frac{\sqrt{7}}{14}\right) \approx 10{,}8934^\circ$ |
| Sphere $K: (x - 1)^2 + (y - 2)^2 + (z - 3)^2 = 25$ and plane $E: z = 6$ | intersection circle around $M' = (1\|2\|6)$ with $\rho = 4$ |
| $E: \vec{x} = (1\|1\|1) + \lambda (1, 2, 3) + \mu (-2, 0, 4)$ into coordinate form | $4x - 5y + 2z = 1$ |

**Copy** puts the answer and the steps on the clipboard as an `align*` block.

## The answer in the view

The 3D view also draws the answer, in the accent colour, on top of the scene. The other objects
fade until you hover a row or select an object:

| Task | Drawn |
|---|---|
| Distance | the shortest segment between the two objects, labelled $d = 3$ or $d \approx 3{.}46$ (nothing when they meet) |
| Foot of the perpendicular | the foot $F$ and the perpendicular, dashed |
| Reflection | the image ($A'$ for the point $A$) and the dashed line to it |
| Midpoint | $M$ and the dashed segment between the two points |
| Relation | where the objects meet: the point $S$, the points $S_1$ and $S_2$, the line $s$ or the circle $k$ |
| Angle | the two directions from the point where the objects meet (the origin for vectors) and an arc, labelled $\varphi \approx \ldots$ |
| Tangent | the tangent plane $T$ or the tangent $t$ |

The labels in the view are rounded to two decimals. The exact answer stays in the sidebar.
Converting a plane, measuring and evaluating an expression draw nothing. The view's box grows
when the answer lies outside it, for example a far image point.

## The steps

The steps follow the setting **Step detail**. For the two lines above:

| Level | What is added |
|---|---|
| **Brief** | The results: $S = (4\|3\|0)$, $\varphi = 45^\circ$ |
| **Standard** | The school method: the directions are no multiples, the line equations set equal, λ and μ from two rows, the check in the third, the cosine of the angle |
| **Detailed** | The system of equations written out |
| **Maximal** | The check that μ in the second line gives the same point |

A plane in another form is put into the coordinate form first, and that is part of the steps.
**None** hides them. More examples level by level are in the
[step detail spec](https://github.com/iswearimmrtobi/quantus/blob/master/design/step-detail.md#geometry-mode).

## See also

- [Objects](objects.md)
- [Algebra](../algebra/index.md): how numbers are read
- [Working and output](../../basics/working-and-output.md)
