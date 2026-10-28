//! The tasks beside the curve sketch: tangent and normal at a point, the area between the graph
//! and the x-axis, and a table of values.

use dashu::rational::RBig;

use super::budget;
use super::derive::derive_chain;
use super::domain::{domain, DomainContext};
use super::expr::Expr;
use super::integrate::{antiderivative, IntegrateContext};
use super::interval::{Domain, Point};
use super::rules::{step, AnalysisRule};
use super::simplify::{display, is_zero, simplify};
use super::solve::{solve_zero, SolveContext};
use crate::eval::Step;
use crate::latex::parser;
use crate::num::{Decimal, Value};
use crate::settings::{Mode, Settings};
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

struct Prepared {
    f: Expr,
    var: String,
    domain: Domain,
    digits: usize,
}

fn prepare(latex: &str, variable: Option<&str>, settings: &Settings, precision: Option<usize>) -> Result<Prepared> {
    let (f, var) = super::input::parse_function(latex, variable, settings)?;
    let digits = super::digits(settings, precision);
    let (domain, _) = domain(&f, &DomainContext { settings, var: &var, digits })?;
    Ok(Prepared { f: simplify(&f), var, domain, digits })
}

/// A number typed by the user: `2`, `\frac{\pi}{4}`, `-\sqrt{2}`.
fn parse_number(latex: &str, settings: &Settings, what: &str) -> Result<Point> {
    let options = Settings { mode: Mode::Algebra, ..settings.clone() }.parse_options();
    let node = parser::parse_with(latex, &options)?.expression;
    let not_a_number = || QuantusError::other(msg!("{} must be a number.", "{} muss eine Zahl sein.", what));
    let e = Expr::from_node(&node, "\u{0}").map_err(|_| not_a_number())?;
    if !e.is_constant() {
        return Err(not_a_number());
    }
    Point::exact(simplify(&e)).ok_or_else(|| QuantusError::other(msg!("{} is not a real number.", "{} ist keine reelle Zahl.", what)))
}

/// Tangent and normal to the graph at `x0`.
pub fn tangent(latex: &str, variable: Option<&str>, x0: &str, settings: &Settings, precision: Option<usize>) -> Result<TangentResult> {
    let _language = Scope::new(settings.language);
    let _budget = budget::Guard::new(super::TASK_LIMIT);
    let p = prepare(latex, variable, settings, precision)?;
    let var = p.var.as_str();
    let tex = |e: &Expr| e.to_latex(var);
    let x = parse_number(x0, settings, "x_0")?;
    let not_differentiable = || {
        QuantusError::other(msg!(
            "f is not differentiable at x_0 = {}.",
            "f ist an der Stelle x_0 = {} nicht differenzierbar.",
            x.to_latex(p.digits)
        ))
    };
    if !p.domain.contains(x.approx) {
        return Err(not_differentiable());
    }
    let at = x.exact.clone().unwrap();
    let y = Point::exact(simplify(&p.f.substitute(&at))).ok_or_else(not_differentiable)?;
    let f1 = derive_chain(&p.f, 1, var, settings).remove(0).result;
    // Checked in floating point first: simplifying x/|x| at 0 would fold 0·0^{-1} to 0.
    if f1.eval_f64(x.approx).is_none() {
        return Err(not_differentiable());
    }
    let m = Point::exact(simplify(&f1.substitute(&at))).ok_or_else(not_differentiable)?;
    let (x_tex, y_tex, m_tex) = (x.to_latex(p.digits), y.to_latex(p.digits), m.to_latex(p.digits));
    let mut steps = vec![
        step(AnalysisRule::Tangent, msg!("Point on the graph", "Punkt auf dem Graphen"), format!("f({x_tex}) = {y_tex}")),
        step(
            AnalysisRule::Tangent,
            msg!("Slope: the derivative at x_0", "Steigung: die Ableitung an der Stelle x_0"),
            format!("f'({var}) = {}, \\quad m = f'({x_tex}) = {m_tex}", tex(&f1)),
        ),
    ];
    let line = |slope: &Expr| {
        let shifted = Expr::Sum(vec![Expr::Var, Expr::neg(at.clone())]);
        let raw = Expr::Sum(vec![Expr::Product(vec![slope.clone(), shifted]), y.exact.clone().unwrap()]);
        let simplified = simplify(&raw);
        let intercept = simplify(&simplified.substitute(&Expr::int(0)));
        (simplified, intercept)
    };
    let m_exact = m.exact.clone().unwrap();
    let (t, t_intercept) = line(&m_exact);
    steps.push(step(
        AnalysisRule::Tangent,
        msg!("Tangent: t(x) = f'(x_0)(x - x_0) + f(x_0), values put in", "Tangente: t(x) = f'(x_0)(x - x_0) + f(x_0), Werte eingesetzt"),
        format!("t({var}) = {m_tex} \\cdot ({var} - {}) + {}", bracket(&x_tex), bracket(&y_tex)),
    ));
    steps.push(step(AnalysisRule::Simplify, msg!("Simplify", "Zusammenfassen"), format!("t({var}) = {}", tex(&display(&t, settings)))));
    let tangent = Line::Sloped { slope: m_exact.clone(), intercept: t_intercept, latex: format!("t({var}) = {}", tex(&display(&t, settings))) };
    let normal = if is_zero(&m_exact) {
        let latex = format!("{var} = {x_tex}");
        steps.push(step(AnalysisRule::Normal, msg!("The tangent is horizontal, so the normal is vertical", "Die Tangente ist waagrecht, also ist die Normale senkrecht"), latex.clone()));
        Line::Vertical { x: x.clone(), latex }
    } else {
        let slope = simplify(&Expr::neg(Expr::recip(m_exact)));
        let (n, n_intercept) = line(&slope);
        let shown = tex(&display(&n, settings));
        steps.push(step(
            AnalysisRule::Normal,
            msg!("Normal: slope -1/m", "Normale: Steigung -1/m"),
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
pub fn area(latex: &str, variable: Option<&str>, from: Option<&str>, to: Option<&str>, settings: &Settings, precision: Option<usize>) -> Result<AreaResult> {
    let _language = Scope::new(settings.language);
    let _budget = budget::Guard::new(super::TASK_LIMIT);
    let p = prepare(latex, variable, settings, precision)?;
    let var = p.var.as_str();
    let improper = || QuantusError::other(msg!("Improper integral, not supported yet.", "Uneigentliches Integral, noch nicht unterstützt."));
    let enter_bounds = || QuantusError::other(msg!("Please enter the bounds.", "Bitte Grenzen angeben."));
    let infinite = |bound: Option<&str>| bound.is_some_and(|b| b.contains("\\infty") || b.contains('∞'));
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
    // The zeros on [a, b] itself, or all of them when the bounds are to be the outermost ones.
    let window = bounds.as_ref().map_or((-10.0, 10.0), |(a, b)| (a.approx, b.approx));
    let ctx = SolveContext { settings, var, domain: &p.domain, window, digits: p.digits };
    let solved = solve_zero(&p.f, &ctx);
    let mut zeros: Vec<Point> = solved.solutions.roots.into_iter().map(|r| r.point).collect();
    if bounds.is_some() {
        for family in &solved.solutions.families {
            zeros.extend(family.points_in(window.0, window.1, MAX_TABLE_ROWS));
        }
    }
    zeros.sort_by(|a, b| a.approx.partial_cmp(&b.approx).unwrap_or(std::cmp::Ordering::Equal));
    let (a, b) = match bounds {
        Some(bounds) => bounds,
        // Infinitely many zeros have no outermost ones.
        None if zeros.len() >= 2 && solved.solutions.families.is_empty() => (zeros[0].clone(), zeros[zeros.len() - 1].clone()),
        None => return Err(enter_bounds()),
    };
    if !covers(&p.domain, a.approx, b.approx) {
        return Err(improper());
    }
    let mut cuts = vec![a.clone()];
    cuts.extend(zeros.iter().filter(|z| z.approx > a.approx + 1e-12 && z.approx < b.approx - 1e-12).cloned());
    cuts.push(b.clone());
    let d = p.digits;
    let mut steps = Vec::new();
    if cuts.len() > 2 {
        let inner: Vec<String> = cuts[1..cuts.len() - 1].iter().map(|z| z.to_latex(d)).collect();
        steps.push(step(AnalysisRule::Area, msg!("Zeros inside the interval split it", "Nullstellen im Intervall teilen es auf"), format!("{var} = {}", inner.join(", \\ "))));
    }
    let primitive = antiderivative(&p.f, &IntegrateContext { settings, var, domain: &p.domain }).map(|i| i.result);
    if let Some(big_f) = &primitive {
        steps.push(step(AnalysisRule::Area, msg!("Antiderivative", "Stammfunktion"), format!("F({var}) = {}", big_f.to_latex(var))));
    }
    let mut parts: Vec<Point> = Vec::new();
    let mut regions = Vec::new();
    for (i, pair) in cuts.windows(2).enumerate() {
        let (lo, hi) = (&pair[0], &pair[1]);
        let value = match &primitive {
            Some(big_f) => exact_integral(big_f, lo, hi).or_else(|| numeric_integral(&p.f, lo, hi, d)),
            None => numeric_integral(&p.f, lo, hi, d),
        }
        .ok_or_else(|| QuantusError::other(msg!("The integral could not be computed.", "Das Integral konnte nicht berechnet werden.")))?;
        let bounds = format!("\\int_{{{}}}^{{{}}} f({var}) \\, d{var}", lo.to_latex(d), hi.to_latex(d));
        let shown = if value.exact.is_some() { value.to_latex(d) } else { format!("\\approx {}", value.to_latex(d)) };
        let latex = match &primitive {
            Some(_) if value.exact.is_some() => format!("A_{{{}}} = {bounds} = \\left[F({var})\\right]_{{{}}}^{{{}}} = {shown}", i + 1, lo.to_latex(d), hi.to_latex(d)),
            _ => format!("A_{{{}}} = {bounds} = {shown}", i + 1),
        };
        steps.push(step(AnalysisRule::Area, msg!("Integral over the part", "Integral über das Teilstück"), latex));
        regions.push(Region { from: lo.approx, to: hi.approx, positive: value.approx >= 0.0 });
        parts.push(value);
    }
    let signed = sum(&parts, false);
    let area = sum(&parts, true);
    let names: Vec<String> = (1..=parts.len()).map(|i| format!("\\left|A_{{{i}}}\\right|")).collect();
    let area_shown = if area.exact.is_some() { area.to_latex(d) } else { format!("\\approx {}", area.to_latex(d)) };
    let decimal = if area.exact.is_some() && !area.is_rational() { format!(" \\approx {}", area.decimal_latex(d)) } else { String::new() };
    steps.push(step(AnalysisRule::Area, msg!("Parts below the axis count with their absolute value", "Teile unter der Achse zählen mit ihrem Betrag"), format!("A = {} = {area_shown}{decimal}", names.join(" + "))));
    // The signed integral, where parts below the axis count negatively.
    let signed_shown = if signed.exact.is_some() { signed.to_latex(d) } else { format!("\\approx {}", signed.to_latex(d)) };
    let parts_sum: Vec<String> = (1..=parts.len()).map(|i| format!("A_{{{i}}}")).collect();
    steps.push(step(
        AnalysisRule::Area,
        msg!("The integral itself counts parts below the axis negatively", "Das Integral selbst zählt Teile unter der Achse negativ"),
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
    Point::exact(simplify(&Expr::Sum(vec![big_f.substitute(&b), Expr::neg(big_f.substitute(&a))])))
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
pub fn value_table(latex: &str, variable: Option<&str>, from: &str, to: &str, step_size: &str, settings: &Settings, precision: Option<usize>) -> Result<Vec<TableRow>> {
    let _language = Scope::new(settings.language);
    let _budget = budget::Guard::new(super::TASK_LIMIT);
    let p = prepare(latex, variable, settings, precision)?;
    let (start, end, step) = (parse_number(from, settings, "from")?, parse_number(to, settings, "to")?, parse_number(step_size, settings, "step")?);
    if step.approx <= 0.0 {
        return Err(QuantusError::other(msg!("The step must be positive.", "Die Schrittweite muss positiv sein.")));
    }
    let count = ((end.approx - start.approx) / step.approx + 1e-9).floor();
    if count < 0.0 || count as usize + 1 > MAX_TABLE_ROWS {
        return Err(QuantusError::other(msg!(
            "A table has at most {} rows.",
            "Eine Tabelle hat höchstens {} Zeilen.",
            MAX_TABLE_ROWS
        )));
    }
    let mut rows = Vec::new();
    for i in 0..=count as i64 {
        let x = simplify(&Expr::Sum(vec![start.exact.clone().unwrap(), Expr::Product(vec![Expr::int(i), step.exact.clone().unwrap()])]));
        let point = Point::exact(x.clone()).unwrap();
        let y = if p.domain.contains(point.approx) { Point::exact(simplify(&p.f.substitute(&x))) } else { None };
        let y_text = y.as_ref().map(|y| if closed_form(y, &p.f, &x) { y.to_latex(p.digits) } else { y.decimal_latex(p.digits) });
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
        let r = tangent("x^2", None, "1", &Settings::default(), None).unwrap();
        assert_eq!(line(&r.tangent), "t(x) = 2x - 1");
        assert_eq!(line(&r.normal), "n(x) = -\\frac{1}{2}x + \\frac{3}{2}");
        assert_eq!(r.steps.len(), 5);
        assert_eq!(r.steps[2].latex, "t(x) = 2 \\cdot (x - 1) + 1");
        assert_eq!(r.steps[3].latex, "t(x) = 2x - 1");
        let flat = tangent("x^2", None, "0", &Settings::default(), None).unwrap();
        assert_eq!(line(&flat.normal), "x = 0");
        let exact = tangent("\\sin x", None, "\\frac{\\pi}{2}", &Settings::default(), None).unwrap();
        assert_eq!(line(&exact.tangent), "t(x) = 1");
        assert!(tangent("\\ln x", None, "-1", &Settings::default(), None).unwrap_err().to_string().contains("not differentiable"));
        assert!(tangent("|x|", None, "0", &Settings::default(), None).is_err());
    }

    #[test]
    fn area_between_the_zeros() {
        let r = area("x^3-3x", None, None, None, &Settings::default(), None).unwrap();
        assert_eq!(r.area.to_latex(6), "\\frac{9}{2}");
        assert_eq!(r.signed.to_latex(6), "0");
        assert_eq!(r.regions.len(), 2);
        assert!(r.regions[0].positive && !r.regions[1].positive);
        assert_eq!(r.steps[r.steps.len() - 2].latex, "A = \\left|A_{1}\\right| + \\left|A_{2}\\right| = \\frac{9}{2}");
        assert_eq!(r.steps.last().unwrap().latex, "\\int_{-\\sqrt{3}}^{\\sqrt{3}} f(x) \\, dx = A_{1} + A_{2} = 0");
        let parts: Vec<String> = r.steps.iter().filter(|s| s.latex.starts_with("A_{")).map(|s| s.latex.rsplit(" = ").next().unwrap().to_string()).collect();
        assert_eq!(parts, ["\\frac{9}{4}", "-\\frac{9}{4}"]);
        let wide = area("x - 12", None, Some("0"), Some("20"), &Settings::default(), None).unwrap();
        assert_eq!(wide.regions.len(), 2);
        assert_eq!(wide.area.to_latex(6), "104");
        assert!(area("e^{-x}", None, Some("0"), Some("\\infty"), &Settings::default(), None).unwrap_err().to_string().contains("Improper integral"));
        let bounded = area("x^2", None, Some("0"), Some("3"), &Settings::default(), None).unwrap();
        assert_eq!(bounded.area.to_latex(6), "9");
        let numeric = area("e^{x^2}", None, Some("0"), Some("1"), &Settings::default(), None).unwrap();
        assert_eq!(numeric.area.to_latex(6), "1{,}46265");
        assert!(area("\\frac{1}{x}", None, Some("-1"), Some("1"), &Settings::default(), None).unwrap_err().to_string().contains("Improper integral, not supported yet"));
        assert!(area("x^2 + 1", None, None, None, &Settings::default(), None).unwrap_err().to_string().contains("Please enter the bounds"));
        assert!(area("\\sin x", None, None, None, &Settings::default(), None).unwrap_err().to_string().contains("Please enter the bounds"));
    }

    #[test]
    fn out_of_time_the_tasks_still_answer() {
        let _budget = budget::Guard::new(std::time::Duration::ZERO);
        assert!(tangent("x^2", None, "1", &Settings::default(), None).is_ok());
        assert!(value_table("x^2", None, "-2", "2", "1", &Settings::default(), None).is_ok());
    }

    #[test]
    fn a_table_of_values() {
        let rows = value_table("x^2", None, "-2", "2", "1", &Settings::default(), None).unwrap();
        let ys: Vec<_> = rows.iter().map(|r| (r.x.clone(), r.y.clone().unwrap())).collect();
        assert_eq!(ys[0], ("-2".into(), "4".into()));
        assert_eq!(rows.len(), 5);
        let log = value_table("\\ln x", None, "0", "2", "1", &Settings::default(), None).unwrap();
        assert_eq!(log[0].y, None);
        assert_eq!(log[2].y.as_deref(), Some("0{,}693147"));
        assert_eq!(log[2].y_value, Some(std::f64::consts::LN_2));
        let root = value_table("\\sqrt{x}", None, "0", "2", "1", &Settings::default(), None).unwrap();
        assert_eq!(root[2].y.as_deref(), Some("\\sqrt{2}"));
        assert!(value_table("x", None, "0", "1000", "1", &Settings::default(), None).is_err());
        assert!(value_table("x", None, "0", "1", "-1", &Settings::default(), None).is_err());
    }
}
