//! Products of rational powers, the algebra behind both units and dimensions.
//!
//! Mirrors Indriya's `ProductUnit`: a list of `item^(pow/root)` factors in insertion order.
//! The order is observable (it decides how `kg·m/s²` and `[M]·[L]/[T]²` print), so the merge
//! rules follow `ProductUnit.getInstance` exactly: factors of the left operand keep their
//! place, equal factors add their exponents, cancelled factors vanish, and factors that only
//! occur on the right are appended.

use std::fmt::Write;

/// `item^(pow/root)`, always reduced (`gcd(|pow|, root) == 1`) and never `pow == 0`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Element<T> {
    pub item: T,
    pub pow: i32,
    pub root: i32,
}

impl<T> Element<T> {
    pub fn new(item: T, pow: i32, root: i32) -> Self {
        Element { item, pow, root }
    }

    pub fn once(item: T) -> Self {
        Element { item, pow: 1, root: 1 }
    }
}

fn gcd(a: i32, b: i32) -> i32 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

/// `left * right`: `ProductUnit.getInstance`.
pub(crate) fn merge<T: Clone + PartialEq>(left: &[Element<T>], right: &[Element<T>]) -> Vec<Element<T>> {
    let mut result = Vec::with_capacity(left.len() + right.len());
    for l in left {
        let (p2, r2) = right
            .iter()
            .find(|r| r.item == l.item)
            .map_or((0, 1), |r| (r.pow, r.root));
        let pow = l.pow * r2 + p2 * l.root;
        let root = l.root * r2;
        if pow != 0 {
            let g = gcd(pow.abs(), root);
            result.push(Element::new(l.item.clone(), pow / g, root / g));
        }
    }
    for r in right {
        if !left.iter().any(|l| l.item == r.item) {
            result.push(r.clone());
        }
    }
    result
}

/// Every exponent negated: the right operand of a quotient.
pub(crate) fn inverse<T: Clone>(elements: &[Element<T>]) -> Vec<Element<T>> {
    elements
        .iter()
        .map(|e| Element::new(e.item.clone(), -e.pow, e.root))
        .collect()
}

/// `ProductUnit.ofPow`; `n == 0` leaves the empty product.
pub(crate) fn pow<T: Clone + PartialEq>(elements: &[Element<T>], n: i32) -> Vec<Element<T>> {
    let scaled: Vec<_> = elements
        .iter()
        .map(|e| Element::new(e.item.clone(), e.pow * n, e.root))
        .collect();
    merge(&scaled, &[])
}

/// `ProductUnit.ofRoot` for `n > 0`.
pub(crate) fn root<T: Clone + PartialEq>(elements: &[Element<T>], n: i32) -> Vec<Element<T>> {
    let scaled: Vec<_> = elements
        .iter()
        .map(|e| Element::new(e.item.clone(), e.pow, e.root * n))
        .collect();
    merge(&scaled, &[])
}

/// `SimpleUnitFormat` for a non-empty product: positive powers joined by `·`, then `/`, then
/// the negative ones (in brackets when there is more than one). `1/s` when nothing is positive.
///
/// Squares and cubes use `²` and `³`, everything else `^p`, roots `^p:r`.
pub(crate) fn format<T>(elements: &[Element<T>], name: impl Fn(&T) -> String) -> String {
    let mut out = String::new();
    let mut start = true;
    let mut inverted = 0;
    for e in elements {
        if e.pow >= 0 {
            if !start {
                out.push('·');
            }
            append(&mut out, &name(&e.item), e.pow, e.root);
            start = false;
        } else {
            inverted += 1;
        }
    }
    if inverted != 0 {
        if start {
            out.push('1');
        }
        out.push('/');
        if inverted > 1 {
            out.push('(');
        }
        let mut start = true;
        for e in elements.iter().filter(|e| e.pow < 0) {
            if !start {
                out.push('·');
            }
            append(&mut out, &name(&e.item), -e.pow, e.root);
            start = false;
        }
        if inverted > 1 {
            out.push(')');
        }
    }
    out
}

fn append(out: &mut String, symbol: &str, pow: i32, root: i32) {
    out.push_str(symbol);
    match (pow, root) {
        (1, 1) => {}
        (2, 1) => out.push('²'),
        (3, 1) => out.push('³'),
        (_, 1) => {
            let _ = write!(out, "^{pow}");
        }
        _ => {
            let _ = write!(out, "^{pow}:{root}");
        }
    }
}

/// Order-insensitive equality, as `ProductUnit.equals` (`arrayEqualsArbitraryOrder`).
pub(crate) fn same_elements<T: PartialEq>(a: &[Element<T>], b: &[Element<T>]) -> bool {
    a.len() == b.len() && a.iter().all(|x| b.iter().any(|y| x == y))
}
