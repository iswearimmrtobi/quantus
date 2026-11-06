---
sidebar_position: 1
title: Geometry
---

# Geometry

**Geometry** mode (German **Geometrie**) is analytic geometry in space: points, vectors, lines,
planes, spheres and circles, drawn in a 3D view. Pick two objects and a task, and the page gives
the relation, the intersection, the distance or the angle exactly, with the steps written the
way they are done at school.

Objects have names of their own, apart from the session variables: a point $A$, a vector
$\vec{a}$ and a variable $a$ can all exist at once. Inputs are read as in
[Algebra](../algebra/index.md), without units.

## The page

Switch to **Geometry** in the top bar. From top to bottom the page has:

1. a line to **type** an object with its name,
2. the **3D view** of the scene,
3. the **objects**, each as a form,
4. the **task** with its answer and steps.

The scene and the task are kept when you switch to another mode and back, and saved with the
session.

## The 3D view

The view draws every object in its own colour, with its name; the object rows use the same
colour. Planes are cut off at a box around the scene and drawn translucent, lines are clipped to
it, vectors are arrows from the origin, spheres have rings of latitude and longitude.

- Drag to turn the scene, scroll to zoom.
- **View from above** looks down the z-axis, which is the view for a 2D scene.
- **Reset** goes back to the start view.

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

See [Settings](../../reference/settings.md).

## See also

- [Objects](objects.md): adding points, vectors, lines, planes, spheres and circles
- [Tasks](tasks.md): relations, intersections, distances and angles, with the answer and steps
- [Algebra](../algebra/index.md): how numbers are read
- [Working and output](../../basics/working-and-output.md)
