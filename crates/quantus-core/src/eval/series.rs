//! Sums to infinity, `\sum_{k=a}^{\infty}`. What is summed decides how:
//!
//! - **A rational function of k** (`\frac{1}{k^2}`, `\frac{1}{k(k+1)}`): it converges exactly when
//!   the denominator's degree exceeds the numerator's by two or more, which is checked first. The
//!   first terms are added exactly, the tail by the Euler–Maclaurin formula: the integral from N
//!   to ∞, half the term at N and Bernoulli corrections whose derivatives come from the Taylor
//!   expansion of `P/Q` at N, exact in rationals. With N far beyond the poles of `Q` the error
//!   is far below the 30 digits shown.
//! - **Alternating terms** (`\frac{(-1)^k}{k}`): the acceleration of Cohen, Rodriguez Villegas and
//!   Zagier, about 0.77 digits per term, once the terms are seen to fall towards 0.
//! - **Terms that fall at least geometrically** (`\frac{k}{2^k}`, `\frac{1}{k!}`): added one by
//!   one until they no longer matter.
//!
//! Anything else is refused rather than guessed: a series such as `\frac{1}{k \ln k}` diverges so
//! slowly that no finite number of terms shows it.

use dashu::integer::{IBig, UBig};
use dashu::rational::RBig;

use crate::{QuantusError, Result};

/// B₂, B₄, …, B₂₀.
const BERNOULLI: [(i64, u64); 10] = [
    (1, 6),
    (-1, 30),
    (1, 42),
    (-1, 30),
    (5, 66),
    (-691, 2730),
    (7, 6),
    (-3617, 510),
    (43867, 798),
    (-174611, 330),
];

/// How far beyond the poles of `Q` the tail of a rational series starts.
const TAIL_START: i64 = 200;

/// Most terms added one by one for a series that falls geometrically.
const MAX_TERMS: i64 = 200_000;

fn diverges(why: &str) -> QuantusError {
    QuantusError::other(format!("The series diverges: {why}."))
}

fn too_slow() -> QuantusError {
    QuantusError::other(
        "This series cannot be summed to \\infty: only terms that are a rational function of the index, alternate, or fall at least geometrically are.",
    )
}

/// Where the tail of `\sum_{k=from}^{\infty} P(k)/Q(k)` starts: past every real zero of `Q` and
/// far beyond its Cauchy bound. `None` when the series diverges, with the reason.
pub(crate) fn rational_tail_start(p: &[RBig], q: &[RBig], from: i64) -> Result<i64> {
    let (dp, dq) = (degree(p), degree(q));
    if dq < dp + 1 {
        return Err(diverges("its terms do not tend to 0"));
    }
    if dq < dp + 2 {
        return Err(diverges("its terms fall like 1/k, as in the harmonic series"));
    }
    let lead = abs(&q[dq]);
    let bound = q[..dq].iter().map(|c| abs(c) / &lead).max().unwrap_or(RBig::ZERO) + RBig::ONE;
    let bound = i64::try_from(bound.ceil()).map_err(|_| too_slow())?;
    Ok(from.max(bound) + TAIL_START)
}

/// `Σ_{k=from}^{n-1} P(k)/Q(k)` exactly.
pub(crate) fn rational_partial_sum(p: &[RBig], q: &[RBig], from: i64, n: i64) -> Result<RBig> {
    let mut sum = RBig::ZERO;
    for k in from..n {
        let x = RBig::from(k);
        let d = evaluate(q, &x);
        if d == RBig::ZERO {
            return Err(QuantusError::other(format!("The term is undefined at k = {k}.")));
        }
        sum += evaluate(p, &x) / d;
    }
    Ok(sum)
}

/// The Euler–Maclaurin tail `Σ_{k=n}^{∞} f(k) - ∫_n^∞ f` for `f = P/Q`: `f(n)/2 - Σ_j B_{2j}/(2j)·c_{2j-1}`
/// with `c_r` the Taylor coefficients of `f(n + h)`.
pub(crate) fn euler_maclaurin_correction(p: &[RBig], q: &[RBig], n: i64) -> RBig {
    let order = 2 * BERNOULLI.len();
    let (pc, qc) = (taylor_shift(p, n), taylor_shift(q, n));
    let mut c: Vec<RBig> = Vec::with_capacity(order);
    for r in 0..order {
        let mut top = pc.get(r).cloned().unwrap_or(RBig::ZERO);
        for i in 1..=r {
            if let Some(qi) = qc.get(i) {
                top -= qi * &c[r - i];
            }
        }
        c.push(top / &qc[0]);
    }
    let mut correction = &c[0] / RBig::from(2);
    for (j, (num, den)) in BERNOULLI.iter().enumerate() {
        let b = RBig::from_parts(IBig::from(*num), UBig::from(*den));
        correction -= b / RBig::from(2 * (j as i64 + 1)) * &c[2 * j + 1];
    }
    correction
}

/// `Σ_{k=from}^{∞} a(k)` for alternating or geometrically falling terms, to `digits` digits.
pub(crate) fn numeric_series(term: &mut dyn FnMut(i64) -> Result<RBig>, from: i64, digits: usize) -> Result<RBig> {
    let window: Vec<RBig> = (from..from + 40).map(&mut *term).collect::<Result<_>>()?;
    let alternates = |w: &[RBig]| w.windows(2).all(|p| (&p[0] * &p[1]) < RBig::ZERO);
    if let Some(start) = (0..20).find(|s| alternates(&window[*s..])) {
        return alternating(term, from, from + start as i64, digits);
    }
    // Same sign from some point on: the ratio of the terms must stay well below 1.
    let ratio = |term: &mut dyn FnMut(i64) -> Result<RBig>, k: i64| -> Result<Option<RBig>> {
        let (a, b) = (term(k)?, term(k + 1)?);
        Ok((a != RBig::ZERO).then(|| abs(&b) / abs(&a)))
    };
    let limit = RBig::from_parts(IBig::from(95), UBig::from(100u8));
    for k in [from + 100, from + 200, from + 400] {
        match ratio(term, k)? {
            Some(r) if r < limit => {}
            None => {}
            Some(r) if r > RBig::ONE + RBig::from_parts(IBig::ONE, UBig::from(1000u16)) => {
                return Err(diverges("its terms grow"));
            }
            Some(_) => return Err(too_slow()),
        }
    }
    let negligible = RBig::from_parts(IBig::ONE, UBig::from(10u8).pow(digits + 5));
    let mut sum = RBig::ZERO;
    let mut quiet = 0;
    for k in from..from + MAX_TERMS {
        let a = term(k)?;
        sum += &a;
        let small = abs(&a) <= &negligible * (abs(&sum) + &negligible);
        quiet = if small && k > from + 10 { quiet + 1 } else { 0 };
        if quiet >= 5 {
            return Ok(sum);
        }
    }
    Err(too_slow())
}

/// An alternating series from `start` on, the terms before it added as they are. The size of
/// the terms must fall towards 0 at points far out.
fn alternating(term: &mut dyn FnMut(i64) -> Result<RBig>, from: i64, start: i64, digits: usize) -> Result<RBig> {
    let mut size = abs(&term(start)?);
    for k in [start + 100, start + 10_000, start + 1_000_000] {
        let next = abs(&term(k)?);
        if next >= size {
            return Err(diverges("its terms do not tend to 0"));
        }
        size = next;
    }
    if size * RBig::from(100) > abs(&term(start)?) {
        return Err(too_slow());
    }
    let mut head = RBig::ZERO;
    for k in from..start {
        head += term(k)?;
    }
    // Σ (-1)^k b_k with b_k = ±a(start + k), the sign that makes b_0 positive.
    let first = term(start)?;
    let sign = if first < RBig::ZERO { -RBig::ONE } else { RBig::ONE };
    let n = (digits as f64 * 1.31).ceil() as i64 + 2;
    // d = ((3 + √8)^n + (3 - √8)^n) / 2, a whole number: u_{k+1} = 6 u_k - u_{k-1}.
    let (mut u0, mut u1) = (IBig::from(2), IBig::from(6));
    for _ in 1..n {
        let next = IBig::from(6) * &u1 - &u0;
        u0 = u1;
        u1 = next;
    }
    let d = RBig::from(u1) / RBig::from(2);
    let (mut b, mut c, mut s) = (-RBig::ONE, -d.clone(), RBig::ZERO);
    for k in 0..n {
        let a = &sign * term(start + k)?;
        let alternating_sign = if k % 2 == 0 { RBig::ONE } else { -RBig::ONE };
        c = &b - &c;
        s += &c * (a * alternating_sign);
        let (kk, nn) = (RBig::from(k), RBig::from(n));
        b = b * (&kk + &nn) * (&kk - &nn) / ((&kk + RBig::from_parts(IBig::ONE, UBig::from(2u8))) * (&kk + RBig::ONE));
    }
    Ok(head + sign * s / d)
}

fn abs(r: &RBig) -> RBig {
    if *r < RBig::ZERO {
        -r.clone()
    } else {
        r.clone()
    }
}

fn degree(c: &[RBig]) -> usize {
    c.iter().rposition(|x| *x != RBig::ZERO).unwrap_or(0)
}

fn evaluate(c: &[RBig], x: &RBig) -> RBig {
    c.iter().rev().fold(RBig::ZERO, |sum, a| sum * x + a)
}

/// The coefficients of `c(n + h)` in `h`.
fn taylor_shift(c: &[RBig], n: i64) -> Vec<RBig> {
    let mut out = c.to_vec();
    let n = RBig::from(n);
    let len = out.len();
    for i in 0..len {
        for j in (i..len - 1).rev() {
            let carry = &out[j + 1] * &n;
            out[j] += carry;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(n: i64) -> RBig {
        RBig::from(n)
    }

    #[test]
    fn a_taylor_shift_moves_the_polynomial() {
        // (n + h)² at n = 3: 9 + 6h + h²
        assert_eq!(taylor_shift(&[r(0), r(0), r(1)], 3), [r(9), r(6), r(1)]);
    }

    #[test]
    fn the_alternating_harmonic_series_is_ln_2() {
        let mut term = |k: i64| Ok(RBig::from_parts(IBig::from(if k % 2 == 1 { 1 } else { -1 }), UBig::from(k as u64)));
        let s = numeric_series(&mut term, 1, 40).unwrap();
        let ln2 = RBig::from_parts("6931471805599453094172321214581765680755".parse::<IBig>().unwrap(), UBig::from(10u8).pow(40));
        assert!(abs(&(s - ln2)) < RBig::from_parts(IBig::ONE, UBig::from(10u8).pow(35)));
    }

    #[test]
    fn rational_series_that_diverge_are_refused() {
        assert!(rational_tail_start(&[r(1)], &[r(0), r(1)], 1).unwrap_err().to_string().contains("1/k"));
        assert!(rational_tail_start(&[r(0), r(1)], &[r(1), r(1)], 1).unwrap_err().to_string().contains("do not tend to 0"));
        assert_eq!(rational_tail_start(&[r(1)], &[r(0), r(0), r(1)], 1).unwrap(), 201);
    }
}
