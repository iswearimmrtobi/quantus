//! Spheres (Kugeln): centre and radius.

use crate::num::Value;
use crate::settings::StepDetail::{Brief, Detailed, Standard};
use crate::text::msg;
use crate::{QuantusError, Result};

use super::show;
use super::working::{Worked, Working};
use super::{equal, triple_product, Point, Vector3};

/// A sphere around `centre` with the radius `radius` (> 0).
#[derive(Clone, Debug, PartialEq)]
pub struct Sphere {
    pub centre: Point,
    pub radius: Value,
}

/// Where a point lies to a sphere, with its distance to the surface.
#[derive(Clone, Debug, PartialEq)]
pub enum SpherePoint {
    Inside { distance: Value },
    On,
    Outside { distance: Value },
}

fn error(message: String) -> QuantusError {
    QuantusError::other(message)
}

fn spatial(p: &Point) -> Point {
    Point { coords: p.coords.clone(), planar: false }
}

/// `(x - a)^2`, or `x^2` for `a = 0`.
fn square(symbol: &str, a: &Value) -> String {
    if a.is_zero() {
        return format!("{symbol}^2");
    }
    format!("\\left({}\\right)^2", show::combination(&[(&Value::one(), symbol), (&a.neg(), "")]))
}

impl Sphere {
    /// The sphere around `centre` with `radius`, which must be positive.
    pub fn new(centre: &Point, radius: Value) -> Result<Sphere> {
        if radius.is_zero() || radius.is_negative()? {
            return Err(error(msg!("The radius of a sphere must be positive.", "Der Radius einer Kugel muss positiv sein.")));
        }
        Ok(Sphere { centre: spatial(centre), radius })
    }

    /// `r^2`.
    pub fn radius_squared(&self) -> Value {
        self.radius.mul(&self.radius)
    }

    /// The sphere around `centre` through `p`: the radius is `|\overrightarrow{MP}|`.
    pub fn through(centre: &Point, p: &Point) -> Result<Worked<Sphere>> {
        let mp = centre.to(p);
        if mp.is_zero() {
            return Err(error(msg!(
                "The point is the centre, so the radius would be 0.",
                "Der Punkt ist der Mittelpunkt, der Radius wäre also 0."
            )));
        }
        let mut working = Working::default();
        let radius = mp.length()?;
        working.add(
            Standard,
            msg!("The radius is the distance from the centre to the point:", "Der Radius ist der Abstand vom Mittelpunkt zum Punkt:"),
            format!("r = \\left|\\overrightarrow{{MP}}\\right| = \\left|{}\\right| = {} = {}", show::vector(&mp), show::length(&mp), show::value(&radius)),
        );
        let sphere = Sphere::new(centre, radius)?;
        working.add(Brief, msg!("The sphere:", "Die Kugel:"), sphere.equation());
        Ok(Worked::new(sphere, working))
    }

    /// The sphere through four points that do not lie in one plane. The centre is equally far
    /// from all four: subtracting `|X - A|^2 = r^2` from the others leaves a linear system.
    pub fn through_four(a: &Point, b: &Point, c: &Point, d: &Point) -> Result<Worked<Sphere>> {
        let others = [b, c, d];
        let rows: Vec<Vector3> = others.iter().map(|p| a.to(p).scale(&Value::int(2))).collect();
        let rhs: Vec<Value> = others.iter().map(|p| p.position().dot(&p.position()).sub(&a.position().dot(&a.position()))).collect();
        let determinant = triple_product(&rows[0], &rows[1], &rows[2])?;
        if determinant.is_zero() {
            return Err(error(msg!(
                "The four points lie in one plane, so no single sphere passes through them.",
                "Die vier Punkte liegen in einer Ebene, also geht keine eindeutige Kugel durch sie."
            )));
        }
        let mut working = Working::default();
        working.add(
            Standard,
            msg!(
                "The centre M is equally far from all points; subtracting the equation of A from the others:",
                "Der Mittelpunkt M ist von allen Punkten gleich weit entfernt; die Gleichung von A von den anderen abziehen:"
            ),
            format!(
                "\\begin{{aligned}} {} \\end{{aligned}}",
                rows.iter()
                    .zip(&rhs)
                    .map(|(row, h)| {
                        let [x, y, z] = &row.coords;
                        format!("{} = {}", show::combination(&[(x, "m_1"), (y, "m_2"), (z, "m_3")]), show::value(h))
                    })
                    .collect::<Vec<_>>()
                    .join(" \\\\ ")
            ),
        );
        // Cramer's rule: the columns of the system, one replaced by the right-hand side at a time.
        let column = |k: usize| Vector3::new(rows[0].coords[k].clone(), rows[1].coords[k].clone(), rows[2].coords[k].clone());
        let right = Vector3::new(rhs[0].clone(), rhs[1].clone(), rhs[2].clone());
        let columns = [column(0), column(1), column(2)];
        let mut coords = Vec::with_capacity(3);
        for k in 0..3 {
            let mut replaced = columns.clone();
            replaced[k] = right.clone();
            coords.push(triple_product(&replaced[0], &replaced[1], &replaced[2])?.div(&determinant)?);
        }
        let centre = Point::new(coords[0].clone(), coords[1].clone(), coords[2].clone());
        working.add(Standard, msg!("Solved, the centre:", "Gelöst, der Mittelpunkt:"), format!("M = {}", show::point(&centre)));
        let ma = centre.to(a);
        let radius = ma.length()?;
        working.add(
            Standard,
            msg!("The radius is the distance to A:", "Der Radius ist der Abstand zu A:"),
            format!("r = \\left|\\overrightarrow{{MA}}\\right| = {} = {}", show::length(&ma), show::value(&radius)),
        );
        let sphere = Sphere::new(&centre, radius)?;
        working.add(Brief, msg!("The sphere:", "Die Kugel:"), sphere.equation());
        Ok(Worked::new(sphere, working))
    }

    /// The coordinate form, `(x - 1)^2 + (y + 2)^2 + z^2 = 9`.
    pub fn equation(&self) -> String {
        let [a, b, c] = &self.centre.coords;
        format!("{} + {} + {} = {}", square("x", a), square("y", b), square("z", c), show::value(&self.radius_squared()))
    }

    /// The vector form, `\left(\vec{x} - \begin{pmatrix} … \end{pmatrix}\right)^2 = 9`.
    pub fn vector_equation(&self) -> String {
        format!("\\left(\\vec{{x}} - {}\\right)^2 = {}", show::vector(&self.centre.position()), show::value(&self.radius_squared()))
    }

    /// Whether `p` lies inside, on or outside the sphere, by comparing `|\overrightarrow{MP}|` with `r`.
    pub fn locate(&self, p: &Point) -> Result<Worked<SpherePoint>> {
        let mut working = Working::default();
        let mp = self.centre.to(p);
        let length = mp.length()?;
        working.add(
            Standard,
            msg!("The distance from the centre:", "Der Abstand zum Mittelpunkt:"),
            format!("\\left|\\overrightarrow{{MP}}\\right| = \\left|{}\\right| = {} = {}", show::vector(&mp), show::length(&mp), show::value(&length)),
        );
        let (squared, r2) = (mp.dot(&mp), self.radius_squared());
        let result = if equal(&squared, &r2) {
            working.add(Brief, msg!("The point lies on the sphere.", "Der Punkt liegt auf der Kugel."), format!("\\left|\\overrightarrow{{MP}}\\right| = r = {}", show::value(&self.radius)));
            SpherePoint::On
        } else if squared.sub(&r2).is_negative()? {
            let distance = self.radius.sub(&length);
            working.add(Brief, msg!("The point lies inside the sphere.", "Der Punkt liegt innerhalb der Kugel."), format!("{} < r = {}", show::value(&length), show::value(&self.radius)));
            working.add(Brief, msg!("Its distance to the sphere:", "Sein Abstand zur Kugel:"), format!("d = r - \\left|\\overrightarrow{{MP}}\\right| = {}", show::value(&distance)));
            SpherePoint::Inside { distance }
        } else {
            let distance = length.sub(&self.radius);
            working.add(Brief, msg!("The point lies outside the sphere.", "Der Punkt liegt außerhalb der Kugel."), format!("{} > r = {}", show::value(&length), show::value(&self.radius)));
            working.add(Brief, msg!("Its distance to the sphere:", "Sein Abstand zur Kugel:"), format!("d = \\left|\\overrightarrow{{MP}}\\right| - r = {}", show::value(&distance)));
            SpherePoint::Outside { distance }
        };
        Ok(Worked::new(result, working))
    }

    /// The surface `O = 4 \pi r^2`.
    pub fn surface(&self) -> Worked<Value> {
        let mut working = Working::default();
        let surface = Value::int(4).mul(&Value::Pi).mul(&self.radius_squared());
        working.add(Detailed, msg!("The formula:", "Die Formel:"), format!("O = 4 \\pi r^2 = 4 \\pi \\cdot {}^2", show::factor(&self.radius)));
        working.add(Brief, msg!("The surface:", "Die Oberfläche:"), format!("O = {}", show::value(&surface)));
        Worked::new(surface, working)
    }

    /// The volume `V = \frac{4}{3} \pi r^3`.
    pub fn volume(&self) -> Worked<Value> {
        let mut working = Working::default();
        let volume = Value::ratio(4, 3).mul(&Value::Pi).mul(&self.radius_squared()).mul(&self.radius);
        working.add(Detailed, msg!("The formula:", "Die Formel:"), format!("V = \\frac{{4}}{{3}} \\pi r^3 = \\frac{{4}}{{3}} \\pi \\cdot {}^3", show::factor(&self.radius)));
        working.add(Brief, msg!("The volume:", "Das Volumen:"), format!("V = {}", show::value(&volume)));
        Worked::new(volume, working)
    }
}
