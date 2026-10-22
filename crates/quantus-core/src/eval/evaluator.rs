//! Evaluates a parsed AST into a [`PhysicalQuantity`].
//!
//! Values stay exact throughout: a number literal enters as a rational, π stays π. Only the
//! caller turns the result into decimal digits.
//!
//! `variables` are the definitions of the current session (F5), keyed as `v_1`. An input that
//! uses a variable the session does not know fails with [`QuantusError::UnresolvedVariable`].

use std::collections::HashMap;

use dashu::integer::IBig;
use dashu::rational::RBig;

use crate::latex::{Node, Operator};
use crate::num::Value;
use crate::quantity::PhysicalQuantity;
use crate::{QuantusError, Result};

pub type Variables = HashMap<String, PhysicalQuantity>;

pub struct Evaluator<'a> {
    variables: &'a Variables,
}

impl<'a> Evaluator<'a> {
    pub fn new(variables: &'a Variables) -> Self {
        Evaluator { variables }
    }

    pub fn evaluate(&self, node: &Node) -> Result<PhysicalQuantity> {
        match node {
            Node::Number { value, .. } => Ok(PhysicalQuantity::dimensionless(Value::from_decimal(value))),
            Node::Unit { unit, .. } => Ok(PhysicalQuantity::new(Value::one(), unit.clone())),
            Node::Constant { name, .. } => constant(name).map(PhysicalQuantity::dimensionless),
            Node::Variable { name, index, .. } => {
                let key = Node::variable_key(name, index);
                self.variables.get(&key).cloned().ok_or_else(|| {
                    QuantusError::UnresolvedVariable(format!(
                        "The variable {key} has no value yet. Define it first, e.g. {key} = 5m."
                    ))
                })
            }
            Node::Group { inner, .. } => self.evaluate(inner),
            Node::Negation { operand, .. } => Ok(self.evaluate(operand)?.neg()),
            Node::Fraction { numerator, denominator, .. } => {
                self.evaluate(numerator)?.div(&self.evaluate(denominator)?)
            }
            Node::Power { base, exponent, .. } => self.evaluate(base)?.pow(&self.evaluate(exponent)?),
            Node::Root { radicand, degree, .. } => {
                let degree = self.degree(degree.as_deref())?;
                self.evaluate(radicand)?.root(degree)
            }
            Node::Binary { operator, left, right, .. } => {
                let left = self.evaluate(left)?;
                let right = self.evaluate(right)?;
                match operator {
                    Operator::Plus => left.add(&right),
                    Operator::Minus => left.sub(&right),
                    Operator::Times | Operator::ImplicitTimes => Ok(left.mul(&right)),
                    Operator::Divide => left.div(&right),
                }
            }
            Node::Abs { operand, .. } => {
                let quantity = self.evaluate(operand)?;
                if quantity.value.is_negative()? { Ok(quantity.neg()) } else { Ok(quantity) }
            }
            Node::Degrees { operand, .. } => {
                let angle = self.plain_number(operand, "An angle in degrees")?;
                Ok(PhysicalQuantity::dimensionless(angle.mul(&Value::Pi).div(&Value::int(180))?))
            }
            Node::Binomial { n, k, .. } => {
                let n = self.whole_number(n, "binom")?;
                let k = self.whole_number(k, "binom")?;
                binomial(n, k).map(|value| PhysicalQuantity::dimensionless(Value::Rat(RBig::from(value))))
            }
            Node::Function { .. }
            | Node::Sum { .. }
            | Node::Integral { .. }
            | Node::Limit { .. }
            | Node::Derivative { .. }
            | Node::Matrix { .. }
            | Node::Determinant { .. }
            | Node::Equation { .. } => Err(QuantusError::other("This kind of expression cannot be evaluated yet.")),
        }
    }

    /// A dimensionless operand's value; `what` names the construct for the error.
    fn plain_number(&self, node: &Node, what: &str) -> Result<Value> {
        let quantity = self.evaluate(node)?;
        if !quantity.is_dimensionless() {
            return Err(QuantusError::other(format!("{what} must be a plain number, not a quantity with a unit.")));
        }
        Ok(quantity.value)
    }

    fn whole_number(&self, node: &Node, command: &str) -> Result<i64> {
        let value = self.plain_number(node, &format!("An argument of \\{command}"))?;
        value.to_i64().filter(|n| *n >= 0).ok_or_else(|| {
            QuantusError::other(format!("The arguments of \\{command} must be non-negative whole numbers, but one was '{value}'."))
        })
    }

    fn degree(&self, degree: Option<&Node>) -> Result<u32> {
        let Some(degree) = degree else { return Ok(2) };
        let quantity = self.evaluate(degree)?;
        if !quantity.is_dimensionless() {
            return Err(QuantusError::other(
                "The degree of a root must be a plain number, not a quantity with a unit.",
            ));
        }
        match quantity.value.to_i64() {
            Some(n) if n > 0 && n <= i64::from(u32::MAX) => Ok(n as u32),
            _ => Err(QuantusError::other(format!(
                "The degree of a root must be a positive whole number, but was '{}'.",
                quantity.value
            ))),
        }
    }
}

/// `n` over `k`, exact. Zero when `k > n`.
fn binomial(n: i64, k: i64) -> Result<IBig> {
    const LIMIT: i64 = 100_000;
    if k > n {
        return Ok(IBig::ZERO);
    }
    let k = k.min(n - k);
    if k > LIMIT {
        return Err(QuantusError::other("\\binom is limited to a lower index of 100000."));
    }
    let mut result = IBig::ONE;
    for i in 0..k {
        result = result * IBig::from(n - i) / IBig::from(i + 1);
    }
    Ok(result)
}

/// Reserved constant names (F2). They are looked up after the unit table, so a symbol that is
/// a unit stays a unit. The values are symbolic: π stays π until the last numeric evaluation.
fn constant(name: &str) -> Result<Value> {
    match name {
        "\\pi" => Ok(Value::Pi),
        "e" => Ok(Value::E),
        "i" => Ok(Value::I),
        _ => Err(QuantusError::other(format!("Unknown constant '{name}'."))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::latex::parser::parse;

    fn eval(input: &str, variables: &Variables) -> Result<PhysicalQuantity> {
        let formula = parse(input).unwrap();
        Evaluator::new(variables).evaluate(&formula.expression)?.simplify()
    }

    fn value(input: &str) -> String {
        let quantity = eval(input, &Variables::new()).unwrap();
        format!("{} {}", quantity.decimal(20).unwrap().to_plain_string(), quantity.unit)
    }

    #[test]
    fn arithmetic_keeps_units() {
        assert_eq!(value("2kg \\cdot 10\\frac{m}{s^2}"), "20 N");
        assert_eq!(value("2kg + 500g"), "2.5 kg");
        assert_eq!(value("\\sqrt{4m^2}"), "2 m");
        assert_eq!(value("\\sqrt[3]{8}"), "2 1");
    }

    #[test]
    fn constants_are_symbolic_until_the_end() {
        assert_eq!(value("2\\pi").as_str(), "6.2831853071795864769 1");
    }

    #[test]
    fn unknown_variables_are_refused_with_a_hint() {
        let error = eval("v_1 \\cdot 2s", &Variables::new()).unwrap_err();
        assert!(matches!(error, QuantusError::UnresolvedVariable(_)));
        assert_eq!(error.to_string(), "The variable v_1 has no value yet. Define it first, e.g. v_1 = 5m.");
    }

    #[test]
    fn variables_are_taken_from_the_map() {
        let mut variables = Variables::new();
        variables.insert("m_1".into(), eval("2kg", &Variables::new()).unwrap());
        let quantity = eval("m_1 \\cdot 3", &variables).unwrap();
        assert_eq!(quantity.decimal(5).unwrap().to_plain_string(), "6");
    }

    #[test]
    fn root_degrees_must_be_plain_positive_whole_numbers() {
        let with_unit = eval("\\sqrt[2m]{4}", &Variables::new()).unwrap_err();
        assert!(with_unit.to_string().contains("plain number"));
        let zero = eval("\\sqrt[0]{4}", &Variables::new()).unwrap_err();
        assert!(zero.to_string().contains("positive whole number, but was '0'"));
    }
}
