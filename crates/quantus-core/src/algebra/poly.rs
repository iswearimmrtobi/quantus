//! Exact multivariate and univariate polynomial arithmetic over rationals (`RBig`).

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use dashu::base::{Abs, Signed, UnsignedAbs};
use dashu::integer::{IBig, UBig};
use dashu::rational::RBig;

use crate::latex::ast::{Node, Operator};
use crate::num::Decimal;
use crate::QuantusError;

/// Greatest common divisor of two non-negative unsigned big integers.
pub fn gcd_ubig(mut a: UBig, mut b: UBig) -> UBig {
    while !b.is_zero() {
        let r = a % &b;
        a = b;
        b = r;
    }
    a
}

/// Least common multiple of two non-negative unsigned big integers.
pub fn lcm_ubig(a: &UBig, b: &UBig) -> UBig {
    if a.is_zero() || b.is_zero() {
        UBig::ZERO
    } else {
        (a / gcd_ubig(a.clone(), b.clone())) * b
    }
}

/// Greatest common divisor of two exact rational numbers.
///
/// For non-zero `a = n1/d1` and `b = n2/d2`:
/// `gcd(a, b) = gcd(|n1|, |n2|) / lcm(d1, d2)`.
pub fn rational_gcd(a: &RBig, b: &RBig) -> RBig {
    if a.is_zero() {
        return b.clone().abs();
    }
    if b.is_zero() {
        return a.clone().abs();
    }
    let n1 = a.numerator().clone().unsigned_abs();
    let n2 = b.numerator().clone().unsigned_abs();
    let g_num = gcd_ubig(n1, n2);
    let l_den = lcm_ubig(a.denominator(), b.denominator());
    RBig::from_parts(IBig::from(g_num), l_den)
}

/// Least common multiple of two exact rational numbers.
///
/// `lcm(a, b) = (|a * b|) / gcd(a, b)`.
pub fn rational_lcm(a: &RBig, b: &RBig) -> RBig {
    if a.is_zero() || b.is_zero() {
        RBig::ZERO
    } else {
        (a.clone().abs() * b.clone().abs()) / rational_gcd(a, b)
    }
}

pub fn unsigned_abs(val: &IBig) -> UBig {
    val.clone().unsigned_abs()
}

/// A monomial representing a product of variables with positive integer powers: `x^2 y^3`.
///
/// An empty map represents the constant 1 (degree 0).
/// Monomials are ordered by graded lexicographical order (`grlex`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Monomial {
    pub vars: BTreeMap<String, usize>,
}

impl Monomial {
    pub fn one() -> Self {
        Self { vars: BTreeMap::new() }
    }

    pub fn is_one(&self) -> bool {
        self.vars.is_empty()
    }

    pub fn from_var(var: impl Into<String>) -> Self {
        Self::single(var, 1)
    }

    pub fn single(var: impl Into<String>, exp: usize) -> Self {
        let mut vars = BTreeMap::new();
        if exp > 0 {
            vars.insert(var.into(), exp);
        }
        Self { vars }
    }

    pub fn from_map(map: BTreeMap<String, usize>) -> Self {
        let mut vars = map;
        vars.retain(|_, &mut exp| exp > 0);
        Self { vars }
    }

    pub fn total_degree(&self) -> usize {
        self.vars.values().sum()
    }

    pub fn degree_of(&self, var: &str) -> usize {
        self.vars.get(var).copied().unwrap_or(0)
    }

    pub fn variables(&self) -> BTreeSet<String> {
        self.vars.keys().cloned().collect()
    }

    pub fn mul(&self, other: &Self) -> Self {
        let mut res = self.vars.clone();
        for (var, &exp) in &other.vars {
            *res.entry(var.clone()).or_insert(0) += exp;
        }
        Self { vars: res }
    }

    /// Exact monomial division. Returns `Some(quotient)` if `other` divides `self`.
    pub fn div(&self, other: &Self) -> Option<Self> {
        let mut res = self.vars.clone();
        for (var, &exp) in &other.vars {
            let cur = res.get(var).copied().unwrap_or(0);
            if cur < exp {
                return None;
            }
            if cur == exp {
                res.remove(var);
            } else {
                res.insert(var.clone(), cur - exp);
            }
        }
        Some(Self { vars: res })
    }

    pub fn gcd(&self, other: &Self) -> Self {
        let mut res = BTreeMap::new();
        for (var, &exp1) in &self.vars {
            if let Some(&exp2) = other.vars.get(var) {
                let m = std::cmp::min(exp1, exp2);
                if m > 0 {
                    res.insert(var.clone(), m);
                }
            }
        }
        Self { vars: res }
    }

    pub fn lcm(&self, other: &Self) -> Self {
        let mut res = self.vars.clone();
        for (var, &exp2) in &other.vars {
            let cur = res.get(var).copied().unwrap_or(0);
            res.insert(var.clone(), std::cmp::max(cur, exp2));
        }
        Self { vars: res }
    }

    pub fn pow(&self, exp: usize) -> Self {
        if exp == 0 {
            return Self::one();
        }
        let mut res = BTreeMap::new();
        for (var, &e) in &self.vars {
            res.insert(var.clone(), e * exp);
        }
        Self { vars: res }
    }
}

impl Ord for Monomial {
    fn cmp(&self, other: &Self) -> Ordering {
        let deg_self = self.total_degree();
        let deg_other = other.total_degree();
        if deg_self != deg_other {
            return deg_self.cmp(&deg_other);
        }
        // Graded lexicographical order:
        // Variables checked alphabetically
        let mut all_vars: BTreeSet<&str> = BTreeSet::new();
        for v in self.vars.keys() {
            all_vars.insert(v.as_str());
        }
        for v in other.vars.keys() {
            all_vars.insert(v.as_str());
        }
        for var in all_vars {
            let e1 = self.degree_of(var);
            let e2 = other.degree_of(var);
            if e1 != e2 {
                return e1.cmp(&e2);
            }
        }
        Ordering::Equal
    }
}

impl PartialOrd for Monomial {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Monomial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_one() {
            return write!(f, "1");
        }
        let mut first = true;
        for (var, &exp) in &self.vars {
            if !first {
                write!(f, " ")?;
            }
            first = false;
            if exp == 1 {
                write!(f, "{var}")?;
            } else {
                write!(f, "{var}^{exp}")?;
            }
        }
        Ok(())
    }
}

/// A single term in a polynomial: `coeff * monomial`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Term {
    pub coeff: RBig,
    pub monomial: Monomial,
}

impl Term {
    pub fn new(coeff: RBig, monomial: Monomial) -> Self {
        Self { coeff, monomial }
    }

    pub fn is_zero(&self) -> bool {
        self.coeff.is_zero()
    }
}

impl Ord for Term {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.monomial.cmp(&other.monomial) {
            Ordering::Equal => self.coeff.cmp(&other.coeff),
            ord => ord,
        }
    }
}

impl PartialOrd for Term {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A sparse multivariate polynomial with rational coefficients (`RBig`).
///
/// Invariants:
/// - Terms are strictly sorted in descending `Monomial` order (`terms[0]` is the leading term).
/// - No coefficient is zero (`!term.coeff.is_zero()`).
/// - The zero polynomial has `terms.is_empty()`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Poly {
    pub terms: Vec<Term>,
}

impl Poly {
    pub fn zero() -> Self {
        Self { terms: Vec::new() }
    }

    pub fn from_const(c: impl Into<RBig>) -> Self {
        let coeff: RBig = c.into();
        if coeff.is_zero() {
            Self::zero()
        } else {
            Self {
                terms: vec![Term::new(coeff, Monomial::one())],
            }
        }
    }

    pub fn from_var(var: &str) -> Self {
        Self {
            terms: vec![Term::new(RBig::ONE, Monomial::from_var(var))],
        }
    }

    pub fn from_term(coeff: RBig, monomial: Monomial) -> Self {
        if coeff.is_zero() {
            Self::zero()
        } else {
            Self {
                terms: vec![Term::new(coeff, monomial)],
            }
        }
    }

    /// Constructs a canonical polynomial from arbitrary terms: merges like monomials,
    /// drops zero coefficients, and sorts in descending monomial order.
    pub fn from_terms(mut raw_terms: Vec<Term>) -> Self {
        raw_terms.retain(|t| !t.coeff.is_zero());
        if raw_terms.is_empty() {
            return Self::zero();
        }
        // Sort descending by monomial
        raw_terms.sort_by(|a, b| b.monomial.cmp(&a.monomial));
        let mut terms: Vec<Term> = Vec::with_capacity(raw_terms.len());
        for term in raw_terms {
            if let Some(last) = terms.last_mut() {
                if last.monomial == term.monomial {
                    last.coeff += term.coeff;
                    continue;
                }
            }
            terms.push(term);
        }
        terms.retain(|t| !t.coeff.is_zero());
        Self { terms }
    }

    pub fn is_zero(&self) -> bool {
        self.terms.is_empty()
    }

    pub fn is_one(&self) -> bool {
        self.terms.len() == 1
            && self.terms[0].monomial.is_one()
            && self.terms[0].coeff == RBig::ONE
    }

    pub fn is_constant(&self) -> bool {
        self.terms.is_empty()
            || (self.terms.len() == 1 && self.terms[0].monomial.is_one())
    }

    pub fn is_univariate(&self) -> bool {
        self.variables().len() <= 1
    }

    pub fn total_degree(&self) -> usize {
        self.terms.first().map(|t| t.monomial.total_degree()).unwrap_or(0)
    }

    pub fn degree_of(&self, var: &str) -> usize {
        self.terms.iter().map(|t| t.monomial.degree_of(var)).max().unwrap_or(0)
    }

    pub fn variables(&self) -> BTreeSet<String> {
        let mut vars = BTreeSet::new();
        for term in &self.terms {
            vars.extend(term.monomial.variables());
        }
        vars
    }

    /// The variable with highest degree (breaking ties alphabetically).
    pub fn main_variable(&self) -> Option<String> {
        let vars = self.variables();
        vars.into_iter().max_by(|a, b| {
            let deg_a = self.degree_of(a);
            let deg_b = self.degree_of(b);
            deg_a.cmp(&deg_b).then_with(|| b.cmp(a))
        })
    }

    pub fn leading_coeff(&self) -> RBig {
        self.terms.first().map(|t| t.coeff.clone()).unwrap_or(RBig::ZERO)
    }

    pub fn leading_monomial(&self) -> Option<&Monomial> {
        self.terms.first().map(|t| &t.monomial)
    }

    pub fn leading_term(&self) -> Option<&Term> {
        self.terms.first()
    }

    pub fn constant_term(&self) -> RBig {
        self.terms
            .iter()
            .find(|t| t.monomial.is_one())
            .map(|t| t.coeff.clone())
            .unwrap_or(RBig::ZERO)
    }

    pub fn coeff_of(&self, monomial: &Monomial) -> RBig {
        self.terms
            .iter()
            .find(|t| &t.monomial == monomial)
            .map(|t| t.coeff.clone())
            .unwrap_or(RBig::ZERO)
    }

    pub fn pow(&self, exp: usize) -> Self {
        if exp == 0 {
            return Self::from_const(RBig::ONE);
        }
        if self.is_zero() {
            return Self::zero();
        }
        let mut base = self.clone();
        let mut res = Self::from_const(RBig::ONE);
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

    /// Multivariate / univariate polynomial division with remainder.
    ///
    /// Computes `(Q, R)` such that `self = Q * divisor + R` and no term of `R` is
    /// divisible by `divisor.leading_term()`. For univariate polynomials over `Q`,
    /// this guarantees `deg(R) < deg(divisor)`.
    pub fn div_rem(&self, divisor: &Poly) -> Result<(Poly, Poly), QuantusError> {
        if divisor.is_zero() {
            return Err(QuantusError::other("Division by zero."));
        }
        if self.is_zero() {
            return Ok((Poly::zero(), Poly::zero()));
        }
        let mut p = self.clone();
        let mut q_terms = Vec::new();
        let mut r_terms = Vec::new();
        let lt_d = divisor.terms.first().unwrap();

        while let Some(lt_p) = p.terms.first().cloned() {
            if let Some(m_div) = lt_p.monomial.div(&lt_d.monomial) {
                let c_div = &lt_p.coeff / &lt_d.coeff;
                let term = Term::new(c_div, m_div);
                let t_poly = Poly { terms: vec![term.clone()] };
                q_terms.push(term);
                let prod = &t_poly * divisor;
                p = &p - &prod;
            } else {
                r_terms.push(lt_p.clone());
                p = Poly::from_terms(p.terms[1..].to_vec());
            }
        }

        Ok((Poly::from_terms(q_terms), Poly::from_terms(r_terms)))
    }

    /// Extracts the greatest rational content `cont(P)`.
    ///
    /// The content sign is chosen such that `P / content` has a strictly positive leading coefficient.
    pub fn content(&self) -> RBig {
        if self.is_zero() {
            return RBig::ZERO;
        }
        let mut l = UBig::from(1u8);
        for t in &self.terms {
            l = lcm_ubig(&l, t.coeff.denominator());
        }
        let mut g = UBig::ZERO;
        for t in &self.terms {
            let num = t.coeff.numerator();
            let den = t.coeff.denominator();
            let int_val = (num * &l) / den;
            let abs_val = unsigned_abs(&int_val);
            g = gcd_ubig(g, abs_val);
        }
        let sign = if self.leading_coeff().is_negative() { -1 } else { 1 };
        let content_mag = RBig::from_parts(IBig::from(g), l);
        if sign == -1 {
            -content_mag
        } else {
            content_mag
        }
    }

    /// Primitive part: `self / content()`, having integer coefficients whose GCD is 1 and
    /// whose leading coefficient is positive.
    pub fn primitive_part(&self) -> Poly {
        if self.is_zero() {
            return Poly::zero();
        }
        let c = self.content();
        self / &c
    }

    /// Makes the polynomial monic (leading coefficient 1), or returns zero if empty.
    pub fn to_monic(&self) -> Poly {
        if self.is_zero() {
            Poly::zero()
        } else {
            self / &self.leading_coeff()
        }
    }

    /// Greatest common divisor of all terms' monomials.
    pub fn monomial_gcd(&self) -> Monomial {
        if self.terms.is_empty() {
            return Monomial::one();
        }
        let mut g = self.terms[0].monomial.clone();
        for t in &self.terms[1..] {
            g = g.gcd(&t.monomial);
        }
        g
    }

    /// Exact polynomial GCD over rationals (`RBig`).
    ///
    /// Preserves numerical content GCD and monomial factors.
    pub fn gcd(&self, other: &Self) -> Poly {
        if self.is_zero() {
            return other.primitive_part();
        }
        if other.is_zero() {
            return self.primitive_part();
        }

        let m1 = self.monomial_gcd();
        let m2 = other.monomial_gcd();
        let m_gcd = m1.gcd(&m2);

        let c1 = self.content();
        let c2 = other.content();
        let c_gcd = rational_gcd(&c1, &c2);

        let p1 = self.primitive_part();
        let p2 = other.primitive_part();

        let div_m = |p: &Poly, m: &Monomial| -> Poly {
            if m.is_one() {
                p.clone()
            } else {
                Poly::from_terms(
                    p.terms
                        .iter()
                        .map(|t| Term::new(t.coeff.clone(), t.monomial.div(m).unwrap()))
                        .collect(),
                )
            }
        };

        let mut r0 = div_m(&p1, &m1);
        let mut r1 = div_m(&p2, &m2);

        if !r0.is_constant() && !r1.is_constant() {
            while !r1.is_zero() {
                if let Ok((_, r2)) = r0.div_rem(&r1) {
                    r0 = r1;
                    r1 = r2.primitive_part();
                } else {
                    break;
                }
            }
        }

        let poly_part = if r0.is_constant() {
            Poly::from_const(RBig::ONE)
        } else {
            r0.primitive_part()
        };

        let with_monomial = if m_gcd.is_one() {
            poly_part
        } else {
            &Poly::from_term(RBig::ONE, m_gcd) * &poly_part
        };

        &Poly::from_const(c_gcd) * &with_monomial
    }

    /// Least common multiple of two polynomials: `(|A * B|) / gcd(A, B)`.
    pub fn lcm(&self, other: &Self) -> Poly {
        if self.is_zero() || other.is_zero() {
            Poly::zero()
        } else {
            let g = self.gcd(other);
            let prod = self * other;
            match prod.div_rem(&g) {
                Ok((q, _)) => q.primitive_part() * rational_lcm(&self.content(), &other.content()),
                Err(_) => prod,
            }
        }
    }

    /// Factors this polynomial into irreducible factors over ℤ and ℚ.
    pub fn factor(&self) -> crate::algebra::factored::FactoredPoly {
        crate::algebra::factor::factor_polynomial(self)
    }

    /// Bidirectional conversion: parses a `latex::ast::Node` into a `Poly` if valid.
    pub fn from_node(node: &Node) -> Result<Poly, QuantusError> {
        match node {
            Node::Number { value, .. } => Ok(Poly::from_const(value.to_rational())),
            Node::Variable { name, index, .. } => {
                let var = Node::variable_key(name, index);
                Ok(Poly::from_var(&var))
            }
            Node::Binary { operator, left, right, .. } => match operator {
                Operator::Plus => Ok(&Self::from_node(left)? + &Self::from_node(right)?),
                Operator::Minus => Ok(&Self::from_node(left)? - &Self::from_node(right)?),
                Operator::Times | Operator::ImplicitTimes => {
                    Ok(&Self::from_node(left)? * &Self::from_node(right)?)
                }
                Operator::Divide => {
                    let num = Self::from_node(left)?;
                    let den = Self::from_node(right)?;
                    if den.is_constant() && !den.is_zero() {
                        let c = den.leading_coeff();
                        Ok(&num / &c)
                    } else {
                        Err(QuantusError::other("Division by non-constant is not a polynomial."))
                    }
                }
            },
            Node::Fraction { numerator, denominator, .. } => {
                let num = Self::from_node(numerator)?;
                let den = Self::from_node(denominator)?;
                if den.is_constant() && !den.is_zero() {
                    let c = den.leading_coeff();
                    Ok(&num / &c)
                } else {
                    Err(QuantusError::other("Division by non-constant is not a polynomial."))
                }
            }
            Node::Negation { operand, .. } => Ok(-&Self::from_node(operand)?),
            Node::Group { inner, .. } => Self::from_node(inner),
            Node::Power { base, exponent, .. } => {
                let b = Self::from_node(base)?;
                let mut exp_node = &**exponent;
                while let Node::Group { inner, .. } = exp_node {
                    exp_node = inner;
                }
                let exp_val = match exp_node {
                    Node::Number { value, .. } => {
                        let r = value.to_rational();
                        if r.denominator() == &dashu::integer::UBig::from(1u8)
                            && !r.numerator().is_negative()
                        {
                            let n = u64::try_from(r.numerator())
                                .map_err(|_| QuantusError::other("Exponent too large."))?;
                            n as usize
                        } else {
                            return Err(QuantusError::other(
                                "Non-integer or negative exponent in polynomial.",
                            ));
                        }
                    }
                    _ => return Err(QuantusError::other("Variable or complex exponent in polynomial.")),
                };
                Ok(b.pow(exp_val))
            }
            _ => Err(QuantusError::other("Expression is not a polynomial.")),
        }
    }

    /// Converts `Poly` back into canonical `latex::ast::Node`.
    pub fn to_node(&self) -> Node {
        if self.is_zero() {
            return Node::Number {
                value: Decimal::new(IBig::ZERO, 0),
                position: 0,
            };
        }
        let mut acc: Option<Node> = None;
        for term in &self.terms {
            let is_neg = term.coeff.is_negative();
            let abs_coeff = term.coeff.clone().abs();
            let term_node = term_to_node(&abs_coeff, &term.monomial);

            match acc {
                None => {
                    if is_neg {
                        acc = Some(Node::Negation {
                            operand: Box::new(term_node),
                            position: 0,
                        });
                    } else {
                        acc = Some(term_node);
                    }
                }
                Some(prev) => {
                    if is_neg {
                        acc = Some(Node::Binary {
                            operator: Operator::Minus,
                            left: Box::new(prev),
                            right: Box::new(term_node),
                            position: 0,
                        });
                    } else {
                        acc = Some(Node::Binary {
                            operator: Operator::Plus,
                            left: Box::new(prev),
                            right: Box::new(term_node),
                            position: 0,
                        });
                    }
                }
            }
        }
        acc.unwrap()
    }

    /// Plain formatted string representation: `3x^2 - 17x - 78`.
    pub fn to_plain_string(&self) -> String {
        if self.is_zero() {
            return "0".to_string();
        }
        let mut s = String::new();
        for (i, term) in self.terms.iter().enumerate() {
            let is_neg = term.coeff.is_negative();
            let abs_coeff = term.coeff.clone().abs();
            if i == 0 {
                if is_neg {
                    s.push('-');
                }
            } else if is_neg {
                s.push_str(" - ");
            } else {
                s.push_str(" + ");
            }

            if term.monomial.is_one() {
                s.push_str(&abs_coeff.to_string());
            } else if abs_coeff == RBig::ONE {
                s.push_str(&term.monomial.to_string());
            } else {
                s.push_str(&format!("{} {}", abs_coeff, term.monomial));
            }
        }
        s
    }

    /// Canonical LaTeX string representation.
    pub fn to_latex(&self) -> String {
        crate::latex::serializer::expression(&self.to_node())
    }
}

pub fn rational_to_node(r: &RBig) -> Node {
    let num = r.numerator().clone();
    let den = r.denominator().clone();
    if den == UBig::from(1u8) {
        Node::Number {
            value: Decimal::new(num, 0),
            position: 0,
        }
    } else {
        Node::Fraction {
            numerator: Box::new(Node::Number {
                value: Decimal::new(num, 0),
                position: 0,
            }),
            denominator: Box::new(Node::Number {
                value: Decimal::new(IBig::from(den), 0),
                position: 0,
            }),
            position: 0,
        }
    }
}

pub fn monomial_to_node(m: &Monomial) -> Node {
    let mut factor_nodes = Vec::new();
    for (var, &exp) in &m.vars {
        let var_node = Node::Variable {
            name: var.clone(),
            index: String::new(),
            position: 0,
        };
        if exp == 1 {
            factor_nodes.push(var_node);
        } else {
            factor_nodes.push(Node::Power {
                base: Box::new(var_node),
                exponent: Box::new(Node::Number {
                    value: Decimal::from_i64(exp as i64),
                    position: 0,
                }),
                position: 0,
            });
        }
    }
    if factor_nodes.is_empty() {
        Node::Number {
            value: Decimal::from_i64(1),
            position: 0,
        }
    } else {
        let mut expr = factor_nodes.remove(0);
        for next in factor_nodes {
            expr = Node::Binary {
                operator: Operator::ImplicitTimes,
                left: Box::new(expr),
                right: Box::new(next),
                position: 0,
            };
        }
        expr
    }
}

pub fn term_to_node(coeff: &RBig, monomial: &Monomial) -> Node {
    if monomial.is_one() {
        rational_to_node(coeff)
    } else if *coeff == RBig::ONE {
        monomial_to_node(monomial)
    } else {
        Node::Binary {
            operator: Operator::ImplicitTimes,
            left: Box::new(rational_to_node(coeff)),
            right: Box::new(monomial_to_node(monomial)),
            position: 0,
        }
    }
}

impl Ord for Poly {
    fn cmp(&self, other: &Self) -> Ordering {
        self.terms.cmp(&other.terms)
    }
}

impl PartialOrd for Poly {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Add for &Poly {
    type Output = Poly;
    fn add(self, other: &Poly) -> Poly {
        let mut combined = Vec::with_capacity(self.terms.len() + other.terms.len());
        combined.extend(self.terms.iter().cloned());
        combined.extend(other.terms.iter().cloned());
        Poly::from_terms(combined)
    }
}

impl Add for Poly {
    type Output = Poly;
    fn add(self, other: Poly) -> Poly {
        &self + &other
    }
}

impl AddAssign for Poly {
    fn add_assign(&mut self, other: Self) {
        *self = &*self + &other;
    }
}

impl Sub for &Poly {
    type Output = Poly;
    fn sub(self, other: &Poly) -> Poly {
        let mut combined = Vec::with_capacity(self.terms.len() + other.terms.len());
        combined.extend(self.terms.iter().cloned());
        for t in &other.terms {
            combined.push(Term::new(-&t.coeff, t.monomial.clone()));
        }
        Poly::from_terms(combined)
    }
}

impl Sub for Poly {
    type Output = Poly;
    fn sub(self, other: Poly) -> Poly {
        &self - &other
    }
}

impl SubAssign for Poly {
    fn sub_assign(&mut self, other: Self) {
        *self = &*self - &other;
    }
}

impl Neg for &Poly {
    type Output = Poly;
    fn neg(self) -> Poly {
        let terms = self
            .terms
            .iter()
            .map(|t| Term::new(-&t.coeff, t.monomial.clone()))
            .collect();
        Poly { terms }
    }
}

impl Neg for Poly {
    type Output = Poly;
    fn neg(self) -> Poly {
        -&self
    }
}

impl Mul for &Poly {
    type Output = Poly;
    fn mul(self, other: &Poly) -> Poly {
        if self.is_zero() || other.is_zero() {
            return Poly::zero();
        }
        let mut raw = Vec::with_capacity(self.terms.len() * other.terms.len());
        for t1 in &self.terms {
            for t2 in &other.terms {
                let c = &t1.coeff * &t2.coeff;
                let m = t1.monomial.mul(&t2.monomial);
                raw.push(Term::new(c, m));
            }
        }
        Poly::from_terms(raw)
    }
}

impl Mul for Poly {
    type Output = Poly;
    fn mul(self, other: Poly) -> Poly {
        &self * &other
    }
}

impl MulAssign for Poly {
    fn mul_assign(&mut self, other: Self) {
        *self = &*self * &other;
    }
}

impl Mul<&RBig> for &Poly {
    type Output = Poly;
    fn mul(self, scalar: &RBig) -> Poly {
        if scalar.is_zero() || self.is_zero() {
            Poly::zero()
        } else {
            let terms = self
                .terms
                .iter()
                .map(|t| Term::new(&t.coeff * scalar, t.monomial.clone()))
                .collect();
            Poly { terms }
        }
    }
}

impl Mul<&RBig> for Poly {
    type Output = Poly;
    fn mul(self, scalar: &RBig) -> Poly {
        &self * scalar
    }
}

impl Mul<RBig> for Poly {
    type Output = Poly;
    fn mul(self, scalar: RBig) -> Poly {
        &self * &scalar
    }
}

impl Div<&RBig> for &Poly {
    type Output = Poly;
    fn div(self, scalar: &RBig) -> Poly {
        assert!(!scalar.is_zero(), "Division by zero scalar in polynomial.");
        let terms = self
            .terms
            .iter()
            .map(|t| Term::new(&t.coeff / scalar, t.monomial.clone()))
            .collect();
        Poly { terms }
    }
}

impl Div<&RBig> for Poly {
    type Output = Poly;
    fn div(self, scalar: &RBig) -> Poly {
        &self / scalar
    }
}

impl Div<RBig> for Poly {
    type Output = Poly;
    fn div(self, scalar: RBig) -> Poly {
        &self / &scalar
    }
}

impl DivAssign<&RBig> for Poly {
    fn div_assign(&mut self, scalar: &RBig) {
        *self = &*self / scalar;
    }
}

impl fmt::Display for Poly {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_plain_string())
    }
}
