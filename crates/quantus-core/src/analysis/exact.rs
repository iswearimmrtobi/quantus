//! Exact values the simplifier knows by heart: roots of rational numbers in simplest radical form
//! (`\sqrt{8} = 2\sqrt{2}`, `\frac{1}{\sqrt{2}} = \frac{\sqrt{2}}{2}`), sine, cosine and tangent at
//! multiples of π/6 and π/4, and the inverse functions at those values.

use dashu::base::{Abs, BitTest, Signed, UnsignedAbs};
use dashu::integer::{IBig, UBig};
use dashu::rational::RBig;

use super::expr::{Expr, Fn};

/// Integer powers larger than this many bits are left as they are.
const MAX_POWER_BITS: usize = 100_000;

/// Radicands are only searched for perfect powers by trial division up to this bound.
const TRIAL_DIVISION_LIMIT: u64 = 1_000_000;

/// `base^exponent` in canonical form, or `None` when it is not a real number (an even root of a
/// negative number, a negative power of 0) or too large to work out.
///
/// The result is `c · i^{1/q}` with rational `c` and a whole radicand `i` without q-th powers,
/// so the denominator is rational: `2^{-1/2} = \frac{1}{2}\sqrt{2}`.
pub fn radical(base: &RBig, exponent: &RBig) -> Option<Expr> {
    let p = exponent.numerator().clone();
    let q = exponent.denominator().clone();
    if *base == RBig::ZERO {
        return (p > IBig::ZERO).then(|| Expr::int(0));
    }
    let negative_base = base.is_negative();
    if negative_base && q.clone() % UBig::from(2u8) == UBig::ZERO {
        return None;
    }
    let a = base.clone().abs();
    // a^{p/q} = a^k · a^{r/q} with 0 ≤ r < q.
    let qi = IBig::from(q.clone());
    let k = floor_div(&p, &qi);
    let r = &p - &k * &qi;
    let coefficient = integer_power(&a, &k)?;
    let sign = if negative_base && &p % IBig::from(2) != IBig::ZERO { -RBig::ONE } else { RBig::ONE };
    if r == IBig::ZERO {
        return Some(Expr::Num(sign * coefficient));
    }
    // (n/d)^{r/q} = (n^r d^{q-r})^{1/q} / d
    let n = a.numerator().unsigned_abs();
    let d = a.denominator().clone();
    let r = usize::try_from(r.unsigned_abs()).ok()?;
    let qu = usize::try_from(q.clone()).ok()?;
    let radicand = n.pow(r) * d.pow(qu - r);
    if radicand.bit_len() > MAX_POWER_BITS {
        return None;
    }
    let (outside, inside) = extract_powers(&radicand, qu);
    let c = sign * coefficient * RBig::from_parts(IBig::from(outside), d);
    Some(scaled_root(c, inside, qu))
}

/// `c · i^{1/q}` in canonical shape.
fn scaled_root(c: RBig, inside: UBig, q: usize) -> Expr {
    if inside == UBig::ONE {
        return Expr::Num(c);
    }
    let root = Expr::pow(Expr::Num(RBig::from(IBig::from(inside))), Expr::Num(RBig::from_parts(IBig::ONE, UBig::from(q))));
    if c == RBig::ONE {
        root
    } else {
        Expr::Product(vec![Expr::Num(c), root])
    }
}

/// `c\sqrt{n}` for the trig tables.
fn root_of(c: RBig, n: u64) -> Expr {
    scaled_root(c, UBig::from(n), 2)
}

fn floor_div(a: &IBig, b: &IBig) -> IBig {
    let quotient = a / b;
    if (a % b != IBig::ZERO) && ((a < &IBig::ZERO) != (b < &IBig::ZERO)) {
        quotient - IBig::ONE
    } else {
        quotient
    }
}

fn integer_power(a: &RBig, k: &IBig) -> Option<RBig> {
    let magnitude = usize::try_from(k.unsigned_abs()).ok()?;
    let bits = a.numerator().unsigned_abs().bit_len().max(a.denominator().bit_len());
    if bits.saturating_mul(magnitude) > MAX_POWER_BITS {
        return None;
    }
    let power = RBig::from_parts(a.numerator().pow(magnitude), a.denominator().pow(magnitude));
    Some(if k.is_negative() { RBig::ONE / power } else { power })
}

/// `m = outside^q · inside` with `inside` free of q-th powers (as far as trial division finds).
fn extract_powers(m: &UBig, q: usize) -> (UBig, UBig) {
    let mut outside = UBig::ONE;
    let mut inside = m.clone();
    let mut p = 2u64;
    while p <= TRIAL_DIVISION_LIMIT {
        let prime = UBig::from(p);
        let power = prime.pow(q);
        if power > inside {
            break;
        }
        while inside.clone() % &power == UBig::ZERO {
            inside /= &power;
            outside *= &prime;
        }
        p += if p == 2 { 1 } else { 2 };
    }
    // Beyond the trial bound: the whole rest may still be a perfect power.
    if inside > UBig::ONE {
        if let Some(root) = exact_root(&inside, q) {
            outside *= root;
            inside = UBig::ONE;
        }
    }
    (outside, inside)
}

fn exact_root(n: &UBig, q: usize) -> Option<UBig> {
    let guess = n.to_f64().value().powf(1.0 / q as f64).round();
    if !guess.is_finite() || guess > 1e15 {
        return None;
    }
    let base = guess as u64;
    (base.saturating_sub(1)..=base + 1).map(UBig::from).find(|candidate| candidate.pow(q) == *n)
}

/// `r` when `e` is `r·π` (0 counts, as `0·π`).
pub fn pi_multiple(e: &Expr) -> Option<RBig> {
    match e {
        Expr::Num(r) if *r == RBig::ZERO => Some(RBig::ZERO),
        Expr::Pi => Some(RBig::ONE),
        Expr::Product(factors) => match factors.as_slice() {
            [Expr::Num(r), Expr::Pi] => Some(r.clone()),
            _ => None,
        },
        _ => None,
    }
}

/// `r·π` in canonical shape.
pub fn times_pi(r: RBig) -> Expr {
    if r == RBig::ZERO {
        Expr::int(0)
    } else if r == RBig::ONE {
        Expr::Pi
    } else {
        Expr::Product(vec![Expr::Num(r), Expr::Pi])
    }
}

/// The value of a trig function at a multiple of π/6 or π/4.
pub enum TrigValue {
    Value(Expr),
    /// The function has a pole there (tan at π/2).
    Undefined,
}

/// sin, cos or tan of `r·π`, when `r` is a multiple of 1/6 or 1/4.
pub fn trig_at(function: Fn, r: &RBig) -> Option<TrigValue> {
    let degrees = degrees_of(r)?;
    Some(match function {
        Fn::Sin => TrigValue::Value(sine(degrees)),
        Fn::Cos => TrigValue::Value(sine((degrees + 90) % 360)),
        Fn::Tan => match tangent(degrees) {
            Some(value) => TrigValue::Value(value),
            None => TrigValue::Undefined,
        },
        _ => return None,
    })
}

/// `r·180` reduced to [0, 360), when it is a multiple of 30 or 45.
fn degrees_of(r: &RBig) -> Option<i64> {
    let degrees = r * RBig::from(180);
    if !degrees.is_int() {
        return None;
    }
    let d = i64::try_from(degrees.numerator().clone()).ok()?.rem_euclid(360);
    (d % 30 == 0 || d % 45 == 0).then_some(d)
}

/// sin of a multiple of 30° or 45° in [0, 360).
fn sine(degrees: i64) -> Expr {
    let (reference, negative) = match degrees {
        0..=90 => (degrees, false),
        91..=180 => (180 - degrees, false),
        181..=270 => (degrees - 180, true),
        _ => (360 - degrees, true),
    };
    let half = RBig::from_parts(IBig::ONE, UBig::from(2u8));
    let sign = if negative { -RBig::ONE } else { RBig::ONE };
    match reference {
        0 => Expr::int(0),
        30 => Expr::Num(sign * half),
        45 => root_of(sign * half, 2),
        60 => root_of(sign * half, 3),
        _ => Expr::Num(sign),
    }
}

/// tan of a multiple of 30° or 45° in [0, 360); `None` at 90° and 270°.
fn tangent(degrees: i64) -> Option<Expr> {
    let (reference, negative) = match degrees % 180 {
        d @ 0..=90 => (d, false),
        d => (180 - d, true),
    };
    let sign = if negative { -RBig::ONE } else { RBig::ONE };
    Some(match reference {
        0 => Expr::int(0),
        30 => root_of(sign * RBig::from_parts(IBig::ONE, UBig::from(3u8)), 3),
        45 => Expr::Num(sign),
        60 => root_of(sign, 3),
        _ => return None,
    })
}

/// arcsin, arccos or arctan of a value from the tables, as a multiple of π.
pub fn inverse_trig_at(function: Fn, value: &Expr) -> Option<Expr> {
    let candidates: Vec<i64> = match function {
        Fn::Arcsin => vec![-90, -60, -45, -30, 0, 30, 45, 60, 90],
        Fn::Arccos => vec![0, 30, 45, 60, 90, 120, 135, 150, 180],
        Fn::Arctan => vec![-60, -45, -30, 0, 30, 45, 60],
        _ => return None,
    };
    candidates.into_iter().find_map(|degrees| {
        let at = degrees.rem_euclid(360);
        let matches = match function {
            Fn::Arcsin => sine(at) == *value,
            Fn::Arccos => sine((at + 90) % 360) == *value,
            _ => tangent(at).as_ref() == Some(value),
        };
        matches.then(|| times_pi(RBig::from_parts(IBig::from(degrees), UBig::from(180u8))))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(n: i64, d: u64) -> RBig {
        RBig::from_parts(IBig::from(n), UBig::from(d))
    }

    fn root(c: RBig, n: i64, q: u64) -> Expr {
        let root = Expr::pow(Expr::int(n), Expr::ratio(1, q));
        if c == RBig::ONE { root } else { Expr::Product(vec![Expr::Num(c), root]) }
    }

    #[test]
    fn roots_are_in_simplest_radical_form() {
        let half = r(1, 2);
        assert_eq!(radical(&RBig::from(8), &half), Some(root(RBig::from(2), 2, 2)));
        assert_eq!(radical(&r(4, 9), &half), Some(Expr::Num(r(2, 3))));
        assert_eq!(radical(&RBig::from(2), &r(-1, 2)), Some(root(half.clone(), 2, 2)));
        assert_eq!(radical(&r(1, 3), &half), Some(root(r(1, 3), 3, 2)));
        assert_eq!(radical(&RBig::from(-8), &r(1, 3)), Some(Expr::int(-2)));
        assert_eq!(radical(&RBig::from(16), &r(1, 3)), Some(root(RBig::from(2), 2, 3)));
        assert_eq!(radical(&RBig::from(8), &r(2, 3)), Some(Expr::int(4)));
        assert_eq!(radical(&RBig::from(2), &RBig::from(-3)), Some(Expr::Num(r(1, 8))));
        assert_eq!(radical(&RBig::from(-4), &half), None);
        assert_eq!(radical(&RBig::ZERO, &RBig::from(-1)), None);
        assert_eq!(radical(&RBig::from(3), &half), Some(root(RBig::ONE, 3, 2)));
    }

    #[test]
    fn trig_tables_cover_the_unit_circle() {
        let value = |f, n, d| match trig_at(f, &r(n, d)) {
            Some(TrigValue::Value(v)) => Some(v),
            _ => None,
        };
        assert_eq!(value(Fn::Sin, 1, 6), Some(Expr::Num(r(1, 2))));
        assert_eq!(value(Fn::Cos, 3, 4), Some(root(r(-1, 2), 2, 2)));
        assert_eq!(value(Fn::Sin, 3, 2), Some(Expr::int(-1)));
        assert_eq!(value(Fn::Cos, -1, 3), Some(Expr::Num(r(1, 2))));
        assert_eq!(value(Fn::Tan, 2, 3), Some(root(RBig::from(-1), 3, 2)));
        assert_eq!(value(Fn::Tan, 1, 6), Some(root(r(1, 3), 3, 2)));
        assert_eq!(value(Fn::Sin, 7, 1), Some(Expr::int(0)));
        assert!(matches!(trig_at(Fn::Tan, &r(1, 2)), Some(TrigValue::Undefined)));
        assert!(trig_at(Fn::Sin, &r(1, 5)).is_none());
    }

    #[test]
    fn inverse_functions_read_the_tables_backwards() {
        assert_eq!(inverse_trig_at(Fn::Arcsin, &Expr::Num(r(1, 2))), Some(Expr::Product(vec![Expr::Num(r(1, 6)), Expr::Pi])));
        assert_eq!(inverse_trig_at(Fn::Arccos, &Expr::Num(r(-1, 2))), Some(Expr::Product(vec![Expr::Num(r(2, 3)), Expr::Pi])));
        assert_eq!(inverse_trig_at(Fn::Arctan, &Expr::int(-1)), Some(Expr::Product(vec![Expr::Num(r(-1, 4)), Expr::Pi])));
        assert_eq!(inverse_trig_at(Fn::Arccos, &Expr::int(-1)), Some(Expr::Pi));
        assert_eq!(inverse_trig_at(Fn::Arcsin, &Expr::Num(r(1, 3))), None);
    }
}
