//! Domain restrictions extracted from denominators, fractional powers, and transformation steps.

use std::cmp::Ordering;
use std::fmt;

use dashu::base::{Abs, Signed};
use dashu::integer::UBig;
use dashu::rational::RBig;

use crate::algebra::poly::Poly;

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
