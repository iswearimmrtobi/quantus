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
        msg!("P' lies as far beyond {name} as P lies before it:", "P' liegt so weit hinter {name} wie P davor:"),
        format!(
            "\\vec{{p'}} = \\vec{{p}} + 2 \\cdot \\overrightarrow{{P{name}}} = {} + 2 \\cdot {} = {}",
            show::vector(&p.position()),
            show::vector(&p.to(centre)),
            show::vector(&image.position())
        ),
    );
    working.add(Brief, msg!("The image point:", "Der Bildpunkt:"), format!("P' = {}", show::point(&image)));
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
    /// The reflection of `p` in the plane: along the perpendicular `\vec{x} = \vec{p} + \lambda \vec{n}`
    /// to its foot.
    pub fn reflect(&self, p: &Point) -> Result<Worked<Point>> {
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
            msg!("The perpendicular through P along the normal:", "Die Lotgerade durch P in Richtung des Normalenvektors:"),
            format!("\\vec{{x}} = {} + \\lambda {}", show::vector(&position), show::vector(n)),
        );
        let (constant, coefficient) = (n.dot(&position), n.dot(n));
        let lambda = plane.constant.sub(&constant).div(&coefficient)?;
        working.add(
            Standard,
            msg!("Put it into the coordinate form and solve for λ:", "In die Koordinatenform einsetzen und nach λ auflösen:"),
            format!(
                "{} = {} \\quad \\Rightarrow \\quad \\lambda = {}",
                show::combination(&[(&constant, ""), (&coefficient, "\\lambda")]),
                show::value(&plane.constant),
                show::value(&lambda)
            ),
        );
        let foot = Point { planar: false, ..p + &n.scale(&lambda) };
        working.add(Detailed, msg!("The foot of the perpendicular:", "Der Lotfußpunkt:"), format!("F = {}", show::point(&foot)));
        let image = mirrored(&Point { planar: false, ..p.clone() }, &foot, "F", &mut working);
        Ok(Worked::new(image, working))
    }
}
