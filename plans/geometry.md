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
exact, with steps at the existing `step_detail` levels.

- Typed syntax: vectors `(1|2|3)`, `\begin{pmatrix}`, `\vec{a}`; points `(1,2,3)`. `|v|` stays the
  length; the parser tells `(a|b|c)` from `|v|` by the parentheses.
- `Point` and `Vector3` are separate types (Point − Point = Vector, Point + Vector = Point).
- Angles follow the existing `angle_unit` setting (`AngleUnit` in `settings.rs`).
- A `Circle` is stored as centre, radius and normal; "Kugel ∩ Ebene" is a constructor that
  converts into it.

## 3D view and the handheld

One system for both, but not three.js: it needs a browser, and the handheld has only
embedded-graphics. The shared part is **a camera and a projection in `quantus-core`**: the engine
takes the scene (points, lines, planes) and a camera (yaw, pitch, zoom, centre) and returns 2D
primitives (axes with ticks, point markers, line segments clipped to the view, plane patches as
polygon outlines plus an optional hatch, spheres as an outline circle with a few latitude and
longitude rings, circles as a polyline of short segments). Each shell only draws them:

- desktop: SVG, like `FunctionPlot.tsx` (drag rotates, wheel zooms, no new dependency);
- handheld: short 1 px lines (a long 2 px polyline took 7 s, short 1 px pieces at most 218 ms, see
  [plans.md](plans.md) §4), keys rotate.

Planes are translucent polygons on the desktop and outlines on the handheld, both from the same
primitives. Depth sorting happens in the core. A WebGL view can replace the SVG one later without
touching the engine.

## Concepts (English, German names to confirm)

The glossary goes into the design spec once the German names are settled.
Proposed: Punkt (point), Vektor (vector), Gerade (line), Ebene (plane), Stützvektor (position
vector), Richtungsvektor (direction), Normalenvektor (normal), Parameterform, Normalenform,
Koordinatenform, Kugel (sphere), Kreis (circle), Mittelpunkt, Radius, Tangente, Tangentialebene,
Abstand (distance), Schnittpunkt / Schnittgerade (intersection), Schnittwinkel
(angle), Skalarprodukt (dot), Kreuzprodukt (cross), Betrag (length), Lotfußpunkt (foot of
perpendicular), windschief (skew), parallel, identisch, Spiegelung (reflection).

Operations of the first version:

- **Points and vectors:** distance, midpoint, vector between points, length, unit vector, dot,
  cross, angle, collinear and coplanar test, triple product (Spatprodukt), area of triangle and
  parallelogram, volume of tetrahedron and parallelepiped.
- **Line:** from two points or point and direction; point on line?; relation of two lines
  (identical, parallel, intersecting, skew) with intersection point and angle.
- **Plane:** from three points, point and two directions, point and normal; convert between
  parameter, normal and coordinate form; point on plane?
- **Sphere (Kugel):** from centre and radius, centre and a point, or four points; equation in
  coordinate and vector form; point inside, on or outside; relation to a line (secant, tangent,
  passant, with the intersection points); relation to a plane (intersection circle, tangent, no
  contact); tangent plane at a point on the sphere; relation of two spheres; surface and volume.
- **Circle (Kreis):** in a plane in 3D, as a sphere cut by a plane (centre, radius and plane from
  the cut) or from centre, radius and normal; relation to a line (intersection points, tangent);
  circumference and area. Plain 2D circles (`(x-a)²+(y-b)²=r²`) are the z = 0 case.
- **Mixed:** line–plane (intersection point, angle, parallel, contained), plane–plane (intersection
  line, angle), distance point–line, point–plane, line–line (skew), foot of perpendicular,
  reflection of a point.

All exact in `Value` (fractions, roots, `π`). The angle unit follows the `angle_unit` setting.

## Implementation plan

One commit per step, on `dev`, each leaving `cargo test`, `cargo clippy` and `npm run typecheck`
green.

### Phase 1: vectors in the engine

- [ ] `geometry/` module (`pub` in `lib.rs`) with `Vector3 { [Value; 3] }` and a separate `Point`
  (2D input is z = 0), built on `num::Value`; `Point − Point`, `Point + Vector`, `Vector ± Vector`,
  scalar multiple. Test file `tests/geometry.rs` created with these.
- [ ] `dot`, `length`, `unit`, `angle` (in the `angle_unit` setting), exact in `Value`.
- [ ] `cross` and the triple product (Spatprodukt) via the determinant in `matrix.rs`; `collinear`
  and `coplanar` tests.
- [ ] Areas and volumes: triangle, parallelogram, tetrahedron, parallelepiped.
- [ ] Parser, *check first* how `MatrixEvaluator` in `matrix.rs` reads `pmatrix` and extend that
  instead of duplicating it: `\begin{pmatrix}`, `(1|2|3)` and `\vec{a}` read as vectors, with tests.
- [ ] Parser: points `(1,2,3)`, with tests.
- [ ] Evaluator: `×` is the cross product, `\cdot` the dot product, `|v|` the length (as Casio §3.7),
  with tests.
- [ ] Entry in `docs/docs/contributing/testing.md`.

### Phase 2: objects and relations

Every function returns the result and a `Working` log in the `msg!` bilingual style
(`crate::text`), like `matrix.rs`. Degenerate cases (parallel, identical, skew, point on line) are
results, not errors. Each step below brings its tests.

- [ ] `Line`: from two points or point and direction; point on line?; distance point–line; foot of
  perpendicular.
- [ ] `Line` × `Line`: identical, parallel, intersecting, skew, with intersection point, angle and
  distance.
- [ ] `Plane`: from three points, point and two directions, point and normal; conversions between
  parameter, normal and coordinate form (lines and planes keep the form they were given in);
  point on plane?
- [ ] Mixed line–plane: intersection point, angle, parallel, contained.
- [ ] Mixed plane–plane: intersection line, angle; distance point–plane; reflection of a point.
- [ ] `Sphere`: from centre and radius, centre and a point, or four points; coordinate and vector
  form; point inside, on or outside; surface and volume.
- [ ] Sphere × line (secant, tangent, passant, intersection points) and sphere × plane
  (intersection circle, tangent, no contact); tangent plane at a point.
- [ ] Sphere × sphere.
- [ ] `Circle` (centre, radius, normal): direct constructor, the "Kugel ∩ Ebene" constructor,
  circumference and area; the plain 2D circle as the z = 0 case.
- [ ] Circle × line (intersection points, tangent).
- [ ] Steps at Brief, Standard, Detailed and Maximal across all of the above, and a *Geometry*
  section with worked examples in `design/step-detail.md`.

### Phase 3: API and session

- [ ] `Mode::Geometry` in `settings.rs` (also the `units:` match and the serde test); `'geometry'`
  in `desktop/src/api.ts`.
- [ ] `api.rs`: `Scene { objects }`, `GeometryRequest` and the task enum (`Relation(a, b)`,
  `Distance(a, b)`, `Convert(plane, form)`, …) with `geometry(settings, request)` for the objects
  of Phase 2; the response holds the exact result, LaTeX, steps and the copy block (`align*`).
- [ ] Typed input goes through the same entry and is parsed into objects (`A=(1,2,3)`,
  `g: X=A+\lambda(…)`, `K: (x-1)^2+y^2+z^2=9`).
- [ ] The scene is kept per session in `ModeInputs` (`session.rs`, `get_mode_inputs` and
  `set_mode_inputs`).
- [ ] Tauri command in `desktop/src-tauri/src/commands.rs`, registered in `main.rs`, and the
  wrapper in `api.ts`.

### Phase 4: projection (shared drawing)

Plain `f64`, no caches (see the ESP32 memory notes in [plans.md](plans.md) §4). Each step brings
its tests.

- [ ] `geometry/view.rs`: `Camera` (yaw, pitch, zoom, centre), `Primitive`, and `project` for axes
  with ticks and labels and for points.
- [ ] Line segments clipped to the view.
- [ ] Plane polygons (outline plus optional hatch).
- [ ] Sphere (outline circle plus latitude and longitude rings) and circle polylines.
- [ ] Depth order across all primitives.
- [ ] Auto-fit window for the scene.

### Phase 5: desktop UI

- [ ] `GeometryView.tsx` shell, built like `LinearSystemView`: state in `App.tsx`, saved in the
  session, survives a mode switch; `modeChoices` in `SettingsDialog.tsx`, `PLACEHOLDERS`, the
  `mode.geometry` strings in `en.ts` and `de.ts`.
- [ ] `GeometryPlot.tsx`: SVG like `FunctionPlot.tsx`, drawing the primitives of a fixed demo
  scene with axes.
- [ ] Plot interaction: drag rotates, wheel zooms, a reset button.
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
  `new-areas-and-themes.md` and `firmware/docs/features.md`.

### Later (separate plan)

- The handheld: Geometrie tiles and OPTN (`firmware/ui/src/tree.rs`, `app.rs`,
  `firmware/engine/src/lib.rs` `core_mode`), drawing the shared primitives, the VctA–D editor from
  handheld-menus.md §3.7.
- Shape formulas and triangle solving from [new-areas-and-themes.md](new-areas-and-themes.md)
  (separate from analytic geometry).

## Open points

1. German names: to be confirmed (the operation list is settled for the first version).
2. Decided: Kugel and Kreis belong to the first version (Phase 2, 4 and 5 below).

## Verification

- `cargo test -p quantus-core --test geometry` (targeted runs only),
  `cargo clippy -p quantus-core --all-targets`, `cd desktop && npm run typecheck`.
- By hand: `npm run dev`; enter points, a line and a plane, rotate and zoom the view, check the
  relations and steps in German and English at each detail level.
