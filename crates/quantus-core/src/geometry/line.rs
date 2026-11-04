//! Lines (Geraden) in parameter form `\vec{x} = \vec{p} + \lambda \vec{u}`.

use crate::num::Value;
use crate::settings::StepDetail::{Brief, Detailed, Maximal, Standard};
use crate::text::msg;
use crate::{QuantusError, Result};

use super::show;
use super::working::{Worked, Working};
use super::{equal, Angle, Point, Vector3};
use crate::settings::AngleUnit;

/// How two lines lie to each other (Lagebeziehung).
#[derive(Clone, Debug, PartialEq)]
pub enum LineRelation {
    Identical,
    Parallel { distance: Value },
    /// They meet in `point`, at the parameters `lambda` of the first and `mu` of the second line.
    Intersecting { point: Point, lambda: Value, mu: Value, angle: Angle },
    /// Neither parallel nor meeting (windschief), only in 3D.
    Skew { distance: Value, angle: Angle },
}

/// The acute angle between two directions, `\cos\varphi = \frac{|\vec{u} \cdot \vec{v}|}{|\vec{u}| \cdot |\vec{v}|}`.
pub(crate) fn acute_angle(u: &Vector3, v: &Vector3, unit: AngleUnit, precision: usize, working: &mut Working) -> Result<Angle> {
    let flipped = u.dot(v).is_negative()?;
    let angle = if flipped { u.angle(&-v, unit, precision)? } else { u.angle(v, unit, precision)? };
    working.add(
        Standard,
        msg!("The angle between the directions:", "Der Winkel zwischen den Richtungsvektoren:"),
        format!(
            "\\cos\\varphi = \\frac{{\\left|\\vec{{u}} \\cdot \\vec{{v}}\\right|}}{{\\left|\\vec{{u}}\\right| \\cdot \\left|\\vec{{v}}\\right|}} = \\frac{{{}}}{{{} \\cdot {}}} = {}",
            show::value(&super::abs(u.dot(v))?),
            show::value(&u.length()?),
            show::value(&v.length()?),
            show::value(&angle.ratio)
        ),
    );
    working.add(Brief, msg!("The angle:", "Der Winkel:"), show::angle(&angle));
    Ok(angle)
}

/// Whether `u` is a multiple of `v` (both not zero), with the factor.
pub(crate) fn multiple(u: &Vector3, v: &Vector3) -> Option<Value> {
    if !u.is_collinear(v) {
        return None;
    }
    let i = (0..3).find(|&i| !v.coords[i].is_zero())?;
    u.coords[i].div(&v.coords[i]).ok()
}

/// Compares the directions of two lines (or normals of two planes) in the working: whether
/// `\vec{u} = k \vec{v}`.
pub(crate) fn compare_directions(u: &Vector3, v: &Vector3, working: &mut Working) -> Option<Value> {
    let k = multiple(u, v);
    match &k {
        Some(k) => working.add(
            Standard,
            msg!("The vectors are multiples of each other:", "Die Vektoren sind Vielfache voneinander:"),
            format!("{} = {} \\cdot {}", show::vector(u), show::value(k), show::vector(v)),
        ),
        None => working.add(
            Standard,
            msg!("The vectors are not multiples of each other:", "Die Vektoren sind keine Vielfachen voneinander:"),
            format!("{} \\neq k \\cdot {}", show::vector(u), show::vector(v)),
        ),
    }
    k
}

/// A line through `support` (Stützvektor) along `direction` (Richtungsvektor, never zero).
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub support: Point,
    pub direction: Vector3,
}

impl Line {
    /// The line through `support` along `direction`; the zero vector is no direction.
    pub fn new(support: Point, direction: Vector3) -> Result<Line> {
        if direction.is_zero() {
            return Err(QuantusError::other(msg!(
                "The zero vector is no direction of a line.",
                "Der Nullvektor ist kein Richtungsvektor einer Geraden."
            )));
        }
        Ok(Line { support, direction })
    }

    /// The line through `a` and `b`, with `a` as the support and `\overrightarrow{AB}` as the
    /// direction.
    pub fn through(a: &Point, b: &Point) -> Result<Worked<Line>> {
        if a == b {
            return Err(QuantusError::other(msg!(
                "Two equal points give no line.",
                "Zwei gleiche Punkte legen keine Gerade fest."
            )));
        }
        let direction = a.to(b);
        let mut working = Working::default();
        working.add(
            Standard,
            msg!("The direction is the vector from A to B.", "Der Richtungsvektor ist der Vektor von A nach B."),
            format!(
                "\\vec{{u}} = \\overrightarrow{{AB}} = {} - {} = {}",
                show::vector(&b.position()),
                show::vector(&a.position()),
                show::vector(&direction)
            ),
        );
        let line = Line::new(a.clone(), direction)?;
        working.add(Brief, msg!("The line through A and B:", "Die Gerade durch A und B:"), line.equation());
        Ok(Worked::new(line, working))
    }

    /// Whether the line was entered in 2D.
    pub fn planar(&self) -> bool {
        self.support.planar && self.direction.planar
    }

    /// The point at the parameter `lambda`.
    pub fn at(&self, lambda: &Value) -> Point {
        &self.support + &self.direction.scale(lambda)
    }

    /// The parameter form, `\vec{x} = \begin{pmatrix} … \end{pmatrix} + \lambda \begin{pmatrix} … \end{pmatrix}`.
    pub fn equation(&self) -> String {
        self.equation_in("\\lambda")
    }

    /// The parameter form with another parameter, `\mu` for a second line.
    pub fn equation_in(&self, parameter: &str) -> String {
        format!("\\vec{{x}} = {} + {parameter} {}", show::vector(&self.support.position()), show::vector(&self.direction))
    }

    /// Whether `p` lies on the line (Punktprobe): the parameter of `p` if it does.
    pub fn contains(&self, p: &Point) -> Worked<Option<Value>> {
        let mut working = Working::default();
        working.add(
            Standard,
            msg!("Set the point equal to the line equation.", "Den Punkt mit der Geradengleichung gleichsetzen."),
            format!(
                "{} = {} + \\lambda {}",
                show::vector(&p.position()),
                show::vector(&self.support.position()),
                show::vector(&self.direction)
            ),
        );
        let offset = self.support.to(p);
        let rows = if self.planar() && p.planar { 2 } else { 3 };
        // The parameter from the first row with a direction entry; the other rows must agree.
        let pivot = (0..3).find(|&i| !self.direction.coords[i].is_zero()).expect("a line has a direction");
        let lambda = offset.coords[pivot].div(&self.direction.coords[pivot]).expect("the entry is not zero");
        let mut contradiction = None;
        for i in 0..rows {
            let (d, s, q) = (&self.direction.coords[i], &self.support.coords[i], &p.coords[i]);
            let row = format!("{} = {}", show::value(q), show::combination(&[(s, ""), (d, "\\lambda")]));
            let latex = if d.is_zero() {
                let holds = equal(q, s);
                if !holds && contradiction.is_none() {
                    contradiction = Some(format!("{} \\neq {}", show::value(q), show::value(s)));
                }
                format!("{row} \\quad ({})", if holds { msg!("true", "wahr") } else { msg!("false", "falsch") })
            } else {
                let here = offset.coords[i].div(d).expect("the entry is not zero");
                if !equal(&here, &lambda) && contradiction.is_none() {
                    contradiction = Some(format!("\\lambda = {} \\neq {}", show::value(&lambda), show::value(&here)));
                }
                format!("{row} \\quad \\Rightarrow \\quad \\lambda = {}", show::value(&here))
            };
            working.add(Detailed, msg!("Row {}:", "Zeile {}:", i + 1), latex);
        }
        match contradiction {
            None => {
                working.add(
                    Brief,
                    msg!("The point lies on the line.", "Der Punkt liegt auf der Geraden."),
                    format!("\\lambda = {}", show::value(&lambda)),
                );
                Worked::new(Some(lambda), working)
            }
            Some(latex) => {
                working.add(
                    Brief,
                    msg!("The rows contradict each other: the point does not lie on the line.", "Die Zeilen widersprechen sich: Der Punkt liegt nicht auf der Geraden."),
                    latex,
                );
                Worked::new(None, working)
            }
        }
    }

    /// The foot of the perpendicular (Lotfußpunkt) from `p` onto the line: the point `F` of the
    /// line with `\overrightarrow{PF} \perp \vec{u}`.
    pub fn foot(&self, p: &Point) -> Worked<Point> {
        let mut working = Working::default();
        let u = &self.direction;
        working.add(
            Standard,
            msg!("A general point of the line:", "Ein allgemeiner Punkt der Geraden:"),
            format!("F_\\lambda = {} + \\lambda {}", show::vector(&self.support.position()), show::vector(u)),
        );
        let offset = p.to(&self.support);
        working.add(
            Standard,
            msg!(
                "The vector from P to it is perpendicular to the direction:",
                "Der Vektor von P dorthin steht senkrecht auf dem Richtungsvektor:"
            ),
            format!(
                "\\overrightarrow{{PF_\\lambda}} \\cdot \\vec{{u}} = \\left({} + \\lambda {}\\right) \\cdot {} = 0",
                show::vector(&offset),
                show::vector(u),
                show::vector(u)
            ),
        );
        let (constant, coefficient) = (offset.dot(u), u.dot(u));
        let lambda = constant.neg().div(&coefficient).expect("a direction is not zero");
        working.add(
            Maximal,
            msg!("Both dot products row by row:", "Beide Skalarprodukte zeilenweise:"),
            format!("{} + \\lambda \\left({}\\right) = 0", show::dot(&offset, u), show::dot(u, u)),
        );
        working.add(
            Detailed,
            msg!("Multiply out and solve for λ:", "Ausmultiplizieren und nach λ auflösen:"),
            format!(
                "{} = 0 \\quad \\Rightarrow \\quad \\lambda = {}",
                show::combination(&[(&constant, ""), (&coefficient, "\\lambda")]),
                show::value(&lambda)
            ),
        );
        let foot = self.at(&lambda);
        let foot = Point { planar: foot.planar && p.planar, ..foot };
        working.add(
            Brief,
            msg!("The foot of the perpendicular:", "Der Lotfußpunkt:"),
            format!("F = {}", show::point(&foot)),
        );
        Worked::new(foot, working)
    }

    /// The distance from `p` to the line: the length of `\overrightarrow{PF}` to the foot of the
    /// perpendicular.
    pub fn distance(&self, p: &Point) -> Result<Worked<Value>> {
        let Worked { result: foot, mut working } = self.foot(p);
        let pf = p.to(&foot);
        let distance = pf.length()?;
        working.add(
            Standard,
            msg!("The distance is the length of the vector from P to F:", "Der Abstand ist die Länge des Vektors von P nach F:"),
            format!("d = \\left|\\overrightarrow{{PF}}\\right| = \\left|{}\\right| = {}", show::vector(&pf), show::length(&pf)),
        );
        working.add(Brief, msg!("The distance:", "Der Abstand:"), format!("d = {}", show::value(&distance)));
        Ok(Worked::new(distance, working))
    }

    /// How the line lies to `other`: identical, parallel (with the distance), intersecting (with
    /// the point and the angle) or skew (with the distance and the angle).
    pub fn relation(&self, other: &Line, unit: AngleUnit, precision: usize) -> Result<Worked<LineRelation>> {
        let mut working = Working::default();
        let (u, v) = (&self.direction, &other.direction);
        if compare_directions(u, v, &mut working).is_some() {
            let Worked { result: on, working: test } = self.contains(&other.support);
            working.extend(test);
            if on.is_some() {
                working.add(Brief, msg!("The lines are identical.", "Die Geraden sind identisch."), "g = h");
                return Ok(Worked::new(LineRelation::Identical, working));
            }
            let Worked { result: distance, working: part } = self.distance(&other.support)?;
            working.extend(part);
            working.add(Brief, msg!("The lines are parallel.", "Die Geraden sind parallel."), "g \\parallel h");
            return Ok(Worked::new(LineRelation::Parallel { distance }, working));
        }
        working.add(
            Standard,
            msg!("Set the line equations equal:", "Die Geradengleichungen gleichsetzen:"),
            format!(
                "{} + \\lambda {} = {} + \\mu {}",
                show::vector(&self.support.position()),
                show::vector(u),
                show::vector(&other.support.position()),
                show::vector(v)
            ),
        );
        let w = self.support.to(&other.support);
        let rows = if self.planar() && other.planar() { 2 } else { 3 };
        let negated = -v;
        let row = |i: usize| format!("{} = {}", show::combination(&[(&u.coords[i], "\\lambda"), (&negated.coords[i], "\\mu")]), show::value(&w.coords[i]));
        let system: Vec<String> = (0..rows).map(row).collect();
        working.add(
            Detailed,
            msg!("As a system of equations:", "Als Gleichungssystem:"),
            format!("\\begin{{aligned}} {} \\end{{aligned}}", system.join(" \\\\ ")),
        );
        // Two rows with a non-zero determinant fix λ and μ (Cramer's rule); the third must agree.
        let determinant = |i: usize, j: usize| v.coords[i].mul(&u.coords[j]).sub(&u.coords[i].mul(&v.coords[j]));
        let (i, j) = [(0, 1), (0, 2), (1, 2)]
            .into_iter()
            .find(|&(i, j)| !equal(&determinant(i, j), &Value::zero()))
            .expect("the directions are not collinear");
        let d = determinant(i, j);
        let lambda = v.coords[i].mul(&w.coords[j]).sub(&w.coords[i].mul(&v.coords[j])).div(&d)?;
        let mu = u.coords[i].mul(&w.coords[j]).sub(&w.coords[i].mul(&u.coords[j])).div(&d)?;
        working.add(
            Standard,
            msg!("Rows {} and {} give:", "Aus den Zeilen {} und {}:", i + 1, j + 1),
            format!("\\lambda = {}, \\quad \\mu = {}", show::value(&lambda), show::value(&mu)),
        );
        let k = 3 - i - j;
        let fits = k >= rows || equal(&u.coords[k].mul(&lambda).sub(&v.coords[k].mul(&mu)), &w.coords[k]);
        if k < rows {
            let left = u.coords[k].mul(&lambda).sub(&v.coords[k].mul(&mu));
            working.add(
                Standard,
                msg!("Check in row {}:", "Probe in Zeile {}:", k + 1),
                format!(
                    "{} \\cdot {} - {} \\cdot {} = {} {} {}",
                    show::factor(&u.coords[k]),
                    show::factor(&lambda),
                    show::factor(&v.coords[k]),
                    show::factor(&mu),
                    show::value(&left),
                    if fits { "=" } else { "\\neq" },
                    show::value(&w.coords[k])
                ),
            );
        }
        if fits {
            let point = self.at(&lambda);
            working.add(
                Brief,
                msg!("The lines intersect in the point", "Die Geraden schneiden sich im Punkt"),
                format!("S = {}", show::point(&point)),
            );
            let angle = acute_angle(u, v, unit, precision, &mut working)?;
            return Ok(Worked::new(LineRelation::Intersecting { point, lambda, mu, angle }, working));
        }
        working.add(
            Brief,
            msg!("The system has no solution: the lines are skew.", "Das Gleichungssystem hat keine Lösung: Die Geraden sind windschief."),
            "g \\cap h = \\emptyset",
        );
        // The distance along the common perpendicular n = u × v.
        let n = u.cross(v);
        let distance = super::over_length(&super::abs(w.dot(&n))?, &n)?;
        working.add(
            Standard,
            msg!("A vector perpendicular to both directions:", "Ein Vektor senkrecht zu beiden Richtungen:"),
            format!("\\vec{{n}} = \\vec{{u}} \\times \\vec{{v}} = {}", show::vector(&n)),
        );
        working.add(
            Standard,
            msg!("The distance of the skew lines:", "Der Abstand der windschiefen Geraden:"),
            format!(
                "d = \\frac{{\\left|\\overrightarrow{{PQ}} \\cdot \\vec{{n}}\\right|}}{{\\left|\\vec{{n}}\\right|}} = \\frac{{\\left|{} \\cdot {}\\right|}}{{{}}} = {}",
                show::vector(&w),
                show::vector(&n),
                show::value(&n.length()?),
                show::value(&distance)
            ),
        );
        working.add(Brief, msg!("The distance:", "Der Abstand:"), format!("d = {}", show::value(&distance)));
        let angle = acute_angle(u, v, unit, precision, &mut working)?;
        Ok(Worked::new(LineRelation::Skew { distance, angle }, working))
    }
}
