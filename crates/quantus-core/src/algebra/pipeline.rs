//! The deterministic simplification pipeline (Milestone 5).
//!
//! Orchestrates the individual rule modules (`nested`, `combine`, `expand`, `reduce`, ...) into
//! one fixed sequence of stages:
//!
//! 1. Parse the input into an AST (`simplify`).
//! 2. Normalize notation and signs (negative exponents, double negations, `/` as fractions) and
//!    identify the algebraic variables.
//! 3. Extract the initial domain restrictions (including nested reciprocal denominators).
//! 4. Simplify nested fractions bottom-up (`SIMPLIFY_NESTED_FRACTION`, `APPLY_RECIPROCAL_RULE`).
//! 5. Normalize multiplication, division and signs (products and powers of fractions).
//! 6. Factor polynomial denominators (`FACTOR_DENOMINATORS`).
//! 7. Rewrite every fraction of a sum over the least common denominator (`FIND_COMMON_DENOMINATOR`).
//! 8. Combine the numerators over the common denominator (`COMBINE_FRACTIONS`).
//! 9. Expand products and binomial powers in the numerator (`EXPAND_POLYNOMIAL`, `EXPAND_BINOMIAL`).
//! 10. Collect like terms (`COLLECT_LIKE_TERMS`).
//! 11. Factor numerator and denominator (`FACTOR_POLYNOMIAL`, `APPLY_BINOMIAL_IDENTITY`).
//! 12. Cancel common multiplicative factors (`CANCEL_COMMON_FACTORS`).
//! 13. Normalize the final expression into its canonical readable form (`NORMALIZE_EXPRESSION`):
//!     expanded numerator, denominator kept factored when it is a product of several factors.
//!
//! Every stage is a bounded loop of single, recorded rewrites, so each `AlgebraStep` holds the
//! complete expression before and after the transformation.

use std::collections::BTreeSet;

use dashu::base::Signed;
use dashu::integer::UBig;

use crate::algebra::combine::{
    combine_next_fractions, den_to_factored, factor_denominators_in_node, flatten_additive_terms,
    is_combinable_fraction_sum,
};
use crate::algebra::domain::{unwrap_group, DomainTracker};
use crate::algebra::expand::{collect_like_terms_step, expand_polynomial_products_step};
use crate::algebra::factored::FactoredPoly;
use crate::algebra::nested::{make_fraction, make_product, simplify_next_nested_fraction, split_fraction};
use crate::algebra::poly::Poly;
use crate::algebra::reduce::reduce_next_rational;
use crate::algebra::steps::{AlgebraStep, RuleId, SimplificationResult};
use crate::error::Result;
use crate::latex::ast::{Node, Operator};
use crate::latex::{parser, serializer};
use crate::num::Decimal;

/// Upper bound on the number of rewrites a single stage may perform.
pub const MAX_STAGE_ITERATIONS: usize = 32;

/// Stage 1: parses `latex` and runs the whole pipeline on the resulting expression.
pub fn simplify(latex: &str) -> Result<SimplificationResult> {
    let formula = parser::parse(latex)?;
    simplify_node(&formula.expression)
}

/// Runs stages 2–13 on an already parsed expression.
pub fn simplify_node(node: &Node) -> Result<SimplificationResult> {
    let mut pipeline = Pipeline::new(node)?;
    pipeline.run();
    Ok(pipeline.finish())
}

/// Collects the names of all free algebraic variables of `node`, sorted.
pub fn collect_variables(node: &Node) -> BTreeSet<String> {
    let mut vars = BTreeSet::new();
    collect_variables_rec(node, &mut vars);
    vars
}

fn collect_variables_rec(node: &Node, vars: &mut BTreeSet<String>) {
    if let Node::Variable { name, index, .. } = node {
        vars.insert(Node::variable_key(name, index));
    }
    for child in node.children() {
        collect_variables_rec(child, vars);
    }
}

/// The mutable state threaded through all stages.
struct Pipeline {
    current: Node,
    steps: Vec<AlgebraStep>,
    tracker: DomainTracker,
}

type LocalRewrite = (Node, AlgebraStep);

impl Pipeline {
    /// Stages 2 and 3: normalization and initial domain extraction.
    fn new(node: &Node) -> Result<Self> {
        // Stage 3 runs on the untouched input as well, so no restriction can be lost by stage 2.
        let mut tracker = DomainTracker::from_ast(node)?;
        let mut pipeline = Self {
            current: node.clone(),
            steps: Vec::new(),
            tracker: DomainTracker::new(),
        };

        // Stage 2: normalize notation and signs.
        let normalized = normalize_notation(node);
        if serializer::expression(&normalized) != serializer::expression(node) {
            pipeline.record(
                normalized.clone(),
                "Rewrite negative exponents as fractions and normalize signs",
                RuleId::NormalizeExpression,
            );
        } else {
            pipeline.current = normalized.clone();
        }
        tracker.scan_and_record(&normalized)?;
        pipeline.tracker = tracker;
        Ok(pipeline)
    }

    /// Stages 4–13.
    fn run(&mut self) {
        // Stage 4: nested fractions, bottom-up.
        self.run_stage(|node, _| simplify_next_nested_fraction(node));
        // Stage 5: products, quotients and powers of fractions; signs.
        self.run_stage(|node, _| normalize_products_step(node));
        // Stage 6: factor denominators.
        self.run_stage(|node, _| factor_denominators_in_node(node));
        // Stage 7: rewrite fractions over the least common denominator.
        self.run_stage(|node, _| common_denominator_step(node));
        // Stage 8: combine the numerators over the common denominator.
        self.run_stage(|node, _| combine_next_fractions(node));
        // Stage 9: expand products and binomial powers.
        self.run_stage(|node, _| expand_polynomial_products_step(node));
        // Stage 10: collect like terms.
        self.run_stage(|node, _| collect_like_terms_step(node));
        // Stages 11 & 12: factor and cancel.
        self.run_stage(|node, tracker| reduce_next_rational(node, Some(tracker)));
        // Stage 13: canonical final form.
        self.run_stage(|node, _| canonicalize_step(node));
    }

    /// Repeatedly applies `rewrite` to the whole expression, recording each rewrite as a step.
    fn run_stage<F>(&mut self, mut rewrite: F)
    where
        F: FnMut(&Node, &mut DomainTracker) -> Option<LocalRewrite>,
    {
        for _ in 0..MAX_STAGE_ITERATIONS {
            let Some((next, local)) = rewrite(&self.current, &mut self.tracker) else {
                break;
            };
            if serializer::expression(&next) == serializer::expression(&self.current) {
                // A rewrite that does not change the visible expression would only add noise.
                self.current = next;
                break;
            }
            let rule = local.rule.unwrap_or(RuleId::NormalizeExpression);
            self.record(next, local.explanation, rule);
        }
    }

    fn record(&mut self, after: Node, explanation: impl Into<String>, rule: RuleId) {
        // Every denominator that shows up along the way is a product of original denominators
        // (or numerators of divisors), so its zeros are restrictions of the original expression.
        // This catches e.g. `x != 1` in `(1 + 1/x) / (1 - 1/x)`, which only becomes a polynomial
        // denominator once the inner sums are combined. Constant-zero denominators were already
        // rejected by the initial scan.
        let _ = self.tracker.scan_and_record(&after);
        let before = std::mem::replace(&mut self.current, after.clone());
        self.steps.push(AlgebraStep::new(before, after, explanation, Some(rule)));
    }

    fn finish(self) -> SimplificationResult {
        SimplificationResult::new(self.current, self.steps, self.tracker.into_restrictions())
    }
}

// ---------------------------------------------------------------------------------------------
// Generic tree rewriting
// ---------------------------------------------------------------------------------------------

/// A local rewrite rule: given a node, either rewrites it or declines.
type Rule<'a> = dyn FnMut(&Node) -> Option<(Node, RuleId, String)> + 'a;

/// Replaces the `index`-th direct child of `node` by `replacement`.
fn replace_child(node: &Node, index: usize, replacement: Node) -> Node {
    let mut counter = 0;
    let mut replacement = Some(replacement);
    node.clone().map_children(&mut |child| {
        let result = if counter == index {
            replacement.take().unwrap_or(child)
        } else {
            child
        };
        counter += 1;
        result
    })
}

/// Finds the first (post-order, i.e. innermost-first) node where `rule` applies and rewrites it.
///
/// Returns the rewritten root together with a step describing the local transformation.
pub(crate) fn rewrite_first_bottom_up(root: &Node, rule: &mut Rule<'_>) -> Option<LocalRewrite> {
    for (index, child) in root.children().into_iter().enumerate() {
        if let Some((new_child, step)) = rewrite_first_bottom_up(child, rule) {
            return Some((replace_child(root, index, new_child), step));
        }
    }
    let (after, rule_id, explanation) = rule(root)?;
    let step = AlgebraStep::new(root.clone(), after.clone(), explanation, Some(rule_id));
    Some((after, step))
}

/// Finds the first (pre-order, i.e. outermost-first) node where `rule` applies and rewrites it.
pub(crate) fn rewrite_first_top_down(root: &Node, rule: &mut Rule<'_>) -> Option<LocalRewrite> {
    if let Some((after, rule_id, explanation)) = rule(root) {
        let step = AlgebraStep::new(root.clone(), after.clone(), explanation, Some(rule_id));
        return Some((after, step));
    }
    for (index, child) in root.children().into_iter().enumerate() {
        if let Some((new_child, step)) = rewrite_first_top_down(child, rule) {
            return Some((replace_child(root, index, new_child), step));
        }
    }
    None
}

// ---------------------------------------------------------------------------------------------
// Stage 2: notation and sign normalization
// ---------------------------------------------------------------------------------------------

fn number(value: i64) -> Node {
    Node::Number {
        value: Decimal::from_i64(value),
        position: 0,
    }
}

/// Reads an exponent node as an integer constant, if it is one.
pub(crate) fn integer_exponent(node: &Node) -> Option<i64> {
    let poly = Poly::from_node(unwrap_group(node)).ok()?;
    if !poly.is_constant() {
        return None;
    }
    let value = poly.constant_term();
    if value.denominator() != &UBig::from(1u8) {
        return None;
    }
    i64::try_from(value.numerator()).ok()
}

/// Stage 2: rewrites `b^{-n}` as `\frac{1}{b^{n}}`, removes double negations and turns any
/// remaining `/` into a fraction. Applied to the whole tree at once.
pub fn normalize_notation(node: &Node) -> Node {
    let node = node.clone().map_children(&mut |child| normalize_notation(&child));
    match node {
        Node::Power { ref base, ref exponent, .. } => match integer_exponent(exponent) {
            Some(n) if n < 0 => {
                let positive = if n == -1 {
                    (**base).clone()
                } else {
                    Node::Power {
                        base: base.clone(),
                        exponent: Box::new(number(-n)),
                        position: 0,
                    }
                };
                make_fraction(number(1), positive)
            }
            _ => node,
        },
        Node::Negation { ref operand, .. } => match unwrap_group(operand) {
            Node::Negation { operand: inner, .. } => (**inner).clone(),
            _ => node,
        },
        Node::Binary {
            operator: Operator::Divide,
            left,
            right,
            ..
        } => make_fraction(*left, *right),
        other => other,
    }
}

// ---------------------------------------------------------------------------------------------
// Stage 5: products, quotients and powers of fractions
// ---------------------------------------------------------------------------------------------

/// Stage 5 rule: multiplies fractions, distributes integer powers over fractions and removes
/// paired signs in fractions.
pub fn normalize_product_at(node: &Node) -> Option<(Node, RuleId, String)> {
    match node {
        Node::Binary {
            operator: Operator::Times | Operator::ImplicitTimes,
            left,
            right,
            ..
        } => {
            let lf = split_fraction(left);
            let rf = split_fraction(right);
            let (num, den) = match (lf, rf) {
                (Some((a, b)), Some((c, d))) => (
                    make_product(a.clone(), c.clone()),
                    make_product(b.clone(), d.clone()),
                ),
                (Some((a, b)), None) => (make_product(a.clone(), (**right).clone()), b.clone()),
                (None, Some((c, d))) => (make_product((**left).clone(), c.clone()), d.clone()),
                (None, None) => return None,
            };
            Some((
                make_fraction(num, den),
                RuleId::NormalizeExpression,
                "Multiply fractions: multiply the numerators and the denominators".to_string(),
            ))
        }
        Node::Power { base, exponent, .. } => {
            let (a, b) = split_fraction(base)?;
            let n = integer_exponent(exponent).filter(|n| *n >= 0)?;
            let power = |x: &Node| Node::Power {
                base: Box::new(wrap_power_base(x.clone())),
                exponent: Box::new(number(n)),
                position: 0,
            };
            Some((
                make_fraction(power(a), power(b)),
                RuleId::NormalizeExpression,
                "Raise numerator and denominator to the power separately".to_string(),
            ))
        }
        Node::Fraction {
            numerator,
            denominator,
            ..
        } => match (unwrap_group(numerator), unwrap_group(denominator)) {
            (Node::Negation { operand: a, .. }, Node::Negation { operand: b, .. }) => Some((
                make_fraction((**a).clone(), (**b).clone()),
                RuleId::NormalizeExpression,
                "Cancel the minus signs of numerator and denominator".to_string(),
            )),
            _ => None,
        },
        _ => None,
    }
}

fn wrap_power_base(node: Node) -> Node {
    match unwrap_group(&node) {
        Node::Number { .. } | Node::Variable { .. } | Node::Group { .. } => node,
        _ => Node::Group {
            inner: Box::new(node),
            position: 0,
        },
    }
}

/// Stage 5: applies [`normalize_product_at`] to the innermost applicable node.
pub fn normalize_products_step(root: &Node) -> Option<LocalRewrite> {
    rewrite_first_bottom_up(root, &mut normalize_product_at)
}

// ---------------------------------------------------------------------------------------------
// Stage 7: rewriting fractions over the least common denominator
// ---------------------------------------------------------------------------------------------

/// Stage 7 rule: rewrites every term of a fraction sum with differing denominators as a
/// fraction over the least common denominator, e.g.
/// `\frac{3(x - 2)}{x + 3} + \frac{x - 102}{(x - 4)(x + 3)}` becomes
/// `\frac{3(x - 2)(x - 4)}{(x - 4)(x + 3)} + \frac{x - 102}{(x - 4)(x + 3)}`.
pub fn common_denominator_at(node: &Node) -> Option<(Node, RuleId, String)> {
    if !is_combinable_fraction_sum(node) {
        return None;
    }
    let mut terms = Vec::new();
    flatten_additive_terms(node, true, &mut terms);

    let one = number(1);
    let mut parts = Vec::new();
    let mut lcd = FactoredPoly::one();
    for (positive, term) in terms {
        let (num, den) = match split_fraction(&term) {
            Some((n, d)) => (n.clone(), d.clone()),
            None => (term.clone(), one.clone()),
        };
        // A denominator that is not a polynomial cannot take part in an LCD computation.
        if Poly::from_node(&den).is_err() {
            return None;
        }
        let factored = den_to_factored(&den);
        lcd = lcd.lcm(&factored);
        parts.push((positive, num, factored));
    }

    let distinct: BTreeSet<String> = parts.iter().map(|(_, _, d)| d.to_plain_string()).collect();
    if distinct.len() < 2 {
        return None;
    }

    let lcd_node = lcd.to_node();
    let mut acc: Option<Node> = None;
    for (positive, num, den) in parts {
        let missing = lcd.div(&den).unwrap_or_else(FactoredPoly::one);
        let scaled = if missing.is_one() {
            num
        } else {
            make_product(num, missing.to_node())
        };
        let fraction = make_fraction(scaled, lcd_node.clone());
        acc = Some(match acc {
            None if positive => fraction,
            None => Node::Negation {
                operand: Box::new(fraction),
                position: 0,
            },
            Some(prev) => Node::binary(
                if positive { Operator::Plus } else { Operator::Minus },
                prev,
                fraction,
                0,
            ),
        });
    }
    Some((
        acc?,
        RuleId::FindCommonDenominator,
        format!(
            "Rewrite every fraction over the least common denominator {}",
            serializer::expression(&lcd_node)
        ),
    ))
}

/// Stage 7: applies [`common_denominator_at`] to the outermost applicable sum.
pub fn common_denominator_step(root: &Node) -> Option<LocalRewrite> {
    rewrite_first_top_down(root, &mut common_denominator_at)
}

// ---------------------------------------------------------------------------------------------
// Stage 13: canonical final form
// ---------------------------------------------------------------------------------------------

/// Canonical form of a polynomial denominator: factored when it splits into several factors (or
/// a repeated one), expanded otherwise.
fn canonical_denominator(poly: &Poly) -> Node {
    let factored = FactoredPoly::from_poly(poly.clone());
    let splits = factored.factors.len() > 1 || factored.factors.iter().any(|f| f.exp > 1);
    if splits {
        factored.to_node()
    } else {
        poly.to_node()
    }
}

/// Builds the canonical node for `num / den` (both polynomials, `den` non-zero).
fn canonical_fraction(num: &Poly, den: &Poly) -> Node {
    if den.is_constant() {
        let c = den.leading_coeff();
        return (num / &c).to_node();
    }
    // Keep the leading coefficient of the denominator positive.
    let (num, den) = if den.leading_coeff().is_negative() {
        (-num, -den)
    } else {
        (num.clone(), den.clone())
    };
    make_fraction(num.to_node(), canonical_denominator(&den))
}

/// Stage 13 rule: brings a polynomial or a quotient of polynomials into its canonical readable
/// form (expanded numerator in descending degree order, factored denominator).
pub fn canonical_form(node: &Node) -> Option<Node> {
    let unwrapped = unwrap_group(node);
    if let Ok(poly) = Poly::from_node(unwrapped) {
        return Some(poly.to_node());
    }
    let (sign, inner) = match unwrapped {
        Node::Negation { operand, .. } => (-1, unwrap_group(operand)),
        other => (1, other),
    };
    let (num, den) = split_fraction(inner)?;
    let num = Poly::from_node(num).ok()?;
    let den = Poly::from_node(den).ok()?;
    if den.is_zero() {
        return None;
    }
    let num = if sign < 0 { -&num } else { num };
    Some(canonical_fraction(&num, &den))
}

/// Stage 13: rewrites the whole expression into its canonical form, if that changes it.
pub fn canonicalize_step(root: &Node) -> Option<LocalRewrite> {
    let canonical = canonical_form(root)?;
    if serializer::expression(&canonical) == serializer::expression(root) {
        return None;
    }
    let step = AlgebraStep::new(
        root.clone(),
        canonical.clone(),
        "Write the result in canonical form: terms in descending degree order, denominator factored",
        Some(RuleId::NormalizeExpression),
    );
    Some((canonical, step))
}
