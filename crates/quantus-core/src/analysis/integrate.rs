//! Antiderivatives with the method named, the way a student finds them.
//!
//! The methods, in order: basic integrals with the sum and factor rule (inner functions may be
//! linear, `\int \cos(2x+1)\,dx = \frac{1}{2}\sin(2x+1)`); polynomials term by term; rational
//! functions by polynomial division and partial fractions; substitution `u = g(x)` when the
//! integrand is `h(g(x))·g'(x)`; and integration by parts for `x^n e^{ax}`, `x^n \sin(ax)`,
//! `x^n \ln x`, `\arctan x` and `e^{ax}\sin(bx)`.
//!
//! Every result is differentiated and compared with the integrand before it is shown; when no
//! method passes, there is no antiderivative rather than a wrong one.

use dashu::base::Signed;
use dashu::integer::{IBig, UBig};
use dashu::rational::RBig;

use super::derive::derivative;
use super::expr::{Expr, Fn};
use super::interval::Domain;
use super::polydiv::divide;
use super::rules::{step, AnalysisRule};
use super::simplify::{display, is_zero, is_zero_numeric, simplify};
use crate::algebra::factor::factor_polynomial;
use crate::algebra::poly::{Monomial, Poly};
use crate::eval::Step;
use crate::settings::Settings;
use crate::text::msg;

/// How deep substitution and parts may nest inside each other.
const MAX_DEPTH: usize = 4;

pub struct Integration {
    /// F without the constant C.
    pub result: Expr,
    pub method: AnalysisRule,
    pub steps: Vec<Step>,
}

pub struct IntegrateContext<'a> {
    pub settings: &'a Settings,
    pub var: &'a str,
    /// Where the check by differentiation samples when the exact check is inconclusive.
    pub domain: &'a Domain,
}

/// An antiderivative of `f` with its working, or `None` when no method finds one that checks.
pub fn antiderivative(f: &Expr, ctx: &IntegrateContext) -> Option<Integration> {
    let f = simplify(f);
    let tex = |e: &Expr| e.to_latex(ctx.var);
    let finish = |result: Expr, method: AnalysisRule, mut steps: Vec<Step>| -> Option<Integration> {
        if !verified(&result, &f, ctx.domain) {
            return None;
        }
        let shown = without_needless_abs(&display(&result, ctx.settings), ctx.domain);
        steps.push(step(method, method_text(method), format!("F({}) = {} + C", ctx.var, tex(&shown))));
        Some(Integration { result: shown, method, steps })
    };

    if let Some(result) = table(&f) {
        let polynomial = f.as_rational().is_some_and(|(_, d)| d.is_constant());
        let method = if polynomial { AnalysisRule::IntegralPolynomial } else { AnalysisRule::IntegralTable };
        if let Some(done) = finish(result, method, Vec::new()) {
            return Some(done);
        }
    }
    if let Some((n, d)) = f.as_rational() {
        let mut steps = Vec::new();
        if let Some(result) = rational(&n, &d, Some((&mut steps, ctx.var))) {
            if let Some(done) = finish(result, AnalysisRule::PartialFractions, steps) {
                return Some(done);
            }
        }
    }
    if let Some((result, u, g, big_g)) = substitution(&f, 0) {
        let steps = vec![
            step(
                AnalysisRule::IntegralSubstitution,
                msg!("Substitution", "Substitution"),
                format!("u = {}, \\quad du = {} \\, d{}", tex(&u), tex(&display(&derivative(&u), ctx.settings)), ctx.var),
            ),
            step(
                AnalysisRule::IntegralSubstitution,
                msg!("Integrate in u, then substitute back", "In u integrieren, dann zurück substituieren"),
                format!("\\int {} \\, du = {}", g.to_latex("u"), big_g.to_latex("u")),
            ),
        ];
        if let Some(done) = finish(result, AnalysisRule::IntegralSubstitution, steps) {
            return Some(done);
        }
    }
    if let Some((result, parts)) = by_parts(&f, 0) {
        let latex = match &parts {
            Parts::Plain { u, v_prime, v } => format!(
                "u = {}, \\quad v' = {}, \\quad u' = {}, \\quad v = {}",
                tex(u),
                tex(v_prime),
                tex(&display(&derivative(u), ctx.settings)),
                tex(&display(v, ctx.settings))
            ),
            Parts::Recurring => "\\int e^{ax} \\sin(bx) \\, dx".to_string(),
        };
        let explanation = match parts {
            Parts::Plain { .. } => msg!("Integration by parts: ∫u·v' dx = u·v − ∫u'·v dx with", "Partielle Integration: ∫u·v' dx = u·v − ∫u'·v dx mit"),
            Parts::Recurring => msg!(
                "Integrating by parts twice brings back the same integral, which is then solved for",
                "Zweimal partiell integrieren liefert dasselbe Integral wieder, nach dem dann aufgelöst wird"
            ),
        };
        let steps = vec![step(AnalysisRule::IntegrationByParts, explanation, latex)];
        if let Some(done) = finish(result, AnalysisRule::IntegrationByParts, steps) {
            return Some(done);
        }
    }
    None
}

fn method_text(method: AnalysisRule) -> String {
    match method {
        AnalysisRule::IntegralPolynomial => msg!("Power rule, term by term", "Potenzregel, gliedweise"),
        AnalysisRule::PartialFractions => msg!("Integrate the partial fractions", "Partialbrüche integrieren"),
        AnalysisRule::IntegralSubstitution => msg!("Substitute back", "Rücksubstitution"),
        AnalysisRule::IntegrationByParts => msg!("Integration by parts", "Partielle Integration"),
        _ => msg!("Basic integrals with the sum and factor rule", "Grundintegrale mit Summen- und Faktorregel"),
    }
}

/// F' = f, exactly or at sample points of the domain.
fn verified(big_f: &Expr, f: &Expr, domain: &Domain) -> bool {
    let difference = Expr::Sum(vec![derivative(big_f), Expr::neg(f.clone())]);
    is_zero(&difference) || is_zero_numeric(&simplify(&difference), &domain.sample_points(9))
}

/// `\ln|u|` becomes `\ln(u)` where u is positive on the whole domain.
fn without_needless_abs(e: &Expr, domain: &Domain) -> Expr {
    match e {
        Expr::Func(Fn::Ln, argument) => match &**argument {
            Expr::Func(Fn::Abs, u) if positive_on(u, domain) => Expr::func(Fn::Ln, without_needless_abs(u, domain)),
            other => Expr::func(Fn::Ln, without_needless_abs(other, domain)),
        },
        Expr::Sum(items) => Expr::Sum(items.iter().map(|i| without_needless_abs(i, domain)).collect()),
        Expr::Product(items) => Expr::Product(items.iter().map(|i| without_needless_abs(i, domain)).collect()),
        Expr::Pow(b, x) => Expr::pow(without_needless_abs(b, domain), without_needless_abs(x, domain)),
        Expr::Func(function, argument) => Expr::func(*function, without_needless_abs(argument, domain)),
        other => other.clone(),
    }
}

fn positive_on(u: &Expr, domain: &Domain) -> bool {
    // A polynomial without real zeros and positive somewhere is positive everywhere.
    if let Some((n, d)) = u.as_rational() {
        if d.is_constant() && !n.is_constant() && crate::algebra::domain::has_no_real_roots(&n) {
            return u.eval_f64(0.0).is_some_and(|v| v > 0.0);
        }
    }
    // Otherwise only where the domain bounds it: \ln x on ]0; ∞[.
    let points = domain.sample_points(9);
    let lowest = domain.set.0.first().map(|i| i.lo.value()).unwrap_or(f64::NEG_INFINITY);
    *u == Expr::Var && lowest >= 0.0 && !points.is_empty()
}

// ---- basic integrals ---------------------------------------------------------------------------

/// `(a, b)` for an inner function `ax + b` with a constant `a ≠ 0`.
fn linear(u: &Expr) -> Option<(Expr, Expr)> {
    let a = simplify(&derivative(u));
    if !a.is_constant() || is_zero(&a) {
        return None;
    }
    let b = simplify(&u.substitute(&Expr::int(0)));
    Some((a, b))
}

/// Basic integrals with the sum and factor rule.
fn table(e: &Expr) -> Option<Expr> {
    if e.is_constant() {
        return Some(Expr::Product(vec![e.clone(), Expr::Var]));
    }
    match e {
        Expr::Sum(terms) => Some(Expr::Sum(terms.iter().map(table).collect::<Option<Vec<_>>>()?)),
        Expr::Product(factors) => {
            let (constant, rest): (Vec<Expr>, Vec<Expr>) = factors.iter().cloned().partition(Expr::is_constant);
            if constant.is_empty() || rest.is_empty() {
                return None;
            }
            Some(Expr::Product(vec![Expr::product(constant), table(&Expr::product(rest))?]))
        }
        _ => leaf(e),
    }
}

/// One basic integral with a linear inner function, divided by its slope.
fn leaf(e: &Expr) -> Option<Expr> {
    let over = |result: Expr, a: &Expr| Some(Expr::Product(vec![result, Expr::recip(a.clone())]));
    match e {
        Expr::Var => Some(Expr::Product(vec![Expr::ratio(1, 2), Expr::pow(Expr::Var, Expr::int(2))])),
        Expr::Pow(base, exponent) if exponent.is_constant() => {
            if let Some((a, _)) = linear(base) {
                if is_zero(&Expr::Sum(vec![(**exponent).clone(), Expr::int(1)])) {
                    return over(Expr::func(Fn::Ln, Expr::func(Fn::Abs, (**base).clone())), &a);
                }
                let raised = simplify(&Expr::Sum(vec![(**exponent).clone(), Expr::int(1)]));
                return over(Expr::Product(vec![Expr::pow((**base).clone(), raised.clone()), Expr::recip(raised)]), &a);
            }
            // \int \frac{1}{\cos^2 u} = \tan u
            if let (Expr::Func(Fn::Cos, u), Some(r)) = (&**base, exponent.as_num()) {
                if *r == RBig::from(-2) {
                    let (a, _) = linear(u)?;
                    return over(Expr::func(Fn::Tan, (**u).clone()), &a);
                }
            }
            // \int (c - kx^2)^{-1/2} = \frac{1}{\sqrt{k}} \arcsin(\sqrt{k/c}\,x)
            if exponent.as_num() == Some(&RBig::from_parts(IBig::from(-1), UBig::from(2u8))) {
                let (n, d) = base.as_rational()?;
                let x = Expr::poly_variable();
                if d.is_constant() && n.degree_of(x) == 2 {
                    let d0 = d.constant_term();
                    let (k, b, c) = (-n.coeff_of(&Monomial::single(x, 2)) / &d0, n.coeff_of(&Monomial::single(x, 1)), n.constant_term() / &d0);
                    if k.is_positive() && c.is_positive() && b == RBig::ZERO {
                        let scale = Expr::pow(Expr::Num(k.clone() / &c), Expr::ratio(1, 2));
                        let arcsin = Expr::func(Fn::Arcsin, Expr::Product(vec![scale, Expr::Var]));
                        return Some(Expr::Product(vec![Expr::pow(Expr::Num(k), Expr::ratio(-1, 2)), arcsin]));
                    }
                }
            }
            None
        }
        // \int c^{u} = \frac{c^u}{\ln c}
        Expr::Pow(base, exponent) if base.is_constant() => {
            let (a, _) = linear(exponent)?;
            over(Expr::Product(vec![e.clone(), Expr::recip(Expr::func(Fn::Ln, (**base).clone()))]), &a)
        }
        Expr::Func(function, u) => {
            let (a, _) = linear(u)?;
            let u = (**u).clone();
            let result = match function {
                Fn::Exp => e.clone(),
                Fn::Sin => Expr::neg(Expr::func(Fn::Cos, u)),
                Fn::Cos => Expr::func(Fn::Sin, u),
                Fn::Tan => Expr::neg(Expr::func(Fn::Ln, Expr::func(Fn::Abs, Expr::func(Fn::Cos, u)))),
                Fn::Sinh => Expr::func(Fn::Cosh, u),
                Fn::Cosh => Expr::func(Fn::Sinh, u),
                Fn::Ln => Expr::Sum(vec![Expr::Product(vec![u.clone(), Expr::func(Fn::Ln, u.clone())]), Expr::neg(u)]),
                _ => return None,
            };
            over(result, &a)
        }
        _ => None,
    }
}

// ---- rational functions ------------------------------------------------------------------------

/// `\int \frac{n}{d}` by polynomial division and partial fractions, with the working pushed to
/// `steps` when asked for.
fn rational(n: &Poly, d: &Poly, mut steps: Option<(&mut Vec<Step>, &str)>) -> Option<Expr> {
    let x = Expr::poly_variable();
    if d.is_constant() {
        return table(&simplify(&Expr::Product(vec![Expr::from_poly(n), Expr::recip(Expr::from_poly(d))])));
    }
    let common = n.gcd(d);
    let (n, d) = (n.div_rem(&common).ok()?.0, d.div_rem(&common).ok()?.0);
    let (quotient, remainder) = if n.degree_of(x) >= d.degree_of(x) {
        let division = divide(&n, &d, steps.as_ref().map(|(_, var)| *var).unwrap_or("x"));
        if let Some((list, _)) = steps.as_mut() {
            list.extend(division.steps);
        }
        (division.quotient, division.remainder)
    } else {
        (Poly::zero(), n.clone())
    };
    let mut parts: Vec<Expr> = Vec::new();
    let mut integral: Vec<Expr> = vec![table(&Expr::from_poly(&quotient))?];
    if !remainder.is_zero() {
        let factored = factor_polynomial(&d);
        let terms = partial_fractions(&remainder, &factored.coeff, &factored.factors)?;
        for (numerator, base, power) in terms {
            let fraction = Expr::Product(vec![Expr::from_poly(&numerator), Expr::pow(Expr::from_poly(&base), Expr::int(-(power as i64)))]);
            parts.push(fraction);
            integral.push(fraction_integral(&numerator, &base, power)?);
        }
        if let Some((list, var)) = steps.as_mut() {
            let shown = Expr::Product(vec![Expr::from_poly(&remainder), Expr::recip(Expr::from_poly(&d))]);
            let latex = format!("{} = {}", shown.to_latex(var), Expr::Sum(parts).to_latex(var));
            list.push(step(AnalysisRule::PartialFractions, msg!("Partial fraction decomposition", "Partialbruchzerlegung"), latex));
        }
    }
    Some(Expr::Sum(integral))
}

/// `remainder / (coeff · Π base^exp)` as a sum of `numerator / base^power`, numerators constant
/// for linear bases and linear for quadratic ones. `None` for a repeated quadratic or a base of
/// degree three or more.
fn partial_fractions(remainder: &Poly, coeff: &RBig, factors: &[crate::algebra::factored::Factor]) -> Option<Vec<(Poly, Poly, usize)>> {
    let x = Expr::poly_variable();
    let product: Poly = factors.iter().fold(Expr::poly_from_coefficients(&[RBig::ONE]), |p, f| &p * &f.base.pow(f.exp));
    let degree = product.degree_of(x);
    let target = remainder.clone() / coeff;
    // Unknowns: (base, power, is_x_term) and the polynomial they multiply.
    let mut unknowns: Vec<(Poly, usize, bool, Poly)> = Vec::new();
    for factor in factors {
        match factor.base.degree_of(x) {
            1 => {
                for power in 1..=factor.exp {
                    let basis = product.div_rem(&factor.base.pow(power)).ok()?.0;
                    unknowns.push((factor.base.clone(), power, false, basis));
                }
            }
            2 if factor.exp == 1 => {
                let basis = product.div_rem(&factor.base).ok()?.0;
                unknowns.push((factor.base.clone(), 1, true, &basis * &Expr::poly_from_coefficients(&[RBig::ZERO, RBig::ONE])));
                unknowns.push((factor.base.clone(), 1, false, basis));
            }
            _ => return None,
        }
    }
    // Coefficients of x^k on both sides: a square system.
    let mut matrix: Vec<Vec<RBig>> = (0..degree)
        .map(|k| {
            let mut row: Vec<RBig> = unknowns.iter().map(|u| u.3.coeff_of(&Monomial::single(x, k))).collect();
            row.push(target.coeff_of(&Monomial::single(x, k)));
            row
        })
        .collect();
    let values = solve_linear(&mut matrix)?;
    let mut out: Vec<(Poly, Poly, usize)> = Vec::new();
    for ((base, power, is_x, _), value) in unknowns.into_iter().zip(values) {
        let numerator = if is_x { Expr::poly_from_coefficients(&[RBig::ZERO, value]) } else { Expr::poly_from_coefficients(&[value]) };
        match out.iter_mut().find(|(_, b, p)| *b == base && *p == power) {
            Some((n, _, _)) => *n = &*n + &numerator,
            None => out.push((numerator, base, power)),
        }
    }
    out.retain(|(n, _, _)| !n.is_zero());
    Some(out)
}

/// Gaussian elimination over ℚ on an augmented square matrix.
fn solve_linear(matrix: &mut [Vec<RBig>]) -> Option<Vec<RBig>> {
    let n = matrix.len();
    for col in 0..n {
        let pivot = (col..n).find(|&r| matrix[r][col] != RBig::ZERO)?;
        matrix.swap(col, pivot);
        let lead = matrix[col][col].clone();
        for value in matrix[col].iter_mut() {
            *value = value.clone() / &lead;
        }
        for row in 0..n {
            if row != col && matrix[row][col] != RBig::ZERO {
                let factor = matrix[row][col].clone();
                for k in col..=n {
                    let subtract = &factor * &matrix[col][k];
                    matrix[row][k] -= subtract;
                }
            }
        }
    }
    Some(matrix.iter().map(|row| row[n].clone()).collect())
}

/// `\int \frac{numerator}{base^{power}}` for a linear base, or a quadratic one with power 1.
fn fraction_integral(numerator: &Poly, base: &Poly, power: usize) -> Option<Expr> {
    let x = Expr::poly_variable();
    let c = |p: &Poly, k: usize| p.coeff_of(&Monomial::single(x, k));
    let b = Expr::from_poly(base);
    if base.degree_of(x) == 1 {
        let (a, value) = (c(base, 1), c(numerator, 0));
        let scale = Expr::Num(value / &a);
        return Some(if power == 1 {
            Expr::Product(vec![scale, Expr::func(Fn::Ln, Expr::func(Fn::Abs, b))])
        } else {
            let k = 1 - power as i64;
            Expr::Product(vec![scale, Expr::Num(RBig::ONE / RBig::from(k)), Expr::pow(b, Expr::int(k))])
        });
    }
    // (Bx + C) / (ax^2 + bx + c) = \frac{B}{2a} \frac{2ax + b}{…} + \frac{C - Bb/2a}{…}
    let (qa, qb, qc) = (c(base, 2), c(base, 1), c(base, 0));
    let (big_b, big_c) = (c(numerator, 1), c(numerator, 0));
    let two_a = RBig::from(2) * &qa;
    let log_part = Expr::Product(vec![Expr::Num(big_b.clone() / &two_a), Expr::func(Fn::Ln, b)]);
    let k = big_c - big_b * &qb / &two_a;
    // a((x + h)^2 + w^2) with h = b/2a, w^2 = (4ac - b^2)/4a^2
    let h = qb.clone() / &two_a;
    let w_squared = (RBig::from(4) * &qa * &qc - &qb * &qb) / (&two_a * &two_a);
    if !w_squared.is_positive() {
        return None;
    }
    let w = Expr::pow(Expr::Num(w_squared), Expr::ratio(1, 2));
    let shifted = Expr::Product(vec![Expr::Sum(vec![Expr::Var, Expr::Num(h)]), Expr::recip(w.clone())]);
    let arctan = Expr::Product(vec![Expr::Num(k / &qa), Expr::recip(w), Expr::func(Fn::Arctan, shifted)]);
    Some(Expr::Sum(vec![log_part, arctan]))
}

// ---- substitution ------------------------------------------------------------------------------

/// A stand-in for u while checking that the rest of the integrand is a function of u alone. It
/// is a constant no input produces, so nothing else matches it.
fn marker() -> Expr {
    Expr::func(Fn::Arccos, Expr::Num(RBig::from_parts(IBig::from(7919), UBig::from(104_729u32))))
}

/// `\int h(g(x)) g'(x) dx = H(g(x))`: the result, u = g(x), h and H as functions of u (in the
/// variable slot).
fn substitution(f: &Expr, depth: usize) -> Option<(Expr, Expr, Expr, Expr)> {
    if depth > MAX_DEPTH {
        return None;
    }
    let mut candidates = Vec::new();
    inner_functions(f, &mut candidates);
    for u in candidates {
        let q = simplify(&Expr::Product(vec![f.clone(), Expr::recip(derivative(&u))]));
        let in_u = replace(&q, &u, &marker());
        if in_u.depends_on_var() {
            continue;
        }
        let h = replace(&in_u, &marker(), &Expr::Var);
        let Some(big_h) = basic(&simplify(&h), depth + 1) else { continue };
        return Some((simplify(&big_h.substitute(&u)), u, h, big_h));
    }
    None
}

/// The non-linear functions and arguments in `e`, the candidates for u.
fn inner_functions(e: &Expr, out: &mut Vec<Expr>) {
    let mut push = |u: &Expr| {
        if u.depends_on_var() && linear(u).is_none() && !out.contains(u) {
            out.push(u.clone());
        }
    };
    match e {
        Expr::Func(_, argument) => {
            push(argument);
            push(e);
            inner_functions(argument, out);
        }
        Expr::Pow(base, exponent) => {
            push(base);
            if !exponent.is_constant() {
                push(exponent);
            }
            inner_functions(base, out);
            inner_functions(exponent, out);
        }
        Expr::Sum(items) | Expr::Product(items) => {
            for item in items {
                inner_functions(item, out);
            }
        }
        _ => {}
    }
}

fn replace(e: &Expr, from: &Expr, to: &Expr) -> Expr {
    if e == from {
        return to.clone();
    }
    match e {
        Expr::Sum(items) => Expr::Sum(items.iter().map(|i| replace(i, from, to)).collect()),
        Expr::Product(items) => Expr::Product(items.iter().map(|i| replace(i, from, to)).collect()),
        Expr::Pow(b, x) => Expr::pow(replace(b, from, to), replace(x, from, to)),
        Expr::Func(function, argument) => Expr::func(*function, replace(argument, from, to)),
        other => other.clone(),
    }
}

// ---- integration by parts ----------------------------------------------------------------------

enum Parts {
    Plain { u: Expr, v_prime: Expr, v: Expr },
    /// `e^{ax}\sin(bx)`: the integral comes back and is solved for.
    Recurring,
}

fn by_parts(f: &Expr, depth: usize) -> Option<(Expr, Parts)> {
    if depth > MAX_DEPTH {
        return None;
    }
    let factors: Vec<Expr> = match f {
        Expr::Product(factors) => factors.clone(),
        other => vec![other.clone()],
    };
    let (constant, rest): (Vec<Expr>, Vec<Expr>) = factors.into_iter().partition(Expr::is_constant);
    let c = Expr::product(constant);
    let (polynomial, other): (Vec<Expr>, Vec<Expr>) =
        rest.into_iter().partition(|e| e.as_rational().is_some_and(|(_, d)| d.is_constant()));
    let p = Expr::product(polynomial);
    let scaled = |result: Expr| simplify(&Expr::Product(vec![c.clone(), result]));
    match other.as_slice() {
        // x^n e^{ax}, x^n sin(ax): differentiate the polynomial.
        [t @ (Expr::Func(Fn::Exp | Fn::Sin | Fn::Cos | Fn::Sinh | Fn::Cosh, _) | Expr::Pow(..))] if p != Expr::int(1) && leaf(t).is_some() => {
            let v = leaf(t)?;
            let rest = basic(&simplify(&Expr::Product(vec![derivative(&p), v.clone()])), depth + 1)?;
            let result = Expr::Sum(vec![Expr::Product(vec![p.clone(), v.clone()]), Expr::neg(rest)]);
            Some((scaled(result), Parts::Plain { u: p, v_prime: t.clone(), v }))
        }
        // x^n \ln x, \arctan x: differentiate the function, integrate the polynomial.
        [t @ Expr::Func(Fn::Ln | Fn::Arctan | Fn::Arcsin | Fn::Arccos, argument)] if linear(argument).is_some() => {
            let v = table(&p)?;
            let rest = basic(&simplify(&Expr::Product(vec![derivative(t), v.clone()])), depth + 1)?;
            let result = Expr::Sum(vec![Expr::Product(vec![t.clone(), v.clone()]), Expr::neg(rest)]);
            Some((scaled(result), Parts::Plain { u: t.clone(), v_prime: p, v }))
        }
        // e^{ax+β} sin(bx+γ): ∫ = e^{…}(a sin − b cos)/(a² + b²), cos: (a cos + b sin)/(a² + b²).
        [Expr::Func(Fn::Exp, e_arg), Expr::Func(trig @ (Fn::Sin | Fn::Cos), t_arg)] | [Expr::Func(trig @ (Fn::Sin | Fn::Cos), t_arg), Expr::Func(Fn::Exp, e_arg)]
            if p == Expr::int(1) =>
        {
            let (a, _) = linear(e_arg)?;
            let (b, _) = linear(t_arg)?;
            let exp = Expr::func(Fn::Exp, (**e_arg).clone());
            let (sin, cos) = (Expr::func(Fn::Sin, (**t_arg).clone()), Expr::func(Fn::Cos, (**t_arg).clone()));
            let bracket = match trig {
                Fn::Sin => Expr::Sum(vec![Expr::Product(vec![a.clone(), sin]), Expr::neg(Expr::Product(vec![b.clone(), cos]))]),
                _ => Expr::Sum(vec![Expr::Product(vec![a.clone(), cos]), Expr::Product(vec![b.clone(), sin])]),
            };
            let norm = Expr::Sum(vec![Expr::pow(a, Expr::int(2)), Expr::pow(b, Expr::int(2))]);
            Some((scaled(Expr::Product(vec![exp, bracket, Expr::recip(norm)])), Parts::Recurring))
        }
        _ => None,
    }
}

/// Any method without working, for the integrals inside substitution and parts.
fn basic(e: &Expr, depth: usize) -> Option<Expr> {
    if depth > MAX_DEPTH {
        return None;
    }
    if let Some(result) = table(e) {
        return Some(result);
    }
    if let Some((n, d)) = e.as_rational() {
        if let Some(result) = rational(&n, &d, None) {
            return Some(result);
        }
    }
    if let Some((result, _)) = by_parts(e, depth + 1) {
        return Some(result);
    }
    substitution(e, depth + 1).map(|(result, ..)| result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::domain::{domain, DomainContext};
    use crate::latex::parser;
    use crate::settings::Mode;

    fn expr(latex: &str) -> Expr {
        let options = Settings { mode: Mode::Algebra, ..Settings::default() }.parse_options();
        Expr::from_node(&parser::parse_with(latex, &options).unwrap().expression, "x").unwrap()
    }

    fn integrate(latex: &str) -> Option<Integration> {
        let f = expr(latex);
        let settings = Settings::default();
        let (d, _) = domain(&f, &DomainContext { settings: &settings, var: "x", digits: 6 }).unwrap();
        antiderivative(&f, &IntegrateContext { settings: &settings, var: "x", domain: &d })
    }

    fn primitive(latex: &str) -> String {
        integrate(latex).map(|i| i.result.to_latex("x")).unwrap_or_else(|| "none".into())
    }

    fn method(latex: &str) -> String {
        integrate(latex).unwrap().method.as_str().to_string()
    }

    #[test]
    fn polynomials_and_basic_integrals() {
        assert_eq!(primitive("x^3-3x"), "\\frac{1}{4}x^{4} - \\frac{3}{2}x^{2}");
        assert_eq!(method("x^3-3x"), "INTEGRAL_POLYNOMIAL");
        assert_eq!(primitive("\\cos(2x+1)"), "\\frac{1}{2}\\sin(2x + 1)");
        assert_eq!(primitive("e^{3x}"), "\\frac{1}{3}e^{3x}");
        assert_eq!(primitive("\\frac{1}{x}"), "\\ln\\left|x\\right|");
        assert_eq!(primitive("2^x"), "\\frac{2^{x}}{\\ln(2)}");
        assert_eq!(primitive("\\sin x"), "-\\cos(x)");
        assert_eq!(primitive("\\sqrt{x}"), "\\frac{2}{3}\\sqrt{x^{3}}");
        assert_eq!(primitive("\\frac{1}{\\sqrt{1-x^2}}"), "\\arcsin(x)");
        assert_eq!(primitive("\\ln x"), "x \\cdot \\ln(x) - x");
    }

    #[test]
    fn rational_functions_by_partial_fractions() {
        assert_eq!(primitive("\\frac{1}{x^2+1}"), "\\arctan(x)");
        assert_eq!(primitive("\\frac{x^2-1}{x-2}"), "\\frac{1}{2}x^{2} + 2x + 3\\ln\\left|x - 2\\right|");
        assert_eq!(method("\\frac{x^2-1}{x-2}"), "PARTIAL_FRACTIONS");
        assert_eq!(primitive("\\frac{1}{x^2-1}"), "\\frac{1}{2}\\ln\\left|x - 1\\right| - \\frac{1}{2}\\ln\\left|x + 1\\right|");
        assert_eq!(primitive("\\frac{x}{x^2+1}"), "\\frac{1}{2}\\ln(x^{2} + 1)");
        assert_eq!(primitive("\\frac{1}{(x-1)^2}"), "-\\frac{1}{x - 1}");
        let steps = integrate("\\frac{1}{x^2-1}").unwrap().steps;
        assert_eq!(steps[0].latex, "\\frac{1}{x^{2} - 1} = \\frac{1}{2(x - 1)} - \\frac{1}{2(x + 1)}");
    }

    #[test]
    fn substitution_and_parts() {
        assert_eq!(primitive("2x e^{x^2}"), "e^{x^{2}}");
        assert_eq!(primitive("\\frac{\\ln x}{x}"), "\\frac{1}{2}\\ln^{2}(x)");
        assert_eq!(method("\\frac{\\ln x}{x}"), "INTEGRAL_SUBSTITUTION");
        assert_eq!(primitive("x e^x"), "(x - 1)e^{x}");
        assert_eq!(method("x e^x"), "INTEGRATION_BY_PARTS");
        assert_eq!(primitive("x^2 e^x"), "(x^{2} - 2x + 2)e^{x}");
        assert_eq!(primitive("x \\sin x"), "\\sin(x) - x \\cdot \\cos(x)");
        assert_eq!(primitive("x \\ln x"), "\\frac{1}{2}x^{2} \\cdot \\ln(x) - \\frac{1}{4}x^{2}");
        assert_eq!(primitive("\\arctan x"), "x \\cdot \\arctan(x) - \\frac{1}{2}\\ln(x^{2} + 1)");
        assert_eq!(primitive("e^x \\sin x"), "\\frac{1}{2}(\\sin(x) - \\cos(x))e^{x}");
    }

    #[test]
    fn without_an_elementary_antiderivative_there_is_none() {
        assert_eq!(primitive("e^{x^2}"), "none");
        assert_eq!(primitive("\\frac{\\sin x}{x}"), "none");
        assert_eq!(primitive("\\frac{1}{x^3+x+1}"), "none");
    }

    #[test]
    fn every_result_differentiates_back() {
        for input in ["x^3-3x", "x e^x", "\\frac{x^2-1}{x-2}", "\\frac{\\ln x}{x}", "\\frac{1}{x^2+1}", "\\sin x", "\\frac{3x+1}{x^2+2x+5}", "\\tan x"] {
            let f = expr(input);
            let big_f = integrate(input).unwrap().result;
            let back = simplify(&Expr::Sum(vec![derivative(&big_f), Expr::neg(f.clone())]));
            assert!(is_zero(&back) || is_zero_numeric(&back, &[0.3, 0.7, 1.3, 2.1]), "{input}");
        }
    }
}
