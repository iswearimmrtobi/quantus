//! Lines (Geraden) in parameter form `\vec{x} = \vec{p} + \lambda \vec{u}`.

use crate::num::Value;
use crate::settings::StepDetail::{Brief, Detailed, Maximal, Standard};
use crate::text::msg;
use crate::{QuantusError, Result};

use super::show;
use super::working::{Worked, Working};
use super::{equal, Point, Vector3};

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
}
