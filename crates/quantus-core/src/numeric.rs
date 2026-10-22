//! Numerical integration at the working precision: tanh-sinh quadrature.
//!
//! The double-exponential substitution converges so fast that fifty digits take a few hundred
//! samples, and it copes with integrable singularities at the ends (`\ln x`, `1/\sqrt{x}` on
//! `[0, 1]`) because its nodes crowd towards the endpoints without ever reaching them.
//!
//! Everything runs on [`BigFloat`]; samples go to the integrand as [`Decimal`]s, which the
//! evaluator turns back into exact rationals.

use std::cell::RefCell;
use std::collections::HashMap;

use astro_float::{BigFloat, Consts, Radix, RoundingMode};

use crate::num::{parse_float_text, Decimal};
use crate::{QuantusError, Result};

const RM: RoundingMode = RoundingMode::ToEven;
const MAX_LEVEL: u32 = 9;

struct Context {
    bits: usize,
    digits: usize,
    consts: Consts,
}

impl Context {
    fn new(digits: usize) -> Result<Context> {
        let consts = Consts::new().map_err(|_| QuantusError::other("could not initialise constants"))?;
        let bits = ((digits + 10) as f64 * std::f64::consts::LOG2_10).ceil() as usize + 64;
        Ok(Context { bits, digits, consts })
    }

    fn float(&mut self, decimal: &Decimal) -> BigFloat {
        BigFloat::parse(&decimal.to_plain_string(), Radix::Dec, self.bits, RM, &mut self.consts)
    }

    fn int(&self, value: i64) -> BigFloat {
        BigFloat::from_i64(value, self.bits)
    }

    fn decimal(&mut self, value: &BigFloat) -> Result<Decimal> {
        if value.is_zero() {
            return Ok(Decimal::from_i64(0));
        }
        let text = value
            .format(Radix::Dec, RM, &mut self.consts)
            .map_err(|_| QuantusError::other("could not format a number"))?;
        parse_float_text(&text, self.digits + 10)
            .ok_or_else(|| QuantusError::other("could not read a computed number"))
    }

    fn exp(&mut self, x: &BigFloat) -> BigFloat {
        x.exp(self.bits, RM, &mut self.consts)
    }
}

/// One pair of symmetric quadrature nodes: the sample points (`None` where one rounded onto an
/// endpoint) and their common weight.
#[derive(Clone)]
struct Node {
    points: [Option<Decimal>; 2],
    weight: BigFloat,
}

type NodeLevels = Vec<Vec<Node>>;

thread_local! {
    /// Nodes depend only on the interval and the precision, and a nested integral asks for the
    /// same interval again for every sample of the outer one. Borrowed only for the moment of a
    /// lookup, because the integrand may itself integrate.
    static NODES: RefCell<HashMap<String, NodeLevels>> = RefCell::new(HashMap::new());
}

const MAX_CACHED_RULES: usize = 64;

fn cached_node(key: &str, level: usize, index: usize) -> Option<Node> {
    NODES.with(|cache| cache.borrow().get(key)?.get(level)?.get(index).cloned())
}

fn store_node(key: &str, level: usize, index: usize, node: &Node) {
    NODES.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() >= MAX_CACHED_RULES && !cache.contains_key(key) {
            cache.clear();
        }
        let levels = cache.entry(key.to_string()).or_default();
        if levels.len() <= level {
            levels.resize_with(level + 1, Vec::new);
        }
        // Nodes are filled in order, so a slot is either the next one or already there.
        if levels[level].len() == index {
            levels[level].push(node.clone());
        }
    });
}

/// `∫ f` from `a` to `b`, accurate to roughly `digits` significant digits.
///
/// `f` receives each sample point and returns the integrand there in one fixed unit. It is
/// never called on an endpoint itself.
pub(crate) fn integrate(
    a: &Decimal,
    b: &Decimal,
    digits: usize,
    f: &mut dyn FnMut(&Decimal) -> Result<Decimal>,
) -> Result<Decimal> {
    let (lower, upper) = (a.to_rational(), b.to_rational());
    if lower == upper {
        return Ok(Decimal::from_i64(0));
    }
    let (a, b, sign) = if lower < upper { (a, b, 1) } else { (b, a, -1) };

    // Samples sit at distances from an endpoint far below the requested precision, so that an
    // integrable singularity like 1/sqrt(x) has its tail cut off where it no longer matters.
    let mut cx = Context::new(2 * digits + 20)?;
    let (fa, fb) = (cx.float(a), cx.float(b));
    let bits = cx.bits;
    let two = cx.int(2);
    let c = fb.sub(&fa, bits, RM).div(&two, bits, RM);
    let half_pi = cx.consts.pi(bits, RM).div(&two, bits, RM);
    let c_half_pi = c.mul(&half_pi, bits, RM);

    // Where the nodes get closer to an endpoint than the working precision can tell apart.
    let u_needed = 0.5 * std::f64::consts::LN_10 * cx.digits as f64;
    let t_max = (u_needed * 2.0 / std::f64::consts::PI).asinh();

    let key = format!("{}|{}|{digits}", a.to_plain_string(), b.to_plain_string());
    let rule = Rule { fa: fa.clone(), fb: fb.clone(), c, half_pi, c_half_pi: c_half_pi.clone() };

    let mid = fa.add(&fb, bits, RM).div(&two, bits, RM);
    let mid = cx.decimal(&mid)?;
    let mut total = if mid.to_rational() == lower || mid.to_rational() == upper {
        cx.int(0)
    } else {
        let y = f(&mid)?;
        cx.float(&y).mul(&c_half_pi, bits, RM)
    };
    let mut magnitude = total.abs();

    let tolerance_exponent = digits.saturating_sub(3);
    let ten = cx.int(10);
    let tolerance = cx.int(1).div(&ten.powi(tolerance_exponent, bits, RM), bits, RM);

    let cutoff_factor = cx.int(1).div(&ten.powi(digits + 8, bits, RM), bits, RM);

    let mut previous: Option<BigFloat> = None;
    for level in 0..=MAX_LEVEL {
        let h = cx.int(1).div(&cx.int(1_i64 << level), bits, RM);
        let step: i64 = if level == 0 { 1 } else { 2 };
        let mut k = 1_i64;
        let mut negligible = 0;
        while (k as f64) / ((1_i64 << level) as f64) <= t_max {
            let index = (k / step) as usize;
            let node = match cached_node(&key, level as usize, index) {
                Some(node) => node,
                None => {
                    let node = compute_node(&mut cx, &rule, k, &h, &lower, &upper)?;
                    store_node(&key, level as usize, index, &node);
                    node
                }
            };
            let weight = &node.weight;
            let mut largest = cx.int(0);
            for x in node.points.iter().flatten() {
                let y = f(x)?;
                let y = cx.float(&y);
                let term = weight.mul(&y, bits, RM);
                let size = term.abs();
                if size.cmp(&largest).is_some_and(|o| o > 0) {
                    largest = size.clone();
                }
                magnitude = magnitude.add(&size, bits, RM);
                total = total.add(&term, bits, RM);
            }
            // Past the point where nodes stop contributing there is nothing left to add.
            let cutoff = magnitude.mul(&cutoff_factor, bits, RM);
            negligible = if largest.cmp(&cutoff).is_some_and(|o| o <= 0) { negligible + 1 } else { 0 };
            if negligible >= 2 {
                break;
            }
            k += step;
        }

        let estimate = total.mul(&h, bits, RM);
        if let Some(previous) = &previous {
            let difference = estimate.sub(previous, bits, RM).abs();
            let allowed = magnitude.mul(&h, bits, RM).mul(&tolerance, bits, RM);
            if level >= 2 && difference.cmp(&allowed).is_some_and(|ordering| ordering <= 0) {
                let value = if sign < 0 { estimate.neg() } else { estimate };
                return cx.decimal(&value);
            }
        }
        previous = Some(estimate);
    }
    Err(QuantusError::other(
        "The integral did not converge. The integrand may be singular inside the interval or oscillate too fast.",
    ))
}

struct Rule {
    fa: BigFloat,
    fb: BigFloat,
    /// Half the interval's length.
    c: BigFloat,
    half_pi: BigFloat,
    c_half_pi: BigFloat,
}

/// The nodes at `t = k h`: with `u = (π/2) sinh t` they sit `2c / (e^{2u} + 1)` inside each end.
fn compute_node(
    cx: &mut Context,
    rule: &Rule,
    k: i64,
    h: &BigFloat,
    lower: &dashu::rational::RBig,
    upper: &dashu::rational::RBig,
) -> Result<Node> {
    let bits = cx.bits;
    let two = cx.int(2);
    let t = cx.int(k).mul(h, bits, RM);
    let e_t = cx.exp(&t);
    let e_minus_t = cx.int(1).div(&e_t, bits, RM);
    let sinh = e_t.sub(&e_minus_t, bits, RM).div(&two, bits, RM);
    let cosh = e_t.add(&e_minus_t, bits, RM).div(&two, bits, RM);
    let u2 = rule.half_pi.mul(&sinh, bits, RM).mul(&two, bits, RM);
    let e2u = cx.exp(&u2);
    let denominator = e2u.add(&cx.int(1), bits, RM);
    let delta = rule.c.mul(&two, bits, RM).div(&denominator, bits, RM);
    let weight = rule
        .c_half_pi
        .mul(&cosh, bits, RM)
        .mul(&cx.int(4), bits, RM)
        .mul(&e2u, bits, RM)
        .div(&denominator.mul(&denominator, bits, RM), bits, RM);

    let mut points = [None, None];
    for (slot, x) in points.iter_mut().zip([rule.fa.add(&delta, bits, RM), rule.fb.sub(&delta, bits, RM)]) {
        let x = cx.decimal(&x)?;
        let r = x.to_rational();
        // A point that rounded onto an endpoint has a negligible weight, and the integrand may
        // be singular right there.
        if r != *lower && r != *upper {
            *slot = Some(x);
        }
    }
    Ok(Node { points, weight })
}
