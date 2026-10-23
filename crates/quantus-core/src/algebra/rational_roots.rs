//! Rational roots of a univariate polynomial with rational coefficients.
//!
//! The rational root theorem says a root `p/q` in lowest terms has `p | a_0` and `q | a_n`.
//! Listing every divisor works while both coefficients are small; an expanded product of a dozen
//! linear factors has a constant term far beyond that (`…·15^8 ≈ 2.5·10^9`). For those the
//! substitution `y = a_n x` turns f into the monic integer polynomial
//! g(y) = a_n^(n-1) f(y / a_n), whose rational roots are integers dividing g(0) and lying within
//! Fujiwara's bound: one scan with a cheap divisibility filter finds them all. Only when that range
//! is too wide are the roots located numerically (Aberth's method in f64) and the rationals next
//! to them tested. Every candidate is checked exactly. Roots found recently are tried first, since
//! the callers divide off one root at a time and ask again about the quotient.

use std::cell::RefCell;
use std::collections::HashMap;

use dashu::base::UnsignedAbs;
use dashu::integer::{IBig, UBig};
use dashu::rational::RBig;

use crate::algebra::poly::Poly;

/// Divisor lists are complete up to this size; beyond it the bounded scan takes over.
const EXHAUSTIVE_LIMIT: u64 = 100_000;

/// Most numerators the bounded scan tries in total, over all denominators.
const SCAN_LIMIT: f64 = 5_000_000.0;

thread_local! {
    static CACHE: RefCell<HashMap<Vec<IBig>, Option<RBig>>> = RefCell::new(HashMap::new());
    /// Roots found by the last scans, tried first on the next polynomial.
    static RECENT: RefCell<Vec<RBig>> = const { RefCell::new(Vec::new()) };
}

/// A rational root of `poly` in `var`, if it has one. `poly` must be univariate in `var`.
pub fn find_rational_root(poly: &Poly, var: &str) -> Option<RBig> {
    let degree = poly.degree_of(var);
    if degree == 0 {
        return None;
    }
    let coefficients = integer_coefficients(poly, var, degree);
    if coefficients[0].is_zero() {
        return Some(RBig::ZERO);
    }
    if let Some(known) = CACHE.with(|c| c.borrow().get(&coefficients).cloned()) {
        return known;
    }
    let constant = coefficients[0].clone().unsigned_abs();
    let leading = coefficients[degree].clone().unsigned_abs();
    let recent = RECENT.with(|r| r.borrow().iter().find(|x| evaluate(&coefficients, x).is_zero()).cloned());
    let root = if recent.is_some() {
        recent
    } else if constant <= UBig::from(EXHAUSTIVE_LIMIT) && leading <= UBig::from(EXHAUSTIVE_LIMIT) {
        exhaustive(&coefficients, &constant, &leading)
    } else if let Some(all) = bounded_scan(&coefficients).or_else(|| {
        // Repeated factors inflate the leading coefficient and spoil the numerics; the
        // square-free part f / gcd(f, f') has the same roots, each once.
        let reduced = square_free(poly, var)?;
        let d = reduced.degree_of(var);
        (d > 0 && d < degree).then(|| bounded_scan(&integer_coefficients(&reduced, var, d))).flatten()
    }) {
        RECENT.with(|r| {
            let mut r = r.borrow_mut();
            r.retain(|x| !all.contains(x));
            r.splice(0..0, all.iter().cloned());
            r.truncate(256);
        });
        all.into_iter().next()
    } else {
        numerical(&coefficients, &leading)
    };
    CACHE.with(|c| {
        let mut cache = c.borrow_mut();
        if cache.len() > 4096 {
            cache.clear();
        }
        cache.insert(coefficients, root.clone());
    });
    root
}

/// Fujiwara's bound on the absolute value of every root.
fn root_bound(coefficients: &[IBig]) -> Option<f64> {
    let n = coefficients.len() - 1;
    let lead = to_f64(&coefficients[n])?.abs();
    let mut bound = 0.0f64;
    for k in 1..=n {
        let c = to_f64(&coefficients[n - k])?.abs() / lead;
        let c = if k == n { c / 2.0 } else { c };
        bound = bound.max(c.powf(1.0 / k as f64));
    }
    let bound = 2.0 * bound;
    bound.is_finite().then_some(bound)
}

/// All rational roots, from the integer roots of g(y) = a_n^(n-1) f(y / a_n) within the root
/// bound, or `None` when that range is too wide to scan.
fn bounded_scan(coefficients: &[IBig]) -> Option<Vec<RBig>> {
    let n = coefficients.len() - 1;
    let lead = coefficients[n].clone();
    let lead_abs = to_f64(&lead)?.abs();
    let range = (root_bound(coefficients)? * lead_abs).ceil() + 1.0;
    if !(range <= SCAN_LIMIT) {
        return None;
    }
    // g_i = a_i a_n^(n-1-i), g_n = 1
    let mut g = vec![IBig::ZERO; n + 1];
    let mut power = IBig::ONE;
    for i in (0..n).rev() {
        g[i] = &coefficients[i] * &power;
        power *= &lead;
    }
    g[n] = IBig::ONE;
    let constant = g[0].clone();
    let mut roots = Vec::new();
    for magnitude in 1..=(range as i64) {
        let m = IBig::from(magnitude);
        if !(&constant % &m).is_zero() {
            continue;
        }
        for y in [m.clone(), -m] {
            let mut value = IBig::ONE;
            for c in g[..n].iter().rev() {
                value = value * &y + c;
            }
            if value.is_zero() {
                roots.push(RBig::from(y) / RBig::from(lead.clone()));
            }
        }
    }
    Some(roots)
}

/// f / gcd(f, f'), or `None` when f has no repeated factor.
pub(crate) fn square_free(poly: &Poly, var: &str) -> Option<Poly> {
    let terms = poly
        .terms
        .iter()
        .filter_map(|t| {
            let e = t.monomial.degree_of(var);
            (e > 0).then(|| {
                crate::algebra::poly::Term::new(
                    &t.coeff * RBig::from(e),
                    crate::algebra::poly::Monomial::single(var, e - 1),
                )
            })
        })
        .collect();
    let derivative = Poly::from_terms(terms);
    let g = poly.gcd(&derivative);
    if g.degree_of(var) == 0 {
        return None;
    }
    let (quotient, remainder) = poly.div_rem(&g).ok()?;
    remainder.is_zero().then_some(quotient)
}

/// `a_0 … a_n` with integer values, scaled by the common denominator.
fn integer_coefficients(poly: &Poly, var: &str, degree: usize) -> Vec<IBig> {
    let mut rational = vec![RBig::ZERO; degree + 1];
    for term in &poly.terms {
        rational[term.monomial.degree_of(var)] += &term.coeff;
    }
    let mut lcm = UBig::ONE;
    for c in &rational {
        let d = c.denominator().clone();
        let g = gcd(&lcm, &d);
        lcm = &lcm / &g * &d;
    }
    rational.iter().map(|c| (c * RBig::from(IBig::from(lcm.clone()))).numerator().clone()).collect()
}

fn gcd(a: &UBig, b: &UBig) -> UBig {
    let (mut a, mut b) = (a.clone(), b.clone());
    while !b.is_zero() {
        let r = &a % &b;
        a = b;
        b = r;
    }
    a
}

fn evaluate(coefficients: &[IBig], x: &RBig) -> RBig {
    let mut value = RBig::ZERO;
    for c in coefficients.iter().rev() {
        value = value * x + RBig::from(c.clone());
    }
    value
}

fn small_divisors(n: u64) -> Vec<u64> {
    let mut divisors = Vec::new();
    let mut i = 1;
    while i * i <= n {
        if n % i == 0 {
            divisors.push(i);
            if i * i != n {
                divisors.push(n / i);
            }
        }
        i += 1;
    }
    divisors.sort_unstable();
    divisors
}

fn exhaustive(coefficients: &[IBig], constant: &UBig, leading: &UBig) -> Option<RBig> {
    let ps = small_divisors(u64::try_from(constant).ok()?);
    let qs = small_divisors(u64::try_from(leading).ok()?);
    for p in &ps {
        for q in &qs {
            for sign in [1i64, -1] {
                let candidate = RBig::from_parts(IBig::from(*p) * sign, UBig::from(*q));
                if evaluate(coefficients, &candidate).is_zero() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

/// Divisors of the leading coefficient, from its small prime factors; a large leftover cofactor
/// counts as one more prime. Capped, since only a few are ever needed in practice.
fn leading_divisors(leading: &UBig) -> Vec<UBig> {
    let mut primes: Vec<(UBig, u32)> = Vec::new();
    let mut rest = leading.clone();
    let mut p = 2u64;
    while p < 10_000 && rest > UBig::ONE {
        let up = UBig::from(p);
        let mut e = 0;
        while (&rest % &up).is_zero() {
            rest = &rest / &up;
            e += 1;
        }
        if e > 0 {
            primes.push((up, e));
        }
        p += if p == 2 { 1 } else { 2 };
    }
    if rest > UBig::ONE {
        primes.push((rest, 1));
    }
    let mut divisors = vec![UBig::ONE];
    for (prime, e) in primes {
        let mut next = Vec::new();
        for d in &divisors {
            let mut power = UBig::ONE;
            for _ in 0..=e {
                next.push(d * &power);
                power = &power * &prime;
            }
        }
        divisors = next;
        if divisors.len() > 4096 {
            divisors.truncate(4096);
            break;
        }
    }
    divisors.sort();
    divisors
}

fn numerical(coefficients: &[IBig], leading: &UBig) -> Option<RBig> {
    let approximations = aberth(coefficients)?;
    let denominators = leading_divisors(leading);
    for (re, _) in approximations {
        if !re.is_finite() {
            continue;
        }
        for q in &denominators {
            let Ok(qf) = f64::try_from(q.clone()) else { continue };
            let center = (re * qf).round();
            if !center.is_finite() || center.abs() > 1e18 {
                continue;
            }
            for offset in [0.0, -1.0, 1.0] {
                let candidate = RBig::from_parts(IBig::from((center + offset) as i64), q.clone());
                if evaluate(coefficients, &candidate).is_zero() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

/// All complex roots of the polynomial, approximately, by Aberth–Ehrlich iteration.
fn aberth(coefficients: &[IBig]) -> Option<Vec<(f64, f64)>> {
    let n = coefficients.len() - 1;
    let lead = to_f64(&coefficients[n])?;
    // Monic, in f64: a_i / a_n.
    let monic: Vec<f64> = coefficients.iter().map(|c| to_f64(c).map(|v| v / lead)).collect::<Option<_>>()?;
    if monic.iter().any(|c| !c.is_finite()) {
        return None;
    }
    // Cauchy bound for the starting circle.
    let bound = 1.0 + monic[..n].iter().fold(0.0f64, |m, c| m.max(c.abs()));
    let radius = bound.min(1e6);
    let mut z: Vec<(f64, f64)> = (0..n)
        .map(|k| {
            let angle = 2.0 * std::f64::consts::PI * (k as f64 + 0.25) / n as f64 + 0.4;
            (radius * angle.cos(), radius * angle.sin())
        })
        .collect();
    // Only the neighbourhood of each root matters (the candidates are rounded), so a modest
    // tolerance is enough; multiple roots converge slowly and would otherwise use every step.
    for _ in 0..300 {
        let mut largest_step = 0.0f64;
        for i in 0..n {
            let (p, dp) = horner(&monic, z[i]);
            if p == (0.0, 0.0) {
                continue;
            }
            let ratio = div(p, dp);
            let mut sum = (0.0, 0.0);
            for (j, zj) in z.iter().enumerate() {
                if j != i {
                    let d = sub(z[i], *zj);
                    if d != (0.0, 0.0) {
                        sum = add(sum, div((1.0, 0.0), d));
                    }
                }
            }
            let denominator = sub((1.0, 0.0), mul(ratio, sum));
            let step = if denominator == (0.0, 0.0) { ratio } else { div(ratio, denominator) };
            if !step.0.is_finite() || !step.1.is_finite() {
                continue;
            }
            z[i] = sub(z[i], step);
            largest_step = largest_step.max((step.0 * step.0 + step.1 * step.1).sqrt() / (1.0 + abs(z[i])));
        }
        if largest_step < 1e-10 {
            break;
        }
    }
    Some(z)
}

fn to_f64(c: &IBig) -> Option<f64> {
    let v = c.to_f64().value();
    v.is_finite().then_some(v)
}

fn horner(coefficients: &[f64], z: (f64, f64)) -> ((f64, f64), (f64, f64)) {
    let n = coefficients.len() - 1;
    let mut p = (1.0, 0.0); // monic
    let mut dp = (0.0, 0.0);
    for k in (0..n).rev() {
        dp = add(mul(dp, z), p);
        p = add(mul(p, z), (coefficients[k], 0.0));
    }
    (p, dp)
}

fn add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 + b.0, a.1 + b.1)
}

fn sub(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 - b.0, a.1 - b.1)
}

fn mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}

fn div(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    let d = b.0 * b.0 + b.1 * b.1;
    ((a.0 * b.0 + a.1 * b.1) / d, (a.1 * b.0 - a.0 * b.1) / d)
}

fn abs(a: (f64, f64)) -> f64 {
    (a.0 * a.0 + a.1 * a.1).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::poly::Poly;

    fn from_roots(roots: &[(i64, u64)]) -> Poly {
        let mut p = Poly::from_const(1);
        for (n, d) in roots {
            let r = RBig::from_parts(IBig::from(*n), UBig::from(*d));
            p = &p * &(&Poly::from_var("x") - &Poly::from_const(r));
        }
        p
    }

    #[test]
    fn small_coefficients_use_the_divisor_list() {
        let p = from_roots(&[(3, 1), (-2, 1), (1, 2)]);
        let r = find_rational_root(&p, "x").unwrap();
        assert!(evaluate(&integer_coefficients(&p, "x", 3), &r).is_zero());
    }

    #[test]
    fn large_constant_terms_are_found_numerically() {
        // (x + 15)^8 (x - 7)(3x + 11): the constant term is far beyond the divisor limit.
        let mut roots = vec![(-15, 1); 8];
        roots.extend([(7, 1), (-11, 3)]);
        let mut p = from_roots(&roots);
        let mut found = Vec::new();
        while let Some(r) = find_rational_root(&p, "x") {
            found.push(r.clone());
            p = p.div_rem(&(&Poly::from_var("x") - &Poly::from_const(r))).unwrap().0;
        }
        assert_eq!(found.len(), 10, "{found:?}");
    }

    #[test]
    fn no_rational_root() {
        // x^4 + 4x^2 + 1000003 · 7
        let x4 = Poly::from_term(RBig::ONE, crate::algebra::poly::Monomial::single("x", 4));
        let x2 = Poly::from_term(RBig::from(4), crate::algebra::poly::Monomial::single("x", 2));
        let p = &(&x4 + &x2) + &Poly::from_const(7_000_021);
        assert_eq!(find_rational_root(&p, "x"), None);
    }
}
