//! Replaces variables in an AST by their values: the middle of the three steps from F6.
//! `F = m_1 \cdot a_1` becomes `F = 2\,\mathrm{kg} \cdot 9{,}81\,\frac{\mathrm{m}}{\mathrm{s}^{2}}`.
//!
//! The substituted tree is a display artefact only; the actual arithmetic runs on the exact
//! values in the evaluator, never on these decimals.

use super::evaluator::Variables;
use crate::latex::{Node, Operator};
use crate::quantity::PhysicalQuantity;
use crate::units::registry;

pub fn substitute(node: &Node, variables: &Variables, precision: usize) -> Node {
    let recurse = |n: &Node| Box::new(substitute(n, variables, precision));
    match node {
        Node::Variable { name, index, position } => variables
            .get(&Node::variable_key(name, index))
            .and_then(|quantity| value_node(quantity, *position, precision))
            .unwrap_or_else(|| node.clone()),
        Node::Group { inner, position } => Node::Group { inner: recurse(inner), position: *position },
        Node::Negation { operand, position } => Node::Negation { operand: recurse(operand), position: *position },
        Node::Fraction { numerator, denominator, position } => {
            Node::Fraction { numerator: recurse(numerator), denominator: recurse(denominator), position: *position }
        }
        Node::Power { base, exponent, position } => {
            Node::Power { base: recurse(base), exponent: recurse(exponent), position: *position }
        }
        Node::Root { radicand, degree, position } => Node::Root {
            radicand: recurse(radicand),
            degree: degree.as_deref().map(&recurse),
            position: *position,
        },
        Node::Binary { operator, left, right, position } => {
            Node::Binary { operator: *operator, left: recurse(left), right: recurse(right), position: *position }
        }
        Node::Number { .. } | Node::Unit { .. } | Node::Constant { .. } => node.clone(),
    }
}

/// A quantity as an AST: number, and (unless it is dimensionless) its unit next to it.
/// Values without a decimal form (a complex intermediate result, say) yield `None`; the
/// variable then stays symbolic in the substituted step instead of the step being lost.
fn value_node(quantity: &PhysicalQuantity, position: usize, precision: usize) -> Option<Node> {
    let value = quantity.decimal(precision).ok()?;
    let number = Node::Number { value, position };
    if quantity.unit.is_one() {
        return Some(number);
    }
    let unit = Node::Unit { unit: quantity.unit.clone(), symbol: registry::format(&quantity.unit), position };
    Some(Node::binary(Operator::ImplicitTimes, number, unit, position))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::Evaluator;
    use crate::latex::{parser::parse, serializer};

    fn variables(definitions: &[(&str, &str)]) -> Variables {
        let mut variables = Variables::new();
        for (key, input) in definitions {
            let formula = parse(input).unwrap();
            let quantity = Evaluator::new(&variables).evaluate(&formula.expression).unwrap();
            variables.insert((*key).to_string(), quantity);
        }
        variables
    }

    fn substituted(input: &str, variables: &Variables) -> String {
        serializer::expression(&substitute(&parse(input).unwrap().expression, variables, 50))
    }

    #[test]
    fn values_and_units_are_put_in() {
        let vars = variables(&[("m_1", "2kg"), ("a_1", "9.81\\frac{m}{s^2}")]);
        assert_eq!(
            substituted("m_1 \\cdot a_1", &vars),
            "2\\,\\mathrm{kg} \\cdot 9{,}81\\,\\frac{\\mathrm{m}}{\\mathrm{s}^{2}}"
        );
    }

    #[test]
    fn a_substituted_power_base_gets_brackets() {
        let vars = variables(&[("v_1", "\\frac{200km}{2.5h}")]);
        assert_eq!(substituted("v_1^2", &vars), "(80\\,\\frac{\\mathrm{km}}{\\mathrm{h}})^{2}");
    }

    #[test]
    fn unknown_and_complex_variables_stay_symbolic() {
        let vars = variables(&[("z_1", "i")]);
        assert_eq!(substituted("z_1 + w_1", &vars), "z_{1} + w_{1}");
    }
}
