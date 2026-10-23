//! Rational expression reduction and factor cancellation (Rules E & G).
//!
//! Handles:
//! - Factoring numerators into `FactoredPoly` (`FACTOR_POLYNOMIAL` / `APPLY_BINOMIAL_IDENTITY`).
//! - Multiplicative common factor cancellation (`CANCEL_COMMON_FACTORS`), preserving domain restrictions.
//! - Strict multiplicative-only cancellation: additive terms across `+` or `-` are never cancelled.
//! - Special rational cases:
//!   * Zero numerators: `0 / D -> 0` (`NORMALIZE_EXPRESSION`)
//!   * Constant denominators: `P / 1 -> P` (`NORMALIZE_EXPRESSION`)
//!   * Equal numerator and denominator: `P / P -> 1` (`CANCEL_COMMON_FACTORS`)
//!   * Opposite signs: `(a - b) / (b - a) -> -1` (`CANCEL_COMMON_FACTORS`).

use dashu::rational::RBig;

use crate::algebra::binomial::factor_binomial_node;
use crate::algebra::combine::den_to_factored;
use crate::algebra::domain::{is_constant_zero, unwrap_group, DomainTracker};
use crate::algebra::nested::{is_constant_one, make_fraction, split_fraction};
use crate::algebra::poly::Poly;
use crate::algebra::steps::{AlgebraStep, RuleId};
use crate::latex::ast::Node;
use crate::latex::serializer;
use crate::num::Decimal;

/// Attempts to reduce a fraction node by factoring, cancellation, or special cases.
pub fn reduce_rational_node(
    node: &Node,
    mut tracker: Option<&mut DomainTracker>,
) -> Option<(Node, RuleId, String)> {
    let unwrapped = unwrap_group(node);
    let (num, den) = split_fraction(unwrapped)?;

    // 1. Zero numerator: 0 / D -> 0
    if is_constant_zero(num) {
        let zero_node = Node::Number {
            value: Decimal::from_i64(0),
            position: 0,
        };
        let rule = RuleId::NormalizeExpression;
        let explanation = "Zero divided by any non-zero expression is 0".to_string();
        return Some((zero_node, rule, explanation));
    }

    // 2. Denominator is 1: P / 1 -> P
    if is_constant_one(den) {
        let rule = RuleId::NormalizeExpression;
        let explanation = "Division by 1 leaves the expression unchanged".to_string();
        return Some((num.clone(), rule, explanation));
    }

    // 3. Polynomial equality: P / P -> 1
    if let (Ok(p_num), Ok(p_den)) = (Poly::from_node(num), Poly::from_node(den)) {
        if !p_num.is_zero() && p_num == p_den {
            if let Some(t) = tracker.as_deref_mut() {
                t.record_cancellation(&p_num);
            }
            let one_node = Node::Number {
                value: Decimal::from_i64(1),
                position: 0,
            };
            let rule = RuleId::CancelCommonFactors;
            let explanation = "Cancel identical numerator and denominator to get 1".to_string();
            return Some((one_node, rule, explanation));
        }

        // 4. Opposite signs: (a - b) / (b - a) -> -1
        if !p_num.is_zero() && p_num == -&p_den {
            if let Some(t) = tracker.as_deref_mut() {
                t.record_cancellation(&p_den);
            }
            let neg_one_node = Node::Number {
                value: Decimal::from_i64(-1),
                position: 0,
            };
            let rule = RuleId::CancelCommonFactors;
            let explanation = "Cancel opposite terms to get -1".to_string();
            return Some((neg_one_node, rule, explanation));
        }
    }

    // 5. Multiplicative cancellation via FactoredPoly GCD
    let fact_den = den_to_factored(den);
    let fact_num = den_to_factored(num).split_by(&fact_den);

    if fact_num.is_zero() || fact_den.is_zero() {
        return None;
    }

    let gcd = fact_num.gcd(&fact_den);
    let has_common_factors = !gcd.factors.is_empty() || gcd.coeff != RBig::ONE;

    if !has_common_factors {
        return None;
    }

    // Check if the numerator or denominator in the AST needs to be factored first
    let num_is_unfactored = if fact_num.factors.len() > 1 || (fact_num.factors.len() == 1 && fact_num.factors[0].exp > 1) {
        let mut num_unwrapped = unwrap_group(num);
        while let Node::Negation { operand, .. } = num_unwrapped {
            num_unwrapped = unwrap_group(operand);
        }
        !matches!(num_unwrapped, Node::Binary { operator: crate::latex::ast::Operator::ImplicitTimes | crate::latex::ast::Operator::Times, .. } | Node::Power { .. })
    } else {
        false
    };

    if num_is_unfactored {
        // First emit factoring step
        let factored_num_node = if let Some(step) = factor_binomial_node(num) {
            step.after
        } else {
            fact_num.to_node()
        };

        let new_frac = make_fraction(factored_num_node, den.clone());
        let rule = if factor_binomial_node(num).is_some() {
            RuleId::ApplyBinomialIdentity
        } else {
            RuleId::FactorPolynomial
        };
        let explanation = format!(
            "Factor numerator to identify common factors: {}",
            serializer::expression(&fact_num.to_node())
        );
        return Some((new_frac, rule, explanation));
    }

    // Perform factor cancellation
    let new_fact_num = fact_num.div(&gcd).unwrap();
    let new_fact_den = fact_den.div(&gcd).unwrap();

    // Record cancelled factors in domain tracker
    if let Some(t) = tracker {
        for f in &gcd.factors {
            t.record_cancellation(&f.base);
        }
    }

    let res_node = if new_fact_den.is_one() {
        new_fact_num.to_node()
    } else {
        make_fraction(new_fact_num.to_node(), new_fact_den.to_node())
    };

    let rule = RuleId::CancelCommonFactors;
    let explanation = format!(
        "Cancel common factor {} from numerator and denominator",
        serializer::expression(&gcd.to_node())
    );

    Some((res_node, rule, explanation))
}

/// Recursively traverses `root` to locate and perform the next reduction step.
pub fn reduce_next_rational(
    root: &Node,
    tracker: Option<&mut DomainTracker>,
) -> Option<(Node, AlgebraStep)> {
    let mut step_info: Option<AlgebraStep> = None;
    let new_root = reduce_recursive(root, &mut step_info, tracker);
    step_info.map(|step| (new_root, step))
}

fn reduce_recursive(
    node: &Node,
    step_info: &mut Option<AlgebraStep>,
    mut tracker: Option<&mut DomainTracker>,
) -> Node {
    if step_info.is_some() {
        return node.clone();
    }

    // Try reducing current node if it's a fraction
    if split_fraction(unwrap_group(node)).is_some() {
        if let Some((reduced, rule, explanation)) =
            reduce_rational_node(node, tracker.as_deref_mut())
        {
            *step_info = Some(AlgebraStep::new(node.clone(), reduced.clone(), explanation, Some(rule)));
            return reduced;
        }
    }

    match node {
        Node::Fraction {
            numerator,
            denominator,
            position,
        } => {
            let new_num = reduce_recursive(numerator, step_info, tracker.as_deref_mut());
            if step_info.is_some() {
                return Node::Fraction {
                    numerator: Box::new(new_num),
                    denominator: denominator.clone(),
                    position: *position,
                };
            }
            let new_den = reduce_recursive(denominator, step_info, tracker.as_deref_mut());
            Node::Fraction {
                numerator: Box::new(new_num),
                denominator: Box::new(new_den),
                position: *position,
            }
        }
        Node::Binary {
            operator,
            left,
            right,
            position,
        } => {
            let new_left = reduce_recursive(left, step_info, tracker.as_deref_mut());
            if step_info.is_some() {
                return Node::Binary {
                    operator: *operator,
                    left: Box::new(new_left),
                    right: right.clone(),
                    position: *position,
                };
            }
            let new_right = reduce_recursive(right, step_info, tracker.as_deref_mut());
            Node::Binary {
                operator: *operator,
                left: Box::new(new_left),
                right: Box::new(new_right),
                position: *position,
            }
        }
        Node::Group { inner, position } => {
            let new_inner = reduce_recursive(inner, step_info, tracker);
            Node::Group {
                inner: Box::new(new_inner),
                position: *position,
            }
        }
        Node::Negation { operand, position } => {
            let new_op = reduce_recursive(operand, step_info, tracker);
            Node::Negation {
                operand: Box::new(new_op),
                position: *position,
            }
        }
        _ => node.clone(),
    }
}

/// Repeatedly reduces rational expressions until no more cancellations or special cases apply.
pub fn reduce_all_rationals(
    root: &Node,
    mut tracker: Option<&mut DomainTracker>,
) -> (Node, Vec<AlgebraStep>) {
    let mut current = root.clone();
    let mut steps = Vec::new();
    let mut iterations = 0;
    const MAX_ITERATIONS: usize = 20;

    while iterations < MAX_ITERATIONS {
        iterations += 1;
        let before_tree = current.clone();
        if let Some((new_tree, local_step)) =
            reduce_next_rational(&current, tracker.as_deref_mut())
        {
            let full_step = AlgebraStep::new(
                before_tree,
                new_tree.clone(),
                local_step.explanation,
                local_step.rule,
            );
            steps.push(full_step);
            current = new_tree;
        } else {
            break;
        }
    }

    (current, steps)
}
