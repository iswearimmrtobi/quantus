//! The curve sketch: one section per question a student answers about f, each with its working
//! and a result line, plus the points and asymptotes to draw.

use std::cmp::Ordering;

use dashu::base::{Abs, UnsignedAbs};
use dashu::integer::{IBig, UBig};
use dashu::rational::RBig;

use super::budget::Budget;
use super::derive::{derive_chain, derivative_name, Derivation};
use super::domain::{domain, sign_at, DomainContext};
use super::exact::times_pi;
use super::expr::{Expr, Fn};
use super::integrate::{antiderivative, IntegrateContext};
use super::interval::{simplest_between, Bound, Domain, Interval, PeriodicSet, Point};
use super::limits::{behaviour, oblique, pole, Approach, Behaviour, LimitContext};
use super::rules::{step, AnalysisRule};
use super::simplify::{display, is_zero, is_zero_numeric, simplify};
use super::solve::{solve_zero, Root, SolveContext, Solutions};
use crate::eval::Step;
use crate::settings::{Language, Settings};
use crate::text::msg;
use crate::Result;

/// Most members of a periodic family listed and drawn in the window.
const MAX_WINDOW_POINTS: usize = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SectionStatus {
    Ok,
    /// Some of it was found numerically in a window or could not be decided.
    Partial,
    Failed,
}

impl SectionStatus {
    pub fn name(self) -> &'static str {
        match self {
            SectionStatus::Ok => "ok",
            SectionStatus::Partial => "partial",
            SectionStatus::Failed => "failed",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Section {
    pub id: &'static str,
    pub title: String,
    pub status: SectionStatus,
    pub message: Option<String>,
    pub steps: Vec<Step>,
    pub result: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointKind {
    Zero,
    YIntercept,
    Max,
    Min,
    Inflection,
    Saddle,
    Hole,
    BoundaryMax,
    BoundaryMin,
}

impl PointKind {
    pub fn name(self) -> &'static str {
        match self {
            PointKind::Zero => "zero",
            PointKind::YIntercept => "yIntercept",
            PointKind::Max => "max",
            PointKind::Min => "min",
            PointKind::Inflection => "inflection",
            PointKind::Saddle => "saddle",
            PointKind::Hole => "hole",
            PointKind::BoundaryMax => "boundaryMax",
            PointKind::BoundaryMin => "boundaryMin",
        }
    }

    /// The letter the point is named with: H, T, W, … in German.
    fn letter(self) -> &'static str {
        match (self, crate::text::language()) {
            (PointKind::Zero, Language::De) => "N",
            (PointKind::Zero, Language::En) => "Z",
            (PointKind::YIntercept, _) => "S_y",
            (PointKind::Max, Language::De) => "H",
            (PointKind::Max, Language::En) => "Max",
            (PointKind::Min, Language::De) => "T",
            (PointKind::Min, Language::En) => "Min",
            (PointKind::Inflection, Language::De) => "W",
            (PointKind::Inflection, Language::En) => "I",
            (PointKind::Saddle, _) => "S",
            (PointKind::Hole, Language::De) => "L",
            (PointKind::Hole, Language::En) => "G",
            (PointKind::BoundaryMax, Language::De) => "R",
            (PointKind::BoundaryMax, Language::En) => "B",
            (PointKind::BoundaryMin, Language::De) => "R",
            (PointKind::BoundaryMin, Language::En) => "B",
        }
    }
}

/// A point to mark in the graph.
#[derive(Clone, Debug)]
pub struct MarkedPoint {
    pub kind: PointKind,
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub latex: String,
    /// Stands for the name in texts written before the points are named.
    token: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    PlusInfinity,
    MinusInfinity,
    Both,
}

impl Side {
    pub fn name(self) -> &'static str {
        match self {
            Side::PlusInfinity => "plusInfinity",
            Side::MinusInfinity => "minusInfinity",
            Side::Both => "both",
        }
    }
}

#[derive(Clone, Debug)]
pub enum Asymptote {
    Vertical { x: f64, latex: String },
    Horizontal { y: f64, side: Side, latex: String },
    Oblique { slope: f64, intercept: f64, side: Side, latex: String },
}

#[derive(Debug)]
pub struct Discussion {
    /// `f(x) = …`
    pub function: String,
    pub variable: String,
    pub sections: Vec<Section>,
    pub points: Vec<MarkedPoint>,
    pub asymptotes: Vec<Asymptote>,
    /// The period in LaTeX, for a periodic f.
    pub period: Option<String>,
    pub f: Expr,
    pub domain: Domain,
    /// Poles and other gaps inside the window, where the graph breaks.
    pub breaks: Vec<f64>,
    /// Where periodic families are listed.
    pub window: (f64, f64),
}

pub struct Ctx<'a> {
    pub settings: &'a Settings,
    pub var: &'a str,
    pub digits: usize,
    pub budget: Budget,
}

/// A candidate x for an extremum or inflection point, with the family it belongs to if any.
struct Candidate {
    point: Point,
    /// `b + k·P` written generally, when the candidate stands for a whole family.
    family: Option<PeriodicSet>,
}

/// The whole curve sketch of `f` (the tree as typed). Fails only when the domain is empty.
pub fn discuss(f: &Expr, ctx: &Ctx) -> Result<Discussion> {
    let (dom, domain_steps) = domain(f, &DomainContext { settings: ctx.settings, var: ctx.var, digits: ctx.digits })?;
    let var = ctx.var;
    let tex = |e: &Expr| e.to_latex(var);
    let period = period_of(&simplify(f));
    let window = match &period {
        Some(p) => {
            let p = p.to_f64().value() * std::f64::consts::PI;
            (-p, p)
        }
        None => (-10.0, 10.0),
    };
    let mut sketch = Sketch {
        ctx,
        f: simplify(f),
        domain: dom.clone(),
        window,
        period: period.clone(),
        points: Vec::new(),
        asymptotes: Vec::new(),
        breaks: Vec::new(),
    };

    let derivatives = derive_chain(f, 3, var, ctx.settings);
    let mut sections = vec![sketch.derivatives(&derivatives)];
    sections.push(Section {
        id: "domain",
        title: msg!("Domain", "Definitionsbereich"),
        status: SectionStatus::Ok,
        message: None,
        result: domain_steps.last().map(|s| s.latex.clone()),
        steps: domain_steps,
    });
    type Builder<'b> = Box<dyn std::ops::Fn(&mut Sketch, &[Derivation]) -> Section + 'b>;
    let builders: Vec<(&'static str, String, Builder)> = vec![
        ("symmetry", msg!("Symmetry", "Symmetrie"), Box::new(|s: &mut Sketch, _: &[Derivation]| s.symmetry())),
        ("yIntercept", msg!("y-intercept", "Schnittpunkt mit der y-Achse"), Box::new(|s: &mut Sketch, _: &[Derivation]| s.y_intercept())),
        ("zeros", msg!("Zeros", "Nullstellen"), Box::new(|s: &mut Sketch, _: &[Derivation]| s.zeros())),
        ("extrema", msg!("Extrema", "Extrempunkte"), Box::new(|s: &mut Sketch, d: &[Derivation]| s.extrema(d))),
        ("inflection", msg!("Inflection points", "Wendepunkte"), Box::new(|s: &mut Sketch, d: &[Derivation]| s.inflection(d))),
        ("monotonicity", msg!("Monotonicity", "Monotonie"), Box::new(|s: &mut Sketch, d: &[Derivation]| s.monotonicity(&d[0].result))),
        ("curvature", msg!("Curvature", "Krümmung"), Box::new(|s: &mut Sketch, d: &[Derivation]| s.curvature(&d[1].result))),
        ("behaviour", msg!("Behaviour at the edges and asymptotes", "Verhalten an den Rändern und Asymptoten"), Box::new(|s: &mut Sketch, _: &[Derivation]| s.behaviour())),
        ("antiderivative", msg!("Antiderivative", "Stammfunktion"), Box::new(|s: &mut Sketch, _: &[Derivation]| s.antiderivative())),
    ];
    for (id, title, build) in builders {
        if ctx.budget.exceeded() {
            sections.push(Section {
                id,
                title,
                status: SectionStatus::Partial,
                message: Some(msg!("Time limit reached", "Zeitlimit erreicht")),
                steps: Vec::new(),
                result: None,
            });
            continue;
        }
        let mut section = build(&mut sketch, &derivatives);
        section.title = title;
        sections.push(section);
    }
    let mut points = sketch.points;
    name_points(&mut points);
    // The sections refer to the points by a token until the names are known.
    let tokens: Vec<(String, String)> = points.iter().map(|p| (p.token.clone(), p.name.clone())).collect();
    let rename = |text: &mut String| {
        for (token, name) in &tokens {
            if text.contains(token.as_str()) {
                *text = text.replace(token.as_str(), name);
            }
        }
    };
    for section in &mut sections {
        if let Some(result) = section.result.as_mut() {
            rename(result);
        }
        for s in &mut section.steps {
            rename(&mut s.latex);
        }
    }
    for point in &mut points {
        rename(&mut point.latex);
    }
    Ok(Discussion {
        function: format!("f({var}) = {}", tex(f)),
        variable: var.to_string(),
        sections,
        points,
        asymptotes: sketch.asymptotes,
        period: period.map(times_pi).map(|p| tex(&p)),
        f: sketch.f,
        domain: dom,
        breaks: sketch.breaks,
        window,
    })
}

/// The period of f as a multiple of π, when every x sits in sin, cos or tan of a linear
/// argument with rational slopes.
pub fn period_of(f: &Expr) -> Option<RBig> {
    fn walk(e: &Expr, periods: &mut Vec<RBig>) -> bool {
        match e {
            Expr::Var => false,
            Expr::Num(_) | Expr::Pi | Expr::E => true,
            Expr::Func(function @ (Fn::Sin | Fn::Cos | Fn::Tan), u) if u.depends_on_var() => {
                let slope = simplify(&super::derive::derivative(u));
                let Some(a) = slope.as_num().filter(|a| **a != RBig::ZERO) else { return false };
                let base = if *function == Fn::Tan { RBig::ONE } else { RBig::from(2) };
                periods.push(base / a.clone().abs());
                true
            }
            Expr::Sum(items) | Expr::Product(items) => items.iter().all(|i| walk(i, periods)),
            Expr::Pow(b, x) => walk(b, periods) && walk(x, periods),
            Expr::Func(_, u) => walk(u, periods),
        }
    }
    let mut periods = Vec::new();
    if !walk(f, &mut periods) || periods.is_empty() {
        return None;
    }
    // lcm of p/q: lcm of the numerators over the gcd of the denominators.
    let mut numerator = UBig::ONE;
    let mut denominator = UBig::ZERO;
    for p in &periods {
        let n = p.numerator().clone().unsigned_abs();
        numerator = crate::algebra::poly::lcm_ubig(&numerator, &n);
        denominator = crate::algebra::poly::gcd_ubig(denominator, p.denominator().clone());
    }
    Some(RBig::from_parts(IBig::from(numerator), denominator))
}

/// Names the points by kind, numbered where a kind occurs more than once: `H_{1}`, `H_{2}`.
fn name_points(points: &mut [MarkedPoint]) {
    points.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(Ordering::Equal));
    let kinds = [
        PointKind::Zero,
        PointKind::YIntercept,
        PointKind::Max,
        PointKind::Min,
        PointKind::Inflection,
        PointKind::Saddle,
        PointKind::Hole,
        PointKind::BoundaryMax,
        PointKind::BoundaryMin,
    ];
    for kind in kinds {
        let count = points.iter().filter(|p| p.kind == kind).count();
        let mut i = 0;
        for point in points.iter_mut().filter(|p| p.kind == kind) {
            i += 1;
            let letter = kind.letter();
            point.name = if count == 1 { letter.to_string() } else { format!("{letter}_{{{i}}}") };
        }
    }
}

struct Sketch<'a> {
    ctx: &'a Ctx<'a>,
    f: Expr,
    domain: Domain,
    window: (f64, f64),
    period: Option<RBig>,
    points: Vec<MarkedPoint>,
    asymptotes: Vec<Asymptote>,
    breaks: Vec<f64>,
}

impl Sketch<'_> {
    fn tex(&self, e: &Expr) -> String {
        e.to_latex(self.ctx.var)
    }

    fn solve(&self, e: &Expr) -> super::solve::Solved {
        let ctx = SolveContext { settings: self.ctx.settings, var: self.ctx.var, domain: &self.domain, window: self.window, digits: self.ctx.digits };
        solve_zero(e, &ctx)
    }

    fn section(&self, id: &'static str, steps: Vec<Step>, result: Option<String>, complete: bool) -> Section {
        let (status, message) = if complete {
            (SectionStatus::Ok, None)
        } else {
            (
                SectionStatus::Partial,
                Some(msg!(
                    "Found numerically in a window; there may be more outside it.",
                    "Numerisch in einem Bereich gefunden; außerhalb kann es weitere geben."
                )),
            )
        };
        Section { id, title: String::new(), status, message, steps, result }
    }

    /// The value of f at an exact or numeric point.
    fn value_at(&self, e: &Expr, x: &Point) -> Option<Point> {
        match &x.exact {
            Some(exact) => Point::exact(simplify(&e.substitute(exact))),
            None => {
                let v = e.eval_f64(x.approx)?;
                Some(Point { exact: None, approx: v, refined: None })
            }
        }
    }

    /// `(x \mid y)`, followed by a decimal version when the exact one is not plain.
    fn coordinates(&self, x: &Point, y: &Point) -> String {
        let d = self.ctx.digits;
        let exact = format!("\\left({} \\mid {}\\right)", x.to_latex(d), y.to_latex(d));
        let plain = (x.exact.is_none() || x.is_rational()) && (y.exact.is_none() || y.is_rational());
        let numeric = x.exact.is_none() || y.exact.is_none();
        if plain && numeric {
            format!("\\left({} \\mid {}\\right)", approx_text(x, d), approx_text(y, d))
        } else if plain {
            exact
        } else {
            format!("{exact} \\approx \\left({} \\mid {}\\right)", x.decimal_latex(d), y.decimal_latex(d))
        }
    }

    /// Marks a point and returns `token(x | y)`; the token becomes the point's name.
    fn mark(&mut self, kind: PointKind, x: &Point, y: &Point) -> String {
        let token = format!("@{}@", self.points.len());
        let latex = format!("{token}{}", self.coordinates(x, y));
        self.points.push(MarkedPoint { kind, name: String::new(), x: x.approx, y: y.approx, latex: latex.clone(), token });
        latex
    }

    // ---- sections ------------------------------------------------------------------------------

    fn derivatives(&self, derivatives: &[Derivation]) -> Section {
        let mut steps = Vec::new();
        let mut lines = Vec::new();
        for (i, d) in derivatives.iter().enumerate() {
            steps.extend(d.steps.iter().cloned());
            lines.push(format!("{}({}) = {}", derivative_name(i + 1), self.ctx.var, self.tex(&d.result)));
        }
        let mut section = self.section("derivatives", steps, Some(lines.join(", \\quad ")), true);
        section.title = msg!("Derivatives", "Ableitungen");
        section
    }

    fn symmetry(&mut self) -> Section {
        let var = self.ctx.var;
        let mut steps = Vec::new();
        let symmetric_domain = self.domain.set.is_symmetric()
            && self.domain.excluded.iter().all(|family| family.base.iter().all(|b| b.eval_f64(0.0).is_some_and(|b| family.contains(-b))));
        if !symmetric_domain {
            let result = msg!("No symmetry: D is not symmetric about 0", "Keine Symmetrie: D ist nicht symmetrisch zu 0");
            steps.push(step(AnalysisRule::Symmetry, result.clone(), format!("D = {}", self.domain.to_latex(self.ctx.digits, self.ctx.settings.language))));
            return self.section("symmetry", steps, Some(text(&result)), true);
        }
        let mirrored = simplify(&self.f.substitute(&Expr::neg(Expr::Var)));
        steps.push(step(
            AnalysisRule::Symmetry,
            msg!("Replace x by -x", "x durch -x ersetzen"),
            format!("f(-{var}) = {}", self.tex(&display(&mirrored, self.ctx.settings))),
        ));
        let even = Expr::Sum(vec![mirrored.clone(), Expr::neg(self.f.clone())]);
        let odd = Expr::Sum(vec![mirrored, self.f.clone()]);
        let points: Vec<f64> = self.domain.sample_points(9).into_iter().filter(|x| self.domain.contains(-x)).collect();
        let (result, checked) = if is_zero(&even) {
            (msg!("Symmetric about the y-axis", "Achsensymmetrisch zur y-Achse"), false)
        } else if is_zero(&odd) {
            (msg!("Symmetric about the origin", "Punktsymmetrisch zum Ursprung"), false)
        } else if is_zero_numeric(&simplify(&even), &points) {
            (msg!("Symmetric about the y-axis", "Achsensymmetrisch zur y-Achse"), true)
        } else if is_zero_numeric(&simplify(&odd), &points) {
            (msg!("Symmetric about the origin", "Punktsymmetrisch zum Ursprung"), true)
        } else {
            (msg!("No simple symmetry", "Keine einfache Symmetrie"), false)
        };
        let relation = if result.contains("y-") {
            format!("f(-{var}) = f({var})")
        } else if result.starts_with("No") || result.starts_with("Keine") {
            format!("f(-{var}) \\neq \\pm f({var})")
        } else {
            format!("f(-{var}) = -f({var})")
        };
        let result = if checked { format!("{result} {}", msg!("(checked numerically)", "(numerisch geprüft)")) } else { result };
        steps.push(step(AnalysisRule::Symmetry, result.clone(), relation));
        self.section("symmetry", steps, Some(text(&result)), true)
    }

    fn y_intercept(&mut self) -> Section {
        let var = self.ctx.var;
        if !self.domain.contains(0.0) {
            let result = msg!("0 is not in D: no y-intercept", "0 ∉ D: kein Schnittpunkt mit der y-Achse");
            return self.section("yIntercept", Vec::new(), Some(text(&result)), true);
        }
        let zero = Point::rational(RBig::ZERO);
        let Some(y) = self.value_at(&self.f, &zero) else {
            return self.section("yIntercept", Vec::new(), None, false);
        };
        let steps = vec![step(AnalysisRule::Simplify, msg!("Put in x = 0", "x = 0 einsetzen"), format!("f(0) = {}", y.to_latex(self.ctx.digits)))];
        let point = self.mark(PointKind::YIntercept, &zero, &y);
        let _ = var;
        self.section("yIntercept", steps, Some(point), true)
    }

    fn zeros(&mut self) -> Section {
        let solved = self.solve(&self.f);
        let mut steps = vec![step(AnalysisRule::ZeroProduct, msg!("Set f(x) = 0", "f(x) = 0 setzen"), format!("{} = 0", self.tex(&display(&self.f, self.ctx.settings))))];
        steps.extend(solved.steps);
        if solved.solutions.all {
            let result = msg!("f is 0 everywhere", "f ist überall 0");
            return self.section("zeros", steps, Some(text(&result)), true);
        }
        let mut lines = Vec::new();
        for root in self.members(&solved.solutions) {
            let zero = Point::rational(RBig::ZERO);
            let coordinates = self.mark(PointKind::Zero, &root.point, &zero);
            let note = match root.multiplicity {
                Some(2) => format!(" \\ \\text{{({})}}", msg!("double root: the graph touches the axis", "doppelte Nullstelle: Berührpunkt")),
                Some(m) if m >= 3 && m % 2 == 1 => format!(" \\ \\text{{({})}}", msg!("multiple root with a sign change", "mehrfache Nullstelle mit Vorzeichenwechsel")),
                _ => String::new(),
            };
            lines.push(format!("{coordinates}{note}"));
        }
        for family in &solved.solutions.families {
            for general in family.to_latex() {
                lines.push(format!("{} = {general}, \\ k \\in \\mathbb{{Z}}", self.ctx.var));
            }
        }
        let result = if lines.is_empty() { text(&msg!("No zeros", "Keine Nullstellen")) } else { lines.join(", \\quad ") };
        self.section("zeros", steps, Some(result), solved.complete)
    }

    /// The roots, plus the members of each family inside the window.
    fn members(&self, solutions: &Solutions) -> Vec<Root> {
        let mut out = solutions.roots.clone();
        for family in &solutions.families {
            for point in family.points_in(self.window.0, self.window.1, MAX_WINDOW_POINTS) {
                if self.domain.contains(point.approx) {
                    out.push(Root { point, multiplicity: Some(1) });
                }
            }
        }
        out.sort_by(|a, b| a.point.approx.partial_cmp(&b.point.approx).unwrap_or(Ordering::Equal));
        out
    }

    /// Where `g` is zero, plus the points of D where it is undefined; each family split to the
    /// period of f with a representative in [0, P), or listed in the window when f has none.
    fn candidates(&self, g: &Expr, steps: &mut Vec<Step>, complete: &mut bool) -> Vec<Candidate> {
        let solved = self.solve(g);
        steps.extend(solved.steps);
        *complete &= solved.complete;
        let mut out: Vec<Candidate> = solved.solutions.roots.into_iter().map(|r| Candidate { point: r.point, family: None }).collect();
        for family in solved.solutions.families {
            match &self.period {
                Some(p) => {
                    for split in split_family(&family, &times_pi(p.clone())) {
                        if let Some(point) = Point::exact(split.base[0].clone()) {
                            out.push(Candidate { point, family: Some(split) });
                        }
                    }
                }
                None => {
                    for point in family.points_in(self.window.0, self.window.1, MAX_WINDOW_POINTS) {
                        out.push(Candidate { point, family: None });
                    }
                }
            }
        }
        // Points of D where g is undefined (a kink of f for g = f′).
        if let Ok((own, _)) = domain(g, &DomainContext { settings: self.ctx.settings, var: self.ctx.var, digits: self.ctx.digits }) {
            for interval in &own.set.0 {
                for bound in [&interval.lo, &interval.hi] {
                    if let Bound::At { point, .. } = bound {
                        let interior = self.domain.contains(point.approx) && !self.is_domain_end(point);
                        if interior && !out.iter().any(|c| c.point.same(point)) {
                            out.push(Candidate { point: point.clone(), family: None });
                        }
                    }
                }
            }
        }
        out.sort_by(|a, b| a.point.approx.partial_cmp(&b.point.approx).unwrap_or(Ordering::Equal));
        out
    }

    fn is_domain_end(&self, p: &Point) -> bool {
        self.domain.set.0.iter().any(|i| {
            [&i.lo, &i.hi].iter().any(|b| matches!(b, Bound::At { point, .. } if point.same(p)))
        })
    }

    /// The x where the sign of `g` must be tested around `x0`: the nearest candidates, gaps and
    /// domain bounds on either side.
    fn neighbours(&self, x0: f64, others: &[f64]) -> (f64, f64) {
        let mut stops: Vec<f64> = others.to_vec();
        for interval in &self.domain.set.0 {
            stops.push(interval.lo.value());
            stops.push(interval.hi.value());
        }
        for family in &self.domain.excluded {
            stops.extend(family.points_in(x0 - 10.0, x0 + 10.0, 40).iter().map(|p| p.approx));
        }
        let left = stops.iter().copied().filter(|s| *s < x0 - 1e-12).fold(f64::NEG_INFINITY, f64::max);
        let right = stops.iter().copied().filter(|s| *s > x0 + 1e-12).fold(f64::INFINITY, f64::min);
        (left, right)
    }

    /// The signs of `g` just left and right of `x0`.
    fn sign_change(&self, g: &Expr, x0: f64, others: &[f64]) -> (i32, i32) {
        let (left, right) = self.neighbours(x0, others);
        let l = simplest_between(left.max(x0 - 1.0), x0);
        let r = simplest_between(x0, right.min(x0 + 1.0));
        (sign_at(g, &l), sign_at(g, &r))
    }

    fn general_x(&self, candidate: &Candidate) -> String {
        match &candidate.family {
            Some(family) => family.to_latex()[0].clone(),
            None => candidate.point.to_latex(self.ctx.digits),
        }
    }

    fn extrema(&mut self, d: &[Derivation]) -> Section {
        let var = self.ctx.var.to_string();
        let (f1, f2) = (&d[0].result, &d[1].result);
        let mut steps = vec![step(
            AnalysisRule::SecondDerivativeTest,
            msg!("Necessary condition", "Notwendige Bedingung"),
            format!("f'({var}) = 0 \\Leftrightarrow {} = 0", self.tex(f1)),
        )];
        if f1.is_constant() {
            let result = if is_zero(f1) { msg!("f is constant", "f ist konstant") } else { msg!("No extrema", "Keine Extrempunkte") };
            return self.section("extrema", steps, Some(text(&result)), true);
        }
        let mut complete = true;
        let candidates = self.candidates(f1, &mut steps, &mut complete);
        let xs: Vec<f64> = candidates.iter().map(|c| c.point.approx).collect();
        let mut lines = Vec::new();
        for candidate in &candidates {
            let x0 = &candidate.point;
            let x_text = self.general_x(candidate);
            let second = if f2.is_constant() || self.domain_of_defined(f2, x0) { self.value_at(f2, x0) } else { None };
            let kind = match second.as_ref().map(|v| (v, v.exact.as_ref().is_some_and(is_zero) || v.approx == 0.0)) {
                Some((v, false)) => {
                    let kind = if v.approx < 0.0 { PointKind::Max } else { PointKind::Min };
                    let relation = if v.approx < 0.0 { "< 0" } else { "> 0" };
                    let name = if kind == PointKind::Max { msg!("maximum", "Hochpunkt") } else { msg!("minimum", "Tiefpunkt") };
                    steps.push(step(
                        AnalysisRule::SecondDerivativeTest,
                        msg!("Sufficient condition: f''({}) {} gives a {}", "Hinreichende Bedingung: f''({}) {} ergibt einen {}", x_text, relation, name),
                        format!("f''({x_text}) = {} {relation}", v.to_latex(self.ctx.digits)),
                    ));
                    Some(kind)
                }
                _ => {
                    let others: Vec<f64> = xs.iter().copied().filter(|x| (x - x0.approx).abs() > 1e-12).collect();
                    let (l, r) = self.sign_change(f1, x0.approx, &others);
                    let sign = |s: i32| if s > 0 { "+" } else if s < 0 { "-" } else { "0" };
                    let kind = match (l, r) {
                        (1, -1) => Some(PointKind::Max),
                        (-1, 1) => Some(PointKind::Min),
                        _ => None,
                    };
                    let verdict = match kind {
                        Some(PointKind::Max) => msg!("sign change + to -: maximum", "Vorzeichenwechsel von + nach -: Hochpunkt"),
                        Some(_) => msg!("sign change - to +: minimum", "Vorzeichenwechsel von - nach +: Tiefpunkt"),
                        None => msg!("no sign change: no extremum", "kein Vorzeichenwechsel: kein Extrempunkt"),
                    };
                    steps.push(step(
                        AnalysisRule::SignChangeTest,
                        msg!("Sign change of f' at {}: {}", "Vorzeichenwechsel von f' bei {}: {}", x_text, verdict),
                        format!("f'({var}) \\colon {} \\to {}", sign(l), sign(r)),
                    ));
                    kind
                }
            };
            let Some(kind) = kind else { continue };
            let Some(y) = self.value_at(&self.f, x0) else { continue };
            let letter = kind.letter();
            match &candidate.family {
                Some(family) => {
                    lines.push(format!("{letter}\\left({} \\mid {}\\right), \\ k \\in \\mathbb{{Z}}", family.to_latex()[0], y.to_latex(self.ctx.digits)));
                    for member in family.points_in(self.window.0, self.window.1, MAX_WINDOW_POINTS) {
                        if let Some(y) = self.value_at(&self.f, &member) {
                            self.mark(kind, &member, &y);
                        }
                    }
                }
                None => lines.push(self.mark(kind, x0, &y)),
            }
        }
        lines.extend(self.boundary_points(f1));
        let result = if lines.is_empty() { text(&msg!("No extrema", "Keine Extrempunkte")) } else { lines.join(", \\quad ") };
        self.section("extrema", steps, Some(result), complete)
    }

    /// Whether `g` is defined at `x` (its own domain, which may be smaller than f's).
    fn domain_of_defined(&self, g: &Expr, x: &Point) -> bool {
        g.eval_f64(x.approx).is_some()
    }

    /// Closed ends of D: a boundary maximum or minimum by the sign of f′ just inside.
    fn boundary_points(&mut self, f1: &Expr) -> Vec<String> {
        let mut out = Vec::new();
        let ends: Vec<(Point, bool)> = self
            .domain
            .set
            .0
            .iter()
            .flat_map(|i| {
                let mut ends = Vec::new();
                if let Bound::At { point, closed: true } = &i.lo {
                    ends.push((point.clone(), true));
                }
                if let Bound::At { point, closed: true } = &i.hi {
                    ends.push((point.clone(), false));
                }
                ends
            })
            .collect();
        for (end, left_end) in ends {
            let inside = if left_end { end.approx + 1e-6 * end.approx.abs().max(1.0) } else { end.approx - 1e-6 * end.approx.abs().max(1.0) };
            let Some(slope) = f1.eval_f64(inside) else { continue };
            let rising = slope > 0.0;
            let kind = if rising == left_end { PointKind::BoundaryMin } else { PointKind::BoundaryMax };
            let Some(y) = self.value_at(&self.f, &end) else { continue };
            let name = if kind == PointKind::BoundaryMin { msg!("boundary minimum", "Randminimum") } else { msg!("boundary maximum", "Randmaximum") };
            let point = self.mark(kind, &end, &y);
            out.push(format!("{point} \\ \\text{{({name})}}"));
        }
        out
    }

    fn inflection(&mut self, d: &[Derivation]) -> Section {
        let var = self.ctx.var.to_string();
        let (f1, f2, f3) = (&d[0].result, &d[1].result, &d[2].result);
        let mut steps = vec![step(
            AnalysisRule::ThirdDerivativeTest,
            msg!("Necessary condition", "Notwendige Bedingung"),
            format!("f''({var}) = 0 \\Leftrightarrow {} = 0", self.tex(f2)),
        )];
        if f2.is_constant() {
            let result = if is_zero(f2) && f1.is_constant() {
                msg!("f is linear: no inflection points", "f ist linear: keine Wendepunkte")
            } else {
                msg!("No inflection points", "Keine Wendepunkte")
            };
            return self.section("inflection", steps, Some(text(&result)), true);
        }
        let mut complete = true;
        let candidates = self.candidates(f2, &mut steps, &mut complete);
        let xs: Vec<f64> = candidates.iter().map(|c| c.point.approx).collect();
        let mut lines = Vec::new();
        for candidate in &candidates {
            let x0 = &candidate.point;
            let x_text = self.general_x(candidate);
            let third = if self.domain_of_defined(f3, x0) { self.value_at(f3, x0) } else { None };
            let inflects = match third.as_ref().filter(|v| !(v.exact.as_ref().is_some_and(is_zero) || v.approx == 0.0)) {
                Some(v) => {
                    steps.push(step(
                        AnalysisRule::ThirdDerivativeTest,
                        msg!("Sufficient condition: f'''({}) ≠ 0", "Hinreichende Bedingung: f'''({}) ≠ 0", x_text),
                        format!("f'''({x_text}) = {} \\neq 0", v.to_latex(self.ctx.digits)),
                    ));
                    true
                }
                None => {
                    let others: Vec<f64> = xs.iter().copied().filter(|x| (x - x0.approx).abs() > 1e-12).collect();
                    let (l, r) = self.sign_change(f2, x0.approx, &others);
                    let changes = l != 0 && r != 0 && l != r;
                    let verdict = if changes { msg!("sign change: inflection point", "Vorzeichenwechsel: Wendepunkt") } else { msg!("no sign change", "kein Vorzeichenwechsel") };
                    let sign = |s: i32| if s > 0 { "+" } else if s < 0 { "-" } else { "0" };
                    steps.push(step(
                        AnalysisRule::SignChangeTest,
                        msg!("Sign change of f'' at {}: {}", "Vorzeichenwechsel von f'' bei {}: {}", x_text, verdict),
                        format!("f''({var}) \\colon {} \\to {}", sign(l), sign(r)),
                    ));
                    changes
                }
            };
            if !inflects {
                continue;
            }
            let slope = self.value_at(f1, x0);
            let saddle = slope.as_ref().is_some_and(|s| s.exact.as_ref().is_some_and(is_zero) || s.approx == 0.0);
            let kind = if saddle { PointKind::Saddle } else { PointKind::Inflection };
            if saddle {
                steps.push(step(
                    AnalysisRule::ThirdDerivativeTest,
                    msg!("f'({}) = 0 as well: a saddle point", "Zusätzlich f'({}) = 0: ein Sattelpunkt (Terrassenpunkt)", x_text),
                    format!("f'({x_text}) = 0"),
                ));
            }
            let Some(y) = self.value_at(&self.f, x0) else { continue };
            let letter = kind.letter();
            match &candidate.family {
                Some(family) => {
                    let y_text = if self.f_is_periodic() { y.to_latex(self.ctx.digits) } else { family.to_latex()[0].clone() };
                    lines.push(format!("{letter}\\left({} \\mid {y_text}\\right), \\ k \\in \\mathbb{{Z}}", family.to_latex()[0]));
                    for member in family.points_in(self.window.0, self.window.1, MAX_WINDOW_POINTS) {
                        if let Some(y) = self.value_at(&self.f, &member) {
                            self.mark(kind, &member, &y);
                        }
                    }
                }
                None => lines.push(self.mark(kind, x0, &y)),
            }
        }
        let result = if lines.is_empty() { text(&msg!("No inflection points", "Keine Wendepunkte")) } else { lines.join(", \\quad ") };
        self.section("inflection", steps, Some(result), complete)
    }

    fn f_is_periodic(&self) -> bool {
        self.period.is_some()
    }

    /// The intervals between the zeros and gaps of `g` inside D (one period for a periodic f),
    /// each with the sign of g at its simplest point.
    fn sign_intervals(&self, g: &Expr) -> (Vec<(Interval, i32)>, bool) {
        let solved = self.solve(g);
        let mut cuts: Vec<Point> = self.members(&solved.solutions).into_iter().map(|r| r.point).collect();
        if let Ok((own, _)) = domain(g, &DomainContext { settings: self.ctx.settings, var: self.ctx.var, digits: self.ctx.digits }) {
            for interval in &own.set.0 {
                for bound in [&interval.lo, &interval.hi] {
                    if let Bound::At { point, .. } = bound {
                        cuts.push(point.clone());
                    }
                }
            }
        }
        let set = match &self.period {
            Some(p) => {
                let period = Point::exact(times_pi(p.clone())).unwrap();
                let one = super::interval::IntervalSet(vec![Interval {
                    lo: Bound::At { point: Point::rational(RBig::ZERO), closed: true },
                    hi: Bound::At { point: period, closed: true },
                }]);
                self.domain.set.intersect(&one)
            }
            None => self.domain.set.clone(),
        };
        for family in &self.domain.excluded {
            cuts.extend(family.points_in(self.window.0.min(0.0) - 1.0, self.window.1 + 1.0, 40));
        }
        let mut out: Vec<(Interval, i32)> = Vec::new();
        for interval in &set.0 {
            let mut inside: Vec<Point> = cuts.iter().filter(|p| interval.contains(p.approx) && !bound_is(&interval.lo, p) && !bound_is(&interval.hi, p)).cloned().collect();
            inside.sort_by(|a, b| a.approx.partial_cmp(&b.approx).unwrap_or(Ordering::Equal));
            inside.dedup_by(|a, b| a.same(b));
            let mut bounds = vec![interval.lo.clone()];
            for p in inside {
                let in_domain = self.domain.contains(p.approx);
                bounds.push(Bound::At { point: p, closed: in_domain });
            }
            bounds.push(interval.hi.clone());
            for pair in bounds.windows(2) {
                let piece = Interval { lo: pair[0].clone(), hi: pair[1].clone() };
                let sign = sign_at(g, &piece.test_point());
                // No sign change at a point of D: one interval (x^3 rises on all of ℝ).
                match out.last_mut() {
                    Some((last, last_sign)) if *last_sign == sign && last.hi.closed() && bound_is(&last.hi, piece_lo(&piece)) => {
                        last.hi = piece.hi.clone();
                    }
                    _ => out.push((piece, sign)),
                }
            }
        }
        (out, solved.complete)
    }

    fn sign_table(&self, g_name: &str, pieces: &[(Interval, i32)], up: &str, down: &str) -> String {
        let language = self.ctx.settings.language;
        let columns = "c|".repeat(pieces.len());
        let header: Vec<String> = pieces.iter().map(|(i, _)| i.to_latex(self.ctx.digits, language)).collect();
        let signs: Vec<&str> = pieces.iter().map(|(_, s)| if *s > 0 { "+" } else if *s < 0 { "-" } else { "0" }).collect();
        let arrows: Vec<&str> = pieces.iter().map(|(_, s)| if *s > 0 { up } else if *s < 0 { down } else { "\\rightarrow" }).collect();
        format!(
            "\\begin{{array}}{{c|{}}} {var} & {} \\\\ \\hline {g_name}({var}) & {} \\\\ f & {} \\end{{array}}",
            columns.trim_end_matches('|'),
            header.join(" & "),
            signs.join(" & "),
            arrows.join(" & "),
            var = self.ctx.var
        )
    }

    fn monotonicity(&mut self, f1: &Expr) -> Section {
        if f1.is_constant() && is_zero(f1) {
            return self.section("monotonicity", Vec::new(), Some(text(&msg!("f is constant", "f ist konstant"))), true);
        }
        let (pieces, complete) = self.sign_intervals(f1);
        let table = self.sign_table("f'", &pieces, "\\nearrow", "\\searrow");
        let mut steps = vec![step(AnalysisRule::SignChangeTest, msg!("Sign of f' between its zeros and gaps", "Vorzeichen von f' zwischen den Nullstellen und Lücken"), table)];
        let language = self.ctx.settings.language;
        let list = |sign: i32| pieces.iter().filter(|(_, s)| *s == sign).map(|(i, _)| i.to_latex(self.ctx.digits, language)).collect::<Vec<_>>();
        let (up, down) = (list(1), list(-1));
        let mut parts = Vec::new();
        if !up.is_empty() {
            parts.push(format!("{}: {}", text(&msg!("strictly increasing on", "streng monoton steigend auf")), up.join(", ")));
        }
        if !down.is_empty() {
            parts.push(format!("{}: {}", text(&msg!("strictly decreasing on", "streng monoton fallend auf")), down.join(", ")));
        }
        if self.f_is_periodic() {
            steps.push(step(AnalysisRule::SignChangeTest, msg!("continued periodically", "periodisch fortgesetzt"), String::new()));
        }
        self.section("monotonicity", steps, Some(parts.join(", \\quad ")), complete)
    }

    fn curvature(&mut self, f2: &Expr) -> Section {
        if f2.is_constant() && is_zero(f2) {
            return self.section("curvature", Vec::new(), Some(text(&msg!("f is linear: no curvature", "f ist linear: keine Krümmung"))), true);
        }
        let (pieces, complete) = self.sign_intervals(f2);
        let table = self.sign_table("f''", &pieces, "\\smile", "\\frown");
        let mut steps = vec![step(AnalysisRule::SignChangeTest, msg!("Sign of f'' between its zeros and gaps", "Vorzeichen von f'' zwischen den Nullstellen und Lücken"), table)];
        let language = self.ctx.settings.language;
        let list = |sign: i32| pieces.iter().filter(|(_, s)| *s == sign).map(|(i, _)| i.to_latex(self.ctx.digits, language)).collect::<Vec<_>>();
        let (left, right) = (list(1), list(-1));
        let mut parts = Vec::new();
        if !left.is_empty() {
            parts.push(format!("{}: {}", text(&msg!("left-curved (convex) on", "linksgekrümmt auf")), left.join(", ")));
        }
        if !right.is_empty() {
            parts.push(format!("{}: {}", text(&msg!("right-curved (concave) on", "rechtsgekrümmt auf")), right.join(", ")));
        }
        if self.f_is_periodic() {
            steps.push(step(AnalysisRule::SignChangeTest, msg!("continued periodically", "periodisch fortgesetzt"), String::new()));
        }
        self.section("curvature", steps, Some(parts.join(", \\quad ")), complete)
    }

    fn behaviour(&mut self) -> Section {
        let ctx = LimitContext { var: self.ctx.var, digits: self.ctx.digits };
        let mut steps = Vec::new();
        let mut complete = true;
        let mut lines = Vec::new();
        let set = self.domain.set.0.clone();
        // The outer ends of D.
        let mut ends: Vec<Approach> = Vec::new();
        match set.first().map(|i| &i.lo) {
            Some(Bound::NegInf) => ends.push(Approach::NegInf),
            Some(Bound::At { point, closed: false }) => ends.push(Approach::Right(point.clone())),
            _ => {}
        }
        match set.last().map(|i| &i.hi) {
            Some(Bound::PosInf) => ends.push(Approach::PosInf),
            Some(Bound::At { point, closed: false }) => ends.push(Approach::Left(point.clone())),
            _ => {}
        }
        let mut horizontal: Vec<(Point, Side)> = Vec::new();
        let mut oblique_lines: Vec<(Expr, Expr, Side)> = Vec::new();
        for end in ends {
            let (result, s) = behaviour(&self.f, &end, &ctx);
            steps.push(s);
            complete &= !matches!(result, Behaviour::Unknown);
            match (&end, &result) {
                (Approach::PosInf | Approach::NegInf, Behaviour::Finite(l)) => {
                    let side = if matches!(end, Approach::PosInf) { Side::PlusInfinity } else { Side::MinusInfinity };
                    match horizontal.iter_mut().find(|(p, _)| p.same(l)) {
                        Some((_, existing)) => *existing = Side::Both,
                        None => horizontal.push((l.clone(), side)),
                    }
                }
                (Approach::PosInf | Approach::NegInf, b) if b.is_infinite() => {
                    let side = if matches!(end, Approach::PosInf) { Side::PlusInfinity } else { Side::MinusInfinity };
                    if let Some((m, b, division)) = oblique(&self.f, &end, &ctx) {
                        match oblique_lines.iter_mut().find(|(m2, b2, _)| *m2 == m && *b2 == b) {
                            Some((.., existing)) => *existing = Side::Both,
                            None => {
                                steps.extend(division);
                                oblique_lines.push((m, b, side));
                            }
                        }
                    }
                }
                (Approach::Left(p) | Approach::Right(p), b) if b.is_infinite() => {
                    self.vertical(p, &mut lines);
                }
                _ => {}
            }
        }
        // Gaps inside D: poles and removable gaps.
        for pair in set.windows(2) {
            if let (Bound::At { point: a, .. }, Bound::At { point: b, .. }) = (&pair[0].hi, &pair[1].lo) {
                if a.same(b) {
                    self.gap(a, &ctx, &mut steps, &mut lines);
                }
            }
        }
        for family in self.domain.excluded.clone() {
            let members = family.points_in(self.window.0, self.window.1, MAX_WINDOW_POINTS);
            if let Some(first) = members.iter().find(|p| p.approx >= 0.0).or(members.first()) {
                let (_, _, s) = pole(&self.f, first, &ctx);
                steps.push(s);
            }
            for general in family.to_latex() {
                lines.push(format!("{} {} = {general}, \\ k \\in \\mathbb{{Z}}", text(&msg!("vertical asymptotes", "senkrechte Asymptoten")), self.ctx.var));
            }
            for member in members {
                self.breaks.push(member.approx);
                self.asymptotes.push(Asymptote::Vertical { x: member.approx, latex: format!("{} = {}", self.ctx.var, member.to_latex(self.ctx.digits)) });
            }
        }
        for (l, side) in horizontal {
            let latex = format!("y = {}", l.to_latex(self.ctx.digits));
            lines.push(format!("{} {latex}{}", text(&msg!("horizontal asymptote", "waagrechte Asymptote")), side_text(side)));
            self.asymptotes.push(Asymptote::Horizontal { y: l.approx, side, latex });
        }
        for (m, b, side) in oblique_lines {
            let line = simplify(&Expr::Sum(vec![Expr::Product(vec![m.clone(), Expr::Var]), b.clone()]));
            let latex = format!("y = {}", self.tex(&line));
            lines.push(format!("{} {latex}{}", text(&msg!("oblique asymptote", "schiefe Asymptote")), side_text(side)));
            self.asymptotes.push(Asymptote::Oblique {
                slope: m.eval_f64(0.0).unwrap_or(f64::NAN),
                intercept: b.eval_f64(0.0).unwrap_or(f64::NAN),
                side,
                latex,
            });
        }
        let result = if lines.is_empty() { text(&msg!("No asymptotes", "Keine Asymptoten")) } else { lines.join(", \\quad ") };
        self.section("behaviour", steps, Some(result), complete)
    }

    fn vertical(&mut self, p: &Point, lines: &mut Vec<String>) {
        let latex = format!("{} = {}", self.ctx.var, p.to_latex(self.ctx.digits));
        if self.asymptotes.iter().any(|a| matches!(a, Asymptote::Vertical { x, .. } if (x - p.approx).abs() < 1e-12)) {
            return;
        }
        lines.push(format!("{} {latex}", text(&msg!("vertical asymptote", "senkrechte Asymptote"))));
        self.breaks.push(p.approx);
        self.asymptotes.push(Asymptote::Vertical { x: p.approx, latex });
    }

    fn gap(&mut self, at: &Point, ctx: &LimitContext, steps: &mut Vec<Step>, lines: &mut Vec<String>) {
        let (left, right, s) = pole(&self.f, at, ctx);
        steps.push(s);
        if left.is_infinite() || right.is_infinite() {
            self.vertical(at, lines);
        } else if let (Behaviour::Finite(l), Behaviour::Finite(r)) = (&left, &right) {
            if l.same(r) {
                let coordinates = self.mark(PointKind::Hole, at, l);
                lines.push(format!("{} {coordinates}", text(&msg!("removable gap", "hebbare Lücke"))));
                self.breaks.push(at.approx);
            }
        }
    }

    fn antiderivative(&mut self) -> Section {
        let ctx = IntegrateContext { settings: self.ctx.settings, var: self.ctx.var, domain: &self.domain };
        match antiderivative(&self.f, &ctx) {
            Some(found) => {
                let result = format!("F({}) = {} + C", self.ctx.var, self.tex(&found.result));
                self.section("antiderivative", found.steps, Some(result), true)
            }
            None => {
                let message = msg!("No elementary antiderivative found", "Keine elementare Stammfunktion gefunden");
                let mut section = self.section("antiderivative", Vec::new(), Some(text(&message)), true);
                section.status = SectionStatus::Partial;
                section.message = Some(message);
                section
            }
        }
    }
}

/// The lower bound point of an interval (a dummy at -∞, which no bound equals).
fn piece_lo(interval: &Interval) -> &Point {
    static NOWHERE: std::sync::OnceLock<Point> = std::sync::OnceLock::new();
    match &interval.lo {
        Bound::At { point, .. } => point,
        _ => NOWHERE.get_or_init(|| Point::approx(f64::NAN)),
    }
}

fn bound_is(bound: &Bound, p: &Point) -> bool {
    matches!(bound, Bound::At { point, .. } if point.same(p))
}

/// `b + kp` split into the families with period `big` (a multiple of p).
fn split_family(family: &PeriodicSet, big: &Expr) -> Vec<PeriodicSet> {
    let ratio = (big.eval_f64(0.0).unwrap_or(1.0) / family.period_f64()).round().max(1.0) as i64;
    let mut out = Vec::new();
    for base in &family.base {
        for j in 0..ratio {
            let shifted = simplify(&Expr::Sum(vec![base.clone(), Expr::Product(vec![Expr::int(j), family.period.clone()])]));
            // The representative in [0, P).
            let value = shifted.eval_f64(0.0).unwrap_or(0.0);
            let p = big.eval_f64(0.0).unwrap_or(1.0);
            let k = (value / p).floor() as i64;
            let normal = simplify(&Expr::Sum(vec![shifted, Expr::Product(vec![Expr::int(-k), big.clone()])]));
            out.push(PeriodicSet { base: vec![normal], period: big.clone() });
        }
    }
    out
}

fn side_text(side: Side) -> String {
    match side {
        Side::Both => String::new(),
        Side::PlusInfinity => format!(" \\ \\text{{({})}}", msg!("for x → ∞", "für x → ∞")),
        Side::MinusInfinity => format!(" \\ \\text{{({})}}", msg!("for x → −∞", "für x → −∞")),
    }
}

fn text(words: &str) -> String {
    format!("\\text{{{words}}}")
}

fn approx_text(p: &Point, digits: usize) -> String {
    if p.exact.is_some() {
        p.to_latex(digits)
    } else {
        format!("\\approx {}", p.to_latex(digits))
    }
}
