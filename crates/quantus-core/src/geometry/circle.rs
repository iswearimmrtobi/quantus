//! Circles (Kreise) in space: centre, radius and the normal of their plane. A plain 2D circle
//! `(x - a)^2 + (y - b)^2 = r^2` is the case z = 0 with the normal along z.

use crate::num::Value;
use crate::settings::StepDetail::{Brief, Detailed, Standard};
use crate::text::msg;
use crate::{QuantusError, Result};

use super::line::multiple;
use super::relations::cut_line;
use super::show;
use super::working::{Worked, Working};
use super::{equal, Line, Plane, PlaneForm, Point, Sphere, SphereLine, SpherePlane, Vector3};

/// A circle around `centre` with `radius` (> 0) in the plane through the centre perpendicular
/// to `normal`.
#[derive(Clone, Debug, PartialEq)]
pub struct Circle {
    pub centre: Point,
    pub radius: Value,
    pub normal: Vector3,
}

/// How a line lies to a circle.
#[derive(Clone, Debug, PartialEq)]
pub enum CircleLine {
    /// In the plane of the circle, through two of its points (Sekante).
    Secant { points: [Point; 2] },
    /// In the plane of the circle, touching it (Tangente).
    Tangent { point: Point },
    /// In the plane of the circle, missing it (Passante).
    Passant,
    /// Through the plane of the circle in `point`, which lies on the circle.
    Crossing { point: Point },
    /// No common point, and not in the plane of the circle.
    Apart,
}

/// How a plane lies to a circle.
#[derive(Clone, Debug, PartialEq)]
pub enum CirclePlane {
    /// The plane cuts the circle in two points.
    Cut { points: [Point; 2] },
    /// The plane touches the circle in one point.
    Touch { point: Point },
    /// The planes meet, but their line misses the circle.
    Miss,
    /// The circle lies in the plane.
    Contained,
    /// The plane is parallel to the circle's and not the same.
    Parallel,
}

fn error(message: String) -> QuantusError {
    QuantusError::other(message)
}

/// `(x - a)^2`, or `x^2` for `a = 0`.
fn square(symbol: &str, a: &Value) -> String {
    if a.is_zero() {
        return format!("{symbol}^2");
    }
    format!("\\left({}\\right)^2", show::combination(&[(&Value::one(), symbol), (&a.neg(), "")]))
}

impl Circle {
    /// The circle around `centre` with `radius` perpendicular to `normal`.
    pub fn new(centre: &Point, radius: Value, normal: &Vector3) -> Result<Circle> {
        if radius.is_zero() || radius.is_negative()? {
            return Err(error(msg!("The radius of a circle must be positive.", "Der Radius eines Kreises muss positiv sein.")));
        }
        if normal.is_zero() {
            return Err(error(msg!("The zero vector is no normal of a circle.", "Der Nullvektor ist kein Normalenvektor eines Kreises.")));
        }
        Ok(Circle { centre: centre.clone(), radius, normal: Vector3 { planar: false, ..normal.clone() } })
    }

    /// The plain circle around a 2D `centre` in the xy plane.
    pub fn planar(centre: &Point, radius: Value) -> Result<Circle> {
        let centre = Point { planar: true, ..centre.clone() };
        Circle::new(&centre, radius, &Vector3::from_ints(0, 0, 1))
    }

    /// The circle where `plane` cuts `sphere` (Schnittkreis); an error if they only touch or
    /// do not meet.
    pub fn cut(sphere: &Sphere, plane: &Plane) -> Result<Worked<Circle>> {
        let Worked { result, working } = sphere.meet_plane(plane)?;
        match result {
            SpherePlane::Circle { circle, .. } => Ok(Worked::new(circle, working)),
            SpherePlane::Tangent { .. } => Err(error(msg!(
                "The plane only touches the sphere, so they have no circle in common.",
                "Die Ebene berührt die Kugel nur, sie haben also keinen gemeinsamen Kreis."
            ))),
            SpherePlane::Apart { .. } => Err(error(msg!(
                "The plane does not meet the sphere, so they have no circle in common.",
                "Die Ebene trifft die Kugel nicht, sie haben also keinen gemeinsamen Kreis."
            ))),
        }
    }

    /// `r^2`.
    pub fn radius_squared(&self) -> Value {
        self.radius.mul(&self.radius)
    }

    /// Whether the circle lies in the xy plane around a 2D centre.
    pub fn is_planar(&self) -> bool {
        self.centre.planar && self.normal.coords[0].is_zero() && self.normal.coords[1].is_zero()
    }

    /// The plane of the circle, in normal form.
    pub fn plane(&self) -> Plane {
        Plane::with_normal(&self.centre, &self.normal).expect("a circle has a normal")
    }

    /// `(x - a)^2 + (y - b)^2 = r^2` for a plain circle; in space the sphere around the centre
    /// with the plane of the circle.
    pub fn equation(&self) -> String {
        let [a, b, _] = &self.centre.coords;
        let r2 = show::value(&self.radius_squared());
        if self.is_planar() {
            return format!("{} + {} = {r2}", square("x", a), square("y", b));
        }
        let m = show::vector(&self.centre.position());
        format!("\\left(\\vec{{x}} - {m}\\right)^2 = {r2}, \\quad \\left(\\vec{{x}} - {m}\\right) \\cdot {} = 0", show::vector(&self.normal))
    }

    /// Whether `p` lies on the circle: in its plane and at the distance `r` from the centre.
    pub fn contains(&self, p: &Point) -> Result<Worked<bool>> {
        let mut working = Working::default();
        let mp = Vector3 { planar: self.is_planar() && p.planar, ..self.centre.to(p) };
        let in_plane = equal(&mp.dot(&self.normal), &Value::zero());
        if !self.is_planar() || !p.planar {
            working.add(
                Standard,
                msg!("Is the point in the plane of the circle?", "Liegt der Punkt in der Ebene des Kreises?"),
                format!(
                    "\\overrightarrow{{MP}} \\cdot \\vec{{n}} = {} \\cdot {} = {} {} 0",
                    show::vector(&mp),
                    show::vector(&self.normal),
                    show::value(&mp.dot(&self.normal)),
                    if in_plane { "=" } else { "\\neq" }
                ),
            );
        }
        if !in_plane {
            working.add(Brief, msg!("The point does not lie in the plane of the circle.", "Der Punkt liegt nicht in der Ebene des Kreises."), "P \\notin k");
            return Ok(Worked::new(false, working));
        }
        let length = mp.length()?;
        let on = equal(&mp.dot(&mp), &self.radius_squared());
        working.add(
            Standard,
            msg!("The distance from the centre:", "Der Abstand zum Mittelpunkt:"),
            format!(
                "\\left|\\overrightarrow{{MP}}\\right| = {} = {} {} r = {}",
                show::length(&mp),
                show::value(&length),
                if on { "=" } else { "\\neq" },
                show::value(&self.radius)
            ),
        );
        working.add(
            Brief,
            if on { msg!("The point lies on the circle.", "Der Punkt liegt auf dem Kreis.") } else { msg!("The point does not lie on the circle.", "Der Punkt liegt nicht auf dem Kreis.") },
            if on { "P \\in k" } else { "P \\notin k" },
        );
        Ok(Worked::new(on, working))
    }

    /// The circumference `U = 2 \pi r`.
    pub fn circumference(&self) -> Worked<Value> {
        let mut working = Working::default();
        let value = Value::int(2).mul(&Value::Pi).mul(&self.radius);
        working.add(Detailed, msg!("The formula:", "Die Formel:"), format!("U = 2 \\pi r = 2 \\pi \\cdot {}", show::factor(&self.radius)));
        working.add(Brief, msg!("The circumference:", "Der Umfang:"), format!("U = {}", show::value(&value)));
        Worked::new(value, working)
    }

    /// The area `A = \pi r^2`.
    pub fn area(&self) -> Worked<Value> {
        let mut working = Working::default();
        let value = Value::Pi.mul(&self.radius_squared());
        working.add(Detailed, msg!("The formula:", "Die Formel:"), format!("A = \\pi r^2 = \\pi \\cdot {}^2", show::factor(&self.radius)));
        working.add(Brief, msg!("The area:", "Der Flächeninhalt:"), format!("A = {}", show::value(&value)));
        Worked::new(value, working)
    }

    /// The sphere with the circle's centre and radius: the circle is where it meets the plane.
    /// Its centre stays 2D for a plain circle, so that the working is written in 2D.
    fn sphere(&self) -> Sphere {
        Sphere { centre: Point { planar: self.is_planar(), ..self.centre.clone() }, radius: self.radius.clone() }
    }

    /// How `line` lies to the circle. In the plane of the circle it is a secant, tangent or
    /// passant (as to the sphere with the same centre and radius); else it crosses the plane in
    /// one point, which may lie on the circle.
    pub fn meet_line(&self, line: &Line) -> Result<Worked<CircleLine>> {
        let mut working = Working::default();
        let n = &self.normal;
        let (coefficient, offset) = (n.dot(&line.direction), n.dot(&self.centre.to(&line.support)));
        let planar = self.is_planar() && line.planar();
        let flat = |p: Point| Point { planar, ..p };
        if equal(&coefficient, &Value::zero()) {
            if !equal(&offset, &Value::zero()) {
                working.add(
                    Standard,
                    msg!("The line is parallel to the plane of the circle and not in it:", "Die Gerade ist parallel zur Ebene des Kreises und liegt nicht in ihr:"),
                    format!("\\vec{{n}} \\cdot \\vec{{u}} = 0, \\quad \\vec{{n}} \\cdot \\overrightarrow{{MP}} = {} \\neq 0", show::value(&offset)),
                );
                working.add(Brief, msg!("The line misses the circle.", "Die Gerade verfehlt den Kreis."), "g \\cap k = \\emptyset");
                return Ok(Worked::new(CircleLine::Apart, working));
            }
            if !planar {
                working.add(
                    Standard,
                    msg!("The line lies in the plane of the circle:", "Die Gerade liegt in der Ebene des Kreises:"),
                    "\\vec{n} \\cdot \\vec{u} = 0, \\quad \\vec{n} \\cdot \\overrightarrow{MP} = 0",
                );
            }
            let Worked { result, working: cut } = self.sphere().meet_line(line)?;
            working.extend(cut);
            let relation = match result {
                SphereLine::Secant { points: [a, b], .. } => CircleLine::Secant { points: [flat(a), flat(b)] },
                SphereLine::Tangent { point, .. } => CircleLine::Tangent { point: flat(point) },
                SphereLine::Passant => CircleLine::Passant,
            };
            return Ok(Worked::new(relation, working));
        }
        let lambda = offset.neg().div(&coefficient)?;
        let point = Point { planar: false, ..line.at(&lambda) };
        working.add(
            Standard,
            msg!("The line crosses the plane of the circle where", "Die Gerade durchstößt die Ebene des Kreises, wo"),
            format!(
                "\\vec{{n}} \\cdot \\left(\\overrightarrow{{MP}} + \\lambda \\vec{{u}}\\right) = {} = 0 \\quad \\Rightarrow \\quad \\lambda = {}",
                show::combination(&[(&offset, ""), (&coefficient, "\\lambda")]),
                show::value(&lambda)
            ),
        );
        working.add(Detailed, msg!("The point in the plane:", "Der Punkt in der Ebene:"), format!("Q = {}", show::point(&point)));
        let mq = self.centre.to(&point);
        let on = equal(&mq.dot(&mq), &self.radius_squared());
        working.add(
            Standard,
            msg!("Its distance from the centre:", "Sein Abstand zum Mittelpunkt:"),
            format!("\\left|\\overrightarrow{{MQ}}\\right| = {} {} r = {}", show::value(&mq.length()?), if on { "=" } else { "\\neq" }, show::value(&self.radius)),
        );
        if on {
            working.add(Brief, msg!("The line goes through the circle in", "Die Gerade geht durch den Kreis in"), format!("Q = {}", show::point(&point)));
            return Ok(Worked::new(CircleLine::Crossing { point }, working));
        }
        working.add(Brief, msg!("The line misses the circle.", "Die Gerade verfehlt den Kreis."), "g \\cap k = \\emptyset");
        Ok(Worked::new(CircleLine::Apart, working))
    }

    /// The tangent at `p`, which must lie on the circle: perpendicular to the radius and to the
    /// normal, along `\vec{n} \times \overrightarrow{MP}`.
    pub fn tangent(&self, p: &Point) -> Result<Worked<Line>> {
        if !self.contains(p)?.result {
            return Err(error(msg!(
                "The point does not lie on the circle, so it has no tangent there.",
                "Der Punkt liegt nicht auf dem Kreis, dort gibt es also keine Tangente."
            )));
        }
        let mut working = Working::default();
        let planar = self.is_planar() && p.planar;
        let mp = self.centre.to(p);
        let direction = Vector3 { planar, ..self.normal.cross(&mp) };
        working.add(
            Standard,
            msg!("The tangent is perpendicular to the radius and to the normal:", "Die Tangente steht senkrecht auf dem Radius und dem Normalenvektor:"),
            format!("\\vec{{u}} = \\vec{{n}} \\times \\overrightarrow{{MP}} = {} \\times {} = {}", show::vector(&self.normal), show::vector(&Vector3 { planar: false, ..mp }), show::vector(&direction)),
        );
        let line = Line::new(Point { planar, ..p.clone() }, direction)?;
        working.add(Brief, msg!("The tangent:", "Die Tangente:"), line.equation());
        Ok(Worked::new(line, working))
    }

    /// How `plane` lies to the circle: the circle lies in it, it is parallel, or the two planes
    /// meet in a line, which cuts, touches or misses the circle.
    pub fn meet_plane(&self, plane: &Plane) -> Result<Worked<CirclePlane>> {
        let mut working = Working::default();
        let e = plane.in_coordinates(&mut working);
        if multiple(&self.normal, &e.normal).is_some() {
            let inside = equal(&e.normal.dot(&self.centre.position()), &e.constant);
            working.add(
                Standard,
                msg!("The plane is parallel to the plane of the circle; is the centre in it?", "Die Ebene ist parallel zur Ebene des Kreises; liegt der Mittelpunkt in ihr?"),
                format!("{} {} {}", show::dot(&e.normal, &Vector3 { planar: false, ..self.centre.position() }), if inside { "=" } else { "\\neq" }, show::value(&e.constant)),
            );
            if inside {
                working.add(Brief, msg!("The circle lies in the plane.", "Der Kreis liegt in der Ebene."), "k \\subset E");
                return Ok(Worked::new(CirclePlane::Contained, working));
            }
            working.add(Brief, msg!("The plane is parallel to the circle.", "Die Ebene ist parallel zum Kreis."), "k \\cap E = \\emptyset");
            return Ok(Worked::new(CirclePlane::Parallel, working));
        }
        let own = self.plane().convert(PlaneForm::Coordinate).result;
        working.add(Standard, msg!("The plane of the circle:", "Die Ebene des Kreises:"), own.equation());
        let line = cut_line(&own, &e, &mut working)?;
        let Worked { result, working: cut } = self.meet_line(&line)?;
        working.extend(cut);
        let relation = match result {
            CircleLine::Secant { points } => CirclePlane::Cut { points },
            CircleLine::Tangent { point } => CirclePlane::Touch { point },
            CircleLine::Passant => CirclePlane::Miss,
            CircleLine::Crossing { .. } | CircleLine::Apart => unreachable!("the line lies in the plane of the circle"),
        };
        Ok(Worked::new(relation, working))
    }
}
