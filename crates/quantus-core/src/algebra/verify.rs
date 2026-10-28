//! Mathematical equivalence verification for algebraic simplification (Step 5.3).
//!
//! Verifies that an algebraic transformation or simplification result preserves mathematical
//! equivalence across multiple deterministic rational sample points for all free variables.
//!
//! Sample points that violate domain restrictions or cause any denominator to evaluate to zero
//! are automatically detected and skipped.

use std::collections::BTreeMap;

use dashu::base::{Abs, Signed};
use dashu::integer::{IBig, UBig};
use dashu::rational::RBig;

use crate::algebra::domain::DomainRestriction;
use crate::algebra::pipeline::collect_variables;
use crate::algebra::poly::Poly;
use crate::latex::ast::{Node, Operator};

/// Result of equivalence verification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerificationResult {
    /// Expressions were verified to be equivalent across `samples_checked` valid test points.
    Equivalent { samples_checked: usize },
    /// Verification failed at a specific test point with unequal evaluations.
    Mismatch {
        point: BTreeMap<String, RBig>,
        original_val: RBig,
        simplified_val: RBig,
    },
    /// No valid sample points could be evaluated (e.g. all points violated domain restrictions).
    Inconclusive { reason: String },
}

impl VerificationResult {
    pub fn is_equivalent(&self) -> bool {
        matches!(self, Self::Equivalent { .. })
    }
}

/// Evaluates an AST `node` over exact rational arithmetic `RBig` given variable assignments.
/// Returns `None` if evaluation encounters division by zero, non-integer powers, or unsupported nodes.
pub fn eval_rational(node: &Node, env: &BTreeMap<String, RBig>) -> Option<RBig> {
    match node {
        Node::Number { value, .. } => Some(value.to_rational()),
        Node::Variable { name, index, .. } => {
            let key = Node::variable_key(name, index);
            env.get(&key).cloned()
        }
        Node::Group { inner, .. } => eval_rational(inner, env),
        Node::Negation { operand, .. } => eval_rational(operand, env).map(|v| -v),
        Node::Binary { operator, left, right, .. } => {
            let l = eval_rational(left, env)?;
            let r = eval_rational(right, env)?;
            match operator {
                Operator::Plus => Some(l + r),
                Operator::Minus => Some(l - r),
                Operator::Times | Operator::ImplicitTimes => Some(l * r),
                Operator::Divide => {
                    if r.is_zero() {
                        None
                    } else {
                        Some(l / r)
                    }
                }
            }
        }
        Node::Fraction { numerator, denominator, .. } => {
            let num = eval_rational(numerator, env)?;
            let den = eval_rational(denominator, env)?;
            if den.is_zero() {
                None
            } else {
                Some(num / den)
            }
        }
        Node::Power { base, exponent, .. } => {
            let b = eval_rational(base, env)?;
            let exp_val = eval_rational(exponent, env)?;
            if exp_val.denominator() != &UBig::from(1u8) {
                return None;
            }
            let num = exp_val.numerator();
            let is_neg = num.is_negative();
            let abs_num = num.clone().abs();
            let exp_usize = usize::try_from(abs_num).ok()?;

            let mut res = RBig::ONE;
            let mut cur = b;
            let mut e = exp_usize;
            while e > 0 {
                if e % 2 == 1 {
                    res *= &cur;
                }
                if e > 1 {
                    cur = &cur * &cur;
                }
                e /= 2;
            }

            if is_neg {
                if res.is_zero() {
                    None
                } else {
                    Some(RBig::ONE / res)
                }
            } else {
                Some(res)
            }
        }
        _ => None,
    }
}

/// Evaluates a `Poly` given variable rational assignments.
pub fn eval_poly(poly: &Poly, env: &BTreeMap<String, RBig>) -> RBig {
    let mut sum = RBig::ZERO;
    for term in &poly.terms {
        let mut term_val = term.coeff.clone();
        for (var, &deg) in &term.monomial.vars {
            let var_val = env.get(var).cloned().unwrap_or(RBig::ZERO);
            for _ in 0..deg {
                term_val *= &var_val;
            }
        }
        sum += term_val;
    }
    sum
}

/// Checks if an environment assignment violates any known domain restrictions.
pub fn violates_restrictions(env: &BTreeMap<String, RBig>, restrictions: &[DomainRestriction]) -> bool {
    for restriction in restrictions {
        match restriction {
            DomainRestriction::VariableNotEqual { variable, value } => {
                if let Some(env_val) = env.get(variable) {
                    if env_val == value {
                        return true;
                    }
                }
            }
            DomainRestriction::PolyNotZero(p) => {
                let p_val = eval_poly(p, env);
                if p_val.is_zero() {
                    return true;
                }
            }
            DomainRestriction::PolyNonNegative(p) => {
                if eval_poly(p, env) < RBig::ZERO {
                    return true;
                }
            }
            DomainRestriction::PolyPositive(p) => {
                if eval_poly(p, env) <= RBig::ZERO {
                    return true;
                }
            }
        }
    }
    false
}

/// Generates a set of deterministic rational candidate values for testing.
fn default_candidate_values() -> Vec<RBig> {
    vec![
        RBig::from(2),
        RBig::from(-2),
        RBig::from(3),
        RBig::from(-3),
        RBig::from(5),
        RBig::from(-5),
        RBig::from(7),
        RBig::from_parts(IBig::from(1), UBig::from(2u8)),
        RBig::from_parts(IBig::from(-1), UBig::from(2u8)),
        RBig::from_parts(IBig::from(2), UBig::from(3u8)),
        RBig::from_parts(IBig::from(-3), UBig::from(4u8)),
        RBig::from_parts(IBig::from(5), UBig::from(2u8)),
        RBig::from(11),
        RBig::from(-11),
    ]
}

/// Generates multi-variable rational test point environments.
fn generate_sample_points(vars: &[String], max_samples: usize) -> Vec<BTreeMap<String, RBig>> {
    let candidates = default_candidate_values();
    let num_vars = vars.len();
    if num_vars == 0 {
        return vec![BTreeMap::new()];
    }

    let mut points = Vec::new();

    // Strategy 1: Staggered offsets across candidates
    for i in 0..candidates.len() {
        if points.len() >= max_samples {
            break;
        }
        let mut point = BTreeMap::new();
        for (v_idx, var) in vars.iter().enumerate() {
            let cand_idx = (i + v_idx * 3) % candidates.len();
            point.insert(var.clone(), candidates[cand_idx].clone());
        }
        points.push(point);
    }

    // Strategy 2: Permutations / different combinations if more samples needed
    if points.len() < max_samples && num_vars >= 2 {
        for c1 in candidates.iter().take(5) {
            for c2 in candidates.iter().take(5) {
                if points.len() >= max_samples {
                    break;
                }
                let mut point = BTreeMap::new();
                for (v_idx, var) in vars.iter().enumerate() {
                    let val = if v_idx % 2 == 0 {
                        c1.clone() + RBig::from(v_idx as i64)
                    } else {
                        c2.clone() - RBig::from(v_idx as i64)
                    };
                    point.insert(var.clone(), val);
                }
                if !points.contains(&point) {
                    points.push(point);
                }
            }
        }
    }

    points
}

/// Verifies that `original` and `simplified` evaluate to identical rational values across
/// multiple deterministic test points.
///
/// Automatically skips sample points where:
/// - Any domain restriction is violated.
/// - Any denominator in either expression evaluates to zero.
///
/// Returns `VerificationResult`.
pub fn verify_equivalence(
    original: &Node,
    simplified: &Node,
    domain_restrictions: &[DomainRestriction],
    num_samples_target: usize,
) -> VerificationResult {
    let mut vars_set = collect_variables(original);
    vars_set.extend(collect_variables(simplified));
    let vars: Vec<String> = vars_set.into_iter().collect();

    let candidate_points = generate_sample_points(&vars, num_samples_target.max(5) * 4);
    let mut successful_checks = 0;

    for point in candidate_points {
        if violates_restrictions(&point, domain_restrictions) {
            continue;
        }

        let orig_val = match eval_rational(original, &point) {
            Some(v) => v,
            None => continue, // Hit a zero denominator or unhandled evaluation
        };

        let simp_val = match eval_rational(simplified, &point) {
            Some(v) => v,
            None => continue, // Hit a zero denominator or unhandled evaluation
        };

        if orig_val != simp_val {
            return VerificationResult::Mismatch {
                point,
                original_val: orig_val,
                simplified_val: simp_val,
            };
        }

        successful_checks += 1;
        if successful_checks >= num_samples_target {
            break;
        }
    }

    if successful_checks > 0 || vars.is_empty() {
        VerificationResult::Equivalent {
            samples_checked: successful_checks,
        }
    } else {
        VerificationResult::Inconclusive {
            reason: "Could not find valid non-zero denominator sample points within candidate bounds.".to_string(),
        }
    }
}
