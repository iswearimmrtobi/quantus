//! Identities of powers with exponents the polynomials cannot hold (`a^{x}`, `a^{\frac{1}{2}}`,
//! `e^{x}`), as rules on the atomized tree (see `atoms` and `identities`).
//!
//! - A rational exponent is a root: `a^{\frac{1}{2}} = \sqrt{a}`, `a^{-\frac{2}{3}} = \frac{1}{\sqrt[3]{a^{2}}}`.
//! - Powers of one base are multiplied and divided through their exponents:
//!   `a^{x} a^{y} = a^{x + y}`, `\frac{a^{x}}{a^{y}} = a^{x - y}`, `a \cdot a^{x} = a^{x + 1}`.
//! - A power of a power is one power: `(a^{x})^{y} = a^{xy}`.
//! - Powers with one exponent are one power: `a^{x} b^{x} = (ab)^{x}`.
//!
//! For a symbolic exponent these need a positive base. The splitting directions
//! (`a^{x + y} = a^{x} a^{y}`, `(ab)^{x} = a^{x} b^{x}`) are not applied: they undo the merges.

use std::collections::BTreeMap;

use dashu::base::Signed;
use dashu::integer::IBig;
use dashu::rational::RBig;

use crate::algebra::atoms::Atoms;
use crate::algebra::identities::{each_atom, each_poly, everywhere, note, Condition, Context, Local, Rewrite};
use crate::algebra::nested::{make_fraction, split_fraction};
use crate::algebra::poly::{Monomial, Poly};
use crate::algebra::radicals::make_root;
use crate::algebra::steps::RuleId;
use crate::latex::ast::Node;
use crate::latex::serializer;
use crate::num::Decimal;

/// The base and exponent (as a polynomial) of the atom with `index`, if it is a power.
fn power_atom(atoms: &Atoms, index: usize) -> Option<(&Node, Poly)> {
    match atoms.node(index) {
        Node::Power { base, exponent, .. } => Some((base, Poly::from_node(exponent).ok()?)),
        _ => None,
    }
}

fn integer_power(base: Node, exp: &IBig) -> Node {
    Node::Power { base: Box::new(base), exponent: Box::new(Node::Number { value: Decimal::new(exp.clone(), 0), position: 0 }), position: 0 }
}

/// The integer a constant polynomial stands for, if it is one.
fn as_integer(poly: &Poly) -> Option<IBig> {
    let value = poly.constant_term();
    (poly.is_constant() && value.denominator() == &dashu::integer::UBig::ONE).then(|| value.numerator().clone())
}

/// `base^{exponent}` as a node: a polynomial factor for a positive integer exponent, an atom otherwise.
fn power_of(base: &Node, exponent: &Poly, atoms: &mut Atoms) -> Option<Poly> {
    match as_integer(exponent) {
        Some(k) if !k.is_negative() => Some(Poly::from_node(base).ok()?.pow(usize::try_from(k).ok()?)),
        // A negative integer power belongs in a denominator, which a term cannot hold.
        Some(_) => None,
        None => {
            let atom = atoms.intern(Node::Power { base: Box::new(base.clone()), exponent: Box::new(exponent.to_node()), position: 0 });
            Poly::from_node(&atom).ok()
        }
    }
}

fn latex(node: &Node) -> String {
    serializer::expression(node)
}

// ---- rational exponents -----------------------------------------------------------------------

fn rational_exponent(index: usize, ctx: &mut Context) -> Option<Local> {
    let (base, exponent) = power_atom(ctx.atoms, index)?;
    if !exponent.is_constant() {
        return None;
    }
    let value = exponent.constant_term();
    let q = usize::try_from(value.denominator().clone()).ok().filter(|&q| q >= 2)?;
    let p = value.numerator().clone();
    let magnitude = if p.is_negative() { -p.clone() } else { p.clone() };
    let radicand = if magnitude == IBig::ONE { base.clone() } else { integer_power(base.clone(), &magnitude) };
    let root = ctx.atoms.intern(make_root(q, radicand));
    let result = if p.is_negative() { make_fraction(Node::Number { value: Decimal::from_i64(1), position: 0 }, root) } else { root };
    let detail = format!("{} = {}", latex(&ctx.atoms.var(index)), latex(&result));
    Some(Local::new(result, detail))
}

/// `a^{\frac{p}{q}} = \sqrt[q]{a^{p}}`.
pub fn rational_exponents(node: &Node, ctx: &mut Context) -> Option<Rewrite> {
    each_atom(node, ctx, "Write the rational exponent as a root", RuleId::RationalExponent, rational_exponent)
}

// ---- powers of one base -----------------------------------------------------------------------

/// One factor of a term as a power: its base, exponent and the variable it was.
struct Part {
    key: String,
    base: Node,
    exponent: Poly,
    var: String,
    is_atom: bool,
    /// Whether it is a factor of a denominator.
    below: bool,
    /// How often the variable occurs in the term.
    count: usize,
}

/// The factors of `monomial` as powers, with `sign` on the exponents (-1 for a denominator).
fn parts(monomial: &Monomial, atoms: &Atoms, sign: i64) -> Vec<Part> {
    let mut parts = Vec::new();
    for (var, &exp) in &monomial.vars {
        let scale = Poly::from_const(RBig::from(sign * exp as i64));
        let (base, exponent, is_atom) = match atoms.index_of_key(var).and_then(|i| power_atom(atoms, i)) {
            Some((base, exponent)) => (base.clone(), &exponent * &scale, true),
            None => (Node::Variable { name: var.clone(), index: String::new(), position: 0 }, scale, false),
        };
        // A variable rebuilt from its key has an empty index; key the base the same way.
        let key = Poly::from_node(&base).map_or_else(|_| latex(&base), |poly| poly.to_plain_string());
        parts.push(Part { key, base, exponent, var: var.clone(), is_atom, below: sign < 0, count: exp });
    }
    parts
}

/// The parts of one base that a merge takes together: at least one power with a symbolic exponent
/// and something to merge it with, or such a power taken more than once: `(a^{x})^{2} = a^{2x}`.
fn groups(parts: Vec<Part>) -> Vec<Vec<Part>> {
    let mut by_base: BTreeMap<String, Vec<Part>> = BTreeMap::new();
    for part in parts {
        by_base.entry(part.key.clone()).or_default().push(part);
    }
    by_base
        .into_values()
        .filter(|group| group.iter().any(|p| p.is_atom) && (group.len() >= 2 || group[0].count >= 2))
        .collect()
}

/// What merging `group` needs: a positive base, unless it is one power taken an integer number
/// of times, which holds wherever the power is defined.
fn merge_conditions(group: &[Part]) -> Vec<Condition> {
    if group.len() == 1 {
        Vec::new()
    } else {
        vec![Condition::Positive(group[0].base.clone())]
    }
}

fn describe(group: &[Part], result: &Node) -> String {
    let factors: Vec<String> = group
        .iter()
        .map(|part| latex(&Node::Power { base: Box::new(part.base.clone()), exponent: Box::new(part.exponent.to_node()), position: 0 }))
        .collect();
    format!("{} = {}", factors.join(" \\cdot "), latex(result))
}

/// `a^{x} a^{y} = a^{x + y}` in every term of `poly`.
fn merge_poly(poly: &Poly, ctx: &mut Context, details: &mut Vec<String>, conditions: &mut Vec<Condition>) -> Option<Poly> {
    let mut changed = false;
    let mut sum = Poly::zero();
    for term in &poly.terms {
        let mut vars = term.monomial.vars.clone();
        let mut factor = Poly::from_const(RBig::ONE);
        for group in groups(parts(&term.monomial, ctx.atoms, 1)) {
            let needs = merge_conditions(&group);
            if !ctx.allows(&needs) {
                continue;
            }
            let exponent = group.iter().fold(Poly::zero(), |total, part| &total + &part.exponent);
            let Some(merged) = power_of(&group[0].base, &exponent, ctx.atoms) else { continue };
            for part in &group {
                vars.remove(&part.var);
            }
            note(details, describe(&group, &merged.to_node()));
            factor = &factor * &merged;
            conditions.extend(needs);
            changed = true;
        }
        sum = &sum + &(&Poly::from_term(term.coeff.clone(), Monomial::from_map(vars)) * &factor);
    }
    changed.then_some(sum)
}

/// `a^{x} a^{y} = a^{x + y}`, `a \cdot a^{x} = a^{x + 1}`.
pub fn merge_powers(node: &Node, ctx: &mut Context) -> Option<Rewrite> {
    let (mut details, mut conditions) = (Vec::new(), Vec::new());
    let merged = each_poly(node, ctx, merge_poly, &mut details, &mut conditions);
    (!details.is_empty()).then(|| Rewrite {
        node: merged,
        explanation: format!("Add the exponents: {}", details.join(", ")),
        rule: RuleId::MergePowers,
        conditions,
    })
}

/// `\frac{a^{x}}{a^{y}} = a^{x - y}` for a fraction of two single terms.
fn divide_at(node: &Node, ctx: &mut Context) -> Option<Local> {
    let (numerator, denominator) = split_fraction(node)?;
    let (num, den) = (Poly::from_node(numerator).ok()?, Poly::from_node(denominator).ok()?);
    if num.terms.len() != 1 || den.terms.len() != 1 {
        return None;
    }
    let mut all = parts(&num.terms[0].monomial, ctx.atoms, 1);
    all.extend(parts(&den.terms[0].monomial, ctx.atoms, -1));
    let mut num_vars = num.terms[0].monomial.vars.clone();
    let mut den_vars = den.terms[0].monomial.vars.clone();
    let mut num_factor = Poly::from_const(RBig::ONE);
    let mut den_factor = Poly::from_const(RBig::ONE);
    let mut details = Vec::new();
    let mut conditions = Vec::new();
    for group in groups(all) {
        // Only a base on both sides is divided.
        if !group.iter().any(|part| part.below) || group.iter().all(|part| part.below) {
            continue;
        }
        let condition = Condition::Positive(group[0].base.clone());
        if !ctx.allows(&[condition.clone()]) {
            continue;
        }
        let exponent = group.iter().fold(Poly::zero(), |total, part| &total + &part.exponent);
        let (merged, below) = match as_integer(&exponent) {
            Some(k) if k.is_negative() => (power_of(&group[0].base, &Poly::from_const(RBig::from(-k)), ctx.atoms)?, true),
            _ => (power_of(&group[0].base, &exponent, ctx.atoms)?, false),
        };
        for part in &group {
            if part.below {
                den_vars.remove(&part.var);
            } else {
                num_vars.remove(&part.var);
            }
        }
        if below {
            den_factor = &den_factor * &merged;
        } else {
            num_factor = &num_factor * &merged;
        }
        details.push(describe(&group, &if below { make_fraction(Node::Number { value: Decimal::from_i64(1), position: 0 }, merged.to_node()) } else { merged.to_node() }));
        conditions.push(condition);
    }
    if details.is_empty() {
        return None;
    }
    let num = &Poly::from_term(num.terms[0].coeff.clone(), Monomial::from_map(num_vars)) * &num_factor;
    let den = &Poly::from_term(den.terms[0].coeff.clone(), Monomial::from_map(den_vars)) * &den_factor;
    Some(Local::new(make_fraction(num.to_node(), den.to_node()), details.join(", ")).with(conditions))
}

pub fn divide_powers(node: &Node, ctx: &mut Context) -> Option<Rewrite> {
    everywhere(node, ctx, "Subtract the exponents", RuleId::MergePowers, divide_at)
}

// ---- powers of powers, powers with one exponent -----------------------------------------------

fn nest_power(index: usize, ctx: &mut Context) -> Option<Local> {
    let (base, outer) = power_atom(ctx.atoms, index)?;
    let inner_index = ctx.atoms.index_of(base)?;
    let (inner_base, inner) = match ctx.atoms.node(inner_index) {
        Node::Power { base, exponent, .. } => ((**base).clone(), Poly::from_node(exponent).ok()?),
        // (\sqrt[n]{u})^{x} = u^{\frac{x}{n}}
        Node::Root { radicand, degree, .. } => {
            let n = degree.as_ref().map_or(Some(RBig::from(2)), |d| Poly::from_node(d).ok().map(|p| p.constant_term()))?;
            ((**radicand).clone(), Poly::from_const(RBig::ONE / n))
        }
        _ => return None,
    };
    let exponent = &inner * &outer;
    let merged = power_of(&inner_base, &exponent, ctx.atoms)?.to_node();
    let detail = format!("{} = {}", latex(&ctx.atoms.var(index)), latex(&merged));
    Some(Local::new(merged, detail).with([Condition::Positive(inner_base)]))
}

/// `(a^{x})^{y} = a^{xy}`.
pub fn nest_powers(node: &Node, ctx: &mut Context) -> Option<Rewrite> {
    each_atom(node, ctx, "Multiply the exponents", RuleId::NestPowers, nest_power)
}

/// `a^{x} b^{x} = (ab)^{x}` in every term of `poly`.
fn merge_bases_poly(poly: &Poly, ctx: &mut Context, details: &mut Vec<String>, conditions: &mut Vec<Condition>) -> Option<Poly> {
    let mut changed = false;
    let mut sum = Poly::zero();
    for term in &poly.terms {
        let mut by_exponent: BTreeMap<String, Vec<Part>> = BTreeMap::new();
        for part in parts(&term.monomial, ctx.atoms, 1) {
            let single = term.monomial.vars.get(&part.var) == Some(&1);
            if part.is_atom && single && ctx.allows(&[Condition::Positive(part.base.clone())]) {
                by_exponent.entry(part.exponent.to_plain_string()).or_default().push(part);
            }
        }
        let mut vars = term.monomial.vars.clone();
        let mut factor = Poly::from_const(RBig::ONE);
        for group in by_exponent.into_values().filter(|group| group.len() >= 2) {
            let Ok(bases) = group.iter().map(|part| Poly::from_node(&part.base)).collect::<Result<Vec<_>, _>>() else { continue };
            let base = bases.iter().fold(Poly::from_const(RBig::ONE), |product, base| &product * base).to_node();
            let Some(merged) = power_of(&base, &group[0].exponent, ctx.atoms) else { continue };
            for part in &group {
                vars.remove(&part.var);
                conditions.push(Condition::Positive(part.base.clone()));
            }
            note(details, describe(&group, &merged.to_node()));
            factor = &factor * &merged;
            changed = true;
        }
        sum = &sum + &(&Poly::from_term(term.coeff.clone(), Monomial::from_map(vars)) * &factor);
    }
    changed.then_some(sum)
}

/// `a^{x} b^{x} = (ab)^{x}`.
pub fn merge_bases(node: &Node, ctx: &mut Context) -> Option<Rewrite> {
    let (mut details, mut conditions) = (Vec::new(), Vec::new());
    let merged = each_poly(node, ctx, merge_bases_poly, &mut details, &mut conditions);
    (!details.is_empty()).then(|| Rewrite {
        node: merged,
        explanation: format!("Multiply the bases: {}", details.join(", ")),
        rule: RuleId::MergeBases,
        conditions,
    })
}
