//! Square roots in the rational pipeline.
//!
//! The pipeline only knows polynomials, so a square root `\sqrt{u}` blocks every rule that touches
//! it. Two rewrites around the pipeline lift that for square roots of polynomials:
//! - Every distinct root is an atom, a variable of its own, while the pipeline runs, and is put
//!   back into the result and its steps afterwards.
//! - Around each run the atoms are tied back to their radicands: a product of a root with itself
//!   is its radicand, `\sqrt{u} \cdot \sqrt{u} = u`, and a root factor in a denominator is
//!   rationalized, `\frac{p}{q \sqrt{u}} = \frac{p \sqrt{u}}{q u}`. When either changes the
//!   pipeline's result, it runs again.
//!
//! So `\frac{1}{\sqrt{a}} + \frac{\sqrt{a}}{a}` becomes `\frac{\sqrt{a}}{a} + \frac{\sqrt{a}}{a}`
//! and then `\frac{2 \sqrt{a}}{a}`, and `(\sqrt{a} + 1)(\sqrt{a} - 1)` becomes `a - 1`.

use crate::algebra::domain::{unwrap_group, DomainRestriction};
use crate::algebra::nested::{make_fraction, make_product, split_fraction};
use crate::algebra::poly::{Monomial, Poly, Term};
use crate::algebra::steps::{AlgebraStep, RuleId, SimplificationResult};
use crate::latex::ast::{Node, Operator};
use crate::latex::serializer;

use dashu::rational::RBig;

/// The name of the atom variables; it cannot be typed, so it never clashes with the input.
const ATOM: &str = "√";

/// Runs of the pipeline at most; each one after the first starts from a root it could not see.
const MAX_ROUNDS: usize = 4;

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

/// Which atom a variable key names, if any.
fn atom_index(key: &str, roots: &[Node]) -> Option<usize> {
    (0..roots.len()).find(|&i| atom_key(i) == key)
}

fn radicand(root: &Node) -> &Node {
    match root {
        Node::Root { radicand, .. } => radicand,
        other => other,
    }
}

/// `\frac{p}{q \sqrt{u}}` as `\frac{p \sqrt{u}}{q u}`, for the first root factor of the denominator.
fn rationalize_fraction(node: &Node, roots: &[Node]) -> Option<Node> {
    let (numerator, denominator) = split_fraction(node)?;
    let denominator = factors(denominator);
    let (position, index) = denominator.iter().enumerate().find_map(|(position, factor)| match factor {
        Node::Variable { name, index, .. } => atom_index(&Node::variable_key(name, index), roots).map(|i| (position, i)),
        _ => None,
    })?;
    let rest = denominator
        .iter()
        .enumerate()
        .filter(|(other, _)| *other != position)
        .fold(radicand(&roots[index]).clone(), |product, (_, factor)| make_product((*factor).clone(), product));
    Some(make_fraction(make_product(numerator.clone(), atom(index)), rest))
}

/// Rationalizes every denominator of `node`, innermost first. Each rewrite takes one root out of
/// a denominator, so this ends.
fn rationalize(node: &Node, roots: &[Node]) -> Node {
    let mut node = node.clone().map_children(&mut |child| rationalize(&child, roots));
    while let Some(next) = rationalize_fraction(&node, roots) {
        node = next;
    }
    node
}

/// `poly` with every power of a root above the first written through its radicand,
/// `\sqrt{u}^{2k + r} = u^k \sqrt{u}^r`, or `None` if it has no such power.
fn reduce_poly(poly: &Poly, roots: &[Node]) -> Option<Poly> {
    let reducible = |term: &Term| term.monomial.vars.iter().any(|(var, &exp)| exp >= 2 && atom_index(var, roots).is_some());
    if !poly.terms.iter().any(reducible) {
        return None;
    }
    let mut sum = Poly::zero();
    for term in &poly.terms {
        let mut product = Poly::from_const(term.coeff.clone());
        let mut vars = std::collections::BTreeMap::new();
        for (var, &exp) in &term.monomial.vars {
            match atom_index(var, roots) {
                Some(index) if exp >= 2 => {
                    // The radicand is a polynomial: `is_square_root` only lets those through.
                    let radicand = Poly::from_node(radicand(&roots[index])).ok()?;
                    product = &product * &radicand.pow(exp / 2);
                    if exp % 2 == 1 {
                        vars.insert(var.clone(), 1);
                    }
                }
                _ => {
                    vars.insert(var.clone(), exp);
                }
            }
        }
        sum = &sum + &(&product * &Poly::from_term(RBig::ONE, Monomial::from_map(vars)));
    }
    Some(sum)
}

/// Applies `\sqrt{u} \cdot \sqrt{u} = u` to every polynomial part of `node`.
fn reduce(node: &Node, roots: &[Node]) -> Node {
    if let Ok(poly) = Poly::from_node(node) {
        return reduce_poly(&poly, roots).map_or_else(|| node.clone(), |reduced| reduced.to_node());
    }
    node.clone().map_children(&mut |child| reduce(&child, roots))
}

fn restore(node: Node, roots: &[Node]) -> Node {
    if let Node::Variable { name, index, .. } = &node {
        if let Some(index) = atom_index(&Node::variable_key(name, index), roots) {
            return roots[index].clone();
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
    let rename = |var: String| match atom_index(&var, roots) {
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

/// The roots whose powers `reduce` would write through their radicands, in atom order.
fn reducible_roots(node: &Node, roots: &[Node], found: &mut Vec<usize>) {
    if let Ok(poly) = Poly::from_node(node) {
        for term in &poly.terms {
            for (var, &exp) in &term.monomial.vars {
                if let Some(index) = atom_index(var, roots).filter(|i| exp >= 2 && !found.contains(i)) {
                    found.push(index);
                }
            }
        }
        return;
    }
    for child in node.children() {
        reducible_roots(child, roots, found);
    }
}

/// Records `current → next` as a step and moves on to `next`, if that changes anything.
fn apply(current: &mut Node, steps: &mut Vec<AlgebraStep>, next: Node, explanation: String, rule: RuleId) -> bool {
    if serializer::expression(&next) == serializer::expression(current) {
        return false;
    }
    steps.push(AlgebraStep::new(current.clone(), next.clone(), explanation, Some(rule)));
    *current = next;
    true
}

/// Applies `\sqrt{u} \cdot \sqrt{u} = u` to `current` as a step naming the roots it multiplies.
fn multiply_roots(current: &mut Node, steps: &mut Vec<AlgebraStep>, roots: &[Node]) -> bool {
    let mut found = Vec::new();
    reducible_roots(current, roots, &mut found);
    let identities: Vec<String> = found
        .iter()
        .map(|&i| {
            let root = serializer::expression(&roots[i]);
            format!("{root} \\cdot {root} = {}", serializer::expression(radicand(&roots[i])))
        })
        .collect();
    let next = reduce(current, roots);
    apply(current, steps, next, format!("Multiply the square roots: {}", identities.join(", ")), RuleId::MultiplyRoots)
}

/// Runs `simplify` on `node` with its square roots held as atoms, or returns `None` when `node`
/// has no square root of a polynomial.
pub fn simplify_with_roots(
    node: &Node,
    mut simplify: impl FnMut(&Node) -> crate::Result<SimplificationResult>,
) -> Option<crate::Result<SimplificationResult>> {
    if !contains_square_root(node) {
        return None;
    }
    let mut roots = Vec::new();
    let mut current = atomize(node.clone(), &mut roots);
    let mut steps = Vec::new();
    let mut restrictions: Vec<DomainRestriction> = Vec::new();
    let mut diagnostic = None;

    for round in 0..MAX_ROUNDS {
        let mut multiplied = multiply_roots(&mut current, &mut steps, &roots);
        let next = rationalize(&current, &roots);
        let rationalized = apply(
            &mut current,
            &mut steps,
            next,
            "Rationalize the denominator: multiply numerator and denominator by the square root".into(),
            RuleId::RationalizeDenominator,
        );
        // Rationalizing can put a root next to itself in the numerator.
        multiplied |= multiply_roots(&mut current, &mut steps, &roots);
        // After the first run, the pipeline's result is final unless a root rewrite changed it.
        if round > 0 && !multiplied && !rationalized {
            break;
        }
        let result = match simplify(&current) {
            Ok(result) => result,
            Err(error) => return Some(Err(error)),
        };
        steps.extend(result.steps);
        for restriction in result.domain_restrictions {
            if !restrictions.contains(&restriction) {
                restrictions.push(restriction);
            }
        }
        diagnostic = result.diagnostic;
        current = result.node;
    }

    // A step that only changed how an atom was written looks like no step once it is a root again.
    let steps = steps.into_iter().map(|step| restore_step(step, &roots)).filter(|step| step.before_latex != step.after_latex).collect();
    let restrictions = restrictions.into_iter().map(|r| restore_restriction(r, &roots)).collect();
    let mut restored = SimplificationResult::new(restore(current, &roots), steps, restrictions);
    restored.diagnostic = diagnostic.map(|text| restore_text(&text, &roots));
    Some(Ok(restored))
}
