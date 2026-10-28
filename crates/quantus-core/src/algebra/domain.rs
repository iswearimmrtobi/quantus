//! Domain restrictions extracted from denominators, fractional powers, and transformation steps.

use std::cmp::Ordering;
use std::fmt;

use dashu::base::{Abs, Signed, SquareRoot};
use dashu::integer::{IBig, UBig};
use dashu::rational::RBig;

use crate::algebra::poly::{Monomial, Poly, Term};
use crate::error::QuantusError;
use crate::latex::ast::{Node, Operator};

/// A domain restriction indicating that a variable or sub-expression cannot equal a specific value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DomainRestriction {
    /// A single variable cannot equal a specific rational value: `x != -3`.
    VariableNotEqual {
        variable: String,
        value: RBig,
    },
    /// A general polynomial cannot be zero: `P(x, y) != 0`.
    PolyNotZero(Poly),
    /// A polynomial must not be negative: `a \geq 0`, the condition of `\sqrt{a} \sqrt{b} = \sqrt{ab}`.
    PolyNonNegative(Poly),
    /// A polynomial must be positive: `a > 0`, the condition of `\ln a + \ln b = \ln(ab)`.
    PolyPositive(Poly),
}

impl DomainRestriction {
    pub fn var_not_equal(var: impl Into<String>, val: impl Into<RBig>) -> Self {
        Self::VariableNotEqual {
            variable: var.into(),
            value: val.into(),
        }
    }

    pub fn poly_not_zero(poly: Poly) -> Self {
        Self::PolyNotZero(poly)
    }

    pub fn to_plain_string(&self) -> String {
        match self {
            Self::VariableNotEqual { variable, value } => {
                format!("{variable} != {value}")
            }
            Self::PolyNotZero(poly) => {
                format!("{} != 0", poly.to_plain_string())
            }
            Self::PolyNonNegative(poly) => format!("{} >= 0", poly.to_plain_string()),
            Self::PolyPositive(poly) => format!("{} > 0", poly.to_plain_string()),
        }
    }

    pub fn to_latex(&self) -> String {
        match self {
            Self::VariableNotEqual { variable, value } => {
                format!("{variable} \\neq {}", format_rbig_latex(value))
            }
            Self::PolyNotZero(poly) => {
                format!("{} \\neq 0", poly.to_latex())
            }
            Self::PolyNonNegative(poly) => format!("{} \\geq 0", poly.to_latex()),
            Self::PolyPositive(poly) => format!("{} > 0", poly.to_latex()),
        }
    }

    /// Where the restriction sorts: `≠` before `≥` before `>`.
    fn rank(&self) -> u8 {
        match self {
            Self::VariableNotEqual { .. } => 0,
            Self::PolyNotZero(_) => 1,
            Self::PolyNonNegative(_) => 2,
            Self::PolyPositive(_) => 3,
        }
    }
}

impl Ord for DomainRestriction {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (
                DomainRestriction::VariableNotEqual { variable: v1, value: val1 },
                DomainRestriction::VariableNotEqual { variable: v2, value: val2 },
            ) => match v1.cmp(v2) {
                Ordering::Equal => val1.cmp(val2),
                ord => ord,
            },
            (DomainRestriction::PolyNotZero(p1), DomainRestriction::PolyNotZero(p2)) => p1.cmp(p2),
            // Inequalities read in the order of their text: a \geq 0, b \geq 0.
            (DomainRestriction::PolyNonNegative(p1), DomainRestriction::PolyNonNegative(p2))
            | (DomainRestriction::PolyPositive(p1), DomainRestriction::PolyPositive(p2)) => {
                p1.to_plain_string().cmp(&p2.to_plain_string()).then_with(|| p1.cmp(p2))
            }
            _ => self.rank().cmp(&other.rank()),
        }
    }
}

impl PartialOrd for DomainRestriction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for DomainRestriction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_plain_string())
    }
}

/// Formats a list of domain restrictions into canonical LaTeX representation,
/// grouping multiple values for the same variable (e.g. `x \neq -3, 2, 4`).
pub fn format_restrictions_latex(restrictions: &[DomainRestriction]) -> String {
    format_restrictions_latex_list(restrictions).join(", ")
}

/// Formats a list of domain restrictions into canonical LaTeX strings per variable/polynomial.
pub fn format_restrictions_latex_list(restrictions: &[DomainRestriction]) -> Vec<String> {
    if restrictions.is_empty() {
        return Vec::new();
    }
    let mut sorted = restrictions.to_vec();
    sorted.sort();
    sorted.dedup();

    let mut parts = Vec::new();
    let mut i = 0;
    while i < sorted.len() {
        match &sorted[i] {
            DomainRestriction::VariableNotEqual { variable, .. } => {
                let var_name = variable.clone();
                let mut values = Vec::new();
                while i < sorted.len() {
                    if let DomainRestriction::VariableNotEqual { variable: v, value } = &sorted[i] {
                        if v == &var_name {
                            values.push(format_rbig_latex(value));
                            i += 1;
                            continue;
                        }
                    }
                    break;
                }
                parts.push(format!("{var_name} \\neq {}", values.join(", ")));
            }
            other => {
                parts.push(other.to_latex());
                i += 1;
            }
        }
    }
    parts
}

/// Formats a list of domain restrictions into plain text representation,
/// grouping multiple values for the same variable (e.g. `x != -3, 2, 4`).
pub fn format_restrictions_plain(restrictions: &[DomainRestriction]) -> String {
    if restrictions.is_empty() {
        return String::new();
    }
    let mut sorted = restrictions.to_vec();
    sorted.sort();
    sorted.dedup();

    let mut parts = Vec::new();
    let mut i = 0;
    while i < sorted.len() {
        match &sorted[i] {
            DomainRestriction::VariableNotEqual { variable, .. } => {
                let var_name = variable.clone();
                let mut values = Vec::new();
                while i < sorted.len() {
                    if let DomainRestriction::VariableNotEqual { variable: v, value } = &sorted[i] {
                        if v == &var_name {
                            values.push(value.to_string());
                            i += 1;
                            continue;
                        }
                    }
                    break;
                }
                parts.push(format!("{var_name} != {}", values.join(", ")));
            }
            other => {
                parts.push(other.to_plain_string());
                i += 1;
            }
        }
    }
    parts.join(", ")
}

/// `p \geq 0` together with `p \neq 0` is `p > 0`, and `p > 0` makes both redundant.
pub fn merge_inequalities(restrictions: Vec<DomainRestriction>) -> Vec<DomainRestriction> {
    let not_zero = |r: &DomainRestriction| match r {
        DomainRestriction::VariableNotEqual { variable, value } if *value == RBig::ZERO => Some(Poly::from_var(variable)),
        DomainRestriction::PolyNotZero(p) => Some(p.clone()),
        _ => None,
    };
    let excluded: Vec<Poly> = restrictions.iter().filter_map(not_zero).collect();
    let mut positive: Vec<Poly> = Vec::new();
    for restriction in &restrictions {
        let poly = match restriction {
            DomainRestriction::PolyPositive(p) => p,
            DomainRestriction::PolyNonNegative(p) if excluded.contains(p) => p,
            _ => continue,
        };
        if !positive.contains(poly) {
            positive.push(poly.clone());
        }
    }
    let mut merged: Vec<DomainRestriction> = Vec::new();
    for restriction in restrictions {
        let redundant = match &restriction {
            DomainRestriction::PolyNonNegative(p) | DomainRestriction::PolyPositive(p) => positive.contains(p),
            other => not_zero(other).is_some_and(|p| positive.contains(&p)),
        };
        if !redundant && !merged.contains(&restriction) {
            merged.push(restriction);
        }
    }
    merged.extend(positive.into_iter().map(DomainRestriction::PolyPositive));
    merged
}

pub fn format_rbig_latex(r: &RBig) -> String {
    let num = r.numerator();
    let den = r.denominator();
    if den == &UBig::from(1u8) {
        num.to_string()
    } else if num.is_negative() {
        format!("-\\frac{{{}}}{{{den}}}", num.clone().abs())
    } else {
        format!("\\frac{{{num}}}{{{den}}}")
    }
}

/// Unwraps nested `Node::Group`s to get to the underlying node.
pub fn unwrap_group(mut node: &Node) -> &Node {
    while let Node::Group { inner, .. } = node {
        node = inner;
    }
    node
}

/// Checks if an exponent node represents a negative number or negative constant polynomial.
pub fn is_negative_exponent(node: &Node) -> bool {
    let unwrapped = unwrap_group(node);
    if let Ok(poly) = Poly::from_node(unwrapped) {
        if poly.is_constant() {
            return poly.leading_coeff().is_negative();
        }
    }
    match unwrapped {
        Node::Negation { .. } => true,
        Node::Number { value, .. } => value.is_negative() || value.to_rational().numerator().is_negative(),
        _ => false,
    }
}

/// Recursively decomposes an expression in a denominator / divisor position.
///
/// In particular:
/// - In `a / (b / c)`, applying the reciprocal rule gives `a * (c / b)`.
///   Therefore both `c != 0` and `b != 0` are required.
/// - Products `b * c` in a denominator require both `b != 0` and `c != 0`.
/// - Powers `base^k` in a denominator require `base != 0`.
pub fn extract_denominators_from_divisor(divisor: &Node, out: &mut Vec<Node>) {
    let unwrapped = unwrap_group(divisor);
    match unwrapped {
        Node::Fraction { numerator, denominator, .. }
        | Node::Binary {
            operator: Operator::Divide,
            left: numerator,
            right: denominator,
            ..
        } => {
            extract_denominators_from_divisor(denominator, out);
            extract_denominators_from_divisor(numerator, out);
        }
        Node::Binary {
            operator: Operator::Times | Operator::ImplicitTimes,
            left,
            right,
            ..
        } => {
            extract_denominators_from_divisor(left, out);
            extract_denominators_from_divisor(right, out);
        }
        Node::Power { base, .. } => {
            extract_denominators_from_divisor(base, out);
        }
        _ => {
            out.push(unwrapped.clone());
        }
    }
}

fn collect_raw_denominators_rec(node: &Node, out: &mut Vec<Node>) {
    let unwrapped = unwrap_group(node);
    match unwrapped {
        Node::Fraction { numerator, denominator, .. } => {
            extract_denominators_from_divisor(denominator, out);
            collect_raw_denominators_rec(numerator, out);
            collect_raw_denominators_rec(denominator, out);
        }
        Node::Binary {
            operator: Operator::Divide,
            left,
            right,
            ..
        } => {
            extract_denominators_from_divisor(right, out);
            collect_raw_denominators_rec(left, out);
            collect_raw_denominators_rec(right, out);
        }
        Node::Power { base, exponent, .. } => {
            if is_negative_exponent(exponent) {
                extract_denominators_from_divisor(base, out);
            }
            collect_raw_denominators_rec(base, out);
            collect_raw_denominators_rec(exponent, out);
        }
        _ => {
            for child in unwrapped.children() {
                collect_raw_denominators_rec(child, out);
            }
        }
    }
}

/// Recursively scans an AST before any cancellation or reciprocal rewrites to collect
/// all expressions that appear in a denominator or divisor position.
///
/// This includes:
/// - Explicit denominators of `Node::Fraction` and `Operator::Divide`.
/// - Sub-expressions that become denominators after the reciprocal rule `a / (b / c) -> (a * c) / b`.
/// - Bases of negative exponents `base^(-n)`.
///
/// Results are deduplicated while preserving discovery order.
pub fn collect_raw_denominators(node: &Node) -> Vec<Node> {
    let mut raw = Vec::new();
    collect_raw_denominators_rec(node, &mut raw);
    let mut unique = Vec::new();
    for item in raw {
        if !unique.contains(&item) {
            unique.push(item);
        }
    }
    unique
}

fn exact_rational_sqrt(r: &RBig) -> Option<RBig> {
    let num = r.numerator();
    let den = r.denominator();
    if num.is_negative() {
        return None;
    }
    let (_, num_abs) = num.clone().into_parts();
    let num_s = num_abs.sqrt();
    if &num_s * &num_s != num_abs {
        return None;
    }
    let den_s = den.sqrt();
    if &den_s * &den_s != *den {
        return None;
    }
    Some(RBig::from_parts(IBig::from(num_s), den_s))
}

/// Solves for rational roots of a univariate polynomial.
/// Returns the list of distinct rational roots, and any remaining irreducible cofactor of degree >= 2.
pub fn solve_univariate_rational_roots(poly: &Poly, var: &str) -> (Vec<RBig>, Option<Poly>) {
    let mut current = poly.primitive_part();
    if current.leading_coeff().is_negative() {
        current = -&current;
    }
    let mut roots = Vec::new();

    // 1. Monomial root at 0: factor out powers of var
    while current.degree_of(var) > 0 {
        let c0 = current.coeff_of(&Monomial::one());
        if c0.is_zero() {
            if !roots.contains(&RBig::ZERO) {
                roots.push(RBig::ZERO);
            }
            let x_poly = Poly::from_var(var);
            if let Ok((q, _)) = current.div_rem(&x_poly) {
                current = q.primitive_part();
                if current.leading_coeff().is_negative() {
                    current = -&current;
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // 2. While degree >= 3, test rational roots via Rational Root Theorem
    while current.degree_of(var) >= 3 {
        let deg = current.degree_of(var);
        let cn = current.coeff_of(&Monomial::single(var, deg));
        let c0 = current.coeff_of(&Monomial::one());
        if cn.is_zero() || c0.is_zero() {
            break;
        }

        if let Some(r) = crate::algebra::rational_roots::find_rational_root(&current, var) {
            roots.push(r.clone());
            let divisor = &Poly::from_var(var) - &Poly::from_const(r);
            if let Ok((q, _)) = current.div_rem(&divisor) {
                current = q.primitive_part();
                if current.leading_coeff().is_negative() {
                    current = -&current;
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // 3. Degree 2: quadratic ax^2 + bx + c
    if current.degree_of(var) == 2 {
        let a = current.coeff_of(&Monomial::single(var, 2));
        let b = current.coeff_of(&Monomial::single(var, 1));
        let c = current.coeff_of(&Monomial::one());
        let disc = &b * &b - &a * &c * RBig::from(4);
        if disc.is_zero() {
            let r = -&b / (&a * RBig::from(2));
            roots.push(r);
            current = Poly::from_const(1);
        } else if let Some(sqrt_disc) = exact_rational_sqrt(&disc) {
            let two_a = &a * RBig::from(2);
            let r1 = (-&b - &sqrt_disc) / &two_a;
            let r2 = (-&b + &sqrt_disc) / &two_a;
            roots.push(r1);
            roots.push(r2);
            current = Poly::from_const(1);
        }
    } else if current.degree_of(var) == 1 {
        // 4. Degree 1: ax + b
        let a = current.coeff_of(&Monomial::single(var, 1));
        let b = current.coeff_of(&Monomial::one());
        if !a.is_zero() {
            let r = -&b / &a;
            roots.push(r);
            current = Poly::from_const(1);
        }
    }

    let remaining = if current.degree_of(var) > 0 {
        Some(current)
    } else {
        None
    };

    roots.sort();
    roots.dedup();
    (roots, remaining)
}

/// Solves denominator polynomials and extracts all exact domain restrictions.
pub fn solve_poly_restrictions(poly: &Poly) -> Vec<DomainRestriction> {
    if poly.is_constant() {
        return Vec::new();
    }

    let mut restrictions = Vec::new();

    // 1. Extract monomial GCD factors (e.g. x^2 * y -> x != 0, y != 0)
    let m_gcd = poly.monomial_gcd();
    for var in m_gcd.variables() {
        if m_gcd.degree_of(&var) > 0 {
            restrictions.push(DomainRestriction::var_not_equal(var, RBig::ZERO));
        }
    }

    let cofactor = if !m_gcd.is_one() {
        let m_poly = Poly::from_term(RBig::ONE, m_gcd);
        match poly.div_rem(&m_poly) {
            Ok((q, _)) => q,
            Err(_) => poly.clone(),
        }
    } else {
        poly.clone()
    };

    if cofactor.is_constant() {
        restrictions.sort();
        restrictions.dedup();
        return restrictions;
    }

    let vars = cofactor.variables();
    if vars.len() == 1 {
        let var = vars.into_iter().next().unwrap();
        let (roots, remaining) = solve_univariate_rational_roots(&cofactor, &var);
        let found_roots = !roots.is_empty();
        for r in roots {
            restrictions.push(DomainRestriction::var_not_equal(&var, r));
        }
        if let Some(rem) = remaining {
            // What is left after dividing off rational roots and has no real zero excludes
            // nothing that the roots do not already exclude.
            if found_roots && has_no_real_roots(&rem) {
                restrictions.sort();
                restrictions.dedup();
                return restrictions;
            }
            // (w^2 + 4)^2 is zero exactly where w^2 + 4 is.
            let rem = crate::algebra::rational_roots::square_free(&rem, &var).unwrap_or(rem);
            let mut p = rem.primitive_part();
            if p.leading_coeff().is_negative() {
                p = -&p;
            }
            restrictions.push(DomainRestriction::poly_not_zero(p));
        }
    } else {
        // Multivariate irreducible factor
        let mut p = cofactor.primitive_part();
        if p.leading_coeff().is_negative() {
            p = -&p;
        }
        restrictions.push(DomainRestriction::poly_not_zero(p));
    }

    restrictions.sort();
    restrictions.dedup();
    restrictions
}

/// Whether a univariate polynomial provably has no real zero (Sturm's theorem).
pub fn has_no_real_roots(poly: &Poly) -> bool {
    if !poly.is_univariate() || poly.is_constant() {
        return false;
    }
    let var = poly.main_variable().unwrap();
    let derivative = |p: &Poly| -> Poly {
        Poly::from_terms(
            p.terms
                .iter()
                .filter_map(|t| {
                    let e = t.monomial.degree_of(&var);
                    (e > 0).then(|| {
                        Term::new(&t.coeff * RBig::from(e), Monomial::single(&var, e - 1))
                    })
                })
                .collect(),
        )
    };
    let mut seq = vec![poly.clone(), derivative(poly)];
    while !seq[seq.len() - 1].is_constant() {
        let n = seq.len();
        let Ok((_, rem)) = seq[n - 2].div_rem(&seq[n - 1]) else {
            return false;
        };
        if rem.is_zero() {
            break;
        }
        seq.push(-rem);
    }
    let sign_changes = |signs: Vec<bool>| signs.windows(2).filter(|w| w[0] != w[1]).count();
    let at_plus: Vec<bool> = seq.iter().map(|p| !p.leading_coeff().is_negative()).collect();
    let at_minus: Vec<bool> = seq
        .iter()
        .map(|p| !p.leading_coeff().is_negative() ^ (p.total_degree() % 2 == 1))
        .collect();
    sign_changes(at_minus) == sign_changes(at_plus)
}

/// Solves a collection of denominator AST nodes into canonical domain restrictions.
pub fn solve_denominator_roots(denominators: &[Node]) -> Vec<DomainRestriction> {
    let mut restrictions = Vec::new();
    for node in denominators {
        if let Ok(poly) = Poly::from_node(node) {
            restrictions.extend(solve_poly_restrictions(&poly));
        }
    }
    restrictions.sort();
    restrictions.dedup();
    restrictions
}

/// Checks if an AST node evaluates to an exact constant zero.
pub fn is_constant_zero(node: &Node) -> bool {
    let unwrapped = unwrap_group(node);
    match unwrapped {
        Node::Number { value, .. } => value.is_zero(),
        _ => {
            if let Ok(poly) = Poly::from_node(unwrapped) {
                poly.is_zero()
            } else {
                false
            }
        }
    }
}

/// Convenience entry point: scans an AST and extracts all domain restrictions,
/// immediately failing with `QuantusError::DivisionByZero` if any denominator is constant zero.
pub fn extract_domain_restrictions(node: &Node) -> Result<Vec<DomainRestriction>, QuantusError> {
    let raw = collect_raw_denominators(node);
    for den in &raw {
        if is_constant_zero(den) {
            return Err(QuantusError::other("Division by zero."));
        }
    }
    Ok(solve_denominator_roots(&raw))
}

/// Pipeline context for tracking domain restrictions across transformations and cancellations.
///
/// Ensures that when common factors (such as `(x - 1)` from `(x^2 - 1)/(x - 1)`) are cancelled,
/// their domain exclusions (`x != 1`) are retained in the final output.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DomainTracker {
    restrictions: Vec<DomainRestriction>,
}

impl DomainTracker {
    pub fn new() -> Self {
        Self {
            restrictions: Vec::new(),
        }
    }

    /// Initializes restrictions by scanning the initial unsimplified AST.
    /// Fails immediately if any denominator evaluates to constant zero.
    pub fn from_ast(node: &Node) -> Result<Self, QuantusError> {
        let restrictions = extract_domain_restrictions(node)?;
        Ok(Self { restrictions })
    }

    /// Scans an AST node, validates against division by zero, and merges newly discovered restrictions.
    pub fn scan_and_record(&mut self, node: &Node) -> Result<(), QuantusError> {
        let found = extract_domain_restrictions(node)?;
        self.add_all(found);
        Ok(())
    }

    /// Like `scan_and_record`, for denominators that only appear during simplification.
    ///
    /// A univariate factor without a real zero (such as `x^2 + x + 1`) is left out: it came from
    /// factoring a denominator and excludes no real number. Denominators of the input are
    /// recorded in full by `from_ast`.
    pub fn scan_and_record_derived(&mut self, node: &Node) -> Result<(), QuantusError> {
        let found = extract_domain_restrictions(node)?;
        self.add_all(found.into_iter().filter(|r| match r {
            DomainRestriction::PolyNotZero(p) => !has_no_real_roots(p),
            _ => true,
        }));
        Ok(())
    }

    /// Adds a single domain restriction, keeping the collection sorted and deduplicated.
    pub fn add(&mut self, restriction: DomainRestriction) {
        if !self.restrictions.contains(&restriction) {
            self.restrictions.push(restriction);
            self.restrictions.sort();
        }
    }

    /// Adds multiple domain restrictions, keeping the collection sorted and deduplicated.
    pub fn add_all(&mut self, restrictions: impl IntoIterator<Item = DomainRestriction>) {
        for r in restrictions {
            if !self.restrictions.contains(&r) {
                self.restrictions.push(r);
            }
        }
        self.restrictions.sort();
    }

    /// Explicitly records cancelled polynomial factors to preserve their domain restrictions.
    pub fn record_cancellation(&mut self, cancelled_factor: &Poly) {
        // A cancelled factor without a real zero excludes no real number; if it was a factor of
        // the input denominator, the initial scan has recorded it already.
        let restrictions = solve_poly_restrictions(cancelled_factor).into_iter().filter(|r| match r {
            DomainRestriction::PolyNotZero(p) => !has_no_real_roots(p),
            _ => true,
        });
        self.add_all(restrictions);
    }

    /// Explicitly records cancelled AST node factors.
    pub fn record_cancellation_node(&mut self, factor_node: &Node) {
        if let Ok(poly) = Poly::from_node(factor_node) {
            self.record_cancellation(&poly);
        }
    }

    pub fn restrictions(&self) -> &[DomainRestriction] {
        &self.restrictions
    }

    pub fn into_restrictions(self) -> Vec<DomainRestriction> {
        self.restrictions
    }

    pub fn format_latex(&self) -> String {
        format_restrictions_latex(&self.restrictions)
    }

    pub fn format_plain(&self) -> String {
        format_restrictions_plain(&self.restrictions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inequalities_are_written_and_merged() {
        let a = Poly::from_var("a");
        let b = Poly::from_var("b");
        let restrictions = vec![
            DomainRestriction::var_not_equal("a", RBig::ZERO),
            DomainRestriction::PolyNonNegative(a.clone()),
            DomainRestriction::PolyNonNegative(b.clone()),
            DomainRestriction::PolyPositive(b),
        ];
        let merged = merge_inequalities(restrictions);
        assert_eq!(format_restrictions_plain(&merged), "a > 0, b > 0");
        assert_eq!(format_restrictions_latex(&[DomainRestriction::PolyNonNegative(a)]), "a \\geq 0");
    }
}
