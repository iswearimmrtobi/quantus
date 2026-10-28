//! Solving `f(x) = 0` with the working a student writes: exact where a school method applies,
//! numerically (and said so) where none does.
//!
//! The strategies, in order: a constant; a fraction (its numerator); a product (each factor, the
//! zero-product rule, naming factors that are never zero); a polynomial (factoring out x, the
//! linear and quadratic formulas, substitution, guessing a root and dividing it off, numerically
//! inside the Cauchy bound when nothing else is left); an equation with one outer function,
//! which is inverted (exp, ln, roots, powers, the trig functions with their periodic solutions);
//! and finally a numeric scan.

use std::cmp::Ordering;

use dashu::base::{Abs, Signed, UnsignedAbs};
use dashu::integer::{IBig, UBig};
use dashu::rational::RBig;

use super::derive::derivative;
use super::exact::times_pi;
use super::expr::{split_coefficient, Expr, Fn};
use super::interval::{Bound, Domain, Interval, IntervalSet, PeriodicSet, Point};
use super::polydiv::{divide, linear_factor};
use super::rules::{step, AnalysisRule};
use super::simplify::{display, is_zero, simplify};
use crate::algebra::domain::has_no_real_roots;
use crate::algebra::poly::{Monomial, Poly, Term};
use crate::algebra::rational_roots::{find_rational_root, square_free};
use crate::algebra::verify::eval_poly;
use crate::eval::solver::{scan_roots, Sample};
use crate::eval::Step;
use crate::settings::Settings;
use crate::text::msg;

/// How far the numeric scan looks when no other method applies.
pub const SCAN_LIMIT: f64 = 100.0;

/// Most roots a numeric scan reports; past that the list is cut and marked incomplete.
const MAX_NUMERIC_ROOTS: usize = 50;

/// Rational-root candidates are listed when both end coefficients are at most this large.
const CANDIDATE_LIMIT: u64 = 1000;

#[derive(Clone, Debug)]
pub struct Root {
    pub point: Point,
    /// How often the root counts (2 for a double root), when known.
    pub multiplicity: Option<u32>,
}

/// The solutions of an equation: single roots, periodic families (`x = k\pi`), or every number.
#[derive(Clone, Debug, Default)]
pub struct Solutions {
    pub roots: Vec<Root>,
    pub families: Vec<PeriodicSet>,
    pub all: bool,
}

impl Solutions {
    fn none() -> Solutions {
        Solutions::default()
    }

    fn root(point: Point, multiplicity: u32) -> Solutions {
        Solutions { roots: vec![Root { point, multiplicity: Some(multiplicity) }], ..Solutions::default() }
    }

    fn extend(&mut self, other: Solutions) {
        self.roots.extend(other.roots);
        self.families.extend(other.families);
        self.all |= other.all;
    }

    pub fn is_empty(&self) -> bool {
        self.roots.is_empty() && self.families.is_empty() && !self.all
    }
}

pub struct Solved {
    pub solutions: Solutions,
    pub steps: Vec<Step>,
    /// False when part of the search was numeric in a window, so solutions outside it may exist.
    pub complete: bool,
}

pub struct SolveContext<'a> {
    pub settings: &'a Settings,
    /// How the variable is written.
    pub var: &'a str,
    /// Solutions outside are dropped.
    pub domain: &'a Domain,
    /// Where periodic solutions of a non-linear argument are searched numerically.
    pub window: (f64, f64),
    /// Significant digits of numeric solutions.
    pub digits: usize,
}

struct Solver<'a> {
    ctx: &'a SolveContext<'a>,
    steps: Vec<Step>,
    complete: bool,
}

/// The solutions of `e = 0` inside the domain, with the working.
pub fn solve_zero(e: &Expr, ctx: &SolveContext) -> Solved {
    let mut solver = Solver { ctx, steps: Vec::new(), complete: true };
    let mut solutions = solver.solve(e);
    let mut kept = Vec::new();
    for root in std::mem::take(&mut solutions.roots) {
        if ctx.domain.contains(root.point.approx) {
            kept.push(root);
        } else {
            let x = root.point.to_latex(ctx.digits);
            solver.steps.push(step(
                AnalysisRule::DomainCondition,
                msg!("Not in the domain, so no solution", "Nicht im Definitionsbereich, also keine Lösung"),
                format!("{} = {x} \\notin D", ctx.var),
            ));
        }
    }
    solutions.roots = merge(kept);
    solutions.families = merge_families(solutions.families);
    Solved { solutions, steps: solver.steps, complete: solver.complete }
}

/// Sorted by value; the same root found twice counts with both multiplicities.
fn merge(mut roots: Vec<Root>) -> Vec<Root> {
    roots.sort_by(|a, b| a.point.approx.partial_cmp(&b.point.approx).unwrap_or(Ordering::Equal));
    let mut out: Vec<Root> = Vec::new();
    for root in roots {
        match out.last_mut() {
            Some(last) if last.point.same(&root.point) => {
                last.multiplicity = match (last.multiplicity, root.multiplicity) {
                    (Some(a), Some(b)) => Some(a + b),
                    _ => None,
                };
                if last.point.exact.is_none() {
                    last.point = root.point;
                }
            }
            _ => out.push(root),
        }
    }
    out
}

impl Solver<'_> {
    fn tex(&self, e: &Expr) -> String {
        e.to_latex(self.ctx.var)
    }

    fn push(&mut self, rule: AnalysisRule, explanation: String, latex: String) {
        self.steps.push(step(rule, explanation, latex));
    }

    fn solve(&mut self, e: &Expr) -> Solutions {
        let shown = display(e, self.ctx.settings);
        if shown.is_constant() {
            if is_zero(&shown) {
                return Solutions { all: true, ..Solutions::default() };
            }
            let latex = format!("{} \\neq 0", self.tex(&shown));
            self.push(AnalysisRule::ZeroProduct, msg!("No solution", "Keine Lösung"), latex);
            return Solutions::none();
        }
        if matches!(simplify(e), Expr::Sum(_)) && matches!(shown, Expr::Product(_)) {
            let latex = format!("{} = 0", self.tex(&shown));
            self.push(AnalysisRule::FactorOut, msg!("Factor out", "Ausklammern"), latex);
        }
        match &shown {
            Expr::Product(factors) => {
                let rest: Vec<Expr> = factors.iter().filter(|f| !f.is_constant()).cloned().collect();
                let (below, above): (Vec<Expr>, Vec<Expr>) = rest.into_iter().partition(is_below_the_line);
                if !below.is_empty() {
                    let numerator = Expr::product(above);
                    let latex = format!("{} = 0", self.tex(&numerator));
                    self.push(
                        AnalysisRule::ZeroProduct,
                        msg!("A fraction is zero where its numerator is zero", "Ein Bruch ist null, wenn der Zähler null ist"),
                        latex,
                    );
                    return self.solve(&numerator);
                }
                if above.len() == 1 {
                    return self.solve_factor(&above[0]);
                }
                let latex = above.iter().map(|f| format!("{} = 0", self.tex(f))).collect::<Vec<_>>().join(" \\lor ");
                self.push(
                    AnalysisRule::ZeroProduct,
                    msg!("A product is zero where one of its factors is zero", "Ein Produkt ist null, wenn ein Faktor null ist"),
                    latex,
                );
                let mut all = Solutions::none();
                for factor in &above {
                    all.extend(self.solve_factor(factor));
                }
                all
            }
            Expr::Pow(..) => self.solve_factor(&shown),
            _ => self.solve_single(&shown),
        }
    }

    fn solve_factor(&mut self, factor: &Expr) -> Solutions {
        if let Some(sign) = never_zero(factor) {
            let latex = format!("{} {sign} 0", self.tex(factor));
            self.push(AnalysisRule::ZeroProduct, msg!("This factor is never zero", "Dieser Faktor wird nie null"), latex);
            return Solutions::none();
        }
        if let Expr::Pow(base, exponent) = factor {
            if let Some(r) = exponent.as_num().filter(|r| r.is_positive()) {
                let mut inner = self.solve(base);
                let times = if r.is_int() { u32::try_from(r.numerator().unsigned_abs()).ok() } else { None };
                for root in &mut inner.roots {
                    root.multiplicity = match (root.multiplicity, times) {
                        (Some(m), Some(t)) => Some(m * t),
                        _ => None,
                    };
                }
                return inner;
            }
        }
        self.solve_single(factor)
    }

    fn solve_single(&mut self, e: &Expr) -> Solutions {
        if let Some((numerator, denominator)) = e.as_rational() {
            if denominator.is_constant() {
                return self.polynomial(&numerator);
            }
            let poly = Expr::from_poly(&numerator);
            return self.solve(&poly);
        }
        if let Some(solutions) = self.isolate(e) {
            return solutions;
        }
        self.numeric(e, (-SCAN_LIMIT, SCAN_LIMIT))
    }

    // ---- polynomials ---------------------------------------------------------------------------

    fn polynomial(&mut self, p: &Poly) -> Solutions {
        let x = Expr::poly_variable();
        let degree = p.degree_of(x);
        let c = |k: usize| p.coeff_of(&Monomial::single(x, k));
        if degree == 0 {
            return Solutions::none();
        }
        let k = p.monomial_gcd().degree_of(x);
        if k == degree {
            let latex = format!("{} = 0", self.ctx.var);
            self.push(AnalysisRule::ZeroProduct, msg!("Only zero solves it", "Nur null löst die Gleichung"), latex);
            return Solutions::root(Point::rational(RBig::ZERO), k as u32);
        }
        if k > 0 {
            let rest = Poly::from_terms(
                p.terms.iter().map(|t| Term::new(t.coeff.clone(), Monomial::single(x, t.monomial.degree_of(x) - k))).collect(),
            );
            let power = if k == 1 { Expr::Var } else { Expr::pow(Expr::Var, Expr::int(k as i64)) };
            let latex = format!("{} \\cdot ({}) = 0", self.tex(&power), self.tex(&Expr::from_poly(&rest)));
            let name = self.tex(&power);
            self.push(AnalysisRule::FactorOut, msg!("Factor out {}", "{} ausklammern", name), latex);
            let mut all = Solutions::root(Point::rational(RBig::ZERO), k as u32);
            all.extend(self.polynomial(&rest));
            return all;
        }
        if degree == 1 {
            let root = -c(0) / c(1);
            let latex = format!("{} = {}", self.ctx.var, self.tex(&Expr::Num(root.clone())));
            self.push(AnalysisRule::InverseFunction, msg!("Solve the linear equation", "Lineare Gleichung lösen"), latex);
            return Solutions::root(Point::rational(root), 1);
        }
        if degree == 2 {
            return self.quadratic(&c(2), &c(1), &c(0), self.ctx.var);
        }
        let nonzero: Vec<usize> = p.terms.iter().map(|t| t.monomial.degree_of(x)).collect();
        // a x^n + c: a pure power.
        if nonzero.iter().all(|&d| d == 0 || d == degree) {
            let value = Expr::Num(-c(0) / c(degree));
            let latex = format!("{} = {}", self.tex(&Expr::pow(Expr::Var, Expr::int(degree as i64))), self.tex(&value));
            self.push(AnalysisRule::InverseFunction, msg!("Isolate the power", "Potenz isolieren"), latex);
            return self.pure_power(degree, &value, 1);
        }
        // a x^{2m} + b x^m + c: substitution z = x^m.
        if degree % 2 == 0 && nonzero.iter().all(|&d| d % (degree / 2) == 0) {
            return self.substitution(degree / 2, &c(degree), &c(degree / 2), &c(0));
        }
        if let Some(r) = find_rational_root(p, x) {
            self.guess(p, &r);
            let division = divide(p, &linear_factor(&r), self.ctx.var);
            self.steps.extend(division.steps);
            let mut all = Solutions::root(Point::rational(r), 1);
            all.extend(self.polynomial(&division.quotient));
            return all;
        }
        self.numeric_polynomial(p)
    }

    fn guess(&mut self, p: &Poly, r: &RBig) {
        let x = Expr::poly_variable();
        let primitive = p.primitive_part();
        let ends = (primitive.constant_term(), primitive.coeff_of(&Monomial::single(x, p.degree_of(x))));
        let small = |v: &RBig| v.is_int() && v.numerator().unsigned_abs() <= UBig::from(CANDIDATE_LIMIT);
        let candidates = if small(&ends.0) && small(&ends.1) {
            let a0 = u64::try_from(ends.0.numerator().unsigned_abs()).unwrap();
            let an = u64::try_from(ends.1.numerator().unsigned_abs()).unwrap();
            let mut list: Vec<RBig> = Vec::new();
            for p in divisors(a0) {
                for q in divisors(an) {
                    let candidate = RBig::from_parts(IBig::from(p), UBig::from(q));
                    if !list.contains(&candidate) {
                        list.push(candidate);
                    }
                }
            }
            list.sort();
            let mut shown: Vec<String> = list.iter().take(12).map(|c| format!("\\pm {}", self.tex(&Expr::Num(c.clone())))).collect();
            if list.len() > 12 {
                shown.push("\\dots".into());
            }
            format!("{} \\in \\{{{}\\}}, \\quad ", self.ctx.var, shown.join(", "))
        } else {
            String::new()
        };
        let value = self.tex(&Expr::Num(r.clone()));
        let latex = format!("{candidates}{} = {value}", self.ctx.var);
        self.push(
            AnalysisRule::RationalRootGuess,
            msg!("By inspection: {} = {} is a root", "Raten: {} = {} ist eine Nullstelle", self.ctx.var, value),
            latex,
        );
    }

    /// `a·s^2 + b·s + c = 0` in the symbol `s` (x, or z after a substitution).
    fn quadratic(&mut self, a: &RBig, b: &RBig, c: &RBig, s: &str) -> Solutions {
        let num = |r: &RBig| Expr::Num(r.clone()).to_latex(s);
        let two = RBig::from(2);
        if *b == RBig::ZERO {
            let value = Expr::Num(-c.clone() / a);
            let latex = format!("{s}^{{2}} = {}", num(&(-c.clone() / a)));
            self.push(AnalysisRule::InverseFunction, msg!("Isolate the square", "Quadrat isolieren"), latex);
            return self.pure_power(2, &value, 1);
        }
        let (discriminant, centre, root_scale);
        if self.ctx.settings.uses_pq_formula() {
            let (p, q) = (b / a, c / a);
            if *a != RBig::ONE {
                let normal = Expr::poly_from_coefficients(&[q.clone(), p.clone(), RBig::ONE]);
                let latex = format!("{} = 0", Expr::from_poly(&normal).to_latex(s));
                self.push(AnalysisRule::QuadraticPq, msg!("Divide by {}", "Durch {} teilen", num(a)), latex);
            }
            let half = -p.clone() / &two;
            discriminant = &half * &half - &q;
            let latex = format!(
                "{s}_{{1,2}} = -\\frac{{p}}{{2}} \\pm \\sqrt{{\\left(\\frac{{p}}{{2}}\\right)^{{2}} - q}} = {} \\pm \\sqrt{{{}}}",
                num(&half),
                num(&discriminant)
            );
            self.push(
                AnalysisRule::QuadraticPq,
                msg!("pq formula with p = {}, q = {}", "pq-Formel mit p = {}, q = {}", plain(&p), plain(&q)),
                latex,
            );
            centre = half;
            root_scale = RBig::ONE;
        } else {
            discriminant = b * b - RBig::from(4) * a * c;
            let latex = format!(
                "{s}_{{1,2}} = \\frac{{-b \\pm \\sqrt{{b^{{2}} - 4ac}}}}{{2a}} = \\frac{{{} \\pm \\sqrt{{{}}}}}{{{}}}",
                num(&-b.clone()),
                num(&discriminant),
                num(&(&two * a))
            );
            self.push(
                AnalysisRule::QuadraticAbc,
                msg!(
                    "Quadratic formula with a = {}, b = {}, c = {}",
                    "Mitternachtsformel mit a = {}, b = {}, c = {}",
                    plain(a),
                    plain(b),
                    plain(c)
                ),
                latex,
            );
            centre = -b.clone() / (&two * a);
            root_scale = RBig::ONE / (&two * a).abs();
        }
        if discriminant.is_negative() {
            let latex = format!("{} < 0", num(&discriminant));
            self.push(
                AnalysisRule::QuadraticAbc,
                msg!("The radicand is negative: no real solution", "Der Radikand ist negativ: keine reelle Lösung"),
                latex,
            );
            return Solutions::none();
        }
        if discriminant == RBig::ZERO {
            let latex = format!("{s} = {}", num(&centre));
            self.push(AnalysisRule::QuadraticAbc, msg!("Double solution", "Doppelte Lösung"), latex);
            return Solutions::root(Point::rational(centre), 2);
        }
        let root = Expr::Product(vec![Expr::Num(root_scale), Expr::pow(Expr::Num(discriminant), Expr::ratio(1, 2))]);
        let mut points: Vec<Point> = [RBig::ONE, -RBig::ONE]
            .into_iter()
            .filter_map(|sign| {
                Point::exact(simplify(&Expr::Sum(vec![Expr::Num(centre.clone()), Expr::Product(vec![Expr::Num(sign), root.clone()])])))
            })
            .collect();
        points.sort_by(|a, b| a.approx.partial_cmp(&b.approx).unwrap_or(Ordering::Equal));
        let latex = format!("{s}_{{1}} = {}, \\quad {s}_{{2}} = {}", points[0].to_latex(self.ctx.digits), points[1].to_latex(self.ctx.digits));
        self.push(AnalysisRule::QuadraticAbc, msg!("Two solutions", "Zwei Lösungen"), latex);
        let mut all = Solutions::none();
        for point in points {
            all.extend(Solutions::root(point, 1));
        }
        all
    }

    /// `x^n = value`, each solution counted `multiplicity` times.
    fn pure_power(&mut self, n: usize, value: &Expr, multiplicity: u32) -> Solutions {
        let var = self.ctx.var;
        let v = value.eval_f64(0.0).unwrap_or(f64::NAN);
        if v == 0.0 && is_zero(value) {
            let latex = format!("{var} = 0");
            self.push(AnalysisRule::InverseFunction, msg!("Only zero", "Nur null"), latex);
            return Solutions::root(Point::rational(RBig::ZERO), n as u32 * multiplicity);
        }
        let exponent = Expr::Num(RBig::from_parts(IBig::ONE, UBig::from(n)));
        if n % 2 == 0 {
            if v < 0.0 {
                let latex = format!("{} < 0", self.tex(value));
                self.push(
                    AnalysisRule::InverseFunction,
                    msg!("An even power is never negative: no solution", "Eine gerade Potenz ist nie negativ: keine Lösung"),
                    latex,
                );
                return Solutions::none();
            }
            let root = simplify(&Expr::pow(value.clone(), exponent));
            let latex = format!("{var}_{{1,2}} = \\pm {}", self.tex(&root));
            self.push(AnalysisRule::InverseFunction, msg!("Take the root", "Wurzel ziehen"), latex);
            let mut all = Solutions::none();
            for candidate in [simplify(&Expr::neg(root.clone())), root] {
                if let Some(point) = Point::exact(candidate) {
                    all.extend(Solutions::root(point, multiplicity));
                }
            }
            return all;
        }
        let root = if v < 0.0 {
            simplify(&Expr::neg(Expr::pow(simplify(&Expr::neg(value.clone())), exponent)))
        } else {
            simplify(&Expr::pow(value.clone(), exponent))
        };
        let latex = format!("{var} = {}", self.tex(&root));
        self.push(AnalysisRule::InverseFunction, msg!("Take the root", "Wurzel ziehen"), latex);
        match Point::exact(root) {
            Some(point) => Solutions::root(point, multiplicity),
            None => Solutions::none(),
        }
    }

    fn substitution(&mut self, m: usize, a: &RBig, b: &RBig, c: &RBig) -> Solutions {
        let var = self.ctx.var;
        let z_poly = Expr::from_poly(&Expr::poly_from_coefficients(&[c.clone(), b.clone(), a.clone()]));
        let power = Expr::pow(Expr::Var, Expr::int(m as i64)).to_latex(var);
        let latex = format!("z = {power}: \\quad {} = 0", z_poly.to_latex("z"));
        self.push(AnalysisRule::Substitution, msg!("Substitution", "Substitution"), latex);
        let in_z = self.quadratic(a, b, c, "z");
        let mut all = Solutions::none();
        for root in in_z.roots {
            let z = root.point.exact.clone().unwrap_or(Expr::int(0));
            let latex = format!("{power} = {}", z.to_latex(var));
            self.push(AnalysisRule::Substitution, msg!("Substitute back", "Rücksubstitution"), latex);
            all.extend(self.pure_power(m, &z, root.multiplicity.unwrap_or(1)));
        }
        all
    }

    fn numeric_polynomial(&mut self, p: &Poly) -> Solutions {
        let x = Expr::poly_variable();
        let simple = square_free(p, x).unwrap_or_else(|| p.clone());
        let degree = simple.degree_of(x);
        let lead = simple.coeff_of(&Monomial::single(x, degree));
        let bound = simple
            .terms
            .iter()
            .map(|t| (t.coeff.clone() / &lead).abs())
            .max()
            .unwrap_or(RBig::ONE)
            + RBig::ONE;
        let bound = bound.ceil();
        let span = u64::try_from((&bound).unsigned_abs()).unwrap_or(u64::MAX).max(1);
        let steps_per_unit = (2000 / (2 * span)).max(1);
        let mut sample = |v: &RBig| -> crate::Result<Option<Sample>> {
            let mut env = std::collections::BTreeMap::new();
            env.insert(x.to_string(), v.clone());
            let y = eval_poly(&simple, &env);
            let sign = if y == RBig::ZERO { 0 } else if y.is_negative() { -1 } else { 1 };
            Ok(Some((sign, y.abs())))
        };
        let tolerance = RBig::from_parts(IBig::ONE, UBig::from(10u8).pow(self.ctx.digits + 5));
        let accept = RBig::from_parts(IBig::ONE, UBig::from(10u8).pow(self.ctx.digits + 2));
        let bound = RBig::from(bound);
        let found = scan_roots(&mut sample, &-bound.clone(), &bound, steps_per_unit, &tolerance, &accept).unwrap_or_default();
        let points: Vec<Point> = found
            .into_iter()
            .map(|r| Point { exact: None, approx: r.to_f64().value(), refined: Some(r) })
            .collect();
        self.numeric_step(&points, None);
        Solutions { roots: points.into_iter().map(|point| Root { point, multiplicity: None }).collect(), ..Solutions::default() }
    }

    fn numeric_step(&mut self, points: &[Point], window: Option<(f64, f64)>) {
        let var = self.ctx.var;
        let listed = if points.is_empty() {
            msg!("no solution found", "keine Lösung gefunden")
        } else {
            points
                .iter()
                .enumerate()
                .map(|(i, p)| format!("{var}_{{{}}} \\approx {}", i + 1, p.to_latex(self.ctx.digits)))
                .collect::<Vec<_>>()
                .join(", \\quad ")
        };
        let explanation = match window {
            None => msg!("No exact method applies: solved numerically", "Kein exaktes Verfahren: numerisch gelöst"),
            Some((lo, hi)) => msg!(
                "Solved numerically on [{}, {}]",
                "Numerisch gelöst im Bereich [{}; {}]",
                lo,
                hi
            ),
        };
        let latex = if points.is_empty() { format!("\\text{{{listed}}}") } else { listed };
        self.push(AnalysisRule::NumericRoot, explanation, latex);
    }

    // ---- one outer function --------------------------------------------------------------------

    /// `c·T(u) + b = 0` with a single x-dependent term: invert T, then solve for u.
    fn isolate(&mut self, e: &Expr) -> Option<Solutions> {
        let terms: Vec<Expr> = match e {
            Expr::Sum(terms) => terms.clone(),
            other => vec![other.clone()],
        };
        let (dependent, constant): (Vec<Expr>, Vec<Expr>) = terms.into_iter().partition(Expr::depends_on_var);
        let [term] = dependent.as_slice() else { return None };
        let (coefficient, outer) = split_constant_factor(term);
        if outer == Expr::Var {
            let x = simplify(&Expr::Product(vec![Expr::int(-1), Expr::sum(constant), Expr::recip(coefficient)]));
            let latex = format!("{} = {}", self.ctx.var, self.tex(&x));
            self.push(AnalysisRule::InverseFunction, msg!("Solve for {}", "Nach {} auflösen", self.ctx.var), latex);
            return Some(Point::exact(x).map(|p| Solutions::root(p, 1)).unwrap_or_default());
        }
        let invertible = match &outer {
            Expr::Func(..) => true,
            Expr::Pow(base, exponent) => base.is_constant() || exponent.is_constant(),
            _ => false,
        };
        if !invertible {
            return None;
        }
        let value = simplify(&Expr::Product(vec![Expr::int(-1), Expr::sum(constant.clone()), Expr::recip(coefficient.clone())]));
        if !constant.is_empty() || coefficient != Expr::int(1) {
            let latex = format!("{} = {}", self.tex(&outer), self.tex(&value));
            self.push(AnalysisRule::InverseFunction, msg!("Isolate the function", "Funktion isolieren"), latex);
        }
        let (inner, candidates) = self.invert(&outer, &value)?;
        let mut all = Solutions::none();
        for candidate in candidates {
            all.extend(self.solve_inner(&inner, candidate, e)?);
        }
        Some(all)
    }

    /// The inner function and the values it must take; `None` if the function cannot be inverted.
    fn invert(&mut self, outer: &Expr, value: &Expr) -> Option<(Expr, Vec<InnerValue>)> {
        let v = value.eval_f64(0.0)?;
        let none = |solver: &mut Self, reason: String| {
            let latex = format!("{} = {}", solver.tex(outer), solver.tex(value));
            solver.push(AnalysisRule::InverseFunction, reason, latex);
        };
        let one = |e: Expr| InnerValue::One(simplify(&e));
        let two_pi = Expr::Product(vec![Expr::int(2), Expr::Pi]);
        Some(match outer {
            Expr::Func(function, u) => {
                let u = (**u).clone();
                let values = match function {
                    Fn::Exp | Fn::Cosh if (*function == Fn::Exp && v <= 0.0) || (*function == Fn::Cosh && v < 1.0) => {
                        none(self, msg!("The function never takes this value: no solution", "Die Funktion nimmt diesen Wert nie an: keine Lösung"));
                        vec![]
                    }
                    Fn::Exp => vec![one(Expr::func(Fn::Ln, value.clone()))],
                    Fn::Ln => vec![one(Expr::func(Fn::Exp, value.clone()))],
                    Fn::Sin | Fn::Cos if v.abs() > 1.0 + 1e-15 => {
                        none(self, msg!("sin and cos stay between -1 and 1: no solution", "sin und cos bleiben zwischen -1 und 1: keine Lösung"));
                        vec![]
                    }
                    Fn::Sin => {
                        let alpha = simplify(&Expr::func(Fn::Arcsin, value.clone()));
                        let other = simplify(&Expr::Sum(vec![Expr::Pi, Expr::neg(alpha.clone())]));
                        let mut values = vec![InnerValue::Family(alpha.clone(), two_pi.clone())];
                        if !Point::exact(alpha.clone()).zip(Point::exact(Expr::Sum(vec![other.clone(), Expr::neg(two_pi.clone())]))).is_some_and(|(a, b)| a.same(&b))
                            && !Point::exact(alpha).zip(Point::exact(other.clone())).is_some_and(|(a, b)| a.same(&b))
                        {
                            values.push(InnerValue::Family(other, two_pi.clone()));
                        }
                        values
                    }
                    Fn::Cos => {
                        let alpha = simplify(&Expr::func(Fn::Arccos, value.clone()));
                        let mut values = vec![InnerValue::Family(alpha.clone(), two_pi.clone())];
                        let a = alpha.eval_f64(0.0).unwrap_or(0.0);
                        if a.abs() > 1e-12 && (a - std::f64::consts::PI).abs() > 1e-12 {
                            values.push(InnerValue::Family(simplify(&Expr::neg(alpha)), two_pi.clone()));
                        }
                        values
                    }
                    Fn::Tan => vec![InnerValue::Family(simplify(&Expr::func(Fn::Arctan, value.clone())), Expr::Pi)],
                    Fn::Arcsin | Fn::Arccos | Fn::Arctan => {
                        let half_pi = std::f64::consts::FRAC_PI_2;
                        let (inside, back) = match function {
                            Fn::Arcsin => (v.abs() <= half_pi, Fn::Sin),
                            Fn::Arccos => ((0.0..=std::f64::consts::PI).contains(&v), Fn::Cos),
                            _ => (v.abs() < half_pi, Fn::Tan),
                        };
                        if !inside {
                            none(self, msg!("The function never takes this value: no solution", "Die Funktion nimmt diesen Wert nie an: keine Lösung"));
                            vec![]
                        } else {
                            vec![one(Expr::func(back, value.clone()))]
                        }
                    }
                    Fn::Sinh => {
                        // arsinh v = ln(v + \sqrt{v^2 + 1})
                        let root = Expr::pow(Expr::Sum(vec![Expr::pow(value.clone(), Expr::int(2)), Expr::int(1)]), Expr::ratio(1, 2));
                        vec![one(Expr::func(Fn::Ln, Expr::Sum(vec![value.clone(), root])))]
                    }
                    Fn::Cosh => {
                        let root = Expr::pow(Expr::Sum(vec![Expr::pow(value.clone(), Expr::int(2)), Expr::int(-1)]), Expr::ratio(1, 2));
                        let positive = simplify(&Expr::func(Fn::Ln, Expr::Sum(vec![value.clone(), root])));
                        if v == 1.0 { vec![InnerValue::One(positive)] } else { vec![one(Expr::neg(positive.clone())), InnerValue::One(positive)] }
                    }
                    Fn::Abs => {
                        if v < 0.0 {
                            none(self, msg!("An absolute value is never negative: no solution", "Ein Betrag ist nie negativ: keine Lösung"));
                            vec![]
                        } else if v == 0.0 {
                            vec![InnerValue::One(Expr::int(0))]
                        } else {
                            vec![one(Expr::neg(value.clone())), InnerValue::One(value.clone())]
                        }
                    }
                };
                (u, values)
            }
            // a^u = v
            Expr::Pow(base, exponent) if base.is_constant() => {
                if v <= 0.0 {
                    none(self, msg!("A power of a positive number is positive: no solution", "Eine Potenz einer positiven Zahl ist positiv: keine Lösung"));
                    return Some(((**exponent).clone(), vec![]));
                }
                let u = exact_log(base, value).unwrap_or_else(|| {
                    Expr::Product(vec![Expr::func(Fn::Ln, value.clone()), Expr::recip(Expr::func(Fn::Ln, (**base).clone()))])
                });
                ((**exponent).clone(), vec![one(u)])
            }
            // u^r = v
            Expr::Pow(base, exponent) => {
                let r = exponent.as_num()?.clone();
                let even_root = r.denominator().clone() % UBig::from(2u8) == UBig::ZERO;
                let even_power = r.numerator() % IBig::from(2) == IBig::ZERO;
                if (even_root || even_power) && v < 0.0 {
                    none(self, msg!("This power is never negative: no solution", "Diese Potenz ist nie negativ: keine Lösung"));
                    return Some(((**base).clone(), vec![]));
                }
                if v == 0.0 {
                    if r.is_negative() {
                        none(self, msg!("This power is never zero: no solution", "Diese Potenz wird nie null: keine Lösung"));
                        return Some(((**base).clone(), vec![]));
                    }
                    return Some(((**base).clone(), vec![InnerValue::One(Expr::int(0))]));
                }
                let inverse = Expr::Num(RBig::ONE / &r);
                let magnitude = if v < 0.0 {
                    simplify(&Expr::neg(Expr::pow(simplify(&Expr::neg(value.clone())), inverse)))
                } else {
                    simplify(&Expr::pow(value.clone(), inverse))
                };
                let values = if even_power {
                    vec![one(Expr::neg(magnitude.clone())), InnerValue::One(magnitude)]
                } else {
                    vec![InnerValue::One(magnitude)]
                };
                ((**base).clone(), values)
            }
            _ => return None,
        })
    }

    /// `u = value`: directly when u is x or linear, otherwise by solving `u - value = 0`.
    /// `None` sends the whole equation `original` to the numeric scan.
    fn solve_inner(&mut self, u: &Expr, value: InnerValue, original: &Expr) -> Option<Solutions> {
        let var = self.ctx.var;
        let (base, period) = match &value {
            InnerValue::One(v) => (v.clone(), None),
            InnerValue::Family(v, p) => (v.clone(), Some(p.clone())),
        };
        let shown = match &period {
            None => format!("{} = {}", self.tex(u), self.tex(&base)),
            Some(p) => format!("{} = {} + k \\cdot {}, \\quad k \\in \\mathbb{{Z}}", self.tex(u), self.tex(&base), self.tex(p)),
        };
        self.push(AnalysisRule::InverseFunction, msg!("Apply the inverse function", "Umkehrfunktion anwenden"), shown);
        let slope = simplify(&derivative(u));
        let linear = slope.is_constant() && !is_zero(&slope);
        if *u != Expr::Var && !linear {
            return match period {
                None => Some(self.solve(&Expr::Sum(vec![u.clone(), Expr::neg(base)]))),
                Some(_) => {
                    let window = self.ctx.window;
                    Some(self.numeric(original, window))
                }
            };
        }
        let (alpha, beta) = if *u == Expr::Var {
            (Expr::int(1), Expr::int(0))
        } else {
            (slope, simplify(&u.substitute(&Expr::int(0))))
        };
        let x = simplify(&Expr::Product(vec![Expr::Sum(vec![base, Expr::neg(beta)]), Expr::recip(alpha.clone())]));
        match period {
            None => {
                if *u != Expr::Var {
                    let latex = format!("{var} = {}", self.tex(&x));
                    self.push(AnalysisRule::InverseFunction, msg!("Solve for {}", "Nach {} auflösen", var), latex);
                }
                Some(Point::exact(x).map(|p| Solutions::root(p, 1)).unwrap_or_default())
            }
            Some(p) => {
                let period = simplify(&Expr::Product(vec![p, Expr::recip(Expr::func(Fn::Abs, alpha))]));
                let family = PeriodicSet { base: vec![x], period };
                if *u != Expr::Var {
                    let latex = format!("{var} = {}, \\quad k \\in \\mathbb{{Z}}", family.to_latex()[0]);
                    self.push(AnalysisRule::InverseFunction, msg!("Solve for {}", "Nach {} auflösen", var), latex);
                }
                Some(Solutions { families: vec![family], ..Solutions::default() })
            }
        }
    }

    // ---- numeric -------------------------------------------------------------------------------

    /// Scans the domain inside `range` for sign changes and touching zeros.
    fn numeric(&mut self, e: &Expr, range: (f64, f64)) -> Solutions {
        self.complete = false;
        let window = IntervalSet(vec![Interval {
            lo: Bound::At { point: Point::approx(range.0), closed: true },
            hi: Bound::At { point: Point::approx(range.1), closed: true },
        }]);
        let mut found: Vec<f64> = Vec::new();
        for interval in self.ctx.domain.set.intersect(&window).0 {
            let (lo, hi) = (interval.lo.value(), interval.hi.value());
            let samples = (((hi - lo) / (range.1 - range.0)) * 4000.0).max(200.0) as usize;
            found.extend(scan_f64(e, lo, hi, samples));
        }
        found.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        found.dedup_by(|a, b| (*a - *b).abs() <= 1e-9 * a.abs().max(1.0));
        found.truncate(MAX_NUMERIC_ROOTS);
        let points: Vec<Point> = found.into_iter().map(|x| recognise(e, x)).collect();
        self.numeric_step(&points, Some(range));
        Solutions { roots: points.into_iter().map(|point| Root { point, multiplicity: None }).collect(), ..Solutions::default() }
    }
}

/// A numerically found root as an exact number when a simple one fits and is exactly a root:
/// a rational with a small denominator, or such a multiple of π.
fn recognise(e: &Expr, x: f64) -> Point {
    let tolerance = 1e-9 * x.abs().max(1.0);
    let simple = |v: f64| {
        let r = super::interval::simplest_between(v - tolerance, v + tolerance);
        (r.denominator() <= &UBig::from(100u8)).then_some(r)
    };
    let mut candidates = Vec::new();
    if let Some(r) = simple(x) {
        candidates.push(Expr::Num(r));
    }
    if let Some(r) = simple(x / std::f64::consts::PI) {
        candidates.push(times_pi(r));
    }
    for candidate in candidates {
        if is_zero(&simplify(&e.substitute(&candidate))) {
            if let Some(point) = Point::exact(candidate) {
                return point;
            }
        }
    }
    Point::approx(x)
}

/// What the inner function of an inverted equation must equal.
enum InnerValue {
    One(Expr),
    /// `base + k·period`.
    Family(Expr, Expr),
}

/// `(c, T)` for a term `c·T` with constant `c`.
fn split_constant_factor(term: &Expr) -> (Expr, Expr) {
    match term {
        Expr::Product(factors) => {
            let (constant, rest): (Vec<Expr>, Vec<Expr>) = factors.iter().cloned().partition(Expr::is_constant);
            (Expr::product(constant), Expr::product(rest))
        }
        other => (Expr::int(1), other.clone()),
    }
}

fn is_below_the_line(factor: &Expr) -> bool {
    matches!(factor, Expr::Pow(_, exponent) if exponent.as_num().is_some_and(|e| e.is_negative()))
}

/// `">"` or `"\\neq"` when the factor has no zero at all.
fn never_zero(factor: &Expr) -> Option<&'static str> {
    match factor {
        Expr::Func(Fn::Exp | Fn::Cosh, _) => Some(">"),
        Expr::Pow(base, _) if base.is_constant() && base.eval_f64(0.0).is_some_and(|b| b > 0.0) => Some(">"),
        _ => {
            let (numerator, denominator) = factor.as_rational()?;
            if !denominator.is_constant() || numerator.is_constant() || !has_no_real_roots(&numerator) {
                return None;
            }
            let (coefficient, _) = split_coefficient(factor);
            let positive = factor.eval_f64(0.0).is_some_and(|v| v > 0.0) || coefficient.is_positive();
            Some(if positive { ">" } else { "\\neq" })
        }
    }
}

/// Roots of `e` in [lo, hi] from `samples` floating-point samples: sign changes (not across a
/// pole) refined by bisection, and minima of |e| that reach zero.
fn scan_f64(e: &Expr, lo: f64, hi: f64, samples: usize) -> Vec<f64> {
    let f = |x: f64| e.eval_f64(x);
    let nudge = (hi - lo) * 1e-9;
    let xs: Vec<f64> = (0..=samples).map(|i| (lo + (hi - lo) * i as f64 / samples as f64).clamp(lo + nudge, hi - nudge)).collect();
    let ys: Vec<Option<f64>> = xs.iter().map(|&x| f(x)).collect();
    let mut roots = Vec::new();
    for i in 0..samples {
        let (Some(a), Some(b)) = (ys[i], ys[i + 1]) else { continue };
        if a == 0.0 {
            roots.push(xs[i]);
            continue;
        }
        if a.signum() != b.signum() && b != 0.0 {
            let (mut l, mut r, mut fl) = (xs[i], xs[i + 1], a);
            for _ in 0..100 {
                let m = (l + r) / 2.0;
                let Some(fm) = f(m) else { break };
                if fm == 0.0 {
                    l = m;
                    r = m;
                    break;
                }
                if fm.signum() == fl.signum() {
                    l = m;
                    fl = fm;
                } else {
                    r = m;
                }
            }
            let m = (l + r) / 2.0;
            // Across a pole the values grow instead of shrinking.
            if f(m).is_some_and(|fm| fm.abs() <= 1e-8 * a.abs().max(b.abs()).max(1.0)) {
                roots.push(m);
            }
        }
    }
    if let Some(Some(last)) = ys.last() {
        if *last == 0.0 {
            roots.push(*xs.last().unwrap());
        }
    }
    // Zeros the function only touches.
    for i in 1..samples {
        let (Some(a), Some(b), Some(c)) = (ys[i - 1], ys[i], ys[i + 1]) else { continue };
        if a.signum() == b.signum() && b.signum() == c.signum() && b.abs() < a.abs() && b.abs() < c.abs() {
            let (mut l, mut r) = (xs[i - 1], xs[i + 1]);
            for _ in 0..200 {
                let m1 = l + (r - l) * 0.382;
                let m2 = l + (r - l) * 0.618;
                match (f(m1), f(m2)) {
                    (Some(f1), Some(f2)) if f1.abs() < f2.abs() => r = m2,
                    (Some(_), Some(_)) => l = m1,
                    _ => break,
                }
            }
            let m = (l + r) / 2.0;
            if f(m).is_some_and(|fm| fm.abs() <= 1e-10 * (a.abs() + c.abs()).max(1e-300)) {
                roots.push(m);
            }
        }
    }
    roots
}

/// `k` with `base^k = value` for whole k, when both are rational: `\log_2 8 = 3`.
fn exact_log(base: &Expr, value: &Expr) -> Option<Expr> {
    let (Expr::Num(a), Expr::Num(v)) = (base, value) else { return None };
    (-64i64..=64).find(|k| super::exact::radical(a, &RBig::from(*k)) == Some(Expr::Num(v.clone()))).map(Expr::int)
}

/// Two families with the same period whose bases lie half a period apart are one family with
/// half the period: `0 + 2k\pi` and `\pi + 2k\pi` are `k\pi`.
fn merge_families(mut families: Vec<PeriodicSet>) -> Vec<PeriodicSet> {
    let mut i = 0;
    while i < families.len() {
        let mut merged = false;
        for j in i + 1..families.len() {
            let (a, b) = (&families[i], &families[j]);
            let (p, q) = (a.period_f64(), b.period_f64());
            if a.base.len() != 1 || b.base.len() != 1 || (p - q).abs() > 1e-12 * p.abs() {
                continue;
            }
            let (x, y) = (a.base[0].eval_f64(0.0).unwrap_or(f64::NAN), b.base[0].eval_f64(0.0).unwrap_or(f64::NAN));
            let offset = ((x - y) / (p / 2.0)).round();
            if offset as i64 % 2 == 0 || ((x - y) - offset * p / 2.0).abs() > 1e-9 * p {
                continue;
            }
            let half = simplify(&Expr::Product(vec![a.period.clone(), Expr::ratio(1, 2)]));
            let k = (x / (p / 2.0)).floor() as i64;
            let base = simplify(&Expr::Sum(vec![a.base[0].clone(), Expr::Product(vec![Expr::int(-k), half.clone()])]));
            families[i] = PeriodicSet { base: vec![base], period: half };
            families.remove(j);
            merged = true;
            break;
        }
        if !merged {
            i += 1;
        }
    }
    families
}

fn divisors(n: u64) -> Vec<u64> {
    if n == 0 {
        return vec![1];
    }
    (1..=n).filter(|d| n % d == 0).collect()
}

/// A rational as plain text: `-2`, `\frac{1}{2}` is `1/2`.
fn plain(r: &RBig) -> String {
    if r.is_int() {
        r.numerator().to_string()
    } else {
        format!("{}/{}", r.numerator(), r.denominator())
    }
}

/// `k·π` multiples as exact expressions, for tests and callers that build families.
pub fn pi_family(base: RBig, period: RBig) -> PeriodicSet {
    PeriodicSet { base: vec![times_pi(base)], period: times_pi(period) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::latex::parser;
    use crate::settings::{Language, Mode, QuadraticFormula};

    fn expr(latex: &str) -> Expr {
        let options = Settings { mode: Mode::Algebra, ..Settings::default() }.parse_options();
        Expr::from_node(&parser::parse_with(latex, &options).unwrap().expression, "x").unwrap()
    }

    fn solve_with(latex: &str, settings: &Settings, domain: &Domain) -> Solved {
        let ctx = SolveContext { settings, var: "x", domain, window: (-7.0, 7.0), digits: 6 };
        solve_zero(&expr(latex), &ctx)
    }

    fn solve(latex: &str) -> Solved {
        solve_with(latex, &Settings::default(), &Domain::reals())
    }

    fn roots(latex: &str) -> Vec<String> {
        solve(latex).solutions.roots.iter().map(|r| r.point.to_latex(6)).collect()
    }

    fn rules(solved: &Solved) -> Vec<String> {
        solved.steps.iter().map(|s| s.rule.clone().unwrap()).collect()
    }

    #[test]
    fn polynomials_by_school_methods() {
        assert_eq!(roots("x^3-3x"), ["-\\sqrt{3}", "0", "\\sqrt{3}"]);
        assert_eq!(roots("2x - 3"), ["\\frac{3}{2}"]);
        assert_eq!(roots("x^2 - 2x - 3"), ["-1", "3"]);
        assert_eq!(roots("x^2 - 2x - 2"), ["1 - \\sqrt{3}", "1 + \\sqrt{3}"]);
        assert_eq!(roots("x^2 + 1"), Vec::<String>::new());
        assert_eq!(roots("x^2 - 4x + 1"), ["2 - \\sqrt{3}", "2 + \\sqrt{3}"]);
        let double = solve("x^2 - 2x + 1");
        assert_eq!(double.solutions.roots[0].multiplicity, Some(2));
    }

    #[test]
    fn factoring_out_substitution_and_guessing() {
        let quartic = solve("x^4 - 2x^2");
        let found: Vec<_> = quartic.solutions.roots.iter().map(|r| (r.point.to_latex(6), r.multiplicity)).collect();
        assert_eq!(found, [("-\\sqrt{2}".into(), Some(1)), ("0".into(), Some(2)), ("\\sqrt{2}".into(), Some(1))]);
        assert_eq!(rules(&quartic)[0], "FACTOR_OUT");

        let biquadratic = solve("x^4 - 5x^2 + 4");
        assert_eq!(biquadratic.solutions.roots.iter().map(|r| r.point.to_latex(6)).collect::<Vec<_>>(), ["-2", "-1", "1", "2"]);
        assert_eq!(rules(&biquadratic)[0], "SUBSTITUTION");

        let cubic = solve("x^3-6x^2+11x-6");
        assert_eq!(cubic.solutions.roots.iter().map(|r| r.point.to_latex(6)).collect::<Vec<_>>(), ["1", "2", "3"]);
        assert_eq!(&rules(&cubic)[..2], ["RATIONAL_ROOT_GUESS", "POLYNOMIAL_DIVISION"]);
        assert!(cubic.steps[0].latex.starts_with("x \\in \\{\\pm 1, \\pm 2, \\pm 3, \\pm 6\\}"));
    }

    #[test]
    fn the_quadratic_formula_follows_the_setting() {
        let pq = Settings { quadratic_formula: QuadraticFormula::Pq, ..Settings::default() };
        let solved = solve_with("x^2 - 2x - 3", &pq, &Domain::reals());
        assert_eq!(solved.steps[0].rule.as_deref(), Some("QUADRATIC_PQ"));
        assert_eq!(
            solved.steps[0].latex,
            "x_{1,2} = -\\frac{p}{2} \\pm \\sqrt{\\left(\\frac{p}{2}\\right)^{2} - q} = 1 \\pm \\sqrt{4}"
        );
        assert_eq!(solved.steps[0].explanation.as_deref(), Some("pq formula with p = -2, q = -3"));
        let abc = solve("2x^2 - 4x - 6");
        assert_eq!(abc.steps[0].rule.as_deref(), Some("QUADRATIC_ABC"));
        assert!(abc.steps[0].latex.ends_with("= \\frac{4 \\pm \\sqrt{64}}{4}"));
        let german = Settings { language: Language::De, ..Settings::default() };
        assert_eq!(solve_with("x^2 - 2x - 3", &german, &Domain::reals()).steps[0].rule.as_deref(), Some("QUADRATIC_PQ"));
    }

    #[test]
    fn products_and_fractions() {
        assert_eq!(roots("x e^x"), ["0"]);
        let product = solve("x e^x");
        assert!(product.steps.iter().any(|s| s.latex == "e^{x} > 0"));
        assert_eq!(roots("\\frac{x^2-1}{x-2}"), ["-1", "1"]);
        assert_eq!(roots("(x - 1)^2 (x + 2)"), ["-2", "1"]);
        assert_eq!(roots("\\frac{1}{x}"), Vec::<String>::new());
    }

    #[test]
    fn one_outer_function_is_inverted() {
        assert_eq!(roots("e^x - 2"), ["\\ln(2)"]);
        assert_eq!(roots("\\ln x"), ["1"]);
        assert_eq!(roots("\\ln(x) - 1"), ["e"]);
        assert_eq!(roots("1 - \\ln(x)"), ["e"]);
        assert_eq!(roots("2\\ln(x) - 3"), ["e^{\\frac{3}{2}}"]);
        assert_eq!(roots("\\sqrt{x} - 3"), ["9"]);
        assert_eq!(roots("e^x + 1"), Vec::<String>::new());
        assert_eq!(roots("2^x - 8"), ["3"]);
    }

    #[test]
    fn trig_equations_have_periodic_solutions() {
        let sine = solve("\\sin x");
        let families: Vec<Vec<String>> = sine.solutions.families.iter().map(PeriodicSet::to_latex).collect();
        assert_eq!(families, [vec!["k\\pi".to_string()]]);
        let cosine = solve("\\cos x");
        assert_eq!(cosine.solutions.families.iter().flat_map(PeriodicSet::to_latex).collect::<Vec<_>>(), ["\\frac{\\pi}{2} + k\\pi"]);
        let shifted = solve("\\sin(2x) - \\frac{1}{2}");
        assert_eq!(shifted.solutions.families.iter().flat_map(PeriodicSet::to_latex).collect::<Vec<_>>(), ["\\frac{\\pi}{12} + k\\pi", "\\frac{5\\pi}{12} + k\\pi"]);
        assert_eq!(solve("\\tan x - 1").solutions.families.iter().flat_map(PeriodicSet::to_latex).collect::<Vec<_>>(), ["\\frac{\\pi}{4} + k\\pi"]);
        assert!(solve("\\sin(x) - 2").solutions.is_empty());
    }

    #[test]
    fn what_no_method_solves_is_found_numerically() {
        let mixed = solve("e^x + x");
        assert!(!mixed.complete);
        assert_eq!(mixed.solutions.roots.len(), 1);
        assert!((mixed.solutions.roots[0].point.approx + 0.5671432904097838).abs() < 1e-12);
        let quintic = solve("x^5 - x - 1");
        assert!(quintic.complete);
        assert_eq!(quintic.solutions.roots[0].point.to_latex(6), "1{,}1673");
        let touching = solve("(x + e^x)^2");
        assert_eq!(touching.solutions.roots.len(), 1);
        assert!((touching.solutions.roots[0].point.approx + 0.5671432904097838).abs() < 1e-6);
        assert_eq!(roots("x - e"), ["e"]);
    }

    #[test]
    fn solutions_outside_the_domain_are_dropped() {
        let positive = Domain { set: IntervalSet(vec![Interval { lo: Bound::At { point: Point::rational(RBig::ZERO), closed: false }, hi: Bound::PosInf }]), excluded: vec![], holes: vec![] };
        let solved = solve_with("x^2 - 1", &Settings::default(), &positive);
        assert_eq!(solved.solutions.roots.len(), 1);
        assert_eq!(solved.steps.last().unwrap().latex, "x = -1 \\notin D");
    }
}
