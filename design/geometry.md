# Geometry mode (Geometrie)

Analytic geometry in space: points, vectors, lines, planes, spheres and circles, drawn in a 3D
view, with their relations, distances and angles worked out exactly and with steps. The plan it
was built from is [plans/geometry.md](../plans/geometry.md).

Code: `crates/quantus-core/src/geometry/` (maths, reader, view), `geometry/interface.rs` (the
scene and the tasks as the shells see them, re-exported in `api.rs`), and in the desktop app
`GeometryView.tsx`, `GeometryObjects.tsx`, `GeometryTask.tsx`, `GeometryPlot.tsx` and
`geometry.ts`. Tests: `tests/geometry.rs` and `tests/geometry_view.rs`.

## 1. Scope

**In the first version:**

- Objects: points, vectors, lines, planes, spheres and circles in 3D; a 2D input is the z = 0
  case and its results are written without z.
- Every operation of §4, exact in `Value` (fractions, roots, `π`), with a working at the four
  step levels (§6).
- Objects entered through forms or typed as one line with their name (§3).
- A 3D view, computed in the core as 2D primitives that each shell only draws (§7).

**Not in the first version:**

- Coordinates with a free variable or a unit: a coordinate is a number (`A coordinate must be a
  number, but 'a' has no value.`). Sets of lines and planes with a parameter (`E_a`) need
  symbolic coordinates and come later.
- Equations that are neither linear in x, y, z nor a sphere or a circle in the x-y plane
  (ellipsoids, cones).
- The handheld: it can reuse the engine and the view, but has no Geometrie tiles yet.
- Shape formulas and triangle solving (plane geometry), which are a separate area.

## 2. Glossary

The English names are those of the app; the German ones those of `de.ts` and the `msg!` texts.

| English | German | Notes |
|---|---|---|
| Point | Punkt | `A(1\|2\|3)`, a name with an optional index: `P_1` |
| Vector | Vektor | `\vec{a}`, a column `\begin{pmatrix} … \end{pmatrix}` |
| Position vector | Stützvektor (Ortsvektor) | of a line or plane in parameter form |
| Direction | Richtungsvektor | of a line; a plane in parameter form has two (Spannvektoren) |
| Normal | Normalenvektor | of a plane or of a circle's plane |
| Line | Gerade | `g: \vec{x} = \vec{p} + \lambda \vec{u}` |
| Plane | Ebene | in one of three forms |
| Parameter form | Parameterform | $\vec{x} = \vec{p} + \lambda \vec{u} + \mu \vec{v}$ |
| Normal form | Normalenform | $\vec{n} \cdot (\vec{x} - \vec{p}) = 0$ |
| Coordinate form | Koordinatenform | $ax + by + cz = d$ |
| Sphere | Kugel | $(x - m_1)^2 + (y - m_2)^2 + (z - m_3)^2 = r^2$ |
| Circle | Kreis | centre, radius and normal |
| Centre | Mittelpunkt | of a sphere or circle |
| Radius | Radius | |
| Relation | Lagebeziehung | how two objects lie to each other |
| Distance | Abstand | |
| Angle | Winkel (Schnittwinkel) | |
| Intersection point / line | Schnittpunkt / Schnittgerade | |
| Intersection circle | Schnittkreis | sphere ∩ plane, sphere ∩ sphere |
| Foot of the perpendicular | Lotfußpunkt | |
| Reflection | Spiegelung | of a point in a point, line or plane |
| Midpoint | Mittelpunkt | of two points |
| Tangent / tangent plane | Tangente / Tangentialebene | |
| Dot product | Skalarprodukt | `\vec{a} \cdot \vec{b}` |
| Cross product | Kreuzprodukt (Vektorprodukt) | `\vec{a} \times \vec{b}` |
| Triple product | Spatprodukt | |
| Length | Betrag (Länge) | `\left\|\vec{a}\right\|` |
| Identical, parallel, intersecting, skew | identisch, parallel, schneidend, windschief | |
| Contained | enthalten | a line in a plane, a circle in a plane |
| Secant, tangent, passant | Sekante, Tangente, Passante | a line to a sphere or circle |
| Inside, on, outside | innerhalb, liegt darauf, außerhalb | a point to a sphere |
| Concentric, touching | konzentrisch, berührend | two spheres |
| Potenzebene | Potenzebene | the plane through the intersection circle of two spheres |
| Surface, volume | Oberfläche, Volumen | of a sphere |
| Circumference, area | Umfang, Flächeninhalt | of a circle |

## 3. Input

### Objects and their names

A scene is a list of named objects, built in order, so a later object can use an earlier point or
vector by its name. Names are letters with an optional index (`A`, `P_1`, `g`, `E`, `K`); a vector
is named `\vec{a}` and is keyed `a`. Objects live only in the scene, in their own namespace like
the matrices of the matrix mode: they are not session variables, and a vector `\vec{a}`, a point
`A` and a variable `a` may coexist. `x`, `y` and `z` in an equation are always the coordinates.

An object that cannot be built is reported with its error and left out; the others are still
built, and a task that names the failed one gets an error of its own.

### Forms

Each kind of object has the ways it can be given (the shape kinds of `interface::Shape`):

| Kind | Given by | Shape |
|---|---|---|
| Point | its coordinates | `point` |
| Vector | its coordinates | `vector` |
| Line | point and direction | `line` |
| | two points | `lineThrough` |
| Plane | parameter form: point and two directions | `plane` |
| | normal form: point and normal | `planeNormal` |
| | coordinate form: a, b, c, d | `planeCoordinate` |
| | three points | `planeThrough` |
| Sphere | centre and radius | `sphere` |
| | centre and a point on it | `sphereThrough` |
| | four points not in one plane | `sphereFour` |
| Circle | centre, radius and normal (empty normal: the x-y plane) | `circle` |
| | a sphere and a plane of the scene (Kugel ∩ Ebene) | `circleCut` |

A point or vector field (an `Entry`) is either two or three coordinate cells, each a LaTeX number
(an empty cell is 0), or one LaTeX entry: a name (`A`, `\vec{a}`) or an expression (`B - A`,
`(1|2|3)`). Two cells make a 2D object.

### Typed lines

One line with the name in it creates an object, or replaces the one of that name where it stands:

| Line | Object |
|---|---|
| `A(1\|2\|3)`, `A = (1\|2\|3)` | point |
| `\vec{a} = \begin{pmatrix} 1 \\ 2 \\ 3 \end{pmatrix}` | vector |
| `g: X = A + \lambda \vec{u}`, `g: \vec{x} = …` | line (one parameter) |
| `E: X = A + \lambda \vec{u} + \mu \vec{v}` | plane in parameter form (two parameters) |
| `E: 2x + y - z = 4` | plane in coordinate form (linear in x, y, z) |
| `E: \begin{pmatrix} 1 \\ 0 \\ 2 \end{pmatrix} \cdot (X - A) = 0` | plane (linear in X) |
| `K: (x-1)^2 + y^2 + z^2 = 9` | sphere: x², y², z² with one coefficient, no mixed terms |
| `k: x^2 + y^2 = 4` | circle in the x-y plane: x² and y² alone |

The lexer drops `:`, so the name before it is split off by the reader (`interface/typed.rs`);
`Name = …` is the parser's label. The parameters are `\lambda`, `\mu`, `\sigma` and `\tau`, taken
in that order; in Geometry mode they are not the micro prefix. Any other line is refused with the
list of forms above.

### Syntax

- **Points** in school notation: `P(1|2|3)` or `(1|2|3)`; `;` separates too: `(1;2;3)`. Not
  `(1,2,3)`: `1,2` is a decimal number. A `|` right after `(` or after a separator starts no
  absolute value; an absolute value inside a coordinate is written `\left|…\right|`.
  `(1,5|2|3)` is the point with x = 1.5.
- **Vectors** as `\begin{pmatrix}` columns of two or three entries, read by the matrix mode's
  `MatrixEvaluator`, or as `\vec{a}` for the scene's vector `a`.
- **Operators** follow the types: point − point is the vector between them, point + vector a
  point, vector ± vector, number · vector the multiple, vector · vector the dot product,
  vector × vector the cross product, `|\vec{a}|` the length, `\vec{a}^2` the dot product with
  itself, a vector over a number its multiple. Between numbers everything means what it means in
  Algebra; `\times` between numbers stays multiplication.
- **Refused**, each with its own message: adding two points, a number and a vector, multiplying
  or negating a point, dividing by a vector, any other power of a vector, units in a coordinate.

The rest of an input is read as in Algebra. Matrix mode is unchanged: no `\times` or `|v|` there.

## 4. Operations

Every operation returns its result with a `Working` (§6). Degenerate cases (parallel, identical,
skew, a point off a line) are results with their own working, not errors.

### Objects

| Object | Operations |
|---|---|
| Points and vectors | distance, midpoint, vector between points, length, unit vector, dot and cross product, angle, collinear and coplanar test, triple product, area of triangle and parallelogram, volume of tetrahedron and parallelepiped |
| Line | from two points or point and direction; point on line?; distance point–line; foot of the perpendicular; reflection |
| Plane | from three points, point and two directions, point and normal, coordinates; each form converted into the others; point on plane?; distance point–plane (Hesse); foot of the perpendicular; reflection |
| Sphere | from centre and radius, centre and point, four points; coordinate and vector form; point inside, on or outside, with its distance to the sphere; tangent plane at a point on it; surface and volume |
| Circle | from centre, radius and normal, or a sphere cut by a plane; point on circle?; tangent at a point on it; circumference and area |

Lines and planes keep the form they were given in; a conversion is a task.

### Pairs (Lagebeziehung)

| Pair | Results |
|---|---|
| Line – line | identical; parallel (distance); intersecting (point, angle); skew (distance, angle) |
| Line – plane | intersecting (point, angle); parallel (distance); contained |
| Plane – plane | identical; parallel (distance); intersecting (line, angle) |
| Point – line, plane, circle | on it or not |
| Point – sphere | inside, on or outside, with the distance to the sphere |
| Sphere – line | secant (two points), tangent (the point), passant |
| Sphere – plane | intersection circle, tangent (the point), no common point (distance) |
| Sphere – sphere | identical, concentric, apart (the gap), one inside the other, touching (the point, the common tangent plane), intersecting (the circle and the Potenzebene) |
| Circle – line | in the circle's plane: secant, tangent, passant; otherwise through the circle (the point) or no common point |
| Circle – plane | two points, touching (the point), no common point, contained, parallel |

### Tasks

A task (`interface::Task`) works on objects of the scene, named as there (a vector as `\vec{a}`):

| Task | Objects | Answer |
|---|---|---|
| `relation` | any pair of §4 *Pairs* | the relation and what comes with it |
| `distance` | point, line, plane, sphere (in pairs) | the distance, through the relation where one is needed |
| `angle` | two vectors, lines or planes | the angle |
| `foot` | a point onto a line or plane | the foot of the perpendicular |
| `reflect` | a point in a point, line or plane | the image |
| `midpoint` | two points | the midpoint |
| `tangent` | a sphere or circle at a point | the tangent plane or tangent |
| `convert` | a plane into parameter, normal or coordinate form | the plane in that form |
| `measure` | a vector (length), sphere (surface, volume), circle (circumference, area) | the measure |
| `evaluate` | an expression with the scene's points and vectors | its value: a number, vector or point |

A pair that does not fit the task is an error (`A distance does not work for these objects.`).

### Output

- Exact values stay exact; `exact_roots` is on in this mode, so $\sqrt{12}$ is $2\sqrt{3}$.
- Points are written $(1|2|3)$, vectors as columns, lines and planes in the form they have, a
  sphere in coordinate form; a circle in the x-y plane as $(x - a)^2 + (y - b)^2 = r^2$, any other
  as the sphere around its centre together with its plane:
  $\left(\vec{x} - \vec{m}\right)^2 = r^2, \quad \left(\vec{x} - \vec{m}\right) \cdot \vec{n} = 0$.
- Angles follow the `angle_unit` setting. One with a closed form is written as it, with the
  decimal unless it is a whole number ($45^\circ$, $\frac{\pi}{4} \approx 0.785398$); otherwise
  as $\arccos(…)$ or $\arcsin(…)$ with the decimal:
  $\arcsin\left(\frac{\sqrt{7}}{14}\right) \approx 10.8934^\circ$.
- When every input was 2D, points and vectors are written without z. The view stays 3D.
- A relation also returns its keyword (`TaskResponse::relation`): `identical`, `parallel`,
  `intersecting`, `skew`, `contained`, `on`, `off`, `inside`, `outside`, `secant`, `tangent`,
  `passant`, `circle`, `apart`, `concentric`, `touching`, `crossing`, `cut`, `touch`, `miss`;
  the shell names it in its language.

## 5. Engine interface

| Function | Request | Response |
|---|---|---|
| `api::geometry` | `GeometryRequest { scene, task? }` | `GeometryResponse { objects[], task? }` |
| `api::geometry_view` | `ViewRequest { scene, yaw, pitch, zoom, hatch }` | `ViewResponse { primitives[], objects[], camera, bounds }` |

- `ObjectResponse { name, kind, latex, steps[], error? }`: each object as built, with its name
  (`A = (1|2|3)`, `g: \vec{x} = …`) and the working of a construction that has one (a line
  through two points, a sphere through four).
- `TaskResponse { result[], relation?, steps[], alignBlock, error? }`: `result` holds the lines
  shown at Brief, `alignBlock` the result and the steps as an `align*` block for **Copy**.
- Both are stateless and touch no session. The shell keeps the scene and the task in the
  session's `ModeInputs` (`geometry`, the `GeometryRequest`), so it survives a mode switch and is
  saved with the session.
- The Tauri commands are `geometry` and `geometry_view`.

## 6. Steps

Each note of a `Working` has a text (through `msg!`, English and German), a formula and the
lowest level it shows at:

| Level | What is shown |
|---|---|
| Brief | The result lines: the point, the equation, the angle, the distance, the relation. |
| Standard | The school method: the equation set up, the vector computed, the values that come out, the check of a third row. |
| Detailed | The formula with the values put in: the system written out, the cross product component by component, the discriminant, the foot of a perpendicular as a point. |
| Maximal | The checks of a result (the intersection point in the other object, the other points of a sphere through four) and the dot products row by row. |

A plane in another form is put into the coordinate form first, and that conversion is part of
the working. The worked examples, level by level (two lines, a plane from parameter to
coordinate form, a line and a plane, a sphere and a plane), are in
[step-detail.md § Geometry mode](step-detail.md#geometry-mode).

A task whose working has no Brief line of its own (the foot of a perpendicular, which also
stands alone as part of other workings) gets its last line as the result.

## 7. The 3D view

One system for the desktop and the handheld, which has only embedded-graphics, so no three.js:
the core takes the scene and a camera and returns 2D primitives; each shell only draws them.
Plain `f64`, no caches.

- **Camera** (`view::Camera`): yaw, pitch, zoom and centre. The default is the school view (x to
  the lower left, y to the right, z up; yaw 25°, pitch 20°); *from above* is x to the right, y up.
  `zoom` in the request is on top of the fit, so 1 shows the whole box.
- **Box**: an auto-fit 3D box around the scene (`Bounds::fit`), with the origin in it.
- **Primitives** (`view::Primitive`), far to near, labels last: `segment` (with a stroke
  `axis`, `tick`, `object`, `hatch` or `hidden`), `polygon`, `marker` and `label`. A primitive's
  `object` is an index into the response's `objects`.
- **Figures**: axes with round ticks and their numbers; points as markers with their names;
  vectors as arrows from the origin; lines clipped to the box; planes as the polygon where they
  cut the box, with an optional hatch; spheres as an outline with rings of latitude and
  longitude, their far halves hidden; circles as rings of short segments (the handheld draws
  short 1 px lines fast, a long polyline slowly).
- **Depth**: sorted in the core; crossing planes are split along their intersection line, and
  lines and axes are cut where they pass through a plane, so the order stays right.

The desktop draws the primitives in SVG (`GeometryPlot.tsx`): planes as translucent polygons, a
colour per object; drag turns the scene, the wheel zooms, with buttons for *View from above* and
*Reset*. A WebGL view could replace it without touching the engine.

## 8. Desktop page

From top to bottom: the typed line, the view, the object list with a form per object (kind,
variant, name, MathLive cells), and the task panel (task, its objects, the answer with its
steps and **Copy**). The page works the scene out again on every change and when the language,
precision, step detail or angle unit changes; an answer to an older request is dropped. All labels
are in `en.ts` and `de.ts` under `mode.geometry` and `geometry.*`.

## 9. Later

- The handheld: Geometrie tiles and OPTN, drawing the shared primitives, the VctA–D editor of
  [handheld-menus.md](handheld-menus.md) §3.7.
- Sets of lines and planes with a parameter (`E_a`).
- Shape formulas and triangle solving.
