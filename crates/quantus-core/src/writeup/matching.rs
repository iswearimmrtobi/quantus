//! Reading a calculation as a monomial (`\frac{10V}{0{,}5A}` = (10 V)¹ · (0,5 A)⁻¹) and finding
//! the law it applies.

use std::collections::{HashMap, HashSet};

use dashu::integer::IBig;
use dashu::rational::RBig;

use super::laws::{self, Form, Law, LAWS};
use crate::eval::Evaluator;
use crate::latex::ast::{Node, Operator};
use crate::latex::serializer;
use crate::quantity::PhysicalQuantity;
use crate::units::Dimension;

#[derive(Clone, Debug, PartialEq)]
pub enum LeafKind {
    /// A number with a unit as typed: `10V`, `9{,}81\frac{m}{s^2}`.
    Literal,
    /// A session variable, a physical constant or `Ans`, by its key.
    Variable(String),
    /// Anything that is not a product of powers (`10Ω + 20Ω`), or a part split off on purpose:
    /// it gets a name and a calculation of its own.
    Composite,
}

/// One factor of a monomial: what it is, its power in the monomial and its value.
#[derive(Clone, Debug)]
pub struct Leaf {
    pub node: Node,
    pub exponent: i32,
    pub quantity: PhysicalQuantity,
    pub kind: LeafKind,
}

/// `(c · Π leafᵢ^eᵢ)^(1/root)`.
#[derive(Clone, Debug)]
pub struct Monomial {
    pub root: u32,
    pub coefficient: RBig,
    pub leaves: Vec<Leaf>,
}

/// A law that fits a monomial: the law solved for the wanted symbol, and for each factor of
/// that form the leaf it stands for.
#[derive(Clone, Debug)]
pub struct Match {
    pub law: &'static Law,
    pub form: Form,
    pub leaves: Vec<usize>,
}

/// The id of a node, to name nodes that are to be read as one leaf.
pub fn node_id(node: &Node) -> (usize, String) {
    (node.position(), serializer::expression(node))
}

/// `node` as a monomial. Nodes in `split` are leaves of their own (composites), whatever they
/// look like. `None` when a part has no value.
pub fn monomial(node: &Node, evaluator: &Evaluator, split: &HashSet<(usize, String)>) -> Option<Monomial> {
    let (root, radicand) = match strip(node) {
        Node::Root { radicand, degree: None, .. } => (2, &**radicand),
        Node::Root { radicand, degree: Some(degree), .. } => match integer(degree) {
            Some(n) if n > 1 => (n as u32, &**radicand),
            _ => (1, node),
        },
        _ => (1, node),
    };
    let mut builder = Builder { evaluator, split, coefficient: RBig::ONE, leaves: Vec::new() };
    builder.walk(radicand, 1)?;
    // Equal factors become one power: `U · U` is `U²`.
    let mut merged: Vec<Leaf> = Vec::new();
    let mut seen: HashMap<String, usize> = HashMap::new();
    for leaf in builder.leaves {
        let key = serializer::expression(&leaf.node);
        match seen.get(&key) {
            Some(&i) => merged[i].exponent += leaf.exponent,
            None => {
                seen.insert(key, merged.len());
                merged.push(leaf);
            }
        }
    }
    merged.retain(|leaf| leaf.exponent != 0);
    Some(Monomial { root, coefficient: builder.coefficient, leaves: merged })
}

struct Builder<'a, 'b> {
    evaluator: &'a Evaluator<'b>,
    split: &'a HashSet<(usize, String)>,
    coefficient: RBig,
    leaves: Vec<Leaf>,
}

impl Builder<'_, '_> {
    fn walk(&mut self, node: &Node, exponent: i32) -> Option<()> {
        if self.split.contains(&node_id(node)) {
            return self.leaf(node, exponent, LeafKind::Composite);
        }
        if is_literal(node) {
            return self.leaf(node, exponent, LeafKind::Literal);
        }
        match node {
            Node::Group { inner, .. } => self.walk(inner, exponent),
            Node::Number { value, .. } => {
                self.coefficient = &self.coefficient * power(&value.to_rational(), exponent)?;
                Some(())
            }
            Node::Fraction { numerator, denominator, .. } => {
                self.walk(numerator, exponent)?;
                self.walk(denominator, -exponent)
            }
            Node::Binary { operator: Operator::Times | Operator::ImplicitTimes, .. } => self.product(node, exponent),
            Node::Power { base, exponent: power, .. } => match integer(power) {
                Some(n) => self.walk(base, exponent.checked_mul(n)?),
                None => self.leaf(node, exponent, LeafKind::Composite),
            },
            Node::Variable { name, index, .. } => {
                self.leaf(node, exponent, LeafKind::Variable(Node::variable_key(name, index)))
            }
            _ => self.leaf(node, exponent, LeafKind::Composite),
        }
    }

    /// A product chain. It is parsed from the left, `10V \cdot 2A` as `((10 V) · 2) A`, so a
    /// number and the units right after it are put together again into one value.
    fn product(&mut self, node: &Node, exponent: i32) -> Option<()> {
        let mut factors = Vec::new();
        product_factors(node, Operator::Times, &mut factors);
        let mut i = 0;
        while i < factors.len() {
            let factor = factors[i].1;
            let mut end = i + 1;
            if is_number(factor) {
                while end < factors.len() && factors[end].0 == Operator::ImplicitTimes && is_unit(factors[end].1) {
                    end += 1;
                }
            }
            if end > i + 1 {
                let unit = factors[i + 2..end].iter().fold(factors[i + 1].1.clone(), |acc, (_, u)| {
                    Node::binary(Operator::ImplicitTimes, acc, (*u).clone(), u.position())
                });
                let literal = Node::binary(Operator::ImplicitTimes, factor.clone(), unit, factor.position());
                self.leaf(&literal, exponent, LeafKind::Literal)?;
            } else {
                self.walk(factor, exponent)?;
            }
            i = end;
        }
        Some(())
    }

    fn leaf(&mut self, node: &Node, exponent: i32, kind: LeafKind) -> Option<()> {
        let quantity = self.evaluator.evaluate(node).ok()?;
        self.leaves.push(Leaf { node: node.clone(), exponent, quantity, kind });
        Some(())
    }
}

/// The factors of a product chain with the operator in front of each.
fn product_factors<'n>(node: &'n Node, joined: Operator, out: &mut Vec<(Operator, &'n Node)>) {
    match node {
        Node::Binary { operator: operator @ (Operator::Times | Operator::ImplicitTimes), left, right, .. } => {
            product_factors(left, joined, out);
            product_factors(right, *operator, out);
        }
        _ => out.push((joined, node)),
    }
}

fn is_number(node: &Node) -> bool {
    match node {
        Node::Number { .. } => true,
        Node::Negation { operand, .. } => matches!(**operand, Node::Number { .. }),
        _ => false,
    }
}

fn strip(node: &Node) -> &Node {
    match node {
        Node::Group { inner, .. } => strip(inner),
        _ => node,
    }
}

/// An integer exponent: `2`, `-1`, `{3}`.
pub fn integer(node: &Node) -> Option<i32> {
    match strip(node) {
        Node::Number { value, .. } => {
            let rational = value.to_rational();
            (rational.denominator() == &dashu::integer::UBig::ONE)
                .then(|| i32::try_from(rational.numerator()).ok())
                .flatten()
        }
        Node::Negation { operand, .. } => integer(operand).map(|n| -n),
        _ => None,
    }
}

fn power(base: &RBig, exponent: i32) -> Option<RBig> {
    if exponent < 0 && base == &RBig::ZERO {
        return None;
    }
    let mut result = RBig::ONE;
    for _ in 0..exponent.unsigned_abs() {
        result = &result * base;
    }
    Some(if exponent < 0 { RBig::ONE / result } else { result })
}

/// Only units: `V`, `\frac{m}{s^2}`, `kg \cdot m`.
fn is_unit(node: &Node) -> bool {
    match node {
        Node::Unit { .. } => true,
        Node::Group { inner, .. } => is_unit(inner),
        Node::Fraction { numerator, denominator, .. } => {
            (is_unit(numerator) || matches!(**numerator, Node::Number { .. })) && is_unit(denominator)
        }
        Node::Power { base, exponent, .. } => is_unit(base) && integer(exponent).is_some(),
        Node::Binary { operator: Operator::Times | Operator::ImplicitTimes, left, right, .. } => is_unit(left) && is_unit(right),
        _ => false,
    }
}

/// A number with a unit: `10V`, `-3\,\mathrm{m}`, `9{,}81\frac{m}{s^2}`.
pub fn is_literal(node: &Node) -> bool {
    match node {
        Node::Binary { operator: Operator::ImplicitTimes, left, right, .. } => is_number(left) && is_unit(right),
        Node::Group { inner, .. } => is_literal(inner),
        _ => false,
    }
}

/// The parts of a node that could be worked out on their own: groups, numerators and
/// denominators and the bases of powers, outermost first. Values and names are left out.
pub fn candidates(node: &Node) -> Vec<&Node> {
    fn visit<'n>(node: &'n Node, top: bool, out: &mut Vec<&'n Node>) {
        if is_literal(node) || matches!(node, Node::Number { .. } | Node::Variable { .. } | Node::Unit { .. }) {
            return;
        }
        let worth = !top && !matches!(node, Node::Group { .. });
        if worth {
            out.push(node);
        }
        match node {
            Node::Group { inner, .. } => visit(inner, false, out),
            Node::Fraction { numerator, denominator, .. } => {
                visit(numerator, false, out);
                visit(denominator, false, out);
            }
            Node::Power { base, .. } => visit(base, false, out),
            Node::Root { radicand, .. } if top => visit(radicand, true, out),
            Node::Binary { operator: Operator::Times | Operator::ImplicitTimes, .. } => {
                let mut factors = Vec::new();
                product_factors(node, Operator::Times, &mut factors);
                for (_, factor) in factors {
                    visit(factor, false, out);
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    visit(node, true, &mut out);
    out
}

/// The terms of a sum with whether each is subtracted: `a + b - c`. `None` for anything else.
pub fn sum_terms(node: &Node) -> Option<Vec<(bool, &Node)>> {
    fn visit<'n>(node: &'n Node, negative: bool, out: &mut Vec<(bool, &'n Node)>) {
        match node {
            Node::Binary { operator: operator @ (Operator::Plus | Operator::Minus), left, right, .. } => {
                visit(left, negative, out);
                visit(right, negative ^ (*operator == Operator::Minus), out);
            }
            _ => out.push((negative, node)),
        }
    }
    let node = strip(node);
    if !matches!(node, Node::Binary { operator: Operator::Plus | Operator::Minus, .. }) {
        return None;
    }
    let mut out = Vec::new();
    visit(node, false, &mut out);
    Some(out)
}

/// The first law that fits `monomial` with its left side, or any other symbol, as the result.
pub fn find_law(monomial: &Monomial, result: &Dimension, precision: usize) -> Option<Match> {
    for law in LAWS {
        let symbols = law.symbols();
        for (target, symbol) in symbols.iter().enumerate() {
            if symbol.dimension() != *result {
                continue;
            }
            let form = law.solve_for(target);
            if form.root != monomial.root || form.coefficient != monomial.coefficient || form.factors.len() != monomial.leaves.len() {
                continue;
            }
            let mut assignment = Vec::new();
            let mut used = vec![false; monomial.leaves.len()];
            if assign(&form, &symbols, monomial, precision, &mut assignment, &mut used) {
                return Some(Match { law, form, leaves: assignment });
            }
        }
    }
    None
}

/// Pairs the form's factors with distinct leaves of the same power and dimension, trying
/// every order (laws have at most four symbols).
fn assign(form: &Form, symbols: &[laws::Symbol], monomial: &Monomial, precision: usize, assignment: &mut Vec<usize>, used: &mut [bool]) -> bool {
    let Some(&(symbol, exponent)) = form.factors.get(assignment.len()) else { return true };
    let symbol = symbols[symbol];
    for (i, leaf) in monomial.leaves.iter().enumerate() {
        if used[i] || leaf.exponent != exponent || leaf.quantity.dimension() != symbol.dimension() || !fits(&symbol, leaf, precision) {
            continue;
        }
        used[i] = true;
        assignment.push(i);
        if assign(form, symbols, monomial, precision, assignment, used) {
            return true;
        }
        assignment.pop();
        used[i] = false;
    }
    false
}

/// Whether a leaf has the fixed value its symbol may ask for.
fn fits(symbol: &laws::Symbol, leaf: &Leaf, precision: usize) -> bool {
    if symbol.near.is_none() {
        return true;
    }
    let value = leaf
        .quantity
        .to(&symbol.si_unit())
        .ok()
        .and_then(|q| q.decimal(precision).ok())
        .and_then(|d| d.to_plain_string().parse::<f64>().ok());
    value.is_some_and(|v| laws::near(symbol, v))
}

/// A rational as a fraction of two integers, for display: `(1, 2)`.
pub fn parts(value: &RBig) -> (IBig, IBig) {
    (value.numerator().clone(), IBig::from(value.denominator().clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::Variables;
    use crate::latex::parser;
    use crate::settings::Settings;

    fn matched(latex: &str) -> Option<&'static str> {
        let variables = Variables::new();
        let evaluator = Evaluator::new(&variables);
        let formula = parser::parse_with(latex, &Settings::default().parse_options()).unwrap();
        let monomial = monomial(&formula.expression, &evaluator, &HashSet::new()).unwrap();
        let result = evaluator.evaluate(&formula.expression).unwrap().dimension();
        find_law(&monomial, &result, 20).map(|m| m.law.id)
    }

    #[test]
    fn a_fraction_of_a_voltage_and_a_current_is_ohms_law() {
        assert_eq!(matched("\\frac{10V}{0{,}5A}"), Some("ohm"));
    }

    #[test]
    fn the_exponents_tell_the_power_laws_apart() {
        assert_eq!(matched("10V \\cdot 2A"), Some("electric-power"));
        assert_eq!(matched("\\frac{(10V)^2}{5\\Omega}"), Some("power-voltage-resistance"));
    }

    #[test]
    fn the_coefficient_has_to_fit() {
        assert_eq!(matched("\\frac{1}{2} \\cdot 2kg \\cdot (3\\frac{m}{s})^2"), Some("kinetic-energy"));
        assert_eq!(matched("2kg \\cdot (3\\frac{m}{s})^2"), None);
    }

    #[test]
    fn an_acceleration_near_g_makes_a_weight() {
        assert_eq!(matched("2kg \\cdot 9{,}81\\frac{m}{s^2}"), Some("weight"));
        assert_eq!(matched("2kg \\cdot 3\\frac{m}{s^2}"), Some("newton"));
    }

    #[test]
    fn a_root_is_read_as_a_rearranged_law() {
        assert_eq!(matched("\\sqrt{\\frac{2 \\cdot 9J}{2kg}}"), Some("kinetic-energy"));
    }
}
