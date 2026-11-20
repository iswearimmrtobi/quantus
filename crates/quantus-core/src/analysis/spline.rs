//! A curve through given points: a cubic spline (natural, or with given slopes at the ends) or
//! the interpolating polynomial. The conditions are written out as a student sets them up, the
//! linear system is solved exactly, and the result is a [`Function`] of pieces the curve sketch
//! takes like any other.

use dashu::rational::RBig;

use super::expr::Expr;
use super::function::{parse_bound_number, Function, Piece};
use super::interval::{Bound, Interval, IntervalSet, Point};
use super::rules::{step, AnalysisRule};
use super::simplify::simplify;
use crate::eval::Step;
use crate::num::Value;
use crate::settings::{Language, Settings, StepDetail};
use crate::text::msg;
use crate::{QuantusError, Result};

/// Most points a spline may go through.
pub const MAX_POINTS: usize = 30;

/// How the curve ends, or that it is one polynomial.
#[derive(Clone, Debug)]
pub enum Ends {
    /// s″ = 0 at both ends.
    Natural,
    /// The slopes at the first and the last point.
    Clamped(RBig, RBig),
    /// The polynomial of the least degree through all points.
    Polynomial,
}

pub struct Spline {
    pub function: Function,
    pub steps: Vec<Step>,
    /// The given points, sorted by x.
    pub points: Vec<(RBig, RBig)>,
}

/// Whether the input is a list of points, `(0, 1), (1, 3)` or `(0|1); (1|3)`, not a function.
pub fn is_point_list(latex: &str) -> bool {
    let cleaned = clean(latex);
    let text = cleaned.trim();
    text.starts_with('(') && text.ends_with(')') && split_points(text).is_some_and(|points| points.len() >= 2 && points.iter().all(|p| split_pair(p).is_some()))
}

fn clean(latex: &str) -> String {
    let mut s = latex.to_string();
    for spacing in ["\\left", "\\right", "\\qquad", "\\quad", "\\,", "\\;", "\\ ", "~"] {
        s = s.replace(spacing, " ");
    }
    s.replace("\\mid", "|").replace("\\vert", "|")
}

/// The points' insides, `0, 1` of `(0, 1)`, or `None` if the brackets do not pair up.
fn split_points(text: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = None;
    for (i, c) in text.char_indices() {
        match c {
            '(' => {
                if depth == 0 {
                    start = Some(i + 1);
                }
                depth += 1;
            }
            ')' => {
                depth -= 1;
                if depth == 0 {
                    out.push(text[start?..i].to_string());
                }
                if depth < 0 {
                    return None;
                }
            }
            ',' | ';' | ' ' if depth == 0 => {}
            _ if depth == 0 => return None,
            _ => {}
        }
    }
    (depth == 0).then_some(out)
}

/// x and y of a point's inside, split at `|`, `;` or, without those, the comma.
fn split_pair(inside: &str) -> Option<(String, String)> {
    let separator = ['|', ';', ','].into_iter().find(|c| inside.contains(*c))?;
    let mut parts = inside.splitn(2, separator);
    let (x, y) = (parts.next()?.trim(), parts.next()?.trim());
    (!x.is_empty() && !y.is_empty() && !y.contains(separator)).then(|| (x.to_string(), y.to_string()))
}

/// The points of a list, sorted by x; at least two, no x twice, rational coordinates.
pub fn parse_points(latex: &str, settings: &Settings) -> Result<Vec<(RBig, RBig)>> {
    let not_understood = || QuantusError::other(msg!("core.analysis.spline.points_not_understood"));
    let cleaned = clean(latex);
    let insides = split_points(cleaned.trim()).ok_or_else(not_understood)?;
    let rational = |text: &str| -> Result<RBig> {
        let point = parse_bound_number(text, settings)?;
        point.exact.as_ref().and_then(|e| e.as_num().cloned()).ok_or_else(|| QuantusError::other(msg!("core.analysis.spline.not_rational", text.trim())))
    };
    let mut points = Vec::new();
    for inside in insides {
        let (x, y) = split_pair(&inside).ok_or_else(not_understood)?;
        points.push((rational(&x)?, rational(&y)?));
    }
    if points.len() < 2 {
        return Err(QuantusError::other(msg!("core.analysis.spline.too_few_points")));
    }
    if points.len() > MAX_POINTS {
        return Err(QuantusError::other(msg!("core.analysis.spline.too_many_points", MAX_POINTS)));
    }
    points.sort_by(|a, b| a.0.cmp(&b.0));
    if let Some(pair) = points.windows(2).find(|w| w[0].0 == w[1].0) {
        return Err(QuantusError::other(msg!("core.analysis.spline.same_x", Expr::Num(pair[0].0.clone()).to_latex("x"))));
    }
    Ok(points)
}

fn num(r: &RBig) -> String {
    Expr::Num(r.clone()).to_latex("x")
}

/// `c·name` as a term of a sum: `2b_{1}`, `-c_{2}`, `\frac{1}{4}d_{1}`; empty for c = 0.
fn term(first: bool, c: &RBig, name: &str) -> String {
    if *c == RBig::ZERO {
        return String::new();
    }
    let negative = *c < RBig::ZERO;
    let size = if negative { -c.clone() } else { c.clone() };
    let factor = if size == RBig::ONE { String::new() } else { num(&size) };
    let sign = match (first, negative) {
        (true, false) => "",
        (true, true) => "-",
        (false, false) => " + ",
        (false, true) => " - ",
    };
    format!("{sign}{factor}{name}")
}

/// One equation of the system: its row (coefficients and right side) and how it reads.
struct Equation {
    row: Vec<(usize, RBig)>,
    rhs: RBig,
    /// What it says before the coefficients are put in: `s_{1}(1) = 3`.
    meaning: String,
}

impl Equation {
    fn latex(&self, names: &[String]) -> String {
        let mut left = String::new();
        for (unknown, c) in &self.row {
            left.push_str(&term(left.is_empty(), c, &names[*unknown]));
        }
        if left.is_empty() {
            left.push('0');
        }
        format!("{} \\Rightarrow {left} = {}", self.meaning, num(&self.rhs))
    }
}

/// Solves the system exactly; it has one solution for distinct x.
fn solve(equations: &[Equation], unknowns: usize) -> Result<Vec<RBig>> {
    let matrix: Vec<Vec<Value>> = equations
        .iter()
        .map(|e| {
            let mut row = vec![Value::zero(); unknowns + 1];
            for (unknown, c) in &e.row {
                row[*unknown] = Value::Rat(c.clone());
            }
            row[unknowns] = Value::Rat(e.rhs.clone());
            row
        })
        .collect();
    let solution = crate::linear::solve_square(matrix, unknowns)?.ok_or_else(|| QuantusError::other(msg!("core.analysis.spline.no_solution")))?;
    solution.iter().map(|v| v.as_rational().cloned().ok_or_else(|| QuantusError::other(msg!("core.analysis.spline.no_solution")))).collect()
}

/// `(x - x_i)`, or `x` when x_i = 0, as a tree.
fn shifted(x0: &RBig) -> Expr {
    Expr::sum(vec![Expr::Var, Expr::Num(-x0.clone())])
}

/// The polynomial multiplied out: `x^{3} - 2x + 1`.
fn expanded(e: &Expr) -> Expr {
    match simplify(e).as_rational() {
        Some((numerator, denominator)) if denominator.is_constant() => simplify(&Expr::product(vec![Expr::from_poly(&numerator), Expr::recip(Expr::from_poly(&denominator))])),
        _ => simplify(e),
    }
}

/// The curve through `points` with the working.
pub fn interpolate(points: &[(RBig, RBig)], ends: &Ends, var: &str, language: Language) -> Result<Spline> {
    match ends {
        Ends::Polynomial => polynomial(points, var),
        _ => cubic(points, ends, var, language),
    }
}

fn polynomial(points: &[(RBig, RBig)], var: &str) -> Result<Spline> {
    let n = points.len() - 1;
    let names: Vec<String> = (0..=n).map(|k| format!("a_{{{k}}}")).collect();
    let mut steps = Vec::new();
    let ansatz: Vec<String> = (0..=n)
        .rev()
        .map(|k| match k {
            0 => names[0].clone(),
            1 => format!("{}{var}", names[1]),
            _ => format!("{}{var}^{{{k}}}", names[k]),
        })
        .collect();
    steps.push(step(AnalysisRule::Spline, msg!("core.analysis.spline.polynomial_ansatz", n, points.len()), format!("p({var}) = {}", ansatz.join(" + "))));
    let equations: Vec<Equation> = points
        .iter()
        .map(|(x, y)| {
            let mut power = RBig::ONE;
            let mut row = Vec::new();
            for k in 0..=n {
                row.push((k, power.clone()));
                power *= x;
            }
            row.reverse();
            Equation { row, rhs: y.clone(), meaning: format!("p({}) = {}", num(x), num(y)) }
        })
        .collect();
    let lines: Vec<String> = equations.iter().map(|e| e.latex(&names)).collect();
    steps.push(step(AnalysisRule::Spline, msg!("core.analysis.spline.through_points"), array(&lines)));
    let solution = solve(&equations, n + 1)?;
    let values: Vec<String> = (0..=n).rev().map(|k| format!("{} = {}", names[k], num(&solution[k]))).collect();
    steps.push(step(AnalysisRule::Spline, msg!("core.analysis.spline.solve"), values.join(", \\quad ")));
    let p = expanded(&Expr::sum((0..=n).map(|k| Expr::product(vec![Expr::Num(solution[k].clone()), Expr::pow(Expr::Var, Expr::int(k as i64))])).collect()));
    steps.push(step(AnalysisRule::Spline, msg!("core.analysis.spline.result"), format!("p({var}) = {}", p.to_latex(var))));
    Ok(Spline { function: Function::plain(p), steps, points: points.to_vec() })
}

/// The unknowns a, b, c, d of piece i.
fn unknown(piece: usize, which: usize) -> usize {
    4 * piece + which
}

fn cubic(points: &[(RBig, RBig)], ends: &Ends, var: &str, language: Language) -> Result<Spline> {
    let n = points.len() - 1;
    let names: Vec<String> = (0..n).flat_map(|i| ["a", "b", "c", "d"].map(|l| format!("{l}_{{{}}}", i + 1))).collect();
    let h: Vec<RBig> = (0..n).map(|i| &points[i + 1].0 - &points[i].0).collect();
    let two = RBig::from(2);
    let three = RBig::from(3);
    let six = RBig::from(6);
    let mut steps = Vec::new();

    // The ansatz, one cubic per interval in powers of (x - x_i).
    let pieces: Vec<String> = (0..n)
        .map(|i| {
            let k = i + 1;
            let s = shifted(&points[i].0).to_latex(var);
            let s = if points[i].0 == RBig::ZERO { s } else { format!("({s})") };
            let interval = Interval {
                lo: Bound::At { point: Point::rational(points[i].0.clone()), closed: true },
                hi: Bound::At { point: Point::rational(points[i + 1].0.clone()), closed: true },
            };
            format!("s_{{{k}}}({var}) = a_{{{k}}} + b_{{{k}}}{s} + c_{{{k}}}{s}^{{2}} + d_{{{k}}}{s}^{{3}} \\ \\text{{on}} \\ {}", interval.to_latex(6, language))
        })
        .collect();
    steps.push(step(AnalysisRule::Spline, msg!("core.analysis.spline.ansatz", n), array(&pieces)));

    let mut through = Vec::new();
    let mut smooth = Vec::new();
    let mut at_ends = Vec::new();
    for i in 0..n {
        let k = i + 1;
        let (x0, y0) = &points[i];
        let (x1, y1) = &points[i + 1];
        let hi = &h[i];
        through.push(Equation { row: vec![(unknown(i, 0), RBig::ONE)], rhs: y0.clone(), meaning: format!("s_{{{k}}}({}) = {}", num(x0), num(y0)) });
        through.push(Equation {
            row: vec![(unknown(i, 0), RBig::ONE), (unknown(i, 1), hi.clone()), (unknown(i, 2), hi * hi), (unknown(i, 3), hi * hi * hi)],
            rhs: y1.clone(),
            meaning: format!("s_{{{k}}}({}) = {}", num(x1), num(y1)),
        });
        if i + 1 < n {
            let x = num(x1);
            smooth.push(Equation {
                row: vec![(unknown(i, 1), RBig::ONE), (unknown(i, 2), &two * hi), (unknown(i, 3), &three * hi * hi), (unknown(i + 1, 1), -RBig::ONE)],
                rhs: RBig::ZERO,
                meaning: format!("s_{{{k}}}'({x}) = s_{{{}}}'({x})", k + 1),
            });
            smooth.push(Equation {
                row: vec![(unknown(i, 2), two.clone()), (unknown(i, 3), &six * hi), (unknown(i + 1, 2), -two.clone())],
                rhs: RBig::ZERO,
                meaning: format!("s_{{{k}}}''({x}) = s_{{{}}}''({x})", k + 1),
            });
        }
    }
    let last = n - 1;
    let (first_x, last_x) = (num(&points[0].0), num(&points[n].0));
    let hl = &h[last];
    let ends_text = match ends {
        Ends::Clamped(m0, m1) => {
            at_ends.push(Equation { row: vec![(unknown(0, 1), RBig::ONE)], rhs: m0.clone(), meaning: format!("s_{{1}}'({first_x}) = {}", num(m0)) });
            at_ends.push(Equation {
                row: vec![(unknown(last, 1), RBig::ONE), (unknown(last, 2), &two * hl), (unknown(last, 3), &three * hl * hl)],
                rhs: m1.clone(),
                meaning: format!("s_{{{n}}}'({last_x}) = {}", num(m1)),
            });
            msg!("core.analysis.spline.clamped_ends")
        }
        _ => {
            at_ends.push(Equation { row: vec![(unknown(0, 2), two.clone())], rhs: RBig::ZERO, meaning: format!("s_{{1}}''({first_x}) = 0") });
            at_ends.push(Equation {
                row: vec![(unknown(last, 2), two.clone()), (unknown(last, 3), &six * hl)],
                rhs: RBig::ZERO,
                meaning: format!("s_{{{n}}}''({last_x}) = 0"),
            });
            msg!("core.analysis.spline.natural_ends")
        }
    };
    let latex = |equations: &[Equation]| array(&equations.iter().map(|e| e.latex(&names)).collect::<Vec<_>>());
    steps.push(step(AnalysisRule::Spline, msg!("core.analysis.spline.through_points"), latex(&through)));
    if !smooth.is_empty() {
        steps.push(step(AnalysisRule::Spline, msg!("core.analysis.spline.smooth"), latex(&smooth)));
    }
    steps.push(step(AnalysisRule::Spline, ends_text, latex(&at_ends)));
    let equations: Vec<Equation> = through.into_iter().chain(smooth).chain(at_ends).collect();
    steps.push(step(AnalysisRule::Spline, msg!("core.analysis.spline.count", equations.len(), 4 * n), String::new()).with_level(StepDetail::Detailed));
    let solution = solve(&equations, 4 * n)?;
    let values: Vec<String> = (0..n)
        .map(|i| (0..4).map(|w| format!("{} = {}", names[unknown(i, w)], num(&solution[unknown(i, w)]))).collect::<Vec<_>>().join(", \\ "))
        .collect();
    steps.push(step(AnalysisRule::Spline, msg!("core.analysis.spline.solve"), array(&values)));

    let mut function_pieces = Vec::new();
    let mut shifted_lines = Vec::new();
    let mut expanded_lines = Vec::new();
    for i in 0..n {
        let base = shifted(&points[i].0);
        let terms: Vec<Expr> = (0..4).map(|w| Expr::product(vec![Expr::Num(solution[unknown(i, w)].clone()), Expr::pow(base.clone(), Expr::int(w as i64))])).collect();
        let raw = Expr::sum(terms.clone());
        let shown = simplify(&Expr::sum(terms.into_iter().map(|t| simplify(&t)).collect()));
        let poly = expanded(&raw);
        shifted_lines.push(format!("s_{{{}}}({var}) = {}", i + 1, shown.to_latex(var)));
        expanded_lines.push(format!("s_{{{}}}({var}) = {}", i + 1, poly.to_latex(var)));
        let lo = Bound::At { point: Point::rational(points[i].0.clone()), closed: i == 0 };
        let hi = Bound::At { point: Point::rational(points[i + 1].0.clone()), closed: true };
        function_pieces.push(Piece { expr: poly, on: IntervalSet(vec![Interval { lo, hi }]) });
    }
    steps.push(step(AnalysisRule::Spline, msg!("core.analysis.spline.pieces"), array(&shifted_lines)).with_level(StepDetail::Standard));
    steps.push(step(AnalysisRule::Spline, msg!("core.analysis.spline.result"), array(&expanded_lines)));
    Ok(Spline { function: Function { pieces: function_pieces }, steps, points: points.to_vec() })
}

/// Lines below each other.
fn array(lines: &[String]) -> String {
    if lines.len() == 1 {
        return lines[0].clone();
    }
    format!("\\begin{{array}}{{l}} {} \\end{{array}}", lines.join(" \\\\ "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn points(latex: &str) -> Vec<(RBig, RBig)> {
        parse_points(latex, &Settings::default()).unwrap()
    }

    fn at(f: &Function, x: f64) -> f64 {
        f.eval_f64(x).unwrap()
    }

    #[test]
    fn point_lists_are_recognised() {
        assert!(is_point_list("(0,1),(1,3),(2,2)"));
        assert!(is_point_list("\\left(0|1\\right);\\ \\left(2|5\\right)"));
        assert!(is_point_list("(0 \\mid 1), (1 \\mid 0)"));
        assert!(!is_point_list("(x+1)(x-1)"));
        assert!(!is_point_list("(1,2)"));
        assert!(!is_point_list("x^2"));
        assert_eq!(points("(2,1),(0,\\frac{1}{2})")[0], (RBig::ZERO, RBig::from_parts(1.into(), 2u8.into())));
        assert!(parse_points("(0,1),(0,2)", &Settings::default()).unwrap_err().to_string().contains("same x"));
        assert!(parse_points("(0,1),(\\sqrt{2},2)", &Settings::default()).is_err());
    }

    #[test]
    fn a_natural_spline_goes_through_the_points_smoothly() {
        let p = points("(0,0),(1,1),(2,0),(3,1)");
        let s = interpolate(&p, &Ends::Natural, "x", Language::En).unwrap();
        let f = &s.function;
        assert_eq!(f.pieces.len(), 3);
        for (x, y) in &p {
            assert!((at(f, x.to_f64().value()) - y.to_f64().value()).abs() < 1e-12);
        }
        // Exactly the same value, slope and curvature at the inner points.
        for i in 0..2 {
            let x = Expr::Num(p[i + 1].0.clone());
            let (left, right) = (&f.pieces[i].expr, &f.pieces[i + 1].expr);
            let mut l = left.clone();
            let mut r = right.clone();
            for _ in 0..3 {
                assert!(super::super::simplify::is_zero(&simplify(&Expr::sum(vec![l.substitute(&x), Expr::negate(r.substitute(&x))]))));
                l = simplify(&super::super::derive::derivative(&l));
                r = simplify(&super::super::derive::derivative(&r));
            }
        }
        // s'' = 0 at both ends.
        let second = |e: &Expr| simplify(&super::super::derive::derivative(&super::super::derive::derivative(e)));
        assert_eq!(second(&f.pieces[0].expr).eval_f64(0.0), Some(0.0));
        assert_eq!(second(&f.pieces[2].expr).eval_f64(3.0), Some(0.0));
        // The textbook coefficients of the first piece: s_1(x) = \frac{5}{3}x - \frac{2}{3}x^3.
        assert_eq!(f.pieces[0].expr.to_latex("x"), "-\\frac{2}{3}x^{3} + \\frac{5}{3}x");
        assert!(s.steps.iter().any(|s| s.latex.contains("s_{1}''(0) = 0 \\Rightarrow 2c_{1} = 0")), "{:?}", s.steps.iter().map(|s| s.latex.clone()).collect::<Vec<_>>());
    }

    #[test]
    fn clamped_ends_and_the_polynomial() {
        let p = points("(0,0),(1,1)");
        let clamped = interpolate(&p, &Ends::Clamped(RBig::ZERO, RBig::ZERO), "x", Language::En).unwrap();
        assert_eq!(clamped.function.pieces[0].expr.to_latex("x"), "-2x^{3} + 3x^{2}");
        let line = interpolate(&p, &Ends::Natural, "x", Language::En).unwrap();
        assert_eq!(line.function.pieces[0].expr.to_latex("x"), "x");
        let parabola = interpolate(&points("(-1,1),(0,0),(1,1)"), &Ends::Polynomial, "x", Language::En).unwrap();
        assert_eq!(parabola.function.pieces[0].expr.to_latex("x"), "x^{2}");
        assert!(parabola.function.pieces[0].on.is_reals());
    }

    #[test]
    fn the_curve_is_sketched_like_any_function() {
        use crate::analysis::interface::{analyze, spline, AnalysisRequest, SplineRequest};
        let settings = Settings::default();
        let request = SplineRequest { points: "(0,0),(1,1),(2,0),(3,1)".into(), ..Default::default() };
        let curve = spline(&settings, &request).unwrap();
        assert!(curve.function.starts_with("\\begin{cases} -\\frac{2}{3}x^{3} + \\frac{5}{3}x & 0 \\le x \\le 1 \\\\ "), "{}", curve.function);
        assert_eq!(curve.points.len(), 4);
        let sketch = analyze(&settings, &AnalysisRequest { latex: curve.function.clone(), ..Default::default() }).unwrap();
        let joins = sketch.sections.iter().find(|s| s.id == "joins").unwrap();
        assert_eq!(joins.result.as_deref(), Some("x = 1\\colon \\ \\text{continuous and differentiable}, \\quad x = 2\\colon \\ \\text{continuous and differentiable}"));
        let clamped = SplineRequest { points: "(0|0); (1|1)".into(), ends: "clamped".into(), start_slope: Some("0".into()), end_slope: Some("0".into()) };
        assert_eq!(spline(&settings, &clamped).unwrap().function, "-2x^{3} + 3x^{2}, \\ 0 \\le x \\le 1");
        let polynomial = SplineRequest { points: "(-1,1),(0,0),(1,1)".into(), ends: "polynomial".into(), ..Default::default() };
        assert_eq!(spline(&settings, &polynomial).unwrap().function, "x^{2}");
    }
}
