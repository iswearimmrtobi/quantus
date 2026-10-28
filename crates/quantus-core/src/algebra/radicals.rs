//! Square roots in the rational pipeline.
//!
//! The pipeline only knows polynomials, so a square root `\sqrt{u}` blocks every rule that touches
//! it. Two rewrites around the pipeline lift that for square roots of polynomials:
//! - A root factor in a denominator is rationalized: `\frac{p}{q \sqrt{u}} = \frac{p \sqrt{u}}{q u}`.
//! - Every distinct root is then an atom, a variable of its own, while the pipeline runs, and is put
//!   back into the result and its steps afterwards. So `\frac{1}{\sqrt{a}} + \frac{\sqrt{a}}{a}`
//!   becomes `\frac{\sqrt{a}}{a} + \frac{\sqrt{a}}{a}` and then `\frac{2 \sqrt{a}}{a}`.
//!
//! Treating a root as independent of its radicand is always valid, if not always the simplest:
//! `\sqrt{a} \cdot \sqrt{a}` stays as it is.

use crate::algebra::domain::{unwrap_group, DomainRestriction};
use crate::algebra::nested::{make_fraction, make_product, split_fraction};
use crate::algebra::poly::{Monomial, Poly, Term};
use crate::algebra::steps::{AlgebraStep, RuleId, SimplificationResult};
use crate::latex::ast::{Node, Operator};
use crate::latex::serializer;

/// The name of the atom variables; it cannot be typed, so it never clashes with the input.
const ATOM: &str = "√";

/// Whether `node` is `\sqrt{u}` with a polynomial radicand.
fn is_square_root(node: &Node) -> bool {
    matches!(node, Node::Root { radicand, degree: None, .. } if Poly::from_node(radicand).is_ok())
}

fn contains_square_root(node: &Node) -> bool {
    is_square_root(node) || node.children().into_iter().any(contains_square_root)
}

/// The factors of a product, groups removed.
fn factors(node: &Node) -> Vec<&Node> {
    match unwrap_group(node) {
        Node::Binary { operator: Operator::Times | Operator::ImplicitTimes, left, right, .. } => {
            let mut all = factors(left);
            all.extend(factors(right));
            all
        }
        other => vec![other],
    }
}

/// `\frac{p}{q \sqrt{u}}` as `\frac{p \sqrt{u}}{q u}`, if the denominator has exactly one root factor.
fn rationalize_fraction(node: &Node) -> Option<Node> {
    let (numerator, denominator) = split_fraction(node)?;
    let denominator = factors(denominator);
    let mut roots = denominator.iter().filter(|factor| is_square_root(factor));
    let root = *roots.next()?;
    if roots.next().is_some() {
        return None;
    }
    let Node::Root { radicand, .. } = root else { return None };
    let rest = denominator
        .iter()
        .filter(|factor| !std::ptr::eq(**factor, root))
        .fold((**radicand).clone(), |product, factor| make_product((*factor).clone(), product));
    Some(make_fraction(make_product(numerator.clone(), root.clone()), rest))
}

/// Rationalizes every denominator of `node`, innermost first.
fn rationalize(node: &Node) -> Node {
    let node = node.clone().map_children(&mut |child| rationalize(&child));
    rationalize_fraction(&node).unwrap_or(node)
}

/// `node` with each distinct square root replaced by an atom; `roots` collects them in order.
fn atomize(node: Node, roots: &mut Vec<Node>) -> Node {
    if is_square_root(&node) {
        let key = serializer::expression(&node);
        let index = match roots.iter().position(|root| serializer::expression(root) == key) {
            Some(index) => index,
            None => {
                roots.push(node);
                roots.len() - 1
            }
        };
        return atom(index);
    }
    node.map_children(&mut |child| atomize(child, roots))
}

fn atom(index: usize) -> Node {
    Node::Variable { name: ATOM.into(), index: (index + 1).to_string(), position: 0 }
}

/// How the atom with `index` is addressed; a polynomial rebuilds its variables from this key.
fn atom_key(index: usize) -> String {
    Node::variable_key(ATOM, &(index + 1).to_string())
}

fn restore(node: Node, roots: &[Node]) -> Node {
    if let Node::Variable { name, index, .. } = &node {
        let key = Node::variable_key(name, index);
        if let Some(root) = (0..roots.len()).position(|i| atom_key(i) == key).map(|i| &roots[i]) {
            return root.clone();
        }
    }
    node.map_children(&mut |child| restore(child, roots))
}

/// An explanation names expressions in LaTeX; their atoms are written back as roots, last first
/// so that the 1st atom never matches inside the 10th.
fn restore_text(text: &str, roots: &[Node]) -> String {
    roots.iter().enumerate().rev().fold(text.to_string(), |text, (index, root)| {
        let root = serializer::expression(root);
        text.replace(&serializer::expression(&atom(index)), &root).replace(&atom_key(index), &root)
    })
}

/// A restriction is a polynomial over variable names, so an atom in it is renamed to its root.
fn restore_restriction(restriction: DomainRestriction, roots: &[Node]) -> DomainRestriction {
    let rename = |var: String| match (0..roots.len()).find(|&i| atom_key(i) == var) {
        Some(i) => serializer::expression(&roots[i]),
        None => var,
    };
    match restriction {
        DomainRestriction::VariableNotEqual { variable, value } => {
            DomainRestriction::VariableNotEqual { variable: rename(variable), value }
        }
        DomainRestriction::PolyNotZero(poly) => DomainRestriction::PolyNotZero(Poly::from_terms(
            poly.terms
                .into_iter()
                .map(|term| {
                    let vars = term.monomial.vars.into_iter().map(|(var, exp)| (rename(var), exp)).collect();
                    Term::new(term.coeff, Monomial::from_map(vars))
                })
                .collect(),
        )),
    }
}

fn restore_step(step: AlgebraStep, roots: &[Node]) -> AlgebraStep {
    AlgebraStep::new(
        restore(step.before, roots),
        restore(step.after, roots),
        restore_text(&step.explanation, roots),
        step.rule,
    )
}

/// Runs `simplify` on `node` with its square roots rationalized and held as atoms, or returns
/// `None` when `node` has no square root of a polynomial.
pub fn simplify_with_roots(
    node: &Node,
    simplify: impl FnOnce(&Node) -> crate::Result<SimplificationResult>,
) -> Option<crate::Result<SimplificationResult>> {
    if !contains_square_root(node) {
        return None;
    }
    let rationalized = rationalize(node);
    let mut roots = Vec::new();
    let atomized = atomize(rationalized.clone(), &mut roots);
    let result = match simplify(&atomized) {
        Ok(result) => result,
        Err(error) => return Some(Err(error)),
    };

    let mut steps = Vec::new();
    if serializer::expression(&rationalized) != serializer::expression(node) {
        steps.push(AlgebraStep::new(
            node.clone(),
            rationalized,
            "Rationalize the denominator: multiply numerator and denominator by the square root",
            Some(RuleId::RationalizeDenominator),
        ));
    }
    // A step that only changed how an atom was written looks like no step once it is a root again.
    steps.extend(
        result.steps.into_iter().map(|step| restore_step(step, &roots)).filter(|step| step.before_latex != step.after_latex),
    );

    let restrictions = result.domain_restrictions.into_iter().map(|r| restore_restriction(r, &roots)).collect();
    let mut restored = SimplificationResult::new(restore(result.node, &roots), steps, restrictions);
    restored.diagnostic = result.diagnostic.map(|text| restore_text(&text, &roots));
    Some(Ok(restored))
}
