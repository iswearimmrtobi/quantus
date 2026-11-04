# Geometry mode (Geometrie)

Vectors, points, lines and planes with a 3D coordinate system. Replaces the "Analytic geometry"
bullet of [new-areas-and-themes.md](new-areas-and-themes.md) and the "Geometrie" lines in
[plans.md](plans.md) §5 and [handheld-menus.md](../design/handheld-menus.md) §3.7/§8 (the Casio
Vektorrechnung goes here, not into its own mode).

## Context

Nothing exists yet: no vector type, no vector syntax, no 3D drawing. This adds the mode, desktop
first, with the maths and the 3D projection in `quantus-core` so the handheld can reuse both later
(shells only add UI and storage).

Decisions so far: desktop first; objects entered through forms **and** typed expressions; results
exact, with steps at the existing `step_detail` levels.

## 3D view and the handheld

One system for both, but not three.js: it needs a browser, and the handheld has only
embedded-graphics. The shared part is **a camera and a projection in `quantus-core`**: the engine
takes the scene (points, lines, planes) and a camera (yaw, pitch, zoom, centre) and returns 2D
primitives (axes with ticks, point markers, line segments clipped to the view, plane patches as
polygon outlines plus an optional hatch). Each shell only draws them:

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
Koordinatenform, Abstand (distance), Schnittpunkt / Schnittgerade (intersection), Schnittwinkel
(angle), Skalarprodukt (dot), Kreuzprodukt (cross), Betrag (length), Lotfußpunkt (foot of
perpendicular), windschief (skew), parallel, identisch, Spiegelung (reflection), Mittelpunkt.

Operations of the first version:

- **Points and vectors:** distance, midpoint, vector between points, length, unit vector, dot,
  cross, angle, collinear and coplanar test, triple product (Spatprodukt), area of triangle and
  parallelogram, volume of tetrahedron and parallelepiped.
- **Line:** from two points or point and direction; point on line?; relation of two lines
  (identical, parallel, intersecting, skew) with intersection point and angle.
- **Plane:** from three points, point and two directions, point and normal; convert between
  parameter, normal and coordinate form; point on plane?
- **Mixed:** line–plane (intersection point, angle, parallel, contained), plane–plane (intersection
  line, angle), distance point–line, point–plane, line–line (skew), foot of perpendicular,
  reflection of a point.

All exact in `Value` (fractions, roots, `π`). The angle unit follows the planned Grad/Bogenmaß
setting.

## Implementation plan

One commit per step, on `dev`, each leaving `cargo test`, `cargo clippy` and `npm run typecheck`
green.

### Phase 1: vectors in the engine

- [ ] `crates/quantus-core/src/geometry/` (new module, `pub` in `lib.rs`): `Vector3 { [Value; 3] }`
  (2D input is z = 0), `dot`, `cross`, `length`, `unit`, `angle`, built on `num::Value`; reuse
  `linear::gauss` and `matrix.rs` for solving and for determinants (Spatprodukt).
- [ ] Vector syntax in the parser and evaluator for the typed path: `\begin{pmatrix}`, `(1|2|3)`
  and `\vec{a}`; `×` is the cross product, `\cdot` the dot product, `|v|` the length (as Casio
  §3.7). *Check first* how `MatrixEvaluator` in `matrix.rs` reads `pmatrix`, and extend that
  instead of duplicating it.
- [ ] Tests in `crates/quantus-core/tests/geometry.rs`; an entry in
  `docs/docs/contributing/testing.md`.

### Phase 2: objects and relations

- [ ] `Point`, `Line`, `Plane` (lines and planes keep the form they were given in, plus
  conversions); a function per operation above, each returning the result and a `Working` log in
  the `msg!` bilingual style (`crate::text`), like `matrix.rs`.
- [ ] Degenerate cases are results, not errors (parallel, identical, skew, point on line).
- [ ] Steps at Brief, Standard, Detailed and Maximal; a *Geometry* section with worked examples in
  `design/step-detail.md`.

### Phase 3: API and session

- [ ] `Mode::Geometry` in `settings.rs` (also the `units:` match and the serde test); `'geometry'`
  in `desktop/src/api.ts`.
- [ ] `api.rs`: `geometry(settings, GeometryRequest)` with a `Scene { objects }` and a task
  (`Relation(a, b)`, `Distance(a, b)`, `Convert(plane, form)`, …). The response holds the exact
  result, LaTeX, steps and the copy block (`align*`). Typed input goes through the same entry and
  is parsed into objects.
- [ ] The scene is kept per session in `ModeInputs` (`session.rs`, `get_mode_inputs` and
  `set_mode_inputs`).
- [ ] Tauri command in `desktop/src-tauri/src/commands.rs`, registered in `main.rs`.

### Phase 4: projection (shared drawing)

- [ ] `geometry/view.rs`: `Camera` and `project(scene, camera, viewport) -> Vec<Primitive>`: axes
  with ticks and labels, points, clipped line segments, plane polygons, depth order, an auto-fit
  window for the scene. Plain `f64`, no caches (see the ESP32 memory notes in
  [plans.md](plans.md) §4).
- [ ] Tests: known projections, clipping, depth order.

### Phase 5: desktop UI

- [ ] `components/GeometryView.tsx`, built like `LinearSystemView` (state in `App.tsx`, saved in
  the session, survives a mode switch), and `components/GeometryPlot.tsx` (SVG like
  `FunctionPlot.tsx`: drag rotates, wheel zooms, a reset button, named labels, a colour per
  object).
- [ ] Object list with forms: add Punkt, Gerade or Ebene; MathLive cells for the coordinates (as in
  `MatrixGrid.tsx`); a form switch for lines and planes. A typed-expression field as the shortcut
  (`A=(1,2,3)`, `g: X=A+\lambda(…)`) that creates or updates the same objects.
- [ ] Task panel: pick objects, pick relation, distance, …; shows the result and `Steps`.
- [ ] `en.ts` and `de.ts` (all labels), `PLACEHOLDERS` in `App.tsx`, `modeChoices` in
  `SettingsDialog.tsx`, the `mode.geometry` strings.

### Phase 6: docs

- [ ] Spec `design/geometry.md` (glossary, operations, step layouts); a guide in
  `docs/docs/guides/`; the settings reference; `CHANGELOG.md` under `[Unreleased]`; remove the
  "Later" lines for Geometrie in `plans.md` §5, `new-areas-and-themes.md` and
  `firmware/docs/features.md`.

### Later (separate plan)

- The handheld: Geometrie tiles and OPTN (`firmware/ui/src/tree.rs`, `app.rs`,
  `firmware/engine/src/lib.rs` `core_mode`), drawing the shared primitives, the VctA–D editor from
  handheld-menus.md §3.7.
- Shape formulas and triangle solving from [new-areas-and-themes.md](new-areas-and-themes.md)
  (separate from analytic geometry); spheres and circles (Kugel, Kreis) as further object types.

## Open points

1. German names and the operation list above: to be corrected and extended.
2. Whether Kugel and Kreis belong in the first version (currently: later).

## Verification

- `cargo test -p quantus-core --test geometry` (targeted runs only),
  `cargo clippy -p quantus-core --all-targets`, `cd desktop && npm run typecheck`.
- By hand: `npm run dev`; enter points, a line and a plane, rotate and zoom the view, check the
  relations and steps in German and English at each detail level.
