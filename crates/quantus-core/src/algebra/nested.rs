//! Simplification of complex and nested fractions (Rule A).
//!
//! Handles:
//! - `a / (b / c) -> (a * c) / b` (`RuleId::ApplyReciprocalRule`)
//! - `(a / b) / c -> a / (b * c)` (`RuleId::SimplifyNestedFraction`)
//! - `(a / b) / (c / d) -> (a * d) / (b * c)` (`RuleId::ApplyReciprocalRule`)
//! - Sums/differences of fractions in numerators or denominators: combined first, then reciprocal applied.
//! - Preservation of factored structure: e.g. `3 / ((x + 3)/(x - 2)) -> 3(x - 2)/(x + 3)`
//!   and `((x - 102)/(x - 4))/(x + 3) -> (x - 102)/((x - 4)(x + 3))`.

use dashu::rational::RBig;

use crate::algebra::domain::unwrap_group;
use crate::algebra::poly::Poly;
use crate::algebra::steps::{AlgebraStep, RuleId};
use crate::latex::ast::{Node, Operator};
use crate::latex::serializer;
use crate::num::Decimal;
use crate::settings::StepDetail;

/// Helper to wrap an additive or negation node in `Node::Group` when needed as a multiplicative factor.
pub fn wrap_factor(node: Node) -> Node {
    let needs_group = match unwrap_group(&node) {
        Node::Binary { operator, .. } => matches!(operator, Operator::Plus | Operator::Minus),
        Node::Negation { .. } => true,
        _ => false,
    };
    if needs_group && !matches!(node, Node::Group { .. }) {
        Node::Group {
            inner: Box::new(node),
            position: 0,
        }
    } else {
        node
    }
}

/// Checks whether an AST node is a number/poly representing constant 1.
pub fn is_constant_one(node: &Node) -> bool {
    let unwrapped = unwrap_group(node);
    if let Node::Number { value, .. } = unwrapped {
        return value.to_rational() == RBig::ONE;
    }
    if let Ok(poly) = Poly::from_node(unwrapped) {
        return poly.is_one();
    }
    false
}

/// Checks whether an AST node is a number/poly representing constant -1.
pub fn is_constant_neg_one(node: &Node) -> bool {
    let unwrapped = unwrap_group(node);
    if let Node::Number { value, .. } = unwrapped {
        return value.to_rational() == -RBig::ONE;
    }
    if let Ok(poly) = Poly::from_node(unwrapped) {
        return poly.is_constant() && poly.leading_coeff() == -RBig::ONE;
    }
    false
}

/// Constructs a product AST node, preserving factors cleanly and simplifying 1 / -1 / 0.
pub fn make_product(left: Node, right: Node) -> Node {
    if is_constant_one(&left) {
        return right;
    }
    if is_constant_one(&right) {
        return left;
    }
    if crate::algebra::domain::is_constant_zero(&left) || crate::algebra::domain::is_constant_zero(&right) {
        return Node::Number {
            value: Decimal::from_i64(0),
            position: 0,
        };
    }
    if is_constant_neg_one(&left) {
        return Node::Negation {
            operand: Box::new(wrap_factor(right)),
            position: 0,
        };
    }
    if is_constant_neg_one(&right) {
        return Node::Negation {
            operand: Box::new(wrap_factor(left)),
            position: 0,
        };
    }

    // If both are numeric constants, multiply directly
    if let (Ok(pl), Ok(pr)) = (Poly::from_node(&left), Poly::from_node(&right)) {
        if pl.is_constant() && pr.is_constant() {
            let prod = &pl * &pr;
            return prod.to_node();
        }
    }

    let left_wrapped = wrap_factor(left);
    let right_wrapped = wrap_factor(right);

    Node::Binary {
        operator: Operator::ImplicitTimes,
        left: Box::new(left_wrapped),
        right: Box::new(right_wrapped),
        position: 0,
    }
}

/// Constructs a fraction AST node.
pub fn make_fraction(numerator: Node, denominator: Node) -> Node {
    Node::Fraction {
        numerator: Box::new(numerator),
        denominator: Box::new(denominator),
        position: 0,
    }
}

/// Splits a node if it represents a fraction (either `Node::Fraction` or `Node::Binary(Divide)`),
/// unwrapping outer groups.
pub fn split_fraction(node: &Node) -> Option<(&Node, &Node)> {
    let unwrapped = unwrap_group(node);
    match unwrapped {
        Node::Fraction {
            numerator,
            denominator,
            ..
        } => Some((numerator.as_ref(), denominator.as_ref())),
        Node::Binary {
            operator: Operator::Divide,
            left,
            right,
            ..
        } => Some((left.as_ref(), right.as_ref())),
        _ => None,
    }
}

/// Checks if an AST node contains any fraction.
pub fn has_fraction(node: &Node) -> bool {
    let unwrapped = unwrap_group(node);
    match unwrapped {
        Node::Fraction { .. } => true,
        Node::Binary {
            operator: Operator::Divide,
            ..
        } => true,
        Node::Binary { left, right, .. } => has_fraction(left) || has_fraction(right),
        Node::Negation { operand, .. } => has_fraction(operand),
        Node::Group { inner, .. } => has_fraction(inner),
        _ => false,
    }
}

/// Checks if an AST node is a sum/difference of terms where at least one term is a fraction.
pub fn is_fraction_sum(node: &Node) -> bool {
    let unwrapped = unwrap_group(node);
    match unwrapped {
        Node::Binary {
            operator: Operator::Plus | Operator::Minus,
            left,
            right,
            ..
        } => has_fraction(left) || has_fraction(right),
        _ => false,
    }
}

/// Combines a binary addition or subtraction containing fractions into a single fraction.
pub fn combine_simple_fraction_sum(node: &Node) -> Option<Node> {
    let unwrapped = unwrap_group(node);
    if let Node::Binary {
        operator,
        left,
        right,
        ..
    } = unwrapped
    {
        if *operator != Operator::Plus && *operator != Operator::Minus {
            return None;
        }

        let left_combined = if is_fraction_sum(left) {
            combine_simple_fraction_sum(left)?
        } else {
            left.as_ref().clone()
        };

        let right_combined = if is_fraction_sum(right) {
            combine_simple_fraction_sum(right)?
        } else {
            right.as_ref().clone()
        };

        let left_frac = split_fraction(&left_combined);
        let right_frac = split_fraction(&right_combined);

        if left_frac.is_none() && right_frac.is_none() {
            return None;
        }

        let one_node = Node::Number {
            value: Decimal::from_i64(1),
            position: 0,
        };

        let (n1, d1) = left_frac
            .map(|(n, d)| (n.clone(), d.clone()))
            .unwrap_or_else(|| (left_combined.clone(), one_node.clone()));

        let (n2, d2) = right_frac
            .map(|(n, d)| (n.clone(), d.clone()))
            .unwrap_or_else(|| (right_combined.clone(), one_node.clone()));

        // Check if denominators are equal
        let dens_equal = if let (Ok(p1), Ok(p2)) = (Poly::from_node(&d1), Poly::from_node(&d2)) {
            p1 == p2
        } else {
            unwrap_group(&d1) == unwrap_group(&d2)
        };

        let (combined_num, combined_den) = if dens_equal {
            let num = Node::Binary {
                operator: *operator,
                left: Box::new(n1),
                right: Box::new(wrap_factor(n2)),
                position: 0,
            };
            (num, d1)
        } else if is_constant_one(&d1) {
            let left_scaled = make_product(n1, d2.clone());
            let num = Node::Binary {
                operator: *operator,
                left: Box::new(left_scaled),
                right: Box::new(wrap_factor(n2)),
                position: 0,
            };
            (num, d2)
        } else if is_constant_one(&d2) {
            let right_scaled = make_product(n2, d1.clone());
            let num = Node::Binary {
                operator: *operator,
                left: Box::new(n1),
                right: Box::new(wrap_factor(right_scaled)),
                position: 0,
            };
            (num, d1)
        } else {
            let left_scaled = make_product(n1, d2.clone());
            let right_scaled = make_product(n2, d1.clone());
            let num = Node::Binary {
                operator: *operator,
                left: Box::new(left_scaled),
                right: Box::new(wrap_factor(right_scaled)),
                position: 0,
            };
            let den = make_product(d1, d2);
            (num, den)
        };

        return Some(make_fraction(combined_num, combined_den));
    }

    None
}

/// Checks whether `node` is directly a complex fraction that can be simplified.
pub fn is_nested_fraction(node: &Node) -> bool {
    let unwrapped = unwrap_group(node);
    if let Some((num, den)) = split_fraction(unwrapped) {
        if split_fraction(num).is_some() || split_fraction(den).is_some() {
            return true;
        }
        if is_fraction_sum(num) || is_fraction_sum(den) {
            return true;
        }
    }
    false
}

/// Attempts to simplify a complex fraction at `node`.
/// Returns `Some((simplified_node, rule, explanation))` if `node` was a complex fraction.
pub fn simplify_nested_fraction_at(node: &Node) -> Option<(Node, RuleId, String)> {
    let step = nested_fraction_step(node)?;
    Some((step.after, step.rule?, step.summary.unwrap_or(step.explanation)))
}

/// [`simplify_nested_fraction_at`] as a step with its working: inner sums written over one
/// denominator and combined, then the division written as a multiplication by the reciprocal.
fn nested_fraction_step(node: &Node) -> Option<AlgebraStep> {
    let (simplified, rule, explanation) = simplify_nested_fraction_parts(node)?;
    let (num, den) = split_fraction(unwrap_group(node))?;
    let mut lines: Vec<(Node, String)> = Vec::new();
    let (mut num_now, mut den_now) = (num.clone(), den.clone());
    for numerator_side in [true, false] {
        let side = if numerator_side { num } else { den };
        let Some(combined) = is_fraction_sum(side).then(|| combine_simple_fraction_sum(side)).flatten() else { continue };
        let place = |part: Node, num_now: &Node, den_now: &Node| {
            if numerator_side { make_fraction(part, den_now.clone()) } else { make_fraction(num_now.clone(), part) }
        };
        if let Some((over, text)) = write_over_denominator(side) {
            lines.push((place(over, &num_now, &den_now), text));
        }
        lines.push((place(combined.clone(), &num_now, &den_now), "Combine the inner fractions".to_string()));
        if numerator_side { num_now = combined } else { den_now = combined }
    }
    let last = if let Some((c, d)) = split_fraction(&den_now) {
        let reciprocal = make_fraction(d.clone(), c.clone());
        let text = format!(
            "Dividing by {} is multiplying by its reciprocal {}",
            serializer::expression(&den_now),
            serializer::expression(&reciprocal)
        );
        lines.push((Node::binary(Operator::Times, num_now, reciprocal, 0), text));
        "Multiply numerators and denominators"
    } else {
        "Multiply the denominators"
    };
    let mut before = node.clone();
    let working = lines
        .into_iter()
        .map(|(after, text)| {
            let line = AlgebraStep::line(&before, after.clone(), text, StepDetail::Standard);
            before = after;
            line
        })
        .collect();
    let step = AlgebraStep::new(node.clone(), simplified, explanation, Some(rule));
    Some(step.with_working(working, last))
}

/// `\frac{x}{x} + \frac{1}{x}` for `1 + \frac{1}{x}`: the one term without a denominator written
/// over the denominator of the other.
fn write_over_denominator(sum: &Node) -> Option<(Node, String)> {
    let Node::Binary { operator: operator @ (Operator::Plus | Operator::Minus), left, right, .. } = unwrap_group(sum) else {
        return None;
    };
    let over = |term: &Node, d: &Node| make_fraction(make_product(term.clone(), d.clone()), d.clone());
    let text = |term: &Node, d: &Node| format!("Write {} over the denominator {}", serializer::expression(term), serializer::expression(d));
    let (left_frac, right_frac) = (split_fraction(left), split_fraction(right));
    let (written, text) = match (left_frac, right_frac) {
        (None, Some((_, d))) => (Node::binary(*operator, over(left, d), (**right).clone(), 0), text(left, d)),
        (Some((_, d)), None) => (Node::binary(*operator, (**left).clone(), over(right, d), 0), text(right, d)),
        _ => return None,
    };
    Some((written, text))
}

fn simplify_nested_fraction_parts(node: &Node) -> Option<(Node, RuleId, String)> {
    let unwrapped = unwrap_group(node);
    let (num, den) = split_fraction(unwrapped)?;

    // If num or den is a sum of fractions, combine it first
    let num_combined = if is_fraction_sum(num) {
        combine_simple_fraction_sum(num).unwrap_or_else(|| num.clone())
    } else {
        num.clone()
    };

    let den_combined = if is_fraction_sum(den) {
        combine_simple_fraction_sum(den).unwrap_or_else(|| den.clone())
    } else {
        den.clone()
    };

    let num_frac = split_fraction(&num_combined);
    let den_frac = split_fraction(&den_combined);

    match (num_frac, den_frac) {
        // Case 1: (a / b) / (c / d) -> (a * d) / (b * c)
        (Some((a, b)), Some((c, d))) => {
            let new_num = make_product(a.clone(), d.clone());
            let new_den = make_product(b.clone(), c.clone());
            let res = if is_constant_one(&new_den) {
                new_num
            } else {
                make_fraction(new_num, new_den)
            };
            let rule = RuleId::ApplyReciprocalRule;
            let explanation = "Apply the reciprocal rule: divide fractions by multiplying by the reciprocal".to_string();
            Some((res, rule, explanation))
        }

        // Case 2: a / (b / c) -> (a * c) / b
        (None, Some((b, c))) => {
            let new_num = make_product(num_combined, c.clone());
            let new_den = b.clone();
            let res = if is_constant_one(&new_den) {
                new_num
            } else {
                make_fraction(new_num, new_den)
            };
            let rule = RuleId::ApplyReciprocalRule;
            let explanation = "Apply the reciprocal rule: divide by a fraction by multiplying by its reciprocal".to_string();
            Some((res, rule, explanation))
        }

        // Case 3: (a / b) / c -> a / (b * c)
        (Some((a, b)), None) => {
            let new_num = a.clone();
            let new_den = make_product(b.clone(), den_combined);
            let res = if is_constant_one(&new_den) {
                new_num
            } else {
                make_fraction(new_num, new_den)
            };
            let rule = RuleId::SimplifyNestedFraction;
            let explanation = "Simplify complex fraction by multiplying the denominators".to_string();
            Some((res, rule, explanation))
        }

        // Not a complex fraction
        (None, None) => None,
    }
}

/// Recursively traverses `root` bottom-up to locate and simplify the next complex fraction.
/// Returns the updated AST root and the recorded `AlgebraStep`.
pub fn simplify_next_nested_fraction(root: &Node) -> Option<(Node, AlgebraStep)> {
    let mut step_info: Option<AlgebraStep> = None;
    let new_root = transform_bottom_up(root, &mut step_info);
    step_info.map(|step| (new_root, step))
}

fn transform_bottom_up(node: &Node, step_info: &mut Option<AlgebraStep>) -> Node {
    if step_info.is_some() {
        return node.clone();
    }

    match node {
        Node::Group { inner, position } => {
            let new_inner = transform_bottom_up(inner, step_info);
            Node::Group {
                inner: Box::new(new_inner),
                position: *position,
            }
        }
        Node::Negation { operand, position } => {
            let new_op = transform_bottom_up(operand, step_info);
            Node::Negation {
                operand: Box::new(new_op),
                position: *position,
            }
        }
        Node::Binary {
            operator,
            left,
            right,
            position,
        } => {
            // First check children
            let new_left = transform_bottom_up(left, step_info);
            if step_info.is_some() {
                return Node::Binary {
                    operator: *operator,
                    left: Box::new(new_left),
                    right: right.clone(),
                    position: *position,
                };
            }
            let new_right = transform_bottom_up(right, step_info);
            if step_info.is_some() {
                return Node::Binary {
                    operator: *operator,
                    left: Box::new(new_left),
                    right: Box::new(new_right),
                    position: *position,
                };
            }

            // Now check self if division
            if *operator == Operator::Divide {
                let candidate = Node::Binary {
                    operator: *operator,
                    left: Box::new(new_left),
                    right: Box::new(new_right),
                    position: *position,
                };
                if let Some(step) = nested_fraction_step(&candidate) {
                    let simplified = step.after.clone();
                    *step_info = Some(step);
                    return simplified;
                }
                return candidate;
            }

            Node::Binary {
                operator: *operator,
                left: Box::new(new_left),
                right: Box::new(new_right),
                position: *position,
            }
        }
        Node::Fraction {
            numerator,
            denominator,
            position,
        } => {
            // First check children
            let new_num = transform_bottom_up(numerator, step_info);
            if step_info.is_some() {
                return Node::Fraction {
                    numerator: Box::new(new_num),
                    denominator: denominator.clone(),
                    position: *position,
                };
            }
            let new_den = transform_bottom_up(denominator, step_info);
            if step_info.is_some() {
                return Node::Fraction {
                    numerator: Box::new(new_num),
                    denominator: Box::new(new_den),
                    position: *position,
                };
            }

            // Now check self
            let candidate = Node::Fraction {
                numerator: Box::new(new_num),
                denominator: Box::new(new_den),
                position: *position,
            };
            if let Some(step) = nested_fraction_step(&candidate) {
                let simplified = step.after.clone();
                *step_info = Some(step);
                return simplified;
            }
            candidate
        }
        Node::Power {
            base,
            exponent,
            position,
        } => {
            let new_base = transform_bottom_up(base, step_info);
            if step_info.is_some() {
                return Node::Power {
                    base: Box::new(new_base),
                    exponent: exponent.clone(),
                    position: *position,
                };
            }
            let new_exp = transform_bottom_up(exponent, step_info);
            Node::Power {
                base: Box::new(new_base),
                exponent: Box::new(new_exp),
                position: *position,
            }
        }
        _ => node.clone(),
    }
}

/// Simplifies all nested fractions in `root` iteratively, recording each transformation step.
pub fn simplify_all_nested_fractions(root: &Node) -> (Node, Vec<AlgebraStep>) {
    let mut current = root.clone();
    let mut steps = Vec::new();
    let mut iterations = 0;
    const MAX_ITERATIONS: usize = 20;

    while iterations < MAX_ITERATIONS {
        iterations += 1;
        let before_tree = current.clone();
        if let Some((new_tree, local_step)) = simplify_next_nested_fraction(&current) {
            // Record full tree transition for the step
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
