//! Inequalities in one unknown, `x^2 - 4 < 0` or chains such as `1 < 2x + 1 \le 5`, polynomial
//! and rational ones, solved exactly by a sign chart.
//!
//! Each relation's left side minus its right side is written as one fraction `P/Q` of
//! polynomials with rational coefficients. The real zeros of `P` and of every denominator met
//! on the way split the line into pieces on which the sign is fixed, and one rational point in
//! each piece decides it exactly. Zeros of a denominator are never part of the solution, even
//! where a factor cancels: `\frac{(x-1)^2}{x-1} \ge 0` is `x > 1`.

use dashu::base::UnsignedAbs;
use dashu::integer::{IBig, UBig};
use dashu::rational::RBig;

use super::derivative::depends_on;
use super::evaluator::Evaluator;
use super::polynomial;
use crate::algebra::poly::gcd_ubig;
use crate::latex::{Node, Operator};
use crate::num::Value;
use crate::{QuantusError, Result};
use crate::text::msg;

/// A polynomial by its coefficients, the constant first.
type Coefficients = Vec<RBig>;

/// An end of an interval: the exact value and a rational key that orders it.
#[derive(Clone, Debug)]
pub(crate) struct Point {
    pub value: Value,
    pub key: RBig,
}

/// A piece of the solution set; `None` is an infinite end.
#[derive(Clone, Debug)]
pub(crate) struct Interval {
    pub lo: Option<Point>,
    pub hi: Option<Point>,
    pub lo_closed: bool,
    pub hi_closed: bool,
}

/// A point of the sign chart: a zero of the numerator, a pole (a zero of a denominator, never
/// part of the solution), or both where a factor cancels.
#[derive(Clone, Debug)]
pub(crate) struct ChartPoint {
    pub point: Point,
    pub zero: bool,
    pub pole: bool,
}

/// One relation `left ⋚ right` as `P/Q ⋚ 0`, with `P` and `Q` scaled to whole coprime
/// coefficients by a positive factor: its critical points in order and the signs of `P` and
/// `Q` on the pieces between them (`signs.len() == points.len() + 1`).
#[derive(Clone, Debug)]
pub(crate) struct SignChart {
    pub relation: String,
    pub numerator: Vec<RBig>,
    pub denominator: Vec<RBig>,
    pub points: Vec<ChartPoint>,
    pub signs: Vec<(i8, i8)>,
}

/// The solution of a chain of relations in `variable`: the intersection of each relation's
/// solution, with the sign chart of each relation for the steps.
pub(crate) struct Solution {
    pub intervals: Vec<Interval>,
    pub charts: Vec<SignChart>,
}

/// Solves `operands[0] op_1 operands[1] op_2 …` for `variable`.
pub(crate) fn solve(evaluator: &Evaluator, operands: &[Node], relations: &[&str], variable: &str, digits: usize) -> Result<Solution> {
    let mut intervals: Option<Vec<Interval>> = None;
    let mut charts = Vec::new();
    for (pair, relation) in operands.windows(2).zip(relations) {
        let chart = sign_chart(evaluator, &pair[0], &pair[1], relation, variable, digits)?;
        let solved = solution_of(&chart);
        intervals = Some(match intervals {
            None => solved,
            Some(before) => intersect(&before, &solved),
        });
        charts.push(chart);
    }
    Ok(Solution { intervals: intervals.unwrap_or_default(), charts })
}

fn sign_chart(evaluator: &Evaluator, left: &Node, right: &Node, relation: &str, variable: &str, digits: usize) -> Result<SignChart> {
    let mut poles = Vec::new();
    let (ln, ld) = rational_function(evaluator, left, variable, &mut poles)?;
    let (rn, rd) = rational_function(evaluator, right, variable, &mut poles)?;
    // ln/ld - rn/rd = (ln·rd - rn·ld) / (ld·rd)
    let (mut numerator, denominator) = whole(trim(sub(&mul(&ln, &rd), &mul(&rn, &ld))), trim(mul(&ld, &rd)));
    // `-x < 0` reads better as `x > 0`: without a denominator, the leading coefficient positive.
    let mut relation = relation.to_string();
    if denominator.len() == 1 && numerator.last().is_some_and(|c| *c < RBig::ZERO) {
        numerator = scale(&numerator, &-RBig::ONE);
        relation = flipped(&relation).to_string();
    }

    let key = |value: &Value| -> Result<RBig> { Ok(value.decimal(digits + 20)?.to_rational()) };
    let mut points: Vec<ChartPoint> = Vec::new();
    let mut add = |value: Value, zero: bool| -> Result<()> {
        let key = key(&value)?;
        match points.iter_mut().find(|p| p.point.key == key) {
            Some(existing) => {
                existing.zero |= zero;
                existing.pole |= !zero;
            }
            None => points.push(ChartPoint { point: Point { value, key }, zero, pole: !zero }),
        }
        Ok(())
    };
    for pole in &poles {
        for root in polynomial::roots(pole, false, digits + 10)? {
            add(root, false)?;
        }
    }
    if !is_zero(&numerator) {
        for root in polynomial::roots(&numerator, false, digits + 10)? {
            add(root, true)?;
        }
    }
    points.sort_by(|a, b| a.point.key.cmp(&b.point.key));

    // A rational point inside each piece decides its sign.
    let keys: Vec<RBig> = points.iter().map(|p| p.point.key.clone()).collect();
    let mut samples = Vec::new();
    match (keys.first(), keys.last()) {
        (Some(first), Some(last)) => {
            samples.push(first - RBig::ONE);
            samples.extend(keys.windows(2).map(|w| (&w[0] + &w[1]) / RBig::from(2)));
            samples.push(last + RBig::ONE);
        }
        _ => samples.push(RBig::ZERO),
    }
    let signs = samples
        .iter()
        .map(|t| {
            (sign(&polynomial::evaluate(&numerator, t)), sign(&polynomial::evaluate(&denominator, t)))
        })
        .collect();
    Ok(SignChart { relation, numerator, denominator, points, signs })
}

/// `P/Q` with `P` and `Q` each scaled by a positive factor to whole, coprime coefficients; a
/// constant `Q` only gives its sign. Only the sign of `P/Q` matters, and it does not change.
fn whole(numerator: Coefficients, denominator: Coefficients) -> (Coefficients, Coefficients) {
    if denominator.len() == 1 {
        let numerator = if denominator[0] < RBig::ZERO { scale(&numerator, &-RBig::ONE) } else { numerator };
        return (primitive(numerator), vec![RBig::ONE]);
    }
    (primitive(numerator), primitive(denominator))
}

/// The coefficients times a positive factor that makes them whole and coprime.
fn primitive(c: Coefficients) -> Coefficients {
    if is_zero(&c) {
        return c;
    }
    let lcm = c.iter().fold(UBig::ONE, |l, x| {
        let d = x.denominator();
        let g = gcd_ubig(l.clone(), d.clone());
        &l / g * d
    });
    let c = scale(&c, &RBig::from(IBig::from(lcm)));
    let g = c.iter().filter(|x| **x != RBig::ZERO).fold(UBig::ZERO, |g, x| gcd_ubig(g, x.numerator().unsigned_abs()));
    scale(&c, &RBig::from_parts(IBig::ONE, g))
}

/// A polynomial in `x` as LaTeX, the highest power first: `3x^{2} - 4x + \frac{1}{2}`.
pub(crate) fn polynomial_latex(c: &[RBig], x: &str) -> String {
    let mut out = String::new();
    for k in (0..c.len()).rev().filter(|k| c[*k] != RBig::ZERO) {
        let negative = c[k] < RBig::ZERO;
        let magnitude = if negative { -c[k].clone() } else { c[k].clone() };
        let number = crate::algebra::domain::format_rbig_latex(&magnitude);
        let power = match k {
            0 => String::new(),
            1 => x.to_string(),
            _ => format!("{x}^{{{k}}}"),
        };
        let term = match (k, magnitude == RBig::ONE) {
            (0, _) => number,
            (_, true) => power,
            _ if number.starts_with("\\frac") => format!("{number} {power}"),
            _ => format!("{number}{power}"),
        };
        if out.is_empty() {
            out = if negative { format!("-{term}") } else { term };
        } else {
            out.push_str(if negative { " - " } else { " + " });
            out.push_str(&term);
        }
    }
    if out.is_empty() {
        out.push('0');
    }
    out
}

fn sign(r: &RBig) -> i8 {
    match r.cmp(&RBig::ZERO) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// The relation after both sides are multiplied by -1.
fn flipped(relation: &str) -> &str {
    match relation {
        "<" => ">",
        ">" => "<",
        "\\le" => "\\ge",
        "\\ge" => "\\le",
        other => other,
    }
}

/// Whether a sign satisfies the relation `left - right ⋚ 0`.
fn wanted(relation: &str, sign: i8) -> bool {
    match relation {
        "<" => sign < 0,
        ">" => sign > 0,
        "\\le" => sign <= 0,
        "\\ge" => sign >= 0,
        "\\neq" => sign != 0,
        _ => sign == 0,
    }
}

/// The intervals a sign chart gives, adjacent pieces and points joined.
fn solution_of(chart: &SignChart) -> Vec<Interval> {
    let region_in: Vec<bool> = chart.signs.iter().map(|(n, d)| wanted(&chart.relation, n * d)).collect();
    let point_in: Vec<bool> = chart.points.iter().map(|p| p.zero && !p.pole && wanted(&chart.relation, 0)).collect();
    let n = chart.points.len();
    let point = |k: usize| Some(chart.points[k].point.clone());
    let mut out = Vec::new();
    // The open interval being built: its lower end and whether that end is in it.
    let mut start: Option<(Option<Point>, bool)> = None;
    for k in 0..=n {
        let (lo, lo_in) = if k == 0 { (None, false) } else { (point(k - 1), point_in[k - 1]) };
        if region_in[k] {
            if start.is_none() {
                start = Some((lo, lo_in));
            }
            if k < n && !point_in[k] {
                let (s, closed) = start.take().expect("set above");
                out.push(Interval { lo: s, hi: point(k), lo_closed: closed, hi_closed: false });
            }
        } else if let Some((s, closed)) = start.take() {
            out.push(Interval { lo: s, hi: lo, lo_closed: closed, hi_closed: lo_in });
        } else if lo_in {
            out.push(Interval { lo: lo.clone(), hi: lo, lo_closed: true, hi_closed: true });
        }
    }
    if let Some((s, closed)) = start {
        out.push(Interval { lo: s, hi: None, lo_closed: closed, hi_closed: false });
    }
    out
}

/// The intervals in both sets.
fn intersect(a: &[Interval], b: &[Interval]) -> Vec<Interval> {
    let mut out = Vec::new();
    for x in a {
        for y in b {
            // The larger lower end and the smaller upper end; at equal ends both must be closed.
            let (lo, lo_closed) = match (&x.lo, &y.lo) {
                (None, None) => (None, false),
                (Some(p), None) => (Some(p.clone()), x.lo_closed),
                (None, Some(q)) => (Some(q.clone()), y.lo_closed),
                (Some(p), Some(q)) if p.key == q.key => (Some(p.clone()), x.lo_closed && y.lo_closed),
                (Some(p), Some(q)) if p.key > q.key => (Some(p.clone()), x.lo_closed),
                (_, Some(q)) => (Some(q.clone()), y.lo_closed),
            };
            let (hi, hi_closed) = match (&x.hi, &y.hi) {
                (None, None) => (None, false),
                (Some(p), None) => (Some(p.clone()), x.hi_closed),
                (None, Some(q)) => (Some(q.clone()), y.hi_closed),
                (Some(p), Some(q)) if p.key == q.key => (Some(p.clone()), x.hi_closed && y.hi_closed),
                (Some(p), Some(q)) if p.key < q.key => (Some(p.clone()), x.hi_closed),
                (_, Some(q)) => (Some(q.clone()), y.hi_closed),
            };
            let nonempty = match (&lo, &hi) {
                (Some(l), Some(h)) => l.key < h.key || (l.key == h.key && lo_closed && hi_closed),
                _ => true,
            };
            if nonempty {
                out.push(Interval { lo, hi, lo_closed, hi_closed });
            }
        }
    }
    out
}

fn unsupported() -> QuantusError {
    QuantusError::other(
        msg!("core.eval.inequality.only_rational"),
    )
}

/// `node` as a fraction of polynomials in `variable`. Every denominator that depends on the
/// variable is pushed to `poles`, whose zeros are no part of the solution.
pub(crate) fn rational_function(evaluator: &Evaluator, node: &Node, variable: &str, poles: &mut Vec<Coefficients>) -> Result<(Coefficients, Coefficients)> {
    if !depends_on(node, variable) {
        let quantity = evaluator.evaluate(node)?.simplify()?;
        if !quantity.unit.is_one() {
            return Err(QuantusError::other(msg!("core.eval.inequality.units_unsupported")));
        }
        let value = quantity.value.as_rational().cloned().or_else(|| {
            // A value like 2^{-1} that is rational but not folded.
            quantity.value.decimal(60).ok().map(|d| d.to_rational()).filter(|r| Value::Rat(r.clone()) == quantity.value)
        });
        return value.map(|r| (vec![r], vec![RBig::ONE])).ok_or_else(unsupported);
    }
    let fraction = |n: Coefficients, d: Coefficients| (trim(n), trim(d));
    Ok(match node {
        Node::Variable { name, index, .. } if index.is_empty() && name == variable => (vec![RBig::ZERO, RBig::ONE], vec![RBig::ONE]),
        Node::Group { inner, .. } => rational_function(evaluator, inner, variable, poles)?,
        Node::Negation { operand, .. } => {
            let (n, d) = rational_function(evaluator, operand, variable, poles)?;
            (scale(&n, &-RBig::ONE), d)
        }
        Node::Binary { operator, left, right, .. } => {
            let (a, b) = rational_function(evaluator, left, variable, poles)?;
            let (c, d) = rational_function(evaluator, right, variable, poles)?;
            match operator {
                Operator::Plus => fraction(add(&mul(&a, &d), &mul(&c, &b)), mul(&b, &d)),
                Operator::Minus => fraction(sub(&mul(&a, &d), &mul(&c, &b)), mul(&b, &d)),
                Operator::Times | Operator::ImplicitTimes => fraction(mul(&a, &c), mul(&b, &d)),
                Operator::Divide => divide(a, b, c, d, poles)?,
            }
        }
        Node::Fraction { numerator, denominator, .. } => {
            let (a, b) = rational_function(evaluator, numerator, variable, poles)?;
            let (c, d) = rational_function(evaluator, denominator, variable, poles)?;
            divide(a, b, c, d, poles)?
        }
        Node::Power { base, exponent, .. } if !depends_on(exponent, variable) => {
            let exponent = evaluator.evaluate(exponent)?.value;
            let k = exponent.as_rational().filter(|r| r.denominator().is_one()).map(|r| r.numerator().clone()).ok_or_else(unsupported)?;
            let k = i64::try_from(k).map_err(|_| unsupported())?;
            if k.unsigned_abs() > 64 {
                return Err(unsupported());
            }
            let (n, d) = rational_function(evaluator, base, variable, poles)?;
            let (mut pn, mut pd) = (vec![RBig::ONE], vec![RBig::ONE]);
            for _ in 0..k.unsigned_abs() {
                pn = mul(&pn, &n);
                pd = mul(&pd, &d);
            }
            if k < 0 {
                poles.push(n.clone());
                (pd, pn)
            } else {
                (pn, pd)
            }
        }
        _ => return Err(unsupported()),
    })
}

/// `(a/b) / (c/d)`, with the zeros of `c` as poles.
fn divide(a: Coefficients, b: Coefficients, c: Coefficients, d: Coefficients, poles: &mut Vec<Coefficients>) -> Result<(Coefficients, Coefficients)> {
    if is_zero(&c) {
        return Err(QuantusError::other(msg!("core.common.division_zero")));
    }
    if c.len() > 1 {
        poles.push(c.clone());
    }
    Ok((trim(mul(&a, &d)), trim(mul(&b, &c))))
}

fn trim(mut c: Coefficients) -> Coefficients {
    while c.len() > 1 && c.last().is_some_and(|x| *x == RBig::ZERO) {
        c.pop();
    }
    if c.is_empty() {
        c.push(RBig::ZERO);
    }
    c
}

fn is_zero(c: &[RBig]) -> bool {
    c.iter().all(|x| *x == RBig::ZERO)
}

fn add(a: &[RBig], b: &[RBig]) -> Coefficients {
    (0..a.len().max(b.len())).map(|k| a.get(k).cloned().unwrap_or(RBig::ZERO) + b.get(k).cloned().unwrap_or(RBig::ZERO)).collect()
}

fn sub(a: &[RBig], b: &[RBig]) -> Coefficients {
    add(a, &scale(b, &-RBig::ONE))
}

fn scale(a: &[RBig], factor: &RBig) -> Coefficients {
    a.iter().map(|x| x * factor).collect()
}

fn mul(a: &[RBig], b: &[RBig]) -> Coefficients {
    let mut out = vec![RBig::ZERO; a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            out[i + j] += x * y;
        }
    }
    out
}
