//! Planes (Ebenen) in parameter, normal and coordinate form.

use dashu::base::{Gcd, UnsignedAbs};
use dashu::integer::{IBig, UBig};
use dashu::rational::RBig;

use crate::num::Value;
use crate::settings::StepDetail::{Brief, Detailed, Standard};
use crate::text::msg;
use crate::{QuantusError, Result};

use super::line::two_parameters;
use super::show;
use super::working::{Worked, Working};
use super::{equal, Point, Vector3};

/// The form a plane is written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaneForm {
    /// `\vec{x} = \vec{p} + \lambda \vec{u} + \mu \vec{v}`.
    Parameter,
    /// `(\vec{x} - \vec{p}) \cdot \vec{n} = 0`.
    Normal,
    /// `ax + by + cz = d`.
    Coordinate,
}

/// A plane through `support` with the normal `normal` and the directions `spans`, which
/// satisfies `\vec{n} \cdot \vec{x} = constant`. All four describe the same plane; `form` is
/// the one it is written in (the one it was given in, until it is converted).
#[derive(Clone, Debug, PartialEq)]
pub struct Plane {
    pub support: Point,
    pub spans: [Vector3; 2],
    pub normal: Vector3,
    pub constant: Value,
    pub form: PlaneForm,
}

fn error(message: String) -> QuantusError {
    QuantusError::other(message)
}

/// A point or vector of a plane is always written in 3D.
fn spatial(p: &Point) -> Point {
    Point { coords: p.coords.clone(), planar: false }
}

fn spatial_vector(v: &Vector3) -> Vector3 {
    Vector3 { coords: v.coords.clone(), planar: false }
}

/// The factor that turns a vector of rationals into the shortest one with whole coprime
/// entries, `(8, -10, 4) → (4, -5, 2)` by `\frac{1}{2}`; `None` if it is 1 or an entry is no rational.
pub(crate) fn shortening(v: &Vector3) -> Option<Value> {
    let rationals: Vec<&RBig> = v.coords.iter().map(Value::as_rational).collect::<Option<_>>()?;
    let lcm = rationals.iter().fold(UBig::ONE, |l, r| {
        let d = r.denominator();
        let g = l.clone().gcd(d);
        l * d / g
    });
    let lcm = IBig::from(lcm);
    let whole: Vec<IBig> = rationals.iter().map(|r| r.numerator() * &lcm / IBig::from(r.denominator().clone())).collect();
    let gcd = whole.iter().filter(|n| *n != &IBig::ZERO).fold(UBig::ZERO, |g, n| if g == UBig::ZERO { n.unsigned_abs() } else { g.gcd(n.unsigned_abs()) });
    if gcd == UBig::ZERO {
        return None;
    }
    let factor = Value::Rat(RBig::from_parts(lcm, gcd));
    (!factor.is_one()).then_some(factor)
}

/// Two directions perpendicular to `n` (not zero): with the first non-zero entry `n_i`, the
/// vector with `-n_j` at `i` and `n_i` at `j`, for both other `j`.
fn perpendicular_pair(n: &Vector3) -> [Vector3; 2] {
    let i = (0..3).find(|&i| !n.coords[i].is_zero()).expect("a normal is not zero");
    let mut spans = (0..3).filter(|&j| j != i).map(|j| {
        let mut coords = [Value::zero(), Value::zero(), Value::zero()];
        coords[i] = n.coords[j].neg();
        coords[j] = n.coords[i].clone();
        Vector3 { coords, planar: false }
    });
    [spans.next().unwrap(), spans.next().unwrap()]
}

impl Plane {
    fn build(support: &Point, normal: Vector3, spans: [Vector3; 2], form: PlaneForm) -> Plane {
        let support = spatial(support);
        let constant = normal.dot(&support.position());
        Plane { support, spans, normal, constant, form }
    }

    /// The plane through `support` spanned by `u` and `v` (parameter form); they must not be
    /// collinear.
    pub fn parametric(support: &Point, u: &Vector3, v: &Vector3) -> Result<Plane> {
        let normal = u.cross(v);
        if normal.is_zero() {
            return Err(error(msg!(
                "The two directions are collinear, so they span no plane.",
                "Die beiden Richtungsvektoren sind kollinear und spannen keine Ebene auf."
            )));
        }
        let normal = match shortening(&normal) {
            Some(factor) => normal.scale(&factor),
            None => normal,
        };
        Ok(Plane::build(support, normal, [spatial_vector(u), spatial_vector(v)], PlaneForm::Parameter))
    }

    /// The plane through `support` with the normal `normal` (normal form).
    pub fn with_normal(support: &Point, normal: &Vector3) -> Result<Plane> {
        if normal.is_zero() {
            return Err(error(msg!("The zero vector is no normal of a plane.", "Der Nullvektor ist kein Normalenvektor einer Ebene.")));
        }
        let normal = spatial_vector(normal);
        Ok(Plane::build(support, normal.clone(), perpendicular_pair(&normal), PlaneForm::Normal))
    }

    /// The plane `ax + by + cz = d` (coordinate form); its support is where it meets the first
    /// axis it is not parallel to.
    pub fn coordinate(a: Value, b: Value, c: Value, d: Value) -> Result<Plane> {
        let normal = Vector3::new(a, b, c);
        if normal.is_zero() {
            return Err(error(msg!(
                "In a coordinate equation of a plane, x, y and z cannot all be missing.",
                "In einer Koordinatengleichung einer Ebene können x, y und z nicht alle fehlen."
            )));
        }
        let i = (0..3).find(|&i| !normal.coords[i].is_zero()).expect("not zero");
        let mut coords = [Value::zero(), Value::zero(), Value::zero()];
        coords[i] = d.div(&normal.coords[i])?;
        let spans = perpendicular_pair(&normal);
        Ok(Plane { support: Point::new(coords[0].clone(), coords[1].clone(), coords[2].clone()), spans, normal, constant: d, form: PlaneForm::Coordinate })
    }

    /// The plane through `a`, `b` and `c`, spanned by `\overrightarrow{AB}` and `\overrightarrow{AC}`;
    /// the points must not lie on one line.
    pub fn through(a: &Point, b: &Point, c: &Point) -> Result<Worked<Plane>> {
        let (u, v) = (a.to(b), a.to(c));
        if u.is_collinear(&v) {
            return Err(error(msg!(
                "The three points lie on one line, so they give no plane.",
                "Die drei Punkte liegen auf einer Geraden und legen keine Ebene fest."
            )));
        }
        let mut working = Working::default();
        let (u, v) = (spatial_vector(&u), spatial_vector(&v));
        working.add(
            Standard,
            msg!("The directions from A to B and from A to C:", "Die Richtungsvektoren von A nach B und von A nach C:"),
            format!("\\overrightarrow{{AB}} = {}, \\quad \\overrightarrow{{AC}} = {}", show::vector(&u), show::vector(&v)),
        );
        let plane = Plane::parametric(a, &u, &v)?;
        working.add(Brief, msg!("The plane through A, B and C:", "Die Ebene durch A, B und C:"), plane.equation());
        Ok(Worked::new(plane, working))
    }

    /// The plane in its form.
    pub fn equation(&self) -> String {
        self.equation_in(self.form)
    }

    /// The plane in `form`.
    pub fn equation_in(&self, form: PlaneForm) -> String {
        match form {
            PlaneForm::Parameter => format!(
                "\\vec{{x}} = {} + \\lambda {} + \\mu {}",
                show::vector(&self.support.position()),
                show::vector(&self.spans[0]),
                show::vector(&self.spans[1])
            ),
            PlaneForm::Normal => format!(
                "\\left(\\vec{{x}} - {}\\right) \\cdot {} = 0",
                show::vector(&self.support.position()),
                show::vector(&self.normal)
            ),
            PlaneForm::Coordinate => {
                let [a, b, c] = &self.normal.coords;
                format!("{} = {}", show::combination(&[(a, "x"), (b, "y"), (c, "z")]), show::value(&self.constant))
            }
        }
    }

    /// The same plane written in `form`, with the working of the conversion.
    pub fn convert(&self, form: PlaneForm) -> Worked<Plane> {
        let mut working = Working::default();
        if form == self.form {
            working.add(Brief, msg!("The plane is already in this form.", "Die Ebene ist schon in dieser Form."), self.equation());
            return Worked::new(self.clone(), working);
        }
        if self.form == PlaneForm::Parameter {
            let [u, v] = &self.spans;
            let cross = u.cross(v);
            working.add(
                Standard,
                msg!("A normal is the cross product of the directions:", "Ein Normalenvektor ist das Kreuzprodukt der Richtungsvektoren:"),
                format!("\\vec{{n}} = {} \\times {} = {}", show::vector(u), show::vector(v), show::vector(&cross)),
            );
            working.add(Detailed, msg!("Component by component:", "Komponentenweise:"), format!("\\vec{{n}} = {} = {}", show::cross(u, v), show::vector(&cross)));
            if let Some(factor) = shortening(&cross) {
                working.add(
                    Standard,
                    msg!("Shortened:", "Gekürzt:"),
                    format!("\\vec{{n}} = {} \\cdot {} = {}", show::value(&factor), show::vector(&cross), show::vector(&self.normal)),
                );
            }
        }
        if form == PlaneForm::Coordinate {
            let n = &self.normal;
            if self.form == PlaneForm::Normal {
                working.add(
                    Standard,
                    msg!("Multiply out the normal form:", "Die Normalenform ausmultiplizieren:"),
                    "\\vec{n} \\cdot \\vec{x} = \\vec{n} \\cdot \\vec{p}",
                );
            }
            working.add(
                Standard,
                msg!("The constant is the normal times the support:", "Die Konstante ist der Normalenvektor mal dem Stützvektor:"),
                format!("d = {} \\cdot {} = {} = {}", show::vector(n), show::vector(&self.support.position()), show::dot(n, &self.support.position()), show::value(&self.constant)),
            );
        }
        if self.form == PlaneForm::Coordinate {
            working.add(
                Standard,
                msg!("The coefficients of x, y and z are a normal:", "Die Koeffizienten von x, y und z bilden einen Normalenvektor:"),
                format!("\\vec{{n}} = {}", show::vector(&self.normal)),
            );
            working.add(
                Standard,
                msg!("A point of the plane:", "Ein Punkt der Ebene:"),
                format!("P = {}", show::point(&self.support)),
            );
        }
        if form == PlaneForm::Parameter {
            let [u, v] = &self.spans;
            working.add(
                Standard,
                msg!("Two directions perpendicular to the normal:", "Zwei Richtungsvektoren senkrecht zum Normalenvektor:"),
                format!("\\vec{{u}} = {}, \\quad \\vec{{v}} = {}", show::vector(u), show::vector(v)),
            );
            working.add(
                Detailed,
                msg!("Check:", "Probe:"),
                format!("\\vec{{n}} \\cdot \\vec{{u}} = {} = 0, \\quad \\vec{{n}} \\cdot \\vec{{v}} = {} = 0", show::dot(&self.normal, u), show::dot(&self.normal, v)),
            );
        }
        let plane = Plane { form, ..self.clone() };
        let named = match form {
            PlaneForm::Parameter => msg!("The parameter form:", "Die Parameterform:"),
            PlaneForm::Normal => msg!("The normal form:", "Die Normalenform:"),
            PlaneForm::Coordinate => msg!("The coordinate form:", "Die Koordinatenform:"),
        };
        working.add(Brief, named, plane.equation());
        Worked::new(plane, working)
    }

    /// Whether `p` lies on the plane (Punktprobe), in the plane's form: by the system of the
    /// parameter form, else by putting the point into the equation.
    pub fn contains(&self, p: &Point) -> Result<Worked<bool>> {
        let mut working = Working::default();
        let on = if self.form == PlaneForm::Parameter {
            let [u, v] = &self.spans;
            working.add(
                Standard,
                msg!("Set the point equal to the plane equation.", "Den Punkt mit der Ebenengleichung gleichsetzen."),
                format!("{} = {} + \\lambda {} + \\mu {}", show::vector(&spatial(p).position()), show::vector(&self.support.position()), show::vector(u), show::vector(v)),
            );
            let Some((_, _, fits)) = two_parameters(u, v, &self.support.to(p), 3, &mut working)? else {
                unreachable!("the directions of a plane are not collinear")
            };
            fits
        } else {
            let left = self.normal.dot(&p.position());
            let fits = equal(&left, &self.constant);
            let relation = if fits { "=" } else { "\\neq" };
            let latex = if self.form == PlaneForm::Normal {
                let offset = spatial_vector(&self.support.to(p));
                let product = left.sub(&self.constant);
                format!("{} \\cdot {} = {} {relation} 0", show::vector(&offset), show::vector(&self.normal), show::value(&product))
            } else {
                format!("{} = {} {relation} {}", show::dot(&self.normal, &spatial(p).position()), show::value(&left), show::value(&self.constant))
            };
            working.add(Standard, msg!("Put the point into the equation:", "Den Punkt in die Gleichung einsetzen:"), latex);
            fits
        };
        working.add(
            Brief,
            if on { msg!("The point lies on the plane.", "Der Punkt liegt in der Ebene.") } else { msg!("The point does not lie on the plane.", "Der Punkt liegt nicht in der Ebene.") },
            if on { "P \\in E" } else { "P \\notin E" },
        );
        Ok(Worked::new(on, working))
    }
}
