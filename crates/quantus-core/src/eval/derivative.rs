//! Symbolic differentiation of an AST with respect to one variable.
//!
//! The derivative is another [`Node`] that the evaluator then evaluates like any other, so a
//! derivative can contain session variables, units and nested sums. Rules that would build a
//! needless `0 + …` or `1 · …` are folded as they are made: besides keeping the tree small, a
//! literal `0` would not add to a quantity with a unit.

use crate::latex::{Node, Operator};
use crate::num::Decimal;
use crate::{QuantusError, Result};

pub fn differentiate(node: &Node, var: &str) -> Result<Node> {
    let position = node.position();
    let d = |n: &Node| differentiate(n, var);
    Ok(match node {
        Node::Number { .. } | Node::Unit { .. } | Node::Constant { .. } => number(0, position),
        Node::Variable { name, index, .. } => number(i64::from(index.is_empty() && name == var), position),
        Node::Group { inner, .. } => d(inner)?,
        Node::Negation { operand, .. } => negate(d(operand)?, position),
        Node::Binary { operator, left, right, .. } => match operator {
            Operator::Plus => add(d(left)?, d(right)?, position),
            Operator::Minus => subtract(d(left)?, d(right)?, position),
            Operator::Times | Operator::ImplicitTimes => product_rule(left, right, var, position)?,
            Operator::Divide => quotient_rule(left, right, var, position)?,
        },
        Node::Fraction { numerator, denominator, .. } => quotient_rule(numerator, denominator, var, position)?,
        Node::Power { base, exponent, .. } => power_rule(base, exponent, var, position)?,
        Node::Root { radicand, degree, .. } => {
            let degree = degree.as_deref().cloned().unwrap_or_else(|| number(2, position));
            let exponent = Node::fraction(number(1, position), degree, position);
            power_rule(radicand, &exponent, var, position)?
        }
        Node::Function { name, argument, .. } => {
            let inner = d(argument)?;
            if is_zero(&inner) {
                return Ok(inner);
            }
            let u = (**argument).clone();
            let outer = match name.as_str() {
                "sin" => function("cos", u, position),
                "cos" => negate(function("sin", u, position), position),
                "tan" => Node::fraction(number(1, position), square(function("cos", u, position), position), position),
                "ln" => Node::fraction(number(1, position), u, position),
                "log" => Node::fraction(
                    number(1, position),
                    multiply(u, function("ln", number(10, position), position), position),
                    position,
                ),
                other => return Err(QuantusError::other(format!("Cannot differentiate \\{other}."))),
            };
            multiply(outer, inner, position)
        }
        // |u|' = u u' / |u|
        Node::Abs { operand, .. } => {
            let inner = d(operand)?;
            if is_zero(&inner) {
                return Ok(inner);
            }
            let u = (**operand).clone();
            Node::fraction(multiply(u.clone(), inner, position), Node::Abs { operand: Box::new(u), position }, position)
        }
        // x° is x π / 180
        Node::Degrees { operand, .. } => {
            let inner = d(operand)?;
            let factor = Node::fraction(Node::Constant { name: "\\pi".into(), position }, number(180, position), position);
            multiply(inner, factor, position)
        }
        Node::Sum { index, from, to, body, .. } => {
            require_independent([from, to], var, "the limits of a sum")?;
            if index == var {
                return Ok(number(0, position));
            }
            let body = d(body)?;
            if is_zero(&body) {
                return Ok(body);
            }
            Node::Sum { index: index.clone(), from: from.clone(), to: to.clone(), body: Box::new(body), position }
        }
        Node::Integral { variable, lower, upper, body, .. } => {
            require_independent([lower, upper], var, "the limits of an integral")?;
            if variable == var {
                return Ok(number(0, position));
            }
            let body = d(body)?;
            if is_zero(&body) {
                return Ok(body);
            }
            Node::Integral {
                variable: variable.clone(),
                lower: lower.clone(),
                upper: upper.clone(),
                body: Box::new(body),
                position,
            }
        }
        // A derivative inside a derivative: work the inner one out first.
        Node::Derivative { variable, body, at: None, .. } => d(&differentiate(body, variable)?)?,
        Node::Binomial { .. } | Node::Limit { .. } | Node::Derivative { .. } | Node::Matrix { .. } | Node::Determinant { .. } | Node::Equation { .. } => {
            if !depends_on(node, var) {
                return Ok(number(0, position));
            }
            return Err(QuantusError::other("This expression cannot be differentiated."));
        }
    })
}

/// Does `node` mention `var` as a free variable?
pub fn depends_on(node: &Node, var: &str) -> bool {
    match node {
        Node::Variable { name, index, .. } => index.is_empty() && name == var,
        Node::Sum { index: bound, from, to, body, .. } => {
            depends_on(from, var) || depends_on(to, var) || (bound != var && depends_on(body, var))
        }
        Node::Integral { variable: bound, lower, upper, body, .. } => {
            depends_on(lower, var) || depends_on(upper, var) || (bound != var && depends_on(body, var))
        }
        Node::Limit { variable: bound, target, body, .. } => {
            depends_on(target, var) || (bound != var && depends_on(body, var))
        }
        // Without a point the derivative is still a function of its variable.
        Node::Derivative { body, at: None, .. } => depends_on(body, var),
        Node::Derivative { variable: bound, body, at: Some(at), .. } => {
            depends_on(at, var) || (bound != var && depends_on(body, var))
        }
        other => other.children().into_iter().any(|child| depends_on(child, var)),
    }
}

fn require_independent<'a>(nodes: impl IntoIterator<Item = &'a Box<Node>>, var: &str, what: &str) -> Result<()> {
    if nodes.into_iter().any(|node| depends_on(node, var)) {
        return Err(QuantusError::other(format!("Cannot differentiate when {what} depend on {var}.")));
    }
    Ok(())
}

fn product_rule(u: &Node, v: &Node, var: &str, position: usize) -> Result<Node> {
    let (du, dv) = (differentiate(u, var)?, differentiate(v, var)?);
    Ok(add(multiply(du, v.clone(), position), multiply(u.clone(), dv, position), position))
}

fn quotient_rule(u: &Node, v: &Node, var: &str, position: usize) -> Result<Node> {
    let (du, dv) = (differentiate(u, var)?, differentiate(v, var)?);
    if is_zero(&dv) {
        return Ok(if is_zero(&du) { number(0, position) } else { Node::fraction(du, v.clone(), position) });
    }
    let top = subtract(multiply(du, v.clone(), position), multiply(u.clone(), dv, position), position);
    Ok(Node::fraction(top, square(v.clone(), position), position))
}

fn power_rule(base: &Node, exponent: &Node, var: &str, position: usize) -> Result<Node> {
    let (db, de) = (differentiate(base, var)?, differentiate(exponent, var)?);
    let (b, e) = (base.clone(), exponent.clone());
    let one = number(1, position);
    // d(b^e) = e b^(e-1) b'  when e is constant
    let by_base = || {
        multiply(
            multiply(e.clone(), power(b.clone(), subtract(e.clone(), one.clone(), position), position), position),
            db.clone(),
            position,
        )
    };
    // d(b^e) = b^e ln(b) e'  when b is constant
    let by_exponent = || {
        multiply(multiply(power(b.clone(), e.clone(), position), function("ln", b.clone(), position), position), de.clone(), position)
    };
    Ok(match (is_zero(&db), is_zero(&de)) {
        (true, true) => number(0, position),
        (false, true) => by_base(),
        (true, false) => by_exponent(),
        (false, false) => {
            // b^e (e' ln b + e b'/b)
            let inner = add(
                multiply(de, function("ln", b.clone(), position), position),
                Node::fraction(multiply(e.clone(), db, position), b.clone(), position),
                position,
            );
            multiply(power(b, e, position), inner, position)
        }
    })
}

// ---- constructors that fold the trivial cases ----------------------------------------------

fn number(value: i64, position: usize) -> Node {
    Node::Number { value: Decimal::from_i64(value), position }
}

fn is_number(node: &Node, value: i64) -> bool {
    matches!(node, Node::Number { value: v, .. } if v.to_rational() == dashu::rational::RBig::from(value))
}

fn is_zero(node: &Node) -> bool {
    is_number(node, 0)
}

fn add(a: Node, b: Node, position: usize) -> Node {
    match (is_zero(&a), is_zero(&b)) {
        (true, _) => b,
        (_, true) => a,
        _ => Node::binary(Operator::Plus, a, b, position),
    }
}

fn subtract(a: Node, b: Node, position: usize) -> Node {
    match (is_zero(&a), is_zero(&b)) {
        (_, true) => a,
        (true, _) => negate(b, position),
        _ => Node::binary(Operator::Minus, a, b, position),
    }
}

fn negate(a: Node, position: usize) -> Node {
    if is_zero(&a) {
        a
    } else {
        Node::Negation { operand: Box::new(a), position }
    }
}

fn multiply(a: Node, b: Node, position: usize) -> Node {
    if is_zero(&a) || is_zero(&b) {
        return number(0, position);
    }
    if is_number(&a, 1) {
        return b;
    }
    if is_number(&b, 1) {
        return a;
    }
    Node::binary(Operator::Times, a, b, position)
}

fn power(base: Node, exponent: Node, position: usize) -> Node {
    Node::Power { base: Box::new(base), exponent: Box::new(exponent), position }
}

fn square(base: Node, position: usize) -> Node {
    power(base, number(2, position), position)
}

fn function(name: &str, argument: Node, position: usize) -> Node {
    Node::Function { name: name.to_string(), argument: Box::new(argument), position }
}
