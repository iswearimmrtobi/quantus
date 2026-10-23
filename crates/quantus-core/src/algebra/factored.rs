//! Factored polynomial representation preserving explicit factors across transformations.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;

use dashu::base::{Abs, Signed};
use dashu::integer::IBig;
use dashu::rational::RBig;

use crate::algebra::poly::{rational_gcd, rational_lcm, rational_to_node, Poly};
use crate::latex::ast::{Node, Operator};
use crate::num::Decimal;

fn split_base(base: &Poly, divisors: &FactoredPoly) -> FactoredPoly {
    for d in &divisors.factors {
        if d.base == *base || d.base.total_degree() >= base.total_degree() {
            continue;
        }
        if let Ok((q, rem)) = base.div_rem(&d.base) {
            if rem.is_zero() && !q.is_constant() {
                let rest = split_base(&q.primitive_part(), divisors);
                let content = FactoredPoly::from_const(q.content());
                return FactoredPoly::from_factors(RBig::ONE, vec![Factor::new(d.base.clone(), 1)])
                    .mul(&rest)
                    .mul(&content);
            }
        }
    }
    FactoredPoly::from_factors(RBig::ONE, vec![Factor::new(base.clone(), 1)])
}

/// An irreducible/primitive polynomial base raised to a positive integer multiplicity.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Factor {
    pub base: Poly,
    pub exp: usize,
}

impl Factor {
    pub fn new(base: Poly, exp: usize) -> Self {
        Self { base, exp }
    }
}

impl Ord for Factor {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.base.cmp(&other.base) {
            Ordering::Equal => self.exp.cmp(&other.exp),
            ord => ord,
        }
    }
}

impl PartialOrd for Factor {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A factored polynomial: overall rational coefficient `coeff` multiplied by a canonical list of `Factor`s.
///
/// Invariants:
/// - If `coeff.is_zero()`, `factors.is_empty()`.
/// - Every `Factor` has `exp >= 1` and `base` is primitive with positive leading coefficient.
/// - No two factors share the same base (duplicate bases are combined by summing exponents).
/// - `factors` are sorted in canonical ascending order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FactoredPoly {
    pub coeff: RBig,
    pub factors: Vec<Factor>,
}

impl FactoredPoly {
    pub fn zero() -> Self {
        Self {
            coeff: RBig::ZERO,
            factors: Vec::new(),
        }
    }

    pub fn one() -> Self {
        Self {
            coeff: RBig::ONE,
            factors: Vec::new(),
        }
    }

    pub fn from_const(c: impl Into<RBig>) -> Self {
        let coeff = c.into();
        Self {
            coeff,
            factors: Vec::new(),
        }
    }

    /// Fully factors a polynomial over ℤ and ℚ into irreducible factors.
    pub fn from_poly(poly: Poly) -> Self {
        crate::algebra::factor::factor_polynomial(&poly)
    }

    /// Constructs and canonicalizes a `FactoredPoly` from a coefficient and a list of factors.
    pub fn from_factors(mut coeff: RBig, raw_factors: Vec<Factor>) -> Self {
        if coeff.is_zero() {
            return Self::zero();
        }

        let mut map: BTreeMap<Poly, usize> = BTreeMap::new();
        for f in raw_factors {
            if f.exp == 0 || f.base.is_zero() {
                if f.base.is_zero() {
                    return Self::zero();
                }
                continue;
            }
            if f.base.is_constant() {
                let c = f.base.leading_coeff();
                coeff *= pow_rbig(&c, f.exp);
                continue;
            }
            // Normalize factor base to be primitive with positive leading coefficient
            let c = f.base.content();
            let prim = f.base.primitive_part();
            coeff *= pow_rbig(&c, f.exp);
            *map.entry(prim).or_insert(0) += f.exp;
        }

        if coeff.is_zero() {
            return Self::zero();
        }

        let factors: Vec<Factor> = map
            .into_iter()
            .filter(|(_, exp)| *exp > 0)
            .map(|(base, exp)| Factor::new(base, exp))
            .collect();

        Self { coeff, factors }
    }

    /// Splits off every factor of `divisors` that divides one of this polynomial's factors exactly.
    ///
    /// The factorizer does not find every irreducible factor of high degree, but a factor of the
    /// denominator can be tested by trial division, so cancellation still sees it.
    pub fn split_by(&self, divisors: &Self) -> Self {
        let mut result = Self::from_const(self.coeff.clone());
        for f in &self.factors {
            let split = split_base(&f.base, divisors).pow(f.exp);
            result = result.mul(&split);
        }
        if result.to_poly() == self.to_poly() {
            result
        } else {
            self.clone()
        }
    }

    pub fn is_zero(&self) -> bool {
        self.coeff.is_zero()
    }

    pub fn is_one(&self) -> bool {
        self.coeff == RBig::ONE && self.factors.is_empty()
    }

    pub fn is_constant(&self) -> bool {
        self.factors.is_empty()
    }

    /// Multiplies two factored polynomials, preserving existing factor structure.
    pub fn mul(&self, other: &Self) -> Self {
        if self.is_zero() || other.is_zero() {
            return Self::zero();
        }
        let coeff = &self.coeff * &other.coeff;
        let mut map: BTreeMap<Poly, usize> = BTreeMap::new();
        for f in &self.factors {
            *map.entry(f.base.clone()).or_insert(0) += f.exp;
        }
        for f in &other.factors {
            *map.entry(f.base.clone()).or_insert(0) += f.exp;
        }
        let factors = map
            .into_iter()
            .map(|(base, exp)| Factor::new(base, exp))
            .collect();
        Self { coeff, factors }
    }

    /// Raises to non-negative integer power `exp`.
    pub fn pow(&self, exp: usize) -> Self {
        if exp == 0 {
            return Self::one();
        }
        if self.is_zero() {
            return Self::zero();
        }
        let coeff = pow_rbig(&self.coeff, exp);
        let factors = self
            .factors
            .iter()
            .map(|f| Factor::new(f.base.clone(), f.exp * exp))
            .collect();
        Self { coeff, factors }
    }

    /// Greatest common denominator directly over factored forms.
    pub fn gcd(&self, other: &Self) -> Self {
        if self.is_zero() {
            return other.clone();
        }
        if other.is_zero() {
            return self.clone();
        }

        let coeff = rational_gcd(&self.coeff, &other.coeff);
        let mut factors = Vec::new();

        for f1 in &self.factors {
            if let Some(f2) = other.factors.iter().find(|f| f.base == f1.base) {
                let exp = std::cmp::min(f1.exp, f2.exp);
                if exp > 0 {
                    factors.push(Factor::new(f1.base.clone(), exp));
                }
            }
        }

        Self { coeff, factors }
    }

    /// Least common denominator directly over factored forms.
    pub fn lcm(&self, other: &Self) -> Self {
        if self.is_zero() || other.is_zero() {
            return Self::zero();
        }

        let coeff = rational_lcm(&self.coeff, &other.coeff);
        let mut map: BTreeMap<Poly, usize> = BTreeMap::new();

        for f in &self.factors {
            map.insert(f.base.clone(), f.exp);
        }
        for f in &other.factors {
            let cur = map.get(&f.base).copied().unwrap_or(0);
            map.insert(f.base.clone(), std::cmp::max(cur, f.exp));
        }

        let factors = map
            .into_iter()
            .map(|(base, exp)| Factor::new(base, exp))
            .collect();

        Self { coeff, factors }
    }

    /// Exact division of factored polynomials.
    /// Returns `Some(quotient)` if `other` divides `self`.
    pub fn div(&self, other: &Self) -> Option<Self> {
        if other.is_zero() {
            return None;
        }
        if self.is_zero() {
            return Some(Self::zero());
        }

        let mut map: BTreeMap<Poly, usize> = BTreeMap::new();
        for f in &self.factors {
            map.insert(f.base.clone(), f.exp);
        }

        for f in &other.factors {
            let cur = map.get(&f.base).copied().unwrap_or(0);
            if cur < f.exp {
                return None;
            }
            if cur == f.exp {
                map.remove(&f.base);
            } else {
                map.insert(f.base.clone(), cur - f.exp);
            }
        }

        let coeff = &self.coeff / &other.coeff;
        let factors = map
            .into_iter()
            .map(|(base, exp)| Factor::new(base, exp))
            .collect();

        Some(Self { coeff, factors })
    }

    /// Expands the factored representation into an expanded `Poly`.
    pub fn to_poly(&self) -> Poly {
        if self.is_zero() {
            return Poly::zero();
        }
        let mut res = Poly::from_const(self.coeff.clone());
        for f in &self.factors {
            let factor_expanded = f.base.pow(f.exp);
            res = &res * &factor_expanded;
        }
        res
    }

    /// Converts `FactoredPoly` back into canonical `latex::ast::Node`,
    /// rendering `(x - 4)(x + 3)` or `(x + 3)^2` cleanly without redundant brackets.
    pub fn to_node(&self) -> Node {
        if self.is_zero() {
            return Node::Number {
                value: Decimal::new(IBig::ZERO, 0),
                position: 0,
            };
        }
        if self.factors.is_empty() {
            return rational_to_node(&self.coeff);
        }

        let mut factor_nodes = Vec::new();
        for factor in &self.factors {
            let base_is_single_var = factor.base.terms.len() == 1
                && factor.base.terms[0].coeff == RBig::ONE
                && factor.base.terms[0].monomial.total_degree() == 1;

            let factor_node = if base_is_single_var {
                let var_name = factor.base.terms[0].monomial.vars.keys().next().unwrap();
                let var_node = Node::Variable {
                    name: var_name.clone(),
                    index: String::new(),
                    position: 0,
                };
                if factor.exp == 1 {
                    var_node
                } else {
                    Node::Power {
                        base: Box::new(var_node),
                        exponent: Box::new(Node::Number {
                            value: Decimal::from_i64(factor.exp as i64),
                            position: 0,
                        }),
                        position: 0,
                    }
                }
            } else {
                let base_node = factor.base.to_node();
                let grouped = Node::Group {
                    inner: Box::new(base_node),
                    position: 0,
                };
                if factor.exp == 1 {
                    grouped
                } else {
                    Node::Power {
                        base: Box::new(grouped),
                        exponent: Box::new(Node::Number {
                            value: Decimal::from_i64(factor.exp as i64),
                            position: 0,
                        }),
                        position: 0,
                    }
                }
            };
            factor_nodes.push(factor_node);
        }

        let mut product = factor_nodes.remove(0);
        for next in factor_nodes {
            product = Node::Binary {
                operator: Operator::ImplicitTimes,
                left: Box::new(product),
                right: Box::new(next),
                position: 0,
            };
        }

        if self.coeff == RBig::ONE {
            product
        } else if self.coeff == -RBig::ONE {
            Node::Negation {
                operand: Box::new(product),
                position: 0,
            }
        } else if self.coeff.is_negative() {
            Node::Negation {
                operand: Box::new(Node::Binary {
                    operator: Operator::ImplicitTimes,
                    left: Box::new(rational_to_node(&self.coeff.clone().abs())),
                    right: Box::new(product),
                    position: 0,
                }),
                position: 0,
            }
        } else {
            Node::Binary {
                operator: Operator::ImplicitTimes,
                left: Box::new(rational_to_node(&self.coeff)),
                right: Box::new(product),
                position: 0,
            }
        }
    }

    pub fn to_latex(&self) -> String {
        crate::latex::serializer::expression(&self.to_node())
    }

    pub fn to_plain_string(&self) -> String {
        if self.is_zero() {
            return "0".to_string();
        }
        if self.factors.is_empty() {
            return self.coeff.to_string();
        }

        let mut s = String::new();
        if self.coeff == -RBig::ONE {
            s.push('-');
        } else if self.coeff != RBig::ONE {
            s.push_str(&self.coeff.to_string());
        }

        for factor in &self.factors {
            let base_is_single_var = factor.base.terms.len() == 1
                && factor.base.terms[0].coeff == RBig::ONE
                && factor.base.terms[0].monomial.total_degree() == 1;

            if base_is_single_var {
                let var = factor.base.terms[0].monomial.vars.keys().next().unwrap();
                if factor.exp == 1 {
                    s.push_str(var);
                } else {
                    s.push_str(&format!("{var}^{}", factor.exp));
                }
            } else if factor.exp == 1 {
                s.push_str(&format!("({})", factor.base.to_plain_string()));
            } else {
                s.push_str(&format!("({})^{}", factor.base.to_plain_string(), factor.exp));
            }
        }
        s
    }
}

pub fn pow_rbig(r: &RBig, exp: usize) -> RBig {
    if exp == 0 {
        return RBig::ONE;
    }
    let mut base = r.clone();
    let mut res = RBig::ONE;
    let mut e = exp;
    while e > 0 {
        if e % 2 == 1 {
            res = &res * &base;
        }
        if e > 1 {
            base = &base * &base;
        }
        e /= 2;
    }
    res
}

impl fmt::Display for FactoredPoly {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_plain_string())
    }
}
