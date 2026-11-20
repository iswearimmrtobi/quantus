//! The tasks beside the curve sketch: tangent and normal at a point, the area between the graph
//! and the x-axis, and a table of values.

use dashu::rational::RBig;

use super::budget;
use super::derive::derive_chain;
use super::domain::{function_domain, DomainContext};
use super::function::Function;
use super::expr::Expr;
use super::integrate::{antiderivative, IntegrateContext};
use super::interval::{Domain, Point};
use super::rules::{put_in, step, written_out, AnalysisRule};
use super::simplify::{display, is_zero, simplify};
use super::solve::{solve_zero, SolveContext};
use crate::eval::Step;
use crate::latex::parser;
use crate::num::{Decimal, Value};
use crate::settings::{Mode, Settings, StepDetail};
use crate::text::{msg, Scope};
use crate::{QuantusError, Result};

/// Most rows a value table may have.
pub const MAX_TABLE_ROWS: usize = 201;

/// A straight line: `y = mx + b`, or `x = c` when it is vertical.
#[derive(Clone, Debug)]
pub enum Line {
    Sloped { slope: Expr, intercept: Expr, latex: String },
    Vertical { x: Point, latex: String },
}

#[derive(Debug)]
pub struct TangentResult {
    pub steps: Vec<Step>,
    pub x: Point,
    pub y: Point,
    pub tangent: Line,
    pub normal: Line,
}

#[derive(Debug)]
pub struct Region {
    pub from: f64,
    pub to: f64,
    pub positive: bool,
}

#[derive(Debug)]
pub struct AreaResult {
    pub steps: Vec<Step>,
    pub area: Point,
    /// The integral with its sign, parts below the axis counting negative.
    pub signed: Point,
    pub regions: Vec<Region>,
}

#[derive(Debug)]
pub struct TableRow {
    pub x: String,
    /// `None` outside the domain.
    pub y: Option<String>,
    pub x_value: f64,
    pub y_value: Option<f64>,
}

/// The function of a task, read and with its domain.
pub(super) struct Prepared {
    /// The function simplified, piece by piece.
    pub function: Function,
    pub var: String,
    /// Where the function is defined: the union of the pieces' domains.
    pub domain: Domain,
    /// The domain of each piece.
    pub pieces: Vec<Domain>,
    pub digits: usize,
}

pub(super) fn prepare(latex: &str, variable: Option<&str>, given: Option<&str>, settings: &Settings, precision: Option<usize>) -> Result<Prepared> {
    let (function, var) = super::input::parse_function(latex, variable, given, settings)?;
    let digits = super::digits(settings, precision);
    let (domain, _, pieces) = function_domain(&function, &DomainContext { settings, var: &var, digits })?;
    Ok(Prepared { function: function.simplified(), var, domain, pieces, digits })
}

impl Prepared {
    /// The index of the piece that holds x.
    fn piece_at(&self, x: f64) -> Option<usize> {
        self.function.pieces.iter().zip(&self.pieces).position(|(piece, domain)| piece.on.contains(x) && domain.contains(x))
    }
}

/// Where two curves meet.
#[derive(Debug)]
pub struct IntersectionResult {
    pub steps: Vec<Step>,
    /// The points, sorted by x.
    pub points: Vec<(Point, Point)>,
    /// The points of periodic families, written generally: `x = \frac{\pi}{4} + k\pi`.
    pub families: Vec<String>,
}

/// The intersections of f and g: f(x) = g(x) as f(x) − g(x) = 0 on each part where both are
/// given, solved like a zero, with y from f. `names` are what f and g are called.
pub fn intersections(f: &str, g: &str, names: (&str, &str), settings: &Settings, precision: Option<usize>) -> Result<IntersectionResult> {
    let _language = Scope::new(settings.language);
    let _budget = budget::Guard::new(super::TASK_LIMIT);
    let p = prepare(f, None, None, settings, precision)?;
    let q = prepare(g, Some(p.var.as_str()), None, settings, precision)?;
    let var = p.var.as_str();
    let d = p.digits;
    let (fname, gname) = names;
    let mut steps = Vec::new();
    let mut points: Vec<(Point, Point)> = Vec::new();
    let mut families = Vec::new();
    let several = p.function.pieces.len() * q.function.pieces.len() > 1;
    let pairs = p.function.pieces.iter().zip(&p.pieces).flat_map(|a| q.function.pieces.iter().zip(&q.pieces).map(move |b| (a, b)));
    for ((a, a_domain), (b, b_domain)) in pairs {
        let fi = &a.expr;
        let gi = b;
        // Where both are given and defined.
        let mut both = a_domain.clone();
        both.set = a_domain.set.intersect(&b_domain.set).intersect(&a.on).intersect(&b.on);
        both.excluded.extend(b_domain.excluded.iter().cloned());
        if both.set.is_empty() {
            continue;
        }
        let difference = Expr::sum(vec![a.expr.clone(), Expr::negate(b.expr.clone())]);
        let simplified = simplify(&difference);
        let equation = format!(
            "{fname}({var}) = {gname}({var}) \\Leftrightarrow {} = {} \\Leftrightarrow {} = 0",
            fi.to_latex(var),
            gi.expr.to_latex(var),
            display(&simplified, settings).to_latex(var)
        );
        if several {
            let condition = super::function::condition_latex(&both.set, var, d, settings.language);
            steps.push(step(AnalysisRule::Piecewise, msg!("core.analysis.tasks.where_both"), condition));
        }
        steps.push(step(AnalysisRule::Intersection, msg!("core.analysis.tasks.set_equal"), equation));
        let ctx = SolveContext { settings, var, domain: &both, window: (-10.0, 10.0), digits: d };
        let solved = solve_zero(&simplified, &ctx);
        steps.extend(solved.steps);
        let mut found: Vec<Point> = solved.solutions.roots.into_iter().map(|r| r.point).collect();
        for family in &solved.solutions.families {
            if both.set.is_reals() {
                for general in family.to_latex() {
                    families.push(format!("{var} = {general}, \\ k \\in \\mathbb{{Z}}"));
                }
            }
            found.extend(family.points_in(-10.0, 10.0, MAX_TABLE_ROWS));
        }
        if solved.solutions.all {
            steps.push(step(AnalysisRule::Intersection, msg!("core.analysis.tasks.curves_equal"), String::new()));
        }
        for x in found.into_iter().filter(|x| both.contains(x.approx)) {
            let y = match &x.exact {
                Some(at) => Point::exact(simplify(&fi.substitute(at))),
                None => fi.eval_f64(x.approx).map(Point::approx),
            };
            if let Some(y) = y {
                if !points.iter().any(|(px, _)| px.same(&x)) {
                    points.push((x, y));
                }
            }
        }
    }
    points.sort_by(|a, b| a.0.approx.partial_cmp(&b.0.approx).unwrap_or(std::cmp::Ordering::Equal));
    for (x, y) in &points {
        if let Some(at) = &x.exact {
            let latex = format!("{fname}({}) = {} = {}", x.to_latex(d), put_in(&p.function.piece_at(x.approx).map_or(p.function.pieces[0].expr.clone(), |pc| pc.expr.clone()), at, var), y.to_latex(d));
            steps.push(step(AnalysisRule::Simplify, msg!("core.analysis.discussion.y_value"), latex).with_level(StepDetail::Standard));
        }
    }
    Ok(IntersectionResult { steps, points, families })
}

/// A number typed by the user: `2`, `\frac{\pi}{4}`, `-\sqrt{2}`.
fn parse_number(latex: &str, settings: &Settings, what: &str) -> Result<Point> {
    let options = Settings { mode: Mode::Algebra, ..settings.clone() }.parse_options();
    let node = parser::parse_with(latex, &options)?.expression;
    let not_a_number = || QuantusError::other(msg!("core.analysis.tasks.must_number", what));
    let e = Expr::from_node(&node, "\u{0}").map_err(|_| not_a_number())?;
    if !e.is_constant() {
        return Err(not_a_number());
    }
    Point::exact(simplify(&e)).ok_or_else(|| QuantusError::other(msg!("core.analysis.tasks.not_real_number", what)))
}

/// Tangent and normal to the graph at `x0`.
pub fn tangent(latex: &str, variable: Option<&str>, given: Option<&str>, x0: &str, settings: &Settings, precision: Option<usize>) -> Result<TangentResult> {
    let _language = Scope::new(settings.language);
    let _budget = budget::Guard::new(super::TASK_LIMIT);
    let p = prepare(latex, variable, given, settings, precision)?;
    let var = p.var.as_str();
    let tex = |e: &Expr| e.to_latex(var);
    let x = parse_number(x0, settings, "x_0")?;
    let not_differentiable = || {
        QuantusError::other(msg!(
            "core.analysis.tasks.not_differentiable",
            x.to_latex(p.digits)
        ))
    };
    let Some(index) = p.piece_at(x.approx) else { return Err(not_differentiable()) };
    let f = &p.function.pieces[index].expr;
    let at = x.exact.clone().unwrap();
    let y = Point::exact(simplify(&f.substitute(&at))).ok_or_else(not_differentiable)?;
    let f1 = derive_chain(f, 1, var, settings).remove(0).result;
    // At a join the piece on the other side must meet f with the same value and slope.
    if p.function.joins().iter().any(|j| j.same(&x)) {
        let slope = Point::exact(simplify(&f1.substitute(&at))).ok_or_else(not_differentiable)?;
        for (i, other) in p.function.pieces.iter().enumerate() {
            let touches = super::function::finite_bounds(&other.on).iter().any(|b| b.same(&x));
            if i == index || !touches {
                continue;
            }
            let other_slope = derive_chain(&other.expr, 1, var, settings).remove(0).result;
            let value = Point::exact(simplify(&other.expr.substitute(&at)));
            let m = Point::exact(simplify(&other_slope.substitute(&at)));
            if !matches!((value, m), (Some(v), Some(m)) if v.same(&y) && m.same(&slope)) {
                return Err(not_differentiable());
            }
        }
    }
    // Checked in floating point first: simplifying x/|x| at 0 would fold 0·0^{-1} to 0.
    if f1.eval_f64(x.approx).is_none() {
        return Err(not_differentiable());
    }
    let m = Point::exact(simplify(&f1.substitute(&at))).ok_or_else(not_differentiable)?;
    let (x_tex, y_tex, m_tex) = (x.to_latex(p.digits), y.to_latex(p.digits), m.to_latex(p.digits));
    let point = msg!("core.analysis.tasks.point_graph");
    let slope = msg!("core.analysis.tasks.slope_is_derivative");
    let mut steps = Vec::new();
    steps.extend(written_out(
        step(AnalysisRule::Tangent, point.clone(), format!("f({x_tex}) = {y_tex}")),
        step(AnalysisRule::Tangent, point, format!("f({x_tex}) = {} = {y_tex}", put_in(f, &at, var))),
    ));
    steps.extend(written_out(
        step(AnalysisRule::Tangent, slope.clone(), format!("f'({var}) = {}, \\quad m = f'({x_tex}) = {m_tex}", tex(&f1))),
        step(AnalysisRule::Tangent, slope, format!("f'({var}) = {}, \\quad m = f'({x_tex}) = {} = {m_tex}", tex(&f1), put_in(&f1, &at, var))),
    ));
    let line = |slope: &Expr| {
        let shifted = Expr::Sum(vec![Expr::Var, Expr::negate(at.clone())]);
        let raw = Expr::Sum(vec![Expr::Product(vec![slope.clone(), shifted]), y.exact.clone().unwrap()]);
        let simplified = simplify(&raw);
        let intercept = simplify(&simplified.substitute(&Expr::int(0)));
        (simplified, intercept)
    };
    let m_exact = m.exact.clone().unwrap();
    let (t, t_intercept) = line(&m_exact);
    steps.push(step(
        AnalysisRule::Tangent,
        msg!("core.analysis.tasks.tangent_equation"),
        format!("t({var}) = {m_tex} \\cdot ({var} - {}) + {}", bracket(&x_tex), bracket(&y_tex)),
    ));
    let simplified = format!("t({var}) = {}", tex(&display(&t, settings)));
    let raw = Expr::Sum(vec![Expr::Product(vec![m_exact.clone(), Expr::Sum(vec![Expr::Var, Expr::negate(at.clone())])]), y.exact.clone().unwrap()]);
    steps.extend(super::simplify::working(&raw, var, &format!("t({var}) = "), &simplified, settings));
    steps.push(step(AnalysisRule::Simplify, msg!("core.analysis.tasks.simplify"), simplified));
    let tangent = Line::Sloped { slope: m_exact.clone(), intercept: t_intercept, latex: format!("t({var}) = {}", tex(&display(&t, settings))) };
    let normal = if is_zero(&m_exact) {
        let latex = format!("{var} = {x_tex}");
        steps.push(step(AnalysisRule::Normal, msg!("core.analysis.tasks.tangent_horizontal_normal_vertical"), latex.clone()));
        Line::Vertical { x: x.clone(), latex }
    } else {
        let slope = simplify(&Expr::negate(Expr::recip(m_exact)));
        let (n, n_intercept) = line(&slope);
        let shown = tex(&display(&n, settings));
        let slope_tex = Point::exact(slope.clone()).map(|s| s.to_latex(p.digits)).unwrap_or_else(|| tex(&slope));
        let latex = format!("-\\frac{{1}}{{m}} = -\\frac{{1}}{{{m_tex}}} = {slope_tex}");
        steps.push(step(AnalysisRule::Normal, msg!("core.analysis.tasks.slope_normal"), latex).with_level(StepDetail::Standard));
        steps.push(step(
            AnalysisRule::Normal,
            msg!("core.analysis.tasks.normal_slope"),
            format!("n({var}) = -\\frac{{1}}{{{m_tex}}} \\cdot ({var} - {}) + {} = {shown}", bracket(&x_tex), bracket(&y_tex)),
        ));
        Line::Sloped { slope, intercept: n_intercept, latex: format!("n({var}) = {shown}") }
    };
    Ok(TangentResult { steps, x, y, tangent, normal })
}

fn bracket(text: &str) -> String {
    if text.starts_with('-') { format!("({text})") } else { text.to_string() }
}

/// The area between the graph and the x-axis over [from, to], or between the outermost zeros.
pub fn area(latex: &str, variable: Option<&str>, given: Option<&str>, from: Option<&str>, to: Option<&str>, settings: &Settings, precision: Option<usize>) -> Result<AreaResult> {
    let _language = Scope::new(settings.language);
    let _budget = budget::Guard::new(super::TASK_LIMIT);
    let p = prepare(latex, variable, given, settings, precision)?;
    let var = p.var.as_str();
    let improper = || QuantusError::other(msg!("core.analysis.tasks.improper_integral"));
    let enter_bounds = || QuantusError::other(msg!("core.analysis.tasks.please_enter_bounds"));
    let infinite = |bound: Option<&str>| bound.is_some_and(|b| b.contains("\\infty") || b.contains("\\infin") || b.contains('∞'));
    if infinite(from) || infinite(to) {
        return Err(improper());
    }
    let bounds = match (from, to) {
        (Some(from), Some(to)) => {
            let (a, b) = (parse_number(from, settings, "a")?, parse_number(to, settings, "b")?);
            Some(if a.approx <= b.approx { (a, b) } else { (b, a) })
        }
        (None, None) => None,
        _ => return Err(enter_bounds()),
    };
    // The zeros on [a, b] itself, or all of them when the bounds are to be the outermost ones,
    // piece by piece.
    let window = bounds.as_ref().map_or((-10.0, 10.0), |(a, b)| (a.approx, b.approx));
    let mut zeros: Vec<Point> = Vec::new();
    let mut families = false;
    for (piece, domain) in p.function.pieces.iter().zip(&p.pieces) {
        let ctx = SolveContext { settings, var, domain, window, digits: p.digits };
        let solved = solve_zero(&piece.expr, &ctx);
        zeros.extend(solved.solutions.roots.into_iter().map(|r| r.point).filter(|z| domain.contains(z.approx)));
        families |= !solved.solutions.families.is_empty();
        if bounds.is_some() {
            for family in &solved.solutions.families {
                zeros.extend(family.points_in(window.0, window.1, MAX_TABLE_ROWS).into_iter().filter(|z| domain.contains(z.approx)));
            }
        }
    }
    zeros.sort_by(|a, b| a.approx.partial_cmp(&b.approx).unwrap_or(std::cmp::Ordering::Equal));
    zeros.dedup_by(|a, b| a.same(b));
    let (a, b) = match bounds {
        Some(bounds) => bounds,
        // Infinitely many zeros have no outermost ones.
        None if zeros.len() >= 2 && !families => (zeros[0].clone(), zeros[zeros.len() - 1].clone()),
        None => return Err(enter_bounds()),
    };
    if !covers(&p.domain, a.approx, b.approx) {
        return Err(improper());
    }
    let inside = |z: &&Point| z.approx > a.approx + 1e-12 && z.approx < b.approx - 1e-12;
    let joins: Vec<Point> = p.function.joins().into_iter().filter(|j| inside(&j)).collect();
    let mut cuts = vec![a.clone()];
    cuts.extend(zeros.iter().filter(inside).cloned());
    cuts.extend(joins.iter().cloned());
    cuts.push(b.clone());
    cuts.sort_by(|a, b| a.approx.partial_cmp(&b.approx).unwrap_or(std::cmp::Ordering::Equal));
    cuts.dedup_by(|a, b| a.same(b));
    let d = p.digits;
    let mut steps = Vec::new();
    let inner_zeros: Vec<String> = zeros.iter().filter(inside).map(|z| z.to_latex(d)).collect();
    if !inner_zeros.is_empty() {
        steps.push(step(AnalysisRule::Area, msg!("core.analysis.tasks.zeros_inside_interval_split"), format!("{var} = {}", inner_zeros.join(", \\ "))));
    }
    if !joins.is_empty() {
        let listed: Vec<String> = joins.iter().map(|j| j.to_latex(d)).collect();
        steps.push(step(AnalysisRule::Area, msg!("core.analysis.tasks.joins_split"), format!("{var} = {}", listed.join(", \\ "))));
    }
    // Each part is integrated with the piece that holds it, which must be defined on all of it.
    let piecewise = p.function.is_piecewise();
    let mut primitives: Vec<Option<Option<Expr>>> = vec![None; p.function.pieces.len()];
    let mut parts: Vec<Point> = Vec::new();
    let mut regions = Vec::new();
    for (i, pair) in cuts.windows(2).enumerate() {
        let (lo, hi) = (&pair[0], &pair[1]);
        let index = p.piece_at((lo.approx + hi.approx) / 2.0).ok_or_else(improper)?;
        let f = &p.function.pieces[index].expr;
        if piecewise {
            let (own, _) = super::domain::domain(f, &DomainContext { settings, var, digits: d })?;
            if !covers(&own, lo.approx, hi.approx) {
                return Err(improper());
            }
        }
        let primitive = primitives[index]
            .get_or_insert_with(|| {
                let found = antiderivative(f, &IntegrateContext { settings, var, domain: &p.pieces[index] }).map(|i| i.result);
                if let Some(big_f) = &found {
                    let name = if piecewise { format!("F_{{{}}}", index + 1) } else { "F".to_string() };
                    steps.push(step(AnalysisRule::Area, msg!("core.analysis.tasks.antiderivative"), format!("{name}({var}) = {}", big_f.to_latex(var))));
                }
                found
            })
            .clone();
        let value = match &primitive {
            Some(big_f) => exact_integral(big_f, lo, hi).or_else(|| numeric_integral(f, lo, hi, d)),
            None => numeric_integral(f, lo, hi, d),
        }
        .ok_or_else(|| QuantusError::other(msg!("core.analysis.tasks.integral_could_not_computed")))?;
        let big = if piecewise { format!("F_{{{}}}", index + 1) } else { "F".to_string() };
        let bounds = format!("\\int_{{{}}}^{{{}}} f({var}) \\, d{var}", lo.to_latex(d), hi.to_latex(d));
        let shown = if value.exact.is_some() { value.to_latex(d) } else { format!("\\approx {}", value.to_latex(d)) };
        let latex = match &primitive {
            Some(_) if value.exact.is_some() => format!("A_{{{}}} = {bounds} = \\left[{big}({var})\\right]_{{{}}}^{{{}}} = {shown}", i + 1, lo.to_latex(d), hi.to_latex(d)),
            _ => format!("A_{{{}}} = {bounds} = {shown}", i + 1),
        };
        // Upper minus lower bound, put into F.
        if let (Some(big_f), Some(lo_exact), Some(hi_exact), Some(_)) = (&primitive, &lo.exact, &hi.exact, &value.exact) {
            let put = |at: &Expr| format!("\\left({}\\right)", put_in(big_f, at, var));
            let latex = format!("A_{{{}}} = {big}({}) - {big}({}) = {} - {}", i + 1, hi.to_latex(d), lo.to_latex(d), put(hi_exact), put(lo_exact));
            steps.push(step(AnalysisRule::Area, msg!("core.analysis.tasks.upper_minus_lower_bound"), latex).with_level(StepDetail::Standard));
        }
        steps.push(step(AnalysisRule::Area, msg!("core.analysis.tasks.integral_over_part"), latex));
        regions.push(Region { from: lo.approx, to: hi.approx, positive: value.approx >= 0.0 });
        parts.push(value);
    }
    let signed = sum(&parts, false);
    let area = sum(&parts, true);
    let names: Vec<String> = (1..=parts.len()).map(|i| format!("\\left|A_{{{i}}}\\right|")).collect();
    let area_shown = if area.exact.is_some() { area.to_latex(d) } else { format!("\\approx {}", area.to_latex(d)) };
    let decimal = if area.exact.is_some() && !area.is_rational() { format!(" \\approx {}", area.decimal_latex(d)) } else { String::new() };
    steps.push(step(AnalysisRule::Area, msg!("core.analysis.tasks.parts_below_axis"), format!("A = {} = {area_shown}{decimal}", names.join(" + "))));
    // The signed integral, where parts below the axis count negatively.
    let signed_shown = if signed.exact.is_some() { signed.to_latex(d) } else { format!("\\approx {}", signed.to_latex(d)) };
    let parts_sum: Vec<String> = (1..=parts.len()).map(|i| format!("A_{{{i}}}")).collect();
    steps.push(step(
        AnalysisRule::Area,
        msg!("core.analysis.tasks.signed_integral"),
        format!("\\int_{{{}}}^{{{}}} f({var}) \\, d{var} = {} = {signed_shown}", a.to_latex(d), b.to_latex(d), parts_sum.join(" + ")),
    ));
    Ok(AreaResult { steps, area, signed, regions })
}

/// Whether [a, b] lies in one interval of D with no gap inside.
fn covers(domain: &Domain, a: f64, b: f64) -> bool {
    let inside = domain.set.0.iter().any(|i| i.contains(a) && i.contains(b));
    let gap = domain.excluded.iter().any(|family| !family.points_in(a, b, 1).is_empty())
        || domain.holes.iter().any(|h| h.approx >= a && h.approx <= b);
    inside && !gap && a.is_finite() && b.is_finite()
}

fn exact_integral(big_f: &Expr, lo: &Point, hi: &Point) -> Option<Point> {
    let (a, b) = (lo.exact.clone()?, hi.exact.clone()?);
    Point::exact(simplify(&Expr::Sum(vec![big_f.substitute(&b), Expr::negate(big_f.substitute(&a))])))
}

fn numeric_integral(f: &Expr, lo: &Point, hi: &Point, digits: usize) -> Option<Point> {
    let working = digits + 5;
    let decimal = |p: &Point| -> Option<Decimal> {
        match &p.exact {
            Some(e) => e.eval_exact(&Value::zero(), working).ok()?.decimal(working).ok(),
            None => crate::num::parse_float_text(&format!("{:.17e}", p.approx), 17),
        }
    };
    let (a, b) = (decimal(lo)?, decimal(hi)?);
    let mut integrand = |x: &Decimal| -> Result<Decimal> { f.eval_exact(&Value::from_decimal(x), working)?.decimal(working) };
    let value = crate::numeric::integrate(&a, &b, working.min(30), &mut integrand).ok()?;
    let r = value.to_rational();
    Some(Point { exact: None, approx: r.to_f64().value(), refined: Some(r) })
}

/// The sum of the parts, each by its absolute value if asked.
fn sum(parts: &[Point], absolute: bool) -> Point {
    let sign = |p: &Point| if absolute && p.approx < 0.0 { -1 } else { 1 };
    if parts.iter().all(|p| p.exact.is_some()) {
        let total = Expr::Sum(parts.iter().map(|p| Expr::Product(vec![Expr::int(sign(p)), p.exact.clone().unwrap()])).collect());
        if let Some(point) = Point::exact(simplify(&total)) {
            return point;
        }
    }
    let refined: Option<RBig> = parts
        .iter()
        .map(|p| p.refined.clone().or_else(|| p.exact.as_ref().and_then(|e| e.eval_exact(&Value::zero(), 30).ok()?.decimal(30).ok().map(|d| d.to_rational()))))
        .collect::<Option<Vec<_>>>()
        .map(|values| values.into_iter().zip(parts).map(|(v, p)| if sign(p) < 0 { -v } else { v }).fold(RBig::ZERO, |s, v| s + v));
    let approx: f64 = parts.iter().map(|p| p.approx * sign(p) as f64).sum();
    Point { exact: None, approx, refined }
}

/// Whether a table value reads best exactly: a rational, or a value with roots and π that
/// `latex::exact` writes in closed form (`\sqrt{2}`), not one with ln, e or trig left in it.
fn closed_form(y: &Point, f: &Expr, x: &Expr) -> bool {
    if y.is_rational() {
        return true;
    }
    let Some(value) = x.eval_exact(&Value::zero(), 30).ok().and_then(|x| f.eval_exact(&x, 30).ok()) else { return false };
    crate::latex::exact::format(&value).is_some()
}

/// f at `from`, `from + step`, … up to `to`.
#[allow(clippy::too_many_arguments, reason = "the fields of a table request")]
pub fn value_table(latex: &str, variable: Option<&str>, given: Option<&str>, from: &str, to: &str, step_size: &str, settings: &Settings, precision: Option<usize>) -> Result<Vec<TableRow>> {
    let _language = Scope::new(settings.language);
    let _budget = budget::Guard::new(super::TASK_LIMIT);
    let p = prepare(latex, variable, given, settings, precision)?;
    let (start, end, step) = (parse_number(from, settings, "from")?, parse_number(to, settings, "to")?, parse_number(step_size, settings, "step")?);
    if step.approx <= 0.0 {
        return Err(QuantusError::other(msg!("core.analysis.tasks.step_must_positive")));
    }
    let count = ((end.approx - start.approx) / step.approx + 1e-9).floor();
    if count < 0.0 || count as usize + 1 > MAX_TABLE_ROWS {
        return Err(QuantusError::other(msg!(
            "core.analysis.tasks.table_has_most_rows",
            MAX_TABLE_ROWS
        )));
    }
    let mut rows = Vec::new();
    for i in 0..=count as i64 {
        let x = simplify(&Expr::Sum(vec![start.exact.clone().unwrap(), Expr::Product(vec![Expr::int(i), step.exact.clone().unwrap()])]));
        let point = Point::exact(x.clone()).unwrap();
        let piece = p.piece_at(point.approx).map(|i| &p.function.pieces[i].expr);
        let y = piece.and_then(|f| Point::exact(simplify(&f.substitute(&x))));
        let y_text = y.as_ref().zip(piece).map(|(y, f)| if closed_form(y, f, &x) { y.to_latex(p.digits) } else { y.decimal_latex(p.digits) });
        rows.push(TableRow { x: point.to_latex(p.digits), y: y_text, x_value: point.approx, y_value: y.map(|y| y.approx) });
    }
    let _ = p.var;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(l: &Line) -> &str {
        match l {
            Line::Sloped { latex, .. } | Line::Vertical { latex, .. } => latex,
        }
    }

    #[test]
    fn tangent_and_normal() {
        let r = tangent("x^2", None, None, "1", &Settings::default(), None).unwrap();
        assert_eq!(line(&r.tangent), "t(x) = 2x - 1");
        assert_eq!(line(&r.normal), "n(x) = -\\frac{1}{2}x + \\frac{3}{2}");
        let brief = crate::eval::calculator::shown_steps(&r.steps, StepDetail::Brief);
        assert_eq!(brief.len(), 5);
        assert_eq!(brief[2].latex, "t(x) = 2 \\cdot (x - 1) + 1");
        assert_eq!(brief[3].latex, "t(x) = 2x - 1");
        let flat = tangent("x^2", None, None, "0", &Settings::default(), None).unwrap();
        assert_eq!(line(&flat.normal), "x = 0");
        let exact = tangent("\\sin x", None, None, "\\frac{\\pi}{2}", &Settings::default(), None).unwrap();
        assert_eq!(line(&exact.tangent), "t(x) = 1");
        assert!(tangent("\\ln x", None, None, "-1", &Settings::default(), None).unwrap_err().to_string().contains("not differentiable"));
        assert!(tangent("|x|", None, None, "0", &Settings::default(), None).is_err());
    }

    #[test]
    fn area_between_the_zeros() {
        let r = area("x^3-3x", None, None, None, None, &Settings::default(), None).unwrap();
        assert_eq!(r.area.to_latex(6), "\\frac{9}{2}");
        assert_eq!(r.signed.to_latex(6), "0");
        assert_eq!(r.regions.len(), 2);
        assert!(r.regions[0].positive && !r.regions[1].positive);
        assert_eq!(r.steps[r.steps.len() - 2].latex, "A = \\left|A_{1}\\right| + \\left|A_{2}\\right| = \\frac{9}{2}");
        assert_eq!(r.steps.last().unwrap().latex, "\\int_{-\\sqrt{3}}^{\\sqrt{3}} f(x) \\, dx = A_{1} + A_{2} = 0");
        let brief = crate::eval::calculator::shown_steps(&r.steps, StepDetail::Brief);
        let parts: Vec<String> = brief.iter().filter(|s| s.latex.starts_with("A_{")).map(|s| s.latex.rsplit(" = ").next().unwrap().to_string()).collect();
        assert_eq!(parts, ["\\frac{9}{4}", "-\\frac{9}{4}"]);
        let wide = area("x - 12", None, None, Some("0"), Some("20"), &Settings::default(), None).unwrap();
        assert_eq!(wide.regions.len(), 2);
        assert_eq!(wide.area.to_latex(6), "104");
        assert!(area("e^{-x}", None, None, Some("0"), Some("\\infty"), &Settings::default(), None).unwrap_err().to_string().contains("Improper integral"));
        let bounded = area("x^2", None, None, Some("0"), Some("3"), &Settings::default(), None).unwrap();
        assert_eq!(bounded.area.to_latex(6), "9");
        let numeric = area("e^{x^2}", None, None, Some("0"), Some("1"), &Settings::default(), None).unwrap();
        assert_eq!(numeric.area.to_latex(6), "1{,}46265");
        assert!(area("\\frac{1}{x}", None, None, Some("-1"), Some("1"), &Settings::default(), None).unwrap_err().to_string().contains("Improper integral, not supported yet"));
        assert!(area("x^2 + 1", None, None, None, None, &Settings::default(), None).unwrap_err().to_string().contains("Please enter the bounds"));
        assert!(area("\\sin x", None, None, None, None, &Settings::default(), None).unwrap_err().to_string().contains("Please enter the bounds"));
    }

    #[test]
    fn out_of_time_the_tasks_still_answer() {
        let _budget = budget::Guard::new(std::time::Duration::ZERO);
        assert!(tangent("x^2", None, None, "1", &Settings::default(), None).is_ok());
        assert!(value_table("x^2", None, None, "-2", "2", "1", &Settings::default(), None).is_ok());
    }

    #[test]
    fn a_table_of_values() {
        let rows = value_table("x^2", None, None, "-2", "2", "1", &Settings::default(), None).unwrap();
        let ys: Vec<_> = rows.iter().map(|r| (r.x.clone(), r.y.clone().unwrap())).collect();
        assert_eq!(ys[0], ("-2".into(), "4".into()));
        assert_eq!(rows.len(), 5);
        let log = value_table("\\ln x", None, None, "0", "2", "1", &Settings::default(), None).unwrap();
        assert_eq!(log[0].y, None);
        assert_eq!(log[2].y.as_deref(), Some("0{,}693147"));
        assert_eq!(log[2].y_value, Some(std::f64::consts::LN_2));
        let root = value_table("\\sqrt{x}", None, None, "0", "2", "1", &Settings::default(), None).unwrap();
        assert_eq!(root[2].y.as_deref(), Some("\\sqrt{2}"));
        assert!(value_table("x", None, None, "0", "1000", "1", &Settings::default(), None).is_err());
        assert!(value_table("x", None, None, "0", "1", "-1", &Settings::default(), None).is_err());
    }

    #[test]
    fn the_tasks_take_the_piece_that_holds_x() {
        let smooth = "\\begin{cases}x^2 & x\\le 1\\\\ 2x-1 & x>1\\end{cases}";
        let r = tangent(smooth, None, None, "2", &Settings::default(), None).unwrap();
        assert_eq!(line(&r.tangent), "t(x) = 2x - 1");
        let join = tangent(smooth, None, None, "1", &Settings::default(), None).unwrap();
        assert_eq!(line(&join.tangent), "t(x) = 2x - 1");
        let kink = "\\begin{cases}-x & x<0\\\\ x & x\\ge 0\\end{cases}";
        assert!(tangent(kink, None, None, "0", &Settings::default(), None).unwrap_err().to_string().contains("not differentiable"));
        assert!(tangent("x^2", None, Some("[0, 1]"), "2", &Settings::default(), None).is_err());

        let a = area(kink, None, None, Some("-1"), Some("2"), &Settings::default(), None).unwrap();
        assert_eq!(a.area.to_latex(6), "\\frac{5}{2}");
        assert_eq!(a.regions.len(), 2);
        assert!(a.steps.iter().any(|s| s.latex.starts_with("F_{1}(x) = ")));
        let pole = "\\begin{cases}\\frac{1}{x} & x<0\\\\ x & x\\ge 0\\end{cases}";
        assert!(area(pole, None, None, Some("-1"), Some("1"), &Settings::default(), None).unwrap_err().to_string().contains("Improper"));

        let rows = value_table(kink, None, None, "-1", "1", "1", &Settings::default(), None).unwrap();
        let ys: Vec<_> = rows.iter().map(|r| r.y.clone().unwrap()).collect();
        assert_eq!(ys, ["1", "0", "1"]);
        let gap = value_table("x^2", None, Some("x > 0"), "-1", "1", "1", &Settings::default(), None).unwrap();
        assert_eq!(gap.iter().map(|r| r.y.is_some()).collect::<Vec<_>>(), [false, false, true]);
    }

    #[test]
    fn two_curves_meet_where_their_difference_is_zero() {
        let r = intersections("x^2", "x + 2", ("f", "g"), &Settings::default(), None).unwrap();
        let xs: Vec<String> = r.points.iter().map(|(x, y)| format!("{}|{}", x.to_latex(6), y.to_latex(6))).collect();
        assert_eq!(xs, ["-1|1", "2|4"]);
        assert!(r.steps[0].latex.starts_with("f(x) = g(x) \\Leftrightarrow x^{2} = x + 2 \\Leftrightarrow "), "{}", r.steps[0].latex);
        let numeric = intersections("e^x", "x + 2", ("f", "g"), &Settings::default(), None).unwrap();
        assert_eq!(numeric.points.len(), 2);
        assert!((numeric.points[1].0.approx - 1.146193).abs() < 1e-5);
        let none = intersections("x^2 + 1", "0", ("f", "g"), &Settings::default(), None).unwrap();
        assert!(none.points.is_empty());
        let pieces = intersections("\\begin{cases}x & x<0\\\\ 1 & x\\ge 0\\end{cases}", "x^2", ("f", "g"), &Settings::default(), None).unwrap();
        let xs: Vec<f64> = pieces.points.iter().map(|(x, _)| x.approx).collect();
        assert_eq!(xs, [1.0]);
    }
}
