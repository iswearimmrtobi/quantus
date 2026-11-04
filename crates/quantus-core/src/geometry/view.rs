//! The 3D view of a scene, shared by the shells: a [`Camera`] and [`project`], which turns the
//! scene's [`Figure`]s into flat [`Primitive`]s (segments, markers, labels) that a shell only
//! draws, as SVG on the desktop and as short 1 px lines on the handheld.
//!
//! Plain `f64`, no caches. Screen coordinates are in view units with the origin at the camera's
//! centre, x to the right and y **up**; a shell scales them to its pixels and flips y.

use serde::{Deserialize, Serialize};

use crate::num::Value;

use super::Point;

/// A point or vector in plain floats.
pub type V3 = [f64; 3];
/// A point on the screen in view units, y up.
pub type V2 = [f64; 2];

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
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

/// An object of the scene in floats, for the view.
#[derive(Clone, Debug, PartialEq)]
pub enum Figure {
    Point(V3),
}

impl Figure {
    pub fn point(p: &Point) -> Option<Figure> {
        approx3(&p.coords).map(Figure::Point)
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
}

/// What a shell draws. `depth` is larger nearer the viewer; `object` is the index of the
/// figure, `None` for the axes.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Primitive {
    Segment { from: V2, to: V2, depth: f64, stroke: Stroke, object: Option<usize> },
    /// The marker of a point.
    Marker { at: V2, depth: f64, object: usize },
    /// A name, an axis or a tick number, placed next to `at`.
    Label { at: V2, depth: f64, text: String, object: Option<usize> },
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
fn axes(camera: &Camera, bounds: &Bounds, out: &mut Vec<Primitive>) {
    for axis in 0..3 {
        let lo = bounds.min[axis].min(0.0);
        let hi = bounds.max[axis].max(0.0);
        let along = |t: f64| {
            let mut p = [0.0; 3];
            p[axis] = t;
            p
        };
        out.push(segment(camera, along(lo), along(hi), Stroke::Axis, None));
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

/// The primitives of the scene as `camera` sees it, with the axes over `bounds`.
pub fn project(figures: &[Shown], camera: &Camera, bounds: &Bounds) -> Vec<Primitive> {
    let mut out = Vec::new();
    axes(camera, bounds, &mut out);
    for (object, shown) in figures.iter().enumerate() {
        match &shown.figure {
            Figure::Point(p) => {
                let (at, depth) = camera.project(*p);
                out.push(Primitive::Marker { at, depth, object });
                out.push(Primitive::Label { at, depth, text: shown.name.clone(), object: Some(object) });
            }
        }
    }
    out
}
