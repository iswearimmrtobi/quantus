//! Identities of square roots, as rules on the atomized tree (see `atoms`).
//!
//! - A product of a root with itself is its radicand: `\sqrt{u} \cdot \sqrt{u} = u`.
//! - A root factor in a denominator is rationalized: `\frac{p}{q \sqrt{u}} = \frac{p \sqrt{u}}{q u}`.
//!
//! So `\frac{1}{\sqrt{a}} + \frac{\sqrt{a}}{a}` becomes `\frac{\sqrt{a}}{a} + \frac{\sqrt{a}}{a}`
//! and then `\frac{2 \sqrt{a}}{a}`, and `(\sqrt{a} + 1)(\sqrt{a} - 1)` becomes `a - 1`.

use std::collections::BTreeMap;

use dashu::rational::RBig;

use crate::algebra::atoms::Atoms;
use crate::algebra::identities::{Context, Rewrite};
use crate::algebra::domain::unwrap_group;
use crate::algebra::nested::{make_fraction, make_product, split_fraction};
use crate::algebra::poly::{Monomial, Poly};
use crate::algebra::steps::RuleId;
use crate::latex::ast::{Node, Operator};
use crate::latex::serializer;

/// The radicand of the atom with `index`, if it is a square root of a polynomial.
fn square_root_radicand(atoms: &Atoms, index: usize) -> Option<&Node> {
    match atoms.node(index) {
        Node::Root { radicand, degree: None, .. } if Poly::from_node(radicand).is_ok() => Some(radicand),
        _ => None,
    }
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

/// `\frac{p}{q \sqrt{u}}` as `\frac{p \sqrt{u}}{q u}`, for the first root factor of the denominator.
fn rationalize_fraction(node: &Node, atoms: &Atoms) -> Option<Node> {
    let (numerator, denominator) = split_fraction(node)?;
    let denominator = factors(denominator);
    let (position, index, radicand) = denominator.iter().enumerate().find_map(|(position, factor)| {
        let index = atoms.index_of(factor)?;
        Some((position, index, square_root_radicand(atoms, index)?))
    })?;
    let rest = denominator
        .iter()
        .enumerate()
        .filter(|(other, _)| *other != position)
        .fold(radicand.clone(), |product, (_, factor)| make_product((*factor).clone(), product));
    Some(make_fraction(make_product(numerator.clone(), atoms.var(index)), rest))
}

/// Rationalizes every denominator of `node`, innermost first. Each rewrite takes one root out of
/// a denominator, so this ends.
fn rationalize(node: &Node, atoms: &Atoms) -> Node {
    let mut node = node.clone().map_children(&mut |child| rationalize(&child, atoms));
    while let Some(next) = rationalize_fraction(&node, atoms) {
        node = next;
    }
    node
}

pub fn rationalize_denominators(node: &Node, ctx: &mut Context) -> Option<Rewrite> {
    Some(Rewrite {
        node: rationalize(node, ctx.atoms),
        explanation: "Rationalize the denominator: multiply numerator and denominator by the square root".into(),
        rule: RuleId::RationalizeDenominator,
        conditions: Vec::new(),
    })
}

/// `poly` with every power of a root above the first written through its radicand,
/// `\sqrt{u}^{2k + r} = u^k \sqrt{u}^r`, or `None` if it has no such power. `found` collects the
/// roots it wrote through.
fn reduce_poly(poly: &Poly, atoms: &Atoms, found: &mut Vec<usize>) -> Option<Poly> {
    let reducible = |var: &str, exp: usize| -> Option<(usize, Poly)> {
        let index = atoms.index_of_key(var).filter(|_| exp >= 2)?;
        let radicand = Poly::from_node(square_root_radicand(atoms, index)?).ok()?;
        Some((index, radicand))
    };
    if !poly.terms.iter().any(|term| term.monomial.vars.iter().any(|(var, &exp)| reducible(var, exp).is_some())) {
        return None;
    }
    let mut sum = Poly::zero();
    for term in &poly.terms {
        let mut product = Poly::from_const(term.coeff.clone());
        let mut vars = BTreeMap::new();
        for (var, &exp) in &term.monomial.vars {
            match reducible(var, exp) {
                Some((index, radicand)) => {
                    if !found.contains(&index) {
                        found.push(index);
                    }
                    product = &product * &radicand.pow(exp / 2);
                    if exp % 2 == 1 {
                        vars.insert(var.clone(), 1);
                    }
                }
                None => {
                    vars.insert(var.clone(), exp);
                }
            }
        }
        sum = &sum + &(&product * &Poly::from_term(RBig::ONE, Monomial::from_map(vars)));
    }
    Some(sum)
}

/// Applies `\sqrt{u} \cdot \sqrt{u} = u` to every polynomial part of `node`.
fn reduce(node: &Node, atoms: &Atoms, found: &mut Vec<usize>) -> Node {
    if let Ok(poly) = Poly::from_node(node) {
        return reduce_poly(&poly, atoms, found).map_or_else(|| node.clone(), |reduced| reduced.to_node());
    }
    node.clone().map_children(&mut |child| reduce(&child, atoms, found))
}

pub fn multiply_roots(node: &Node, ctx: &mut Context) -> Option<Rewrite> {
    let atoms = &*ctx.atoms;
    let mut found = Vec::new();
    let reduced = reduce(node, atoms, &mut found);
    let identities: Vec<String> = found
        .iter()
        .filter_map(|&index| {
            let root = atoms.latex(index);
            let radicand = serializer::expression(&atoms.restore(square_root_radicand(atoms, index)?.clone()));
            Some(format!("{root} \\cdot {root} = {radicand}"))
        })
        .collect();
    (!identities.is_empty()).then(|| Rewrite {
        node: reduced,
        explanation: format!("Multiply the square roots: {}", identities.join(", ")),
        rule: RuleId::MultiplyRoots,
        conditions: Vec::new(),
    })
}
