//! Finding common denominators and combining fractions (Rule B).
//!
//! Handles:
//! - Flattening sums and differences of rational expressions `f_1 ± f_2 ± ... ± f_k`.
//! - Factoring unfactored denominators into `FactoredPoly` (`FACTOR_DENOMINATORS`).
//! - Computing least common denominator (`LCD`) via `FactoredPoly::lcm`.
//! - Multiplying numerators by missing LCD factors and combining over LCD (`COMBINE_FRACTIONS`).

use crate::algebra::domain::unwrap_group;
use crate::algebra::binomial::factor_binomial;
use crate::algebra::factor::{factoring_lines, line_steps};
use crate::algebra::factored::FactoredPoly;
use crate::algebra::nested::{is_constant_one, make_fraction, make_product, split_fraction, wrap_factor};
use crate::algebra::poly::Poly;
use crate::algebra::steps::{AlgebraStep, RuleId};
use crate::latex::ast::{Node, Operator};
use crate::num::Decimal;

/// Represents a single term in a sum/difference of rational expressions.
#[derive(Clone, Debug)]
pub struct RationalTerm {
    pub is_positive: bool,
    pub numerator: Node,
    pub denominator_node: Node,
    pub factored_den: FactoredPoly,
}

/// Recursively flattens an additive subtree into signed `(is_positive, node)` terms.
pub fn flatten_additive_terms(node: &Node, is_positive: bool, terms: &mut Vec<(bool, Node)>) {
    let unwrapped = unwrap_group(node);
    match unwrapped {
        Node::Binary {
            operator: Operator::Plus,
            left,
            right,
            ..
        } => {
            flatten_additive_terms(left, is_positive, terms);
            flatten_additive_terms(right, is_positive, terms);
        }
        Node::Binary {
            operator: Operator::Minus,
            left,
            right,
            ..
        } => {
            flatten_additive_terms(left, is_positive, terms);
            flatten_additive_terms(right, !is_positive, terms);
        }
        Node::Negation { operand, .. } => {
            flatten_additive_terms(operand, !is_positive, terms);
        }
        _ => {
            terms.push((is_positive, node.clone()));
        }
    }
}

/// Converts a denominator AST node to a `FactoredPoly`.
pub fn den_to_factored(den: &Node) -> FactoredPoly {
    if is_constant_one(den) {
        return FactoredPoly::one();
    }
    if let Ok(poly) = Poly::from_node(den) {
        FactoredPoly::from_poly(poly)
    } else {
        FactoredPoly::one()
    }
}

/// Checks if a denominator is factorable and not yet in factored form.
pub fn is_unfactored_denominator(den: &Node) -> bool {
    let unwrapped = unwrap_group(den);
    if let Ok(poly) = Poly::from_node(unwrapped) {
        if poly.total_degree() >= 2 {
            let factored = FactoredPoly::from_poly(poly);
            if factored.factors.len() > 1 || (factored.factors.len() == 1 && factored.factors[0].exp > 1) {
                // Denominator is factorable into multiple factors.
                // Check if it's already a product in the AST.
                if !matches!(unwrapped, Node::Binary { operator: Operator::ImplicitTimes | Operator::Times, .. } | Node::Power { .. }) {
                    return true;
                }
            }
        }
    }
    false
}

/// Checks whether an AST node is a sum or difference of rational expressions with at least one fraction.
pub fn is_combinable_fraction_sum(node: &Node) -> bool {
    let unwrapped = unwrap_group(node);
    if !matches!(unwrapped, Node::Binary { operator: Operator::Plus | Operator::Minus, .. }) {
        return false;
    }
    let mut raw_terms = Vec::new();
    flatten_additive_terms(node, true, &mut raw_terms);
    if raw_terms.len() < 2 {
        return false;
    }
    raw_terms.iter().any(|(_, t)| split_fraction(t).is_some())
}

/// Checks if any denominator in `node` is unfactored and factorable, returning a step rewriting them.
pub fn factor_denominators_in_node(node: &Node) -> Option<(Node, AlgebraStep)> {
    let mut step_info: Option<AlgebraStep> = None;
    let new_node = factor_den_recursive(node, &mut step_info);
    step_info.map(|step| (new_node, step))
}

fn factor_den_recursive(node: &Node, step_info: &mut Option<AlgebraStep>) -> Node {
    if step_info.is_some() {
        return node.clone();
    }

    let unwrapped = unwrap_group(node);
    if let Some((num, den)) = split_fraction(unwrapped) {
        if is_unfactored_denominator(den) {
            let poly = Poly::from_node(den).unwrap();
            let factored = FactoredPoly::from_poly(poly.clone());
            let factored_den_node = factored.to_node();
            let new_frac = make_fraction(num.clone(), factored_den_node);
            let explanation = match factor_binomial(&poly) {
                Some((_, identity)) => format!("Factor the denominator using binomial identity: {}", identity.description()),
                None => "Factor polynomial denominator to find common factors".to_string(),
            };
            let (lines, last) = factoring_lines(&poly);
            let lines = lines.into_iter().map(|line| line.placed(|part| make_fraction(num.clone(), part))).collect();
            let step = AlgebraStep::new(node.clone(), new_frac.clone(), explanation, Some(RuleId::FactorDenominators));
            *step_info = Some(step.with_working(line_steps(node, lines), last));
            return new_frac;
        }
    }

    match node {
        Node::Binary {
            operator,
            left,
            right,
            position,
        } => {
            let new_left = factor_den_recursive(left, step_info);
            if step_info.is_some() {
                return Node::Binary {
                    operator: *operator,
                    left: Box::new(new_left),
                    right: right.clone(),
                    position: *position,
                };
            }
            let new_right = factor_den_recursive(right, step_info);
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
            let new_num = factor_den_recursive(numerator, step_info);
            if step_info.is_some() {
                return Node::Fraction {
                    numerator: Box::new(new_num),
                    denominator: denominator.clone(),
                    position: *position,
                };
            }
            let new_den = factor_den_recursive(denominator, step_info);
            Node::Fraction {
                numerator: Box::new(new_num),
                denominator: Box::new(new_den),
                position: *position,
            }
        }
        Node::Group { inner, position } => {
            let new_inner = factor_den_recursive(inner, step_info);
            Node::Group {
                inner: Box::new(new_inner),
                position: *position,
            }
        }
        Node::Negation { operand, position } => {
            let new_op = factor_den_recursive(operand, step_info);
            Node::Negation {
                operand: Box::new(new_op),
                position: *position,
            }
        }
        _ => node.clone(),
    }
}

/// Combines an additive expression of fractions at `node` into a single fraction over the LCD.
pub fn combine_fractions_at(node: &Node) -> Option<(Node, RuleId, String)> {
    if !is_combinable_fraction_sum(node) {
        return None;
    }

    let mut raw_terms = Vec::new();
    flatten_additive_terms(node, true, &mut raw_terms);

    let one_node = Node::Number {
        value: Decimal::from_i64(1),
        position: 0,
    };

    let mut rational_terms = Vec::new();
    let mut lcd = FactoredPoly::one();

    for (is_positive, term_node) in raw_terms {
        let (num, den) = if let Some((n, d)) = split_fraction(&term_node) {
            (n.clone(), d.clone())
        } else {
            (term_node, one_node.clone())
        };
        // `den_to_factored` reads a denominator that is not a polynomial (`\sqrt{a}`, `\sin(x)`) as 1,
        // which would silently drop it from the sum.
        if !is_constant_one(&den) && Poly::from_node(&den).is_err() {
            return None;
        }

        let fact_den = den_to_factored(&den);
        lcd = lcd.lcm(&fact_den);

        rational_terms.push(RationalTerm {
            is_positive,
            numerator: num,
            denominator_node: den,
            factored_den: fact_den,
        });
    }

    // Multiply each numerator by its missing LCD factors
    let mut scaled_numerator_terms: Vec<(bool, Node)> = Vec::new();

    for term in rational_terms {
        let missing = lcd.div(&term.factored_den).unwrap_or_else(FactoredPoly::one);
        let scaled_num = if missing.is_one() {
            // Keep numerator as is, wrap if it's an additive node to keep [ ... + (x - 102) ]
            term.numerator
        } else {
            let missing_node = missing.to_node();
            make_product(term.numerator, missing_node)
        };
        scaled_numerator_terms.push((term.is_positive, scaled_num));
    }

    if scaled_numerator_terms.is_empty() {
        return None;
    }

    // Build the combined numerator AST
    let (first_pos, first_node) = scaled_numerator_terms.remove(0);
    let mut combined_num = if first_pos {
        first_node
    } else {
        Node::Negation {
            operand: Box::new(wrap_factor(first_node)),
            position: 0,
        }
    };

    for (pos, term_node) in scaled_numerator_terms {
        let wrapped = wrap_factor(term_node);
        if pos {
            combined_num = Node::Binary {
                operator: Operator::Plus,
                left: Box::new(combined_num),
                right: Box::new(wrapped),
                position: 0,
            };
        } else {
            combined_num = Node::Binary {
                operator: Operator::Minus,
                left: Box::new(combined_num),
                right: Box::new(wrapped),
                position: 0,
            };
        }
    }

    let combined_den = lcd.to_node();
    let combined_fraction = make_fraction(combined_num, combined_den);

    let rule = RuleId::CombineFractions;
    let explanation = "Add the numerators over the common denominator".to_string();

    Some((combined_fraction, rule, explanation))
}

/// Recursively traverses `root` to locate and combine the next combinable fraction sum.
pub fn combine_next_fractions(root: &Node) -> Option<(Node, AlgebraStep)> {
    let mut step_info: Option<AlgebraStep> = None;
    let new_root = combine_recursive(root, &mut step_info);
    step_info.map(|step| (new_root, step))
}

fn combine_recursive(node: &Node, step_info: &mut Option<AlgebraStep>) -> Node {
    if step_info.is_some() {
        return node.clone();
    }

    // Check if current node is a combinable fraction sum
    if is_combinable_fraction_sum(node) {
        if let Some((combined, rule, explanation)) = combine_fractions_at(node) {
            *step_info = Some(AlgebraStep::new(node.clone(), combined.clone(), explanation, Some(rule)));
            return combined;
        }
    }

    match node {
        Node::Group { inner, position } => {
            let new_inner = combine_recursive(inner, step_info);
            Node::Group {
                inner: Box::new(new_inner),
                position: *position,
            }
        }
        Node::Negation { operand, position } => {
            let new_op = combine_recursive(operand, step_info);
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
            let new_left = combine_recursive(left, step_info);
            if step_info.is_some() {
                return Node::Binary {
                    operator: *operator,
                    left: Box::new(new_left),
                    right: right.clone(),
                    position: *position,
                };
            }
            let new_right = combine_recursive(right, step_info);
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
            let new_num = combine_recursive(numerator, step_info);
            if step_info.is_some() {
                return Node::Fraction {
                    numerator: Box::new(new_num),
                    denominator: denominator.clone(),
                    position: *position,
                };
            }
            let new_den = combine_recursive(denominator, step_info);
            Node::Fraction {
                numerator: Box::new(new_num),
                denominator: Box::new(new_den),
                position: *position,
            }
        }
        _ => node.clone(),
    }
}

/// Applies denominator factoring if needed, then combines fraction sums iteratively into a single fraction.
pub fn combine_all_fractions(root: &Node) -> (Node, Vec<AlgebraStep>) {
    let mut current = root.clone();
    let mut steps = Vec::new();

    // 1. Factor denominators where factorable
    while let Some((new_tree, step)) = factor_denominators_in_node(&current) {
        steps.push(step);
        current = new_tree;
    }

    // 2. Combine fraction sums
    let mut iterations = 0;
    const MAX_ITERATIONS: usize = 20;
    while iterations < MAX_ITERATIONS {
        iterations += 1;
        let before_tree = current.clone();
        if let Some((new_tree, local_step)) = combine_next_fractions(&current) {
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
