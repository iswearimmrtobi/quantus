//! How lines and planes lie to each other, and distances to a plane.

use crate::num::Value;
use crate::settings::AngleUnit;
use crate::settings::StepDetail::{Brief, Detailed, Maximal, Standard};
use crate::text::msg;
use crate::Result;

use super::line::{acute_angle, compare_directions};
use super::show;
use super::working::{Worked, Working};
use super::{abs, equal, over_length, Angle, Line, Plane, PlaneForm, Point};

/// How two planes lie to each other.
#[derive(Clone, Debug, PartialEq)]
pub enum PlanePlane {
    Identical,
    Parallel { distance: Value },
    /// They meet in `line` (Schnittgerade).
    Intersecting { line: Line, angle: Angle },
}

/// How a line lies to a plane.
#[derive(Clone, Debug, PartialEq)]
pub enum LinePlane {
    /// The line meets the plane in `point`, at its parameter `lambda`.
    Intersecting { point: Point, lambda: Value, angle: Angle },
    Parallel { distance: Value },
    /// The line lies in the plane.
    Contained,
}

impl Plane {
    /// The plane in coordinate form, with the working of the conversion if it was in another.
    pub(crate) fn in_coordinates(&self, working: &mut Working) -> Plane {
        if self.form == PlaneForm::Coordinate {
            return self.clone();
        }
        let Worked { result, working: conversion } = self.convert(PlaneForm::Coordinate);
        working.extend(conversion);
        result
    }

    /// The distance from `p` to the plane, by the Hesse normal form:
    /// `d = \frac{|\vec{n} \cdot \vec{p} - k|}{|\vec{n}|}`.
    pub fn distance(&self, p: &Point) -> Result<Worked<Value>> {
        let mut working = Working::default();
        let plane = self.in_coordinates(&mut working);
        let n = &plane.normal;
        let offset = n.dot(&p.position()).sub(&plane.constant);
        let distance = over_length(&abs(offset.clone())?, n)?;
        let [a, b, c] = &n.coords;
        let [x, y, z] = &p.coords;
        let products: Vec<String> = [(a, x), (b, y), (c, z)]
            .iter()
            .filter(|(a, _)| !a.is_zero())
            .enumerate()
            .map(|(i, (a, x))| format!("{} \\cdot {}", if i == 0 { show::value(a) } else { show::factor(a) }, show::factor(x)))
            .collect();
        let length = n.length()?;
        // `\frac{3}{1}` is just 3.
        let quotient = if length.is_one() { show::value(&abs(offset.clone())?) } else { format!("\\frac{{{}}}{{{}}}", show::value(&abs(offset)?), show::value(&length)) };
        working.add(
            Standard,
            msg!("core.geometry.relations.hesse_distance"),
            format!(
                "d = \\frac{{\\left|{} - {}\\right|}}{{{}}} = {}",
                products.join(" + "),
                show::factor(&plane.constant),
                show::length(n),
                quotient
            ),
        );
        working.add(Brief, msg!("core.geometry.relations.distance"), format!("d = {}", show::value(&distance)));
        Ok(Worked::new(distance, working))
    }

    /// How `line` lies to the plane: it meets it (with the point and the angle), is parallel
    /// (with the distance) or lies in it. The line is put into the coordinate form.
    pub fn meet_line(&self, line: &Line, unit: AngleUnit, precision: usize) -> Result<Worked<LinePlane>> {
        let mut working = Working::default();
        let plane = self.in_coordinates(&mut working);
        let (n, s, u) = (&plane.normal, line.support.position(), &line.direction);
        let mut substituted = String::new();
        for i in (0..3).filter(|&i| !n.coords[i].is_zero()) {
            let c = &n.coords[i];
            let negative = c.is_negative()?;
            let size = if negative { c.neg() } else { c.clone() };
            let inner = show::combination(&[(&s.coords[i], ""), (&u.coords[i], "\\lambda")]);
            let bracketed = inner.contains(' ') || inner.starts_with('-');
            let term = match (size.is_one(), bracketed) {
                (true, true) => format!("\\left({inner}\\right)"),
                (true, false) => inner,
                (false, true) => format!("{} \\left({inner}\\right)", show::factor(&size)),
                (false, false) => format!("{} \\cdot {inner}", show::factor(&size)),
            };
            substituted = match (substituted.is_empty(), negative) {
                (true, false) => term,
                (true, true) => format!("-{term}"),
                (false, false) => format!("{substituted} + {term}"),
                (false, true) => format!("{substituted} - {term}"),
            };
        }
        working.add(
            Standard,
            msg!("core.geometry.relations.line_into_plane"),
            format!("{substituted} = {}", show::value(&plane.constant)),
        );
        let (constant, coefficient) = (n.dot(&s), n.dot(u));
        let equation = format!("{} = {}", show::combination(&[(&constant, ""), (&coefficient, "\\lambda")]), show::value(&plane.constant));
        if coefficient.is_zero() {
            let holds = equal(&constant, &plane.constant);
            working.add(
                Standard,
                msg!("core.geometry.relations.drops_out"),
                format!("{equation} \\quad ({})", if holds { msg!("core.geometry.relations.true") } else { msg!("core.geometry.relations.false") }),
            );
            if holds {
                working.add(Brief, msg!("core.geometry.relations.line_lies_plane"), "g \\subset E");
                return Ok(Worked::new(LinePlane::Contained, working));
            }
            working.add(Brief, msg!("core.geometry.relations.line_parallel_plane"), "g \\parallel E");
            let Worked { result: distance, working: part } = plane.distance(&line.support)?;
            working.extend(part);
            return Ok(Worked::new(LinePlane::Parallel { distance }, working));
        }
        let lambda = plane.constant.sub(&constant).div(&coefficient)?;
        working.add(Detailed, msg!("core.geometry.relations.solve"), format!("{equation} \\quad \\Rightarrow \\quad \\lambda = {}", show::value(&lambda)));
        let point = line.at(&lambda);
        let point = Point { planar: false, ..point };
        working.add(
            Standard,
            msg!("core.geometry.relations.put_into_line"),
            format!("\\vec{{s}} = {} + {} \\cdot {}", show::vector(&s), show::factor(&lambda), show::vector(u)),
        );
        working.add(Brief, msg!("core.geometry.relations.line_meets_plane_point"), format!("S = {}", show::point(&point)));
        working.add(
            Maximal,
            msg!("core.geometry.relations.check_in_plane"),
            format!("{} = {}", show::dot(n, &point.position()), show::value(&n.dot(&point.position()))),
        );
        // The angle to the plane is 90° minus the angle to the normal: its sine is that cosine.
        let ratio = over_length(&over_length(&abs(coefficient.clone())?, n)?, u)?;
        let angle = Angle::of(ratio, true, unit, precision)?;
        working.add(
            Standard,
            msg!("core.geometry.relations.angle_between_line_plane"),
            format!(
                "\\sin\\varphi = \\frac{{\\left|\\vec{{n}} \\cdot \\vec{{u}}\\right|}}{{\\left|\\vec{{n}}\\right| \\cdot \\left|\\vec{{u}}\\right|}} = \\frac{{{}}}{{{} \\cdot {}}} = {}",
                show::value(&abs(coefficient)?),
                show::value(&n.length()?),
                show::value(&u.length()?),
                show::value(&angle.ratio)
            ),
        );
        working.add(Brief, msg!("core.geometry.relations.angle"), show::angle(&angle));
        Ok(Worked::new(LinePlane::Intersecting { point, lambda, angle }, working))
    }

    /// How the plane lies to `other`: identical, parallel (with the distance) or intersecting
    /// (with the line and the angle). Both are put into the coordinate form.
    pub fn meet_plane(&self, other: &Plane, unit: AngleUnit, precision: usize) -> Result<Worked<PlanePlane>> {
        let mut working = Working::default();
        let e = self.in_coordinates(&mut working);
        let f = other.in_coordinates(&mut working);
        let (n1, n2) = (&e.normal, &f.normal);
        if compare_directions(n1, n2, &mut working).is_some() {
            let Worked { result: on, working: test } = e.contains(&f.support)?;
            working.extend(test);
            if on {
                working.add(Brief, msg!("core.geometry.relations.planes_identical"), "E = F");
                return Ok(Worked::new(PlanePlane::Identical, working));
            }
            working.add(Brief, msg!("core.geometry.relations.planes_parallel"), "E \\parallel F");
            let Worked { result: distance, working: part } = e.distance(&f.support)?;
            working.extend(part);
            return Ok(Worked::new(PlanePlane::Parallel { distance }, working));
        }
        let line = cut_line(&e, &f, &mut working)?;
        let angle = acute_angle(n1, n2, true, unit, precision, &mut working)?;
        Ok(Worked::new(PlanePlane::Intersecting { line, angle }, working))
    }
}

/// The line where the planes `e` and `f` (in coordinate form, with normals that are not
/// collinear) meet: along `\vec{n_1} \times \vec{n_2}`, through the common point with one
/// coordinate 0.
pub(crate) fn cut_line(e: &Plane, f: &Plane, working: &mut Working) -> Result<Line> {
    let (n1, n2) = (&e.normal, &f.normal);
    let direction = n1.cross(n2);
    working.add(
        Standard,
        msg!("core.geometry.relations.intersection_direction"),
        format!("\\vec{{u}} = \\vec{{n_1}} \\times \\vec{{n_2}} = {} \\times {} = {}", show::vector(n1), show::vector(n2), show::vector(&direction)),
    );
    // A common point: one coordinate 0, where the other two have a non-zero determinant
    // (the entry of the cross product there); z first, as at school.
    let m = [2, 1, 0].into_iter().find(|&m| !direction.coords[m].is_zero()).expect("the normals are not collinear");
    let (i, j) = match m {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    };
    let names = ["x", "y", "z"];
    let (a, b, d1, d2) = (&n1.coords, &n2.coords, &e.constant, &f.constant);
    let equations = [(a, d1), (b, d2)]
        .map(|(c, d)| format!("{} = {}", show::combination(&[(&c[i], names[i]), (&c[j], names[j])]), show::value(d)));
    working.add(
        Standard,
        msg!("core.geometry.relations.common_point", names[m]),
        format!("\\begin{{aligned}} {} \\\\ {} \\end{{aligned}}", equations[0], equations[1]),
    );
    let determinant = a[i].mul(&b[j]).sub(&a[j].mul(&b[i]));
    let mut coords = [Value::zero(), Value::zero(), Value::zero()];
    coords[i] = d1.mul(&b[j]).sub(&a[j].mul(d2)).div(&determinant)?;
    coords[j] = a[i].mul(d2).sub(&d1.mul(&b[i])).div(&determinant)?;
    let point = Point::new(coords[0].clone(), coords[1].clone(), coords[2].clone());
    working.add(
        Detailed,
        msg!("core.geometry.relations.solved"),
        format!("{} = {}, \\quad {} = {}", names[i], show::value(&point.coords[i]), names[j], show::value(&point.coords[j])),
    );
    working.add(
        Maximal,
        msg!("core.geometry.relations.check_in_both_planes"),
        format!(
            "{} = {}, \\quad {} = {}",
            show::dot(n1, &point.position()),
            show::value(&n1.dot(&point.position())),
            show::dot(n2, &point.position()),
            show::value(&n2.dot(&point.position()))
        ),
    );
    let line = Line::new(point, direction)?;
    working.add(Brief, msg!("core.geometry.relations.planes_meet_line"), line.equation());
    Ok(line)
}
