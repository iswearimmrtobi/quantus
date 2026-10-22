//! The parsed LaTeX input. Every node knows where it started in the source so errors can point
//! at the offending spot (F2: parse errors with a position).

use crate::num::Decimal;
use crate::units::Unit;

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    /// A literal number, never a float.
    Number { value: Decimal, position: usize },
    /// A unit, either bare (`kg`) or explicitly marked up (`\mathrm{kg}`); `symbol` is the
    /// spelling the lookup resolved.
    Unit { unit: Unit, symbol: String, position: usize },
    /// A reserved constant name: `\pi`, `e`, `i`.
    Constant { name: String, position: usize },
    /// A variable: `v_1`, or a single letter without a subscript (`x`, `\theta`) that is neither
    /// a unit nor a constant, in which case `index` is empty. Unindexed variables are free: a
    /// session cannot define them, they only get a value from a construct that binds them.
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
    /// A named function applied to one argument: `\sin x`, `\ln{x}`.
    Function { name: String, argument: Box<Node>, position: usize },
    /// `|x|`.
    Abs { operand: Box<Node>, position: usize },
    /// `\binom{n}{k}`.
    Binomial { n: Box<Node>, k: Box<Node>, position: usize },
    /// `x^\circ`: an angle in degrees.
    Degrees { operand: Box<Node>, position: usize },
    /// `\sum_{i=from}^{to} body`.
    Sum { index: String, from: Box<Node>, to: Box<Node>, body: Box<Node>, position: usize },
    /// `\int_{lower}^{upper} body \, dx`.
    Integral { variable: String, lower: Box<Node>, upper: Box<Node>, body: Box<Node>, position: usize },
    /// `\lim_{x \to target} body`.
    Limit { variable: String, target: Box<Node>, body: Box<Node>, position: usize },
    /// `\frac{d}{dx} body`, optionally evaluated at a point: `\left.\frac{d}{dx} body\right|_{x=at}`.
    Derivative { variable: String, body: Box<Node>, at: Option<Box<Node>>, position: usize },
    /// `\begin{pmatrix} … \end{pmatrix}`, row by row.
    Matrix { rows: Vec<Vec<Node>>, position: usize },
    /// `\det` of a matrix.
    Determinant { operand: Box<Node>, position: usize },
    /// `left = right` with a free variable, to be solved.
    Equation { left: Box<Node>, right: Box<Node>, position: usize },
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
            | Node::Group { position, .. }
            | Node::Function { position, .. }
            | Node::Abs { position, .. }
            | Node::Binomial { position, .. }
            | Node::Degrees { position, .. }
            | Node::Sum { position, .. }
            | Node::Integral { position, .. }
            | Node::Limit { position, .. }
            | Node::Derivative { position, .. }
            | Node::Matrix { position, .. }
            | Node::Determinant { position, .. }
            | Node::Equation { position, .. } => *position,
        }
    }

    /// The direct sub-expressions, in source order.
    pub fn children(&self) -> Vec<&Node> {
        match self {
            Node::Number { .. } | Node::Unit { .. } | Node::Constant { .. } | Node::Variable { .. } => vec![],
            Node::Binary { left, right, .. } | Node::Equation { left, right, .. } => vec![left, right],
            Node::Fraction { numerator, denominator, .. } => vec![numerator, denominator],
            Node::Power { base, exponent, .. } => vec![base, exponent],
            Node::Root { radicand, degree, .. } => {
                std::iter::once(&**radicand).chain(degree.as_deref()).collect()
            }
            Node::Negation { operand, .. }
            | Node::Abs { operand, .. }
            | Node::Degrees { operand, .. }
            | Node::Determinant { operand, .. } => vec![operand],
            Node::Group { inner, .. } => vec![inner],
            Node::Function { argument, .. } => vec![argument],
            Node::Binomial { n, k, .. } => vec![n, k],
            Node::Sum { from, to, body, .. } => vec![from, to, body],
            Node::Integral { lower, upper, body, .. } => vec![lower, upper, body],
            Node::Limit { target, body, .. } => vec![target, body],
            Node::Derivative { body, at, .. } => std::iter::once(&**body).chain(at.as_deref()).collect(),
            Node::Matrix { rows, .. } => rows.iter().flatten().collect(),
        }
    }

    /// Rebuilds the node with `f` applied to every direct sub-expression.
    pub fn map_children(self, f: &mut impl FnMut(Node) -> Node) -> Node {
        let mut b = |n: Box<Node>| Box::new(f(*n));
        match self {
            leaf @ (Node::Number { .. } | Node::Unit { .. } | Node::Constant { .. } | Node::Variable { .. }) => leaf,
            Node::Binary { operator, left, right, position } => {
                Node::Binary { operator, left: b(left), right: b(right), position }
            }
            Node::Equation { left, right, position } => Node::Equation { left: b(left), right: b(right), position },
            Node::Fraction { numerator, denominator, position } => {
                Node::Fraction { numerator: b(numerator), denominator: b(denominator), position }
            }
            Node::Power { base, exponent, position } => Node::Power { base: b(base), exponent: b(exponent), position },
            Node::Root { radicand, degree, position } => {
                Node::Root { radicand: b(radicand), degree: degree.map(&mut b), position }
            }
            Node::Negation { operand, position } => Node::Negation { operand: b(operand), position },
            Node::Abs { operand, position } => Node::Abs { operand: b(operand), position },
            Node::Degrees { operand, position } => Node::Degrees { operand: b(operand), position },
            Node::Determinant { operand, position } => Node::Determinant { operand: b(operand), position },
            Node::Group { inner, position } => Node::Group { inner: b(inner), position },
            Node::Function { name, argument, position } => Node::Function { name, argument: b(argument), position },
            Node::Binomial { n, k, position } => Node::Binomial { n: b(n), k: b(k), position },
            Node::Sum { index, from, to, body, position } => {
                Node::Sum { index, from: b(from), to: b(to), body: b(body), position }
            }
            Node::Integral { variable, lower, upper, body, position } => {
                Node::Integral { variable, lower: b(lower), upper: b(upper), body: b(body), position }
            }
            Node::Limit { variable, target, body, position } => {
                Node::Limit { variable, target: b(target), body: b(body), position }
            }
            Node::Derivative { variable, body, at, position } => {
                Node::Derivative { variable, body: b(body), at: at.map(&mut b), position }
            }
            Node::Matrix { rows, position } => Node::Matrix {
                rows: rows.into_iter().map(|row| row.into_iter().map(&mut *f).collect()).collect(),
                position,
            },
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
        if index.is_empty() {
            name.to_string()
        } else {
            format!("{name}_{index}")
        }
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
