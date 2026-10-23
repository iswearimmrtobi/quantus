//! Exact results with roots in LaTeX: `2\sqrt{3}` or `\frac{\sqrt{2}}{2}` instead of a decimal.
//!
//! Only values built from rationals and rational powers of rationals qualify. Anything else
//! (`\pi`, `e`, functions, the imaginary unit) has no such form here, and the caller falls back
//! to the decimal.

use std::collections::BTreeMap;

use dashu::base::{Abs, Sign, UnsignedAbs};
use dashu::integer::{IBig, UBig};
use dashu::rational::RBig;

use crate::num::Value;

/// Radicands are only searched for repeated factors up to here; a larger cofactor stays under the root.
const TRIAL_DIVISION_LIMIT: u32 = 10_000;
/// A power bigger than this is left to the decimal rather than expanded into a huge radicand.
const MAX_EXPONENT: u32 = 64;

/// A radical part: the `index`-th root of an integer radicand, without a repeated factor
/// that is a full `index`-th power (as far as it was searched for).
type Radical = (u32, UBig);

/// A sum of rational multiples of products of radicals, collected by that product.
type Terms = BTreeMap<Vec<Radical>, RBig>;

/// The LaTeX of `value` if it contains a root and has an exact form; `None` otherwise, including
/// for plain rationals, which read better as the decimal.
pub fn format(value: &Value) -> Option<String> {
    let mut terms = Terms::new();
    match value {
        Value::Add(items) => items.iter().try_for_each(|item| add_term(&mut terms, item))?,
        other => add_term(&mut terms, other)?,
    }
    terms.retain(|_, coefficient| *coefficient != RBig::ZERO);
    if !terms.keys().any(|radicals| !radicals.is_empty()) {
        return None;
    }
    Some(sum(&terms))
}

/// Whether the sum needs parentheses before a unit is attached to it.
pub fn is_sum(latex: &str) -> bool {
    let body = latex.strip_prefix('-').unwrap_or(latex);
    body.contains(" + ") || body.contains(" - ")
}

fn add_term(terms: &mut Terms, term: &Value) -> Option<()> {
    let (coefficient, radicals) = split(term)?;
    *terms.entry(radicals).or_insert(RBig::ZERO) += coefficient;
    Some(())
}

/// One product of a rational and powers of rationals, as coefficient and radicals.
fn split(term: &Value) -> Option<(RBig, Vec<Radical>)> {
    let factors = match term {
        Value::Mul(factors) => factors.as_slice(),
        other => std::slice::from_ref(other),
    };
    let mut coefficient = RBig::ONE;
    let mut by_index: BTreeMap<u32, UBig> = BTreeMap::new();
    for factor in factors {
        match factor {
            Value::Rat(r) => coefficient *= r,
            Value::Pow(base, exponent) => {
                let (Value::Rat(base), Value::Rat(exponent)) = (&**base, &**exponent) else { return None };
                let (c, index, radicand) = power(base, exponent)?;
                coefficient *= c;
                let entry = by_index.entry(index).or_insert_with(|| UBig::ONE);
                *entry *= radicand;
            }
            _ => return None,
        }
    }
    let mut radicals = Vec::new();
    for (index, radicand) in by_index {
        let (outside, inside) = extract(index, radicand);
        coefficient *= RBig::from(outside);
        if inside != UBig::ONE {
            radicals.push((index, inside));
        }
    }
    Some((coefficient, radicals))
}

/// `base^exponent` as `coefficient · radicand^(1/index)`, with the denominator of the base
/// rationalised: `(1/2)^(1/2)` is `1/2 · 2^(1/2)`.
fn power(base: &RBig, exponent: &RBig) -> Option<(RBig, u32, UBig)> {
    let index = u32::try_from(exponent.denominator().clone()).ok()?;
    let mut count = i64::try_from(exponent.numerator().clone()).ok()?;
    if index < 2 || count.unsigned_abs() > u64::from(MAX_EXPONENT) {
        return None;
    }
    let mut base = base.clone();
    if count < 0 {
        base = RBig::from_parts(IBig::from(base.denominator().clone()) * base.numerator().sign(), base.numerator().clone().unsigned_abs());
        count = -count;
    }
    let negative = base.numerator().sign() == Sign::Negative;
    if negative && index % 2 == 0 {
        return None;
    }
    let numerator = base.numerator().clone().abs();
    let denominator = IBig::from(base.denominator().clone());
    let count = count as usize;
    // (a/b)^(p/q) = (a^p · b^((q-1)p))^(1/q) / b^p
    let radicand = numerator.pow(count) * denominator.pow((index as usize - 1) * count);
    let sign = if negative && count % 2 == 1 { IBig::NEG_ONE } else { IBig::ONE };
    let coefficient = RBig::from_parts(sign, UBig::try_from(denominator.pow(count)).ok()?);
    Some((coefficient, index, UBig::try_from(radicand).ok()?))
}

/// `radicand = outside^index · inside`, taking out every repeated factor trial division finds.
fn extract(index: u32, radicand: UBig) -> (UBig, UBig) {
    let (mut outside, mut inside) = (UBig::ONE, radicand);
    let mut prime = 2u32;
    while prime <= TRIAL_DIVISION_LIMIT && UBig::from(prime).pow(index as usize) <= inside {
        let divisor = UBig::from(prime);
        let mut multiplicity = 0;
        while (&inside % &divisor) == UBig::ZERO {
            inside /= &divisor;
            multiplicity += 1;
        }
        outside *= divisor.pow((multiplicity / index) as usize);
        inside *= divisor.pow((multiplicity % index) as usize);
        prime += if prime == 2 { 1 } else { 2 };
    }
    // What is left may itself be a perfect power, e.g. a large prime squared.
    let root = inside.nth_root(index as usize);
    if root != UBig::ONE && root.pow(index as usize) == inside {
        outside *= root;
        inside = UBig::ONE;
    }
    (outside, inside)
}

fn sum(terms: &Terms) -> String {
    let mut out = String::new();
    // The rational part first, then the radicals.
    for (radicals, coefficient) in terms {
        let negative = coefficient.numerator().sign() == Sign::Negative;
        let text = term(&coefficient.clone().abs(), radicals);
        match (out.is_empty(), negative) {
            (true, true) => out = format!("-{text}"),
            (true, false) => out = text,
            (false, true) => out.push_str(&format!(" - {text}")),
            (false, false) => out.push_str(&format!(" + {text}")),
        }
    }
    out
}

/// A positive coefficient with its radicals: `3`, `\sqrt{2}`, `\frac{3\sqrt{2}}{2}`.
fn term(coefficient: &RBig, radicals: &[Radical]) -> String {
    let roots: String = radicals
        .iter()
        .map(|(index, radicand)| match index {
            2 => format!("\\sqrt{{{radicand}}}"),
            n => format!("\\sqrt[{n}]{{{radicand}}}"),
        })
        .collect();
    let numerator = coefficient.numerator().clone().unsigned_abs();
    let numerator = match (numerator == UBig::ONE, roots.is_empty()) {
        (true, false) => roots,
        (_, _) => format!("{numerator}{roots}"),
    };
    if *coefficient.denominator() == UBig::ONE {
        numerator
    } else {
        format!("\\frac{{{numerator}}}{{{}}}", coefficient.denominator())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(base: i64, degree: u64) -> Value {
        Value::int(base).pow(&Value::ratio(1, degree)).unwrap()
    }

    #[test]
    fn a_lone_root() {
        assert_eq!(format(&root(2, 2)).unwrap(), "\\sqrt{2}");
        assert_eq!(format(&root(2, 3)).unwrap(), "\\sqrt[3]{2}");
    }

    #[test]
    fn squares_are_taken_out_of_the_root() {
        assert_eq!(format(&root(8, 2)).unwrap(), "2\\sqrt{2}");
        assert_eq!(format(&root(12, 2)).unwrap(), "2\\sqrt{3}");
        assert_eq!(format(&root(24, 3)).unwrap(), "2\\sqrt[3]{3}");
    }

    #[test]
    fn fractions_and_signs() {
        let half_root = Value::ratio(1, 2).mul(&root(2, 2));
        assert_eq!(format(&half_root).unwrap(), "\\frac{\\sqrt{2}}{2}");
        assert_eq!(format(&half_root.neg()).unwrap(), "-\\frac{\\sqrt{2}}{2}");
        assert_eq!(format(&Value::ratio(3, 2).mul(&root(5, 2))).unwrap(), "\\frac{3\\sqrt{5}}{2}");
        // 1/√2 is rationalised.
        let inverse = root(2, 2).div(&Value::int(2)).unwrap();
        assert_eq!(format(&inverse).unwrap(), "\\frac{\\sqrt{2}}{2}");
    }

    #[test]
    fn sums_collect_like_roots() {
        let sum = Value::int(1).add(&root(2, 2));
        assert_eq!(format(&sum).unwrap(), "1 + \\sqrt{2}");
        assert_eq!(format(&root(8, 2).add(&root(2, 2))).unwrap(), "3\\sqrt{2}");
        assert_eq!(format(&Value::int(3).sub(&root(5, 2))).unwrap(), "3 - \\sqrt{5}");
        assert_eq!(format(&root(2, 2).sub(&root(8, 2))).unwrap(), "-\\sqrt{2}");
    }

    #[test]
    fn roots_of_the_same_degree_combine() {
        assert_eq!(format(&root(2, 2).mul(&root(3, 2))).unwrap(), "\\sqrt{6}");
        assert_eq!(format(&root(2, 2).mul(&root(3, 3))).unwrap(), "\\sqrt{2}\\sqrt[3]{3}");
    }

    #[test]
    fn what_has_no_root_or_no_exact_form_is_left_alone() {
        assert!(format(&Value::ratio(1, 2)).is_none());
        assert!(format(&Value::Pi).is_none());
        assert!(format(&Value::Pi.mul(&root(2, 2))).is_none());
        assert!(format(&root(4, 2)).is_none());
    }

    #[test]
    fn a_sum_is_recognised_for_parentheses() {
        assert!(is_sum("1 + \\sqrt{2}"));
        assert!(!is_sum("-\\sqrt{2}"));
        assert!(!is_sum("\\frac{\\sqrt{2}}{2}"));
    }
}
