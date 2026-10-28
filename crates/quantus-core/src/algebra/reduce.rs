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
use crate::algebra::display::cancel;
use crate::algebra::factor::{factoring_lines, line_steps};
use crate::algebra::factored::Factor;
use crate::algebra::poly::{rational_to_node, Monomial, Poly};
use crate::algebra::steps::{AlgebraStep, RuleId};
use crate::latex::ast::{Node, Operator};
use crate::settings::StepDetail;
use crate::latex::serializer;
use crate::num::Decimal;

/// Attempts to reduce a fraction node by factoring, cancellation, or special cases.
pub fn reduce_rational_node(node: &Node, tracker: Option<&mut DomainTracker>) -> Option<(Node, RuleId, String)> {
    let step = reduce_rational_step(node, tracker)?;
    Some((step.after, step.rule?, step.summary.unwrap_or(step.explanation)))
}

/// [`reduce_rational_node`] as a step with its working: the numerator factored line by line, the
/// common factors marked before they are cancelled, and a note on where the cancelled factors
/// were zero.
fn reduce_rational_step(node: &Node, mut tracker: Option<&mut DomainTracker>) -> Option<AlgebraStep> {
    let simple = |after: Node, rule: RuleId, explanation: String| Some(AlgebraStep::new(node.clone(), after, explanation, Some(rule)));
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
        return simple(zero_node, rule, explanation);
    }

    // 2. Denominator is 1: P / 1 -> P
    if is_constant_one(den) {
        let rule = RuleId::NormalizeExpression;
        let explanation = "Division by 1 leaves the expression unchanged".to_string();
        return simple(num.clone(), rule, explanation);
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
            return simple(one_node, rule, explanation);
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
            return simple(neg_one_node, rule, explanation);
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
        let step = AlgebraStep::new(node.clone(), new_frac, explanation, Some(rule));
        let Ok(poly) = Poly::from_node(num) else { return Some(step) };
        let (lines, last) = factoring_lines(&poly);
        let lines = lines.into_iter().map(|line| line.placed(|part| make_fraction(part, den.clone()))).collect();
        return Some(step.with_working(line_steps(node, lines), last));
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
    let common = serializer::expression(&gcd.to_node());
    let explanation = format!("Cancel common factor {common} from numerator and denominator");
    let mut working = Vec::new();
    if let Some(difference) = exponent_difference(num, den) {
        working.push(AlgebraStep::line(node, difference, "Divide powers of the same base: subtract the exponents", StepDetail::Standard));
    } else {
        let marked = make_fraction(mark_cancelled(num, &mut gcd.factors.clone()), mark_cancelled(den, &mut gcd.factors.clone()));
        if serializer::expression(&marked) != serializer::expression(unwrapped) {
            working.push(AlgebraStep::line(node, marked, "Mark the common factor", StepDetail::Detailed));
        }
    }
    let notes = gcd.factors.iter().map(|f| AlgebraStep::note(res_node.clone(), zero_note(&f.base), None, StepDetail::Maximal)).collect();
    let step = AlgebraStep::new(node.clone(), res_node, explanation, Some(rule));
    Some(step.with_working(working, format!("Cancel {common}")).with_notes(notes))
}

/// `x^{5 - 2}` for `\frac{x^5}{x^2}`, or `\frac{1}{x^{5 - 2}}` the other way round: a power over a
/// power of the same base.
fn exponent_difference(num: &Node, den: &Node) -> Option<Node> {
    let power = |node: &Node| -> Option<(Node, i64)> {
        match unwrap_group(node) {
            Node::Power { base, exponent, .. } => Some(((**base).clone(), crate::algebra::pipeline::integer_exponent(exponent)?)),
            other @ Node::Variable { .. } => Some((other.clone(), 1)),
            _ => None,
        }
    };
    let ((base, m), (other, n)) = (power(num)?, power(den)?);
    if serializer::expression(&base) != serializer::expression(&other) || m == n || !matches!(unwrap_group(&base), Node::Variable { .. }) {
        return None;
    }
    let number = |v: i64| Node::Number { value: Decimal::from_i64(v), position: 0 };
    let (high, low) = if m > n { (m, n) } else { (n, m) };
    let difference = Node::Power { base: Box::new(base), exponent: Box::new(Node::binary(Operator::Minus, number(high), number(low), 0)), position: 0 };
    Some(if m > n { difference } else { make_fraction(number(1), difference) })
}

/// `node` with each factor of `factors` it contains marked for cancelling, keeping the order in
/// which it is written. Marked factors are taken off the list.
fn mark_cancelled(node: &Node, factors: &mut Vec<Factor>) -> Node {
    if let Ok(poly) = Poly::from_node(node) {
        if let Some(index) = factors.iter().position(|f| !poly.is_constant() && f.base.pow(f.exp) == poly) {
            factors.remove(index);
            return cancel(node.clone());
        }
    }
    match node {
        Node::Binary { operator: operator @ (Operator::Times | Operator::ImplicitTimes), left, right, position } => {
            let left = mark_cancelled(left, factors);
            let right = mark_cancelled(right, factors);
            Node::Binary { operator: *operator, left: Box::new(left), right: Box::new(right), position: *position }
        }
        Node::Negation { operand, position } => Node::Negation { operand: Box::new(mark_cancelled(operand, factors)), position: *position },
        Node::Group { inner, position } => Node::Group { inner: Box::new(mark_cancelled(inner, factors)), position: *position },
        other => other.clone(),
    }
}

/// Where the cancelled factor `base` was zero, the input is undefined.
fn zero_note(base: &Poly) -> String {
    let factor = serializer::expression(&base.to_node());
    match (base.is_univariate(), base.total_degree(), base.main_variable()) {
        (true, 1, Some(var)) => {
            let root = -base.coeff_of(&Monomial::one()) / base.coeff_of(&Monomial::from_var(&var));
            let root = serializer::expression(&rational_to_node(&root));
            format!("Note: {var} ≠ {root}: the cancelled factor {factor} is zero there, so the input is undefined at {var} = {root}")
        }
        _ => format!("Note: {factor} ≠ 0: where the cancelled factor is zero, the input is undefined"),
    }
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
        if let Some(step) = reduce_rational_step(node, tracker.as_deref_mut()) {
            let reduced = step.after.clone();
            *step_info = Some(step);
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
