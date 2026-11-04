//! Circles (Kreise) in space: centre, radius and the normal of their plane. A plain 2D circle
//! `(x - a)^2 + (y - b)^2 = r^2` is the case z = 0 with the normal along z.

use crate::num::Value;
use crate::settings::StepDetail::{Brief, Detailed, Standard};
use crate::text::msg;
use crate::{QuantusError, Result};

use super::show;
use super::working::{Worked, Working};
use super::{equal, Plane, Point, Sphere, SpherePlane, Vector3};

/// A circle around `centre` with `radius` (> 0) in the plane through the centre perpendicular
/// to `normal`.
#[derive(Clone, Debug, PartialEq)]
pub struct Circle {
    pub centre: Point,
    pub radius: Value,
    pub normal: Vector3,
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
}
