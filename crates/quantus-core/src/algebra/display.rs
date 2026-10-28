//! Nodes that only exist in the lines of the working: brackets that make a grouping visible.
//! The rules never see them; they appear in the sub-steps of a rewrite, which are only written
//! out, never simplified further.

use crate::latex::ast::Node;

/// The name of the presentation-only call that writes its argument in brackets.
pub const BRACKET: &str = "bracket";

/// `node` written in brackets, even where the order of operations would not need them:
/// `(x^2 - 6x + 9) - 9 + 3`.
pub fn bracket(node: Node) -> Node {
    Node::Call { name: BRACKET.to_string(), arguments: vec![node], position: 0 }
}

/// Builds `first ± second ± …` from signed terms, `true` meaning plus.
pub fn signed_sum(first: Node, rest: impl IntoIterator<Item = (bool, Node)>) -> Node {
    use crate::latex::ast::Operator;
    rest.into_iter().fold(first, |acc, (positive, term)| {
        Node::binary(if positive { Operator::Plus } else { Operator::Minus }, acc, term, 0)
    })
}

/// `±t_1 ± t_2 ± …`, a leading minus written as a negation.
pub fn signed_terms_sum(terms: impl IntoIterator<Item = (bool, Node)>) -> Option<Node> {
    let mut terms = terms.into_iter();
    let (positive, first) = terms.next()?;
    let first = if positive { first } else { Node::Negation { operand: Box::new(first), position: 0 } };
    Some(signed_sum(first, terms))
}
