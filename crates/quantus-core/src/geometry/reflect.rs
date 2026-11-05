//! Reflections (Spiegelungen) of a point in a point, a line and a plane.

use crate::settings::StepDetail::{Brief, Detailed, Standard};
use crate::text::msg;
use crate::Result;

use super::show;
use super::working::{Worked, Working};
use super::{Line, Plane, PlaneForm, Point, Vector3};

/// `P' = P + 2 \overrightarrow{PF}` in the working, with `F` (called `name`) the centre of the reflection.
fn mirrored(p: &Point, centre: &Point, name: &str, working: &mut Working) -> Point {
    let image = centre + &p.to(centre);
    working.add(
        Standard,
        msg!("core.geometry.reflect.p_lies_far_beyond_p"),
        format!(
            "\\vec{{p'}} = \\vec{{p}} + 2 \\cdot \\overrightarrow{{P{name}}} = {} + 2 \\cdot {} = {}",
            show::vector(&p.position()),
            show::vector(&p.to(centre)),
            show::vector(&image.position())
        ),
    );
    working.add(Brief, msg!("core.geometry.reflect.image_point"), format!("P' = {}", show::point(&image)));
    image
}

impl Point {
    /// The reflection of the point in `centre` (Punktspiegelung).
    pub fn reflect_in_point(&self, centre: &Point) -> Worked<Point> {
        let mut working = Working::default();
        let image = mirrored(self, centre, "M", &mut working);
        Worked::new(image, working)
    }
}

impl Line {
    /// The reflection of `p` in the line: through the foot of the perpendicular.
    pub fn reflect(&self, p: &Point) -> Worked<Point> {
        let Worked { result: foot, mut working } = self.foot(p);
        let image = mirrored(p, &foot, "F", &mut working);
        Worked::new(image, working)
    }
}

impl Plane {
    /// The foot of the perpendicular from `p` onto the plane: along `\vec{x} = \vec{p} + \lambda \vec{n}`.
    pub fn foot(&self, p: &Point) -> Result<Worked<Point>> {
        let mut working = Working::default();
        let plane = if self.form == PlaneForm::Coordinate {
            self.clone()
        } else {
            let Worked { result, working: conversion } = self.convert(PlaneForm::Coordinate);
            working.extend(conversion);
            result
        };
        let n = &plane.normal;
        let position = Vector3 { coords: p.coords.clone(), planar: false };
        working.add(
            Standard,
            msg!("core.geometry.reflect.perpendicular_through_point_along_normal"),
            format!("\\vec{{x}} = {} + \\lambda {}", show::vector(&position), show::vector(n)),
        );
        let (constant, coefficient) = (n.dot(&position), n.dot(n));
        let lambda = plane.constant.sub(&constant).div(&coefficient)?;
        working.add(
            Standard,
            msg!("core.geometry.reflect.put_into_coordinate_form_solve"),
            format!(
                "{} = {} \\quad \\Rightarrow \\quad \\lambda = {}",
                show::combination(&[(&constant, ""), (&coefficient, "\\lambda")]),
                show::value(&plane.constant),
                show::value(&lambda)
            ),
        );
        let foot = Point { planar: false, ..p + &n.scale(&lambda) };
        working.add(Detailed, msg!("core.geometry.reflect.foot_perpendicular"), format!("F = {}", show::point(&foot)));
        Ok(Worked::new(foot, working))
    }

    /// The reflection of `p` in the plane: through the foot of the perpendicular.
    pub fn reflect(&self, p: &Point) -> Result<Worked<Point>> {
        let Worked { result: foot, mut working } = self.foot(p)?;
        let image = mirrored(&Point { planar: false, ..p.clone() }, &foot, "F", &mut working);
        Ok(Worked::new(image, working))
    }
}
