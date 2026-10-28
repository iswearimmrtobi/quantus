//! Identities of logarithms and the exponential function, as rules on the atomized tree (see
//! `atoms` and `identities`). `e^{a} e^{b} = e^{a + b}` and `(e^{a})^{n} = e^{na}` are powers of a
//! positive base (`powers`); `\log_b a = \frac{\ln a}{\ln b}` is how the parser reads `\log_b`.
//!
//! - Values: `\ln 1 = 0`, `\ln e = 1`, `\ln(e^{x}) = x`, `\log 1000 = 3`, `e^{\ln a} = a`,
//!   `e^{2 \ln a} = a^{2}`, and `\exp(x) = e^{x}`.
//! - A sum of logarithms is one logarithm: `\ln a + \ln b = \ln(ab)`,
//!   `\ln a - \ln b = \ln \frac{a}{b}`, `2 \ln a + \ln b = \ln(a^{2} b)`, for `a, b > 0`.
//!
//! A single `n \ln a` stays as it is, and `\ln(ab)` is not split: both would undo the merge.

use std::collections::BTreeMap;

use dashu::base::{Abs, Signed};
use dashu::integer::IBig;
use dashu::rational::RBig;

use crate::algebra::atoms::Atoms;
use crate::algebra::domain::unwrap_group;
use crate::algebra::identities::{each_atom, each_poly, everywhere, note, Condition, Context, Local, Rewrite};
use crate::algebra::nested::{make_fraction, split_fraction};
use crate::algebra::poly::{Monomial, Poly};
use crate::algebra::powers::power_of;
use crate::algebra::steps::RuleId;
use crate::latex::ast::Node;
use crate::latex::serializer;
use crate::num::Decimal;

/// Which logarithm a function name is: `ln`, or `log` and `lg`, which are one base 10 logarithm.
fn log_base(name: &str) -> Option<&'static str> {
    match name {
        "ln" => Some("ln"),
        "log" | "lg" => Some("log"),
        _ => None,
    }
}

/// The base (`"ln"` or `"log"`), name and argument of the atom with `index`, if it is a logarithm.
fn log_atom(atoms: &Atoms, index: usize) -> Option<(&'static str, &str, &Node)> {
    match atoms.node(index) {
        Node::Function { name, argument, .. } => Some((log_base(name)?, name.as_str(), argument)),
        _ => None,
    }
}

fn is_e(node: &Node, atoms: &Atoms) -> bool {
    let node = atoms.index_of(node).map_or(unwrap_group(node), |index| atoms.node(index));
    matches!(node, Node::Constant { name, .. } if name == "e")
}

/// `node` as `base^{exponent}`, looking through the atom it may be.
fn as_power<'a>(node: &'a Node, atoms: &'a Atoms) -> Option<(&'a Node, &'a Node)> {
    let node = atoms.index_of(node).map_or(unwrap_group(node), |index| atoms.node(index));
    match node {
        Node::Power { base, exponent, .. } => Some((base, exponent)),
        _ => None,
    }
}

fn number(value: &IBig) -> Node {
    Node::Number { value: Decimal::new(value.clone(), 0), position: 0 }
}

/// `k` with `value = 10^k`, if there is one.
fn power_of_ten(value: &RBig) -> Option<IBig> {
    if !value.is_positive() {
        return None;
    }
    let (mut numerator, mut denominator) = (value.numerator().clone(), IBig::from(value.denominator().clone()));
    let (mut k, ten) = (IBig::ZERO, IBig::from(10));
    while &numerator % &ten == IBig::ZERO && numerator != IBig::ZERO {
        numerator /= &ten;
        k += IBig::ONE;
    }
    while &denominator % &ten == IBig::ZERO {
        denominator /= &ten;
        k -= IBig::ONE;
    }
    (numerator == IBig::ONE && denominator == IBig::ONE).then_some(k)
}

fn log_value(index: usize, ctx: &mut Context) -> Option<Local> {
    let atoms = &*ctx.atoms;
    let (base, _, argument) = log_atom(atoms, index)?;
    let value = match Poly::from_node(argument).ok().filter(Poly::is_constant).map(|p| p.constant_term()) {
        Some(c) if c == RBig::ONE => number(&IBig::ZERO),
        Some(c) if base == "log" => number(&power_of_ten(&c)?),
        _ if base == "ln" && is_e(argument, atoms) => number(&IBig::ONE),
        _ => {
            let (power_base, exponent) = as_power(argument, atoms)?;
            let matches = if base == "ln" {
                is_e(power_base, atoms)
            } else {
                Poly::from_node(power_base).is_ok_and(|p| p.is_constant() && p.constant_term() == RBig::from(10))
            };
            if !matches {
                return None;
            }
            exponent.clone()
        }
    };
    let detail = format!("{} = {}", serializer::expression(&atoms.var(index)), serializer::expression(&value));
    Some(Local::new(value, detail))
}

/// `\ln 1 = 0`, `\ln e = 1`, `\ln(e^{x}) = x`, `\log 1000 = 3`.
pub fn log_values(node: &Node, ctx: &mut Context) -> Option<Rewrite> {
    each_atom(node, ctx, "Evaluate the logarithm", RuleId::LogValue, log_value)
}

fn exp_value(index: usize, ctx: &mut Context) -> Option<Local> {
    let atoms = &*ctx.atoms;
    match atoms.node(index) {
        // \exp(x) = e^{x}
        Node::Function { name, argument, .. } if name == "exp" => {
            let argument = argument.clone();
            let e = ctx.atoms.intern(Node::Constant { name: "e".into(), position: 0 });
            let power = Node::Power { base: Box::new(e), exponent: argument, position: 0 };
            let detail = format!("{} = {}", serializer::expression(&ctx.atoms.var(index)), serializer::expression(&power));
            let power = crate::algebra::atoms::atomize(power, ctx.atoms);
            Some(Local::new(power, detail))
        }
        // e^{c \ln a} = a^{c}
        Node::Power { base, exponent, .. } if is_e(base, atoms) => {
            let exponent = Poly::from_node(exponent).ok()?;
            let [term] = exponent.terms.as_slice() else { return None };
            let [(var, &1)] = term.monomial.vars.iter().collect::<Vec<_>>()[..] else { return None };
            let log = atoms.index_of_key(var)?;
            let ("ln", _, argument) = log_atom(atoms, log)? else { return None };
            let argument = argument.clone();
            let detail_from = serializer::expression(&atoms.var(index));
            let result = power_of(&argument, &Poly::from_const(term.coeff.clone()), ctx.atoms)
                .map(|p| p.to_node())
                .or_else(|| {
                    // A negative power: e^{-\ln a} = \frac{1}{a}.
                    let positive = power_of(&argument, &Poly::from_const(-term.coeff.clone()), ctx.atoms)?;
                    Some(make_fraction(number(&IBig::ONE), positive.to_node()))
                })?;
            let detail = format!("{detail_from} = {}", serializer::expression(&result));
            Some(Local::new(result, detail).with([Condition::Positive(argument)]))
        }
        _ => None,
    }
}

/// `e^{\ln a} = a`, `e^{c \ln a} = a^{c}`, `\exp(x) = e^{x}`.
pub fn exp_values(node: &Node, ctx: &mut Context) -> Option<Rewrite> {
    each_atom(node, ctx, "Write the exponential", RuleId::ExpValue, exp_value)
}

/// `c_1 \ln a_1 + c_2 \ln a_2 = \ln(a_1^{c_1} a_2^{c_2})` for the terms of `poly` that are an integer
/// multiple of one logarithm.
fn merge_poly(poly: &Poly, ctx: &mut Context, details: &mut Vec<String>, conditions: &mut Vec<Condition>) -> Option<Poly> {
    // The log terms by base: (term index, coefficient, atom, argument).
    let mut by_base: BTreeMap<&'static str, Vec<(usize, IBig, usize, Poly)>> = BTreeMap::new();
    for (position, term) in poly.terms.iter().enumerate() {
        let [(var, &1)] = term.monomial.vars.iter().collect::<Vec<_>>()[..] else { continue };
        let Some(index) = ctx.atoms.index_of_key(var) else { continue };
        let Some((base, _, argument)) = log_atom(ctx.atoms, index) else { continue };
        let Ok(argument) = Poly::from_node(argument) else { continue };
        if term.coeff.denominator() != &dashu::integer::UBig::ONE {
            continue;
        }
        if !ctx.allows(&[Condition::Positive(argument.to_node())]) {
            continue;
        }
        by_base.entry(base).or_default().push((position, term.coeff.numerator().clone(), index, argument));
    }
    let mut merged_terms = Vec::new();
    let mut sum = Poly::zero();
    for (_, members) in by_base.into_iter().filter(|(_, members)| members.len() >= 2) {
        let (mut num, mut den) = (Poly::from_const(RBig::ONE), Poly::from_const(RBig::ONE));
        let mut parts = Vec::new();
        for (position, coeff, index, argument) in &members {
            let power = argument.pow(usize::try_from(if coeff.is_negative() { -coeff.clone() } else { coeff.clone() }).ok()?);
            if coeff.is_negative() {
                den = &den * &power;
            } else {
                num = &num * &power;
            }
            merged_terms.push(*position);
            conditions.push(Condition::Positive(argument.to_node()));
            let term = Poly::from_term(RBig::from(coeff.clone()), Monomial::from_var(ctx.atoms.key(*index)));
            parts.push(serializer::expression(&term.to_node()));
        }
        // Cancel what the numerator and denominator share: \ln a - \ln a = \ln 1.
        let common = num.gcd(&den);
        if !common.is_constant() {
            num = num.div_rem(&common).ok()?.0;
            den = den.div_rem(&common).ok()?.0;
        }
        let (_, name, _) = log_atom(ctx.atoms, members[0].2)?;
        let name = name.to_string();
        let argument = if den.is_constant() { (&num * &Poly::from_const(RBig::ONE / den.constant_term())).to_node() } else { make_fraction(num.to_node(), den.to_node()) };
        let log = ctx.atoms.intern(Node::Function { name, argument: Box::new(argument), position: 0 });
        note(details, format!("{} = {}", parts.join(" + ").replace("+ -", "- "), serializer::expression(&log)));
        sum = &sum + &Poly::from_node(&log).ok()?;
    }
    if merged_terms.is_empty() {
        return None;
    }
    for (position, term) in poly.terms.iter().enumerate() {
        if !merged_terms.contains(&position) {
            sum = &sum + &Poly::from_term(term.coeff.clone(), term.monomial.clone());
        }
    }
    Some(sum)
}

/// `\ln a + \ln b = \ln(ab)`, `\ln a - \ln b = \ln \frac{a}{b}` for `a, b > 0`.
pub fn merge_logs(node: &Node, ctx: &mut Context) -> Option<Rewrite> {
    let (mut details, mut conditions) = (Vec::new(), Vec::new());
    let merged = each_poly(node, ctx, merge_poly, &mut details, &mut conditions);
    (!details.is_empty()).then(|| Rewrite {
        node: merged,
        explanation: format!("Combine the logarithms: {}", details.join(", ")),
        rule: RuleId::MergeLogs,
        conditions,
    })
}

/// `k` with `power = base^{k}` for a small `k`, if there is one.
fn exponent_of(power: &Poly, base: &Poly) -> Option<usize> {
    if base.is_constant() && base.constant_term().abs() <= RBig::ONE {
        return None;
    }
    (1..=64).find(|&k| &base.pow(k) == power)
}

/// The smallest `r` that both constants are integer powers of: `8 = 2^{3}`, `4 = 2^{2}`.
fn common_base(a: &Poly, b: &Poly) -> Option<Poly> {
    if !a.is_constant() || !b.is_constant() {
        return None;
    }
    let value = b.constant_term();
    if value <= RBig::ONE || value.denominator() != &dashu::integer::UBig::ONE {
        return None;
    }
    let n = value.numerator().to_f64().value();
    (2..=64).rev().find_map(|j: u32| {
        let r = Poly::from_const(RBig::from(IBig::from(n.powf(1.0 / j as f64).round() as i64)));
        (exponent_of(b, &r).is_some() && exponent_of(a, &r).is_some()).then_some(r)
    })
}

/// `\frac{\ln(u^{k})}{\ln(u^{m})} = \frac{k}{m}`, as `\log_2 8` is read: `\frac{\ln 8}{\ln 2} = 3`.
fn log_quotient(node: &Node, ctx: &mut Context) -> Option<Local> {
    let (numerator, denominator) = split_fraction(node)?;
    let atoms = &*ctx.atoms;
    let log = |node: &Node| -> Option<(&'static str, usize, Poly)> {
        let index = atoms.index_of(node)?;
        let (base, _, argument) = log_atom(atoms, index)?;
        Some((base, index, Poly::from_node(argument).ok()?))
    };
    let (top, top_index, a) = log(numerator)?;
    let (bottom, bottom_index, b) = log(denominator)?;
    if top != bottom {
        return None;
    }
    let base = common_base(&a, &b).unwrap_or_else(|| b.clone());
    let (k, m) = (exponent_of(&a, &base)?, exponent_of(&b, &base)?);
    let value = RBig::from(k) / RBig::from(m);
    let result = Poly::from_const(value).to_node();
    // `\log_2 8` is serialized as typed, a quotient of other logarithms as a fraction.
    let quotient = serializer::expression(&make_fraction(atoms.node(top_index).clone(), atoms.node(bottom_index).clone()));
    let power = |value: &Poly, k: usize| format!("{} = {}^{{{k}}}", serializer::expression(&value.to_node()), serializer::expression(&base.to_node()));
    let detail = match (base.is_constant(), m) {
        (false, _) => format!("{quotient} = {}", serializer::expression(&result)),
        (true, 1) => format!("{}, so {quotient} = {}", power(&a, k), serializer::expression(&result)),
        (true, _) => format!("{}, {}, so {quotient} = {}", power(&a, k), power(&b, m), serializer::expression(&result)),
    };
    Some(Local::new(result, detail).with([Condition::Positive(base.to_node())]))
}

/// `\frac{\ln(a^{2})}{\ln a} = 2` for `a > 0`.
pub fn divide_logs(node: &Node, ctx: &mut Context) -> Option<Rewrite> {
    everywhere(node, ctx, "Divide the logarithms", RuleId::MergeLogs, log_quotient)
}
