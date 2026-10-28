//! Identities of absolute values, as rules on the atomized tree (see `atoms` and `identities`).
//! All of them hold for every value.
//!
//! - Values: `|-3| = 3`, `|-a| = |a|`, `|2a| = 2|a|`, and `|u| = u` for what cannot be negative:
//!   `|a^{2}| = a^{2}`, `|\sqrt{a}| = \sqrt{a}`, `|e^{x}| = e^{x}`.
//! - Even powers: `|a|^{2} = a^{2}`, `|a|^{3} = a^{2} |a|`.
//! - Products and quotients are one absolute value: `|a| |b| = |ab|`, `\frac{|a|}{|b|} = \left|\frac{a}{b}\right|`.

use std::collections::BTreeMap;

use dashu::base::{Abs, Signed};
use dashu::rational::RBig;

use crate::algebra::atoms::Atoms;
use crate::algebra::identities::{each_atom, each_poly, everywhere, note, Condition, Context, Local, Rewrite};
use crate::algebra::nested::{make_fraction, split_fraction};
use crate::algebra::poly::{Monomial, Poly};
use crate::algebra::steps::RuleId;
use crate::latex::ast::Node;
use crate::latex::serializer;
use crate::settings::StepDetail;

/// The operand of the atom with `index`, if it is an absolute value.
fn abs_atom(atoms: &Atoms, index: usize) -> Option<&Node> {
    match atoms.node(index) {
        Node::Abs { operand, .. } => Some(operand),
        _ => None,
    }
}

fn abs_of(operand: Node, atoms: &mut Atoms) -> Poly {
    let atom = atoms.intern(Node::Abs { operand: Box::new(operand), position: 0 });
    Poly::from_node(&atom).expect("an atom is a variable")
}

fn latex_of(poly: &Poly) -> String {
    serializer::expression(&poly.to_node())
}

fn abs_value(index: usize, ctx: &mut Context) -> Option<Local> {
    let operand = abs_atom(ctx.atoms, index)?.clone();
    let before = serializer::expression(&ctx.atoms.var(index));
    // A number keeps its sign for the note: -3 < 0, so |-3| = 3.
    let number = Poly::from_node(&operand).ok().filter(Poly::is_constant).map(|p| (p.constant_term(), serializer::expression(&operand)));
    let result = if ctx.holds(&Condition::NonNegative(operand.clone())) {
        operand
    } else {
        // |c u| = |c| |u| with u primitive and its leading coefficient positive.
        let poly = Poly::from_node(&operand).ok()?;
        if poly.is_zero() {
            return None;
        }
        let mut content = poly.content().abs();
        if poly.leading_coeff().is_negative() {
            content = -content;
        }
        let rest = &poly * &Poly::from_const(RBig::ONE / content.clone());
        if content == RBig::ONE {
            return None;
        }
        let factor = Poly::from_const(content.abs());
        if rest.is_constant() { factor } else { &factor * &abs_of(rest.to_node(), ctx.atoms) }.to_node()
    };
    let detail = format!("{before} = {}", serializer::expression(&result));
    let local = Local::new(result.clone(), detail);
    match number {
        Some((value, operand)) => {
            let sign = if value.is_negative() { "<" } else { "\\geq" };
            let text = format!("{operand} {sign} 0, so {before} = {}", serializer::expression(&result));
            Some(local.with_working(vec![(ctx.atoms.var(index), text, StepDetail::Detailed)], "Take the absolute value"))
        }
        None => Some(local),
    }
}

/// `|-a| = |a|`, `|2a| = 2|a|`, `|a^{2}| = a^{2}`.
pub fn abs_values(node: &Node, ctx: &mut Context) -> Option<Rewrite> {
    each_atom(node, ctx, "Simplify the absolute value", RuleId::AbsValue, abs_value)
}

/// `|u|^{2k + r} = u^{2k} |u|^{r}` in every term of `poly`, and `|a| |b| = |ab|`.
fn abs_poly(poly: &Poly, ctx: &mut Context, details: &mut Vec<String>, _: &mut Vec<Condition>) -> Option<Poly> {
    let mut changed = false;
    let mut sum = Poly::zero();
    for term in &poly.terms {
        let mut vars = BTreeMap::new();
        let mut factor = Poly::from_const(term.coeff.clone());
        let mut singles = Vec::new();
        for (var, &exp) in &term.monomial.vars {
            let Some(operand) = ctx.atoms.index_of_key(var).and_then(|i| abs_atom(ctx.atoms, i)) else {
                vars.insert(var.clone(), exp);
                continue;
            };
            let Ok(operand) = Poly::from_node(operand) else {
                vars.insert(var.clone(), exp);
                continue;
            };
            if exp >= 2 {
                note(details, format!("{} = {}", latex_of(&Poly::from_var(var).pow(2)), latex_of(&operand.pow(2))));
                factor = &factor * &operand.pow(exp - exp % 2);
                changed = true;
            }
            if exp % 2 == 1 {
                singles.push((var.clone(), operand));
            }
        }
        if singles.len() >= 2 {
            let product = singles.iter().fold(Poly::from_const(RBig::ONE), |product, (_, operand)| &product * operand);
            let merged = abs_of(product.to_node(), ctx.atoms);
            let parts: Vec<String> = singles.iter().map(|(var, _)| latex_of(&Poly::from_var(var))).collect();
            note(details, format!("{} = {}", parts.join(" \\cdot "), latex_of(&merged)));
            factor = &factor * &merged;
            changed = true;
        } else {
            for (var, _) in singles {
                vars.insert(var, 1);
            }
        }
        sum = &sum + &(&Poly::from_term(RBig::ONE, Monomial::from_map(vars)) * &factor);
    }
    changed.then_some(sum)
}

/// `|a|^{2} = a^{2}`, `|a| |b| = |ab|`.
pub fn abs_products(node: &Node, ctx: &mut Context) -> Option<Rewrite> {
    let (mut details, mut conditions) = (Vec::new(), Vec::new());
    let node = each_poly(node, ctx, abs_poly, &mut details, &mut conditions);
    (!details.is_empty()).then(|| Rewrite {
        node,
        explanation: format!("Multiply the absolute values: {}", details.join(", ")),
        rule: RuleId::MergeAbs,
        conditions,
        working: Vec::new(),
        last: None,
    })
}

/// `\frac{p |a|}{q |b|} = \frac{p}{q} \left|\frac{a}{b}\right|` for a quotient of two terms.
fn abs_quotient(node: &Node, ctx: &mut Context) -> Option<Local> {
    let (num, den) = split_fraction(node)?;
    let (num, den) = (Poly::from_node(num).ok()?, Poly::from_node(den).ok()?);
    let ([top], [bottom]) = (num.terms.as_slice(), den.terms.as_slice()) else { return None };
    let single_abs = |monomial: &Monomial| -> Option<(String, Node)> {
        monomial.vars.iter().find_map(|(var, &exp)| {
            let operand = ctx.atoms.index_of_key(var).and_then(|i| abs_atom(ctx.atoms, i))?;
            (exp == 1).then(|| (var.clone(), operand.clone()))
        })
    };
    let ((upper, a), (lower, b)) = (single_abs(&top.monomial)?, single_abs(&bottom.monomial)?);
    let mut top_vars = top.monomial.vars.clone();
    let mut bottom_vars = bottom.monomial.vars.clone();
    top_vars.remove(&upper);
    bottom_vars.remove(&lower);
    let merged = abs_of(make_fraction(a, b), ctx.atoms);
    let detail = format!("\\frac{{{}}}{{{}}} = {}", latex_of(&Poly::from_var(&upper)), latex_of(&Poly::from_var(&lower)), latex_of(&merged));
    let num = &Poly::from_term(top.coeff.clone(), Monomial::from_map(top_vars)) * &merged;
    let den = Poly::from_term(bottom.coeff.clone(), Monomial::from_map(bottom_vars));
    let result = if den.is_constant() { (&num * &Poly::from_const(RBig::ONE / den.constant_term())).to_node() } else { make_fraction(num.to_node(), den.to_node()) };
    Some(Local::new(result, detail))
}

/// `\frac{|a|}{|b|} = \left|\frac{a}{b}\right|`.
pub fn abs_quotients(node: &Node, ctx: &mut Context) -> Option<Rewrite> {
    everywhere(node, ctx, "Divide the absolute values", RuleId::MergeAbs, abs_quotient)
}
