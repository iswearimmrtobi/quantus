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
//!     expanded numerator, denominator kept factored when it is a product of several factors
//!     (or always expanded / always factored, per [`OutputStyle::denominator`]).
//! 14. Write a polynomial result in binomial form (`APPLY_BINOMIAL_IDENTITY`, `COMPLETE_SQUARE`):
//!     `a^2 + 2ab + b^2` becomes `(a + b)^2`, and `x^2 + 2 \cdot x \cdot 4 + 11` or
//!     `x^2 + 8x + 11` (monic, integer shift) becomes `(x + 4)^2 - 5`. By default skipped when the
//!     input itself was compact (`(a + b)^2`, `(a - b)(a + b)`), since then the expansion is what
//!     was asked for; [`OutputStyle`] can force either direction and turn off completing the square.
//!
//! Every stage is a bounded loop of single, recorded rewrites, so each `AlgebraStep` holds the
//! complete expression before and after the transformation.
//!
//! Termination (5.2): stages are grouped into [`Phase`]s that only move forward within a pass;
//! passes repeat (at most `max_passes` times) only while the expression is not yet in rational
//! normal form; every stage and the whole run have rewrite budgets; and every state is
//! fingerprinted, so a rewrite that returns to an earlier state rolls the detour back instead of
//! oscillating between expanded and factored forms.

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::hash::{Hash, Hasher};

use dashu::base::Signed;
use dashu::integer::UBig;

use crate::algebra::binomial::{
    complete_square, compress_binomial, expand_binomial_node, find_explicit_double_product,
    integer_square_shift,
};
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
use crate::settings::{BinomialForm, ConditionalIdentities, DenominatorForm, Settings};

/// How the result is written: the user-adjustable choices of stages 13 and 14.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutputStyle {
    pub binomial: BinomialForm,
    pub complete_square: bool,
    pub denominator: DenominatorForm,
    /// Whether identities that need a condition are applied with it or only where it holds.
    pub conditions: ConditionalIdentities,
}

impl Default for OutputStyle {
    fn default() -> Self {
        Settings::default().output_style()
    }
}

/// Limits that guarantee the pipeline terminates, and the style of the result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PipelineConfig {
    /// How often the phase sequence may be repeated when a pass leaves the expression outside
    /// the rational normal form (e.g. a product turned into a new nested fraction).
    pub max_passes: usize,
    /// Upper bound on the number of rewrites a single stage may perform within one pass.
    pub max_stage_iterations: usize,
    /// Upper bound on the number of recorded steps over the whole run.
    pub max_total_steps: usize,
    /// Whether to verify mathematical equivalence against the original AST.
    pub verify_equivalence: bool,
    /// How stages 13 and 14 write the result.
    pub style: OutputStyle,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            max_passes: 4,
            max_stage_iterations: 32,
            max_total_steps: 256,
            verify_equivalence: true,
            style: OutputStyle::default(),
        }
    }
}

/// The phases of one pass, in their strict order. Within a pass the pipeline only ever moves
/// forward through this list, so an expansion can never be followed by a re-factoring that
/// triggers the same expansion again inside the same pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Phase {
    /// Stage 2: notation and sign normalization (only at the very start).
    Normalize,
    /// Stage 4: flatten nested fractions.
    NestedFractions,
    /// Stage 5: inner simplification of products, quotients and powers of fractions.
    InnerSimplify,
    /// Stages 6–8: factor denominators, find the LCD, combine.
    CommonDenominator,
    /// Stages 9–10: expand the numerator and collect like terms.
    ExpandCollect,
    /// Stages 11–12: factor and cancel.
    FactorCancel,
    /// Stage 13: canonical final form.
    Canonicalize,
}

/// Why a run stopped before reaching its natural end, if it did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    /// A rewrite would have reproduced an earlier state (expand/factor oscillation).
    CycleDetected,
    /// `max_total_steps` was reached.
    StepLimit,
    /// `max_passes` passes ran and the expression still changed in the last one.
    PassLimit,
}

/// Stage 1: parses `latex` and runs the whole pipeline on the resulting expression.
pub fn simplify(latex: &str) -> Result<SimplificationResult> {
    let formula = parser::parse(latex)?;
    simplify_node(&formula.expression)
}

/// Runs stages 2–13 on an already parsed expression.
pub fn simplify_node(node: &Node) -> Result<SimplificationResult> {
    simplify_node_with(node, PipelineConfig::default())
}

/// Runs stages 2–13 with explicit termination limits.
pub fn simplify_node_with(node: &Node, config: PipelineConfig) -> Result<SimplificationResult> {
    if let Some(result) = crate::algebra::atoms::simplify_with_atoms(node, config.style.conditions, |atomized| simplify_rational(atomized, config)) {
        return result;
    }
    simplify_rational(node, config)
}

fn simplify_rational(node: &Node, config: PipelineConfig) -> Result<SimplificationResult> {
    let mut pipeline = Pipeline::new(node, config)?;
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

/// Detects if an expression contains constructs outside rational/polynomial algebra.
///
/// Returns a descriptive name if unsupported constructs are found.
pub fn find_unsupported_construct(node: &Node) -> Option<&'static str> {
    match node {
        Node::Matrix { .. } => Some("matrices"),
        Node::Determinant { .. } => Some("determinants"),
        Node::Equation { .. } => Some("equations"),
        Node::Function { .. } => Some("transcendental functions"),
        Node::Call { .. } => Some("integer functions"),
        Node::Sum { .. } => Some("sums"),
        Node::Integral { .. } => Some("integrals"),
        Node::Limit { .. } => Some("limits"),
        Node::Derivative { .. } => Some("derivatives"),
        Node::Power { exponent, .. } => {
            if integer_exponent(exponent).is_none() {
                Some("non-integer exponents")
            } else {
                for child in node.children() {
                    if let Some(desc) = find_unsupported_construct(child) {
                        return Some(desc);
                    }
                }
                None
            }
        }
        _ => {
            for child in node.children() {
                if let Some(desc) = find_unsupported_construct(child) {
                    return Some(desc);
                }
            }
            None
        }
    }
}

/// The note for a result that still contains a construct the pipeline cannot simplify.
pub(crate) fn unsupported_diagnostic(node: &Node) -> Option<String> {
    find_unsupported_construct(node).map(|construct| {
        format!(
            "Expression contains {construct}, which cannot be fully simplified algebraically; supported rational subtrees were simplified."
        )
    })
}

/// Whether `node` is in rational normal form: a polynomial, or (minus) a quotient of two
/// polynomials. Expressions in this form need no further pass.
pub fn is_rational_normal_form(node: &Node) -> bool {
    let node = unwrap_group(node);
    if Poly::from_node(node).is_ok() {
        return true;
    }
    let inner = match node {
        Node::Negation { operand, .. } => unwrap_group(operand),
        other => other,
    };
    match split_fraction(inner) {
        Some((num, den)) => Poly::from_node(num).is_ok() && Poly::from_node(den).is_ok(),
        None => false,
    }
}

/// A fingerprint of an expression state, used to detect revisited states.
fn state_key(node: &Node) -> u64 {
    let mut hasher = DefaultHasher::new();
    serializer::expression(node).hash(&mut hasher);
    hasher.finish()
}

/// The mutable state threaded through all stages.
struct Pipeline {
    config: PipelineConfig,
    current: Node,
    steps: Vec<AlgebraStep>,
    tracker: DomainTracker,
    /// The input expression (state 0).
    original: Node,
    /// Every live state, mapped to the number of steps recorded when it was reached.
    visited: HashMap<u64, usize>,
    /// States that were left behind by a rollback; reaching one again is a cycle.
    abandoned: HashSet<u64>,
    /// Number of accepted rewrites, including rolled-back ones (bounded by `max_total_steps`).
    rewrites: usize,
    phase: Phase,
    stop: Option<StopReason>,
    /// Which way a polynomial result is written by stage 14.
    binomial_style: BinomialStyle,
}

/// The direction in which binomial formulas are applied to a polynomial result.
#[derive(Clone, Debug)]
enum BinomialStyle {
    /// Keep the expanded result: the input was compact (`(a + b)^2`, `(a - b)(a + b)`), or the
    /// user asked for expanded results.
    Expand,
    /// Write the result in binomial form where possible. Carries the `(A, B)` of an explicit
    /// double product `2 \cdot A \cdot B` in the input, used to complete the square, and whether
    /// completing the square is allowed at all.
    Compress {
        double_product: Option<(Poly, Poly)>,
        complete_square: bool,
    },
}

impl BinomialStyle {
    fn of_input(node: &Node, style: &OutputStyle) -> Self {
        let expand = match style.binomial {
            BinomialForm::Auto => is_compact_binomial_input(node),
            BinomialForm::Expanded => true,
            BinomialForm::Binomial => false,
        };
        if expand {
            Self::Expand
        } else {
            Self::Compress {
                double_product: find_explicit_double_product(node),
                complete_square: style.complete_square,
            }
        }
    }
}

type LocalRewrite = (Node, AlgebraStep);

impl Pipeline {
    /// Stages 2 and 3: normalization and initial domain extraction.
    fn new(node: &Node, config: PipelineConfig) -> Result<Self> {
        // Stage 3 runs on the untouched input as well, so no restriction can be lost by stage 2.
        let mut tracker = DomainTracker::from_ast(node)?;
        let mut pipeline = Self {
            config,
            current: node.clone(),
            steps: Vec::new(),
            tracker: DomainTracker::new(),
            original: node.clone(),
            visited: HashMap::from([(state_key(node), 0)]),
            abandoned: HashSet::new(),
            rewrites: 0,
            phase: Phase::Normalize,
            stop: None,
            binomial_style: BinomialStyle::Expand,
        };

        // Stage 2: normalize notation and signs.
        let normalized = normalize_notation(node);
        if serializer::expression(&normalized) != serializer::expression(node) {
            pipeline.visited.insert(state_key(&normalized), 1);
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
        pipeline.binomial_style = BinomialStyle::of_input(&normalized, &config.style);
        Ok(pipeline)
    }

    /// Stages 4–13, repeated in bounded passes until the expression is in normal form.
    fn run(&mut self) {
        for _ in 0..self.config.max_passes {
            let start = state_key(&self.current);
            self.run_pass();
            if self.stop.is_some()
                || is_rational_normal_form(&self.current)
                || state_key(&self.current) == start
            {
                return;
            }
        }
        if !is_rational_normal_form(&self.current) {
            self.stop = Some(StopReason::PassLimit);
        }
    }

    /// One pass through all phases in their strict order.
    fn run_pass(&mut self) {
        self.phase = Phase::Normalize;

        self.enter(Phase::NestedFractions);
        // Stage 4: nested fractions, bottom-up.
        self.run_stage(|node, _| simplify_next_nested_fraction(node));

        self.enter(Phase::InnerSimplify);
        // Stage 5: products, quotients and powers of fractions; signs.
        self.run_stage(|node, _| normalize_products_step(node));

        self.enter(Phase::CommonDenominator);
        // Stage 6: factor denominators.
        self.run_stage(|node, _| factor_denominators_in_node(node));
        // Stage 7: rewrite fractions over the least common denominator.
        self.run_stage(|node, _| common_denominator_step(node));
        // Stage 8: combine the numerators over the common denominator.
        self.run_stage(|node, _| combine_next_fractions(node));

        self.enter(Phase::ExpandCollect);
        // Stage 9: expand products and binomial powers.
        self.run_stage(|node, _| expand_polynomial_products_step(node));
        // Stage 10: collect like terms.
        self.run_stage(|node, _| collect_like_terms_step(node));

        self.enter(Phase::FactorCancel);
        // Stages 11 & 12: factor and cancel.
        self.run_stage(|node, tracker| reduce_next_rational(node, Some(tracker)));

        self.enter(Phase::Canonicalize);
        // Stage 13: canonical final form.
        let denominator = self.config.style.denominator;
        self.run_stage(|node, _| canonicalize_step(node, denominator));
        // Stage 14: binomial form of a polynomial result.
        let style = self.binomial_style.clone();
        self.run_stage(|node, _| binomial_form_step(node, &style));
    }

    fn enter(&mut self, phase: Phase) {
        assert!(phase >= self.phase, "pipeline phases must progress monotonically");
        self.phase = phase;
    }

    /// Repeatedly applies `rewrite` to the whole expression, recording each rewrite as a step.
    ///
    /// Stops when the rule declines, after `max_stage_iterations` rewrites, or when the global
    /// rewrite budget is used up.
    ///
    /// A rewrite that returns to an earlier state means the steps in between were a detour
    /// (typically: expand a product, then factor it back to find a common factor). The detour is
    /// rolled back and the stage continues from the earlier state, which is never worse. Reaching
    /// an abandoned state, or a second revisit within the same stage, is a genuine cycle and stops
    /// the run.
    fn run_stage<F>(&mut self, mut rewrite: F)
    where
        F: FnMut(&Node, &mut DomainTracker) -> Option<LocalRewrite>,
    {
        let mut rolled_back = false;
        for _ in 0..self.config.max_stage_iterations {
            if self.stop.is_some() {
                return;
            }
            if self.rewrites >= self.config.max_total_steps {
                self.stop = Some(StopReason::StepLimit);
                return;
            }
            // Rules see a scratch copy of the tracker: a rejected rewrite must leave no trace.
            let mut tracker = self.tracker.clone();
            let Some((next, local)) = rewrite(&self.current, &mut tracker) else {
                return;
            };
            if serializer::expression(&next) == serializer::expression(&self.current) {
                // A rewrite that does not change the visible expression would only add noise.
                self.current = next;
                return;
            }
            let key = state_key(&next);
            if self.abandoned.contains(&key) {
                self.stop = Some(StopReason::CycleDetected);
                return;
            }
            if let Some(&index) = self.visited.get(&key) {
                if rolled_back {
                    self.stop = Some(StopReason::CycleDetected);
                    return;
                }
                rolled_back = true;
                self.rollback(index);
                continue;
            }
            self.rewrites += 1;
            self.visited.insert(key, self.steps.len() + 1);
            self.tracker = tracker;
            let rule = local.rule.unwrap_or(RuleId::NormalizeExpression);
            self.record(next, local.explanation, rule);
        }
    }

    /// Drops every step after the `index`-th state and makes that state current again.
    fn rollback(&mut self, index: usize) {
        for step in self.steps.drain(index..) {
            let key = state_key(&step.after);
            self.visited.remove(&key);
            self.abandoned.insert(key);
        }
        self.current = match self.steps.last() {
            Some(step) => step.after.clone(),
            None => self.original.clone(),
        };
    }

    fn record(&mut self, after: Node, explanation: impl Into<String>, rule: RuleId) {
        // Every denominator that shows up along the way is a product of original denominators
        // (or numerators of divisors), so its zeros are restrictions of the original expression.
        // This catches e.g. `x != 1` in `(1 + 1/x) / (1 - 1/x)`, which only becomes a polynomial
        // denominator once the inner sums are combined. Constant-zero denominators were already
        // rejected by the initial scan.
        let _ = self.tracker.scan_and_record_derived(&after);
        let before = std::mem::replace(&mut self.current, after.clone());
        self.steps.push(AlgebraStep::new(before, after, explanation, Some(rule)));
    }

    fn finish(self) -> SimplificationResult {
        let restrictions = self.tracker.into_restrictions();

        // Mathematical equivalence verification (Step 5.3)
        if self.config.verify_equivalence && !self.steps.is_empty() {
            let verification = crate::algebra::verify::verify_equivalence(
                &self.original,
                &self.current,
                &restrictions,
                5,
            );
            if let crate::algebra::verify::VerificationResult::Mismatch { point, original_val, simplified_val } = verification {
                // If verification fails, fall back safely to original expression
                let diag = format!(
                    "Equivalence verification failed at test point {:?} (original: {}, simplified: {}). Reverting to original expression.",
                    point, original_val, simplified_val
                );
                return SimplificationResult::new(self.original, Vec::new(), restrictions)
                    .with_diagnostic(diag);
            }
        }

        let diagnostic = self.stop.map(|reason| match reason {
            StopReason::CycleDetected => {
                "Simplification stopped because a transformation would have repeated an earlier \
                 form; the result is equivalent but may not be fully simplified."
            }
            StopReason::StepLimit => {
                "Simplification stopped after reaching the step limit; the result is equivalent \
                 but may not be fully simplified."
            }
            StopReason::PassLimit => {
                "Simplification stopped after reaching the pass limit; the result is equivalent \
                 but may not be fully simplified."
            }
        });
        let mut diagnostic = diagnostic.map(|s| s.to_string());
        if diagnostic.is_none() {
            diagnostic = unsupported_diagnostic(&self.original);
        }

        let result =
            SimplificationResult::new(self.current, self.steps, restrictions);
        match diagnostic {
            Some(message) => result.with_diagnostic(message),
            None => result,
        }
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

/// Canonical form of a polynomial denominator. In the automatic form it is factored when it
/// splits into several factors (or a repeated one), expanded otherwise.
fn canonical_denominator(poly: &Poly, form: DenominatorForm) -> Node {
    let factored = FactoredPoly::from_poly(poly.clone());
    let splits = factored.factors.len() > 1 || factored.factors.iter().any(|f| f.exp > 1);
    let factor = match form {
        DenominatorForm::Auto => splits,
        DenominatorForm::Expanded => false,
        DenominatorForm::Factored => true,
    };
    if factor {
        factored.to_node()
    } else {
        poly.to_node()
    }
}

/// Builds the canonical node for `num / den` (both polynomials, `den` non-zero).
fn canonical_fraction(num: &Poly, den: &Poly, form: DenominatorForm) -> Node {
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
    make_fraction(num.to_node(), canonical_denominator(&den, form))
}

/// Stage 13 rule: brings a polynomial or a quotient of polynomials into its canonical readable
/// form (expanded numerator in descending degree order, factored denominator).
pub fn canonical_form(node: &Node, form: DenominatorForm) -> Option<Node> {
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
    Some(canonical_fraction(&num, &den, form))
}

/// Stage 13: rewrites the whole expression into its canonical form, if that changes it.
pub fn canonicalize_step(root: &Node, form: DenominatorForm) -> Option<LocalRewrite> {
    let canonical = canonical_form(root, form)?;
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

// ---------------------------------------------------------------------------------------------
// Stage 14: binomial form
// ---------------------------------------------------------------------------------------------

fn is_constant_node(node: &Node) -> bool {
    Poly::from_node(node).is_ok_and(|p| p.is_constant())
}

/// Whether `term` is (a constant times) a binomial square or cube, a product of conjugate
/// binomials or a product of two equal binomials.
fn is_compact_binomial_term(term: &Node) -> bool {
    if expand_binomial_node(term).is_some() {
        return true;
    }
    let Node::Binary {
        operator: Operator::Times | Operator::ImplicitTimes,
        left,
        right,
        ..
    } = unwrap_group(term)
    else {
        return false;
    };
    if is_constant_node(left) && is_compact_binomial_term(right)
        || is_constant_node(right) && is_compact_binomial_term(left)
    {
        return true;
    }
    match (Poly::from_node(left), Poly::from_node(right)) {
        (Ok(l), Ok(r)) => l.terms.len() == 2 && l == r,
        _ => false,
    }
}

/// Whether the input is written purely in compact binomial form: binomial powers or products,
/// plus constants (`(x + 3)^2`, `(a - b)(a + b)`, `(x + 4)^2 - 5`).
pub fn is_compact_binomial_input(node: &Node) -> bool {
    let mut terms = Vec::new();
    flatten_additive_terms(node, true, &mut terms);
    let mut has_binomial = false;
    for (_, term) in &terms {
        if is_compact_binomial_term(term) {
            has_binomial = true;
        } else if !is_constant_node(term) {
            return false;
        }
    }
    has_binomial
}

/// Stage 14: writes a polynomial result as a binomial formula, or completes the square when the
/// input spelled out a double product `2 \cdot A \cdot B`.
fn binomial_form_step(root: &Node, style: &BinomialStyle) -> Option<LocalRewrite> {
    let BinomialStyle::Compress { double_product, complete_square: may_complete } = style else {
        return None;
    };
    let poly = Poly::from_node(unwrap_group(root)).ok()?;
    let (after, rule, explanation) = match compress_binomial(&poly) {
        Some((factored, identity)) => (
            factored.to_node(),
            RuleId::ApplyBinomialIdentity,
            format!("Write as a binomial: {}", identity.description()),
        ),
        None if !may_complete => return None,
        None => {
            let (a, b) = double_product.clone().or_else(|| integer_square_shift(&poly))?;
            let after = complete_square(&poly, &a, &b)?;
            let explanation = format!(
                "Complete the square: add and subtract {}, then apply the binomial formula",
                serializer::expression(&(&b * &b).to_node())
            );
            (after, RuleId::CompleteSquare, explanation)
        }
    };
    if serializer::expression(&after) == serializer::expression(root) {
        return None;
    }
    let step = AlgebraStep::new(root.clone(), after.clone(), explanation, Some(rule));
    Some((after, step))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> Node {
        parser::parse(input).unwrap().expression
    }

    /// An expand/factor pair that would loop forever without the visited-state guard.
    fn toggle(node: &Node) -> Option<LocalRewrite> {
        let latex = serializer::expression(node);
        let next = if latex.contains(r"\cdot") {
            parse(r"x^2 - 1")
        } else {
            parse(r"(x - 1) \cdot (x + 1)")
        };
        let step = AlgebraStep::new(node.clone(), next.clone(), "toggle", Some(RuleId::FactorPolynomial));
        Some((next, step))
    }

    #[test]
    fn oscillating_rewrites_are_stopped_by_the_visited_state_guard() {
        let mut pipeline = Pipeline::new(&parse(r"x^2 - 1"), PipelineConfig::default()).unwrap();
        pipeline.run_stage(|node, _| toggle(node));
        // x^2 - 1 -> (x - 1)(x + 1) -> x^2 - 1 rolls the detour back; trying the same rewrite
        // again reaches the abandoned state and stops the run.
        assert!(pipeline.steps.is_empty());
        assert_eq!(serializer::expression(&pipeline.current), "x^{2} - 1");
        assert_eq!(pipeline.stop, Some(StopReason::CycleDetected));
        let result = pipeline.finish();
        assert!(result.diagnostic.unwrap().contains("repeated"));
    }

    #[test]
    fn stage_iterations_are_bounded() {
        let config = PipelineConfig {
            max_stage_iterations: 3,
            ..PipelineConfig::default()
        };
        let mut pipeline = Pipeline::new(&parse("x"), config).unwrap();
        let mut counter = 0;
        pipeline.run_stage(|node, _| {
            counter += 1;
            let next = Node::binary(Operator::Plus, node.clone(), number(counter), 0);
            let step = AlgebraStep::new(node.clone(), next.clone(), "grow", None);
            Some((next, step))
        });
        assert_eq!(pipeline.steps.len(), 3);
        assert_eq!(pipeline.stop, None);
    }

    #[test]
    fn phases_progress_monotonically() {
        let mut pipeline = Pipeline::new(&parse("x"), PipelineConfig::default()).unwrap();
        pipeline.enter(Phase::ExpandCollect);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            pipeline.enter(Phase::NestedFractions)
        }));
        assert!(result.is_err());
    }
}
