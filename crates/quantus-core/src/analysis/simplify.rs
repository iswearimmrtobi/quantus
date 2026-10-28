//! The canonical simplifier of the analysis mode, and the forms results are shown in.
//!
//! [`simplify`] rewrites bottom-up until nothing changes: flat sums and products, folded
//! constants, like terms and like factors collected, the exp/ln and radical rules, exact trig
//! values, and odd/even functions. A part that is a rational function of x goes through the
//! algebra pipeline instead, so polynomials and fractions come out as they do in Algebra mode.
//!
//! No rule here keeps track of the domain: it is worked out on the tree as typed, before anything
//! is simplified. A rule may therefore extend a function beyond its domain (`(\sqrt{x})^2 = x`),
//! but never change a value inside it (`\sqrt{x^2}` stays, it is |x|).

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use dashu::base::{Abs, Signed};
use dashu::integer::IBig;
use dashu::rational::RBig;

use super::exact::{self, TrigValue};
use super::expr::{split_coefficient, summand_rank, Expr, Fn};
use crate::algebra::pipeline::{simplify_node_with, OutputStyle, PipelineConfig};
use crate::latex::{Node, Operator};
use crate::settings::{BinomialForm, DenominatorForm, Settings};

/// Passes over the whole tree before the simplifier gives up on reaching a fixed point.
const MAX_PASSES: usize = 64;

/// Rational parts remembered per thread, since every pass asks again about the same ones.
const CACHE_LIMIT: usize = 4096;

/// How rational parts are written internally: expanded, so like terms meet.
const INTERNAL_STYLE: OutputStyle =
    OutputStyle { binomial: BinomialForm::Expanded, complete_square: false, denominator: DenominatorForm::Expanded };

thread_local! {
    static RATIONAL_CACHE: RefCell<HashMap<(Expr, OutputStyleKey), Expr>> = RefCell::new(HashMap::new());
}

type OutputStyleKey = (BinomialForm, bool, DenominatorForm);

/// The canonical form of `e`.
pub fn simplify(e: &Expr) -> Expr {
    let mut current = e.clone();
    let mut seen = HashSet::new();
    for _ in 0..MAX_PASSES {
        if super::budget::exceeded() {
            break;
        }
        let next = pass(&current);
        if next == current || !seen.insert(next.clone()) {
            return next;
        }
        current = next;
    }
    current
}

/// Whether `e` is exactly zero, as far as the simplifier can tell.
pub fn is_zero(e: &Expr) -> bool {
    simplify(e) == Expr::int(0)
}

/// Whether `e` vanishes at every point of `points` where it is defined (at least three of them),
/// relative to the size of its terms. Only for where the text says "checked numerically".
pub fn is_zero_numeric(e: &Expr, points: &[f64]) -> bool {
    let terms: Vec<&Expr> = match e {
        Expr::Sum(terms) => terms.iter().collect(),
        other => vec![other],
    };
    let mut checked = 0;
    for &x in points {
        let Some(value) = e.eval_f64(x) else { continue };
        let scale: f64 = terms.iter().filter_map(|t| t.eval_f64(x)).map(f64::abs).sum::<f64>().max(1.0);
        if value.abs() > 1e-9 * scale {
            return false;
        }
        checked += 1;
    }
    checked >= 3
}

/// `e` as it is shown: rational parts in the user's denominator style, polynomials expanded
/// unless binomial form is asked for explicitly (a derivative reads `3x^{2} - 3`, not
/// `3(x - 1)(x + 1)`, and never as a completed square), and a factor every summand shares in
/// front of the bracket, `(x + 1)e^{x}`.
pub fn display(e: &Expr, settings: &Settings) -> Expr {
    let simplified = simplify(e);
    let binomial = match settings.binomial_form {
        BinomialForm::Auto => BinomialForm::Expanded,
        other => other,
    };
    let style = OutputStyle { binomial, complete_square: false, denominator: settings.denominator_form };
    if is_rational_candidate(&simplified) {
        if let Some(shown) = rational(&simplified, style) {
            return shown;
        }
    }
    let shown = if needs_atoms(&simplified) { rational_in_atoms(&simplified, style) } else { None };
    let shown = shown.unwrap_or(simplified);
    factor_common(&shown, settings).unwrap_or(shown)
}

fn factor_common(e: &Expr, settings: &Settings) -> Option<Expr> {
    let Expr::Sum(terms) = e else { return None };
    let factor_lists: Vec<Vec<Expr>> = terms.iter().map(|t| factors_of(&split_coefficient(t).1)).collect();
    let mut common: Vec<Expr> = Vec::new();
    for candidate in &factor_lists[0] {
        if candidate.as_rational().is_some() || common.contains(candidate) {
            continue;
        }
        if factor_lists.iter().all(|list| list.contains(candidate)) {
            common.push(candidate.clone());
        }
    }
    if common.is_empty() {
        return None;
    }
    let rest_terms: Vec<Expr> = terms
        .iter()
        .map(|term| {
            let (coefficient, rest) = split_coefficient(term);
            let mut factors = factors_of(&rest);
            for c in &common {
                let i = factors.iter().position(|f| f == c).unwrap();
                factors.remove(i);
            }
            factors.insert(0, Expr::Num(coefficient));
            Expr::product(factors)
        })
        .collect();
    let mut rest = display(&Expr::sum(rest_terms), settings);
    // The rational content of the bracket goes in front: \frac{1}{2}(\sin(x) - \cos(x))e^{x}.
    let mut scale = content(&rest);
    if leading_is_negative(&rest) {
        scale = -scale;
    }
    if scale != RBig::ONE {
        rest = display(&Expr::Product(vec![Expr::Num(RBig::ONE / &scale), rest]), settings);
    }
    let mut factors = vec![Expr::Num(scale), rest];
    factors.extend(common);
    Some(Expr::Product(factors.into_iter().filter(|f| *f != Expr::int(1)).collect()))
}

/// The factors of a product, or the expression itself.
fn factors_of(e: &Expr) -> Vec<Expr> {
    match e {
        Expr::Product(factors) => factors.clone(),
        Expr::Num(r) if *r == RBig::ONE => vec![],
        other => vec![other.clone()],
    }
}

/// Whether the sum would be written with a minus in front: its leading term is negative, or,
/// for a sum with functions (written with a positive term first where possible), all are.
fn leading_is_negative(e: &Expr) -> bool {
    let Expr::Sum(terms) = e else { return split_coefficient(e).0.is_negative() };
    let mixed = terms.iter().any(|t| summand_rank(t).0 == 1);
    if mixed {
        return terms.iter().all(|t| split_coefficient(t).0.is_negative());
    }
    split_coefficient(terms.iter().min_by_key(|t| summand_rank(t)).unwrap()).0.is_negative()
}

/// The positive rational that divides every coefficient of a sum into whole numbers without a
/// common factor: 1/2 for `\frac{1}{2}a - \frac{1}{2}b`, 2 for `2a + 4b`. 1 for anything else.
fn content(e: &Expr) -> RBig {
    let Expr::Sum(terms) = e else { return RBig::ONE };
    // Polynomials keep their coefficients: 3x^{2} - 3 stays as it is.
    if e.as_rational().is_some() {
        return RBig::ONE;
    }
    terms
        .iter()
        .map(|t| split_coefficient(t).0.abs())
        .reduce(|a, b| crate::algebra::poly::rational_gcd(&a, &b))
        .unwrap_or(RBig::ONE)
}

// ---- one pass ----------------------------------------------------------------------------------

fn pass(e: &Expr) -> Expr {
    if is_rational_candidate(e) {
        if let Some(simplified) = rational(e, INTERNAL_STYLE) {
            return simplified;
        }
    }
    let children = map_children(e, pass);
    if needs_atoms(&children) {
        if let Some(simplified) = rational_in_atoms(&children, INTERNAL_STYLE) {
            return simplified;
        }
    }
    local(children)
}

/// The local rules on every node, children first, without the algebra pipeline.
fn canon(e: &Expr) -> Expr {
    local(map_children(e, canon))
}

fn map_children(e: &Expr, f: fn(&Expr) -> Expr) -> Expr {
    match e {
        Expr::Num(_) | Expr::Pi | Expr::E | Expr::Var => e.clone(),
        Expr::Sum(terms) => Expr::Sum(terms.iter().map(f).collect()),
        Expr::Product(factors) => Expr::Product(factors.iter().map(f).collect()),
        Expr::Pow(base, exponent) => Expr::pow(f(base), f(exponent)),
        Expr::Func(function, argument) => Expr::func(*function, f(argument)),
    }
}

fn is_rational_candidate(e: &Expr) -> bool {
    matches!(e, Expr::Sum(_) | Expr::Product(_) | Expr::Pow(..)) && e.as_rational().is_some()
}

/// A rational part through the algebra pipeline, written in `style`.
fn rational(e: &Expr, style: OutputStyle) -> Option<Expr> {
    let key = (e.clone(), (style.binomial, style.complete_square, style.denominator));
    if let Some(hit) = RATIONAL_CACHE.with(|cache| cache.borrow().get(&key).cloned()) {
        return Some(hit);
    }
    let variable = Expr::poly_variable();
    // Cancel the common factor first: the pipeline only finds it when it can factor both sides.
    let reduced = match e.as_rational() {
        Some((n, d)) if !d.is_constant() => {
            let common = n.gcd(&d);
            match (n.div_rem(&common), d.div_rem(&common)) {
                (Ok((n, _)), Ok((d, _))) if !common.is_constant() => {
                    Expr::Product(vec![Expr::from_poly(&n), Expr::recip(Expr::from_poly(&d))])
                }
                _ => e.clone(),
            }
        }
        _ => e.clone(),
    };
    // Verified: without the check the pipeline can return a wrong result for nested fractions
    // such as `\frac{1}{a} \cdot \frac{1}{1 + (\frac{x}{a})^2} \cdot \frac{1}{a} - \frac{1}{1 + x^2}`.
    let config = PipelineConfig { verify_equivalence: true, style, ..PipelineConfig::default() };
    let result = simplify_node_with(&reduced.to_node(variable), config).ok()?;
    let simplified = canon(&Expr::from_node(&result.node, variable).ok()?);
    RATIONAL_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() >= CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(key, simplified.clone());
    });
    Some(simplified)
}

// ---- rational in x and atoms --------------------------------------------------------------------

/// Whether `e` mixes x with functions (the atoms) in a way the algebra pipeline can multiply out
/// or cancel: a sum inside a product or a power, as in `\frac{-x - 2x(1 - \ln x)}{x^{4}}`.
fn needs_atoms(e: &Expr) -> bool {
    fn sum_below(e: &Expr, under_product: bool) -> bool {
        match e {
            Expr::Sum(terms) => under_product || terms.iter().any(|t| sum_below(t, false)),
            Expr::Product(factors) => factors.iter().any(|f| sum_below(f, true)),
            Expr::Pow(base, exponent) if is_integer(exponent) => sum_below(base, true),
            _ => false,
        }
    }
    matches!(e, Expr::Sum(_) | Expr::Product(_)) && e.as_rational().is_none() && sum_below(e, false)
}

fn is_integer(e: &Expr) -> bool {
    e.as_num().is_some_and(|r| r.is_int())
}

/// `e` through the algebra pipeline with every atom (a function, a root, a constant like π) as
/// a variable of its own, `a_1, a_2, …`, put back afterwards. The pipeline treats the atoms as
/// independent, which is always valid, if not always the simplest.
fn rational_in_atoms(e: &Expr, style: OutputStyle) -> Option<Expr> {
    let key = (e.clone(), (style.binomial, style.complete_square, style.denominator));
    if let Some(hit) = RATIONAL_CACHE.with(|cache| cache.borrow().get(&key).cloned()) {
        return Some(hit);
    }
    let mut atoms = Vec::new();
    let node = atom_node(e, &mut atoms)?;
    // Verified: without the check the pipeline can return a wrong result for nested fractions
    // such as `\frac{1}{a} \cdot \frac{1}{1 + (\frac{x}{a})^2} \cdot \frac{1}{a} - \frac{1}{1 + x^2}`.
    let config = PipelineConfig { verify_equivalence: true, style, ..PipelineConfig::default() };
    let result = simplify_node_with(&node, config).ok()?;
    let variable = Expr::poly_variable();
    let restored = result.node.map_children(&mut |child| restore_atoms(child, &atoms, variable));
    let restored = restore_atoms(restored, &atoms, variable);
    let simplified = canon(&Expr::from_node(&restored, variable).ok()?);
    RATIONAL_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() >= CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(key, simplified.clone());
    });
    Some(simplified)
}

/// The rational skeleton of `e` as a node, with each distinct atom as the variable `a_i`.
fn atom_node(e: &Expr, atoms: &mut Vec<Expr>) -> Option<Node> {
    let number = |r: &RBig| Expr::Num(r.clone()).to_node(Expr::poly_variable());
    Some(match e {
        Expr::Num(r) => number(r),
        Expr::Var => Expr::Var.to_node(Expr::poly_variable()),
        Expr::Sum(terms) => {
            terms.iter().map(|t| atom_node(t, atoms)).collect::<Option<Vec<_>>>()?.into_iter().reduce(|a, b| Node::binary(Operator::Plus, a, b, 0))?
        }
        Expr::Product(factors) => factors
            .iter()
            .map(|f| atom_node(f, atoms))
            .collect::<Option<Vec<_>>>()?
            .into_iter()
            .reduce(|a, b| Node::binary(Operator::Times, a, b, 0))?,
        Expr::Pow(base, exponent) if is_integer(exponent) => {
            let n = exponent.as_num().unwrap();
            let base = Node::Group { inner: Box::new(atom_node(base, atoms)?), position: 0 };
            let power = Node::Power { base: Box::new(base), exponent: Box::new(number(&n.clone().abs())), position: 0 };
            if n.is_negative() { Node::fraction(number(&RBig::ONE), power, 0) } else { power }
        }
        atom => {
            let simplified = simplify(atom);
            if let Expr::Num(r) = &simplified {
                return Some(number(r));
            }
            let index = match atoms.iter().position(|a| *a == simplified) {
                Some(i) => i,
                None => {
                    atoms.push(simplified);
                    atoms.len() - 1
                }
            };
            Node::Variable { name: "a".into(), index: (index + 1).to_string(), position: 0 }
        }
    })
}

fn restore_atoms(node: Node, atoms: &[Expr], variable: &str) -> Node {
    // The pipeline writes a variable back under its key, `a_1`.
    let atom = match &node {
        Node::Variable { name, index, .. } => {
            Node::variable_key(name, index).strip_prefix("a_").and_then(|i| i.parse::<usize>().ok())
        }
        _ => None,
    };
    match atom {
        Some(i) => Node::Group { inner: Box::new(atoms[i - 1].to_node(variable)), position: 0 },
        None => node.map_children(&mut |child| restore_atoms(child, atoms, variable)),
    }
}

fn local(e: Expr) -> Expr {
    match e {
        Expr::E => Expr::func(Fn::Exp, Expr::int(1)),
        Expr::Sum(terms) => local_sum(terms),
        Expr::Product(factors) => local_product(factors),
        Expr::Pow(base, exponent) => local_pow(*base, *exponent),
        Expr::Func(function, argument) => local_func(function, *argument),
        other => other,
    }
}

// ---- sums --------------------------------------------------------------------------------------

fn local_sum(terms: Vec<Expr>) -> Expr {
    let mut flat = Vec::new();
    flatten(terms, &mut flat, |e| matches!(e, Expr::Sum(_)));
    // Like terms: the same rest, coefficients added.
    let mut collected: Vec<(Expr, RBig)> = Vec::new();
    for term in flat {
        let (coefficient, rest) = split_coefficient(&term);
        match collected.iter_mut().find(|(r, _)| *r == rest) {
            Some((_, c)) => *c += coefficient,
            None => collected.push((rest, coefficient)),
        }
    }
    let mut out: Vec<Expr> =
        collected.into_iter().filter(|(_, c)| *c != RBig::ZERO).map(|(rest, c)| make_term(c, rest)).collect();
    out.sort();
    Expr::sum(out)
}

/// `c · rest` with the coefficient as the first factor of a flat product.
fn make_term(c: RBig, rest: Expr) -> Expr {
    if rest == Expr::int(1) {
        return Expr::Num(c);
    }
    if c == RBig::ONE {
        return rest;
    }
    let mut factors = vec![Expr::Num(c)];
    match rest {
        Expr::Product(inner) => factors.extend(inner),
        other => factors.push(other),
    }
    Expr::Product(factors)
}

fn flatten(items: Vec<Expr>, out: &mut Vec<Expr>, nested: fn(&Expr) -> bool) {
    for item in items {
        if nested(&item) {
            let (Expr::Sum(inner) | Expr::Product(inner)) = item else { unreachable!() };
            flatten(inner, out, nested);
        } else {
            out.push(item);
        }
    }
}

// ---- products ----------------------------------------------------------------------------------

fn local_product(factors: Vec<Expr>) -> Expr {
    let mut flat = Vec::new();
    flatten(factors, &mut flat, |e| matches!(e, Expr::Product(_)));
    let mut coefficient = RBig::ONE;
    // Like factors: the same base, exponents added where that keeps the value.
    let mut powers: Vec<(Expr, Expr)> = Vec::new();
    let mut exp_arguments: Vec<Expr> = Vec::new();
    for factor in flat {
        match factor {
            Expr::Num(r) => coefficient *= r,
            Expr::Func(Fn::Exp, argument) => exp_arguments.push(*argument),
            Expr::Pow(base, exponent) => add_power(&mut powers, *base, *exponent),
            other => add_power(&mut powers, other, Expr::int(1)),
        }
    }
    if coefficient == RBig::ZERO {
        return Expr::int(0);
    }
    let mut rebuilt: Vec<Expr> = powers.into_iter().map(|(base, exponent)| local_pow(base, exponent)).collect();
    match exp_arguments.len() {
        0 => {}
        1 => rebuilt.push(local_func(Fn::Exp, exp_arguments.pop().unwrap())),
        _ => rebuilt.push(local_func(Fn::Exp, local_sum(exp_arguments))),
    }
    // Rebuilding can give numbers and products again (\sqrt{2}\sqrt{2} = 2).
    let mut out = Vec::new();
    let mut again = Vec::new();
    flatten(rebuilt, &mut again, |e| matches!(e, Expr::Product(_)));
    for factor in again {
        match factor {
            Expr::Num(r) => coefficient *= r,
            other => out.push(other),
        }
    }
    if coefficient == RBig::ZERO {
        return Expr::int(0);
    }
    out.sort();
    if coefficient != RBig::ONE || out.is_empty() {
        out.insert(0, Expr::Num(coefficient));
    }
    Expr::product(out)
}

fn add_power(powers: &mut Vec<(Expr, Expr)>, base: Expr, exponent: Expr) {
    if let Some((_, existing)) = powers.iter_mut().find(|(b, e)| *b == base && mergeable(&base, e, &exponent)) {
        let (Some(a), Some(b)) = (existing.as_num(), exponent.as_num()) else { unreachable!() };
        *existing = Expr::Num(a + b);
        return;
    }
    powers.push((base, exponent));
}

/// Whether `u^a · u^b` can be written `u^{a+b}`: both exponents are numbers. Where an even root
/// makes the left side undefined, the right side merely extends it; the domain is taken before.
fn mergeable(_base: &Expr, a: &Expr, b: &Expr) -> bool {
    a.as_num().is_some() && b.as_num().is_some()
}

fn is_positive_constant(e: &Expr) -> bool {
    e.is_constant() && e.eval_f64(0.0).is_some_and(|v| v > 0.0)
}

// ---- powers ------------------------------------------------------------------------------------

fn local_pow(base: Expr, exponent: Expr) -> Expr {
    if let Some(e) = exponent.as_num() {
        if *e == RBig::ZERO {
            return Expr::int(1);
        }
        if *e == RBig::ONE {
            return base;
        }
    }
    if base == Expr::int(1) {
        return Expr::int(1);
    }
    match (&base, exponent.as_num()) {
        // |u|^{2k} = u^{2k}
        (Expr::Func(Fn::Abs, u), Some(n)) if n.is_int() && n.numerator() % IBig::from(2) == IBig::ZERO => {
            return local_pow((**u).clone(), exponent);
        }
        (Expr::Num(b), Some(e)) => {
            if let Some(value) = exact::radical(b, e) {
                return value;
            }
        }
        // (u^a)^n = u^{an} for whole n. Where an even root makes the left side undefined (u < 0
        // in (\sqrt{u})^2) the right side merely extends it; the domain is taken before this.
        (Expr::Pow(u, a), Some(n)) if n.is_int() => {
            if let Some(a) = a.as_num() {
                return local_pow((**u).clone(), Expr::Num(a * n));
            }
        }
        // (ab)^n = a^n b^n for whole n.
        (Expr::Product(factors), Some(n)) if n.is_int() => {
            let powered = factors.iter().map(|f| local_pow(f.clone(), Expr::Num(n.clone()))).collect();
            return local_product(powered);
        }
        _ => {}
    }
    // (e^u)^c = e^{cu} for a constant c.
    if let Expr::Func(Fn::Exp, u) = &base {
        if exponent.is_constant() {
            return local_func(Fn::Exp, local_product(vec![exponent, (**u).clone()]));
        }
    }
    Expr::pow(base, exponent)
}

// ---- functions ---------------------------------------------------------------------------------

fn local_func(function: Fn, argument: Expr) -> Expr {
    let keep = |argument: Expr| Expr::func(function, argument);
    match function {
        Fn::Exp => match &argument {
            Expr::Num(r) if *r == RBig::ZERO => Expr::int(1),
            Expr::Func(Fn::Ln, u) => (**u).clone(),
            // e^{c \ln u} = u^c
            Expr::Product(factors) => match factors.as_slice() {
                [Expr::Num(c), Expr::Func(Fn::Ln, u)] => local_pow((**u).clone(), Expr::Num(c.clone())),
                _ => keep(argument),
            },
            _ => keep(argument),
        },
        Fn::Ln => match &argument {
            Expr::Num(r) if *r == RBig::ONE => Expr::int(0),
            // \ln\frac{1}{q} = -\ln q
            Expr::Num(r) if r.is_positive() && *r.numerator() == IBig::ONE => {
                local_product(vec![Expr::int(-1), keep(Expr::Num(RBig::from(IBig::from(r.denominator().clone()))))])
            }
            Expr::Func(Fn::Exp, u) => (**u).clone(),
            // \ln(u^{p/q}) = \frac{p}{q}\ln u, with |u| when p is even (u may be negative there).
            Expr::Pow(u, r) => match r.as_num() {
                Some(r) => {
                    let inner = if r.numerator() % IBig::from(2) == IBig::ZERO {
                        local_func(Fn::Abs, (**u).clone())
                    } else {
                        (**u).clone()
                    };
                    local_product(vec![Expr::Num(r.clone()), local_func(Fn::Ln, inner)])
                }
                None => keep(argument),
            },
            _ => keep(argument),
        },
        Fn::Sin | Fn::Cos | Fn::Tan => {
            if let Some(r) = exact::pi_multiple(&argument) {
                if let Some(TrigValue::Value(value)) = exact::trig_at(function, &r) {
                    return value;
                }
            }
            match negated(&argument) {
                Some(positive) if function == Fn::Cos => keep(positive),
                Some(positive) => local_product(vec![Expr::int(-1), keep(positive)]),
                None => keep(argument),
            }
        }
        Fn::Arcsin | Fn::Arccos | Fn::Arctan => {
            if let Some(angle) = exact::inverse_trig_at(function, &argument) {
                return angle;
            }
            match negated(&argument) {
                Some(positive) if function != Fn::Arccos => local_product(vec![Expr::int(-1), keep(positive)]),
                _ => keep(argument),
            }
        }
        Fn::Sinh => match negated(&argument) {
            _ if argument == Expr::int(0) => Expr::int(0),
            Some(positive) => local_product(vec![Expr::int(-1), keep(positive)]),
            None => keep(argument),
        },
        Fn::Cosh => match negated(&argument) {
            _ if argument == Expr::int(0) => Expr::int(1),
            Some(positive) => keep(positive),
            None => keep(argument),
        },
        Fn::Abs => match &argument {
            Expr::Num(r) => Expr::Num(r.clone().abs()),
            Expr::Func(Fn::Abs | Fn::Exp, _) => argument,
            Expr::Pow(_, n) if n.as_num().is_some_and(|n| n.numerator() % IBig::from(2) == IBig::ZERO) => argument,
            _ if is_positive_constant(&argument) => argument,
            // |c u| = |c| |u|
            Expr::Product(factors) => match factors.split_first() {
                Some((Expr::Num(c), rest)) => {
                    local_product(vec![Expr::Num(c.clone().abs()), keep(local_product(rest.to_vec()))])
                }
                _ => keep(argument),
            },
            _ => keep(argument),
        },
    }
}

/// `-u` when `u` has a negative coefficient.
fn negated(u: &Expr) -> Option<Expr> {
    match u {
        Expr::Num(r) if r.is_negative() => Some(Expr::Num(-r.clone())),
        Expr::Product(factors) => match factors.first() {
            Some(Expr::Num(c)) if c.is_negative() => Some(local_product(
                std::iter::once(Expr::Num(-c.clone())).chain(factors[1..].iter().cloned()).collect(),
            )),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::latex::{parser, serializer};
    use crate::settings::Mode;

    fn expr(latex: &str) -> Expr {
        let options = Settings { mode: Mode::Algebra, ..Settings::default() }.parse_options();
        Expr::from_node(&parser::parse_with(latex, &options).unwrap().expression, "x").unwrap()
    }

    fn simplified(latex: &str) -> String {
        serializer::algebraic_expression(&simplify(&expr(latex)).to_node("x"))
    }

    fn shown(latex: &str) -> String {
        serializer::algebraic_expression(&display(&expr(latex), &Settings::default()).to_node("x"))
    }

    #[test]
    fn like_terms_and_factors_are_collected() {
        assert_eq!(simplified("x + x"), "2x");
        assert_eq!(simplified("x \\cdot x^2"), "x^{3}");
        assert_eq!(simplified("2\\pi + \\pi"), "3\\pi");
        assert_eq!(simplified("e \\cdot e^{x}"), "e^{x + 1}");
        assert_eq!(simplified("e^{x} \\cdot e^{-x}"), "1");
        assert_eq!(simplified("\\sin x + 2\\sin x - 3 \\sin x"), "0");
        // The domain (x ≥ 0) is taken before simplifying, so the root may go.
        assert_eq!(simplified("\\sqrt{x} \\cdot \\sqrt{x}"), "x");
        assert_eq!(simplified("(\\sqrt{x})^2"), "x");
        assert_eq!(simplified("\\sqrt{2} \\cdot \\sqrt{2}"), "2");
        assert_eq!(simplified("(\\sqrt[3]{x})^3"), "x");
    }

    #[test]
    fn rational_parts_go_through_the_algebra_pipeline() {
        assert_eq!(simplified("\\frac{x^2-1}{x-1}"), "x + 1");
        assert_eq!(simplified("(2x)^2 + (x+1)^2"), "5x^{2} + 2x + 1");
        assert_eq!(simplified("\\frac{1}{x} + \\frac{1}{x^2}"), "\\frac{x + 1}{x^{2}}");
        assert!(is_zero(&expr("(x+1)^2 - x^2 - 2x - 1")));
    }

    #[test]
    fn radicals_are_in_simplest_form() {
        assert_eq!(simplified("\\sqrt{8}"), "2\\sqrt{2}");
        assert_eq!(simplified("\\frac{1}{\\sqrt{2}}"), "\\frac{\\sqrt{2}}{2}");
        assert_eq!(simplified("\\sqrt[3]{-8}"), "-2");
        assert_eq!(simplified("\\sqrt[3]{16}"), "2\\sqrt[3]{2}");
    }

    #[test]
    fn exp_and_ln_undo_each_other() {
        assert_eq!(simplified("e^{\\ln x}"), "x");
        assert_eq!(simplified("\\ln(e^{x})"), "x");
        assert_eq!(simplified("\\ln 1"), "0");
        assert_eq!(simplified("\\ln e"), "1");
        assert_eq!(simplified("\\ln\\frac{1}{2}"), "-\\ln(2)");
        assert_eq!(simplified("\\ln(x^2)"), "2\\ln\\left|x\\right|");
        assert_eq!(simplified("\\ln(x^3)"), "3\\ln(x)");
        assert_eq!(simplified("e^{2\\ln x}"), "x^{2}");
        assert_eq!(simplified("(e^{x})^2"), "e^{2x}");
    }

    #[test]
    fn trig_functions_know_their_exact_values_and_symmetry() {
        assert_eq!(simplified("\\sin\\frac{\\pi}{6}"), "\\frac{1}{2}");
        assert_eq!(simplified("\\cos\\frac{3\\pi}{4}"), "-\\frac{\\sqrt{2}}{2}");
        assert_eq!(simplified("\\sin(\\pi)"), "0");
        assert_eq!(simplified("\\tan\\frac{\\pi}{2}"), "\\tan\\left(\\frac{\\pi}{2}\\right)");
        assert_eq!(simplified("\\sin(-x)"), "-\\sin(x)");
        assert_eq!(simplified("\\cos(-2x)"), "\\cos(2x)");
        assert_eq!(simplified("\\arcsin\\frac{1}{2}"), "\\frac{\\pi}{6}");
        assert_eq!(simplified("\\arccos(-\\frac{1}{2})"), "\\frac{2\\pi}{3}");
        assert_eq!(simplified("\\arctan(-x)"), "-\\arctan(x)");
    }

    #[test]
    fn absolute_values_drop_what_is_positive() {
        assert_eq!(simplified("|-x|"), "\\left|x\\right|");
        assert_eq!(simplified("|-3x|"), "3\\left|x\\right|");
        assert_eq!(simplified("|e^{x}|"), "e^{x}");
        assert_eq!(simplified("|x^2|"), "x^{2}");
        assert_eq!(simplified("|-\\pi|"), "\\pi");
    }

    #[test]
    fn a_shared_factor_is_written_in_front_of_the_bracket() {
        assert_eq!(shown("x e^x + e^x"), "(x + 1)e^{x}");
        assert_eq!(shown("2x e^{-x} - x^2 e^{-x}"), "-(x^{2} - 2x)e^{-x}");
        assert_eq!(shown("3x^2 - 3"), "3x^{2} - 3");
        assert_eq!(shown("x + \\sin x"), "x + \\sin(x)");
    }

    #[test]
    fn numeric_zero_needs_agreement_at_the_points() {
        let identity = expr("\\sin(x)^2 + \\cos(x)^2 - 1");
        assert!(!is_zero(&identity));
        assert!(is_zero_numeric(&identity, &[0.5, 1.5, 2.5, 3.5]));
        assert!(!is_zero_numeric(&expr("x - 1"), &[0.5, 1.5, 2.5]));
        assert!(!is_zero_numeric(&expr("\\ln x"), &[-1.0, -2.0]));
    }
}
