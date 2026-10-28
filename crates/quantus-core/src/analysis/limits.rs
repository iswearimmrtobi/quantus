//! Behaviour at the edges of the domain and at poles, and the asymptotes that follow from it.
//!
//! Every approach is turned into t → ∞: x = t, x = -t, or x = a ± 1/t. Rational functions are
//! decided by their degrees (at ±∞) or the sign next to the pole; other functions by growth
//! orders, where a term behaves like `c·e^{bt}·t^a·(\ln t)^l` and the largest (b, a, l) wins;
//! sin and cos are bounded; and what none of that decides is sampled numerically.

use dashu::base::{Abs, Signed};
use dashu::rational::RBig;

use super::expr::{Expr, Fn};
use super::interval::Point;
use super::polydiv::divide;
use super::rules::{step, AnalysisRule};
use super::simplify::{is_zero, simplify};
use crate::algebra::poly::{Monomial, Poly};
use crate::eval::Step;
use crate::settings::StepDetail;
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
            Approach::Left(a) => a.approx - 1.0 / t,
            Approach::Right(a) => a.approx + 1.0 / t,
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
            // D(x) ≈ D^{(k)}(x₀)/k! · (x − x₀)^k next to a zero of order k, so the sign on the
            // right is that of N(x₀)·D^{(k)}(x₀), and on the left it flips once more for odd k.
            let (order, derivative_sign) = zero_order(&below, a)?;
            let numerator_sign = sign_of(&Expr::from_poly(&n), a)?;
            let right = numerator_sign * derivative_sign;
            let sign = match at {
                Approach::Left(_) if order % 2 == 1 => -right,
                _ => right,
            };
            let explanation = msg!(
                "Pole of order {}: the signs of the numerator and of (x - x_0)^{} decide",
                "Polstelle der Ordnung {}: die Vorzeichen des Zählers und von (x - x_0)^{} entscheiden",
                order,
                order
            );
            Some((infinite(sign), AnalysisRule::LimitPole, explanation))
        }
    }
}

/// The order k of the zero of `p` at `a` and the sign of its k-th derivative there.
fn zero_order(p: &Expr, a: &Point) -> Option<(u32, i32)> {
    let mut derivative = p.clone();
    for k in 1..=64u32 {
        derivative = simplify(&super::derive::derivative(&derivative));
        let sign = sign_of(&derivative, a)?;
        if sign != 0 {
            return Some((k, sign));
        }
    }
    None
}

/// The sign of `e` at `a`: exactly for an exact point, otherwise in floating point with a
/// tolerance for the error in `a`.
fn sign_of(e: &Expr, a: &Point) -> Option<i32> {
    let value = match &a.exact {
        Some(x) => {
            let value = simplify(&e.substitute(x));
            if is_zero(&value) {
                return Some(0);
            }
            value.eval_f64(0.0)?
        }
        None => {
            let value = e.eval_f64(a.approx)?;
            if value.abs() < 1e-9 {
                return Some(0);
            }
            value
        }
    };
    Some(if value > 0.0 { 1 } else { -1 })
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

// ---- the working behind a limit ----------------------------------------------------------------

/// `p` divided by `x^{n}`, term by term: `1 - \frac{3}{x} + \frac{2}{x^{3}}` for `x^3 - 3x^2 + 2`
/// and n = 3, with the leading coefficient `scale` divided out as well.
fn over_power(p: &Poly, n: usize, scale: &RBig, var: &str) -> String {
    let x = Expr::poly_variable();
    let mut terms: Vec<(usize, RBig)> = p.terms.iter().map(|t| (t.monomial.degree_of(x), t.coeff.clone() / scale)).collect();
    terms.sort_by(|a, b| b.0.cmp(&a.0));
    let mut out = String::new();
    for (index, (k, c)) in terms.into_iter().enumerate() {
        let size = Expr::Num(c.clone().abs()).to_latex(var);
        let term = match n - k {
            0 => size,
            1 => format!("\\frac{{{size}}}{{{var}}}"),
            m => format!("\\frac{{{size}}}{{{var}^{{{m}}}}}"),
        };
        let sign = match (index, c.is_negative()) {
            (0, false) => "",
            (0, true) => "-",
            (_, false) => " + ",
            (_, true) => " - ",
        };
        out.push_str(&format!("{sign}{term}"));
    }
    out
}

/// The lines before a limit at ±∞ of a polynomial or of a quotient of equal degrees: the highest
/// power taken out, and what the rest tends to.
pub fn infinity_working(f: &Expr, at: &Approach, ctx: &LimitContext) -> Vec<Step> {
    let (Approach::PosInf | Approach::NegInf) = at else { return Vec::new() };
    let Some((numerator, denominator)) = f.as_rational() else { return Vec::new() };
    let x = Expr::poly_variable();
    let var = ctx.var;
    let (deg_n, deg_d) = (numerator.degree_of(x), denominator.degree_of(x));
    let leading = |p: &Poly| p.coeff_of(&Monomial::single(x, p.degree_of(x)));
    let limit = at.to_latex(var, ctx.digits);
    if denominator.is_constant() && deg_n >= 1 && numerator.terms.len() > 1 {
        let c = leading(&numerator) / denominator.constant_term();
        let power = Expr::Product(vec![Expr::Num(c.clone()), Expr::pow(Expr::Var, Expr::int(deg_n as i64))]);
        let power = simplify(&power).to_latex(var);
        let bracket = over_power(&numerator, deg_n, &leading(&numerator), var);
        let factored = format!("f({var}) = {power}\\left({bracket}\\right)");
        let result = decide(f, at).0.to_latex(ctx.digits);
        let tends = format!("\\lim_{{{limit}}} {power} \\cdot 1 = {result}");
        return vec![
            step(AnalysisRule::LimitDegree, msg!("Factor out the highest power", "Höchste Potenz ausklammern"), factored).with_level(StepDetail::Standard),
            step(AnalysisRule::LimitDegree, msg!("The bracket tends to 1", "Die Klammer geht gegen 1"), tends).with_level(StepDetail::Standard),
        ];
    }
    if deg_n == deg_d && deg_n >= 1 {
        let divided = format!(
            "f({var}) = \\frac{{{}}}{{{}}}",
            over_power(&numerator, deg_n, &RBig::ONE, var),
            over_power(&denominator, deg_d, &RBig::ONE, var)
        );
        let text = msg!("Divide numerator and denominator by {}^{}", "Zähler und Nenner durch {}^{} teilen", var, deg_n);
        return vec![step(AnalysisRule::LimitDegree, text, divided).with_level(StepDetail::Standard)];
    }
    Vec::new()
}

/// The lines before the two sides of a pole: the numerator at the point, and which side of 0 the
/// denominator comes from.
pub fn pole_working(f: &Expr, at: &Point, ctx: &LimitContext) -> Vec<Step> {
    let (Some((numerator, denominator)), Some(a)) = (f.as_rational(), &at.exact) else { return Vec::new() };
    let common = numerator.gcd(&denominator);
    let (n, d) = match (numerator.div_rem(&common), denominator.div_rem(&common)) {
        (Ok((n, _)), Ok((d, _))) => (Expr::from_poly(&n), Expr::from_poly(&d)),
        _ => return Vec::new(),
    };
    let (Some(numerator_sign), Some((order, derivative_sign))) = (sign_of(&n, at), zero_order(&d, at)) else { return Vec::new() };
    if numerator_sign == 0 {
        return Vec::new();
    }
    let var = ctx.var;
    let value = simplify(&n.substitute(a)).to_latex(var);
    let compared = if numerator_sign > 0 { "> 0" } else { "< 0" };
    let numerator_line = format!("{} = {value} {compared}", super::rules::put_in(&n, a, var));
    let side = |sign: i32| if sign > 0 { "0^{+}" } else { "0^{-}" };
    let left = if order % 2 == 1 { -derivative_sign } else { derivative_sign };
    let (d_tex, a_tex) = (d.to_latex(var), at.to_latex(ctx.digits));
    let denominator_line = format!(
        "{}: {d_tex} \\to {}, \\quad {}: {d_tex} \\to {}",
        Approach::Left(at.clone()).to_latex(var, ctx.digits),
        side(left),
        Approach::Right(at.clone()).to_latex(var, ctx.digits),
        side(derivative_sign)
    );
    vec![
        step(AnalysisRule::LimitPole, msg!("The numerator at {}", "Der Zähler bei {}", a_tex), numerator_line).with_level(StepDetail::Standard),
        step(AnalysisRule::LimitPole, msg!("Sign of the denominator", "Vorzeichen des Nenners"), denominator_line).with_level(StepDetail::Standard),
    ]
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

/// The exponential rate standing for "e to something that grows faster than t".
const SUPER_EXPONENTIAL: i64 = 1_000_000;

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
            let Some(k) = slope.as_num().cloned() else {
                // e^{t^2} and the like: faster (or slower) than any e^{kt} the rest can bring.
                let inner = growth(u)?;
                return match inner.limit() {
                    Behaviour::PosInf => Some(Growth { exp: RBig::from(SUPER_EXPONENTIAL), kinds: EXP, ..Growth::constant(Expr::int(1)) }),
                    Behaviour::NegInf => Some(Growth { exp: -RBig::from(SUPER_EXPONENTIAL), kinds: EXP, ..Growth::constant(Expr::int(1)) }),
                    Behaviour::Finite(p) => Some(Growth::constant(simplify(&Expr::func(Fn::Exp, p.exact?)))),
                    _ => None,
                };
            };
            let c = simplify(&u.substitute(&Expr::int(0)));
            Some(Growth { coefficient: simplify(&Expr::func(Fn::Exp, c)), exp: k, kinds: EXP, ..Growth::constant(Expr::int(1)) })
        }
        Expr::Func(Fn::Ln, u) if **u == Expr::Var => Some(Growth { log: RBig::ONE, kinds: LOG, ..Growth::constant(Expr::int(1)) }),
        // ln(c·e^{bt}·t^a·…) = bt + a·ln t + …: the strongest part of the argument decides.
        Expr::Func(Fn::Ln, u) => {
            let inner = growth(u)?;
            if inner.coefficient.eval_f64(0.0)? <= 0.0 {
                return None;
            }
            if inner.exp > RBig::ZERO {
                Some(Growth { power: RBig::ONE, kinds: POWER | inner.kinds, ..Growth::constant(Expr::Num(inner.exp)) })
            } else if inner.exp == RBig::ZERO && inner.power > RBig::ZERO {
                Some(Growth { log: RBig::ONE, kinds: LOG | inner.kinds, ..Growth::constant(Expr::Num(inner.power)) })
            } else if inner.order() == (RBig::ZERO, RBig::ZERO, RBig::ZERO) {
                Some(Growth::constant(simplify(&Expr::func(Fn::Ln, inner.coefficient))))
            } else {
                None
            }
        }
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
    if super::budget::exceeded() {
        return Behaviour::Unknown;
    }
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
    let rises = values.windows(2).any(|w| w[1] > w[0]);
    let falls = values.windows(2).any(|w| w[1] < w[0]);
    let swings = values.windows(2).filter(|w| w[0].signum() != w[1].signum()).count() >= 2
        || shifted.iter().zip(&values).any(|(s, v)| (s - v).abs() > 1e-3 * v.abs().max(1.0))
        || (rises && falls);
    let bounded = values.iter().all(|v| v.abs() < 1e12);
    if swings && bounded {
        return Behaviour::Oscillates;
    }
    Behaviour::Unknown
}

// ---- asymptotes --------------------------------------------------------------------------------

/// `y = mx + b` that f approaches at ±∞, with the working (a polynomial division for a rational
/// function), or `None` when there is none.
pub fn oblique(f: &Expr, at: &Approach, ctx: &LimitContext) -> Option<(Point, Point, Vec<Step>)> {
    let x = Expr::poly_variable();
    if let Some((n, d)) = f.as_rational() {
        let common = n.gcd(&d);
        let (n, d) = (n.div_rem(&common).ok()?.0, d.div_rem(&common).ok()?.0);
        // A polynomial (also after cancelling) has no asymptote, not even itself.
        if d.is_constant() || n.degree_of(x) != d.degree_of(x) + 1 {
            return None;
        }
        let division = divide(&n, &d, ctx.var);
        let slope = Point::rational(division.quotient.coeff_of(&Monomial::single(x, 1)));
        let intercept = Point::rational(division.quotient.constant_term());
        return Some((slope, intercept, division.steps));
    }
    let (m, step_m) = behaviour(&simplify(&Expr::Product(vec![f.clone(), Expr::recip(Expr::Var)])), at, ctx);
    let Behaviour::Finite(m) = m else { return None };
    if m.approx.abs() < 1e-12 || m.exact.as_ref().is_some_and(is_zero) {
        return None;
    }
    // A slope found only numerically is taken as the decimal it was shown with.
    let slope = m.exact.clone().unwrap_or_else(|| Expr::Num(RBig::try_from(m.approx).unwrap_or(RBig::ZERO)));
    let rest = simplify(&Expr::Sum(vec![f.clone(), Expr::neg(Expr::Product(vec![slope, Expr::Var]))]));
    let (b, step_b) = behaviour(&rest, at, ctx);
    let Behaviour::Finite(b) = b else { return None };
    let mut steps = vec![step_m, step_b];
    for (s, name) in steps.iter_mut().zip(["m", "b"]) {
        s.explanation = Some(format!("{} ({name})", s.explanation.clone().unwrap_or_default()));
    }
    Some((m, b, steps))
}

/// The polynomial part of a rational function whose numerator degree exceeds the denominator's
/// by two or more: the curve f approaches (Näherungskurve), with the division.
pub fn approximating_curve(f: &Expr, ctx: &LimitContext) -> Option<(Expr, Vec<Step>)> {
    let x = Expr::poly_variable();
    let (n, d) = f.as_rational()?;
    let common = n.gcd(&d);
    let (n, d) = (n.div_rem(&common).ok()?.0, d.div_rem(&common).ok()?.0);
    if d.is_constant() || n.degree_of(x) < d.degree_of(x) + 2 {
        return None;
    }
    let division = divide(&n, &d, ctx.var);
    Some((Expr::from_poly(&division.quotient), division.steps))
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
        assert_eq!(limit("\\frac{1}{x^2-3}", Approach::Right(root.clone())), "\\infty");
        assert_eq!(limit("\\frac{1}{x^2-3}", Approach::Left(root)), "-\\infty");
        let (left, right, step) = pole(&expr("\\frac{-x}{(x-1)^2}"), &at(1), &CTX);
        assert!(matches!((left, right), (Behaviour::NegInf, Behaviour::NegInf)));
        assert_eq!(step.explanation.as_deref(), Some("Pole without a sign change"));
        let (_, s) = behaviour(&expr("\\frac{-x}{(x-1)^2}"), &Approach::Right(at(1)), &CTX);
        assert_eq!(s.explanation.as_deref(), Some("Pole of order 2: the signs of the numerator and of (x - x_0)^2 decide"));
        let (left, right, _) = pole(&expr("\\frac{1}{(x+2)^3}"), &at(-2), &CTX);
        assert!(matches!((left, right), (Behaviour::NegInf, Behaviour::PosInf)));
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
        assert_eq!(limit("\\ln(x+1)", Approach::PosInf), "\\infty");
        assert_eq!(limit("\\frac{\\ln(x^2+1)}{x}", Approach::PosInf), "0");
        assert_eq!(limit("\\frac{x}{\\ln(2x)}", Approach::PosInf), "\\infty");
        assert_eq!(limit("\\ln(e^{x} + 1) - 2x", Approach::PosInf), "-\\infty");
        let (_, step) = behaviour(&expr("\\frac{\\ln(x^2+1)}{x}"), &Approach::PosInf, &CTX);
        assert_eq!(step.rule.as_deref(), Some("LIMIT_DOMINANCE"));
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
        let (wave, step) = behaviour(&expr("\\frac{\\sin x}{\\sin x + 2}"), &Approach::PosInf, &CTX);
        assert!(matches!(wave, Behaviour::Oscillates));
        assert_eq!(step.rule.as_deref(), Some("LIMIT_NUMERIC"));
        assert!(matches!(behaviour(&expr("x^{x}"), &Approach::PosInf, &CTX).0, Behaviour::Unknown));
    }

    #[test]
    fn oblique_asymptotes() {
        let (m, b, steps) = oblique(&expr("\\frac{x^2-1}{x-2}"), &Approach::PosInf, &CTX).unwrap();
        assert_eq!((m.to_latex(6), b.to_latex(6)), ("1".into(), "2".into()));
        assert_eq!(steps.last().unwrap().latex, "(x^{2} - 1) : (x - 2) = x + 2 + \\frac{3}{x - 2}");
        let (m, b, _) = oblique(&expr("2x + 1 + e^{-x}"), &Approach::PosInf, &CTX).unwrap();
        assert_eq!((m.to_latex(6), b.to_latex(6)), ("2".into(), "1".into()));
        assert!(oblique(&expr("x^2"), &Approach::PosInf, &CTX).is_none());
        assert!(oblique(&expr("\\frac{1}{x}"), &Approach::PosInf, &CTX).is_none());
        assert!(oblique(&expr("\\frac{x^4}{x-1}"), &Approach::PosInf, &CTX).is_none());
        let (curve, steps) = approximating_curve(&expr("\\frac{x^4}{x-1}"), &CTX).unwrap();
        assert_eq!(curve.to_latex("x"), "x^{3} + x^{2} + x + 1");
        assert!(steps.last().unwrap().latex.starts_with("(x^{4}) : (x - 1) = x^{3} + x^{2} + x + 1"));
        assert!(approximating_curve(&expr("\\frac{x^2-1}{x-2}"), &CTX).is_none());
    }
}
