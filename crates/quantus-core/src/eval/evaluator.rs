//! Evaluates a parsed AST into a [`PhysicalQuantity`].
//!
//! Values stay exact throughout: a number literal enters as a rational, π stays π. Only the
//! caller turns the result into decimal digits.
//!
//! `variables` are the definitions of the current session (F5), keyed as `v_1`. An input that
//! uses a variable the session does not know fails with [`QuantusError::UnresolvedVariable`].

use std::collections::HashMap;

use dashu::base::Abs;
use dashu::integer::{IBig, UBig};
use dashu::rational::RBig;

use crate::latex::{Node, Operator};
use crate::num::{Decimal, MathFunction, Value, DEFAULT_PRECISION, MAX_PRECISION};
use super::derivative;
use crate::numeric;
use crate::quantity::PhysicalQuantity;
use crate::{QuantusError, Result};

pub type Variables = HashMap<String, PhysicalQuantity>;

pub struct Evaluator<'a> {
    pub(super) variables: &'a Variables,
    /// Significant digits that numerical results (integrals, limits) are computed to.
    pub(super) precision: usize,
    /// Digits an integral is computed to: `precision`, but capped, because the work of the
    /// quadrature grows with it and nested integrals multiply that work.
    numeric_digits: usize,
}

/// Longest sum that is worth adding up term by term.
const MAX_SUM_TERMS: i64 = 1_000_000;

/// Extra digits for each level of nested numerical constructs.
const NESTING_DIGITS: usize = 10;

/// Cofactor expansion takes n! steps; this keeps it instant.
const MAX_DETERMINANT_SIZE: usize = 8;

/// Most significant digits an integral is asked for. A numerical result shows its digits as
/// they are, so asking for fewer than the calculator's precision is visible, not silent.
const MAX_NUMERIC_DIGITS: usize = 30;

impl<'a> Evaluator<'a> {
    pub fn new(variables: &'a Variables) -> Self {
        Evaluator { variables, precision: DEFAULT_PRECISION, numeric_digits: DEFAULT_PRECISION.min(MAX_NUMERIC_DIGITS) }
    }

    pub fn with_precision(mut self, precision: usize) -> Self {
        self.precision = precision;
        self.numeric_digits = precision.min(MAX_NUMERIC_DIGITS);
        self
    }

    /// An evaluator that sees the same variables plus `scope`, which the caller updates between
    /// samples.
    fn scoped<'b>(&self, scope: &'b Variables) -> Evaluator<'b> {
        Evaluator { variables: scope, precision: self.precision, numeric_digits: self.numeric_digits }
    }

    pub(super) fn scoped_over<'b>(&self, scope: &'b Variables) -> Evaluator<'b> {
        self.scoped(scope)
    }

    /// Like [`Self::scoped`], for the body of a numerical construct. It is computed with extra
    /// digits: an integral inside an integral would otherwise hand the outer one noise at the
    /// outer tolerance and the outer quadrature would never settle.
    fn scoped_deeper<'b>(&self, scope: &'b Variables) -> Evaluator<'b> {
        Evaluator {
            variables: scope,
            precision: (self.precision + NESTING_DIGITS).min(MAX_PRECISION),
            numeric_digits: self.numeric_digits + NESTING_DIGITS,
        }
    }

    pub fn evaluate(&self, node: &Node) -> Result<PhysicalQuantity> {
        match node {
            Node::Number { value, .. } => Ok(PhysicalQuantity::dimensionless(Value::from_decimal(value))),
            Node::Unit { unit, .. } => Ok(PhysicalQuantity::new(Value::one(), unit.clone())),
            Node::Constant { name, .. } => constant(name).map(PhysicalQuantity::dimensionless),
            Node::Variable { name, index, .. } => {
                let key = Node::variable_key(name, index);
                self.variables.get(&key).cloned().ok_or_else(|| {
                    QuantusError::UnresolvedVariable(if index.is_empty() {
                        format!(
                            "The variable {key} has no value. Only variables with a subscript can be \
                             defined, e.g. {key}_1 = 5m."
                        )
                    } else {
                        format!("The variable {key} has no value yet. Define it first, e.g. {key} = 5m.")
                    })
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
            Node::Function { name, argument, .. } => {
                let function = MathFunction::from_command(name)
                    .ok_or_else(|| QuantusError::other(format!("Unknown function '\\{name}'.")))?;
                let argument = self.plain_number(argument, &format!("The argument of \\{name}"))?;
                argument.apply(function).map(PhysicalQuantity::dimensionless)
            }
            Node::Sum { index, from, to, body, .. } => self.sum(index, from, to, body),
            Node::Integral { variable, lower, upper, body, .. } => self.integral(variable, lower, upper, body),
            Node::Limit { variable, target, body, .. } => self.limit(variable, target, body),
            Node::Derivative { variable, body, at, .. } => self.derivative(variable, body, at.as_deref()),
            Node::Equation { .. } => {
                self.solve(node)?.values.into_iter().next().ok_or_else(|| QuantusError::other("The equation has no solution."))
            }
            Node::Matrix { .. } => Err(QuantusError::other(
                "A matrix is not a number. Take its determinant with \\det.",
            )),
            Node::Determinant { operand, .. } => self.determinant(operand),
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

    fn whole_integer(&self, node: &Node, what: &str) -> Result<i64> {
        let value = self.plain_number(node, what)?;
        value
            .to_i64()
            .ok_or_else(|| QuantusError::other(format!("{what} must be a whole number, but was '{value}'.")))
    }

    // ---- constructs that bind a variable ------------------------------------------------

    fn sum(&self, index: &str, from: &Node, to: &Node, body: &Node) -> Result<PhysicalQuantity> {
        let from = self.whole_integer(from, "The lower limit of a sum")?;
        let to = self.whole_integer(to, "The upper limit of a sum")?;
        if to.saturating_sub(from) >= MAX_SUM_TERMS {
            return Err(QuantusError::other(format!("A sum is limited to {MAX_SUM_TERMS} terms.")));
        }
        let mut scope = self.variables.clone();
        let mut total: Option<PhysicalQuantity> = None;
        for i in from..=to {
            scope.insert(index.to_string(), PhysicalQuantity::dimensionless(Value::int(i)));
            let term = self.scoped(&scope).evaluate(body)?;
            total = Some(match total {
                None => term,
                Some(sum) => sum.add(&term)?,
            });
        }
        Ok(total.unwrap_or_else(|| PhysicalQuantity::dimensionless(Value::zero())))
    }

    /// Numerical, to the calculator's precision. The variable carries the unit of the limits,
    /// and the result the unit of the integrand times that.
    fn integral(&self, variable: &str, lower: &Node, upper: &Node, body: &Node) -> Result<PhysicalQuantity> {
        let (lower, upper) = (self.evaluate(lower)?, self.evaluate(upper)?);
        // A bare 0 as lower limit (`\int_0^{2m}`) takes the unit of the other limit.
        let (lower, upper) = if lower.is_dimensionless() && lower.value.is_zero() {
            (PhysicalQuantity::new(Value::zero(), upper.unit.clone()), upper)
        } else {
            let upper = upper.to(&lower.unit)?;
            (lower, upper)
        };
        let digits = self.numeric_digits + 8;
        let (a, b) = (lower.value.decimal(digits)?, upper.value.decimal(digits)?);

        let mut scope = self.variables.clone();
        let mut integrand_unit = None;
        let mut sample = |x: &Decimal| -> Result<Decimal> {
            scope.insert(
                variable.to_string(),
                PhysicalQuantity::new(Value::from_decimal(x), lower.unit.clone()),
            );
            let y = self.scoped_deeper(&scope).evaluate(body)?;
            let y = match &integrand_unit {
                None => {
                    integrand_unit = Some(y.unit.clone());
                    y
                }
                Some(unit) => y.to(unit)?,
            };
            y.value.decimal(digits + 10)
        };
        let result = numeric::integrate(&a, &b, digits, &mut sample)?;
        let unit = integrand_unit.unwrap_or(crate::units::Unit::ONE);
        let value = Value::from_decimal(&result.round(self.numeric_digits));
        Ok(PhysicalQuantity::new(value, unit).mul(&PhysicalQuantity::new(Value::one(), lower.unit)))
    }

    /// Exact, by cofactor expansion. The entries have to agree in dimension; the result carries
    /// that unit to the power of the matrix size.
    fn determinant(&self, operand: &Node) -> Result<PhysicalQuantity> {
        let mut operand = operand;
        while let Node::Group { inner, .. } = operand {
            operand = inner;
        }
        let Node::Matrix { rows, .. } = operand else {
            return Err(QuantusError::other("\\det needs a matrix, as in \\det\\begin{pmatrix} … \\end{pmatrix}."));
        };
        let size = rows.len();
        if rows.iter().any(|row| row.len() != size) {
            return Err(QuantusError::other(format!(
                "The determinant needs a square matrix, but this one has {size} rows of {} entries.",
                rows[0].len()
            )));
        }
        if size > MAX_DETERMINANT_SIZE {
            return Err(QuantusError::other(format!("Determinants are limited to {MAX_DETERMINANT_SIZE}×{MAX_DETERMINANT_SIZE} matrices.")));
        }

        let mut entries = Vec::with_capacity(size * size);
        for node in rows.iter().flatten() {
            entries.push(self.evaluate(node)?);
        }
        let unit = entries[0].unit.clone();
        let values = entries
            .iter()
            .map(|entry| entry.to(&unit).map(|e| e.value))
            .collect::<Result<Vec<Value>>>()?;

        let determinant = cofactor_determinant(&values, size);
        let scale = PhysicalQuantity::new(Value::one(), unit).pow_int(i32::try_from(size).expect("size is at most 8"))?;
        Ok(PhysicalQuantity::dimensionless(determinant).mul(&scale))
    }

    /// Differentiates symbolically, then evaluates the result at `at`, or with whatever the
    /// variable already stands for.
    fn derivative(&self, variable: &str, body: &Node, at: Option<&Node>) -> Result<PhysicalQuantity> {
        let derived = derivative::differentiate(body, variable)?;
        let mut scope = self.variables.clone();
        match at {
            Some(at) => {
                scope.insert(variable.to_string(), self.evaluate(at)?);
            }
            None if !scope.contains_key(variable) => {
                return Err(QuantusError::UnresolvedVariable(format!(
                    "The derivative with respect to {variable} needs a point to be evaluated at. Write it as \\left.\\frac{{d}}{{d{variable}}} … \\right|_{{{variable}=2}}."
                )));
            }
            None => {}
        }
        self.scoped(&scope).evaluate(&derived)
    }

    /// Two-sided and numerical: the body is sampled just either side of the target and the
    /// samples must agree, which also rules out a limit that diverges.
    fn limit(&self, variable: &str, target: &Node, body: &Node) -> Result<PhysicalQuantity> {
        let target = self.evaluate(target)?;
        let digits = self.precision;
        let working = 3 * digits + 30;
        let centre = target.value.decimal(working)?.to_rational();
        let step = |exponent: usize| Value::Rat(RBig::from_parts(IBig::ONE, UBig::from(10u8).pow(exponent)));

        let mut scope = self.variables.clone();
        let mut unit = None;
        let mut samples = Vec::new();
        for exponent in [digits + 5, digits + 4] {
            for direction in [1, -1] {
                let offset = step(exponent).mul(&Value::int(direction));
                let x = Value::Rat(centre.clone()).add(&offset);
                scope.insert(variable.to_string(), PhysicalQuantity::new(x, target.unit.clone()));
                let y = self.scoped_deeper(&scope).evaluate(body)?;
                let y = match &unit {
                    None => {
                        unit = Some(y.unit.clone());
                        y
                    }
                    Some(u) => y.to(u)?,
                };
                samples.push(y.value.decimal(working)?.to_rational());
            }
        }

        let tolerance = RBig::from_parts(IBig::ONE, UBig::from(10u8).pow(digits.saturating_sub(5)));
        let reference = samples[0].clone();
        let scale = reference.clone().abs().max(RBig::ONE);
        if samples.iter().any(|s| (s.clone() - &reference).abs() > &tolerance * &scale) {
            return Err(QuantusError::other(
                "The limit does not exist: the values on the two sides of the target disagree or grow without bound.",
            ));
        }
        let mean = samples.iter().fold(RBig::ZERO, |sum, s| sum + s) / RBig::from(samples.len());
        let value = Value::from_decimal(&Value::Rat(mean).decimal(digits)?);
        Ok(PhysicalQuantity::new(value, unit.unwrap_or(crate::units::Unit::ONE)))
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

/// The determinant of the row-major `size`×`size` matrix, expanding along the first row.
fn cofactor_determinant(values: &[Value], size: usize) -> Value {
    match size {
        0 => Value::one(),
        1 => values[0].clone(),
        _ => (0..size).fold(Value::zero(), |sum, column| {
            let minor: Vec<Value> = (1..size)
                .flat_map(|row| (0..size).filter(move |c| *c != column).map(move |c| row * size + c))
                .map(|i| values[i].clone())
                .collect();
            let term = values[column].mul(&cofactor_determinant(&minor, size - 1));
            if column % 2 == 0 { sum.add(&term) } else { sum.sub(&term) }
        }),
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
