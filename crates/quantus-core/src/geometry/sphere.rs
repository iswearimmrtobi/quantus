//! Spheres (Kugeln): centre and radius.

use crate::num::Value;
use crate::settings::StepDetail::{Brief, Detailed, Maximal, Standard};
use crate::text::msg;
use crate::{QuantusError, Result};

use super::show;
use super::working::{Worked, Working};
use super::plane::shortening;
use super::{equal, triple_product, Circle, Line, Plane, PlaneForm, Point, Vector3};

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

/// How a line lies to a sphere.
#[derive(Clone, Debug, PartialEq)]
pub enum SphereLine {
    /// Two common points (Sekante), at the parameters `lambdas` of the line.
    Secant { points: [Point; 2], lambdas: [Value; 2] },
    /// One common point (Tangente).
    Tangent { point: Point, lambda: Value },
    /// No common point (Passante).
    Passant,
}

/// How a plane lies to a sphere, with the distance `d` of the centre to the plane.
#[derive(Clone, Debug, PartialEq)]
pub enum SpherePlane {
    /// They cut in a circle (Schnittkreis).
    Circle { circle: Circle, distance: Value },
    /// The plane touches the sphere in `point` (Tangentialebene).
    Tangent { point: Point },
    /// No contact; `gap` is the distance from the plane to the sphere.
    Apart { distance: Value, gap: Value },
}

/// How two spheres lie to each other.
#[derive(Clone, Debug, PartialEq)]
pub enum SphereSphere {
    Identical,
    /// The same centre, different radii: one inside the other, no contact.
    Concentric,
    /// Outside each other; `gap` is the shortest distance between them.
    Apart { gap: Value },
    /// One inside the other without contact.
    Inside,
    /// They touch in `point`, from outside or (`inside`) from within; `plane` is the common
    /// tangent plane, their Potenzebene.
    Touching { point: Point, inside: bool, plane: Plane },
    /// They cut in `circle`, which lies in their Potenzebene `plane`.
    Intersecting { circle: Circle, plane: Plane },
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
            return Err(error(msg!("core.geometry.sphere.radius_sphere_must_positive")));
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
                "core.geometry.sphere.point_centre_radius_would_0")));
        }
        let mut working = Working::default();
        let radius = mp.length()?;
        working.add(
            Standard,
            msg!("core.geometry.sphere.radius_distance_from_centre_point"),
            format!("r = \\left|\\overrightarrow{{MP}}\\right| = \\left|{}\\right| = {} = {}", show::vector(&mp), show::length(&mp), show::value(&radius)),
        );
        let sphere = Sphere::new(centre, radius)?;
        working.add(Brief, msg!("core.geometry.sphere.sphere"), sphere.equation());
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
                "core.geometry.sphere.four_points_lie_one_plane")));
        }
        let mut working = Working::default();
        working.add(
            Standard,
            msg!(
                "core.geometry.sphere.centre_m_equally_far_from"),
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
        working.add(Standard, msg!("core.geometry.sphere.solved_centre"), format!("M = {}", show::point(&centre)));
        let ma = centre.to(a);
        let radius = ma.length()?;
        working.add(
            Standard,
            msg!("core.geometry.sphere.radius_distance"),
            format!("r = \\left|\\overrightarrow{{MA}}\\right| = {} = {}", show::length(&ma), show::value(&radius)),
        );
        let checks: Vec<String> = [("B", b), ("C", c), ("D", d)]
            .iter()
            .map(|(name, p)| Ok(format!("\\left|\\overrightarrow{{M{name}}}\\right| = {}", show::value(&centre.to(p).length()?))))
            .collect::<Result<_>>()?;
        working.add(Maximal, msg!("core.geometry.sphere.check_other_points_far_away"), checks.join(", \\quad "));
        let sphere = Sphere::new(&centre, radius)?;
        working.add(Brief, msg!("core.geometry.sphere.sphere"), sphere.equation());
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
            msg!("core.geometry.sphere.distance_from_centre"),
            format!("\\left|\\overrightarrow{{MP}}\\right| = \\left|{}\\right| = {} = {}", show::vector(&mp), show::length(&mp), show::value(&length)),
        );
        let (squared, r2) = (mp.dot(&mp), self.radius_squared());
        let result = if equal(&squared, &r2) {
            working.add(Brief, msg!("core.geometry.sphere.point_lies_sphere"), format!("\\left|\\overrightarrow{{MP}}\\right| = r = {}", show::value(&self.radius)));
            SpherePoint::On
        } else if squared.sub(&r2).is_negative()? {
            let distance = self.radius.sub(&length);
            working.add(Brief, msg!("core.geometry.sphere.point_lies_inside_sphere"), format!("{} < r = {}", show::value(&length), show::value(&self.radius)));
            working.add(Brief, msg!("core.geometry.sphere.distance_sphere"), format!("d = r - \\left|\\overrightarrow{{MP}}\\right| = {}", show::value(&distance)));
            SpherePoint::Inside { distance }
        } else {
            let distance = length.sub(&self.radius);
            working.add(Brief, msg!("core.geometry.sphere.point_lies_outside_sphere"), format!("{} > r = {}", show::value(&length), show::value(&self.radius)));
            working.add(Brief, msg!("core.geometry.sphere.distance_sphere"), format!("d = \\left|\\overrightarrow{{MP}}\\right| - r = {}", show::value(&distance)));
            SpherePoint::Outside { distance }
        };
        Ok(Worked::new(result, working))
    }

    /// The surface `O = 4 \pi r^2`.
    pub fn surface(&self) -> Worked<Value> {
        let mut working = Working::default();
        let surface = Value::int(4).mul(&Value::Pi).mul(&self.radius_squared());
        working.add(Detailed, msg!("core.geometry.sphere.formula"), format!("O = 4 \\pi r^2 = 4 \\pi \\cdot {}^2", show::factor(&self.radius)));
        working.add(Brief, msg!("core.geometry.sphere.surface"), format!("O = {}", show::value(&surface)));
        Worked::new(surface, working)
    }

    /// The volume `V = \frac{4}{3} \pi r^3`.
    pub fn volume(&self) -> Worked<Value> {
        let mut working = Working::default();
        let volume = Value::ratio(4, 3).mul(&Value::Pi).mul(&self.radius_squared()).mul(&self.radius);
        working.add(Detailed, msg!("core.geometry.sphere.formula"), format!("V = \\frac{{4}}{{3}} \\pi r^3 = \\frac{{4}}{{3}} \\pi \\cdot {}^3", show::factor(&self.radius)));
        working.add(Brief, msg!("core.geometry.sphere.volume"), format!("V = {}", show::value(&volume)));
        Worked::new(volume, working)
    }

    /// How `line` lies to the sphere: put into the sphere equation, it gives a quadratic
    /// equation in λ with two, one or no solutions.
    pub fn meet_line(&self, line: &Line) -> Result<Worked<SphereLine>> {
        let mut working = Working::default();
        let (u, offset) = (&line.direction, self.centre.to(&line.support));
        // Planar only for the 2D circle behind [`super::Circle::meet_line`]; a sphere is spatial.
        let planar = line.planar() && self.centre.planar;
        let offset = Vector3 { planar, ..offset };
        working.add(
            Standard,
            msg!("core.geometry.sphere.put_line_into_vector_form"),
            format!("\\left({} + \\lambda {}\\right)^2 = {}", show::vector(&offset), show::vector(u), show::value(&self.radius_squared())),
        );
        let a = u.dot(u);
        let b = Value::int(2).mul(&u.dot(&offset));
        let c = offset.dot(&offset).sub(&self.radius_squared());
        working.add(
            Standard,
            msg!("core.geometry.sphere.multiplied_out"),
            format!("{} = 0", show::combination(&[(&a, "\\lambda^2"), (&b, "\\lambda"), (&c, "")])),
        );
        let discriminant = b.mul(&b).sub(&Value::int(4).mul(&a).mul(&c));
        working.add(
            Detailed,
            msg!("core.geometry.sphere.discriminant"),
            format!("D = b^2 - 4ac = {}^2 - 4 \\cdot {} \\cdot {} = {}", show::factor(&b), show::factor(&a), show::factor(&c), show::value(&discriminant)),
        );
        let twice = Value::int(2).mul(&a);
        if equal(&discriminant, &Value::zero()) {
            let lambda = b.neg().div(&twice)?;
            let point = Point { planar, ..line.at(&lambda) };
            working.add(Standard, msg!("core.geometry.sphere.one_solution"), format!("\\lambda = {}", show::value(&lambda)));
            working.add(Brief, msg!("core.geometry.sphere.line_touches_sphere_tangent"), format!("B = {}", show::point(&point)));
            return Ok(Worked::new(SphereLine::Tangent { point, lambda }, working));
        }
        if discriminant.is_negative()? {
            working.add(
                Brief,
                msg!("core.geometry.sphere.no_solution_line_misses_sphere"),
                "D < 0",
            );
            return Ok(Worked::new(SphereLine::Passant, working));
        }
        let root = discriminant.pow(&Value::ratio(1, 2))?;
        let lambdas = [b.neg().sub(&root).div(&twice)?, b.neg().add(&root).div(&twice)?];
        working.add(
            Standard,
            msg!("core.geometry.sphere.two_solutions"),
            format!("\\lambda_1 = {}, \\quad \\lambda_2 = {}", show::value(&lambdas[0]), show::value(&lambdas[1])),
        );
        let points = [Point { planar, ..line.at(&lambdas[0]) }, Point { planar, ..line.at(&lambdas[1]) }];
        working.add(
            Brief,
            msg!("core.geometry.sphere.line_cuts_sphere_secant"),
            format!("S_1 = {}, \\quad S_2 = {}", show::point(&points[0]), show::point(&points[1])),
        );
        Ok(Worked::new(SphereLine::Secant { points, lambdas }, working))
    }

    /// How `plane` lies to the sphere, by the distance of the centre to it: a circle of
    /// intersection, a tangent plane or no contact.
    pub fn meet_plane(&self, plane: &Plane) -> Result<Worked<SpherePlane>> {
        let Worked { result: distance, mut working } = plane.distance(&self.centre)?;
        let (d2, r2) = (distance.mul(&distance), self.radius_squared());
        if equal(&d2, &r2) {
            let Worked { result: point, working: foot } = plane.foot(&self.centre)?;
            working.extend(foot);
            working.add(
                Brief,
                msg!("core.geometry.sphere.d_r_plane_touches_sphere"),
                format!("B = {}", show::point(&point)),
            );
            return Ok(Worked::new(SpherePlane::Tangent { point }, working));
        }
        if r2.sub(&d2).is_negative()? {
            let gap = distance.sub(&self.radius);
            working.add(
                Brief,
                msg!("core.geometry.sphere.d_r_plane_does_not"),
                format!("{} > r = {}", show::value(&distance), show::value(&self.radius)),
            );
            working.add(Standard, msg!("core.geometry.sphere.gap_between_them"), format!("d - r = {}", show::value(&gap)));
            return Ok(Worked::new(SpherePlane::Apart { distance, gap }, working));
        }
        working.add(
            Standard,
            msg!("core.geometry.sphere.d_r_plane_cuts_sphere"),
            format!("{} < r = {}", show::value(&distance), show::value(&self.radius)),
        );
        let Worked { result: centre, working: foot } = plane.foot(&self.centre)?;
        working.extend(foot);
        let radius = r2.sub(&d2).pow(&Value::ratio(1, 2))?;
        working.add(
            Standard,
            msg!("core.geometry.sphere.radius_circle_pythagoras"),
            format!("\\rho = \\sqrt{{r^2 - d^2}} = \\sqrt{{{} - {}}} = {}", show::value(&r2), show::value(&d2), show::value(&radius)),
        );
        working.add(
            Brief,
            msg!("core.geometry.sphere.circle_intersection"),
            format!("M' = {}, \\quad \\rho = {}", show::point(&centre), show::value(&radius)),
        );
        let circle = Circle { centre, radius, normal: plane.normal.clone() };
        Ok(Worked::new(SpherePlane::Circle { circle, distance }, working))
    }

    /// The tangent plane at `p`, which must lie on the sphere: `\overrightarrow{MP}` is its normal.
    pub fn tangent_plane(&self, p: &Point) -> Result<Worked<Plane>> {
        let normal = self.centre.to(p);
        if !equal(&normal.dot(&normal), &self.radius_squared()) {
            return Err(error(msg!(
                "core.geometry.sphere.point_does_not_lie_sphere")));
        }
        let mut working = Working::default();
        let normal = Vector3 { planar: false, ..normal };
        working.add(
            Standard,
            msg!("core.geometry.sphere.radius_point_normal"),
            format!("\\vec{{n}} = \\overrightarrow{{MP}} = {}", show::vector(&normal)),
        );
        let plane = Plane::with_normal(p, &normal)?;
        working.add(Brief, msg!("core.geometry.sphere.tangent_plane"), plane.equation());
        working.add(Standard, msg!("core.geometry.sphere.coordinate_form"), plane.equation_in(PlaneForm::Coordinate));
        Ok(Worked::new(plane, working))
    }

    /// How the sphere lies to `other`, by the distance of the centres against the sum and the
    /// difference of the radii. Where they meet, the difference of their equations is the
    /// Potenzebene, and the circle (or point) is where it cuts this sphere.
    pub fn meet_sphere(&self, other: &Sphere) -> Result<Worked<SphereSphere>> {
        let mut working = Working::default();
        let between = self.centre.to(&other.centre);
        let d2 = between.dot(&between);
        let distance = d2.pow(&Value::ratio(1, 2))?;
        working.add(
            Standard,
            msg!("core.geometry.sphere.distance_centres"),
            format!("d = \\left|\\overrightarrow{{M_1 M_2}}\\right| = \\left|{}\\right| = {}", show::vector(&between), show::value(&distance)),
        );
        let (r1, r2) = (&self.radius, &other.radius);
        if d2.is_zero() {
            if equal(r1, r2) {
                working.add(Brief, msg!("core.geometry.sphere.same_centre_radius_spheres_identical"), "K_1 = K_2");
                return Ok(Worked::new(SphereSphere::Identical, working));
            }
            working.add(
                Brief,
                msg!("core.geometry.sphere.same_centre_different_radii_one"),
                format!("r_1 = {} \\neq r_2 = {}", show::value(r1), show::value(r2)),
            );
            return Ok(Worked::new(SphereSphere::Concentric, working));
        }
        let sum = r1.add(r2);
        let difference = super::abs(r1.sub(r2))?;
        working.add(
            Standard,
            msg!("core.geometry.sphere.compare_sum_difference_radii"),
            format!("r_1 + r_2 = {}, \\quad \\left|r_1 - r_2\\right| = {}", show::value(&sum), show::value(&difference)),
        );
        if sum.mul(&sum).sub(&d2).is_negative()? && !equal(&sum.mul(&sum), &d2) {
            let gap = distance.sub(&sum);
            working.add(
                Brief,
                msg!("core.geometry.sphere.d_r_r_spheres_lie"),
                format!("{} > {}", show::value(&distance), show::value(&sum)),
            );
            working.add(Standard, msg!("core.geometry.sphere.gap_between_them"), format!("d - r_1 - r_2 = {}", show::value(&gap)));
            return Ok(Worked::new(SphereSphere::Apart { gap }, working));
        }
        if d2.sub(&difference.mul(&difference)).is_negative()? && !equal(&difference.mul(&difference), &d2) {
            working.add(
                Brief,
                msg!("core.geometry.sphere.d_r_r_one_sphere"),
                format!("{} < {}", show::value(&distance), show::value(&difference)),
            );
            return Ok(Worked::new(SphereSphere::Inside, working));
        }
        // The Potenzebene: the difference of the equations, 2(m₂ − m₁)·x = r₁² − r₂² − m₁² + m₂².
        let (m1, m2) = (self.centre.position(), other.centre.position());
        let normal = between.scale(&Value::int(2));
        let constant = self.radius_squared().sub(&other.radius_squared()).sub(&m1.dot(&m1)).add(&m2.dot(&m2));
        let (normal, constant) = match shortening(&normal) {
            Some(factor) => (normal.scale(&factor), constant.mul(&factor)),
            None => (normal, constant),
        };
        let [a, b, c] = normal.coords.clone();
        let plane = Plane::coordinate(a, b, c, constant)?;
        working.add(
            Standard,
            msg!("core.geometry.sphere.subtract_equations_spheres_x_y"),
            "2 \\left(\\vec{m_2} - \\vec{m_1}\\right) \\cdot \\vec{x} = r_1^2 - r_2^2 - \\vec{m_1}^2 + \\vec{m_2}^2",
        );
        working.add(Brief, msg!("core.geometry.sphere.potenzebene"), plane.equation());
        let Worked { result, working: cut } = self.meet_plane(&plane)?;
        working.extend(cut);
        let relation = match result {
            SpherePlane::Circle { circle, .. } => SphereSphere::Intersecting { circle, plane },
            SpherePlane::Tangent { point } => SphereSphere::Touching { point, inside: equal(&difference.mul(&difference), &d2), plane },
            SpherePlane::Apart { .. } => unreachable!("the distance lies between the difference and the sum of the radii"),
        };
        Ok(Worked::new(relation, working))
    }
}
