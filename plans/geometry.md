# Geometry mode (Geometrie)

Vectors, points, lines, planes, spheres and circles with a 3D coordinate system. Replaces the "Analytic geometry"
bullet of [new-areas-and-themes.md](new-areas-and-themes.md) and the "Geometrie" lines in
[plans.md](plans.md) §5 and [handheld-menus.md](../design/handheld-menus.md) §3.7/§8 (the Casio
Vektorrechnung goes here, not into its own mode).

## Context

Nothing exists yet: no vector type, no vector syntax, no 3D drawing. This adds the mode, desktop
first, with the maths and the 3D projection in `quantus-core` so the handheld can reuse both later
(shells only add UI and storage).

Decisions so far: desktop first; objects entered through forms **and** typed expressions; results
exact, with steps at the existing `step_detail` levels; Kugel and Kreis belong to the first
version.

- Typed syntax in school notation: points `P(1|2|3)` (or `(1|2|3)` in a point field), vectors
  `\begin{pmatrix}` or `\vec{a}`; `;` is accepted as a separator too. No `(1,2,3)`: `1,2` is
  already a decimal number (`latex/lexer.rs`). `|v|` stays the length; a `|` right after `(` or
  after a coordinate separator starts no absolute value, and an absolute value inside a coordinate
  is written `\left|…\right|`.
- The new syntax applies only in Geometry mode. Inputs are read as in `Algebra`; `\lambda`, `\mu`,
  `\sigma`, `\tau` are parameters there, not the micro prefix.
- `Point` and `Vector3` are their own types (Point − Point = Vector, Point + Vector = Point), not
  3×1 matrices. Matrix mode stays unchanged (no `\times` or `|v|` there); `matrix.rs` lends its
  determinant.
- The types decide the operator: scalar·vector is the scalar multiple, vector·vector the dot
  product, vector×vector the cross product; `\times` between scalars stays multiplication.
- Named objects (`A`, `g`, `E`, `K`, `\vec{a}`) live only in the scene (`ModeInputs`), in their
  own namespace like the matrices of matrix mode: not in the variable list, and a variable may
  share a name with an object. `x`, `y`, `z` in a coordinate equation are always coordinates.
- Coordinates are numbers only; a free variable in a coordinate is a clear error (sets like `E_a`
  come later).
- Angles follow the existing `angle_unit` setting (`AngleUnit` in `settings.rs`), shown as the
  exact `\arccos(…)` form plus the decimal.
- Results drop z when every input was 2D; the view stays 3D, with a "view from above" button.
- A `Circle` is stored as centre, radius and normal; "Kugel ∩ Ebene" is a constructor that
  converts into it.

## 3D view and the handheld

One system for both, but not three.js: it needs a browser, and the handheld has only
embedded-graphics. The shared part is **a camera and a projection in `quantus-core`**: the engine
takes the scene (points, lines, planes) and a camera (yaw, pitch, zoom, centre) and returns 2D
primitives (axes with ticks, point markers, line segments, plane patches as polygon outlines plus
an optional hatch, spheres as an outline circle with a few latitude and longitude rings, circles as
a polyline of short segments). Each shell only draws them:

- desktop: SVG, like `FunctionPlot.tsx` (drag rotates, wheel zooms, no new dependency);
- handheld: short 1 px lines (a long 2 px polyline took 7 s, short 1 px pieces at most 218 ms, see
  [plans.md](plans.md) §4), keys rotate.

The window is a 3D box around the scene (auto-fit); lines are clipped to it, and a plane is drawn
as the polygon where it cuts the box. Planes are translucent polygons on the desktop and outlines
on the handheld, both from the same primitives. Depth sorting happens in the core, and plane
polygons are split along their intersection lines so the order stays right. A WebGL view can
replace the SVG one later without touching the engine.

## Concepts (English, German names to confirm)

The glossary goes into the design spec once the German names are settled.
Proposed: Punkt (point), Vektor (vector), Gerade (line), Ebene (plane), Stützvektor (position
vector), Richtungsvektor (direction), Normalenvektor (normal), Parameterform, Normalenform,
Koordinatenform, Kugel (sphere), Kreis (circle), Mittelpunkt, Radius, Tangente, Tangentialebene,
Abstand (distance), Schnittpunkt / Schnittgerade (intersection), Schnittwinkel
(angle), Skalarprodukt (dot), Kreuzprodukt (cross), Betrag (length), Lotfußpunkt (foot of
perpendicular), windschief (skew), parallel, identisch, Spiegelung (reflection), Schnittkreis
(intersection circle), Potenzebene (plane of the intersection of two spheres).

Operations of the first version:

- **Points and vectors:** distance, midpoint, vector between points, length, unit vector, dot,
  cross, angle, collinear and coplanar test, triple product (Spatprodukt), area of triangle and
  parallelogram, volume of tetrahedron and parallelepiped.
- **Line:** from two points or point and direction; point on line?; relation of two lines
  (identical, parallel, intersecting, skew) with intersection point, angle and distance.
- **Plane:** from three points, point and two directions, point and normal; convert between
  parameter, normal and coordinate form; point on plane?
- **Sphere (Kugel):** from centre and radius, centre and a point, or four points; equation in
  coordinate and vector form; point inside, on or outside, and its distance to the sphere; relation
  to a line (secant, tangent, passant, with the intersection points); relation to a plane
  (intersection circle, tangent, no contact, with the distance); tangent plane at a point on the
  sphere; relation of two spheres with the intersection circle and the Potenzebene; surface and
  volume.
- **Circle (Kreis):** in a plane in 3D, as a sphere cut by a plane (centre, radius and plane from
  the cut) or from centre, radius and normal; point on circle?; relation to a line (intersection
  points, tangent); tangent at a point of the circle; relation to a plane (intersection points,
  contained, no contact); circumference and area. Plain 2D circles (`(x-a)²+(y-b)²=r²`) are the
  z = 0 case.
- **Mixed:** line–plane (intersection point, angle, parallel with distance, contained),
  plane–plane (intersection line, angle, parallel with distance), distance point–line,
  point–plane, line–line (skew), foot of perpendicular, reflection of a point in a point, a line
  or a plane.

All exact in `Value` (fractions, roots, `π`). The angle unit follows the `angle_unit` setting.

## Implementation plan

One commit per step, on `dev`, each leaving `cargo test`, `cargo clippy` and `npm run typecheck`
green.

### Phase 1: vectors in the engine

- [x] `Mode::Geometry` in `settings.rs` (read as in `Algebra`; also the `units:` match and the
  serde test); `'geometry'` in `desktop/src/api.ts`.
- [x] `geometry/` module (`pub` in `lib.rs`) with `Vector3 { [Value; 3] }` and a separate `Point`
  (2D input is z = 0, remembered for the output), built on `num::Value`; `Point − Point`,
  `Point + Vector`, `Vector ± Vector`, scalar multiple. Test file `tests/geometry.rs` created with
  these.
- [x] `dot`, `length`, `unit`, `angle` (exact `\arccos` plus decimal, in the `angle_unit` setting),
  exact in `Value`.
- [x] `cross` and the triple product (Spatprodukt) via the determinant in `matrix.rs`; `collinear`
  and `coplanar` tests.
- [x] Areas and volumes: triangle, parallelogram, tetrahedron, parallelepiped.
- [x] Parser, Geometry mode only, *check first* how `MatrixEvaluator` in `matrix.rs` reads
  `pmatrix` and reuse that instead of duplicating it: `\begin{pmatrix}`, `\vec{a}` read as
  vectors, with tests.
- [x] Parser: points `P(1|2|3)` and `(1|2|3)`, `;` as a separator, the rule that tells `(a|b|c)`
  from `|v|`, with tests (nested bars, `\left|…\right|` in a coordinate, `(1,5|2|3)` with a
  decimal comma).
- [x] Evaluator: `×` is the cross product, `\cdot` the dot product (scalar·vector the multiple),
  `|v|` the length (as Casio §3.7), with tests.
- [x] Entry in `docs/docs/contributing/testing.md`.

### Phase 2: objects and relations

Every function returns the result and a `Working` log in the `msg!` bilingual style
(`crate::text`), like `matrix.rs`. Degenerate cases (parallel, identical, skew, point on line) are
results, not errors. Each step below brings its tests.

- [x] `Line`: from two points or point and direction; point on line?; distance point–line; foot of
  perpendicular.
- [x] `Line` × `Line`: identical, parallel, intersecting, skew, with intersection point, angle and
  distance.
- [x] `Plane`: from three points, point and two directions, point and normal; conversions between
  parameter, normal and coordinate form (lines and planes keep the form they were given in);
  point on plane?
- [x] Mixed line–plane: intersection point, angle, parallel (with distance), contained.
- [x] Mixed plane–plane: intersection line, angle, parallel (with distance); distance point–plane.
- [x] Reflection of a point in a point, a line and a plane.
- [x] `Sphere`: from centre and radius, centre and a point, or four points; coordinate and vector
  form; point inside, on or outside, with its distance to the sphere; surface and volume.
- [x] Sphere × line (secant, tangent, passant, intersection points) and sphere × plane
  (intersection circle, tangent, no contact, distance); tangent plane at a point.
- [x] Sphere × sphere: relation, intersection circle and Potenzebene.
- [x] `Circle` (centre, radius, normal): direct constructor, the "Kugel ∩ Ebene" constructor,
  point on circle?, circumference and area; the plain 2D circle as the z = 0 case.
- [x] Circle × line (intersection points, tangent) and the tangent at a point of the circle.
- [x] Circle × plane (intersection points, contained, no contact).
- [x] Steps at Brief, Standard, Detailed and Maximal across all of the above, and a *Geometry*
  section with worked examples in `design/step-detail.md`.

### Phase 3: API and session

- [x] `api.rs`: `Scene { objects }`, `GeometryRequest` and the task enum (`Relation(a, b)`,
  `Distance(a, b)`, `Convert(plane, form)`, …) with `geometry(settings, request)` for the objects
  of Phase 2; the response holds the exact result, LaTeX, steps and the copy block (`align*`).
- [ ] Typed input goes through the same entry and is parsed into objects by a reader of its own
  for `Name: …` and `Name = …` (the lexer drops `:`): `A(1|2|3)`, `g: X=A+\lambda(…)`,
  `K: (x-1)^2+y^2+z^2=9`. Equations are classified: linear in x, y, z is a plane; x², y², z² with
  the same coefficient and no mixed terms is a sphere; anything else is an error.
- [ ] The scene is kept per session in `ModeInputs` (`session.rs`, `get_mode_inputs` and
  `set_mode_inputs`), as its own namespace.
- [ ] Tauri command in `desktop/src-tauri/src/commands.rs`, registered in `main.rs`, and the
  wrapper in `api.ts`.

### Phase 4: projection (shared drawing)

Plain `f64`, no caches (see the ESP32 memory notes in [plans.md](plans.md) §4). Each step brings
its tests.

- [ ] `geometry/view.rs`: `Camera` (yaw, pitch, zoom, centre), `Primitive`, and `project` for axes
  with ticks and labels and for points.
- [ ] Auto-fit 3D box for the scene.
- [ ] Line segments clipped to the box.
- [ ] Plane polygons from the cut with the box (outline plus optional hatch).
- [ ] Sphere (outline circle plus latitude and longitude rings) and circle polylines.
- [ ] Depth order across all primitives, with plane polygons split along their intersection lines.

### Phase 5: desktop UI

- [ ] `GeometryView.tsx` shell, built like `LinearSystemView`: state in `App.tsx`, saved in the
  session, survives a mode switch; `modeChoices` in `SettingsDialog.tsx`, `PLACEHOLDERS`, the
  `mode.geometry` strings in `en.ts` and `de.ts`.
- [ ] `GeometryPlot.tsx`: SVG like `FunctionPlot.tsx`, drawing the primitives of a fixed demo
  scene with axes.
- [ ] Plot interaction: drag rotates, wheel zooms, a reset button and a "view from above" button.
- [ ] Plot labels and a colour per object, fed from the real scene.
- [ ] Object list with forms for Punkt and Vektor: MathLive cells for the coordinates (as in
  `MatrixGrid.tsx`), add and remove.
- [ ] Forms for Gerade and Ebene, with the form switch (parameter, normal, coordinate).
- [ ] Forms for Kugel and Kreis (including the Kugel ∩ Ebene constructor).
- [ ] Typed-expression field that creates or updates the same objects.
- [ ] Task panel: pick objects, pick relation or distance, show the result.
- [ ] Task panel shows the `Steps`; the copy block.
- [ ] `en.ts` and `de.ts`: remaining labels, checked for completeness.

### Phase 6: docs

- [ ] Spec `design/geometry.md` (glossary, operations, step layouts).
- [ ] Guide in `docs/docs/guides/` and the settings reference.
- [ ] `CHANGELOG.md` under `[Unreleased]`; remove the "Later" lines for Geometrie in `plans.md` §5,
  `new-areas-and-themes.md`, `firmware/docs/features.md` and `design/handheld-menus.md` (the
  open list, §8 and the feature table).

### Later (separate plan)

- The handheld: Geometrie tiles and OPTN (`firmware/ui/src/tree.rs`, `app.rs`,
  `firmware/engine/src/lib.rs` `core_mode`), drawing the shared primitives, the VctA–D editor from
  handheld-menus.md §3.7.
- Sets of lines and planes with a parameter (`E_a`), which need symbolic coordinates.
- Shape formulas and triangle solving from [new-areas-and-themes.md](new-areas-and-themes.md)
  (separate from analytic geometry).

## Open points

1. German names: to be confirmed (the operation list is settled for the first version).

## Verification

- `cargo test -p quantus-core --test geometry` (targeted runs only),
  `cargo clippy -p quantus-core --all-targets`, `cd desktop && npm run typecheck`.
- By hand: `npm run dev`; enter points, a line and a plane, rotate and zoom the view, check the
  relations and steps in German and English at each detail level.
