//! Equations with one unknown: `x^2 - 5x + 6 = 0`.
//!
//! The unknown is the single variable without a subscript that has no value; it is a plain
//! number, so an equation whose sides differ in dimension is an error as anywhere else.
//!
//! Polynomials up to degree two are solved exactly. The coefficients are read off by
//! evaluating at a few points and checked at others, which also covers `\pi x = 3` and
//! `\sqrt{2} x^2 = 4`. Anything else is solved numerically: the equation is scanned for sign
//! changes on [-100, 100] and each is refined by bisection to the working precision.

use dashu::base::Abs;
use dashu::integer::{IBig, UBig};
use dashu::rational::RBig;

use super::evaluator::Evaluator;
use crate::latex::Node;
use crate::num::Value;
use crate::quantity::PhysicalQuantity;
use crate::{QuantusError, Result};

const SCAN_LIMIT: i64 = 100;
const SCAN_STEPS_PER_UNIT: i64 = 4;
const MAX_ROOTS: usize = 6;

/// The sign of the residual at a point (-1, 0, 1) and its magnitude.
type Sample = (i32, RBig);

/// The unknown and its solutions, smallest first.
pub struct Solutions {
    pub variable: String,
    pub values: Vec<PhysicalQuantity>,
}

impl Evaluator<'_> {
    pub fn solve(&self, equation: &Node) -> Result<Solutions> {
        let Node::Equation { left, right, .. } = equation else {
            return Err(QuantusError::other("Only an equation can be solved."));
        };
        let variable = self.unknown(equation)?;

        let mut scope = self.variables.clone();
        let mut residual = |x: Value| -> Result<PhysicalQuantity> {
            scope.insert(variable.clone(), PhysicalQuantity::dimensionless(x));
            let evaluator = self.scoped_over(&scope);
            evaluator.evaluate(left)?.sub(&evaluator.evaluate(right)?)
        };

        let roots = match polynomial_roots(&mut residual)? {
            Some(roots) => roots,
            None => self.numeric_roots(&mut residual, &variable)?,
        };
        Ok(Solutions { variable, values: roots.into_iter().map(PhysicalQuantity::dimensionless).collect() })
    }

    /// The one free, unindexed variable of the equation.
    fn unknown(&self, equation: &Node) -> Result<String> {
        let mut found = Vec::new();
        collect_unknowns(equation, &mut Vec::new(), self, &mut found);
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

        let mut roots: Vec<RBig> = Vec::new();
        let mut previous: Option<(RBig, i32, RBig)> = None;
        for i in -SCAN_LIMIT * SCAN_STEPS_PER_UNIT..=SCAN_LIMIT * SCAN_STEPS_PER_UNIT {
            let x = RBig::from_parts(IBig::from(i), UBig::from(SCAN_STEPS_PER_UNIT as u64));
            let Some((sign, magnitude)) = sample(&x)? else {
                previous = None;
                continue;
            };
            if sign == 0 {
                roots.push(x.clone());
            } else if let Some((lo, lo_sign, lo_magnitude)) = &previous {
                if *lo_sign != 0 && *lo_sign != sign {
                    let ceiling = if *lo_magnitude < magnitude { lo_magnitude.clone() } else { magnitude.clone() };
                    if let Some(root) = bisect(&mut sample, lo.clone(), x.clone(), *lo_sign, &ceiling, &tolerance)? {
                        roots.push(root);
                    }
                }
            }
            previous = Some((x, sign, magnitude));
        }

        if roots.is_empty() {
            return Err(QuantusError::other(format!(
                "No solution for {variable} found between -{SCAN_LIMIT} and {SCAN_LIMIT}."
            )));
        }
        roots.sort_by_key(|root| root.clone().abs());
        roots.dedup();
        roots.truncate(MAX_ROOTS);
        roots.sort();
        roots
            .into_iter()
            .map(|root| Ok(Value::from_decimal(&Value::Rat(root).decimal(self.precision)?)))
            .collect()
    }
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

/// Exact roots when the residual is a polynomial of degree at most two, `None` otherwise.
fn polynomial_roots(residual: &mut dyn FnMut(Value) -> Result<PhysicalQuantity>) -> Result<Option<Vec<Value>>> {
    let mut at = |x: i64| -> Result<Option<Value>> {
        match residual(Value::int(x)) {
            Ok(q) => Ok(Some(q.value)),
            Err(e @ (QuantusError::DimensionMismatch(_) | QuantusError::UnresolvedVariable(_))) => Err(e),
            Err(_) => Ok(None),
        }
    };
    let (Some(f0), Some(f1), Some(f2)) = (at(0)?, at(1)?, at(2)?) else { return Ok(None) };

    let c = f0.clone();
    let a = f2.sub(&f1.mul(&Value::int(2))).add(&f0).div(&Value::int(2))?;
    let b = f1.sub(&f0).sub(&a);

    // A quadratic through three points must also hit these.
    let quadratic = |x: i64| a.mul(&Value::int(x * x)).add(&b.mul(&Value::int(x))).add(&c);
    for x in [-1, 3, 5] {
        match at(x)? {
            Some(value) if value == quadratic(x) => {}
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

/// Variables without index and without value, outside whatever binds them.
fn collect_unknowns(node: &Node, bound: &mut Vec<String>, evaluator: &Evaluator, found: &mut Vec<String>) {
    match node {
        Node::Variable { name, index, .. } if index.is_empty() => {
            if !bound.contains(name) && !evaluator.variables.contains_key(name) && !found.contains(name) {
                found.push(name.clone());
            }
        }
        Node::Sum { index: name, .. } | Node::Integral { variable: name, .. } | Node::Limit { variable: name, .. }
        | Node::Derivative { variable: name, .. } => {
            bound.push(name.clone());
            for child in node.children() {
                collect_unknowns(child, bound, evaluator, found);
            }
            bound.pop();
        }
        other => {
            for child in other.children() {
                collect_unknowns(child, bound, evaluator, found);
            }
        }
    }
}
