//! The 3D view of a scene, shared by the shells: a [`Camera`] and [`project`], which turns the
//! scene's [`Figure`]s into flat [`Primitive`]s (segments, polygons, markers, labels) that a shell only
//! draws, as SVG on the desktop and as short 1 px lines on the handheld.
//!
//! [`project`] returns the primitives far to near, the labels last, so that a shell draws them
//! in order; planes are split along their intersection lines, and lines and axes where they
//! pass through a plane, so that the order stays right.
//!
//! Plain `f64`, no caches. Screen coordinates are in view units with the origin at the camera's
//! centre, x to the right and y **up**; a shell scales them to its pixels and flips y. With
//! [`Camera::fit`] the box of [`Bounds::fit`] fits into the unit circle.

use serde::{Deserialize, Serialize};

use crate::num::Value;

use super::{Circle, Line, Plane, Point, Sphere, Vector3};

/// A point or vector in plain floats.
pub type V3 = [f64; 3];
/// A point on the screen in view units, y up.
pub type V2 = [f64; 2];

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale(k: f64, a: V3) -> V3 {
    [k * a[0], k * a[1], k * a[2]]
}

fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn unit(a: V3) -> V3 {
    scale(1.0 / dot(a, a).sqrt(), a)
}

/// Two unit vectors that span the plane perpendicular to `n`, with `n` a right-handed frame.
fn frame(n: V3) -> (V3, V3) {
    let n = unit(n);
    // The axis least along n gives the most stable perpendicular.
    let axis = (0..3).min_by(|&i, &j| n[i].abs().total_cmp(&n[j].abs())).expect("three axes");
    let mut e = [0.0; 3];
    e[axis] = 1.0;
    let first = unit(cross(n, e));
    (first, cross(n, first))
}

/// An exact value as a float; `None` if it is not real.
pub fn approx(value: &Value) -> Option<f64> {
    value.decimal(17).ok()?.to_string().parse().ok()
}

fn approx3(coords: &[Value; 3]) -> Option<V3> {
    Some([approx(&coords[0])?, approx(&coords[1])?, approx(&coords[2])?])
}

/// The length of a tick on the screen, in view units.
pub const TICK: f64 = 0.02;
/// The length of the two strokes of an arrow head on the screen, in view units.
pub const ARROW: f64 = 0.04;
/// The number of segments of a ring.
pub const RING: usize = 48;
/// The latitudes of the rings drawn on a sphere, in degrees, and the number of its meridians.
const LATITUDES: [f64; 3] = [-45.0, 0.0, 45.0];
const MERIDIANS: usize = 4;

/// Where the scene is looked at from. The viewer sits in the direction of yaw (around z, from
/// the x axis towards y) and pitch (up from the x-y plane), looking at `centre`; `zoom` is view
/// units per scene unit.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Camera {
    /// In radians.
    pub yaw: f64,
    /// In radians, from −π/2 (from below) to π/2 (from above).
    pub pitch: f64,
    pub zoom: f64,
    pub centre: V3,
}

impl Default for Camera {
    /// The school view: x towards the viewer to the lower left, y to the right, z up.
    fn default() -> Camera {
        Camera { yaw: 25f64.to_radians(), pitch: 20f64.to_radians(), zoom: 1.0, centre: [0.0; 3] }
    }
}

impl Camera {
    /// The view from above: x to the right, y up.
    pub fn from_above(self) -> Camera {
        Camera { yaw: -std::f64::consts::FRAC_PI_2, pitch: std::f64::consts::FRAC_PI_2, ..self }
    }

    /// This camera's angles, looking at the centre of `bounds` from far enough that the whole box
    /// fits into the unit circle at any angle.
    pub fn fit(self, bounds: &Bounds) -> Camera {
        let centre = [0, 1, 2].map(|i| (bounds.min[i] + bounds.max[i]) / 2.0);
        let radius = dot(sub(bounds.max, centre), sub(bounds.max, centre)).sqrt();
        Camera { zoom: 1.0 / radius, centre, ..self }
    }

    /// The unit vectors of the screen in the scene: right, up, and towards the viewer.
    fn basis(&self) -> [V3; 3] {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        let right = [-sy, cy, 0.0];
        let up = [-sp * cy, -sp * sy, cp];
        let towards = [cp * cy, cp * sy, sp];
        [right, up, towards]
    }

    /// The point on the screen and its depth (larger is nearer the viewer).
    pub fn project(&self, p: V3) -> (V2, f64) {
        let [right, up, towards] = self.basis();
        let q = sub(p, self.centre);
        ([self.zoom * dot(q, right), self.zoom * dot(q, up)], dot(q, towards))
    }
}

/// The box the view shows, from `min` to `max` on each axis.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Bounds {
    pub min: V3,
    pub max: V3,
}

/// The smallest half width of the box on each axis.
const MIN_HALF: f64 = 1.0;
/// The room around the scene, as a share of the box.
const MARGIN: f64 = 0.1;

impl Bounds {
    /// The box around the scene and the origin, with a margin, at least 2 wide on each axis.
    pub fn fit(figures: &[Shown]) -> Bounds {
        let mut min = [0.0; 3];
        let mut max = [0.0; 3];
        let mut take = |p: V3| {
            for i in 0..3 {
                min[i] = f64::min(min[i], p[i]);
                max[i] = f64::max(max[i], p[i]);
            }
        };
        for shown in figures {
            match &shown.figure {
                Figure::Point(p) | Figure::Vector(p) => take(*p),
                Figure::Line { support, .. } | Figure::Plane { support, .. } => take(*support),
                Figure::Sphere { centre, radius } => {
                    take(add(*centre, [*radius; 3]));
                    take(sub(*centre, [*radius; 3]));
                }
                Figure::Circle { centre, radius, normal } => {
                    // Along each axis a circle reaches r · √(1 − n²) with n its unit normal.
                    let n = unit(*normal);
                    let reach = n.map(|c| radius * (1.0 - c * c).max(0.0).sqrt());
                    take(add(*centre, reach));
                    take(sub(*centre, reach));
                }
            }
        }
        for i in 0..3 {
            let middle = (min[i] + max[i]) / 2.0;
            let half = ((max[i] - min[i]) / 2.0 * (1.0 + MARGIN)).max(MIN_HALF);
            min[i] = middle - half;
            max[i] = middle + half;
        }
        Bounds { min, max }
    }
}

/// An object of the scene in floats, for the view.
#[derive(Clone, Debug, PartialEq)]
pub enum Figure {
    Point(V3),
    /// Drawn from the origin to this tip.
    Vector(V3),
    Line { support: V3, direction: V3 },
    /// `normal · x = constant`, through `support`.
    Plane { normal: V3, constant: f64, support: V3 },
    Sphere { centre: V3, radius: f64 },
    Circle { centre: V3, radius: f64, normal: V3 },
}

impl Figure {
    pub fn point(p: &Point) -> Option<Figure> {
        approx3(&p.coords).map(Figure::Point)
    }

    pub fn vector(v: &Vector3) -> Option<Figure> {
        approx3(&v.coords).map(Figure::Vector)
    }

    pub fn line(g: &Line) -> Option<Figure> {
        Some(Figure::Line { support: approx3(&g.support.coords)?, direction: approx3(&g.direction.coords)? })
    }

    pub fn plane(e: &Plane) -> Option<Figure> {
        Some(Figure::Plane { normal: approx3(&e.normal.coords)?, constant: approx(&e.constant)?, support: approx3(&e.support.coords)? })
    }

    pub fn sphere(k: &Sphere) -> Option<Figure> {
        Some(Figure::Sphere { centre: approx3(&k.centre.coords)?, radius: approx(&k.radius)? })
    }

    pub fn circle(k: &Circle) -> Option<Figure> {
        Some(Figure::Circle { centre: approx3(&k.centre.coords)?, radius: approx(&k.radius)?, normal: approx3(&k.normal.coords)? })
    }
}

/// A named figure; its index in the list is the `object` of its primitives (the colour).
#[derive(Clone, Debug, PartialEq)]
pub struct Shown {
    pub name: String,
    pub figure: Figure,
}

/// What a segment belongs to, for its style.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Stroke {
    Axis,
    Tick,
    Object,
    /// The hatch of a plane.
    Hatch,
    /// A ring on the far side of a sphere, hidden by it.
    Hidden,
}

/// What a shell draws. `depth` is larger nearer the viewer; `object` is the index of the
/// figure, `None` for the axes.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Primitive {
    Segment { from: V2, to: V2, depth: f64, stroke: Stroke, object: Option<usize> },
    /// A piece of a plane or the outline of a sphere, closed, to be filled translucent or drawn
    /// as lines.
    Polygon { points: Vec<V2>, depth: f64, object: usize },
    /// The marker of a point.
    Marker { at: V2, depth: f64, object: usize },
    /// A name, an axis or a tick number, placed next to `at`.
    Label { at: V2, depth: f64, text: String, object: Option<usize> },
}

/// How the figures are drawn.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct Options {
    /// Hatch the planes, for a shell that draws them as outlines.
    pub hatch: bool,
}

/// The number of hatch lines across the box's diagonal.
const HATCHES: f64 = 24.0;

/// The corners of the box joined by each of its twelve edges, as bits of x, y and z.
const EDGES: [(usize, usize); 12] =
    [(0, 1), (2, 3), (4, 5), (6, 7), (0, 2), (1, 3), (4, 6), (5, 7), (0, 4), (1, 5), (2, 6), (3, 7)];

impl Bounds {
    fn corner(&self, bits: usize) -> V3 {
        [0, 1, 2].map(|i| if bits >> i & 1 == 1 { self.max[i] } else { self.min[i] })
    }

    fn diagonal(&self) -> f64 {
        let d = sub(self.max, self.min);
        dot(d, d).sqrt()
    }

    /// The polygon where the plane `normal · x = constant` cuts the box, its corners in order
    /// around it; empty if it misses the box or only touches an edge or a corner.
    pub fn cut(&self, normal: V3, constant: f64) -> Vec<V3> {
        let eps = 1e-9 * self.diagonal();
        let side = |p: V3| dot(normal, p) - constant;
        let mut points: Vec<V3> = Vec::new();
        let mut push = |p: V3| {
            if !points.iter().any(|q| dot(sub(*q, p), sub(*q, p)).sqrt() < eps) {
                points.push(p);
            }
        };
        for (a, b) in EDGES {
            let (a, b) = (self.corner(a), self.corner(b));
            let (sa, sb) = (side(a), side(b));
            if sa.abs() < eps * dot(normal, normal).sqrt() {
                push(a);
            }
            if sb.abs() < eps * dot(normal, normal).sqrt() {
                push(b);
            }
            if sa * sb < 0.0 {
                push(add(a, scale(sa / (sa - sb), sub(b, a))));
            }
        }
        if points.len() < 3 {
            return Vec::new();
        }
        let centre = scale(1.0 / points.len() as f64, points.iter().fold([0.0; 3], |sum, p| add(sum, *p)));
        let (e1, e2) = frame(normal);
        let angle = |p: &V3| dot(sub(*p, centre), e2).atan2(dot(sub(*p, centre), e1));
        points.sort_by(|p, q| angle(p).total_cmp(&angle(q)));
        points
    }

    /// The part of the line `support + t · direction` inside the box, as its two ends; `None`
    /// if the line misses the box (Liang–Barsky).
    pub fn clip(&self, support: V3, direction: V3) -> Option<(V3, V3)> {
        let (mut lo, mut hi) = (f64::NEG_INFINITY, f64::INFINITY);
        for i in 0..3 {
            if direction[i].abs() < 1e-12 {
                if support[i] < self.min[i] || support[i] > self.max[i] {
                    return None;
                }
                continue;
            }
            let a = (self.min[i] - support[i]) / direction[i];
            let b = (self.max[i] - support[i]) / direction[i];
            lo = lo.max(a.min(b));
            hi = hi.min(a.max(b));
        }
        (lo.is_finite() && lo <= hi).then(|| (add(support, scale(lo, direction)), add(support, scale(hi, direction))))
    }
}

impl Primitive {
    /// How near the viewer it is.
    pub fn depth(&self) -> f64 {
        match self {
            Primitive::Segment { depth, .. }
            | Primitive::Polygon { depth, .. }
            | Primitive::Marker { depth, .. }
            | Primitive::Label { depth, .. } => *depth,
        }
    }
}

/// A plane that splits what passes through it, `normal · x = constant` with a unit normal.
#[derive(Clone, Copy)]
struct Cutter {
    normal: V3,
    constant: f64,
}

impl Cutter {
    fn side(&self, p: V3) -> f64 {
        dot(self.normal, p) - self.constant
    }
}

/// A convex polygon split by `cutter` into its pieces on either side, or itself whole if the
/// cutter does not pass through it; `eps` is what counts as on the cutter.
fn split_polygon(polygon: Vec<V3>, cutter: Cutter, eps: f64) -> Vec<Vec<V3>> {
    let sides: Vec<f64> = polygon.iter().map(|p| cutter.side(*p)).collect();
    if sides.iter().all(|s| *s > -eps) || sides.iter().all(|s| *s < eps) {
        return vec![polygon];
    }
    let (mut front, mut back) = (Vec::new(), Vec::new());
    for i in 0..polygon.len() {
        let j = (i + 1) % polygon.len();
        let (a, b, sa, sb) = (polygon[i], polygon[j], sides[i], sides[j]);
        if sa >= -eps {
            front.push(a);
        }
        if sa <= eps {
            back.push(a);
        }
        if (sa > eps && sb < -eps) || (sa < -eps && sb > eps) {
            let p = add(a, scale(sa / (sa - sb), sub(b, a)));
            front.push(p);
            back.push(p);
        }
    }
    [front, back].into_iter().filter(|piece| piece.len() >= 3).collect()
}

/// The segment from `a` to `b` in pieces, cut where it passes through each cutter.
fn split_segment(a: V3, b: V3, cutters: &[Cutter]) -> Vec<(V3, V3)> {
    let mut cuts: Vec<f64> = cutters
        .iter()
        .filter_map(|c| {
            let (sa, sb) = (c.side(a), c.side(b));
            let t = sa / (sa - sb);
            ((sa < 0.0) != (sb < 0.0) && t > 1e-9 && t < 1.0 - 1e-9).then_some(t)
        })
        .collect();
    cuts.sort_by(f64::total_cmp);
    let at = |t: f64| add(a, scale(t, sub(b, a)));
    let mut ends = vec![0.0];
    ends.extend(cuts);
    ends.push(1.0);
    ends.windows(2).map(|w| (at(w[0]), at(w[1]))).collect()
}

/// A segment, in pieces where it passes through a plane.
fn segments(camera: &Camera, a: V3, b: V3, stroke: Stroke, object: Option<usize>, cutters: &[Cutter], out: &mut Vec<Primitive>) {
    for (a, b) in split_segment(a, b, cutters) {
        out.push(segment(camera, a, b, stroke, object));
    }
}

/// A step of 1, 2 or 5 times a power of ten that gives about five ticks over `length`.
fn tick_step(length: f64) -> f64 {
    let rough = length / 5.0;
    let power = 10f64.powf(rough.log10().floor());
    let mantissa = rough / power;
    let nice = if mantissa < 1.5 {
        1.0
    } else if mantissa < 3.5 {
        2.0
    } else if mantissa < 7.5 {
        5.0
    } else {
        10.0
    };
    nice * power
}

/// A tick number without float noise or trailing zeros: `2`, `0.5`, `-1.5`.
fn tick_text(value: f64, step: f64) -> String {
    let decimals = (-step.log10().floor()).max(0.0) as usize;
    let text = format!("{value:.decimals$}");
    if text.contains('.') { text.trim_end_matches('0').trim_end_matches('.').to_owned() } else { text }
}

fn segment(camera: &Camera, a: V3, b: V3, stroke: Stroke, object: Option<usize>) -> Primitive {
    let (from, da) = camera.project(a);
    let (to, db) = camera.project(b);
    Primitive::Segment { from, to, depth: (da + db) / 2.0, stroke, object }
}

/// The three axes over the box (each through the origin, reaching 0 if the box does not), with
/// a tick and its number at each step but 0, and the axis name at the end.
fn axes(camera: &Camera, bounds: &Bounds, cutters: &[Cutter], out: &mut Vec<Primitive>) {
    for axis in 0..3 {
        let lo = bounds.min[axis].min(0.0);
        let hi = bounds.max[axis].max(0.0);
        let along = |t: f64| {
            let mut p = [0.0; 3];
            p[axis] = t;
            p
        };
        segments(camera, along(lo), along(hi), Stroke::Axis, None, cutters, out);
        let (start, _) = camera.project(along(lo));
        let (end, end_depth) = camera.project(along(hi));
        let direction = [end[0] - start[0], end[1] - start[1]];
        let length = direction[0].hypot(direction[1]);
        out.push(Primitive::Label { at: end, depth: end_depth, text: ["x", "y", "z"][axis].to_owned(), object: None });
        // An axis seen end-on has no room for ticks.
        if length < 4.0 * TICK || hi <= lo {
            continue;
        }
        let normal = [-direction[1] / length * TICK, direction[0] / length * TICK];
        let step = tick_step(hi - lo);
        let mut k = (lo / step).ceil() as i64;
        while k as f64 * step <= hi + step * 1e-9 {
            let t = k as f64 * step;
            if k != 0 {
                let (at, depth) = camera.project(along(t));
                let from = [at[0] - normal[0] / 2.0, at[1] - normal[1] / 2.0];
                let to = [at[0] + normal[0] / 2.0, at[1] + normal[1] / 2.0];
                out.push(Primitive::Segment { from, to, depth, stroke: Stroke::Tick, object: None });
                out.push(Primitive::Label { at: from, depth, text: tick_text(t, step), object: None });
            }
            k += 1;
        }
    }
}

/// The two crossings of a convex polygon with the plane `normal · x = constant`, if it cuts it.
fn crossing(polygon: &[V3], normal: V3, constant: f64) -> Option<(V3, V3)> {
    let mut found = Vec::new();
    for i in 0..polygon.len() {
        let (a, b) = (polygon[i], polygon[(i + 1) % polygon.len()]);
        let (sa, sb) = (dot(normal, a) - constant, dot(normal, b) - constant);
        if (sa < 0.0) != (sb < 0.0) {
            found.push(add(a, scale(sa / (sa - sb), sub(b, a))));
        }
    }
    match found[..] {
        [p, q] => Some((p, q)),
        _ => None,
    }
}

/// A piece of a plane, its outline and, with `hatch`, lines across it a fixed distance apart
/// in the scene, along the diagonal of its frame.
fn plane_piece(camera: &Camera, polygon: &[V3], normal: V3, spacing: f64, hatch: bool, object: usize, out: &mut Vec<Primitive>) {
    let projected: Vec<(V2, f64)> = polygon.iter().map(|p| camera.project(*p)).collect();
    let depth = projected.iter().map(|(_, d)| d).sum::<f64>() / projected.len() as f64;
    out.push(Primitive::Polygon { points: projected.iter().map(|(p, _)| *p).collect(), depth, object });
    if !hatch {
        return;
    }
    let (e1, e2) = frame(normal);
    // Lines along e1 + e2, so across e1 − e2.
    let across = unit(sub(e1, e2));
    let (lo, hi) = polygon.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| (lo.min(dot(across, *p)), hi.max(dot(across, *p))));
    let mut c = (lo / spacing).floor() * spacing + spacing;
    while c < hi {
        if let Some((a, b)) = crossing(polygon, across, c) {
            out.push(segment(camera, a, b, Stroke::Hatch, Some(object)));
        }
        c += spacing;
    }
}

/// The ring around `centre` with `radius` perpendicular to `normal`, `RING` points in order.
fn ring(centre: V3, radius: f64, normal: V3) -> Vec<V3> {
    let (e1, e2) = frame(normal);
    (0..RING)
        .map(|k| {
            let (sin, cos) = (std::f64::consts::TAU * k as f64 / RING as f64).sin_cos();
            add(centre, add(scale(radius * cos, e1), scale(radius * sin, e2)))
        })
        .collect()
}

/// A closed ring as short segments, each with its own depth; with `behind`, those beyond that
/// depth are [`Stroke::Hidden`].
fn polyline(camera: &Camera, points: &[V3], behind: Option<f64>, object: usize, out: &mut Vec<Primitive>) {
    for i in 0..points.len() {
        let piece = segment(camera, points[i], points[(i + 1) % points.len()], Stroke::Object, Some(object));
        out.push(match (piece, behind) {
            (Primitive::Segment { from, to, depth, object, .. }, Some(limit)) if depth < limit => {
                Primitive::Segment { from, to, depth, stroke: Stroke::Hidden, object }
            }
            (piece, _) => piece,
        });
    }
}

/// The point furthest up and to the right on the screen, where a name goes.
fn top_right(camera: &Camera, points: &[V3]) -> (V2, f64) {
    points
        .iter()
        .map(|p| camera.project(*p))
        .max_by(|(p, _), (q, _)| (p[0] + p[1]).total_cmp(&(q[0] + q[1])))
        .expect("a figure has points")
}

/// A sphere: its outline, a circle on the screen, and rings of latitude and longitude, their
/// far halves hidden.
fn sphere(camera: &Camera, centre: V3, radius: f64, object: usize, out: &mut Vec<Primitive>) {
    let (middle, depth) = camera.project(centre);
    let r = camera.zoom * radius;
    let points = (0..RING)
        .map(|k| {
            let (sin, cos) = (std::f64::consts::TAU * k as f64 / RING as f64).sin_cos();
            [middle[0] + r * cos, middle[1] + r * sin]
        })
        .collect();
    out.push(Primitive::Polygon { points, depth, object });
    for latitude in LATITUDES {
        let (sin, cos) = latitude.to_radians().sin_cos();
        let points = ring(add(centre, [0.0, 0.0, radius * sin]), radius * cos, [0.0, 0.0, 1.0]);
        polyline(camera, &points, Some(depth), object, out);
    }
    for k in 0..MERIDIANS {
        let (sin, cos) = (std::f64::consts::PI * k as f64 / MERIDIANS as f64).sin_cos();
        let points = ring(centre, radius, [-sin, cos, 0.0]);
        polyline(camera, &points, Some(depth), object, out);
    }
}

/// A vector as an arrow from the origin to `tip`, its head two strokes on the screen, its name
/// at the middle.
fn arrow(camera: &Camera, tip: V3, name: &str, object: usize, cutters: &[Cutter], out: &mut Vec<Primitive>) {
    segments(camera, [0.0; 3], tip, Stroke::Object, Some(object), cutters, out);
    let (start, _) = camera.project([0.0; 3]);
    let (end, depth) = camera.project(tip);
    let shaft = [end[0] - start[0], end[1] - start[1]];
    let length = shaft[0].hypot(shaft[1]);
    if length > 1e-12 {
        let back = [-shaft[0] / length * ARROW, -shaft[1] / length * ARROW];
        for side in [1.0, -1.0] {
            // Back along the shaft, turned by ±30°.
            let (sin, cos) = (side * 0.5f64, 0.75f64.sqrt());
            let to = [end[0] + back[0] * cos - back[1] * sin, end[1] + back[0] * sin + back[1] * cos];
            out.push(Primitive::Segment { from: end, to, depth, stroke: Stroke::Object, object: Some(object) });
        }
    }
    let middle = scale(0.5, tip);
    let (at, depth) = camera.project(middle);
    out.push(Primitive::Label { at, depth, text: name.to_owned(), object: Some(object) });
}

/// The primitives of the scene as `camera` sees it, with the axes over `bounds`: far to near,
/// the labels last.
pub fn project(figures: &[Shown], camera: &Camera, bounds: &Bounds, options: &Options) -> Vec<Primitive> {
    let eps = 1e-9 * bounds.diagonal();
    let planes: Vec<(usize, Cutter)> = figures
        .iter()
        .enumerate()
        .filter_map(|(object, shown)| match shown.figure {
            Figure::Plane { normal, constant, .. } => {
                let length = dot(normal, normal).sqrt();
                Some((object, Cutter { normal: scale(1.0 / length, normal), constant: constant / length }))
            }
            _ => None,
        })
        .collect();
    let cutters: Vec<Cutter> = planes.iter().map(|(_, c)| *c).collect();
    let mut out = Vec::new();
    axes(camera, bounds, &cutters, &mut out);
    for (object, shown) in figures.iter().enumerate() {
        match &shown.figure {
            Figure::Point(p) => {
                let (at, depth) = camera.project(*p);
                out.push(Primitive::Marker { at, depth, object });
                out.push(Primitive::Label { at, depth, text: shown.name.clone(), object: Some(object) });
            }
            Figure::Vector(tip) => arrow(camera, *tip, &shown.name, object, &cutters, &mut out),
            Figure::Line { support, direction } => {
                if let Some((a, b)) = bounds.clip(*support, *direction) {
                    segments(camera, a, b, Stroke::Object, Some(object), &cutters, &mut out);
                    let (at, depth) = camera.project(b);
                    out.push(Primitive::Label { at, depth, text: shown.name.clone(), object: Some(object) });
                }
            }
            Figure::Plane { normal, constant, .. } => {
                let polygon = bounds.cut(*normal, *constant);
                if polygon.is_empty() {
                    continue;
                }
                // Split along the line where each other plane cuts it.
                let mut pieces = vec![polygon.clone()];
                for (other, cutter) in &planes {
                    if *other != object {
                        pieces = pieces.into_iter().flat_map(|piece| split_polygon(piece, *cutter, eps)).collect();
                    }
                }
                for piece in &pieces {
                    plane_piece(camera, piece, *normal, bounds.diagonal() / HATCHES, options.hatch, object, &mut out);
                }
                let (at, depth) = top_right(camera, &polygon);
                out.push(Primitive::Label { at, depth, text: shown.name.clone(), object: Some(object) });
            }
            Figure::Sphere { centre, radius } => {
                sphere(camera, *centre, *radius, object, &mut out);
                let (middle, depth) = camera.project(*centre);
                let r = camera.zoom * radius / std::f64::consts::SQRT_2;
                out.push(Primitive::Label { at: [middle[0] + r, middle[1] + r], depth, text: shown.name.clone(), object: Some(object) });
            }
            Figure::Circle { centre, radius, normal } => {
                let points = ring(*centre, *radius, *normal);
                polyline(camera, &points, None, object, &mut out);
                let (at, depth) = top_right(camera, &points);
                out.push(Primitive::Label { at, depth, text: shown.name.clone(), object: Some(object) });
            }
        }
    }
    // Stable, so that what lies at one depth keeps the order it was drawn in.
    out.sort_by(|p, q| {
        let label = |p: &Primitive| matches!(p, Primitive::Label { .. });
        label(p).cmp(&label(q)).then(p.depth().total_cmp(&q.depth()))
    });
    out
}
