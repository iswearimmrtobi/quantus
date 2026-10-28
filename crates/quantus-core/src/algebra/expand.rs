//! Polynomial expansion and combining like terms (Rules C & F).
//!
//! Handles:
//! - Binomial power expansions `(a ± b)^2`, `(a ± b)^3` (`EXPAND_BINOMIAL`).
//! - Multi-factor polynomial products expanded pair-by-pair (`EXPAND_POLYNOMIAL`),
//!   e.g. `3(x - 2)(x - 4) + (x - 102) -> 3(x^2 - 6x + 8) + x - 102`.
//! - Monomial / scalar distribution across parentheses (`EXPAND_POLYNOMIAL`),
//!   e.g. `3(x^2 - 6x + 8) -> 3x^2 - 18x + 24`.
//! - Collecting like terms and sorting in canonical descending degree order (`COLLECT_LIKE_TERMS`),
//!   e.g. `3x^2 - 18x + 24 + x - 102 -> 3x^2 - 17x - 78`.

use crate::algebra::binomial::expand_binomial_node;
use crate::algebra::combine::flatten_additive_terms;
use crate::algebra::domain::unwrap_group;
use crate::algebra::nested::wrap_factor;
use crate::algebra::poly::Poly;
use crate::algebra::display::{bracket, signed_terms_sum};
use crate::algebra::steps::{AlgebraStep, RuleId};
use crate::settings::StepDetail;
use crate::latex::ast::{Node, Operator};
use crate::latex::serializer;

/// Flattens a product node into its factors.
pub fn flatten_multiplicative_factors(node: &Node, factors: &mut Vec<Node>) {
    let unwrapped = unwrap_group(node);
    if let Node::Binary {
        operator: Operator::ImplicitTimes | Operator::Times,
        left,
        right,
        ..
    } = unwrapped
    {
        flatten_multiplicative_factors(left, factors);
        flatten_multiplicative_factors(right, factors);
    } else {
        factors.push(node.clone());
    }
}

/// Rebuilds a product AST node from a list of factors.
pub fn rebuild_product(mut factors: Vec<Node>) -> Node {
    if factors.is_empty() {
        return Node::Number {
            value: crate::num::Decimal::from_i64(1),
            position: 0,
        };
    }
    let mut prod = factors.remove(0);
    for next in factors {
        prod = Node::Binary {
            operator: Operator::ImplicitTimes,
            left: Box::new(prod),
            right: Box::new(wrap_factor(next)),
            position: 0,
        };
    }
    prod
}

/// Checks if a factor is an additive polynomial (i.e. contains `+` or `-` at top level).
fn is_additive_factor(node: &Node) -> bool {
    let unwrapped = unwrap_group(node);
    matches!(
        unwrapped,
        Node::Binary {
            operator: Operator::Plus | Operator::Minus,
            ..
        }
    )
}

/// Attempts to expand a single product or binomial power in `node`.
pub fn expand_product_node(node: &Node) -> Option<(Node, RuleId, String)> {
    let step = expand_product_step(node)?;
    Some((step.after, step.rule?, step.summary.unwrap_or(step.explanation)))
}

/// [`expand_product_node`] as a step with its working.
fn expand_product_step(node: &Node) -> Option<AlgebraStep> {
    let unwrapped = unwrap_group(node);

    // 1. Binomial power check first: (a ± b)^2 or (a ± b)^3
    if let Some(step) = expand_binomial_node(unwrapped) {
        let rule = step.rule.unwrap_or(RuleId::ExpandBinomial);
        return Some(AlgebraStep { rule: Some(rule), ..step.starting_from(node) });
    }

    // 2. Product expansion check
    if matches!(
        unwrapped,
        Node::Binary {
            operator: Operator::ImplicitTimes | Operator::Times,
            ..
        }
    ) {
        let mut factors = Vec::new();
        flatten_multiplicative_factors(unwrapped, &mut factors);

        if factors.len() < 2 {
            return None;
        }

        // Case A: Expand a pair of non-trivial additive polynomial factors (e.g. (x - 2)(x - 4))
        let additive_indices: Vec<usize> = factors
            .iter()
            .enumerate()
            .filter(|(_, f)| is_additive_factor(f) && Poly::from_node(f).is_ok_and(|p| p.total_degree() >= 1))
            .map(|(i, _)| i)
            .collect();

        if additive_indices.len() >= 2 {
            let i1 = additive_indices[0];
            let i2 = additive_indices[1];

            let f1 = factors[i1].clone();
            let f2 = factors[i2].clone();

            let p1 = Poly::from_node(&f1).ok()?;
            let p2 = Poly::from_node(&f2).ok()?;

            let prod_poly = &p1 * &p2;
            let prod_node = prod_poly.to_node();

            // Rebuild with new expanded factor
            let mut new_factors = Vec::new();
            for (idx, f) in factors.into_iter().enumerate() {
                if idx == i1 {
                    new_factors.push(wrap_factor(prod_node.clone()));
                } else if idx != i2 {
                    new_factors.push(f);
                }
            }

            let result_node = rebuild_product(new_factors.clone());
            let rule = RuleId::ExpandPolynomial;
            let explanation = format!(
                "Expand polynomial product {} to {}",
                serializer::expression(&rebuild_product(vec![f1.clone(), f2.clone()])),
                serializer::expression(&prod_node)
            );
            // The lines put in place of the expanded pair.
            let place = |pair: Node| {
                let mut factors = new_factors.clone();
                factors[i1] = wrap_factor(pair);
                if factors.len() == 1 { unwrap_group(&factors[0]).clone() } else { rebuild_product(factors) }
            };
            let (each, uncollected) = term_by_term(&f1, &f2)?;
            let working = vec![
                AlgebraStep::line(node, place(each.clone()), "Write each term of the first bracket times each term of the second", StepDetail::Detailed),
                AlgebraStep::line(&place(each), place(uncollected), "Multiply every term of the first bracket by every term of the second", StepDetail::Standard),
            ];
            let step = AlgebraStep::new(node.clone(), result_node, explanation, Some(rule));
            return Some(step.with_working(working, "Combine like terms"));
        }

        // Case B: Distribute a scalar or monomial multiplier across a sum factor
        // (e.g. 3 * (x^2 - 6x + 8) -> 3x^2 - 18x + 24)
        if additive_indices.len() == 1 {
            let add_idx = additive_indices[0];
            let add_factor = factors[add_idx].clone();
            let p_add = Poly::from_node(&add_factor).ok()?;

            // Find a scalar or monomial factor to distribute
            for (idx, f) in factors.iter().enumerate() {
                if idx != add_idx {
                    if let Ok(p_scalar) = Poly::from_node(f) {
                        let distributed = &p_scalar * &p_add;
                        let dist_node = distributed.to_node();
                        let f_scalar = f.clone();

                        let mut new_factors = Vec::new();
                        for (i, factor) in factors.into_iter().enumerate() {
                            if i == add_idx {
                                new_factors.push(dist_node.clone());
                            } else if i != idx {
                                new_factors.push(factor);
                            }
                        }

                        let result_node = rebuild_product(new_factors);
                        let rule = RuleId::ExpandPolynomial;
                        let explanation = format!(
                            "Distribute {} across ({}) to obtain {}",
                            serializer::expression(&f_scalar),
                            serializer::expression(&add_factor),
                            serializer::expression(&dist_node)
                        );
                        return Some(AlgebraStep::new(node.clone(), result_node, explanation, Some(rule)));
                    }
                }
            }
        }
    }

    None
}

/// `x \cdot x + x \cdot (-5) + 2 \cdot x + 2 \cdot (-5)` and `x^2 - 5x + 2x - 10` for
/// `(x + 2)(x - 5)`: every product of a term of `first` and a term of `second`, written out and
/// then each worked out.
fn term_by_term(first: &Node, second: &Node) -> Option<(Node, Node)> {
    let terms = |node: &Node| {
        let mut terms = Vec::new();
        flatten_additive_terms(node, true, &mut terms);
        terms.into_iter().map(|(positive, term)| if positive { term } else { Node::Negation { operand: Box::new(term), position: 0 } })
    };
    let mut products = Vec::new();
    let mut values = Vec::new();
    for a in terms(first) {
        for b in terms(second) {
            let product = Poly::from_node(&a).ok()? * Poly::from_node(&b).ok()?;
            let negative = product.leading_coeff() < dashu::rational::RBig::ZERO;
            values.push((!negative, if negative { (-product).to_node() } else { product.to_node() }));
            // A negative first factor in brackets: (-2) \cdot x.
            let first = if matches!(a, Node::Negation { .. }) { bracket(a.clone()) } else { a.clone() };
            products.push((true, Node::binary(Operator::Times, first, b, 0)));
        }
    }
    Some((signed_terms_sum(products)?, signed_terms_sum(values)?))
}

/// Recursively traverses `root` to locate and perform the next expansion step.
pub fn expand_polynomial_products_step(root: &Node) -> Option<(Node, AlgebraStep)> {
    let mut step_info: Option<AlgebraStep> = None;
    let new_root = expand_recursive(root, &mut step_info);
    step_info.map(|step| (new_root, step))
}

fn expand_recursive(node: &Node, step_info: &mut Option<AlgebraStep>) -> Node {
    if step_info.is_some() {
        return node.clone();
    }

    // Try expanding current node
    if let Some(step) = expand_product_step(node) {
        let expanded = step.after.clone();
        *step_info = Some(step);
        return expanded;
    }

    match node {
        Node::Fraction {
            numerator,
            denominator,
            position,
        } => {
            // Primarily expand numerator, keeping factored denominator intact
            let new_num = expand_recursive(numerator, step_info);
            Node::Fraction {
                numerator: Box::new(new_num),
                denominator: denominator.clone(),
                position: *position,
            }
        }
        Node::Binary {
            operator,
            left,
            right,
            position,
        } => {
            let new_left = expand_recursive(left, step_info);
            if step_info.is_some() {
                return Node::Binary {
                    operator: *operator,
                    left: Box::new(new_left),
                    right: right.clone(),
                    position: *position,
                };
            }
            let new_right = expand_recursive(right, step_info);
            Node::Binary {
                operator: *operator,
                left: Box::new(new_left),
                right: Box::new(new_right),
                position: *position,
            }
        }
        Node::Group { inner, position } => {
            let new_inner = expand_recursive(inner, step_info);
            Node::Group {
                inner: Box::new(new_inner),
                position: *position,
            }
        }
        Node::Negation { operand, position } => {
            let new_op = expand_recursive(operand, step_info);
            Node::Negation {
                operand: Box::new(new_op),
                position: *position,
            }
        }
        _ => node.clone(),
    }
}

/// Whether the polynomial `node` differs from its canonical form only in the order of its terms:
/// every term is already a single monomial written canonically, and no two of them combine.
pub(crate) fn is_term_reordering(node: &Node) -> bool {
    let Ok(poly) = Poly::from_node(unwrap_group(node)) else {
        return false;
    };
    let mut terms = Vec::new();
    flatten_additive_terms(node, true, &mut terms);
    terms.len() == poly.terms.len()
        && terms.iter().all(|(_, term)| {
            Poly::from_node(term).is_ok_and(|p| {
                p.terms.len() == 1 && serializer::expression(&p.to_node()) == serializer::expression(term)
            })
        })
}

/// The lines before like terms are collected: brackets after a minus sign resolved, like terms
/// ordered (Maximal) and put next to each other in brackets (Detailed), sums that cancel written
/// as 0 (Detailed).
fn collect_working(node: &Node) -> Vec<AlgebraStep> {
    let mut lines = Vec::new();
    let mut terms = Vec::new();
    flatten_additive_terms(node, true, &mut terms);
    let mut current = node.clone();
    let brackets = negated_brackets(unwrap_group(node));
    if !brackets.is_empty() {
        let Some(resolved) = signed_terms_sum(terms.clone()) else {
            return Vec::new();
        };
        let identities: Vec<String> = brackets
            .iter()
            .filter_map(|inner| {
                let mut flipped = Vec::new();
                flatten_additive_terms(inner, false, &mut flipped);
                let flipped = signed_terms_sum(flipped)?;
                Some(format!("-({}) = {}", serializer::expression(inner), serializer::expression(&flipped)))
            })
            .collect();
        let text = format!("A minus sign in front of a bracket flips every sign inside: {}", identities.join(", "));
        lines.push(AlgebraStep::line(&current, resolved.clone(), text, StepDetail::Standard));
        current = resolved;
    }
    lines.extend(like_term_lines(&current, &terms));
    lines
}

/// [`collect_working`] for the signed `terms` of the sum `node`, once no bracket is left.
fn like_term_lines(node: &Node, terms: &[(bool, Node)]) -> Vec<AlgebraStep> {
    // Like terms in groups, in the order of the canonical form: by descending powers.
    let mut groups: Vec<(crate::algebra::poly::Monomial, Vec<(bool, Node)>)> = Vec::new();
    for (positive, term) in terms {
        let Ok(poly) = Poly::from_node(term) else { return Vec::new() };
        let monomial = poly.terms.first().map(|t| t.monomial.clone()).unwrap_or_else(crate::algebra::poly::Monomial::one);
        if poly.terms.len() > 1 {
            return Vec::new();
        }
        match groups.iter_mut().find(|(m, _)| *m == monomial) {
            Some((_, group)) => group.push((*positive, term.clone())),
            None => groups.push((monomial, vec![(*positive, term.clone())])),
        }
    }
    if groups.iter().all(|(_, group)| group.len() == 1) {
        return Vec::new();
    }
    let order = |m: &crate::algebra::poly::Monomial| Poly::from_term(dashu::rational::RBig::ONE, m.clone());
    groups.sort_by(|(a, _), (b, _)| order(b).total_degree().cmp(&order(a).total_degree()));

    let mut lines = Vec::new();
    let mut current = node.clone();
    let mut push = |after: Node, text: &str, level: StepDetail, current: &mut Node| {
        if serializer::expression(&after) != serializer::expression(current) {
            lines.push(AlgebraStep::line(current, after.clone(), text, level));
            *current = after;
        }
    };
    if let Some(ordered) = signed_terms_sum(groups.iter().flat_map(|(_, group)| group.clone())) {
        push(ordered, "Order the terms by degree", StepDetail::Maximal, &mut current);
    }
    let grouped = groups.iter().map(|(_, group)| match group.as_slice() {
        [single] => single.clone(),
        many => (true, bracket(signed_terms_sum(many.to_vec()).expect("a group has terms"))),
    });
    if let Some(grouped) = signed_terms_sum(grouped) {
        push(grouped, "Put like terms next to each other", StepDetail::Detailed, &mut current);
    }
    // Groups that add up to zero.
    let sum = |group: &[(bool, Node)]| {
        group.iter().filter_map(|(positive, term)| Poly::from_node(term).ok().map(|p| if *positive { p } else { -p })).fold(Poly::zero(), |acc, p| &acc + &p)
    };
    let zeros: Vec<String> = groups
        .iter()
        .filter(|(_, group)| group.len() > 1 && sum(group).is_zero())
        .filter_map(|(_, group)| Some(format!("{} = 0", serializer::expression(&signed_terms_sum(group.clone())?))))
        .collect();
    if !zeros.is_empty() {
        let zero = Node::Number { value: crate::num::Decimal::from_i64(0), position: 0 };
        let with_zeros = groups.iter().map(|(_, group)| match group.as_slice() {
            [single] => single.clone(),
            many if sum(many).is_zero() => (true, zero.clone()),
            many => (true, bracket(signed_terms_sum(many.to_vec()).expect("a group has terms"))),
        });
        if let Some(with_zeros) = signed_terms_sum(with_zeros) {
            push(with_zeros, &zeros.join(", "), StepDetail::Detailed, &mut current);
        }
    }
    lines
}

/// The sums that stand in brackets after a minus sign in the sum `node`: `x + 1` in `x^2 - (x + 1)`.
fn negated_brackets(node: &Node) -> Vec<Node> {
    let is_sum = |n: &Node| matches!(unwrap_group(n), Node::Binary { operator: Operator::Plus | Operator::Minus, .. });
    match node {
        Node::Binary { operator: Operator::Plus, left, right, .. } => {
            let mut found = negated_brackets(unwrap_group(left));
            found.extend(negated_brackets(unwrap_group(right)));
            found
        }
        Node::Binary { operator: Operator::Minus, left, right, .. } => {
            let mut found = negated_brackets(unwrap_group(left));
            if is_sum(right) {
                found.push(unwrap_group(right).clone());
            }
            found
        }
        Node::Negation { operand, .. } if is_sum(operand) => vec![unwrap_group(operand).clone()],
        _ => Vec::new(),
    }
}

/// Attempts to collect like terms in an additive polynomial expression.
pub fn collect_like_terms_step(root: &Node) -> Option<(Node, AlgebraStep)> {
    let mut step_info: Option<AlgebraStep> = None;
    let new_root = collect_recursive(root, &mut step_info);
    step_info.map(|step| (new_root, step))
}

fn collect_recursive(node: &Node, step_info: &mut Option<AlgebraStep>) -> Node {
    if step_info.is_some() {
        return node.clone();
    }

    if let Node::Fraction {
        numerator,
        denominator,
        position,
    } = node
    {
        let new_num = collect_recursive(numerator, step_info);
        return Node::Fraction {
            numerator: Box::new(new_num),
            denominator: denominator.clone(),
            position: *position,
        };
    }

    let unwrapped = unwrap_group(node);

    // Only collect if node is an additive expression and has no pending products
    if matches!(
        unwrapped,
        Node::Binary {
            operator: Operator::Plus | Operator::Minus,
            ..
        }
    ) && expand_product_node(unwrapped).is_none() {
        if let Ok(poly) = Poly::from_node(unwrapped) {
            let canonical_node = poly.to_node();
            if serializer::expression(&canonical_node) != serializer::expression(unwrapped) {
                // Only putting the terms in order is bookkeeping (Maximal).
                let (explanation, level) = if is_term_reordering(unwrapped) {
                    ("Order the terms by descending powers".to_string(), StepDetail::Maximal)
                } else {
                    (format!("Combine like terms to obtain {}", serializer::expression(&canonical_node)), StepDetail::Brief)
                };
                let step = AlgebraStep::new(node.clone(), canonical_node.clone(), explanation.clone(), Some(RuleId::CollectLikeTerms))
                    .with_level(level);
                let working = collect_working(node);
                let last = if working.iter().any(|line| line.explanation == "Put like terms next to each other") {
                    "Add the coefficients".to_string()
                } else {
                    explanation
                };
                *step_info = Some(step.with_working(working, last));
                return canonical_node;
            }
        }
    }

    match node {
        Node::Binary {
            operator,
            left,
            right,
            position,
        } => {
            let new_left = collect_recursive(left, step_info);
            if step_info.is_some() {
                return Node::Binary {
                    operator: *operator,
                    left: Box::new(new_left),
                    right: right.clone(),
                    position: *position,
                };
            }
            let new_right = collect_recursive(right, step_info);
            Node::Binary {
                operator: *operator,
                left: Box::new(new_left),
                right: Box::new(new_right),
                position: *position,
            }
        }
        Node::Group { inner, position } => {
            let new_inner = collect_recursive(inner, step_info);
            Node::Group {
                inner: Box::new(new_inner),
                position: *position,
            }
        }
        _ => node.clone(),
    }
}

/// Repeatedly expands products and then collects like terms, recording every step.
pub fn expand_and_collect_all(root: &Node) -> (Node, Vec<AlgebraStep>) {
    let mut current = root.clone();
    let mut steps = Vec::new();
    let mut iterations = 0;
    const MAX_ITERATIONS: usize = 20;

    // 1. Expand products and powers
    while iterations < MAX_ITERATIONS {
        iterations += 1;
        let before_tree = current.clone();
        if let Some((new_tree, local_step)) = expand_polynomial_products_step(&current) {
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

    // 2. Collect like terms
    let before_collect = current.clone();
    if let Some((new_tree, local_step)) = collect_like_terms_step(&current) {
        let full_step = AlgebraStep::new(
            before_collect,
            new_tree.clone(),
            local_step.explanation,
            local_step.rule,
        );
        steps.push(full_step);
        current = new_tree;
    }

    (current, steps)
}
