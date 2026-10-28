//! Behaviour at the edges of the domain and at poles, and the asymptotes that follow from it.
//!
//! Every approach is turned into t → ∞: x = t, x = -t, or x = a ± 1/t. Rational functions are
//! decided by their degrees (at ±∞) or the sign next to the pole; other functions by growth
//! orders, where a term behaves like `c·e^{bt}·t^a·(\ln t)^l` and the largest (b, a, l) wins;
//! sin and cos are bounded; and what none of that decides is sampled numerically.

use dashu::base::Signed;
use dashu::rational::RBig;

use super::expr::{Expr, Fn};
use super::interval::Point;
use super::polydiv::divide;
use super::rules::{step, AnalysisRule};
use super::simplify::{is_zero, simplify};
use crate::algebra::poly::{Monomial, Poly};
use crate::eval::Step;
use crate::text::msg;

#[derive(Clone, Debug)]
pub enum Approach {
    PosInf,
    NegInf,
    /// From the left: x → a⁻.
    Left(Point),
    /// From the right: x → a⁺.
    Right(Point),
}

#[derive(Clone, Debug)]
pub enum Behaviour {
    Finite(Point),
    PosInf,
    NegInf,
    Oscillates,
    Unknown,
}

impl Behaviour {
    pub fn is_infinite(&self) -> bool {
        matches!(self, Behaviour::PosInf | Behaviour::NegInf)
    }

    pub fn to_latex(&self, digits: usize) -> String {
        match self {
            Behaviour::Finite(p) if p.exact.is_some() => p.to_latex(digits),
            Behaviour::Finite(p) => format!("\\approx {}", p.to_latex(digits)),
            Behaviour::PosInf => "\\infty".into(),
            Behaviour::NegInf => "-\\infty".into(),
            Behaviour::Oscillates => format!("\\text{{{}}}", msg!("does not exist (oscillates)", "existiert nicht (oszilliert)")),
            Behaviour::Unknown => "\\text{?}".into(),
        }
    }
}

pub struct LimitContext<'a> {
    pub var: &'a str,
    pub digits: usize,
}

impl Approach {
    pub fn to_latex(&self, var: &str, digits: usize) -> String {
        match self {
            Approach::PosInf => format!("{var} \\to \\infty"),
            Approach::NegInf => format!("{var} \\to -\\infty"),
            Approach::Left(a) => format!("{var} \\to {}^{{-}}", bracketed(&a.to_latex(digits))),
            Approach::Right(a) => format!("{var} \\to {}^{{+}}", bracketed(&a.to_latex(digits))),
        }
    }

    /// The function in t with t → ∞.
    fn substituted(&self, f: &Expr) -> Option<Expr> {
        let shift = |a: &Point, sign: i64| {
            let a = a.exact.clone()?;
            Some(Expr::Sum(vec![a, Expr::Product(vec![Expr::int(sign), Expr::recip(Expr::Var)])]))
        };
        let with = match self {
            Approach::PosInf => return Some(f.clone()),
            Approach::NegInf => Expr::neg(Expr::Var),
            Approach::Left(a) => shift(a, -1)?,
            Approach::Right(a) => shift(a, 1)?,
        };
        Some(simplify(&f.substitute(&with)))
    }

    /// The point x for a large t, for sampling.
    fn sample_at(&self, t: f64) -> f64 {
        match self {
            Approach::PosInf => t,
            Approach::NegInf => -t,
            Approach::Left(a) => a.approx - (1.0 / t) * a.approx.abs().max(1.0),
            Approach::Right(a) => a.approx + (1.0 / t) * a.approx.abs().max(1.0),
        }
    }
}

fn bracketed(text: &str) -> String {
    if text.starts_with('-') || text.contains(' ') || text.contains("\\frac") {
        format!("\\left({text}\\right)")
    } else {
        text.to_string()
    }
}

/// How f behaves when x approaches, with one step saying how that was decided.
pub fn behaviour(f: &Expr, at: &Approach, ctx: &LimitContext) -> (Behaviour, Step) {
    let (result, rule, explanation) = decide(f, at);
    let latex = format!("\\lim_{{{}}} f({}) = {}", at.to_latex(ctx.var, ctx.digits), ctx.var, result.to_latex(ctx.digits));
    (result, step(rule, explanation, latex))
}

fn decide(f: &Expr, at: &Approach) -> (Behaviour, AnalysisRule, String) {
    if let Some((numerator, denominator)) = f.as_rational() {
        if let Some(decided) = rational(&numerator, &denominator, at) {
            return decided;
        }
    }
    if let Some(g) = at.substituted(f) {
        if let Some(decided) = bounded(&g) {
            return decided;
        }
        if let Some(growth) = growth(&g) {
            let behaviour = growth.limit();
            return (behaviour, AnalysisRule::LimitDominance, growth.explanation());
        }
    }
    (numeric(f, at), AnalysisRule::LimitNumeric, msg!("Determined numerically", "Numerisch bestimmt"))
}

// ---- rational functions ------------------------------------------------------------------------

fn rational(numerator: &Poly, denominator: &Poly, at: &Approach) -> Option<(Behaviour, AnalysisRule, String)> {
    let x = Expr::poly_variable();
    let common = numerator.gcd(denominator);
    let (n, d) = if common.is_constant() {
        (numerator.clone(), denominator.clone())
    } else {
        (numerator.div_rem(&common).ok()?.0, denominator.div_rem(&common).ok()?.0)
    };
    let leading = |p: &Poly| p.coeff_of(&Monomial::single(x, p.degree_of(x)));
    match at {
        Approach::PosInf | Approach::NegInf => {
            let (deg_n, deg_d) = (n.degree_of(x), d.degree_of(x));
            let ratio = leading(&n) / leading(&d);
            if n.is_zero() {
                return Some((Behaviour::Finite(Point::rational(RBig::ZERO)), AnalysisRule::LimitDegree, msg!("f is 0", "f ist 0")));
            }
            Some(if deg_d == 0 {
                let sign = sign_at_infinity(&ratio, deg_n, matches!(at, Approach::NegInf));
                let behaviour = if deg_n == 0 { Behaviour::Finite(Point::rational(ratio)) } else { infinite(sign) };
                (behaviour, AnalysisRule::LimitDegree, msg!("The term with the highest power decides", "Der Summand mit dem höchsten Exponenten bestimmt das Verhalten"))
            } else if deg_n < deg_d {
                (
                    Behaviour::Finite(Point::rational(RBig::ZERO)),
                    AnalysisRule::LimitDegree,
                    msg!("Degree of the numerator < degree of the denominator", "Zählergrad < Nennergrad"),
                )
            } else if deg_n == deg_d {
                (
                    Behaviour::Finite(Point::rational(ratio)),
                    AnalysisRule::LimitDegree,
                    msg!(
                        "Degree of the numerator = degree of the denominator: ratio of the leading coefficients",
                        "Zählergrad = Nennergrad: Quotient der Leitkoeffizienten"
                    ),
                )
            } else {
                let sign = sign_at_infinity(&ratio, deg_n - deg_d, matches!(at, Approach::NegInf));
                (
                    infinite(sign),
                    AnalysisRule::LimitDegree,
                    msg!("Degree of the numerator > degree of the denominator", "Zählergrad > Nennergrad"),
                )
            })
        }
        Approach::Left(a) | Approach::Right(a) => {
            let reduced = Expr::Product(vec![Expr::from_poly(&n), Expr::recip(Expr::from_poly(&d))]);
            let below = Expr::from_poly(&d);
            let denominator_vanishes = match &a.exact {
                Some(e) => is_zero(&simplify(&below.substitute(e))),
                None => below.eval_f64(a.approx).is_some_and(|v| v.abs() < 1e-9),
            };
            if !denominator_vanishes {
                // A removable gap: the reduced function is continuous there.
                let value = match &a.exact {
                    Some(e) => Point::exact(simplify(&reduced.substitute(e))),
                    None => reduced.eval_f64(a.approx).map(Point::approx),
                }?;
                return Some((
                    Behaviour::Finite(value),
                    AnalysisRule::LimitPole,
                    msg!("Removable gap: the cancelled function is continuous here", "Hebbare Lücke: die gekürzte Funktion ist hier stetig"),
                ));
            }
            let x = at.sample_at(1e7);
            let value = reduced.eval_f64(x)?;
            Some((infinite(value.signum() as i32), AnalysisRule::LimitPole, msg!("Pole", "Polstelle")))
        }
    }
}

fn sign_at_infinity(ratio: &RBig, odd_power: usize, negative_side: bool) -> i32 {
    let sign = if ratio.is_negative() { -1 } else { 1 };
    if negative_side && odd_power % 2 == 1 { -sign } else { sign }
}

fn infinite(sign: i32) -> Behaviour {
    if sign < 0 { Behaviour::NegInf } else { Behaviour::PosInf }
}

/// Both sides of a pole, and one step naming whether the sign changes.
pub fn pole(f: &Expr, at: &Point, ctx: &LimitContext) -> (Behaviour, Behaviour, Step) {
    let (left, _) = behaviour(f, &Approach::Left(at.clone()), ctx);
    let (right, _) = behaviour(f, &Approach::Right(at.clone()), ctx);
    let latex = format!(
        "\\lim_{{{}}} f({var}) = {}, \\quad \\lim_{{{}}} f({var}) = {}",
        Approach::Left(at.clone()).to_latex(ctx.var, ctx.digits),
        left.to_latex(ctx.digits),
        Approach::Right(at.clone()).to_latex(ctx.var, ctx.digits),
        right.to_latex(ctx.digits),
        var = ctx.var
    );
    let explanation = match (&left, &right) {
        (Behaviour::PosInf, Behaviour::NegInf) | (Behaviour::NegInf, Behaviour::PosInf) => {
            msg!("Pole with a sign change", "Polstelle mit Vorzeichenwechsel")
        }
        (Behaviour::PosInf, Behaviour::PosInf) | (Behaviour::NegInf, Behaviour::NegInf) => {
            msg!("Pole without a sign change", "Polstelle ohne Vorzeichenwechsel")
        }
        (Behaviour::Finite(_), Behaviour::Finite(_)) => msg!("Removable gap", "Hebbare Lücke"),
        _ => msg!("Behaviour at the gap", "Verhalten an der Lücke"),
    };
    (left, right, step(AnalysisRule::LimitPole, explanation, latex))
}

// ---- bounded factors ---------------------------------------------------------------------------

/// A function with sin or cos of something that grows: bounded times something that vanishes
/// is 0, a bounded term next to something infinite does not matter, otherwise it oscillates.
fn bounded(g: &Expr) -> Option<(Behaviour, AnalysisRule, String)> {
    if !has_oscillation(g) {
        return None;
    }
    let squeeze = || msg!("Bounded factor times a factor that tends to 0", "Beschränkter Faktor mal Nullfolge");
    let oscillates = || msg!("sin and cos keep oscillating", "sin und cos schwingen immer weiter");
    match g {
        Expr::Product(factors) => {
            let (wave, rest): (Vec<Expr>, Vec<Expr>) = factors.iter().cloned().partition(is_oscillating);
            if wave.is_empty() {
                return None;
            }
            let rest = growth(&Expr::product(rest))?;
            Some(match rest.limit() {
                Behaviour::Finite(p) if p.approx == 0.0 => (Behaviour::Finite(Point::rational(RBig::ZERO)), AnalysisRule::LimitSqueeze, squeeze()),
                _ => (Behaviour::Oscillates, AnalysisRule::LimitSqueeze, oscillates()),
            })
        }
        Expr::Sum(terms) => {
            let (wave, rest): (Vec<Expr>, Vec<Expr>) = terms.iter().cloned().partition(has_oscillation);
            let rest = growth(&Expr::sum(rest))?;
            let mut vanishing = true;
            for term in &wave {
                match bounded(term) {
                    Some((Behaviour::Finite(p), ..)) if p.approx == 0.0 => {}
                    _ => vanishing = false,
                }
            }
            Some(match rest.limit() {
                b @ (Behaviour::PosInf | Behaviour::NegInf) => (
                    b,
                    AnalysisRule::LimitSqueeze,
                    msg!("The bounded sin or cos term does not matter next to the term that grows", "Der beschränkte sin- bzw. cos-Term spielt neben dem wachsenden Term keine Rolle"),
                ),
                finite if vanishing => (finite, AnalysisRule::LimitSqueeze, squeeze()),
                _ => (Behaviour::Oscillates, AnalysisRule::LimitSqueeze, oscillates()),
            })
        }
        _ if is_oscillating(g) => Some((Behaviour::Oscillates, AnalysisRule::LimitSqueeze, oscillates())),
        _ => None,
    }
}

fn is_oscillating(e: &Expr) -> bool {
    match e {
        Expr::Func(Fn::Sin | Fn::Cos, u) => growth(u).is_some_and(|g| g.limit().is_infinite()),
        Expr::Pow(base, exponent) => exponent.as_num().is_some_and(|r| r.is_positive()) && is_oscillating(base),
        _ => false,
    }
}

fn has_oscillation(e: &Expr) -> bool {
    is_oscillating(e)
        || match e {
            Expr::Sum(items) | Expr::Product(items) => items.iter().any(has_oscillation),
            _ => false,
        }
}

// ---- growth orders -----------------------------------------------------------------------------

const EXP: u8 = 1;
const POWER: u8 = 2;
const LOG: u8 = 4;

/// A term as t → ∞: `coefficient · e^{exp·t} · t^{power} · (\ln t)^{log}`.
#[derive(Clone, Debug)]
struct Growth {
    coefficient: Expr,
    exp: RBig,
    power: RBig,
    log: RBig,
    /// Which kinds of growth met on the way, for the explanation.
    kinds: u8,
}

impl Growth {
    fn constant(c: Expr) -> Growth {
        Growth { coefficient: c, exp: RBig::ZERO, power: RBig::ZERO, log: RBig::ZERO, kinds: 0 }
    }

    fn order(&self) -> (RBig, RBig, RBig) {
        (self.exp.clone(), self.power.clone(), self.log.clone())
    }

    fn times(self, other: Growth) -> Growth {
        Growth {
            coefficient: simplify(&Expr::Product(vec![self.coefficient, other.coefficient])),
            exp: self.exp + other.exp,
            power: self.power + other.power,
            log: self.log + other.log,
            kinds: self.kinds | other.kinds,
        }
    }

    fn limit(&self) -> Behaviour {
        let zero = RBig::ZERO;
        let order = self.order();
        if order == (zero.clone(), zero.clone(), zero.clone()) {
            return Point::exact(self.coefficient.clone()).map(Behaviour::Finite).unwrap_or(Behaviour::Unknown);
        }
        if order > (zero.clone(), zero.clone(), zero) {
            let sign = self.coefficient.eval_f64(0.0).unwrap_or(0.0);
            if sign == 0.0 {
                return Behaviour::Unknown;
            }
            return infinite(sign.signum() as i32);
        }
        Behaviour::Finite(Point::rational(RBig::ZERO))
    }

    fn explanation(&self) -> String {
        if self.kinds & EXP != 0 && self.kinds & (POWER | LOG) != 0 {
            msg!("The exponential function grows faster than any power of x", "Die e-Funktion wächst schneller als jede Potenz von x")
        } else if self.kinds & POWER != 0 && self.kinds & LOG != 0 {
            msg!("Any power x^a (a > 0) grows faster than ln x", "Jede Potenz x^a (a > 0) wächst schneller als ln x")
        } else {
            msg!("The term with the highest power decides", "Der Summand mit dem höchsten Exponenten bestimmt das Verhalten")
        }
    }
}

/// The growth of `e` (a function of t) as t → ∞, when it is made of powers, exponentials and
/// logarithms of t.
fn growth(e: &Expr) -> Option<Growth> {
    if e.is_constant() {
        return Some(Growth::constant(e.clone()));
    }
    match e {
        Expr::Var => Some(Growth { power: RBig::ONE, kinds: POWER, ..Growth::constant(Expr::int(1)) }),
        Expr::Pow(base, exponent) => {
            let r = exponent.as_num()?;
            let g = growth(base)?;
            let c = g.coefficient.eval_f64(0.0)?;
            if c < 0.0 && !r.is_int() {
                return None;
            }
            Some(Growth {
                coefficient: simplify(&Expr::pow(g.coefficient, Expr::Num(r.clone()))),
                exp: g.exp * r,
                power: g.power * r,
                log: g.log * r,
                kinds: g.kinds,
            })
        }
        Expr::Func(Fn::Exp, u) => {
            // e^{kt + c} = e^c · e^{kt}
            let slope = simplify(&super::derive::derivative(u));
            let k = slope.as_num()?.clone();
            let c = simplify(&u.substitute(&Expr::int(0)));
            Some(Growth { coefficient: simplify(&Expr::func(Fn::Exp, c)), exp: k, kinds: EXP, ..Growth::constant(Expr::int(1)) })
        }
        Expr::Func(Fn::Ln, u) if **u == Expr::Var => Some(Growth { log: RBig::ONE, kinds: LOG, ..Growth::constant(Expr::int(1)) }),
        Expr::Func(Fn::Arctan, u) => {
            let inner = growth(u)?;
            let half_pi = Expr::Product(vec![Expr::ratio(1, 2), Expr::Pi]);
            let value = match inner.limit() {
                Behaviour::PosInf => half_pi,
                Behaviour::NegInf => Expr::neg(half_pi),
                Behaviour::Finite(p) => Expr::func(Fn::Arctan, p.exact?),
                _ => return None,
            };
            Some(Growth::constant(simplify(&value)))
        }
        Expr::Product(factors) => factors.iter().map(growth).collect::<Option<Vec<_>>>()?.into_iter().reduce(Growth::times),
        Expr::Sum(terms) => {
            let parts = terms.iter().map(growth).collect::<Option<Vec<_>>>()?;
            let kinds = parts.iter().fold(0, |k, p| k | p.kinds);
            let top = parts.iter().map(Growth::order).max()?;
            let leading: Vec<Expr> = parts.iter().filter(|p| p.order() == top).map(|p| p.coefficient.clone()).collect();
            let coefficient = simplify(&Expr::sum(leading));
            if is_zero(&coefficient) {
                return None; // the leading terms cancel: not decided here
            }
            Some(Growth { coefficient, exp: top.0, power: top.1, log: top.2, kinds })
        }
        _ => None,
    }
}

// ---- numeric -----------------------------------------------------------------------------------

/// Samples ever closer to the approach: growing beyond bounds, settling, or swinging.
fn numeric(f: &Expr, at: &Approach) -> Behaviour {
    let values: Vec<f64> = (1..=12).filter_map(|k| f.eval_f64(at.sample_at(10f64.powi(k)))).collect();
    let shifted: Vec<f64> = (1..=12).filter_map(|k| f.eval_f64(at.sample_at(1.37 * 10f64.powi(k)))).collect();
    if values.len() < 6 {
        return Behaviour::Unknown;
    }
    let tail = &values[values.len() - 4..];
    let same_sign = tail.iter().all(|v| v.signum() == tail[0].signum());
    let growing = tail.windows(2).all(|w| w[1].abs() > w[0].abs());
    if same_sign && growing && tail[3].abs() > 1e9 {
        return infinite(tail[0].signum() as i32);
    }
    let last = values[values.len() - 1];
    let scale = last.abs().max(1.0);
    let settled = (values[values.len() - 2] - last).abs() < 1e-7 * scale
        && shifted.last().is_some_and(|s| (s - last).abs() < 1e-7 * scale);
    if settled {
        return Behaviour::Finite(Point::approx(last));
    }
    let swings = values.windows(2).filter(|w| w[0].signum() != w[1].signum()).count() >= 2
        || shifted.iter().zip(&values).any(|(s, v)| (s - v).abs() > 1e-3 * v.abs().max(1.0));
    let bounded = values.iter().all(|v| v.abs() < 1e12);
    if swings && bounded {
        return Behaviour::Oscillates;
    }
    Behaviour::Unknown
}

// ---- asymptotes --------------------------------------------------------------------------------

/// `y = mx + b` that f approaches at ±∞, with the working (a polynomial division for a rational
/// function), or `None` when there is none.
pub fn oblique(f: &Expr, at: &Approach, ctx: &LimitContext) -> Option<(Expr, Expr, Vec<Step>)> {
    let x = Expr::poly_variable();
    if let Some((n, d)) = f.as_rational() {
        let common = n.gcd(&d);
        let (n, d) = (n.div_rem(&common).ok()?.0, d.div_rem(&common).ok()?.0);
        if n.degree_of(x) != d.degree_of(x) + 1 {
            return None;
        }
        let division = divide(&n, &d, ctx.var);
        let slope = Expr::Num(division.quotient.coeff_of(&Monomial::single(x, 1)));
        let intercept = Expr::Num(division.quotient.constant_term());
        return Some((slope, intercept, division.steps));
    }
    let (m, step_m) = behaviour(&simplify(&Expr::Product(vec![f.clone(), Expr::recip(Expr::Var)])), at, ctx);
    let Behaviour::Finite(m) = m else { return None };
    let slope = m.exact.clone()?;
    if is_zero(&slope) {
        return None;
    }
    let rest = simplify(&Expr::Sum(vec![f.clone(), Expr::neg(Expr::Product(vec![slope.clone(), Expr::Var]))]));
    let (b, step_b) = behaviour(&rest, at, ctx);
    let Behaviour::Finite(b) = b else { return None };
    let mut steps = vec![step_m, step_b];
    for (s, name) in steps.iter_mut().zip(["m", "b"]) {
        s.explanation = Some(format!("{} ({name})", s.explanation.clone().unwrap_or_default()));
    }
    Some((slope, b.exact?, steps))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::latex::parser;
    use crate::settings::{Mode, Settings};

    fn expr(latex: &str) -> Expr {
        let options = Settings { mode: Mode::Algebra, ..Settings::default() }.parse_options();
        Expr::from_node(&parser::parse_with(latex, &options).unwrap().expression, "x").unwrap()
    }

    const CTX: LimitContext = LimitContext { var: "x", digits: 6 };

    fn limit(latex: &str, at: Approach) -> String {
        behaviour(&expr(latex), &at, &CTX).0.to_latex(6)
    }

    fn at(r: i64) -> Point {
        Point::rational(RBig::from(r))
    }

    #[test]
    fn rational_functions_compare_degrees() {
        assert_eq!(limit("x^3 - 3x", Approach::PosInf), "\\infty");
        assert_eq!(limit("x^3 - 3x", Approach::NegInf), "-\\infty");
        assert_eq!(limit("-x^4 + x", Approach::NegInf), "-\\infty");
        assert_eq!(limit("\\frac{1}{x^2+1}", Approach::PosInf), "0");
        assert_eq!(limit("\\frac{3x^2}{2x^2 + 1}", Approach::NegInf), "\\frac{3}{2}");
        assert_eq!(limit("\\frac{x^2-1}{x-2}", Approach::NegInf), "-\\infty");
        let (_, step) = behaviour(&expr("\\frac{1}{x^2+1}"), &Approach::PosInf, &CTX);
        assert_eq!(step.latex, "\\lim_{x \\to \\infty} f(x) = 0");
        assert_eq!(step.rule.as_deref(), Some("LIMIT_DEGREE"));
    }

    #[test]
    fn poles_and_removable_gaps() {
        let (left, right, step) = pole(&expr("\\frac{x^2-1}{x-2}"), &at(2), &CTX);
        assert!(matches!((left, right), (Behaviour::NegInf, Behaviour::PosInf)));
        assert_eq!(step.explanation.as_deref(), Some("Pole with a sign change"));
        assert_eq!(step.latex, "\\lim_{x \\to 2^{-}} f(x) = -\\infty, \\quad \\lim_{x \\to 2^{+}} f(x) = \\infty");
        let (left, right, _) = pole(&expr("\\frac{1}{x^2}"), &at(0), &CTX);
        assert!(matches!((left, right), (Behaviour::PosInf, Behaviour::PosInf)));
        assert_eq!(limit("\\frac{x^2-1}{x-1}", Approach::Right(at(1))), "2");
        let root = Point::exact(Expr::pow(Expr::int(3), Expr::ratio(1, 2))).unwrap();
        assert_eq!(limit("\\frac{1}{x^2-3}", Approach::Right(root)), "\\infty");
    }

    #[test]
    fn growth_orders_decide_mixed_functions() {
        assert_eq!(limit("x e^x", Approach::NegInf), "0");
        assert_eq!(limit("x e^x", Approach::PosInf), "\\infty");
        assert_eq!(limit("\\frac{\\ln x}{x}", Approach::PosInf), "0");
        assert_eq!(limit("\\frac{\\ln x}{x}", Approach::Right(at(0))), "-\\infty");
        assert_eq!(limit("x \\ln x", Approach::Right(at(0))), "0");
        assert_eq!(limit("x - \\ln x", Approach::PosInf), "\\infty");
        assert_eq!(limit("e^{-x} + 2", Approach::PosInf), "2");
        assert_eq!(limit("\\arctan x", Approach::NegInf), "-\\frac{\\pi}{2}");
        assert_eq!(limit("\\sqrt{x}", Approach::PosInf), "\\infty");
        let (_, step) = behaviour(&expr("\\frac{\\ln x}{x}"), &Approach::PosInf, &CTX);
        assert_eq!(step.explanation.as_deref(), Some("Any power x^a (a > 0) grows faster than ln x"));
        let (_, step) = behaviour(&expr("x e^x"), &Approach::NegInf, &CTX);
        assert_eq!(step.explanation.as_deref(), Some("The exponential function grows faster than any power of x"));
    }

    #[test]
    fn sine_and_cosine_are_bounded() {
        assert_eq!(limit("\\sin x", Approach::PosInf), "\\text{does not exist (oscillates)}");
        assert_eq!(limit("\\frac{\\sin x}{x}", Approach::PosInf), "0");
        assert_eq!(limit("x + \\sin x", Approach::NegInf), "-\\infty");
        assert_eq!(limit("x \\sin x", Approach::PosInf), "\\text{does not exist (oscillates)}");
    }

    #[test]
    fn the_rest_is_sampled() {
        assert_eq!(limit("\\tan x", Approach::Left(Point::exact(Expr::Product(vec![Expr::ratio(1, 2), Expr::Pi])).unwrap())), "\\infty");
        assert_eq!(limit("x^{\\frac{1}{x}}", Approach::PosInf), "\\approx 1");
        assert_eq!(limit("e^{\\frac{1}{x}}", Approach::Left(at(0))), "0");
    }

    #[test]
    fn oblique_asymptotes() {
        let (m, b, steps) = oblique(&expr("\\frac{x^2-1}{x-2}"), &Approach::PosInf, &CTX).unwrap();
        assert_eq!((m.to_latex("x"), b.to_latex("x")), ("1".into(), "2".into()));
        assert_eq!(steps.last().unwrap().latex, "(x^{2} - 1) : (x - 2) = x + 2 + \\frac{3}{x - 2}");
        let (m, b, _) = oblique(&expr("2x + 1 + e^{-x}"), &Approach::PosInf, &CTX).unwrap();
        assert_eq!((m.to_latex("x"), b.to_latex("x")), ("2".into(), "1".into()));
        assert!(oblique(&expr("x^2"), &Approach::PosInf, &CTX).is_none());
        assert!(oblique(&expr("\\frac{1}{x}"), &Approach::PosInf, &CTX).is_none());
    }
}
