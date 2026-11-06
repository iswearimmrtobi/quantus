---
sidebar_position: 2
title: Objects
---

# Objects

An object in [Geometry](index.md) mode is added with a form or typed on one line.

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

## See also

- [Geometry](index.md)
- [Tasks](tasks.md)
