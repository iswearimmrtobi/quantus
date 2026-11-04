---
sidebar_position: 8.5
title: Geometry
---

# Geometry

**Geometry** mode (German **Geometrie**) is analytic geometry in space: points, vectors, lines,
planes, spheres and circles, drawn in a 3D view. Pick two objects and a task, and the page gives
the relation, the intersection, the distance or the angle exactly, with the steps written the
way they are done at school.

Objects have names of their own, apart from the session variables: a point $A$, a vector
$\vec{a}$ and a variable $a$ can all exist at once. Inputs are read as in
[Algebra](algebra.md), without units.

## The page

Switch to **Geometry** in the top bar. From top to bottom the page has:

1. a line to **type** an object with its name,
2. the **3D view** of the scene,
3. the **objects**, each as a form,
4. the **task** with its answer and steps.

The scene and the task are kept when you switch to another mode and back, and saved with the
session.

## Adding objects with a form

The buttons after **Add** create a **Point**, **Vector**, **Line**, **Plane**, **Sphere** or
**Circle** with the next free name ($A, B, \dots$ for points, $g, h, \dots$ for lines,
$E, F, \dots$ for planes). The switch at the start of a row chooses how the object is given:

| Object | Given by |
|---|---|
| Point, Vector | its coordinates |
| Line | **Point and direction**, or **Two points** |
| Plane | **Parameter form** (a point and two directions), **Normal form** (a point and a normal), **Coordinate form** ($ax + by + cz = d$), or **Three points** |
| Sphere | **Centre and radius**, **Centre and point**, or **Four points** not in one plane |
| Circle | **Centre, radius and normal** (no normal: a circle in the x-y plane), or **Sphere ∩ plane**: the circle where a sphere of the scene cuts a plane of the scene |

- Each coordinate is a small formula field: `\frac{1}{2}`, `\sqrt{2}` and `\pi` are allowed,
  letters and units are not. An empty cell is 0.
- Leave z empty everywhere and the scene is 2D: the results are written without z.
- The small button beside a point or vector (**A**) swaps the cells for one field that takes a
  name or an expression of the scene: `A`, `\vec{u}`, `B - A`, `(1|2|3)`. Click it again
  (**( | )**) for the cells.
- The name field can be changed; **×** removes the object.

Under each row the engine writes the object as it read it, such as
$g: \vec{x} = \begin{pmatrix} 1 \\ 0 \\ 0 \end{pmatrix} + \lambda \begin{pmatrix} 2 \\ 2 \\ 0 \end{pmatrix}$,
or why it could not be built. A construction that takes some working (a line through two points,
a sphere through four) shows its steps there too. An object that fails is left out of the view
and the tasks; the others still work.

## Typing objects

The line at the top takes one object with its name. <kbd>Enter</kbd> adds it to the scene, or
replaces the object of that name where it stands.

| Input | Object |
|---|---|
| `A(1\|2\|3)` or `A = (1\|2\|3)` | the point $A$ |
| `\vec{a} = \begin{pmatrix} 1 \\ 2 \\ 3 \end{pmatrix}` | the vector $\vec{a}$ |
| `g: X = A + \lambda \vec{u}` | a line through $A$ along $\vec{u}$ |
| `E: X = A + \lambda \vec{u} + \mu \vec{v}` | a plane in parameter form |
| `E: \begin{pmatrix} 1 \\ 0 \\ 2 \end{pmatrix} \cdot (X - A) = 0` | a plane in normal form |
| `E: 2x + y - z = 4` | a plane in coordinate form |
| `K: (x-1)^2 + y^2 + z^2 = 9` | a sphere |
| `k: x^2 + y^2 = 4` | a circle in the x-y plane |

`X` and `\vec{x}` stand for the running point. A line has one parameter and a plane two, from
$\lambda$, $\mu$, $\sigma$ and $\tau$. An equation in x, y and z is a plane when it is linear and a
sphere when $x^2$, $y^2$ and $z^2$ have the same factor and there are no mixed terms; anything
else is refused.

### Points and vectors

- A **point** is written `(1|2|3)` or `P(1|2|3)`; `;` works as a separator too: `(1;2;3)`. The
  comma is the decimal comma, so `(1,5|2|3)` is the point with $x = 1{,}5$.
- A `|` right after `(` or after a separator separates coordinates. An absolute value inside a
  coordinate is written `\left|…\right|`: `(\left|-2\right||1|0)`.
- A **vector** is a column `\begin{pmatrix} 1 \\ 2 \\ 3 \end{pmatrix}` (two or three entries),
  or `\vec{a}` for a vector of the scene.

## The 3D view

The view draws every object in its own colour, with its name; the object rows use the same
colour. Planes are cut off at a box around the scene and drawn translucent, lines are clipped to
it, vectors are arrows from the origin, spheres have rings of latitude and longitude.

- Drag to turn the scene, scroll to zoom.
- **View from above** looks down the z-axis, which is the view for a 2D scene.
- **Reset** goes back to the start view.

## Tasks

Choose a task, then the objects it works on. The choices offer only objects that fit.

| Task | Objects | Answer |
|---|---|---|
| **Relation** | any two | how they lie, and what comes with it (below) |
| **Distance** | points, lines, planes, spheres | the distance |
| **Angle** | vectors, lines, planes | the angle |
| **Foot of the perpendicular** | a point onto a line or plane | the foot $F$ |
| **Reflection** | a point in a point, line or plane | the image $P'$ |
| **Midpoint** | two points | the midpoint $M$ |
| **Tangent** | a sphere or circle at a point on it | the tangent plane or tangent |
| **Convert a plane** | a plane into parameter, normal or coordinate form | the plane in that form |
| **Length, area, volume** | a vector, sphere or circle | length; surface or volume; circumference or area |
| **Expression** | the scene's points and vectors | the value of an expression |

The relations:

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

## What is refused

- A coordinate with a letter that has no value (`(a|1|2)`) or a unit: coordinates are numbers.
- A point with one or four coordinates, a vector that is a matrix.
- An equation that is neither a plane nor a sphere (`x^2 + 2y^2 + z^2 = 1`, `xy = 1`), a sphere
  whose radius squared would be negative.
- A line through two equal points or along the zero vector, a plane through three points on a
  line or with collinear directions, a sphere with a radius that is not positive or through four
  points in one plane, a tangent at a point that is not on the sphere or circle.
- `(1,2,3)`: the comma is the decimal comma; write `(1|2|3)`.
- A task on objects it does not fit: *A distance does not work for these objects.*

## Settings that matter here

- **Angle unit**: angles in degrees or radians.
- **Step detail**: how much of the working is shown.
- **Language**: the steps and the names of the relations.

See [Settings](../reference/settings.md).

## See also

- [Algebra](algebra.md): how numbers are read
- [Working and output](working-and-output.md)
