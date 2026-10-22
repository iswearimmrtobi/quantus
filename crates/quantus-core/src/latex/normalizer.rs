//! F2: the auto-fraction rewrite: every `/` becomes a `\frac{}{}`.
//!
//! Operand boundaries follow standard precedence (`\cdot` and `/` rank equally and associate to
//! the left), so `a \cdot b / c` yields `\frac{a \cdot b}{c}`. Chains are flattened into a
//! common denominator rather than nested: `a/b/c` becomes `\frac{a}{b \cdot c}`, not
//! `\frac{\frac{a}{b}}{c}`. Explicit parentheses always win: `a/(b/c)` keeps its inner
//! fraction, because a parenthesised sub-expression is a `Group` and ends the chain.
//!
//! This runs right after parsing, so evaluation and serialisation both see the same tree.

use std::collections::VecDeque;

use super::ast::{Node, Operator};

pub fn normalize(node: Node) -> Node {
    match node {
        Node::Binary { operator: Operator::Divide, .. } => normalize_division(node),
        Node::Binary { operator, left, right, position } => {
            Node::binary(operator, normalize(*left), normalize(*right), position)
        }
        Node::Fraction { numerator, denominator, position } => {
            Node::fraction(normalize(*numerator), normalize(*denominator), position)
        }
        Node::Power { base, exponent, position } => {
            Node::Power { base: Box::new(normalize(*base)), exponent: Box::new(normalize(*exponent)), position }
        }
        Node::Root { radicand, degree, position } => Node::Root {
            radicand: Box::new(normalize(*radicand)),
            degree: degree.map(|d| Box::new(normalize(*d))),
            position,
        },
        Node::Negation { operand, position } => Node::Negation { operand: Box::new(normalize(*operand)), position },
        Node::Group { inner, position } => Node::Group { inner: Box::new(normalize(*inner)), position },
        leaf => leaf,
    }
}

fn normalize_division(node: Node) -> Node {
    let position = node.position();
    let mut denominators = VecDeque::new();
    let mut numerator = node;
    while let Node::Binary { operator: Operator::Divide, left, right, .. } = numerator {
        denominators.push_front(*right);
        numerator = *left;
    }

    let denominator = denominators
        .into_iter()
        .map(normalize)
        .reduce(|left, right| {
            let position = left.position();
            Node::binary(Operator::Times, left, right, position)
        })
        .expect("a division has at least one denominator");
    Node::fraction(normalize(numerator), denominator, position)
}
