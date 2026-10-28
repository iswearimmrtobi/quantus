//! Identities of roots, as rules on the atomized tree (see `atoms` and `identities`).
//!
//! - Nested roots are one root: `\sqrt[m]{\sqrt[n]{u}} = \sqrt[mn]{u}`.
//! - Powers come out of the radicand: `\sqrt{8} = 2\sqrt{2}`, `\sqrt{a^{3}} = a\sqrt{a}`,
//!   `\sqrt{a^{2}} = |a|`.
//! - A power of a root is written through its radicand: `\sqrt{u} \cdot \sqrt{u} = u`.
//! - Roots of the same degree are multiplied into one: `\sqrt{a} \cdot \sqrt{b} = \sqrt{ab}`.
//! - Denominators are rationalized: `\frac{p}{q \sqrt{u}} = \frac{p \sqrt{u}}{q u}`, and with the
//!   conjugate, `\frac{1}{\sqrt{a} + 1} = \frac{\sqrt{a} - 1}{a - 1}`.
//!
//! The quotient `\frac{\sqrt{a}}{\sqrt{b}}` is not merged into `\sqrt{\frac{a}{b}}`, and a factor is
//! not moved under the root (`2\sqrt{a}` stays): both would undo the rationalized normal form.

use std::collections::BTreeMap;

use dashu::base::{Abs, Signed};
use dashu::integer::{IBig, UBig};
use dashu::rational::RBig;

use crate::algebra::atoms::Atoms;
use crate::algebra::binomial::exact_ubig_sqrt;
use crate::algebra::domain::unwrap_group;
use crate::algebra::identities::{each_atom, each_poly, everywhere, note, number, power, Condition, Context, Local, Rewrite};
use crate::algebra::nested::{make_fraction, make_product, split_fraction};
use crate::algebra::pipeline::integer_exponent;
use crate::algebra::poly::{rational_to_node, Monomial, Poly};
use crate::algebra::steps::RuleId;
use crate::latex::ast::{Node, Operator};
use crate::latex::serializer;
use crate::settings::StepDetail;

/// The degree and radicand of the atom with `index`, if it is a root of a polynomial.
fn root(atoms: &Atoms, index: usize) -> Option<(usize, &Node)> {
    let Node::Root { radicand, degree, .. } = atoms.node(index) else { return None };
    let n = match degree {
        None => 2,
        Some(degree) => usize::try_from(integer_exponent(degree)?).ok().filter(|&n| n >= 2)?,
    };
    Poly::from_node(radicand).ok()?;
    Some((n, radicand))
}

fn root_of(atoms: &Atoms, node: &Node) -> Option<(usize, usize, Poly)> {
    let index = atoms.index_of(node)?;
    let (n, radicand) = root(atoms, index)?;
    Some((index, n, Poly::from_node(radicand).ok()?))
}

pub(crate) fn make_root(n: usize, radicand: Node) -> Node {
    Node::Root { radicand: Box::new(radicand), degree: (n != 2).then(|| Box::new(number(n))), position: 0 }
}

// ---- nested roots ----------------------------------------------------------------------------

fn nest_root(index: usize, ctx: &mut Context) -> Option<Local> {
    let (n, radicand) = root(ctx.atoms, index)?;
    let (inner, m, _) = root_of(ctx.atoms, radicand)?;
    let (_, inner_radicand) = root(ctx.atoms, inner)?;
    let radicand = inner_radicand.clone();
    let merged = ctx.atoms.intern(make_root(n * m, radicand.clone()));
    let detail = format!("{} = {}", serializer::expression(&ctx.atoms.var(index)), serializer::expression(&merged));
    // \sqrt{\sqrt{x}} = (x^{1/2})^{1/2} = x^{1/4}.
    let reciprocal = |k: usize| make_fraction(number(1), number(k));
    let raised = |base: Node, k: usize| Node::Power { base: Box::new(Node::Group { inner: Box::new(base), position: 0 }), exponent: Box::new(reciprocal(k)), position: 0 };
    let powers = raised(raised(radicand.clone(), m), n);
    let product = raised(radicand, n * m);
    let working = vec![
        (powers, "Write the roots as powers".to_string(), StepDetail::Detailed),
        (product, "Multiply the exponents".to_string(), StepDetail::Detailed),
    ];
    Some(Local::new(merged, detail).with_working(working, "Write the power as a root"))
}

/// `\sqrt[m]{\sqrt[n]{u}} = \sqrt[mn]{u}`.
pub fn nest_roots(node: &Node, ctx: &mut Context) -> Option<Rewrite> {
    each_atom(node, ctx, "Write the nested roots as one root", RuleId::NestRoots, nest_root)
}

// ---- powers out of the radicand ---------------------------------------------------------------

/// `m` as `out^n \cdot in` with `out` as large as small prime factors allow.
fn split_power(m: &UBig, n: usize) -> (UBig, UBig) {
    let mut rest = m.clone();
    let (mut out, mut inside) = (UBig::ONE, UBig::ONE);
    let mut d = 2u32;
    while d <= 100_000 && UBig::from(d) * UBig::from(d) <= rest {
        let divisor = UBig::from(d);
        let mut count = 0;
        while (&rest % &divisor).is_zero() {
            rest /= &divisor;
            count += 1;
        }
        out *= divisor.pow(count / n);
        inside *= divisor.pow(count % n);
        d += 1;
    }
    // What is left has no factor below the bound; for a square root it may still be a square.
    match (n, exact_ubig_sqrt(&rest)) {
        (2, Some(root)) if rest > UBig::ONE => out *= root,
        _ => inside *= rest,
    }
    (out, inside)
}

fn extract_root(index: usize, ctx: &mut Context) -> Option<Local> {
    let (n, radicand) = root(ctx.atoms, index)?;
    let poly = Poly::from_node(radicand).ok()?;
    if poly.is_zero() {
        return None;
    }
    let factored = poly.factor();
    let mut changed = false;

    // The constant: \sqrt[n]{\frac{p}{q}} = \frac{1}{q} \sqrt[n]{p q^{n - 1}}, and a minus comes out
    // of an odd root.
    let coeff = factored.coeff.clone();
    let (mut outside, inside);
    // The whole number split into the part that comes out and the part that stays: 12 is 2^2 \cdot 3.
    let mut split = None;
    if coeff.is_negative() && n % 2 == 0 {
        outside = RBig::ONE;
        inside = coeff;
    } else {
        outside = RBig::ONE;
        if coeff.is_negative() {
            outside = -RBig::ONE;
            changed = true;
        }
        let q = coeff.denominator().clone();
        let m = UBig::try_from(coeff.numerator().abs()).ok()? * q.pow(n - 1);
        let (out, rest) = split_power(&m, n);
        changed |= out != UBig::ONE || q != UBig::ONE;
        if q == UBig::ONE {
            split = Some((out.clone(), rest.clone()));
        }
        outside = outside * RBig::from_parts(IBig::from(out), q);
        inside = RBig::from(rest);
    }

    let mut factors_out = Vec::new();
    let mut conditions = Vec::new();
    let mut inside_factors = Vec::new();
    for factor in &factored.factors {
        let (k, r) = (factor.exp / n, factor.exp % n);
        if r > 0 {
            inside_factors.push((factor.base.clone(), r));
        }
        if k == 0 {
            continue;
        }
        changed = true;
        let base = factor.base.to_node();
        // \sqrt[n]{f^{nk}} is |f|^k for an even root; f^k when that cannot be negative.
        let condition = Condition::NonNegative(base.clone());
        let holds = ctx.holds(&condition);
        let plain = n % 2 == 1 || k % 2 == 0 || holds || (r % 2 == 1 && ctx.allows(&[condition.clone()]));
        if plain {
            if n % 2 == 0 && k % 2 == 1 && !holds {
                conditions.push(condition);
            }
            factors_out.push(power(base, k));
        } else {
            let abs = ctx.atoms.intern(Node::Abs { operand: Box::new(base), position: 0 });
            factors_out.push(power(abs, k));
        }
    }

    // A degree that shares a factor g with every exponent left inside: \sqrt[6]{a^{2}} = \sqrt[3]{a}.
    // For an even root this needs the bases not to be negative, unless what stays an even power.
    let mut degree = n;
    let mut inside = inside;
    if let Some(g) = common_degree(n, &inside, &inside_factors) {
        let needs: Vec<Condition> = inside_factors
            .iter()
            .filter(|(_, r)| n % 2 == 0 && (r / g) % 2 == 1)
            .map(|(base, _)| Condition::NonNegative(base.to_node()))
            .collect();
        if ctx.allows(&needs) {
            conditions.extend(needs.into_iter().filter(|c| !ctx.holds(c)));
            degree = n / g;
            inside = exact_root(&inside, g)?;
            for (_, r) in &mut inside_factors {
                *r /= g;
            }
            changed = true;
        }
    }
    if !changed {
        return None;
    }
    let inside_poly = inside_factors.iter().fold(Poly::from_const(inside), |product, (base, r)| &product * &base.pow(*r));

    let mut result = rational_to_node(&outside);
    for factor in factors_out {
        result = make_product(result, factor);
    }
    if !(inside_poly.is_constant() && inside_poly.constant_term() == RBig::ONE) {
        let rest = if degree == 1 { inside_poly.to_node() } else { ctx.atoms.intern(make_root(degree, inside_poly.to_node())) };
        result = make_product(result, rest);
    }
    let detail = format!("{} = {}", serializer::expression(&ctx.atoms.var(index)), serializer::expression(&result));
    let local = Local::new(result, detail).with(conditions);
    // A number under the root: \sqrt{12} = \sqrt{4 \cdot 3} = \sqrt{4} \cdot \sqrt{3} = 2\sqrt{3}.
    match split {
        Some((out, rest)) if factored.factors.is_empty() && degree == n && out > UBig::ONE && rest > UBig::ONE => {
            let power = out.pow(n);
            let number = |u: &UBig| rational_to_node(&RBig::from(u.clone()));
            let split = make_root(n, Node::binary(Operator::Times, number(&power), number(&rest), 0));
            let apart = Node::binary(Operator::Times, make_root(n, number(&power)), make_root(n, number(&rest)), 0);
            let kind = match n {
                2 => "square".to_string(),
                3 => "cube".to_string(),
                n => format!("{n}th power"),
            };
            let rule = if n == 2 { "\\sqrt{ab} = \\sqrt{a} \\cdot \\sqrt{b}".to_string() } else { format!("\\sqrt[{n}]{{ab}} = \\sqrt[{n}]{{a}} \\cdot \\sqrt[{n}]{{b}}") };
            let last = format!("{} = {}", serializer::expression(&make_root(n, number(&power))), out);
            Some(local.with_working(
                vec![
                    (split, format!("Split off the largest {kind} factor"), StepDetail::Standard),
                    (apart, rule, StepDetail::Detailed),
                ],
                last,
            ))
        }
        _ => Some(local),
    }
}

fn gcd(a: usize, b: usize) -> usize {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

/// The exact `g`-th root of a non-negative rational, if there is one.
fn exact_root(value: &RBig, g: usize) -> Option<RBig> {
    let root_of = |m: UBig| -> Option<UBig> {
        let guess = (m.to_f64().value().powf(1.0 / g as f64)).round();
        let guess = UBig::from(guess as u64);
        [guess.clone(), guess.clone() + UBig::ONE, if guess > UBig::ZERO { guess - UBig::ONE } else { UBig::ZERO }]
            .into_iter()
            .find(|candidate| candidate.pow(g) == m)
    };
    if value.is_negative() {
        return None;
    }
    let numerator = root_of(UBig::try_from(value.numerator().clone()).ok()?)?;
    Some(RBig::from_parts(IBig::from(numerator), root_of(value.denominator().clone())?))
}

/// The largest `g > 1` dividing the degree `n` and every exponent inside, with a constant that
/// is a `g`-th power.
fn common_degree(n: usize, constant: &RBig, factors: &[(Poly, usize)]) -> Option<usize> {
    let g = factors.iter().fold(n, |g, (_, r)| gcd(g, *r));
    if factors.is_empty() || g < 2 {
        return None;
    }
    (2..=g).rev().find(|d| g % d == 0 && exact_root(constant, *d).is_some())
}

/// `\sqrt{8} = 2\sqrt{2}`, `\sqrt{4a} = 2\sqrt{a}`, `\sqrt{a^{3}} = a\sqrt{a}`, `\sqrt{a^{2}} = |a|`.
pub fn extract_roots(node: &Node, ctx: &mut Context) -> Option<Rewrite> {
    each_atom(node, ctx, "Take powers out of the root", RuleId::ExtractRoots, extract_root)
}

// ---- powers and products of roots --------------------------------------------------------------

/// `\sqrt[n]{u}^{nk + r} = u^k \sqrt[n]{u}^r` in every term of `poly`.
fn reduce_poly(poly: &Poly, ctx: &mut Context, details: &mut Vec<String>, _: &mut Vec<Condition>) -> Option<Poly> {
    let atoms = &*ctx.atoms;
    let reducible = |var: &str, exp: usize| -> Option<(usize, usize, Poly)> {
        let index = atoms.index_of_key(var)?;
        let (n, radicand) = root(atoms, index)?;
        if exp < n {
            return None;
        }
        Some((index, n, Poly::from_node(radicand).ok()?))
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
                Some((index, n, radicand)) => {
                    let root = serializer::expression(&atoms.var(index));
                    let radicand_latex = serializer::expression(&radicand.to_node());
                    let detail = if n == 2 {
                        format!("{root} \\cdot {root} = {radicand_latex}")
                    } else {
                        format!("({root})^{{{n}}} = {radicand_latex}")
                    };
                    note(details, detail);
                    product = &product * &radicand.pow(exp / n);
                    if exp % n > 0 {
                        vars.insert(var.clone(), exp % n);
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

/// `\sqrt{u} \cdot \sqrt{u} = u`, and `(\sqrt[n]{u})^{n} = u` for any degree.
pub fn multiply_roots(node: &Node, ctx: &mut Context) -> Option<Rewrite> {
    let (mut details, mut conditions) = (Vec::new(), Vec::new());
    let reduced = each_poly(node, ctx, reduce_poly, &mut details, &mut conditions);
    if details.is_empty() {
        return None;
    }
    // One term with a number in front: the roots first, then the numbers, 2 \cdot 2 before 4.
    let mut working = Vec::new();
    if let Ok(poly) = Poly::from_node(node) {
        if let [term] = poly.terms.as_slice() {
            let unit = Poly::from_term(RBig::ONE, term.monomial.clone());
            if term.coeff != RBig::ONE {
                if let Some(roots) = reduce_poly(&unit, ctx, &mut Vec::new(), &mut Vec::new()) {
                    let line = Node::binary(Operator::Times, rational_to_node(&term.coeff), roots.to_node(), 0);
                    working.push(crate::algebra::steps::AlgebraStep::line(node, line, details.join(", "), StepDetail::Standard));
                }
            }
        }
    }
    let last = (!working.is_empty()).then(|| "Multiply the numbers".to_string());
    Some(Rewrite {
        node: reduced,
        explanation: format!("Multiply the roots: {}", details.join(", ")),
        rule: RuleId::MultiplyRoots,
        conditions,
        working,
        last,
    })
}

/// `\sqrt[n]{a}^{i} \sqrt[m]{b}^{j} = \sqrt[L]{a^{iL/n} b^{jL/m}}` with `L` the lcm of the degrees, in
/// every term of `poly`. Under an even root every radicand must not be negative (`a, b \geq 0`), so
/// only the roots whose condition may be used take part.
fn merge_poly(poly: &Poly, ctx: &mut Context, details: &mut Vec<String>, conditions: &mut Vec<Condition>) -> Option<Poly> {
    let mut changed = false;
    let mut sum = Poly::zero();
    for term in &poly.terms {
        // The roots of this term: (variable, degree, exponent, radicand).
        let mut roots = Vec::new();
        for (var, &exp) in &term.monomial.vars {
            let Some(index) = ctx.atoms.index_of_key(var) else { continue };
            let Some((n, radicand)) = root(ctx.atoms, index) else { continue };
            let Ok(radicand) = Poly::from_node(radicand) else { continue };
            if exp < n {
                roots.push((var.clone(), n, exp, radicand));
            }
        }
        if !roots.iter().all(|(_, n, _, _)| n % 2 == 1) {
            roots.retain(|(_, _, _, radicand)| ctx.allows(&[Condition::NonNegative(radicand.to_node())]));
        }
        if roots.len() < 2 && roots.iter().all(|(_, _, exp, _)| *exp < 2) {
            sum = &sum + &Poly::from_term(term.coeff.clone(), term.monomial.clone());
            continue;
        }
        let degree = roots.iter().fold(1, |l, (_, n, _, _)| l / gcd(l, *n) * n);
        let mut vars = term.monomial.vars.clone();
        let mut radicand = Poly::from_const(RBig::ONE);
        let mut parts = Vec::new();
        for (var, n, exp, factor) in &roots {
            radicand = &radicand * &factor.pow(exp * degree / n);
            vars.remove(var);
            let index = ctx.atoms.index_of_key(var)?;
            parts.push(serializer::expression(&power(ctx.atoms.var(index), *exp)));
            if degree % 2 == 0 {
                conditions.push(Condition::NonNegative(factor.to_node()));
            }
        }
        let merged = ctx.atoms.intern(make_root(degree, radicand.to_node()));
        note(details, format!("{} = {}", parts.join(" \\cdot "), serializer::expression(&merged)));
        let Node::Variable { name, index, .. } = &merged else { return None };
        *vars.entry(Node::variable_key(name, index)).or_insert(0) += 1;
        changed = true;
        sum = &sum + &Poly::from_term(term.coeff.clone(), Monomial::from_map(vars));
    }
    changed.then_some(sum)
}

/// `\sqrt{a} \cdot \sqrt{b} = \sqrt{ab}` for `a, b \geq 0`, and the same for roots of any degree.
pub fn merge_roots(node: &Node, ctx: &mut Context) -> Option<Rewrite> {
    let (mut details, mut conditions) = (Vec::new(), Vec::new());
    let merged = each_poly(node, ctx, merge_poly, &mut details, &mut conditions);
    (!details.is_empty()).then(|| Rewrite {
        node: merged,
        explanation: format!("Multiply under one root: {}", details.join(", ")),
        rule: RuleId::MergeRoots,
        conditions,
        working: Vec::new(),
        last: None,
    })
}

// ---- rationalizing ----------------------------------------------------------------------------

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

/// A root factor `\sqrt[n]{u}^{e}` of a denominator: its atom, `n`, `e` and `u`.
fn root_factor(factor: &Node, atoms: &Atoms) -> Option<(usize, usize, usize, Node)> {
    let (base, exp) = match factor {
        Node::Power { base, exponent, .. } => (&**base, usize::try_from(integer_exponent(exponent)?).ok()?),
        other => (other, 1),
    };
    let (index, n, radicand) = root_of(atoms, base)?;
    (exp < n).then(|| (index, n, exp, radicand.to_node()))
}

/// `\frac{p}{q \sqrt[n]{u}^{e}}` as `\frac{p \sqrt[n]{u}^{n - e}}{q u}`, for the first root factor.
fn rationalize_factor(node: &Node, atoms: &Atoms) -> Option<(Node, String)> {
    let (numerator, denominator) = split_fraction(node)?;
    let denominator = factors(denominator);
    let (position, (index, n, exp, radicand)) =
        denominator.iter().enumerate().find_map(|(position, factor)| Some((position, root_factor(factor, atoms)?)))?;
    let rest = denominator
        .iter()
        .enumerate()
        .filter(|(other, _)| *other != position)
        .fold(radicand, |product, (_, factor)| make_product((*factor).clone(), product));
    let by = power(atoms.var(index), n - exp);
    let detail = format!("multiply by {}", serializer::expression(&by));
    Some((make_fraction(make_product(numerator.clone(), by), rest), detail))
}

/// `\frac{1}{\sqrt{2}} \cdot \frac{\sqrt{2}}{\sqrt{2}}`: the fraction extended by `by`.
fn extended(fraction: &Node, by: &Node) -> Node {
    Node::binary(Operator::Times, fraction.clone(), make_fraction(by.clone(), by.clone()), 0)
}

/// `\frac{z}{p + q \sqrt{u}}` as `\frac{z (p - q \sqrt{u})}{p^{2} - q^{2} u}`, for a denominator that
/// is linear in a square root. Needs `p - q \sqrt{u} \neq 0`.
fn rationalize_conjugate(node: &Node, ctx: &Context) -> Option<Local> {
    let atoms = &*ctx.atoms;
    let (numerator, denominator) = split_fraction(node)?;
    let den = Poly::from_node(denominator).ok()?;
    for var in den.variables() {
        let Some(index) = atoms.index_of_key(&var) else { continue };
        let Some((2, radicand)) = root(atoms, index) else { continue };
        if den.degree_of(&var) != 1 {
            continue;
        }
        let Ok(radicand) = Poly::from_node(radicand) else { continue };
        let (mut p, mut q) = (Poly::zero(), Poly::zero());
        for term in &den.terms {
            let mut vars = term.monomial.vars.clone();
            if vars.remove(&var).is_some() {
                q = &q + &Poly::from_term(term.coeff.clone(), Monomial::from_map(vars));
            } else {
                p = &p + &Poly::from_term(term.coeff.clone(), term.monomial.clone());
            }
        }
        if p.is_zero() || q.is_zero() {
            continue;
        }
        let mut conjugate = &p - &(&q * &Poly::from_var(&var));
        let mut rationalized = &(&p * &p) - &(&(&q * &q) * &radicand);
        // `\frac{1}{\sqrt{3} - 1}` is extended by `\sqrt{3} + 1`, not by `-\sqrt{3} - 1`: the sign
        // that keeps the new denominator positive.
        if rationalized.leading_coeff().is_negative() {
            conjugate = -&conjugate;
            rationalized = -&rationalized;
        }
        // The new denominator brings p^{2} - q^{2} u \neq 0 into the domain, which is the condition.
        if !ctx.allows(&[Condition::NonZero(conjugate.to_node())]) {
            continue;
        }
        let detail = format!("multiply by the conjugate {}", serializer::expression(&conjugate.to_node()));
        let result = make_fraction(make_product(numerator.clone(), conjugate.to_node()), rationalized.to_node());
        let extend = (extended(node, &conjugate.to_node()), format!("Extend by the conjugate {}", serializer::expression(&conjugate.to_node())), StepDetail::Standard);
        let mut working = vec![extend];
        // With numbers for p and q, the denominator is the third binomial formula: (\sqrt{3})^2 - 1^2.
        if p.is_constant() && q.is_constant() {
            let square = |n: Node| Node::Power { base: Box::new(Node::Group { inner: Box::new(n), position: 0 }), exponent: Box::new(number(2)), position: 0 };
            let size = |c: &Poly| Poly::from_const(c.constant_term().abs());
            let root_part = square((&size(&q) * &Poly::from_var(&var)).to_node());
            let number_part = square(size(&p).to_node());
            let formula = if (&(&q * &q) * &radicand) - (&p * &p) == rationalized {
                Node::binary(Operator::Minus, root_part, number_part, 0)
            } else {
                Node::binary(Operator::Minus, number_part, root_part, 0)
            };
            let line = make_fraction(make_product(numerator.clone(), conjugate.to_node()), formula);
            working.push((line, "Third binomial formula in the denominator".to_string(), StepDetail::Detailed));
        }
        return Some(Local::new(result, detail).with_working(working, "Work out the denominator"));
    }
    None
}

fn rationalize_at(node: &Node, ctx: &mut Context) -> Option<Local> {
    // Each root factor that is taken out leaves one fewer, so this ends.
    let mut current = node.clone();
    let mut details = Vec::new();
    let mut by = Vec::new();
    while let Some((next, detail)) = rationalize_factor(&current, ctx.atoms) {
        by.push(root_factor_by(&current, ctx.atoms));
        current = next;
        details.push(detail);
    }
    if details.is_empty() {
        return rationalize_conjugate(node, ctx);
    }
    let local = Local::new(current, details.join(", "));
    // One root in the denominator: \frac{1}{\sqrt{2}} \cdot \frac{\sqrt{2}}{\sqrt{2}} first.
    match by.as_slice() {
        [Some(by)] => {
            let text = format!("Extend by {}", serializer::expression(by));
            Some(local.with_working(vec![(extended(node, by), text, StepDetail::Standard)], "Multiply the fractions"))
        }
        _ => Some(local),
    }
}

/// What [`rationalize_factor`] multiplies `node` by.
fn root_factor_by(node: &Node, atoms: &Atoms) -> Option<Node> {
    let (_, denominator) = split_fraction(node)?;
    let (index, n, exp, _) = factors(denominator).iter().find_map(|factor| root_factor(factor, atoms))?;
    Some(power(atoms.var(index), n - exp))
}

pub fn rationalize_denominators(node: &Node, ctx: &mut Context) -> Option<Rewrite> {
    everywhere(node, ctx, "Rationalize the denominator", RuleId::RationalizeDenominator, rationalize_at)
}
