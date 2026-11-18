//! A task's answer as figures for the view: the segment of a distance, the foot of a
//! perpendicular, an image, the points, line or circle two objects meet in, the rays and arc of
//! an angle, a tangent. The objects come from the same calls as the answer; what only the
//! picture needs (the nearest points of skew lines, an angle's rays) is worked out in floats.

use crate::settings::AngleUnit;
use crate::Result;

use super::super::view::{add, approx, approx3, cross, dot, frame, scale, sub, Figure, Shown, V3};
use super::super::{CircleLine, CirclePlane, LinePlane, LineRelation, PlanePlane, Point, SphereLine, SpherePlane, SphereSphere};
use super::{key_of, Builder, Object, Task};

/// What counts as nothing in the view's floats.
const EPS: f64 = 1e-9;

/// The kind of a figure, as [`super::ViewObject::kind`] gives it.
pub(super) fn kind(figure: &Figure) -> &'static str {
    match figure {
        Figure::Point(_) => "point",
        Figure::Vector(_) => "vector",
        Figure::Line { .. } => "line",
        Figure::Plane { .. } => "plane",
        Figure::Sphere { .. } => "sphere",
        Figure::Circle { .. } => "circle",
        Figure::Segment { .. } => "segment",
        Figure::Angle { .. } => "angle",
    }
}

fn length(a: V3) -> f64 {
    dot(a, a).sqrt()
}

/// `symbol = 3` or `symbol ≈ 3.46`: at most two decimals, `≈` when rounded.
fn measured(symbol: &str, value: f64, suffix: &str) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    let text = format!("{rounded:.2}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    let text = if text == "-0" { "0" } else { text };
    let sign = if (rounded - value).abs() < EPS { "=" } else { "≈" };
    format!("{symbol} {sign} {text}{suffix}")
}

fn point(p: &Point) -> Option<V3> {
    approx3(&p.coords)
}

/// A point of the answer with its label.
fn named(name: &str, p: &Point) -> Vec<Shown> {
    point(p).map(|at| Shown::result(name, Figure::Point(at))).into_iter().collect()
}

/// The distance from `a` to `b` as a segment with `d = …` at its middle; nothing if they meet.
fn distance(a: V3, b: V3) -> Vec<Shown> {
    let d = length(sub(b, a));
    if d < EPS {
        return Vec::new();
    }
    vec![Shown::result(measured("d", d, ""), Figure::Segment { from: a, to: b, dashed: false })]
}

/// The point `to` named `name`, with the dashed perpendicular from `from` to it.
fn perpendicular(from: &Point, to: &Point, name: &str) -> Vec<Shown> {
    let (Some(a), Some(b)) = (point(from), point(to)) else { return Vec::new() };
    vec![Shown::result("", Figure::Segment { from: a, to: b, dashed: true }), Shown::result(name, Figure::Point(b))]
}

/// The foot of the perpendicular from `p` onto the line through `support` along `direction`.
fn foot_on_line(p: V3, support: V3, direction: V3) -> V3 {
    add(support, scale(dot(sub(p, support), direction) / dot(direction, direction), direction))
}

/// The foot of the perpendicular from `p` onto the plane `normal · x = constant`.
fn foot_on_plane(p: V3, normal: V3, constant: f64) -> V3 {
    sub(p, scale((dot(normal, p) - constant) / dot(normal, normal), normal))
}

fn parallel(a: V3, b: V3) -> bool {
    length(cross(a, b)) < EPS * length(a) * length(b)
}

impl Builder<'_> {
    /// The figures of the task's answer, marked as results; none if it fails or draws nothing.
    pub(super) fn drawn_answer(&self, task: &Task) -> Vec<Shown> {
        self.answer(task).unwrap_or_default()
    }

    fn answer(&self, task: &Task) -> Result<Vec<Shown>> {
        use Object::{Circle as C, Line as L, Plane as E, Point as P, Sphere as K};
        Ok(match task {
            Task::Distance { a, b } => self.distance_figure(self.get(a)?, self.get(b)?)?,
            Task::Relation { a, b } => self.meeting(self.get(a)?, self.get(b)?)?,
            Task::Angle { a, b } => self.angle_figure(self.get(a)?, self.get(b)?)?,
            Task::Foot { point, onto } => match (self.get(point)?, self.get(onto)?) {
                (P(p), L(g)) => perpendicular(p, &g.foot(p).result, "F"),
                (P(p), E(e)) => perpendicular(p, &e.foot(p)?.result, "F"),
                _ => Vec::new(),
            },
            Task::Reflect { point, mirror } => {
                let image = match (self.get(point)?, self.get(mirror)?) {
                    (P(p), P(m)) => (p, p.reflect_in_point(m).result),
                    (P(p), L(g)) => (p, g.reflect(p).result),
                    (P(p), E(e)) => (p, e.reflect(p)?.result),
                    _ => return Ok(Vec::new()),
                };
                perpendicular(image.0, &image.1, &format!("{}'", key_of(point)))
            }
            Task::Midpoint { a, b } => match (self.get(a)?, self.get(b)?) {
                (P(p), P(q)) => {
                    let half = crate::num::Value::ratio(1, 2);
                    let m = p + &p.to(q).scale(&half);
                    let mut figures = Vec::new();
                    if let (Some(a), Some(b)) = (point(p), point(q)) {
                        figures.push(Shown::result("", Figure::Segment { from: a, to: b, dashed: true }));
                    }
                    figures.extend(named("M", &m));
                    figures
                }
                _ => Vec::new(),
            },
            Task::Tangent { object, point } => match (self.get(object)?, self.get(point)?) {
                (K(k), P(p)) => Figure::plane(&k.tangent_plane(p)?.result).map(|f| Shown::result("T", f)).into_iter().collect(),
                (C(c), P(p)) => Figure::line(&c.tangent(p)?.result).map(|f| Shown::result("t", f)).into_iter().collect(),
                _ => Vec::new(),
            },
            Task::Convert { .. } | Task::Measure { .. } | Task::Evaluate { .. } => Vec::new(),
        })
    }

    /// The segment between the nearest points of the two objects.
    fn distance_figure(&self, a: &Object, b: &Object) -> Result<Vec<Shown>> {
        use Object::{Line as L, Plane as E, Point as P, Sphere as K};
        let figure = |o: &Object| o.figure();
        Ok(match (a, b) {
            (P(p), P(q)) => match (point(p), point(q)) {
                (Some(a), Some(b)) => distance(a, b),
                _ => Vec::new(),
            },
            (P(p), L(g)) | (L(g), P(p)) => match (point(p), point(&g.foot(p).result)) {
                (Some(a), Some(b)) => [distance(a, b), named("", &g.foot(p).result)].concat(),
                _ => Vec::new(),
            },
            (P(p), E(e)) | (E(e), P(p)) => {
                let foot = e.foot(p)?.result;
                match (point(p), point(&foot)) {
                    (Some(a), Some(b)) => [distance(a, b), named("", &foot)].concat(),
                    _ => Vec::new(),
                }
            }
            (P(p), K(k)) | (K(k), P(p)) => match (point(p), point(&k.centre), approx(&k.radius)) {
                (Some(p), Some(c), Some(r)) => {
                    let out = sub(p, c);
                    if length(out) < EPS {
                        return Ok(Vec::new());
                    }
                    distance(p, add(c, scale(r / length(out), out)))
                }
                _ => Vec::new(),
            },
            (L(_), L(_)) => match (figure(a), figure(b)) {
                (Some(Figure::Line { support: s, direction: u }), Some(Figure::Line { support: t, direction: v })) => {
                    if parallel(u, v) {
                        distance(s, foot_on_line(s, t, v))
                    } else {
                        // The nearest points s + λu and t + μv: their join is perpendicular to both.
                        let w = sub(s, t);
                        let (uu, uv, vv, uw, vw) = (dot(u, u), dot(u, v), dot(v, v), dot(u, w), dot(v, w));
                        let det = uu * vv - uv * uv;
                        let lambda = (uv * vw - vv * uw) / det;
                        let mu = (uu * vw - uv * uw) / det;
                        distance(add(s, scale(lambda, u)), add(t, scale(mu, v)))
                    }
                }
                _ => Vec::new(),
            },
            (L(_), E(_)) | (E(_), L(_)) => {
                let (line, plane) = if matches!(a, L(_)) { (a, b) } else { (b, a) };
                match (figure(line), figure(plane)) {
                    (Some(Figure::Line { support, direction }), Some(Figure::Plane { normal, constant, .. }))
                        if dot(normal, direction).abs() < EPS * length(normal) * length(direction) =>
                    {
                        distance(support, foot_on_plane(support, normal, constant))
                    }
                    _ => Vec::new(),
                }
            }
            (E(_), E(_)) => match (figure(a), figure(b)) {
                (Some(Figure::Plane { normal: n, support, .. }), Some(Figure::Plane { normal: m, constant, .. })) if parallel(n, m) => {
                    distance(support, foot_on_plane(support, m, constant))
                }
                _ => Vec::new(),
            },
            _ => Vec::new(),
        })
    }

    /// Where the two objects meet: a point, two points, a line or a circle.
    fn meeting(&self, a: &Object, b: &Object) -> Result<Vec<Shown>> {
        let (unit, precision) = (self.settings.angle_unit, self.settings.precision);
        use Object::{Circle as C, Line as L, Plane as E, Sphere as K};
        let two = |points: &[Point; 2]| [named("S_1", &points[0]), named("S_2", &points[1])].concat();
        let circle = |k: &super::Circle| Figure::circle(k).map(|f| Shown::result("k", f)).into_iter().collect::<Vec<_>>();
        Ok(match (a, b) {
            (L(g), L(h)) => match g.relation(h, unit, precision)?.result {
                LineRelation::Intersecting { point, .. } => named("S", &point),
                _ => Vec::new(),
            },
            (L(g), E(e)) | (E(e), L(g)) => match e.meet_line(g, unit, precision)?.result {
                LinePlane::Intersecting { point, .. } => named("S", &point),
                _ => Vec::new(),
            },
            (E(e), E(f)) => match e.meet_plane(f, unit, precision)?.result {
                PlanePlane::Intersecting { line, .. } => Figure::line(&line).map(|f| Shown::result("s", f)).into_iter().collect(),
                _ => Vec::new(),
            },
            (K(k), L(g)) | (L(g), K(k)) => match k.meet_line(g)?.result {
                SphereLine::Secant { points, .. } => two(&points),
                SphereLine::Tangent { point, .. } => named("S", &point),
                SphereLine::Passant => Vec::new(),
            },
            (K(k), E(e)) | (E(e), K(k)) => match k.meet_plane(e)?.result {
                SpherePlane::Circle { circle: c, .. } => circle(&c),
                SpherePlane::Tangent { point } => named("S", &point),
                SpherePlane::Apart { .. } => Vec::new(),
            },
            (K(k), K(m)) => match k.meet_sphere(m)?.result {
                SphereSphere::Touching { point, .. } => named("S", &point),
                SphereSphere::Intersecting { circle: c, .. } => circle(&c),
                _ => Vec::new(),
            },
            (C(c), L(g)) | (L(g), C(c)) => match c.meet_line(g)?.result {
                CircleLine::Secant { points } => two(&points),
                CircleLine::Tangent { point } | CircleLine::Crossing { point } => named("S", &point),
                CircleLine::Passant | CircleLine::Apart => Vec::new(),
            },
            (C(c), E(e)) | (E(e), C(c)) => match c.meet_plane(e)?.result {
                CirclePlane::Cut { points } => two(&points),
                CirclePlane::Touch { point } => named("S", &point),
                _ => Vec::new(),
            },
            _ => Vec::new(),
        })
    }

    /// The angle between the two objects at the point they meet in (the origin for vectors).
    fn angle_figure(&self, a: &Object, b: &Object) -> Result<Vec<Shown>> {
        let (unit, precision) = (self.settings.angle_unit, self.settings.precision);
        use Object::{Line as L, Plane as E, Vector as V};
        let floats = |p: &Point| point(p);
        // The vertex and the two directions; the acute angle for lines and planes.
        let found: Option<(V3, V3, V3)> = match (a, b) {
            (V(u), V(v)) => approx3(&u.coords).zip(approx3(&v.coords)).map(|(u, v)| ([0.0; 3], u, v)),
            (L(g), L(h)) => match g.relation(h, unit, precision)?.result {
                LineRelation::Intersecting { point, .. } => match (floats(&point), approx3(&g.direction.coords), approx3(&h.direction.coords)) {
                    (Some(s), Some(u), Some(v)) => Some((s, u, if dot(u, v) < 0.0 { scale(-1.0, v) } else { v })),
                    _ => None,
                },
                _ => None,
            },
            (L(g), E(e)) | (E(e), L(g)) => match e.meet_line(g, unit, precision)?.result {
                LinePlane::Intersecting { point, .. } => match (floats(&point), approx3(&g.direction.coords), approx3(&e.normal.coords)) {
                    (Some(s), Some(u), Some(n)) => {
                        // The line's shadow in the plane; a perpendicular line has none, any
                        // direction in the plane then shows the right angle.
                        let shadow = sub(u, scale(dot(u, n) / dot(n, n), n));
                        let shadow = if length(shadow) < EPS * length(u) { frame(n).0 } else { shadow };
                        Some((s, u, shadow))
                    }
                    _ => None,
                },
                _ => None,
            },
            (E(e), E(f)) => match e.meet_plane(f, unit, precision)?.result {
                PlanePlane::Intersecting { line, .. } => match (floats(&line.support), approx3(&line.direction.coords), approx3(&e.normal.coords), approx3(&f.normal.coords)) {
                    (Some(s), Some(d), Some(n), Some(m)) => {
                        // In each plane, the direction across the line.
                        let (u, v) = (cross(n, d), cross(m, d));
                        Some((s, u, if dot(u, v) < 0.0 { scale(-1.0, v) } else { v }))
                    }
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        };
        let Some((vertex, u, v)) = found else { return Ok(Vec::new()) };
        if length(u) < EPS || length(v) < EPS {
            return Ok(Vec::new());
        }
        let radians = (dot(u, v) / (length(u) * length(v))).clamp(-1.0, 1.0).acos();
        let label = match unit {
            AngleUnit::Degree => measured("φ", radians.to_degrees(), "°"),
            AngleUnit::Radian => measured("φ", radians, ""),
        };
        Ok(vec![Shown::result(label, Figure::Angle { vertex, a: u, b: v })])
    }
}
