//! Analytic geometry (the Geometry mode): points and vectors in 3D with exact coordinates.
//!
//! A [`Point`] and a [`Vector3`] are different types: the difference of two points is a vector,
//! a point plus a vector is a point, and vectors add and scale. A 2D input is z = 0 and keeps
//! that it was 2D (`planar`), so that a result of 2D inputs can be written without z.

mod circle;
mod eval;
pub mod interface;
mod line;
mod plane;
mod reflect;
mod relations;
mod show;
mod sphere;
mod working;

pub use circle::{Circle, CircleLine, CirclePlane};
pub use eval::{GeometryEvaluator, Item, Objects};
pub use line::{Line, LineRelation};
pub use plane::{Plane, PlaneForm};
pub use relations::{LinePlane, PlanePlane};
pub use sphere::{Sphere, SphereLine, SpherePlane, SpherePoint, SphereSphere};
pub use working::{Note, Worked, Working};

use std::ops::{Add, Mul, Neg, Sub};

use crate::eval::angles;
use crate::linear;
use crate::matrix::Matrix;
use crate::num::{Decimal, MathFunction, Value};
use crate::settings::AngleUnit;
use crate::{QuantusError, Result};

/// The three coordinates, each exact.
pub type Coordinates = [Value; 3];

/// A vector in space. Equal when the coordinates are; `planar` only decides how it is written.
#[derive(Clone, Debug)]
pub struct Vector3 {
    pub coords: Coordinates,
    /// Entered with two coordinates (z = 0).
    pub planar: bool,
}

/// A point in space. Equal when the coordinates are; `planar` only decides how it is written.
#[derive(Clone, Debug)]
pub struct Point {
    pub coords: Coordinates,
    /// Entered with two coordinates (z = 0).
    pub planar: bool,
}

/// Whether two values are equal, with a difference below the rounding noise of roots taken as 0.
fn equal(a: &Value, b: &Value) -> bool {
    let mut difference = a.sub(b);
    linear::settle(&mut difference);
    difference.is_zero()
}

fn zip(a: &Coordinates, b: &Coordinates, f: impl Fn(&Value, &Value) -> Value) -> Coordinates {
    [f(&a[0], &b[0]), f(&a[1], &b[1]), f(&a[2], &b[2])]
}

impl Vector3 {
    pub fn new(x: Value, y: Value, z: Value) -> Vector3 {
        Vector3 { coords: [x, y, z], planar: false }
    }

    /// A 2D vector, z = 0.
    pub fn planar(x: Value, y: Value) -> Vector3 {
        Vector3 { coords: [x, y, Value::zero()], planar: true }
    }

    pub fn from_ints(x: i64, y: i64, z: i64) -> Vector3 {
        Vector3::new(Value::int(x), Value::int(y), Value::int(z))
    }

    pub fn zero() -> Vector3 {
        Vector3::new(Value::zero(), Value::zero(), Value::zero())
    }

    pub fn is_zero(&self) -> bool {
        self.coords.iter().all(Value::is_zero)
    }

    /// `factor · self`.
    pub fn scale(&self, factor: &Value) -> Vector3 {
        Vector3 { coords: self.coords.clone().map(|c| factor.mul(&c)), planar: self.planar }
    }

    /// The dot product (Skalarprodukt) `self · other`.
    pub fn dot(&self, other: &Vector3) -> Value {
        self.coords.iter().zip(&other.coords).fold(Value::zero(), |sum, (a, b)| sum.add(&a.mul(b)))
    }

    /// The length (Betrag) `|self|`, exact: `√(x² + y² + z²)`.
    pub fn length(&self) -> Result<Value> {
        self.dot(self).pow(&Value::ratio(1, 2))
    }

    /// The unit vector (Einheitsvektor) `self / |self|`; the zero vector has none.
    pub fn unit(&self) -> Result<Vector3> {
        if self.is_zero() {
            return Err(QuantusError::other("The zero vector has no unit vector."));
        }
        // `|v|^{-1}` as one power, so that its square folds back to a fraction.
        Ok(self.scale(&self.dot(self).pow(&Value::ratio(-1, 2))?))
    }

    /// The angle between `self` and `other`, from 0 to π (or 180°), in `unit`; the zero vector
    /// makes none.
    pub fn angle(&self, other: &Vector3, unit: AngleUnit, precision: usize) -> Result<Angle> {
        if self.is_zero() || other.is_zero() {
            return Err(QuantusError::other("The zero vector makes no angle."));
        }
        // One root over the product of the squared lengths keeps the cosine in its simplest form.
        let cosine = self.dot(other).mul(&self.dot(self).mul(&other.dot(other)).pow(&Value::ratio(-1, 2))?);
        Angle::of(cosine, false, unit, precision)
    }

    /// The cross product (Kreuzprodukt) `self × other`, by the cofactors of the first row of
    /// the 3×3 determinant with the unit vectors there.
    pub fn cross(&self, other: &Vector3) -> Vector3 {
        let [a1, a2, a3] = &self.coords;
        let [b1, b2, b3] = &other.coords;
        let minor = |p: &Value, q: &Value, r: &Value, s: &Value| {
            let mut entry = p.mul(q).sub(&r.mul(s));
            linear::settle(&mut entry);
            entry
        };
        Vector3::new(minor(a2, b3, a3, b2), minor(a3, b1, a1, b3), minor(a1, b2, a2, b1))
    }

    /// Whether `self` and `other` are collinear (linear abhängig): their cross product is zero.
    /// The zero vector is collinear with every vector.
    pub fn is_collinear(&self, other: &Vector3) -> bool {
        self.cross(other).is_zero()
    }

    /// The point this vector leads to from the origin.
    pub fn to_point(&self) -> Point {
        Point { coords: self.coords.clone(), planar: self.planar }
    }
}

/// The triple product (Spatprodukt) `(a × b) · c`, the determinant of the matrix with the
/// rows a, b and c: the signed volume of the parallelepiped they span.
pub fn triple_product(a: &Vector3, b: &Vector3, c: &Vector3) -> Result<Value> {
    let mut determinant = Matrix::new(vec![a.coords.to_vec(), b.coords.to_vec(), c.coords.to_vec()])?.determinant()?.matrix;
    linear::settle(&mut determinant);
    Ok(determinant)
}

/// Whether `a`, `b` and `c` are coplanar (lie in one plane): their triple product is zero.
pub fn coplanar(a: &Vector3, b: &Vector3, c: &Vector3) -> Result<bool> {
    Ok(triple_product(a, b, c)?.is_zero())
}

/// Whether three points lie on one line.
pub fn points_collinear(a: &Point, b: &Point, c: &Point) -> bool {
    a.to(b).is_collinear(&a.to(c))
}

/// Whether four points lie in one plane.
pub fn points_coplanar(a: &Point, b: &Point, c: &Point, d: &Point) -> Result<bool> {
    coplanar(&a.to(b), &a.to(c), &a.to(d))
}

/// `value / |v|`, with the length as one power so that roots fold: `\frac{1}{\sqrt{3}}` squares to `\frac{1}{3}`.
fn over_length(value: &Value, v: &Vector3) -> Result<Value> {
    Ok(value.mul(&v.dot(v).pow(&Value::ratio(-1, 2))?))
}

fn abs(value: Value) -> Result<Value> {
    Ok(if value.is_negative()? { value.neg() } else { value })
}

/// The area of the parallelogram spanned by `a` and `b`: `|a × b|`.
pub fn parallelogram_area(a: &Vector3, b: &Vector3) -> Result<Value> {
    a.cross(b).length()
}

/// The area of the triangle ABC: `½ · |AB × AC|`.
pub fn triangle_area(a: &Point, b: &Point, c: &Point) -> Result<Value> {
    Ok(Value::ratio(1, 2).mul(&parallelogram_area(&a.to(b), &a.to(c))?))
}

/// The volume of the parallelepiped (Spat) spanned by `a`, `b` and `c`: `|(a × b) · c|`.
pub fn parallelepiped_volume(a: &Vector3, b: &Vector3, c: &Vector3) -> Result<Value> {
    abs(triple_product(a, b, c)?)
}

/// The volume of the tetrahedron (dreiseitige Pyramide) ABCD: `⅙ · |(AB × AC) · AD|`.
pub fn tetrahedron_volume(a: &Point, b: &Point, c: &Point, d: &Point) -> Result<Value> {
    Ok(Value::ratio(1, 6).mul(&parallelepiped_volume(&a.to(b), &a.to(c), &a.to(d))?))
}

/// `radians` in degrees; a rational multiple of π becomes a plain number.
fn degrees(radians: &Value) -> Result<Value> {
    let multiple = match radians {
        Value::Pi => Some(Value::one()),
        Value::Mul(factors) => match factors.as_slice() {
            [r @ Value::Rat(_), Value::Pi] | [Value::Pi, r @ Value::Rat(_)] => Some(r.clone()),
            _ => None,
        },
        r @ Value::Rat(_) if r.is_zero() => Some(Value::zero()),
        _ => None,
    };
    match multiple {
        Some(multiple) => Ok(multiple.mul(&Value::int(180))),
        None => angles::to_degrees(radians),
    }
}

/// An angle, `\arccos(ratio)` or, between a line and a plane, `\arcsin(ratio)`.
#[derive(Clone, Debug, PartialEq)]
pub struct Angle {
    /// The exact cosine of the angle, or its sine if `sine`.
    pub ratio: Value,
    pub sine: bool,
    /// The exact angle in `unit`: `\frac{\pi}{3}` or `60` where it folds, else the `arccos`.
    pub value: Value,
    pub unit: AngleUnit,
    pub decimal: Decimal,
}

impl Angle {
    /// The angle whose cosine (or sine) is `ratio`, in `unit`.
    pub(crate) fn of(ratio: Value, sine: bool, unit: AngleUnit, precision: usize) -> Result<Angle> {
        let radians = ratio.apply(if sine { MathFunction::Arcsin } else { MathFunction::Arccos })?;
        let value = match unit {
            AngleUnit::Radian => radians,
            AngleUnit::Degree => degrees(&radians)?,
        };
        let decimal = value.decimal(precision)?;
        Ok(Angle { ratio, sine, value, unit, decimal })
    }

    /// The name of the inverse function, `arccos` or `arcsin`.
    pub fn function(&self) -> &'static str {
        if self.sine { "arcsin" } else { "arccos" }
    }
}

impl Point {
    pub fn new(x: Value, y: Value, z: Value) -> Point {
        Point { coords: [x, y, z], planar: false }
    }

    /// A 2D point, z = 0.
    pub fn planar(x: Value, y: Value) -> Point {
        Point { coords: [x, y, Value::zero()], planar: true }
    }

    pub fn from_ints(x: i64, y: i64, z: i64) -> Point {
        Point::new(Value::int(x), Value::int(y), Value::int(z))
    }

    pub fn origin() -> Point {
        Point::new(Value::zero(), Value::zero(), Value::zero())
    }

    /// The position vector (Ortsvektor) of the point.
    pub fn position(&self) -> Vector3 {
        Vector3 { coords: self.coords.clone(), planar: self.planar }
    }

    /// The vector from `self` to `other`: `other − self`.
    pub fn to(&self, other: &Point) -> Vector3 {
        other - self
    }
}

impl PartialEq for Vector3 {
    fn eq(&self, other: &Self) -> bool {
        self.coords == other.coords
    }
}

impl PartialEq for Point {
    fn eq(&self, other: &Self) -> bool {
        self.coords == other.coords
    }
}

impl Sub for &Point {
    type Output = Vector3;
    fn sub(self, other: &Point) -> Vector3 {
        Vector3 { coords: zip(&self.coords, &other.coords, Value::sub), planar: self.planar && other.planar }
    }
}

impl Add<&Vector3> for &Point {
    type Output = Point;
    fn add(self, v: &Vector3) -> Point {
        Point { coords: zip(&self.coords, &v.coords, Value::add), planar: self.planar && v.planar }
    }
}

impl Sub<&Vector3> for &Point {
    type Output = Point;
    fn sub(self, v: &Vector3) -> Point {
        Point { coords: zip(&self.coords, &v.coords, Value::sub), planar: self.planar && v.planar }
    }
}

impl Add for &Vector3 {
    type Output = Vector3;
    fn add(self, other: &Vector3) -> Vector3 {
        Vector3 { coords: zip(&self.coords, &other.coords, Value::add), planar: self.planar && other.planar }
    }
}

impl Sub for &Vector3 {
    type Output = Vector3;
    fn sub(self, other: &Vector3) -> Vector3 {
        Vector3 { coords: zip(&self.coords, &other.coords, Value::sub), planar: self.planar && other.planar }
    }
}

impl Neg for &Vector3 {
    type Output = Vector3;
    fn neg(self) -> Vector3 {
        self.scale(&Value::int(-1))
    }
}

impl Mul<&Vector3> for &Value {
    type Output = Vector3;
    fn mul(self, v: &Vector3) -> Vector3 {
        v.scale(self)
    }
}
