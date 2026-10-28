//! The domain of a function, read off the tree as typed (before anything cancels):
//! denominators are not zero, logarithms see positive arguments, even roots non-negative
//! radicands, arcsin and arccos arguments between -1 and 1, and tan stays away from its poles.
//!
//! An equation `u ≠ 0` is solved with the solver; an inequality `u > 0` by cutting the domain of
//! u at its zeros and testing the sign in each piece.

use std::cmp::Ordering;

use dashu::base::Signed;
use dashu::rational::RBig;

use super::expr::{Expr, Fn};
use super::interval::{Bound, Domain, Interval, IntervalSet, PeriodicSet, Point};
use super::rules::{step, AnalysisRule};
use super::simplify::{is_zero, simplify};
use super::solve::{solve_zero, SolveContext};
use crate::eval::Step;
use crate::num::Value;
use crate::settings::{Settings, StepDetail};
use crate::text::msg;
use crate::{QuantusError, Result};

/// A condition a construct puts on its argument.
#[derive(Clone, Debug, PartialEq)]
enum Condition {
    NonZero(Expr),
    Positive(Expr, Reason),
    NonNegative(Expr, Reason),
    /// `-1 ≤ u ≤ 1`.
    UnitRange(Expr),
    /// `\cos u ≠ 0`, for `\tan u`.
    CosNonZero(Expr),
}

/// Why an expression must be positive.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Reason {
    Logarithm,
    VariablePower,
    RootBelowTheLine,
    EvenRoot,
    /// A power with a constant irrational exponent, `x^{\pi}`.
    RealPower,
}

pub struct DomainContext<'a> {
    pub settings: &'a Settings,
    pub var: &'a str,
    pub digits: usize,
}

/// The domain of `f` with one step per condition. An empty domain is an error.
pub fn domain(f: &Expr, ctx: &DomainContext) -> Result<(Domain, Vec<Step>)> {
    let (mut domain, mut steps) = domain_of(f, ctx)?;
    if domain.set.is_empty() {
        return Err(QuantusError::other(msg!("f is defined nowhere.", "f ist nirgends definiert.")));
    }
    domain.holes = holes(f, &domain);
    let latex = format!("D = {}", domain.to_latex(ctx.digits, ctx.settings.language));
    steps.push(step(AnalysisRule::DomainCondition, msg!("Domain", "Definitionsbereich"), latex));
    Ok((domain, steps))
}

fn domain_of(f: &Expr, ctx: &DomainContext) -> Result<(Domain, Vec<Step>)> {
    let mut conditions = Vec::new();
    collect(f, &mut conditions)?;
    let mut domain = Domain::reals();
    let mut steps = Vec::new();
    for condition in conditions {
        apply(&condition, &mut domain, &mut steps, ctx)?;
    }
    Ok((domain, steps))
}

/// Records a condition on `argument`; one on a constant is checked right away.
fn add(out: &mut Vec<Condition>, condition: Condition, argument: &Expr) -> Result<()> {
    if argument.is_constant() {
        return check_constant(&condition, argument);
    }
    if !out.contains(&condition) {
        out.push(condition);
    }
    Ok(())
}

fn collect(e: &Expr, out: &mut Vec<Condition>) -> Result<()> {
    match e {
        Expr::Num(_) | Expr::Pi | Expr::E | Expr::Var => {}
        Expr::Sum(items) | Expr::Product(items) => {
            for item in items {
                collect(item, out)?;
            }
        }
        Expr::Pow(base, exponent) => {
            collect(base, out)?;
            collect(exponent, out)?;
            // A constant exponent as typed may still be a quotient (`x^{2/3}`).
            let rational = match exponent.as_num() {
                Some(r) => Some(r.clone()),
                None if exponent.is_constant() => simplify(exponent).as_num().cloned(),
                None => None,
            };
            match rational.as_ref() {
                Some(r) => {
                    let even_root = r.denominator().clone() % dashu::integer::UBig::from(2u8) == dashu::integer::UBig::ZERO;
                    if even_root && r.is_positive() {
                        add(out, Condition::NonNegative((**base).clone(), Reason::EvenRoot), base)?;
                    } else if even_root {
                        add(out, Condition::Positive((**base).clone(), Reason::RootBelowTheLine), base)?;
                    } else if r.is_negative() {
                        add(out, Condition::NonZero((**base).clone()), base)?;
                    }
                }
                None if exponent.depends_on_var() => add(out, Condition::Positive((**base).clone(), Reason::VariablePower), base)?,
                // `x^{\pi}` is only real for x ≥ 0, `x^{-\pi}` for x > 0.
                None => match exponent.eval_f64(0.0) {
                    Some(v) if v > 0.0 => add(out, Condition::NonNegative((**base).clone(), Reason::RealPower), base)?,
                    Some(v) if v < 0.0 => add(out, Condition::Positive((**base).clone(), Reason::RealPower), base)?,
                    _ => {}
                },
            }
        }
        Expr::Func(function, argument) => {
            collect(argument, out)?;
            match function {
                Fn::Ln => add(out, Condition::Positive((**argument).clone(), Reason::Logarithm), argument)?,
                Fn::Arcsin | Fn::Arccos => add(out, Condition::UnitRange((**argument).clone()), argument)?,
                Fn::Tan => add(out, Condition::CosNonZero((**argument).clone()), argument)?,
                _ => {}
            }
        }
    }
    Ok(())
}

/// A condition on a constant either always holds or makes the domain empty.
fn check_constant(condition: &Condition, value: &Expr) -> Result<()> {
    let v = value.eval_f64(0.0);
    let holds = match (condition, v) {
        (_, None) => false,
        (Condition::NonZero(_), Some(_)) => !is_zero(value),
        (Condition::Positive(..), Some(v)) => v > 0.0,
        (Condition::NonNegative(..), Some(v)) => v >= 0.0,
        (Condition::UnitRange(_), Some(v)) => v.abs() <= 1.0,
        (Condition::CosNonZero(_), Some(v)) => v.cos().abs() > 1e-15,
    };
    if holds {
        Ok(())
    } else {
        Err(QuantusError::other(msg!("f is defined nowhere.", "f ist nirgends definiert.")))
    }
}

fn apply(condition: &Condition, domain: &mut Domain, steps: &mut Vec<Step>, ctx: &DomainContext) -> Result<()> {
    let tex = |e: &Expr| e.to_latex(ctx.var);
    let var = ctx.var;
    match condition {
        Condition::NonZero(u) => {
            let (points, families, complete) = zeros(u, ctx);
            for point in &points {
                domain.set = domain.set.remove_point(point);
            }
            domain.excluded.extend(families.iter().cloned());
            let listed = exclusions(&points, &families, ctx.digits);
            let latex = match listed {
                None => format!("{} \\neq 0", tex(u)),
                Some(listed) => format!("{} \\neq 0 \\Rightarrow {var} \\neq {listed}", tex(u)),
            };
            let explanation = msg!("Denominator must not be zero", "Nenner darf nicht null sein");
            steps.push(step(AnalysisRule::DomainCondition, numeric_note(explanation, complete), latex));
        }
        Condition::CosNonZero(u) => {
            let cosine = Expr::func(Fn::Cos, u.clone());
            let (points, families, _) = zeros(&cosine, ctx);
            for point in &points {
                domain.set = domain.set.remove_point(point);
            }
            domain.excluded.extend(families.iter().cloned());
            let listed = exclusions(&points, &families, ctx.digits).unwrap_or_default();
            let latex = format!("{} \\neq 0 \\Rightarrow {var} \\neq {listed}", tex(&cosine));
            steps.push(step(
                AnalysisRule::DomainCondition,
                msg!("tan is undefined where cos = 0", "tan ist nicht definiert, wo cos = 0"),
                latex,
            ));
        }
        Condition::Positive(u, _) | Condition::NonNegative(u, _) => {
            let strict = matches!(condition, Condition::Positive(..));
            let (set, complete) = sign_set(u, strict, ctx)?;
            // Where the sign can change: the zeros of u, solved as in the zeros section.
            if *u != Expr::Var {
                steps.extend(zero_working(u, ctx));
            }
            domain.set = domain.set.intersect(&set);
            let relation = if strict { ">" } else { "\\geq" };
            let latex = format!("{} {relation} 0 \\Rightarrow {var} \\in {}", tex(u), set.to_latex(ctx.digits, ctx.settings.language));
            let explanation = match condition {
                Condition::Positive(_, Reason::Logarithm) => {
                    msg!("Argument of the logarithm > 0", "Argument des Logarithmus > 0")
                }
                Condition::Positive(_, Reason::VariablePower) => {
                    msg!("Base of a variable exponent > 0", "Basis einer variablen Potenz > 0")
                }
                Condition::Positive(_, Reason::RootBelowTheLine) => {
                    msg!("Radicand of an even root in the denominator > 0", "Radikand einer geraden Wurzel im Nenner > 0")
                }
                Condition::Positive(_, Reason::RealPower) => {
                    msg!("Base of a power with a negative irrational exponent > 0", "Basis einer Potenz mit negativem irrationalem Exponenten > 0")
                }
                Condition::NonNegative(_, Reason::RealPower) => {
                    msg!("Base of a power with an irrational exponent ≥ 0", "Basis einer Potenz mit irrationalem Exponenten ≥ 0")
                }
                _ => msg!("Radicand of an even root ≥ 0", "Radikand einer geraden Wurzel ≥ 0"),
            };
            steps.push(step(AnalysisRule::DomainCondition, numeric_note(explanation, complete), latex));
        }
        Condition::UnitRange(u) => {
            // Two inequalities, one step each.
            let language = ctx.settings.language;
            let (lower, lower_complete) = sign_set(&Expr::Sum(vec![u.clone(), Expr::int(1)]), false, ctx)?;
            domain.set = domain.set.intersect(&lower);
            let latex = format!("{} \\geq -1 \\Rightarrow {var} \\in {}", tex(u), lower.to_latex(ctx.digits, language));
            let explanation = msg!("Argument of arcsin or arccos ≥ -1", "Argument von arcsin oder arccos ≥ -1");
            steps.push(step(AnalysisRule::DomainCondition, numeric_note(explanation, lower_complete), latex));
            let (upper, upper_complete) = sign_set(&Expr::Sum(vec![Expr::int(1), Expr::neg(u.clone())]), false, ctx)?;
            domain.set = domain.set.intersect(&upper);
            let latex = format!("{} \\leq 1 \\Rightarrow {var} \\in {}", tex(u), upper.to_latex(ctx.digits, language));
            let explanation = msg!("Argument of arcsin or arccos ≤ 1", "Argument von arcsin oder arccos ≤ 1");
            steps.push(step(AnalysisRule::DomainCondition, numeric_note(explanation, upper_complete), latex));
        }
    }
    Ok(())
}

/// `2, -3` or `\frac{\pi}{2} + k\pi` for the excluded points; `None` when there are none.
fn exclusions(points: &[Point], families: &[PeriodicSet], digits: usize) -> Option<String> {
    let mut parts: Vec<String> = points.iter().map(|p| p.to_latex(digits)).collect();
    for family in families {
        for written in family.to_latex() {
            parts.push(format!("{written}, \\ k \\in \\mathbb{{Z}}"));
        }
    }
    (!parts.is_empty()).then(|| parts.join(", "))
}

/// Where periodic zeros of a non-linear argument are listed (`\frac{1}{\sin(x^2)}`); the
/// solver's own numeric fallback searches [-100, 100].
const SEARCH_WINDOW: (f64, f64) = (-10.0, 10.0);

/// The real zeros of `u`, exact where possible, and whether they are all of them (`false` when
/// part of the search was numeric in a window).
fn zeros(u: &Expr, ctx: &DomainContext) -> (Vec<Point>, Vec<PeriodicSet>, bool) {
    let reals = Domain::reals();
    let solve_ctx = SolveContext { settings: ctx.settings, var: ctx.var, domain: &reals, window: SEARCH_WINDOW, digits: ctx.digits };
    let solved = solve_zero(u, &solve_ctx);
    (solved.solutions.roots.into_iter().map(|r| r.point).collect(), solved.solutions.families, solved.complete)
}

/// How the zeros of `u` are found, from Standard on.
fn zero_working(u: &Expr, ctx: &DomainContext) -> Vec<Step> {
    let reals = Domain::reals();
    let solve_ctx = SolveContext { settings: ctx.settings, var: ctx.var, domain: &reals, window: SEARCH_WINDOW, digits: ctx.digits };
    let solved = solve_zero(u, &solve_ctx);
    let lead = step(AnalysisRule::DomainCondition, msg!("Where the sign can change", "Wo das Vorzeichen wechseln kann"), format!("{} = 0", u.to_latex(ctx.var)));
    std::iter::once(lead)
        .chain(solved.steps)
        .map(|s| {
            let level = s.level.max(StepDetail::Standard);
            s.with_level(level)
        })
        .collect()
}

/// A step's explanation, with a note when the zeros behind it were only searched numerically.
fn numeric_note(explanation: String, complete: bool) -> String {
    if complete {
        explanation
    } else {
        msg!("{} (zeros found numerically, some may be missing)", "{} (Nullstellen numerisch gesucht, es können welche fehlen)", explanation)
    }
}

/// Where `u > 0` (or `u ≥ 0`): the domain of u cut at the zeros of u, with the sign tested in
/// each piece at its simplest rational point.
fn sign_set(u: &Expr, strict: bool, ctx: &DomainContext) -> Result<(IntervalSet, bool)> {
    let (own, _) = domain_of(u, ctx)?;
    let (points, families, complete) = zeros(u, ctx);
    if !families.is_empty() || !own.excluded.is_empty() {
        return Err(QuantusError::other(msg!(
            "Domains made of periodic intervals are not supported yet.",
            "Definitionsbereiche aus periodischen Intervallen werden noch nicht unterstützt."
        )));
    }
    let mut pieces: Vec<Interval> = Vec::new();
    for interval in &own.set.0 {
        let mut cuts: Vec<&Point> = points.iter().filter(|p| interval.contains(p.approx)).collect();
        cuts.sort_by(|a, b| a.approx.partial_cmp(&b.approx).unwrap_or(Ordering::Equal));
        let mut bounds: Vec<Bound> = vec![interval.lo.clone()];
        for cut in &cuts {
            bounds.push(Bound::At { point: (*cut).clone(), closed: false });
        }
        bounds.push(interval.hi.clone());
        for pair in bounds.windows(2) {
            let piece = Interval { lo: pair[0].clone(), hi: pair[1].clone() };
            if piece.lo.value() >= piece.hi.value() {
                continue;
            }
            if sign_at(u, &piece.test_point()) > 0 {
                pieces.push(piece);
            }
        }
        if !strict {
            for cut in cuts {
                close_at(&mut pieces, cut);
            }
        }
    }
    Ok((IntervalSet(pieces), complete))
}

/// Adds a zero of u to the pieces where u ≥ 0: it closes the bound it touches, joins two pieces
/// that meet there, or stands alone.
fn close_at(pieces: &mut Vec<Interval>, zero: &Point) {
    let touches = |b: &Bound| matches!(b, Bound::At { point, .. } if point.same(zero));
    let left = pieces.iter().position(|p| touches(&p.hi));
    let right = pieces.iter().position(|p| touches(&p.lo));
    match (left, right) {
        (Some(l), Some(r)) => {
            let hi = pieces[r].hi.clone();
            pieces[l].hi = hi;
            pieces.remove(r);
        }
        (Some(l), None) => pieces[l].hi = Bound::At { point: zero.clone(), closed: true },
        (None, Some(r)) => pieces[r].lo = Bound::At { point: zero.clone(), closed: true },
        (None, None) => {
            let at = Bound::At { point: zero.clone(), closed: true };
            pieces.push(Interval { lo: at.clone(), hi: at });
            pieces.sort_by(|a, b| a.lo.value().partial_cmp(&b.lo.value()).unwrap_or(Ordering::Equal));
        }
    }
}

/// The sign of `u` at `x`: exactly where the evaluator can, in floating point otherwise.
pub fn sign_at(u: &Expr, x: &RBig) -> i32 {
    if let Ok(value) = u.eval_exact(&Value::Rat(x.clone()), 30) {
        if value.is_zero() {
            return 0;
        }
        if let Ok(negative) = value.is_negative() {
            return if negative { -1 } else { 1 };
        }
    }
    match u.eval_f64(x.to_f64().value()) {
        Some(v) if v > 0.0 => 1,
        Some(v) if v < 0.0 => -1,
        _ => 0,
    }
}

/// Excluded points of a rational function where the function could be continued: a factor of
/// the denominator that cancels against the numerator (`\frac{x^2-1}{x-1}` at 1).
fn holes(f: &Expr, domain: &Domain) -> Vec<Point> {
    let Some((numerator, denominator)) = f.as_rational() else { return Vec::new() };
    if denominator.is_constant() {
        return Vec::new();
    }
    let common = numerator.gcd(&denominator);
    if common.is_constant() {
        return Vec::new();
    }
    let reduced = match denominator.div_rem(&common) {
        Ok((quotient, _)) => Expr::from_poly(&quotient),
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for interval in &domain.set.0 {
        if let Bound::At { point, closed: false } = &interval.hi {
            let still_zero = match &point.exact {
                Some(x) => is_zero(&simplify(&reduced.substitute(x))),
                None => reduced.eval_f64(point.approx).is_none_or(|v| v.abs() <= 1e-9),
            };
            if !still_zero && !out.iter().any(|p: &Point| p.same(point)) {
                out.push(point.clone());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::latex::parser;
    use crate::settings::{Language, Mode};

    fn domain_of_latex(latex: &str, language: Language) -> Result<(Domain, Vec<Step>)> {
        let options = Settings { mode: Mode::Algebra, ..Settings::default() }.parse_options();
        let f = Expr::from_node(&parser::parse_with(latex, &options).unwrap().expression, "x").unwrap();
        let settings = Settings { language, ..Settings::default() };
        let _language = crate::text::Scope::new(language);
        domain(&f, &DomainContext { settings: &settings, var: "x", digits: 6 })
    }

    fn written(latex: &str) -> String {
        domain_of_latex(latex, Language::De).unwrap().0.to_latex(6, Language::De)
    }

    #[test]
    fn denominators_exclude_their_zeros() {
        assert_eq!(written("x^3 - 3x"), "\\mathbb{R}");
        assert_eq!(written("\\frac{x^2-1}{x-2}"), "\\mathbb{R} \\setminus \\{2\\}");
        assert_eq!(written("\\frac{1}{x^2-3}"), "\\mathbb{R} \\setminus \\{-\\sqrt{3}; \\sqrt{3}\\}");
        assert_eq!(written("\\frac{1}{x^2+1}"), "\\mathbb{R}");
    }

    #[test]
    fn logarithms_and_roots_need_their_sign() {
        assert_eq!(written("\\frac{\\ln x}{x}"), "]0; \\infty[");
        assert_eq!(written("\\sqrt{x}"), "[0; \\infty[");
        assert_eq!(written("\\frac{1}{\\sqrt{x}}"), "]0; \\infty[");
        assert_eq!(written("\\sqrt{x^2 - 1}"), "]-\\infty; -1] \\cup [1; \\infty[");
        assert_eq!(written("\\ln(4 - x^2)"), "]-2; 2[");
        assert_eq!(written("\\sqrt[3]{x}"), "\\mathbb{R}");
        assert_eq!(written("x^x"), "]0; \\infty[");
        assert_eq!(written("\\arcsin(2x)"), "[-\\frac{1}{2}; \\frac{1}{2}]");
        assert_eq!(written("x^{\\pi}"), "[0; \\infty[");
        assert_eq!(written("x^{-\\sqrt{2}}"), "]0; \\infty[");
        assert_eq!(written("x^{2/3}"), "\\mathbb{R}");
        assert_eq!(written("x^{\\frac{1}{2}}"), "[0; \\infty[");
        assert_eq!(domain_of_latex("\\ln x", Language::En).unwrap().0.to_latex(6, Language::En), "(0, \\infty)");
    }

    #[test]
    fn the_domain_is_taken_before_anything_cancels() {
        let (d, _) = domain_of_latex("e^{\\ln x}", Language::De).unwrap();
        assert_eq!(d.to_latex(6, Language::De), "]0; \\infty[");
        let (d, _) = domain_of_latex("\\frac{x^2-1}{x-1}", Language::De).unwrap();
        assert_eq!(d.to_latex(6, Language::De), "\\mathbb{R} \\setminus \\{1\\}");
        assert_eq!(d.holes.len(), 1);
        let (pole, _) = domain_of_latex("\\frac{x+1}{(x-1)^2}", Language::De).unwrap();
        assert!(pole.holes.is_empty());
    }

    #[test]
    fn tan_has_periodic_gaps() {
        let (d, steps) = domain_of_latex("\\tan x", Language::En).unwrap();
        assert_eq!(d.to_latex(6, Language::En), "\\mathbb{R} \\setminus \\{\\frac{\\pi}{2} + k\\pi \\mid k \\in \\mathbb{Z}\\}");
        assert!(steps[0].latex.starts_with("\\cos(x) \\neq 0"));
        let (d, _) = domain_of_latex("\\tan x", Language::De).unwrap();
        assert_eq!(d.to_latex(6, Language::De), "\\mathbb{R} \\setminus \\{\\frac{\\pi}{2} + k\\pi \\mid k \\in \\mathbb{Z}\\}");
    }

    #[test]
    fn arcsin_gives_two_inequalities() {
        let (_, steps) = domain_of_latex("\\arcsin(2x)", Language::En).unwrap();
        assert_eq!(steps[0].latex, "2x \\geq -1 \\Rightarrow x \\in [-\\frac{1}{2}, \\infty)");
        assert_eq!(steps[1].latex, "2x \\leq 1 \\Rightarrow x \\in (-\\infty, \\frac{1}{2}]");
        assert_eq!(steps[2].latex, "D = [-\\frac{1}{2}, \\frac{1}{2}]");
    }

    #[test]
    fn zeros_found_numerically_are_noted() {
        let (_, steps) = domain_of_latex("\\frac{1}{e^x - x - 2}", Language::En).unwrap();
        assert!(steps[0].explanation.as_deref().unwrap().ends_with("(zeros found numerically, some may be missing)"), "{:?}", steps[0]);
    }

    #[test]
    fn each_condition_is_a_step_and_errors_are_named() {
        let (_, steps) = domain_of_latex("\\frac{\\ln x}{x - 2}", Language::De).unwrap();
        let steps = crate::eval::calculator::shown_steps(&steps, StepDetail::Brief);
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0].latex, "x > 0 \\Rightarrow x \\in ]0; \\infty[");
        assert_eq!(steps[0].explanation.as_deref(), Some("Argument des Logarithmus > 0"));
        assert_eq!(steps[1].latex, "x - 2 \\neq 0 \\Rightarrow x \\neq 2");
        assert_eq!(steps[2].latex, "D = ]0; 2[ \\cup ]2; \\infty[");
        assert!(domain_of_latex("\\ln(-1) + x", Language::En).unwrap_err().to_string().contains("defined nowhere"));
        assert!(domain_of_latex("\\sqrt{-x^2 - 1}", Language::En).unwrap_err().to_string().contains("defined nowhere"));
        assert!(domain_of_latex("\\ln(\\sin x)", Language::En).unwrap_err().to_string().contains("periodic"));
    }
}
