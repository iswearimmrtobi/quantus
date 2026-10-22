//! The parsed LaTeX input. Every node knows where it started in the source so errors can point
//! at the offending spot (F2: parse errors with a position).

use crate::num::Decimal;
use crate::units::Unit;

#[derive(Clone, Debug)]
pub enum Node {
    /// A literal number, never a float.
    Number { value: Decimal, position: usize },
    /// A unit, either bare (`kg`) or explicitly marked up (`\mathrm{kg}`); `symbol` is the
    /// spelling the lookup resolved.
    Unit { unit: Unit, symbol: String, position: usize },
    /// A reserved constant name: `\pi`, `e`, `i`.
    Constant { name: String, position: usize },
    /// A variable. Per F2 it always carries a subscript; an identifier without one is a unit
    /// or a constant, never a variable.
    Variable { name: String, index: String, position: usize },
    Binary { operator: Operator, left: Box<Node>, right: Box<Node>, position: usize },
    /// `\frac{a}{b}`, and what every `/` turns into during normalisation (F2, auto-fraction).
    Fraction { numerator: Box<Node>, denominator: Box<Node>, position: usize },
    Power { base: Box<Node>, exponent: Box<Node>, position: usize },
    /// `\sqrt{x}` or `\sqrt[n]{x}`.
    Root { radicand: Box<Node>, degree: Option<Box<Node>>, position: usize },
    Negation { operand: Box<Node>, position: usize },
    /// An explicitly parenthesised sub-expression. Kept in the tree because explicit
    /// parentheses win over the fraction-chain normalisation (`a/(b/c)` must not collapse); the
    /// serialiser re-derives parentheses from operator precedence and treats this as transparent.
    Group { inner: Box<Node>, position: usize },
}

impl Node {
    pub fn position(&self) -> usize {
        match self {
            Node::Number { position, .. }
            | Node::Unit { position, .. }
            | Node::Constant { position, .. }
            | Node::Variable { position, .. }
            | Node::Binary { position, .. }
            | Node::Fraction { position, .. }
            | Node::Power { position, .. }
            | Node::Root { position, .. }
            | Node::Negation { position, .. }
            | Node::Group { position, .. } => *position,
        }
    }

    pub fn binary(operator: Operator, left: Node, right: Node, position: usize) -> Node {
        Node::Binary { operator, left: Box::new(left), right: Box::new(right), position }
    }

    pub fn fraction(numerator: Node, denominator: Node, position: usize) -> Node {
        Node::Fraction { numerator: Box::new(numerator), denominator: Box::new(denominator), position }
    }

    /// How the session addresses a variable node: `v_1`.
    pub fn variable_key(name: &str, index: &str) -> String {
        format!("{name}_{index}")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operator {
    Plus,
    Minus,
    Times,
    /// `/`, replaced by a fraction during normalisation.
    Divide,
    /// Juxtaposition: `2kg`, `10\frac{m}{s^2}`, `2\pi`.
    ImplicitTimes,
}

/// The left-hand side of `F = …`. A label **with** a subscript is a variable definition (F5):
/// `r_1 = 0{,}5\,\mathrm{m}` defines `r_1`; a label without one is just a name for the line,
/// per the uniqueness rule of F2.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label {
    pub name: String,
    pub index: Option<String>,
    pub position: usize,
}

impl Label {
    /// The session key of the variable this label defines, or `None` if it defines none.
    pub fn key(&self) -> Option<String> {
        self.index.as_ref().map(|index| Node::variable_key(&self.name, index))
    }
}

/// A complete input: an optional `label =` and the expression itself.
#[derive(Clone, Debug)]
pub struct Formula {
    pub label: Option<Label>,
    pub expression: Node,
    pub source: String,
}
