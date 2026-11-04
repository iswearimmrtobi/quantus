//! Equations with one unknown: `x^2 - 5x + 6 = 0`.
//!
//! The unknown is the single variable without a subscript that has no value; it is a plain
//! number, so an equation whose sides differ in dimension is an error as anywhere else.
//!
//! Polynomials up to degree two are solved exactly. The coefficients are read off by
//! evaluating at a few points and checked at others, which also covers `\pi x = 3` and
//! `\sqrt{2} x^2 = 4`. Polynomials of higher degree with rational coefficients go to
//! [`super::polynomial`]: exact up to degree four, and numerically beyond that, on the whole
//! real line. Anything else is solved numerically: the equation is scanned on [-100, 100], each
//! sign change is refined by bisection and each interior minimum of the residual's magnitude by
//! a shrinking-interval search, which finds roots the residual only touches (`|x - 2| = 0`). A
//! point where the equation is undefined (`1/x` at 0) is sampled just beside it, so a root next
//! to it is still bracketed.

use dashu::base::Abs;
use dashu::integer::{IBig, UBig};
use dashu::rational::RBig;

use super::evaluator::Evaluator;
use super::polynomial;
use crate::latex::Node;
use crate::num::Value;
use crate::quantity::PhysicalQuantity;
use crate::units::Unit;
use crate::{QuantusError, Result};

const SCAN_LIMIT: i64 = 100;
const SCAN_STEPS_PER_UNIT: i64 = 4;
const MAX_ROOTS: usize = 6;

/// The sign of the residual at a point (-1, 0, 1) and its magnitude.
pub(crate) type Sample = (i32, RBig);

/// The unknown and its solutions, smallest first.
pub struct Solutions {
    pub variable: String,
    pub values: Vec<PhysicalQuantity>,
}

impl Evaluator<'_> {
    pub fn solve(&self, equation: &Node) -> Result<Solutions> {
        self.solve_for(equation, None, None)
    }

    /// Solves for `target`, or for the one unknown when it is `None`. Every other letter must
    /// have a value. Without an exact method, a `start` value runs Newton's method from there
    /// instead of the scan.
    pub fn solve_for(&self, equation: &Node, target: Option<&str>, start: Option<&Value>) -> Result<Solutions> {
        let Node::Equation { left, right, .. } = equation else {
            return Err(QuantusError::other("Only an equation can be solved."));
        };
        let variable = match target {
            None => self.unknown(equation)?,
            Some(target) => {
                let mut found = Vec::new();
                collect_unknowns(equation, true, &mut Vec::new(), self, &mut found);
                let others: Vec<&String> = found.iter().filter(|key| key.as_str() != target).collect();
                if !others.is_empty() {
                    let names: Vec<&str> = others.iter().map(|key| key.as_str()).collect();
                    return Err(QuantusError::other(format!("Give a value to {} first.", names.join(", "))));
                }
                target.to_string()
            }
        };

        // A letter solved for by name may carry a unit (F = m a for a): the one that makes the
        // two sides agree.
        let unit = if target.is_some() { self.unit_of_unknown(&variable, left, right) } else { Unit::ONE };
        let mut scope = self.variables.clone();
        let mut residual = |x: Value| -> Result<PhysicalQuantity> {
            scope.insert(variable.clone(), PhysicalQuantity::new(x, unit.clone()));
            let evaluator = self.scoped_over(&scope);
            evaluator.evaluate(left)?.sub(&evaluator.evaluate(right)?)
        };

        let roots = match polynomial_roots(&mut residual, self.complex())? {
            Some(roots) => roots,
            None => match higher_polynomial(&mut residual)? {
                Some(coefficients) => {
                    let roots = polynomial::roots(&coefficients, self.complex(), self.precision)?;
                    if roots.is_empty() {
                        return Err(QuantusError::other("This equation has no real solution."));
                    }
                    roots
                }
                None => match start {
                    Some(start) => vec![self.newton(&mut residual, &variable, start)?],
                    None => self.numeric_roots(&mut residual, &variable)?,
                },
            },
        };
        Ok(Solutions { variable, values: roots.into_iter().map(|root| PhysicalQuantity::new(root, unit.clone())).collect() })
    }

    /// The unit that makes both sides of the equation compatible when `variable` carries it:
    /// none, or the quotient of the sides' units either way round.
    fn unit_of_unknown(&self, variable: &str, left: &Node, right: &Node) -> Unit {
        let sides = |unit: &Unit| -> Option<(PhysicalQuantity, PhysicalQuantity)> {
            let mut scope = self.variables.clone();
            scope.insert(variable.to_string(), PhysicalQuantity::new(Value::one(), unit.clone()));
            let evaluator = self.scoped_over(&scope);
            Some((evaluator.evaluate(left).ok()?, evaluator.evaluate(right).ok()?))
        };
        let Some((l, r)) = sides(&Unit::ONE) else { return Unit::ONE };
        if l.unit.is_compatible(&r.unit) {
            return Unit::ONE;
        }
        for candidate in [l.unit.divide(&r.unit), r.unit.divide(&l.unit)] {
            if let Some((l, r)) = sides(&candidate) {
                if l.unit.is_compatible(&r.unit) {
                    return candidate;
                }
            }
        }
        Unit::ONE
    }

    /// Newton's method from `start`, with the slope from a central difference.
    fn newton(&self, residual: &mut dyn FnMut(Value) -> Result<PhysicalQuantity>, variable: &str, start: &Value) -> Result<Value> {
        let digits = self.precision + 10;
        let number = |q: PhysicalQuantity| -> Result<RBig> { Ok(q.value.decimal(digits)?.to_rational()) };
        let mut x = start.decimal(digits)?.to_rational();
        let h = RBig::from_parts(IBig::ONE, UBig::from(10u8).pow(digits / 2 + 2));
        let tolerance = RBig::from_parts(IBig::ONE, UBig::from(10u8).pow(self.precision + 3));
        let failed = || QuantusError::other(format!("Newton's method found no solution for {variable} from this start value."));
        for _ in 0..200 {
            let f = number(residual(Value::Rat(x.clone()))?)?;
            if f == RBig::ZERO {
                return Ok(Value::from_decimal(&Value::Rat(x).decimal(self.precision)?));
            }
            let up = number(residual(Value::Rat(&x + &h))?)?;
            let down = number(residual(Value::Rat(&x - &h))?)?;
            let slope = (up - down) / (RBig::from(2) * &h);
            if slope == RBig::ZERO {
                return Err(failed());
            }
            let step = &f / &slope;
            x = Value::Rat(&x - &step).decimal(digits)?.to_rational();
            if step.abs() < tolerance {
                return Ok(Value::from_decimal(&Value::Rat(x).decimal(self.precision)?));
            }
        }
        Err(failed())
    }

    /// The one free, unindexed variable of the equation.
    fn unknown(&self, equation: &Node) -> Result<String> {
        let mut found = Vec::new();
        collect_unknowns(equation, false, &mut Vec::new(), self, &mut found);
        match found.as_slice() {
            [] => Err(QuantusError::other(
                "The equation has no unknown to solve for. Use a letter such as x without a value.",
            )),
            [one] => Ok(one.clone()),
            many => Err(QuantusError::other(format!(
                "The equation has more than one unknown ({}); only one can be solved for.",
                many.join(", ")
            ))),
        }
    }

    fn numeric_roots(
        &self,
        residual: &mut dyn FnMut(Value) -> Result<PhysicalQuantity>,
        variable: &str,
    ) -> Result<Vec<Value>> {
        let mut sample = |x: &RBig| -> Result<Option<Sample>> {
            match residual(Value::Rat(x.clone())) {
                Ok(q) => match q.value.decimal(40) {
                    Ok(d) => Ok(Some((if d.is_zero() { 0 } else if d.is_negative() { -1 } else { 1 }, d.to_rational().abs()))),
                    Err(_) => Ok(None), // not a real number here
                },
                Err(e @ (QuantusError::DimensionMismatch(_) | QuantusError::UnresolvedVariable(_))) => Err(e),
                Err(_) => Ok(None), // outside the domain, e.g. ln of a negative number
            }
        };
        let tolerance = RBig::from_parts(IBig::ONE, UBig::from(10u8).pow(self.precision + 5));
        let accept = RBig::from_parts(IBig::ONE, UBig::from(10u8).pow(self.precision));
        let mut roots = scan_roots(
            &mut sample,
            &RBig::from(-SCAN_LIMIT),
            &RBig::from(SCAN_LIMIT),
            SCAN_STEPS_PER_UNIT as u64,
            &tolerance,
            &accept,
        )?;

        if roots.is_empty() {
            return Err(QuantusError::other(format!(
                "No solution for {variable} found between -{SCAN_LIMIT} and {SCAN_LIMIT}."
            )));
        }
        roots.sort_by_key(|root| root.clone().abs());
        roots.truncate(MAX_ROOTS);
        roots.sort();
        roots
            .into_iter()
            .map(|root| Ok(Value::from_decimal(&Value::Rat(root).decimal(self.precision)?)))
            .collect()
    }
}

/// The roots of a residual on `[lo, hi]`, smallest first and without duplicates; empty when there
/// are none.
///
/// The residual is scanned in steps of `1 / steps_per_unit`. Each sign change is refined by
/// bisection to `tolerance`, and each interior minimum of its magnitude by a shrinking-interval
/// search, accepted as a root the residual only touches when its magnitude is at most `accept`
/// (only the [`MAX_ROOTS`] smallest minima are refined). A point where the residual is undefined
/// is sampled just beside it, so a root next to it is still bracketed.
pub(crate) fn scan_roots(
    sample: &mut dyn FnMut(&RBig) -> Result<Option<Sample>>,
    lo: &RBig,
    hi: &RBig,
    steps_per_unit: u64,
    tolerance: &RBig,
    accept: &RBig,
) -> Result<Vec<RBig>> {
    let mut roots: Vec<RBig> = Vec::new();
    let mut previous: Option<(RBig, i32, RBig)> = None;
    // The last three samples without a gap, for spotting minima of |f|.
    let mut window: Vec<(RBig, i32, RBig)> = Vec::new();
    let mut minima: Vec<(RBig, RBig, RBig)> = Vec::new();
    let step = RBig::from_parts(IBig::ONE, UBig::from(steps_per_unit));
    let nudge = &step / RBig::from(1024);
    let first = (lo * RBig::from(steps_per_unit)).ceil();
    let last = (hi * RBig::from(steps_per_unit)).floor();
    let mut i = first;
    while i <= last {
        let x = RBig::from_parts(i.clone(), UBig::from(steps_per_unit));
        i += IBig::ONE;
        let points = match sample(&x)? {
            Some(s) => vec![(x.clone(), s)],
            None => {
                // Undefined here: look just left and just right of the point instead.
                window.clear();
                let mut beside = Vec::new();
                for p in [&x - &nudge, &x + &nudge] {
                    if let Some(s) = sample(&p)? {
                        beside.push((p, s));
                    }
                }
                if beside.is_empty() {
                    previous = None;
                }
                beside
            }
        };
        for (x, (sign, magnitude)) in points {
            if sign == 0 {
                roots.push(x.clone());
            } else if let Some((lo, lo_sign, lo_magnitude)) = &previous {
                if *lo_sign != 0 && *lo_sign != sign {
                    let ceiling = if *lo_magnitude < magnitude { lo_magnitude.clone() } else { magnitude.clone() };
                    if let Some(root) = bisect(sample, lo.clone(), x.clone(), *lo_sign, &ceiling, tolerance)? {
                        roots.push(root);
                    }
                }
            }
            window.push((x.clone(), sign, magnitude.clone()));
            if window.len() > 3 {
                window.remove(0);
            }
            if let [(a, sa, ma), (_, sb, mb), (c, sc, mc)] = window.as_slice() {
                if *sa == *sb && *sb == *sc && *sb != 0 && mb < ma && mb < mc {
                    minima.push((mb.clone(), a.clone(), c.clone()));
                }
            }
            previous = Some((x, sign, magnitude));
        }
    }

    // Roots the residual only touches; the smallest minima first, and only a few of them.
    minima.sort_by(|x, y| x.0.cmp(&y.0));
    for (_, lo, hi) in minima.into_iter().take(MAX_ROOTS) {
        if let Some(root) = touch(sample, lo, hi, tolerance, accept)? {
            roots.push(root);
        }
    }
    roots.sort();
    roots.dedup();
    Ok(roots)
}

/// Narrows a sign change in `[lo, hi]` down to `tolerance`. A sign change across a pole
/// (`1/x` at 0) is no root: there the value does not shrink.
fn bisect(
    sample: &mut dyn FnMut(&RBig) -> Result<Option<Sample>>,
    mut lo: RBig,
    mut hi: RBig,
    lo_sign: i32,
    ceiling: &RBig,
    tolerance: &RBig,
) -> Result<Option<RBig>> {
    let two = RBig::from(2);
    while (hi.clone() - &lo) > *tolerance {
        let mid = (lo.clone() + &hi) / &two;
        let Some((sign, _)) = sample(&mid)? else { return Ok(None) };
        if sign == 0 {
            return Ok(Some(mid));
        }
        if sign == lo_sign {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let root = (lo + &hi) / &two;
    let Some((_, magnitude)) = sample(&root)? else { return Ok(None) };
    Ok((magnitude * RBig::from(100_000) <= *ceiling).then_some(root))
}

/// Narrows an interior minimum of |f| in `[lo, hi]` and returns it when |f| vanishes there.
/// Each step keeps 5/8 of the interval, on the side of the smaller of two probes; all points stay
/// dyadic, so the rationals do not grow.
fn touch(
    sample: &mut dyn FnMut(&RBig) -> Result<Option<Sample>>,
    mut lo: RBig,
    mut hi: RBig,
    tolerance: &RBig,
    accept: &RBig,
) -> Result<Option<RBig>> {
    let two = RBig::from(2);
    let eight = RBig::from(8);
    while (hi.clone() - &lo) > *tolerance {
        let mid = (lo.clone() + &hi) / &two;
        let w = (hi.clone() - &lo) / &eight;
        let (left, right) = (&mid - &w, &mid + &w);
        let (Some((sl, ml)), Some((sr, mr))) = (sample(&left)?, sample(&right)?) else { return Ok(None) };
        if sl == 0 {
            return Ok(Some(left));
        }
        if sr == 0 {
            return Ok(Some(right));
        }
        if ml < mr {
            hi = right;
        } else {
            lo = left;
        }
    }
    let root = (lo + &hi) / &two;
    let Some((_, magnitude)) = sample(&root)? else { return Ok(None) };
    Ok((magnitude <= *accept).then_some(root))
}

/// The coefficients when the residual is a polynomial of degree three to
/// [`polynomial::MAX_DEGREE`] with rational coefficients.
fn higher_polynomial(residual: &mut dyn FnMut(Value) -> Result<PhysicalQuantity>) -> Result<Option<Vec<RBig>>> {
    let mut at = |x: &RBig| -> Result<Option<RBig>> {
        match residual(Value::Rat(x.clone())) {
            Ok(q) => Ok(q.value.as_rational().cloned()),
            Err(e @ (QuantusError::DimensionMismatch(_) | QuantusError::UnresolvedVariable(_))) => Err(e),
            Err(_) => Ok(None),
        }
    };
    Ok(polynomial::fit(&mut at)?.filter(|coefficients| coefficients.len() > 3))
}

/// Exact roots when the residual is a polynomial of degree at most two, `None` otherwise.
fn polynomial_roots(
    residual: &mut dyn FnMut(Value) -> Result<PhysicalQuantity>,
    complex: bool,
) -> Result<Option<Vec<Value>>> {
    let mut at = |x: Value| -> Result<Option<Value>> {
        match residual(x) {
            Ok(q) => Ok(Some(q.value)),
            Err(e @ (QuantusError::DimensionMismatch(_) | QuantusError::UnresolvedVariable(_))) => Err(e),
            Err(_) => Ok(None),
        }
    };
    let (Some(f0), Some(f1), Some(f2)) = (at(Value::int(0))?, at(Value::int(1))?, at(Value::int(2))?) else {
        return Ok(None);
    };

    let c = f0.clone();
    let a = f2.sub(&f1.mul(&Value::int(2))).add(&f0).div(&Value::int(2))?;
    let b = f1.sub(&f0).sub(&a);

    // A quadratic through three points must also hit these. Points far out and between the
    // integers catch residuals that only look quadratic near 0, like |x - 9| - 3.
    let quadratic = |x: &Value| a.mul(&x.mul(x)).add(&b.mul(x)).add(&c);
    for x in [Value::int(-1), Value::int(3), Value::int(5), Value::ratio(7, 3), Value::ratio(-11, 2), Value::int(1000), Value::int(-1000)] {
        match at(x.clone())? {
            Some(value) if value == quadratic(&x) => {}
            _ => return Ok(None),
        }
    }

    if a.is_zero() {
        if b.is_zero() {
            return Err(QuantusError::other(if c.is_zero() {
                "Every number solves this equation."
            } else {
                "This equation has no solution."
            }));
        }
        return Ok(Some(vec![c.neg().div(&b)?]));
    }

    let discriminant = b.mul(&b).sub(&a.mul(&c).mul(&Value::int(4)));
    if discriminant.is_zero() {
        return Ok(Some(vec![b.neg().div(&a.mul(&Value::int(2)))?]));
    }
    if discriminant.is_negative()? {
        if complex {
            // (-b ± i√-D) / 2a, the one below the real axis first.
            let two_a = a.mul(&Value::int(2));
            let re = b.neg().div(&two_a)?;
            let im = discriminant.neg().pow(&Value::ratio(1, 2))?.div(&two_a)?;
            let im = if im.is_negative()? { im.neg() } else { im };
            return Ok(Some(vec![re.sub(&im.mul(&Value::I)), re.add(&im.mul(&Value::I))]));
        }
        return Err(QuantusError::other("This equation has no real solution."));
    }
    let root = discriminant.pow(&Value::ratio(1, 2))?;
    let two_a = a.mul(&Value::int(2));
    let mut roots = vec![b.neg().sub(&root).div(&two_a)?, b.neg().add(&root).div(&two_a)?];
    let mut keyed = Vec::new();
    for value in roots.drain(..) {
        keyed.push((value.decimal(40)?.to_rational(), value));
    }
    keyed.sort_by(|x, y| x.0.cmp(&y.0));
    Ok(Some(keyed.into_iter().map(|(_, value)| value).collect()))
}

/// Variables without value and without index, outside whatever binds them, by session key. In a
/// system (`system`) a variable with an index counts too: `x_1` is an unknown there, while a
/// single equation leaves `v_1` and `t_1` to the physics.
pub(crate) fn collect_unknowns(
    node: &Node,
    system: bool,
    bound: &mut Vec<String>,
    evaluator: &Evaluator,
    found: &mut Vec<String>,
) {
    match node {
        Node::Variable { name, index, .. } if system || index.is_empty() => {
            let key = Node::variable_key(name, index);
            if !bound.contains(&key)
                && !evaluator.variables.contains_key(&key)
                && !found.contains(&key)
                && evaluator.physical_constant(&key).is_none()
            {
                found.push(key);
            }
        }
        Node::Sum { index: name, .. } | Node::Integral { variable: name, .. } | Node::Limit { variable: name, .. }
        | Node::Derivative { variable: name, .. } => {
            bound.push(name.clone());
            for child in node.children() {
                collect_unknowns(child, system, bound, evaluator, found);
            }
            bound.pop();
        }
        other => {
            for child in other.children() {
                collect_unknowns(child, system, bound, evaluator, found);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(f: impl Fn(&RBig) -> Option<RBig>, lo: i64, hi: i64) -> Vec<RBig> {
        let mut sample = |x: &RBig| -> Result<Option<Sample>> {
            Ok(f(x).map(|y| {
                let sign = if y == RBig::ZERO { 0 } else if y < RBig::ZERO { -1 } else { 1 };
                (sign, y.abs())
            }))
        };
        let tolerance = RBig::from_parts(IBig::ONE, UBig::from(10u8).pow(20));
        let accept = RBig::from_parts(IBig::ONE, UBig::from(10u8).pow(15));
        scan_roots(&mut sample, &RBig::from(lo), &RBig::from(hi), 4, &tolerance, &accept).unwrap()
    }

    #[test]
    fn scan_roots_finds_crossings_and_touches_in_order() {
        let near = |root: &RBig, value: f64| (root.to_f64().value() - value).abs() < 1e-12;
        // (x - 1/3)(x + 2): two sign changes, neither on the grid.
        let roots = scan(|x| Some((x - RBig::from_parts(IBig::ONE, UBig::from(3u8))) * (x + RBig::from(2))), -5, 5);
        assert_eq!(roots.len(), 2);
        assert!(near(&roots[0], -2.0) && near(&roots[1], 1.0 / 3.0));
        // (x - 0.3)^2 only touches zero.
        let touched = scan(|x| Some((x - RBig::from_parts(IBig::from(3), UBig::from(10u8))).pow(2)), -5, 5);
        assert_eq!(touched.len(), 1);
        assert!(near(&touched[0], 0.3));
    }

    #[test]
    fn scan_roots_is_empty_without_roots_and_skips_poles() {
        assert!(scan(|x| Some(x * x + RBig::ONE), -5, 5).is_empty());
        // 1/x changes sign at its pole, which is no root.
        assert!(scan(|x| (*x != RBig::ZERO).then(|| RBig::ONE / x), -5, 5).is_empty());
        // Only the interval asked for is scanned.
        assert!(scan(|x| Some(x - RBig::from(10)), -5, 5).is_empty());
    }
}
