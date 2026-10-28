//! The identities that tie atoms together (see `atoms`), and the conditions they need.
//!
//! A rule rewrites the atomized tree and names the conditions its rewrite needs, such as
//! `a \geq 0, b \geq 0` for `\sqrt{a} \sqrt{b} = \sqrt{ab}`. The setting `ConditionalIdentities`
//! decides what happens to a condition that does not hold anyway: it is added to the domain of
//! the result, or the rewrite is not made.

use crate::algebra::atoms::Atoms;
use crate::algebra::domain::DomainRestriction;
use crate::algebra::poly::Poly;
use crate::num::Decimal;
use crate::algebra::{logs, powers, radicals};
use crate::algebra::steps::{AlgebraStep, RuleId};
use crate::latex::ast::Node;
use crate::latex::serializer;
use crate::settings::ConditionalIdentities;

use dashu::rational::RBig;

/// What a rewrite needs of an expression (in the atoms) to be valid.
#[derive(Clone, Debug, PartialEq)]
pub enum Condition {
    NonNegative(Node),
    Positive(Node),
    NonZero(Node),
}

impl Condition {
    fn node(&self) -> &Node {
        match self {
            Self::NonNegative(node) | Self::Positive(node) | Self::NonZero(node) => node,
        }
    }

    /// The condition as a restriction of the domain, if its expression is a polynomial.
    fn restriction(&self) -> Option<DomainRestriction> {
        let poly = Poly::from_node(self.node()).ok()?;
        Some(match self {
            Self::NonNegative(_) => DomainRestriction::PolyNonNegative(poly),
            Self::Positive(_) => DomainRestriction::PolyPositive(poly),
            Self::NonZero(_) => DomainRestriction::PolyNotZero(poly),
        })
    }
}

/// What is known about the sign of an expression for every value of its variables.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Sign {
    Unknown,
    NonNegative,
    Positive,
}

/// The sign of an atom on its own: a square root or `|u|` is never negative, `\pi`, `e` and
/// `e^{u}` are positive.
fn atom_sign(atoms: &Atoms, index: usize) -> Sign {
    let is_e = |node: &Node| matches!(node, Node::Constant { name, .. } if name == "e");
    match atoms.node(index) {
        Node::Constant { name, .. } if name == "\\pi" || name == "e" => Sign::Positive,
        Node::Power { base, .. } if is_e(base) => Sign::Positive,
        Node::Function { name, .. } if name == "exp" || name == "cosh" => Sign::Positive,
        Node::Root { degree: None, .. } | Node::Abs { .. } => Sign::NonNegative,
        _ => Sign::Unknown,
    }
}

/// The sign of a polynomial in the atoms: a sum of terms with positive coefficients whose
/// factors are even powers or atoms of known sign.
fn sign(poly: &Poly, atoms: &Atoms) -> Sign {
    let mut positive = false;
    for term in &poly.terms {
        if term.coeff <= RBig::ZERO {
            return Sign::Unknown;
        }
        let mut term_positive = true;
        for (var, exp) in &term.monomial.vars {
            let atom = atoms.index_of_key(var).map_or(Sign::Unknown, |index| atom_sign(atoms, index));
            match atom {
                Sign::Positive => {}
                _ if exp % 2 == 0 || atom == Sign::NonNegative => term_positive = false,
                _ => return Sign::Unknown,
            }
        }
        positive |= term_positive;
    }
    if positive {
        Sign::Positive
    } else {
        Sign::NonNegative
    }
}

/// The value of an expression without variables, if it is a real number.
fn evaluate(node: &Node) -> Option<f64> {
    use crate::latex::ast::Operator;
    let value = match node {
        Node::Number { value, .. } => value.to_rational().to_f64().value(),
        Node::Constant { name, .. } if name == "\\pi" => std::f64::consts::PI,
        Node::Constant { name, .. } if name == "e" => std::f64::consts::E,
        Node::Group { inner, .. } => evaluate(inner)?,
        Node::Negation { operand, .. } => -evaluate(operand)?,
        Node::Abs { operand, .. } => evaluate(operand)?.abs(),
        Node::Binary { operator, left, right, .. } => {
            let (l, r) = (evaluate(left)?, evaluate(right)?);
            match operator {
                Operator::Plus => l + r,
                Operator::Minus => l - r,
                Operator::Times | Operator::ImplicitTimes => l * r,
                Operator::Divide => l / r,
            }
        }
        Node::Fraction { numerator, denominator, .. } => evaluate(numerator)? / evaluate(denominator)?,
        Node::Power { base, exponent, .. } => evaluate(base)?.powf(evaluate(exponent)?),
        Node::Root { radicand, degree, .. } => {
            let n = degree.as_ref().map_or(Some(2.0), |d| evaluate(d))?;
            let u = evaluate(radicand)?;
            if u < 0.0 {
                // Only an odd root of a negative number is real.
                if n.fract() != 0.0 || n as i64 % 2 == 0 {
                    return None;
                }
                -(-u).powf(1.0 / n)
            } else {
                u.powf(1.0 / n)
            }
        }
        Node::Function { name, argument, .. } => {
            let x = evaluate(argument)?;
            match name.as_str() {
                "sin" => x.sin(),
                "cos" => x.cos(),
                "tan" => x.tan(),
                "ln" => x.ln(),
                "exp" => x.exp(),
                _ => return None,
            }
        }
        _ => return None,
    };
    value.is_finite().then_some(value)
}

/// Whether `restriction` (in the atoms) excludes nothing, as `\sqrt{a} \neq -1` or `e^{x} > 0`.
pub fn always_satisfied(restriction: &DomainRestriction, atoms: &Atoms) -> bool {
    // Without variables the restriction has a value to look at: \ln(2) \neq 0.
    let value = |poly: &Poly| evaluate(&atoms.restore(poly.to_node()));
    match restriction {
        DomainRestriction::VariableNotEqual { variable, value: excluded } => {
            if let Some(v) = value(&Poly::from_var(variable)) {
                return (v - excluded.to_f64().value()).abs() > 1e-12;
            }
        }
        DomainRestriction::PolyNotZero(poly) => {
            if let Some(v) = value(poly) {
                return v.abs() > 1e-12;
            }
        }
        DomainRestriction::PolyPositive(poly) => {
            if let Some(v) = value(poly) {
                return v > 1e-12;
            }
        }
        DomainRestriction::PolyNonNegative(poly) => {
            if let Some(v) = value(poly) {
                return v >= 0.0;
            }
        }
    }
    match restriction {
        DomainRestriction::VariableNotEqual { variable, value } => {
            match atoms.index_of_key(variable).map(|index| atom_sign(atoms, index)) {
                Some(Sign::Positive) => *value <= RBig::ZERO,
                Some(Sign::NonNegative) => *value < RBig::ZERO,
                _ => false,
            }
        }
        DomainRestriction::PolyNotZero(poly) | DomainRestriction::PolyPositive(poly) => sign(poly, atoms) == Sign::Positive,
        DomainRestriction::PolyNonNegative(poly) => sign(poly, atoms) != Sign::Unknown,
    }
}

/// The polynomial a `\neq 0` restriction is about.
fn not_zero_poly(restriction: &DomainRestriction) -> Option<Poly> {
    match restriction {
        DomainRestriction::VariableNotEqual { variable, value } if *value == RBig::ZERO => Some(Poly::from_var(variable)),
        DomainRestriction::PolyNotZero(poly) => Some(poly.clone()),
        _ => None,
    }
}

/// What an atom being nonzero means for what it is built from: `\sqrt[n]{u}`, `|u|` and `u^{y}`
/// are zero only where `u` is.
fn atom_not_zero(poly: &Poly, atoms: &Atoms) -> Option<Vec<DomainRestriction>> {
    let [term] = poly.terms.as_slice() else { return None };
    let [(var, _)] = term.monomial.vars.iter().collect::<Vec<_>>()[..] else { return None };
    let inner = match atoms.node(atoms.index_of_key(var)?) {
        Node::Root { radicand, .. } => radicand,
        Node::Abs { operand, .. } => operand,
        Node::Power { base, .. } => base,
        _ => return None,
    };
    Some(crate::algebra::domain::solve_poly_restrictions(&Poly::from_node(inner).ok()?))
}

/// `p + q \sqrt{u}` for a polynomial linear in a square root: `p` and `q`, and `u`.
fn conjugate_norm(poly: &Poly, atoms: &Atoms) -> Option<Poly> {
    for var in poly.variables() {
        let Some(Node::Root { radicand, degree: None, .. }) = atoms.index_of_key(&var).map(|i| atoms.node(i)) else { continue };
        if poly.degree_of(&var) != 1 {
            continue;
        }
        let radicand = Poly::from_node(radicand).ok()?;
        let (mut p, mut q) = (Poly::zero(), Poly::zero());
        for term in &poly.terms {
            let mut vars = term.monomial.vars.clone();
            if vars.remove(&var).is_some() {
                q = &q + &Poly::from_term(term.coeff.clone(), crate::algebra::poly::Monomial::from_map(vars));
            } else {
                p = &p + &Poly::from_term(term.coeff.clone(), term.monomial.clone());
            }
        }
        return Some(square_roots_through_radicands(&(&(&p * &p) - &(&(&q * &q) * &radicand)), atoms));
    }
    None
}

/// `poly` with `\sqrt{u}^{2} = u` applied to each of its square roots.
fn square_roots_through_radicands(poly: &Poly, atoms: &Atoms) -> Poly {
    let mut sum = Poly::zero();
    for term in &poly.terms {
        let mut product = Poly::from_const(term.coeff.clone());
        for (var, &exp) in &term.monomial.vars {
            let radicand = match atoms.index_of_key(var).map(|i| atoms.node(i)) {
                Some(Node::Root { radicand, degree: None, .. }) if exp >= 2 => Poly::from_node(radicand).ok(),
                _ => None,
            };
            product = match radicand {
                Some(radicand) => &(&product * &radicand.pow(exp / 2)) * &Poly::from_var(var).pow(exp % 2),
                None => &product * &Poly::from_var(var).pow(exp),
            };
        }
        sum = &sum + &product;
    }
    sum
}

/// The restrictions without the ones others imply, written through what atoms are built from.
pub fn simplify_restrictions(restrictions: Vec<DomainRestriction>, atoms: &Atoms) -> Vec<DomainRestriction> {
    let mut expanded: Vec<DomainRestriction> = Vec::new();
    for restriction in restrictions {
        let replaced = not_zero_poly(&restriction).and_then(|poly| atom_not_zero(&poly, atoms));
        for r in replaced.unwrap_or_else(|| vec![restriction]) {
            if !expanded.contains(&r) {
                expanded.push(r);
            }
        }
    }
    // p + q\sqrt{u} \neq 0 follows from p^{2} - q^{2} u \neq 0, which rationalizing brings in.
    let implied = |restriction: &DomainRestriction| {
        let Some(norm) = not_zero_poly(restriction).and_then(|poly| conjugate_norm(&poly, atoms)) else { return false };
        let needed = crate::algebra::domain::solve_poly_restrictions(&norm);
        !needed.is_empty() && needed.iter().all(|r| expanded.contains(r))
    };
    let kept: Vec<DomainRestriction> = expanded.iter().filter(|r| !implied(r)).cloned().collect();
    crate::algebra::domain::merge_inequalities(kept).into_iter().filter(|r| !always_satisfied(r, atoms)).collect()
}

/// What a rule may see and decide on.
pub struct Context<'a> {
    pub atoms: &'a mut Atoms,
    policy: ConditionalIdentities,
    /// The domain so far, in the atoms.
    known: &'a [DomainRestriction],
}

impl Context<'_> {
    /// Whether `condition` holds without being added to the domain.
    pub fn holds(&self, condition: &Condition) -> bool {
        let Ok(poly) = Poly::from_node(condition.node()) else { return false };
        // An expression without variables, such as \sqrt{2} - 1, has a value to look at.
        if let Some(value) = evaluate(&self.atoms.restore(condition.node().clone())) {
            return match condition {
                Condition::NonNegative(_) => value >= 0.0,
                Condition::Positive(_) => value > 1e-12,
                Condition::NonZero(_) => value.abs() > 1e-12,
            };
        }
        let sign = sign(&poly, self.atoms);
        let known = |restriction: DomainRestriction| self.known.contains(&restriction);
        let positive = sign == Sign::Positive || known(DomainRestriction::PolyPositive(poly.clone()));
        match condition {
            Condition::NonNegative(_) => {
                positive || sign == Sign::NonNegative || known(DomainRestriction::PolyNonNegative(poly))
            }
            Condition::Positive(_) => positive,
            Condition::NonZero(_) => {
                positive
                    || (poly.is_constant() && !poly.is_zero())
                    || known(DomainRestriction::PolyNotZero(poly.clone()))
                    || poly.variables().len() == 1
                        && poly.terms.len() == 1
                        && poly.terms[0].monomial.total_degree() == 1
                        && poly.variables().iter().all(|var| known(DomainRestriction::var_not_equal(var.clone(), RBig::ZERO)))
            }
        }
    }

    /// Whether a rewrite that needs `conditions` may be made.
    pub fn allows(&self, conditions: &[Condition]) -> bool {
        conditions.iter().all(|condition| {
            self.holds(condition)
                || (self.policy == ConditionalIdentities::WithCondition && condition.restriction().is_some())
        })
    }
}

/// One application of a rule: the rewritten tree, the step it is shown as and what it needs.
pub struct Rewrite {
    pub node: Node,
    pub explanation: String,
    pub rule: RuleId,
    pub conditions: Vec<Condition>,
}

/// A rewrite of one node, as `everywhere` collects them.
pub struct Local {
    pub node: Node,
    /// The identity as applied here: `\sqrt{a} \cdot \sqrt{b} = \sqrt{ab}`.
    pub detail: String,
    pub conditions: Vec<Condition>,
}

impl Local {
    pub fn new(node: Node, detail: String) -> Self {
        Self { node, detail, conditions: Vec::new() }
    }

    pub fn with(mut self, conditions: impl IntoIterator<Item = Condition>) -> Self {
        self.conditions.extend(conditions);
        self
    }
}

pub type LocalRule = fn(&Node, &mut Context) -> Option<Local>;

/// Applies `local` at every node of `node`, innermost first, as one rewrite titled `title`.
pub fn everywhere(node: &Node, ctx: &mut Context, title: &str, rule: RuleId, local: LocalRule) -> Option<Rewrite> {
    fn walk(node: &Node, ctx: &mut Context, local: LocalRule, details: &mut Vec<String>, conditions: &mut Vec<Condition>) -> Node {
        let node = node.clone().map_children(&mut |child| walk(&child, ctx, local, details, conditions));
        match local(&node, ctx) {
            Some(rewrite) if ctx.allows(&rewrite.conditions) => {
                if !details.contains(&rewrite.detail) {
                    details.push(rewrite.detail);
                }
                conditions.extend(rewrite.conditions);
                rewrite.node
            }
            _ => node,
        }
    }
    let mut details = Vec::new();
    let mut conditions = Vec::new();
    let node = walk(node, ctx, local, &mut details, &mut conditions);
    (!details.is_empty()).then(|| Rewrite { node, explanation: format!("{title}: {}", details.join(", ")), rule, conditions })
}

// ---- helpers for the rules -----------------------------------------------------------------

pub(crate) fn number(n: usize) -> Node {
    Node::Number { value: Decimal::from_i64(n as i64), position: 0 }
}

pub(crate) fn power(base: Node, exp: usize) -> Node {
    if exp == 1 {
        base
    } else {
        Node::Power { base: Box::new(base), exponent: Box::new(number(exp)), position: 0 }
    }
}

/// The atoms that occur in `node`, in the order they are met.
pub(crate) fn atoms_in(node: &Node, atoms: &Atoms, found: &mut Vec<usize>) {
    if let Some(index) = atoms.index_of(node) {
        if !found.contains(&index) {
            found.push(index);
        }
    }
    for child in node.children() {
        atoms_in(child, atoms, found);
    }
}

/// `node` with the atom `index` replaced by `replacement`.
pub(crate) fn substitute(node: &Node, index: usize, replacement: &Node, atoms: &Atoms) -> Node {
    if atoms.index_of(node) == Some(index) {
        return replacement.clone();
    }
    node.clone().map_children(&mut |child| substitute(&child, index, replacement, atoms))
}

/// Rewrites each atom of `node` that `rewrite` changes, as one step titled `title`.
pub(crate) fn each_atom(
    node: &Node,
    ctx: &mut Context,
    title: &str,
    rule: RuleId,
    rewrite: fn(usize, &mut Context) -> Option<Local>,
) -> Option<Rewrite> {
    let mut indices = Vec::new();
    atoms_in(node, ctx.atoms, &mut indices);
    let mut current = node.clone();
    let mut details = Vec::new();
    let mut conditions = Vec::new();
    for index in indices {
        let Some(local) = rewrite(index, ctx).filter(|local| ctx.allows(&local.conditions)) else { continue };
        current = substitute(&current, index, &local.node, ctx.atoms);
        details.push(local.detail);
        conditions.extend(local.conditions);
    }
    (!details.is_empty()).then(|| Rewrite { node: current, explanation: format!("{title}: {}", details.join(", ")), rule, conditions })
}

pub(crate) type PolyRule = fn(&Poly, &mut Context, &mut Vec<String>, &mut Vec<Condition>) -> Option<Poly>;

/// Applies `poly_rule` to every polynomial part of `node`, gathering the details it reports.
pub(crate) fn each_poly(node: &Node, ctx: &mut Context, poly_rule: PolyRule, details: &mut Vec<String>, conditions: &mut Vec<Condition>) -> Node {
    if let Ok(poly) = Poly::from_node(node) {
        return poly_rule(&poly, ctx, details, conditions).map_or_else(|| node.clone(), |rewritten| rewritten.to_node());
    }
    node.clone().map_children(&mut |child| each_poly(&child, ctx, poly_rule, details, conditions))
}

pub(crate) fn note(details: &mut Vec<String>, detail: String) {
    if !details.contains(&detail) {
        details.push(detail);
    }
}

pub type Rule = fn(&Node, &mut Context) -> Option<Rewrite>;

/// The identities, in the order they are tried before each run of the pipeline.
const RULES: &[Rule] = &[
    logs::exp_values,
    logs::log_values,
    powers::rational_exponents,
    powers::nest_powers,
    powers::merge_powers,
    powers::divide_powers,
    powers::merge_bases,
    radicals::nest_roots,
    radicals::extract_roots,
    radicals::multiply_roots,
    radicals::merge_roots,
    radicals::extract_roots,
    radicals::rationalize_denominators,
    radicals::multiply_roots,
    radicals::merge_roots,
    radicals::extract_roots,
    logs::merge_logs,
    logs::divide_logs,
    logs::log_values,
];

/// Applies every rule once to `current`, recording each change as a step and each condition
/// that does not hold anyway in `restrictions`; whether anything changed.
pub fn apply_rules(
    current: &mut Node,
    atoms: &mut Atoms,
    policy: ConditionalIdentities,
    restrictions: &mut Vec<DomainRestriction>,
    steps: &mut Vec<AlgebraStep>,
) -> bool {
    let mut changed = false;
    for rule in RULES {
        let mut ctx = Context { atoms, policy, known: restrictions };
        let Some(rewrite) = rule(current, &mut ctx) else { continue };
        if serializer::expression(&rewrite.node) == serializer::expression(current) {
            continue;
        }
        let mut added: Vec<DomainRestriction> = Vec::new();
        for condition in &rewrite.conditions {
            if let Some(restriction) = condition.restriction().filter(|r| !ctx.holds(condition) && !added.contains(r)) {
                added.push(restriction);
            }
        }
        let mut explanation = rewrite.explanation;
        if !added.is_empty() {
            let latex: Vec<String> = added.iter().map(DomainRestriction::to_latex).collect();
            explanation = format!("{explanation}, for {}", latex.join(", "));
        }
        for restriction in added {
            if !restrictions.contains(&restriction) {
                restrictions.push(restriction);
            }
        }
        steps.push(AlgebraStep::new(current.clone(), rewrite.node.clone(), explanation, Some(rewrite.rule)));
        *current = rewrite.node;
        changed = true;
    }
    changed
}
