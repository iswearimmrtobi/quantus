//! Polynomial equations of degree three and more with rational coefficients.
//!
//! The residual is fitted by a polynomial of degree at most [`MAX_DEGREE`] from its values at
//! whole numbers and checked at other points. Its rational roots are found by the rational root
//! test and divided out; what is left is solved exactly up to degree four (Cardano for a cubic,
//! Ferrari for a quartic) and beyond that numerically, without a fixed search interval: the
//! real roots of a polynomial lie between its critical points, which are the roots of its
//! derivative, and within the Cauchy bound.

use dashu::base::{Abs, Gcd, UnsignedAbs};
use dashu::integer::{IBig, UBig};
use dashu::rational::RBig;

use crate::num::{MathFunction, Value};
use crate::Result;

/// The highest degree that is fitted.
pub(crate) const MAX_DEGREE: usize = 12;
/// Coefficients up to this size have their divisors listed for the rational root test.
const DIVISOR_LIMIT: u64 = 1_000_000_000_000;

/// The coefficients `a_0, a_1, …, a_n` of the polynomial the samples come from, if they come
/// from one of degree at most [`MAX_DEGREE`] with rational coefficients. `at` gives the
/// residual at a point, `None` where it is undefined or not rational.
pub(crate) fn fit(at: &mut dyn FnMut(&RBig) -> Result<Option<RBig>>) -> Result<Option<Vec<RBig>>> {
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for k in 0..=MAX_DEGREE as i64 {
        let x = RBig::from(k);
        let Some(y) = at(&x)? else { return Ok(None) };
        xs.push(x);
        ys.push(y);
    }
    let coefficients = trim(interpolate(&xs, &ys));
    let checks = [
        RBig::from(-1),
        RBig::from(-7),
        RBig::from_parts(IBig::from(7), UBig::from(3u8)),
        RBig::from_parts(IBig::from(-11), UBig::from(2u8)),
        RBig::from(1000),
        RBig::from(-1000),
        RBig::from_parts(IBig::from(17), UBig::from(5u8)),
    ];
    for x in checks {
        match at(&x)? {
            Some(y) if y == evaluate(&coefficients, &x) => {}
            _ => return Ok(None),
        }
    }
    Ok(Some(coefficients))
}

/// Newton's divided differences, expanded into the monomial basis.
fn interpolate(xs: &[RBig], ys: &[RBig]) -> Vec<RBig> {
    let n = xs.len();
    let mut table = ys.to_vec();
    for level in 1..n {
        for i in (level..n).rev() {
            table[i] = (&table[i] - &table[i - 1]) / (&xs[i] - &xs[i - level]);
        }
    }
    // Horner from the highest divided difference: p = t_{n-1}, p = p·(x - x_i) + t_i.
    let mut poly = vec![table[n - 1].clone()];
    for i in (0..n - 1).rev() {
        let mut next = vec![RBig::ZERO; poly.len() + 1];
        for (k, c) in poly.iter().enumerate() {
            next[k + 1] += c;
            next[k] -= c * &xs[i];
        }
        next[0] += &table[i];
        poly = next;
    }
    poly
}

fn trim(mut coefficients: Vec<RBig>) -> Vec<RBig> {
    while coefficients.len() > 1 && coefficients.last().is_some_and(|c| *c == RBig::ZERO) {
        coefficients.pop();
    }
    coefficients
}

pub(crate) fn evaluate(coefficients: &[RBig], x: &RBig) -> RBig {
    coefficients.iter().rev().fold(RBig::ZERO, |sum, c| sum * x + c)
}

/// All roots: exact where the degree allows, real ones only unless `complex`, each once and
/// sorted (real ones by value). `digits` is the precision of numeric roots.
pub(crate) fn roots(coefficients: &[RBig], complex: bool, digits: usize) -> Result<Vec<Value>> {
    let mut rest = trim(coefficients.to_vec());
    let mut found: Vec<Value> = Vec::new();
    while rest.len() > 2 {
        let Some(root) = rational_root(&rest) else { break };
        found.push(Value::Rat(root.clone()));
        rest = deflate(&rest, &root);
    }
    let degree = rest.len() - 1;
    let values = |c: &[RBig]| c.iter().cloned().map(Value::Rat).collect::<Vec<_>>();
    match degree {
        0 => {}
        1 | 2 => found.extend(quadratic(&values(&pad(&rest, 3)), complex)?),
        3 => found.extend(cubic(&values(&rest), complex)?),
        4 => found.extend(quartic(&rest, complex, digits)?),
        _ => {
            found.extend(real_roots(&rest, digits)?.into_iter().map(Value::Rat));
            if complex {
                found.extend(complex_pairs(&rest, digits));
            }
        }
    }
    sort_and_dedupe(found)
}

fn pad(coefficients: &[RBig], length: usize) -> Vec<RBig> {
    let mut padded = coefficients.to_vec();
    padded.resize(length, RBig::ZERO);
    padded
}

fn sort_and_dedupe(values: Vec<Value>) -> Result<Vec<Value>> {
    let mut keyed: Vec<(Option<RBig>, RBig, Value)> = Vec::new();
    for value in values {
        let key = value.decimal(40).ok().map(|d| d.to_rational());
        let imaginary = value.imaginary_part().and_then(|im| im.decimal(40).ok()).map(|d| d.to_rational()).unwrap_or(RBig::ZERO);
        let real = key.clone().or_else(|| value.real_part().and_then(|re| re.decimal(40).ok()).map(|d| d.to_rational()));
        if keyed.iter().any(|(r, i, _)| r == &real && i == &imaginary) {
            continue;
        }
        keyed.push((real, imaginary, value));
    }
    keyed.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    Ok(keyed.into_iter().map(|(_, _, value)| value).collect())
}

/// Divides `(x - root)` out of the polynomial.
fn deflate(coefficients: &[RBig], root: &RBig) -> Vec<RBig> {
    let n = coefficients.len() - 1;
    let mut quotient = vec![RBig::ZERO; n];
    let mut carry = RBig::ZERO;
    for k in (1..=n).rev() {
        carry = &carry * root + &coefficients[k];
        quotient[k - 1] = carry.clone();
    }
    quotient
}

/// A rational root by the rational root test: `±p/q` with `p` dividing the constant and `q` the
/// leading coefficient of the polynomial scaled to whole numbers.
fn rational_root(coefficients: &[RBig]) -> Option<RBig> {
    if coefficients[0] == RBig::ZERO {
        return Some(RBig::ZERO);
    }
    let lcm = coefficients.iter().fold(UBig::ONE, |l, c| {
        let d = c.denominator();
        &l / l.clone().gcd(d) * d
    });
    let whole: Vec<IBig> = coefficients.iter().map(|c| (c * RBig::from(IBig::from(lcm.clone()))).numerator().clone()).collect();
    let constant = u64::try_from(whole[0].clone().unsigned_abs()).ok().filter(|c| *c <= DIVISOR_LIMIT)?;
    let leading = u64::try_from(whole.last()?.clone().unsigned_abs()).ok().filter(|c| *c <= DIVISOR_LIMIT)?;
    for p in divisors(constant) {
        for q in divisors(leading) {
            for sign in [1i64, -1] {
                let candidate = RBig::from_parts(IBig::from(sign) * IBig::from(p), UBig::from(q));
                if evaluate(coefficients, &candidate) == RBig::ZERO {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

fn divisors(n: u64) -> Vec<u64> {
    let mut small = Vec::new();
    let mut large = Vec::new();
    let mut d = 1u64;
    while d * d <= n {
        if n.is_multiple_of(d) {
            small.push(d);
            if d != n / d {
                large.push(n / d);
            }
        }
        d += 1;
    }
    small.extend(large.into_iter().rev());
    small
}

/// `a_0 + a_1 x + a_2 x² = 0` for any values: the linear and quadratic case.
pub(crate) fn quadratic(c: &[Value], complex: bool) -> Result<Vec<Value>> {
    let (c0, b, a) = (&c[0], &c[1], &c[2]);
    if a.is_zero() {
        if b.is_zero() {
            return Ok(Vec::new());
        }
        return Ok(vec![c0.neg().div(b)?]);
    }
    let discriminant = b.mul(b).sub(&a.mul(c0).mul(&Value::int(4)));
    let two_a = a.mul(&Value::int(2));
    if discriminant.is_zero() {
        return Ok(vec![b.neg().div(&two_a)?]);
    }
    if discriminant.is_negative()? {
        if !complex {
            return Ok(Vec::new());
        }
        let re = b.neg().div(&two_a)?;
        let im = discriminant.neg().pow(&Value::ratio(1, 2))?.div(&two_a)?;
        let im = if im.is_negative()? { im.neg() } else { im };
        return Ok(vec![re.sub(&im.mul(&Value::I)), re.add(&im.mul(&Value::I))]);
    }
    let root = discriminant.pow(&Value::ratio(1, 2))?;
    Ok(vec![b.neg().sub(&root).div(&two_a)?, b.neg().add(&root).div(&two_a)?])
}

/// `a_0 + a_1 x + a_2 x² + a_3 x³ = 0` by Cardano: with one real root as the sum of two cube
/// roots, with three as cosines.
fn cubic(c: &[Value], complex: bool) -> Result<Vec<Value>> {
    let a = &c[3];
    let (b, cc, d) = (c[2].div(a)?, c[1].div(a)?, c[0].div(a)?);
    // x = t - b/3: t³ + p t + q = 0
    let shift = b.div(&Value::int(3))?;
    let p = cc.sub(&b.mul(&b).div(&Value::int(3))?);
    let q = Value::int(2).mul(&b.mul(&b).mul(&b)).div(&Value::int(27))?.sub(&b.mul(&cc).div(&Value::int(3))?).add(&d);
    let half_q = q.div(&Value::int(2))?;
    let third_p = p.div(&Value::int(3))?;
    // q²/4 + p³/27: negative for three real roots.
    let inner = half_q.mul(&half_q).add(&third_p.mul(&third_p).mul(&third_p));
    let third = Value::ratio(1, 3);
    if inner.is_negative()? {
        // t_k = 2√(-p/3) cos(⅓ arccos(3q/(2p) √(-3/p)) - 2πk/3)
        let m = Value::int(2).mul(&third_p.neg().pow(&Value::ratio(1, 2))?);
        let argument = Value::int(3).mul(&q).div(&Value::int(2).mul(&p))?.mul(&Value::int(-3).div(&p)?.pow(&Value::ratio(1, 2))?);
        let angle = argument.apply(MathFunction::Arccos)?.mul(&third);
        let mut roots = Vec::new();
        for k in 0..3 {
            let turn = Value::ratio(2 * k, 3).mul(&Value::Pi);
            roots.push(m.mul(&angle.sub(&turn).apply(MathFunction::Cos)?).sub(&shift));
        }
        return Ok(roots);
    }
    let root = inner.pow(&Value::ratio(1, 2))?;
    let u = half_q.neg().add(&root).pow(&third)?;
    let v = half_q.neg().sub(&root).pow(&third)?;
    let real = u.add(&v).sub(&shift);
    if inner.is_zero() || !complex {
        // A double root when the inner term vanishes: t = -u.
        let mut roots = vec![real];
        if inner.is_zero() && !u.is_zero() {
            roots.push(u.neg().sub(&shift));
        }
        return Ok(roots);
    }
    // The other two: -(u+v)/2 - b/3 ± i √3/2 (u - v).
    let re = u.add(&v).div(&Value::int(-2))?.sub(&shift);
    let im = Value::int(3).pow(&Value::ratio(1, 2))?.div(&Value::int(2))?.mul(&u.sub(&v));
    let im = if im.is_negative()? { im.neg() } else { im };
    Ok(vec![real, re.sub(&im.mul(&Value::I)), re.add(&im.mul(&Value::I))])
}

/// A quartic by Ferrari: depressed to `y⁴ + p y² + q y + r`, split into two quadratics with a
/// positive root of the resolvent cubic.
fn quartic(coefficients: &[RBig], complex: bool, digits: usize) -> Result<Vec<Value>> {
    let a = &coefficients[4];
    let (b, c, d, e) = (&coefficients[3] / a, &coefficients[2] / a, &coefficients[1] / a, &coefficients[0] / a);
    let shift = &b / RBig::from(4);
    let b2 = &b * &b;
    let p = &c - RBig::from_parts(IBig::from(3), UBig::from(8u8)) * &b2;
    let q = &d - &b * &c / RBig::from(2) + &b2 * &b / RBig::from(8);
    let r = &e - &b * &d / RBig::from(4) + &b2 * &c / RBig::from(16) - RBig::from_parts(IBig::from(3), UBig::from(256u16)) * &b2 * &b2;
    let shift_value = Value::Rat(shift);
    let back = |y: Value| y.sub(&shift_value);
    if q == RBig::ZERO {
        // y⁴ + p y² + r: y² = z for each root z of z² + p z + r.
        let mut roots = Vec::new();
        for z in quadratic(&[Value::Rat(r), Value::Rat(p), Value::one()], complex)? {
            if z.is_zero() {
                roots.push(back(Value::zero()));
                continue;
            }
            let real = z.re_im().is_none_or(|(_, im)| im.is_zero());
            if real && !z.is_negative()? {
                let y = z.pow(&Value::ratio(1, 2))?;
                roots.extend([back(y.neg()), back(y)]);
            } else if complex {
                // A negative z, or a complex one when z² + p z + r has no real root (x⁴ + 1).
                let y = complex_sqrt(&z)?;
                roots.extend([back(y.neg()), back(y)]);
            }
        }
        return Ok(roots);
    }
    // 8m³ + 8p m² + (2p² - 8r) m - q² = 0, a root m > 0.
    let resolvent = [-(&q * &q), RBig::from(2) * &p * &p - RBig::from(8) * &r, RBig::from(8) * &p, RBig::from(8)];
    let mut candidates = roots(&resolvent, false, digits)?;
    candidates.retain(|m| m.decimal(40).is_ok_and(|d| d.to_rational() > RBig::ZERO));
    // A rational one keeps the result simplest.
    candidates.sort_by_key(|m| m.as_rational().is_none());
    let Some(m) = candidates.into_iter().next() else { return Ok(Vec::new()) };
    let s = Value::int(2).mul(&m).pow(&Value::ratio(1, 2))?;
    let base = Value::Rat(&p / RBig::from(2)).add(&m);
    let offset = Value::Rat(q).div(&Value::int(2).mul(&s))?;
    let mut roots = Vec::new();
    roots.extend(quadratic(&[base.add(&offset), s.neg(), Value::one()], complex)?.into_iter().map(&back));
    roots.extend(quadratic(&[base.sub(&offset), s.clone(), Value::one()], complex)?.into_iter().map(&back));
    Ok(roots)
}

/// The non-real roots of a polynomial of degree five and more, to `digits` digits, in conjugate
/// pairs: located in `f64` by the Durand–Kerner iteration on the square-free part (whose roots
/// are simple), then polished by Newton's method in rationals rounded to the working precision.
fn complex_pairs(coefficients: &[RBig], digits: usize) -> Vec<Value> {
    let simple = square_free(coefficients);
    let n = simple.len() - 1;
    let lead = simple[n].to_f64().value();
    let c: Vec<f64> = simple.iter().map(|a| a.to_f64().value() / lead).collect();
    let bound = 1.0 + c[..n].iter().fold(0.0_f64, |m, a| m.max(a.abs()));
    let (mul, div) = (
        |a: (f64, f64), b: (f64, f64)| (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0),
        |a: (f64, f64), b: (f64, f64)| {
            let d = b.0 * b.0 + b.1 * b.1;
            ((a.0 * b.0 + a.1 * b.1) / d, (a.1 * b.0 - a.0 * b.1) / d)
        },
    );
    let at = |z: (f64, f64)| c.iter().rev().fold((0.0, 0.0), |s, a| {
        let p = mul(s, z);
        (p.0 + a, p.1)
    });
    // Start on a circle, turned so that no start is real or symmetric to another.
    let mut z: Vec<(f64, f64)> = (0..n)
        .map(|k| {
            let angle = 0.4 + std::f64::consts::TAU * k as f64 / n as f64;
            (bound * angle.cos(), bound * angle.sin())
        })
        .collect();
    for _ in 0..1000 {
        let mut moved = 0.0_f64;
        for i in 0..n {
            let mut denominator = (1.0, 0.0);
            for j in (0..n).filter(|j| *j != i) {
                denominator = mul(denominator, (z[i].0 - z[j].0, z[i].1 - z[j].1));
            }
            let step = div(at(z[i]), denominator);
            z[i] = (z[i].0 - step.0, z[i].1 - step.1);
            moved = moved.max(step.0.hypot(step.1));
        }
        if moved < 1e-14 * bound {
            break;
        }
    }
    let mut out = Vec::new();
    for start in z.into_iter().filter(|z| z.1 > 1e-9 * bound.max(1.0)) {
        if let Some((re, im)) = polish(&simple, start, digits) {
            if im > RBig::ZERO {
                let (re, im) = (Value::Rat(re), Value::Rat(im));
                out.push(re.sub(&im.mul(&Value::I)));
                out.push(re.add(&im.mul(&Value::I)));
            }
        }
    }
    out
}

/// Newton's method on a complex start in rationals, rounded to `digits + 10` digits each step.
fn polish(coefficients: &[RBig], start: (f64, f64), digits: usize) -> Option<(RBig, RBig)> {
    let scale = UBig::from(10u8).pow(digits + 10);
    let round = |r: RBig| RBig::from_parts((r.numerator() * IBig::from(scale.clone())) / IBig::from(r.denominator().clone()), scale.clone());
    let tolerance = RBig::from_parts(IBig::ONE, UBig::from(10u8).pow(digits + 5));
    let (mut re, mut im) = (RBig::try_from(start.0).ok()?, RBig::try_from(start.1).ok()?);
    let derivative: Vec<RBig> = (1..coefficients.len()).map(|k| &coefficients[k] * RBig::from(k as i64)).collect();
    let horner = |c: &[RBig], re: &RBig, im: &RBig| {
        c.iter().rev().fold((RBig::ZERO, RBig::ZERO), |(a, b), k| (&a * re - &b * im + k, &a * im + &b * re))
    };
    for _ in 0..100 {
        let (pr, pi) = horner(coefficients, &re, &im);
        let (dr, di) = horner(&derivative, &re, &im);
        let norm = &dr * &dr + &di * &di;
        if norm == RBig::ZERO {
            return None;
        }
        let (sr, si) = ((&pr * &dr + &pi * &di) / &norm, (&pi * &dr - &pr * &di) / &norm);
        re = round(&re - &sr);
        im = round(&im - &si);
        if sr.abs() < tolerance && si.abs() < tolerance {
            return Some((re, im));
        }
    }
    None
}

/// `p / gcd(p, p')`: the same roots, each simple.
fn square_free(p: &[RBig]) -> Vec<RBig> {
    let derivative: Vec<RBig> = (1..p.len()).map(|k| &p[k] * RBig::from(k as i64)).collect();
    let g = gcd(p.to_vec(), derivative);
    if g.len() <= 1 {
        return p.to_vec();
    }
    divide(p, &g).0
}

fn gcd(mut a: Vec<RBig>, mut b: Vec<RBig>) -> Vec<RBig> {
    while b.iter().any(|c| *c != RBig::ZERO) {
        let r = divide(&a, &b).1;
        a = b;
        b = r;
    }
    a
}

/// Quotient and remainder of polynomial division.
fn divide(a: &[RBig], b: &[RBig]) -> (Vec<RBig>, Vec<RBig>) {
    let b = trim(b.to_vec());
    let mut r = trim(a.to_vec());
    if r.len() < b.len() {
        return (vec![RBig::ZERO], r);
    }
    let mut q = vec![RBig::ZERO; r.len() - b.len() + 1];
    while r.len() >= b.len() && !(r.len() == 1 && r[0] == RBig::ZERO) {
        let shift = r.len() - b.len();
        let factor = &r[r.len() - 1] / &b[b.len() - 1];
        for (k, c) in b.iter().enumerate() {
            r[k + shift] -= &factor * c;
        }
        q[shift] = factor;
        r.pop();
        r = trim(r);
        if r.len() < b.len() {
            break;
        }
    }
    (q, r)
}

/// The principal square root of `a + bi`, by halves: `√((|z| + a)/2) + i·sgn(b)·√((|z| - a)/2)`,
/// which keeps `√(2i) = 1 + i` exact where the polar form would give cosines of `π/4`.
pub(crate) fn complex_sqrt(z: &Value) -> Result<Value> {
    let half = Value::ratio(1, 2);
    let Some((a, b)) = z.re_im() else { return z.pow(&half) };
    if b.is_zero() {
        return if a.is_negative()? { Ok(a.neg().pow(&half)?.mul(&Value::I)) } else { a.pow(&half) };
    }
    let modulus = a.mul(&a).add(&b.mul(&b)).pow(&half)?;
    let re = modulus.add(&a).div(&Value::int(2))?.pow(&half)?;
    let im = modulus.sub(&a).div(&Value::int(2))?.pow(&half)?;
    let im = if b.is_negative()? { im.neg() } else { im };
    Ok(re.add(&im.mul(&Value::I)))
}

/// The real roots of a polynomial with rational coefficients, to `digits` digits: between
/// consecutive critical points the polynomial is monotonic, so each such interval with a sign
/// change holds one root, found by bisection; a critical point where it vanishes is a multiple
/// root.
pub(crate) fn real_roots(coefficients: &[RBig], digits: usize) -> Result<Vec<RBig>> {
    let coefficients = trim(coefficients.to_vec());
    let degree = coefficients.len() - 1;
    if degree == 0 {
        return Ok(Vec::new());
    }
    if degree == 1 {
        return Ok(vec![-(&coefficients[0] / &coefficients[1])]);
    }
    let derivative: Vec<RBig> = (1..=degree).map(|k| &coefficients[k] * RBig::from(k as i64)).collect();
    let critical = real_roots(&derivative, digits + 5)?;
    // Cauchy: every root lies within 1 + max |a_k / a_n|.
    let leading = coefficients[degree].clone().abs();
    let bound = coefficients[..degree].iter().map(|c| c.clone().abs() / &leading).max().unwrap_or(RBig::ZERO) + RBig::ONE;
    let mut points = vec![-bound.clone()];
    points.extend(critical.iter().cloned());
    points.push(bound);
    let tolerance = RBig::from_parts(IBig::ONE, UBig::from(10u8).pow(digits + 5));
    let sign = |x: &RBig| evaluate(&coefficients, x).signum();
    let mut roots = Vec::new();
    for pair in points.windows(2) {
        let (mut lo, mut hi) = (pair[0].clone(), pair[1].clone());
        let (slo, shi) = (sign(&lo), sign(&hi));
        if slo == RBig::ZERO {
            roots.push(lo.clone());
            continue;
        }
        if slo == shi || shi == RBig::ZERO {
            continue;
        }
        while &hi - &lo > tolerance {
            let mid = (&lo + &hi) / RBig::from(2);
            if sign(&mid) == slo {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        roots.push((&lo + &hi) / RBig::from(2));
    }
    // A multiple root: the polynomial touches zero at a critical point.
    let accept = RBig::from_parts(IBig::ONE, UBig::from(10u8).pow(digits));
    for point in critical {
        let small = evaluate(&coefficients, &point).abs() < accept;
        if small && !roots.iter().any(|r| (r - &point).abs() < accept) {
            roots.push(point);
        }
    }
    roots.sort();
    Ok(roots)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn poly(c: &[i64]) -> Vec<RBig> {
        c.iter().map(|v| RBig::from(*v)).collect()
    }

    fn decimals(values: &[Value]) -> Vec<String> {
        values.iter().map(|v| v.decimal(10).map(|d| d.to_plain_string()).unwrap_or_else(|_| format!("{v}"))).collect()
    }

    #[test]
    fn interpolation_recovers_the_coefficients() {
        let p = poly(&[5, -3, 0, 2]);
        let mut at = |x: &RBig| Ok(Some(evaluate(&p, x)));
        assert_eq!(fit(&mut at).unwrap().unwrap(), p);
    }

    #[test]
    fn rational_roots_are_divided_out() {
        // (x - 1)(x - 2)(x + 3) = x³ - 7x + 6
        assert_eq!(decimals(&roots(&poly(&[6, -7, 0, 1]), false, 20).unwrap()), ["-3", "1", "2"]);
        // (2x - 1)(x² - 2) = 2x³ - x² - 4x + 2
        let r = roots(&poly(&[2, -4, -1, 2]), false, 20).unwrap();
        assert_eq!(decimals(&r), ["-1.414213562", "0.5", "1.414213562"]);
    }

    #[test]
    fn cardano_with_one_and_three_real_roots() {
        // x³ - 2 = 0
        let r = roots(&poly(&[-2, 0, 0, 1]), false, 20).unwrap();
        assert_eq!(decimals(&r), ["1.25992105"]);
        // x³ - 3x + 1 = 0: three irrational real roots
        let r = roots(&poly(&[1, -3, 0, 1]), false, 20).unwrap();
        assert_eq!(decimals(&r), ["-1.879385242", "0.3472963553", "1.532088886"]);
        // x³ + x + 1 with the complex pair
        let r = roots(&poly(&[1, 1, 0, 1]), true, 20).unwrap();
        assert_eq!(r.len(), 3);
    }

    #[test]
    fn ferrari_splits_a_quartic() {
        // x⁴ - 10x² + 1 = 0: ±√2 ± √3
        let r = roots(&poly(&[1, 0, -10, 0, 1]), false, 20).unwrap();
        assert_eq!(decimals(&r), ["-3.14626437", "-0.3178372452", "0.3178372452", "3.14626437"]);
        // x⁴ + x + 1 has no real root; x⁴ - 2x - 1 = (x² ... ) has two
        assert!(roots(&poly(&[1, 1, 0, 0, 1]), false, 20).unwrap().is_empty());
        let r = roots(&poly(&[-1, -2, 0, 0, 1]), false, 20).unwrap();
        assert_eq!(decimals(&r), ["-0.4746266176", "1.395336994"]);
    }

    #[test]
    fn quartics_without_real_roots_have_four_complex_ones() {
        let parts = |values: &[Value]| -> Vec<(String, String)> {
            values
                .iter()
                .map(|v| {
                    let (re, im) = v.re_im().unwrap();
                    (re.decimal(10).unwrap().to_plain_string(), im.decimal(10).unwrap().to_plain_string())
                })
                .collect()
        };
        // x⁴ + 4 = (x² + 2x + 2)(x² - 2x + 2): ±1 ± i, exactly
        let r = roots(&poly(&[4, 0, 0, 0, 1]), true, 20).unwrap();
        assert_eq!(parts(&r), [("-1", "-1"), ("-1", "1"), ("1", "-1"), ("1", "1")].map(|(a, b)| (a.to_string(), b.to_string())));
        // x⁴ + 1: ±√½ ± i√½
        let r = roots(&poly(&[1, 0, 0, 0, 1]), true, 20).unwrap();
        assert_eq!(r.len(), 4);
        assert!(parts(&r).iter().all(|(re, im)| re.trim_start_matches('-') == "0.7071067812" && im.trim_start_matches('-') == "0.7071067812"));
        // x⁴ + 5x² + 13: y² = (-5 ± i√27)/2
        assert_eq!(roots(&poly(&[13, 0, 5, 0, 1]), true, 20).unwrap().len(), 4);
        assert!(roots(&poly(&[13, 0, 5, 0, 1]), false, 20).unwrap().is_empty());
    }

    #[test]
    fn higher_degrees_have_their_complex_roots_in_pairs() {
        // x⁵ + 1: -1 and e^{±iπ/5}, e^{±3iπ/5}
        let r = roots(&poly(&[1, 0, 0, 0, 0, 1]), true, 30).unwrap();
        assert_eq!(r.len(), 5);
        let cos = |v: &Value| v.re_im().unwrap().0.decimal(12).unwrap().to_plain_string();
        let mut re: Vec<String> = r.iter().map(cos).collect();
        re.sort();
        assert_eq!(re, ["-0.309016994375", "-0.309016994375", "-1", "0.809016994375", "0.809016994375"]);
        // (x² + 1)² (x² - 2) = x⁶ - 3x² - 2: the double pair ±i counts once, beside ±√2
        let r = roots(&poly(&[-2, 0, -3, 0, 0, 0, 1]), true, 30).unwrap();
        assert_eq!(r.len(), 4);
        assert_eq!(decimals(&[r[0].clone(), r[3].clone()]), ["-1.414213562", "1.414213562"]);
        assert_eq!(r[1].re_im().unwrap().1.decimal(20).unwrap().to_plain_string(), "-1");
        // Real roots only when not complex
        assert_eq!(roots(&poly(&[1, 0, 0, 0, 0, 1]), false, 30).unwrap().len(), 1);
    }

    #[test]
    fn higher_degrees_have_no_search_interval() {
        // x⁵ - 1000 x⁴ - 1 = 0 has a root near 1000, far beyond ±100.
        let r = roots(&poly(&[-1, 0, 0, 0, -1000, 1]), false, 20).unwrap();
        assert_eq!(decimals(&r), ["1000"]);
    }
}
