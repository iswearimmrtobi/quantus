//! Domain restrictions extracted from denominators, fractional powers, and transformation steps.

use std::cmp::Ordering;
use std::fmt;

use dashu::base::{Abs, Signed};
use dashu::integer::UBig;
use dashu::rational::RBig;

use crate::algebra::poly::Poly;
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
            (DomainRestriction::VariableNotEqual { .. }, DomainRestriction::PolyNotZero(_)) => {
                Ordering::Less
            }
            (DomainRestriction::PolyNotZero(_), DomainRestriction::VariableNotEqual { .. }) => {
                Ordering::Greater
            }
            (DomainRestriction::PolyNotZero(p1), DomainRestriction::PolyNotZero(p2)) => {
                p1.cmp(p2)
            }
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
                            values.push(format_rbig_latex(value));
                            i += 1;
                            continue;
                        }
                    }
                    break;
                }
                parts.push(format!("{var_name} \\neq {}", values.join(", ")));
            }
            DomainRestriction::PolyNotZero(p) => {
                parts.push(format!("{} \\neq 0", p.to_latex()));
                i += 1;
            }
        }
    }
    parts.join(", ")
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
            DomainRestriction::PolyNotZero(p) => {
                parts.push(format!("{} != 0", p.to_plain_string()));
                i += 1;
            }
        }
    }
    parts.join(", ")
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
