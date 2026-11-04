//! Circles (Kreise) in space: centre, radius and the normal of their plane.

use crate::num::Value;

use super::{Point, Vector3};

/// A circle around `centre` with `radius` (> 0) in the plane through the centre perpendicular
/// to `normal`.
#[derive(Clone, Debug, PartialEq)]
pub struct Circle {
    pub centre: Point,
    pub radius: Value,
    pub normal: Vector3,
}
