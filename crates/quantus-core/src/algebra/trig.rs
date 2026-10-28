//! Identities of the trigonometric functions, as rules on the atomized tree (see `atoms` and
//! `identities`). All of them hold for every argument where both sides are defined.
//!
//! - Symmetry: `\sin(-x) = -\sin x`, `\cos(-x) = \cos x`, `\tan(-x) = -\tan x`.
//! - `\tan x = \frac{\sin x}{\cos x}` and `\cot x = \frac{\cos x}{\sin x}` where they meet another
//!   function of `x` in one product (`\tan x \cos x = \sin x`), and back where a quotient is left.
//! - The Pythagorean identity, in the direction that leaves fewer terms:
//!   `\sin^2 x + \cos^2 x = 1`, `1 - \sin^2 x = \cos^2 x`.
//! - `1 + \tan^2 x = \frac{1}{\cos^2 x}`, `1 + \cot^2 x = \frac{1}{\sin^2 x}`.
//! - Double angles: `2 \sin x \cos x = \sin(2x)`, `\cos^2 x - \sin^2 x = \cos(2x)` (also
//!   `2\cos^2 x - 1` and `1 - 2\sin^2 x`).
//!
//! Sums of angles (`\sin(a + b)`) are not expanded, and `\sin(2x)` is not split: both undo a merge.

use std::collections::{BTreeMap, BTreeSet};

use dashu::base::Signed;
use dashu::rational::RBig;

use crate::algebra::atoms::Atoms;
use crate::algebra::identities::{each_atom, each_poly, everywhere, note, Condition, Context, Local, Rewrite};
use crate::algebra::nested::{make_fraction, split_fraction};
use crate::algebra::poly::{Monomial, Poly};
use crate::algebra::steps::RuleId;
use crate::latex::ast::Node;
use crate::latex::serializer;

const TRIG: [&str; 4] = ["sin", "cos", "tan", "cot"];

/// The name and argument of the atom with `index`, if it is `\sin`, `\cos`, `\tan` or `\cot`.
fn trig_atom(atoms: &Atoms, index: usize) -> Option<(&str, &Node)> {
    match atoms.node(index) {
        Node::Function { name, argument, .. } if TRIG.contains(&name.as_str()) => Some((name.as_str(), argument)),
        _ => None,
    }
}

/// How arguments are compared: `x + 1` and `1 + x` are one argument.
fn argument_key(argument: &Node) -> String {
    Poly::from_node(argument).map_or_else(|_| serializer::expression(argument), |poly| poly.to_plain_string())
}

/// The trigonometric atoms among the variables of `poly`: argument key -> name -> (variable, argument).
fn trig_vars(poly: &Poly, atoms: &Atoms) -> BTreeMap<String, BTreeMap<String, (String, Node)>> {
    let mut found: BTreeMap<String, BTreeMap<String, (String, Node)>> = BTreeMap::new();
    for var in poly.variables() {
        let Some((name, argument)) = atoms.index_of_key(&var).and_then(|index| trig_atom(atoms, index)) else { continue };
        found.entry(argument_key(argument)).or_default().insert(name.to_string(), (var.clone(), argument.clone()));
    }
    found
}

/// The variable for `name(argument)`, a new atom if needed.
fn function(name: &str, argument: &Node, atoms: &mut Atoms) -> Poly {
    let atom = atoms.intern(Node::Function { name: name.into(), argument: Box::new(argument.clone()), position: 0 });
    Poly::from_node(&atom).expect("an atom is a variable")
}

fn latex_of(poly: &Poly) -> String {
    serializer::expression(&poly.to_node())
}

// ---- symmetry ---------------------------------------------------------------------------------

fn symmetry(index: usize, ctx: &mut Context) -> Option<Local> {
    let (name, argument) = trig_atom(ctx.atoms, index)?;
    let argument = Poly::from_node(argument).ok()?;
    if argument.is_zero() || !argument.leading_coeff().is_negative() {
        return None;
    }
    let name = name.to_string();
    let positive = function(&name, &(-&argument).to_node(), ctx.atoms).to_node();
    let result = if name == "cos" { positive } else { Node::Negation { operand: Box::new(positive), position: 0 } };
    let detail = format!("{} = {}", serializer::expression(&ctx.atoms.var(index)), serializer::expression(&result));
    Some(Local::new(result, detail))
}

/// `\sin(-x) = -\sin x`, `\cos(-x) = \cos x`.
pub fn symmetric_arguments(node: &Node, ctx: &mut Context) -> Option<Rewrite> {
    each_atom(node, ctx, "Use the symmetry", RuleId::TrigSymmetry, symmetry)
}

// ---- tan and cot ------------------------------------------------------------------------------

/// The arguments at which a `\tan` or `\cot` shares a product with another function of the same
/// argument, in a term or in a quotient of two terms.
fn shared_products(node: &Node, atoms: &Atoms, found: &mut BTreeSet<String>) {
    let mut check = |vars: Vec<String>| {
        let mut names: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for var in vars {
            if let Some((name, argument)) = atoms.index_of_key(&var).and_then(|index| trig_atom(atoms, index)) {
                names.entry(argument_key(argument)).or_default().insert(name.to_string());
            }
        }
        for (key, names) in names {
            if names.len() >= 2 && (names.contains("tan") || names.contains("cot")) {
                found.insert(key);
            }
        }
    };
    if let Ok(poly) = Poly::from_node(node) {
        for term in &poly.terms {
            check(term.monomial.vars.keys().cloned().collect());
        }
        return;
    }
    if let Some((num, den)) = split_fraction(node) {
        if let (Ok(num), Ok(den)) = (Poly::from_node(num), Poly::from_node(den)) {
            if num.terms.len() == 1 && den.terms.len() == 1 {
                check(num.variables().into_iter().chain(den.variables()).collect());
            }
        }
    }
    for child in node.children() {
        shared_products(child, atoms, found);
    }
}

fn substitute_tan(node: &Node, atoms: &mut Atoms, arguments: &BTreeSet<String>, details: &mut Vec<String>) -> Node {
    if let Some(index) = atoms.index_of(node) {
        if let Some((name, argument)) = trig_atom(atoms, index) {
            if (name == "tan" || name == "cot") && arguments.contains(&argument_key(argument)) {
                let (name, argument) = (name.to_string(), argument.clone());
                let sin = function("sin", &argument, atoms).to_node();
                let cos = function("cos", &argument, atoms).to_node();
                let result = if name == "tan" { make_fraction(sin, cos) } else { make_fraction(cos, sin) };
                note(details, format!("{} = {}", serializer::expression(&atoms.var(index)), serializer::expression(&result)));
                return result;
            }
        }
        return node.clone();
    }
    node.clone().map_children(&mut |child| substitute_tan(&child, atoms, arguments, details))
}

/// `\tan x = \frac{\sin x}{\cos x}` where it meets another function of `x` in a product.
pub fn tan_as_quotient(node: &Node, ctx: &mut Context) -> Option<Rewrite> {
    let mut arguments = BTreeSet::new();
    shared_products(node, ctx.atoms, &mut arguments);
    if arguments.is_empty() {
        return None;
    }
    let mut details = Vec::new();
    let node = substitute_tan(node, ctx.atoms, &arguments, &mut details);
    (!details.is_empty()).then(|| Rewrite {
        node,
        explanation: format!("Write as sine and cosine: {}", details.join(", ")),
        rule: RuleId::TrigQuotient,
        conditions: Vec::new(),
    })
}

/// `\frac{\sin^{k} x}{\cos^{k} x} = \tan^{k} x` for a quotient of two terms in which nothing else
/// depends on `x`, so `tan_as_quotient` does not turn it back.
fn quotient_as_tan(node: &Node, ctx: &mut Context) -> Option<Local> {
    let (num, den) = split_fraction(node)?;
    let (num, den) = (Poly::from_node(num).ok()?, Poly::from_node(den).ok()?);
    let ([top], [bottom]) = (num.terms.as_slice(), den.terms.as_slice()) else { return None };
    let (top_trig, bottom_trig) = (trig_vars(&num, ctx.atoms), trig_vars(&den, ctx.atoms));
    for (key, names) in &top_trig {
        let Some(below) = bottom_trig.get(key) else { continue };
        let (Some((upper, argument)), Some((lower, _)), true) = (names.values().next(), below.values().next(), names.len() == 1 && below.len() == 1) else {
            continue;
        };
        let (upper_name, lower_name) = (names.keys().next()?, below.keys().next()?);
        let result_name = match (upper_name.as_str(), lower_name.as_str()) {
            ("sin", "cos") => "tan",
            ("cos", "sin") => "cot",
            _ => continue,
        };
        let k = top.monomial.vars[upper];
        if bottom.monomial.vars[lower] != k {
            continue;
        }
        let mut top_vars = top.monomial.vars.clone();
        let mut bottom_vars = bottom.monomial.vars.clone();
        top_vars.remove(upper);
        bottom_vars.remove(lower);
        let (argument, upper, lower) = (argument.clone(), upper.clone(), lower.clone());
        let tan = function(result_name, &argument, ctx.atoms).pow(k);
        let num = &Poly::from_term(top.coeff.clone(), Monomial::from_map(top_vars)) * &tan;
        let den = Poly::from_term(bottom.coeff.clone(), Monomial::from_map(bottom_vars));
        let detail = format!("\\frac{{{}}}{{{}}} = {}", latex_of(&Poly::from_var(&upper).pow(k)), latex_of(&Poly::from_var(&lower).pow(k)), latex_of(&tan));
        let result = if den.is_constant() { (&num * &Poly::from_const(RBig::ONE / den.constant_term())).to_node() } else { make_fraction(num.to_node(), den.to_node()) };
        return Some(Local::new(result, detail));
    }
    None
}

pub fn quotients_as_tan(node: &Node, ctx: &mut Context) -> Option<Rewrite> {
    let rewrite = everywhere(node, ctx, "Write the quotient as a tangent", RuleId::TrigQuotient, quotient_as_tan)?;
    // A tangent next to a sine or cosine of its argument would be written as a quotient again.
    let mut shared = BTreeSet::new();
    shared_products(&rewrite.node, ctx.atoms, &mut shared);
    shared.is_empty().then_some(rewrite)
}

// ---- Pythagorean identity ---------------------------------------------------------------------

/// `poly` with `from^{2}` written as `1 - to^{2}` throughout.
fn replace_square(poly: &Poly, from: &str, to: &str) -> Poly {
    let one_minus = &Poly::from_const(RBig::ONE) - &Poly::from_var(to).pow(2);
    let mut sum = Poly::zero();
    for term in &poly.terms {
        let mut vars = term.monomial.vars.clone();
        let exp = vars.remove(from).unwrap_or(0);
        if exp % 2 == 1 {
            vars.insert(from.to_string(), 1);
        }
        let rest = Poly::from_term(term.coeff.clone(), Monomial::from_map(vars));
        sum = &sum + &(&rest * &one_minus.pow(exp / 2));
    }
    sum
}

fn pythagorean_poly(poly: &Poly, ctx: &mut Context, details: &mut Vec<String>, _: &mut Vec<Condition>) -> Option<Poly> {
    let mut best = poly.clone();
    for names in trig_vars(poly, ctx.atoms).values() {
        let (Some((sin, _)), Some((cos, _))) = (names.get("sin"), names.get("cos")) else { continue };
        for (from, to) in [(cos, sin), (sin, cos)] {
            let candidate = replace_square(&best, from, to);
            if candidate.terms.len() < best.terms.len() {
                best = candidate;
                note(details, format!("{} + {} = 1", latex_of(&Poly::from_var(sin).pow(2)), latex_of(&Poly::from_var(cos).pow(2))));
            }
        }
    }
    // With one of the two functions only: 1 - \sin^2 x = \cos^2 x needs the other one.
    for names in trig_vars(&best, ctx.atoms).into_values() {
        for (name, other) in [("sin", "cos"), ("cos", "sin")] {
            let (Some((var, argument)), None) = (names.get(name), names.get(other)) else { continue };
            let (var, argument) = (var.clone(), argument.clone());
            let other_var = function(other, &argument, ctx.atoms);
            let other_key = other_var.variables().into_iter().next()?;
            let candidate = replace_square(&best, &var, &other_key);
            if candidate.terms.len() < best.terms.len() {
                note(details, format!("1 - {} = {}", latex_of(&Poly::from_var(&var).pow(2)), latex_of(&other_var.pow(2))));
                best = candidate;
            }
        }
    }
    (best != *poly).then_some(best)
}

/// `\sin^2 x + \cos^2 x = 1`, in the direction that leaves fewer terms.
pub fn pythagorean(node: &Node, ctx: &mut Context) -> Option<Rewrite> {
    let (mut details, mut conditions) = (Vec::new(), Vec::new());
    let node = each_poly(node, ctx, pythagorean_poly, &mut details, &mut conditions);
    (!details.is_empty()).then(|| Rewrite {
        node,
        explanation: format!("Use the Pythagorean identity: {}", details.join(", ")),
        rule: RuleId::Pythagorean,
        conditions,
    })
}

// ---- 1 + tan^2, double angles -----------------------------------------------------------------

/// `poly / divisor`, if it divides.
fn divide(poly: &Poly, divisor: &Poly) -> Option<Poly> {
    let (quotient, remainder) = poly.div_rem(divisor).ok()?;
    (remainder.is_zero() && !quotient.is_zero()).then_some(quotient)
}

/// `(1 + \tan^2 x) q = \frac{q}{\cos^2 x}` for a polynomial part.
fn tan_square(node: &Node, ctx: &mut Context) -> Option<Local> {
    let poly = Poly::from_node(node).ok()?;
    for names in trig_vars(&poly, ctx.atoms).values() {
        for (name, other) in [("tan", "cos"), ("cot", "sin")] {
            let Some((var, argument)) = names.get(name) else { continue };
            let t = Poly::from_var(var);
            let Some(quotient) = divide(&poly, &(&Poly::from_const(RBig::ONE) + &t.pow(2))) else { continue };
            let (var, argument) = (var.clone(), argument.clone());
            let square = function(other, &argument, ctx.atoms).pow(2);
            let detail = format!("1 + {} = \\frac{{1}}{{{}}}", latex_of(&Poly::from_var(&var).pow(2)), latex_of(&square));
            return Some(Local::new(make_fraction(quotient.to_node(), square.to_node()), detail));
        }
    }
    None
}

pub fn tan_squares(node: &Node, ctx: &mut Context) -> Option<Rewrite> {
    everywhere(node, ctx, "Use the Pythagorean identity", RuleId::Pythagorean, tan_square)
}

fn double_angle(node: &Node, ctx: &mut Context) -> Option<Local> {
    let poly = Poly::from_node(node).ok()?;
    if poly.terms.len() > 8 {
        return None;
    }
    let one = Poly::from_const(RBig::ONE);
    let two = Poly::from_const(RBig::from(2));
    for names in trig_vars(&poly, ctx.atoms).values() {
        let argument = names.values().next()?.1.clone();
        let double = (&two * &Poly::from_node(&argument).ok()?).to_node();
        let sin = names.get("sin").map(|(var, _)| Poly::from_var(var));
        let cos = names.get("cos").map(|(var, _)| Poly::from_var(var));
        // The forms of \cos(2x) with what is there, then \sin x \cos x = \frac{1}{2} \sin(2x).
        let mut forms: Vec<(Poly, &str, RBig)> = Vec::new();
        if let (Some(s), Some(c)) = (&sin, &cos) {
            forms.push((&c.pow(2) - &s.pow(2), "cos", RBig::ONE));
        }
        if let Some(c) = &cos {
            forms.push((&(&two * &c.pow(2)) - &one, "cos", RBig::ONE));
        }
        if let Some(s) = &sin {
            forms.push((&one - &(&two * &s.pow(2)), "cos", RBig::ONE));
        }
        if let (Some(s), Some(c)) = (&sin, &cos) {
            forms.push((s * c, "sin", RBig::ONE / RBig::from(2)));
        }
        for (form, name, factor) in forms {
            let Some(quotient) = divide(&poly, &form) else { continue };
            let merged = function(name, &double, ctx.atoms);
            let result = &(&quotient * &Poly::from_const(factor.clone())) * &merged;
            let shown = if factor == RBig::ONE { merged.clone() } else { &Poly::from_const(factor) * &merged };
            let detail = format!("{} = {}", latex_of(&form), latex_of(&shown));
            return Some(Local::new(result.to_node(), detail));
        }
    }
    None
}

/// `2 \sin x \cos x = \sin(2x)`, `\cos^2 x - \sin^2 x = \cos(2x)`.
pub fn double_angles(node: &Node, ctx: &mut Context) -> Option<Rewrite> {
    everywhere(node, ctx, "Use the double angle", RuleId::DoubleAngle, double_angle)
}
