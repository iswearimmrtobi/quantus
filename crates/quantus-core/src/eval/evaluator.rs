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
use super::{angles, derivative, series};
use crate::numeric;
use crate::settings::{AngleUnit, Mode, NumberFormat};
use crate::quantity::PhysicalQuantity;
use crate::{QuantusError, Result};
use crate::text::msg;

pub type Variables = HashMap<String, PhysicalQuantity>;

pub struct Evaluator<'a> {
    pub(crate) variables: &'a Variables,
    /// Significant digits that numerical results (integrals, limits) are computed to.
    pub(super) precision: usize,
    /// Digits an integral is computed to: `precision`, but capped, because the work of the
    /// quadrature grows with it and nested integrals multiply that work.
    numeric_digits: usize,
    /// Algebra and analysis mode know no physical constants (`c_0`, `h_P`, ...); imaginary mode computes
    /// with complex numbers where the other modes report an error.
    mode: Mode,
    /// Whether a plain number in `\sin` is read in degrees (the angle unit setting).
    degrees: bool,
    /// What `\operatorname{Rnd}` rounds to: the number format's digits.
    rounding: NumberFormat,
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
        Evaluator { variables, precision: DEFAULT_PRECISION, numeric_digits: DEFAULT_PRECISION.min(MAX_NUMERIC_DIGITS), mode: Mode::default(), degrees: false, rounding: NumberFormat::Auto }
    }

    pub fn with_precision(mut self, precision: usize) -> Self {
        self.precision = precision;
        self.numeric_digits = precision.min(MAX_NUMERIC_DIGITS);
        self
    }

    pub fn with_mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }

    /// Reads plain numbers in trigonometric functions in this unit. The curve sketch ignores it.
    pub fn with_angle_unit(mut self, unit: AngleUnit) -> Self {
        self.degrees = unit == AngleUnit::Degree;
        self
    }

    /// `\operatorname{Rnd}` rounds to the digits this format shows.
    pub fn with_number_format(mut self, format: NumberFormat) -> Self {
        self.rounding = format;
        self
    }

    pub(super) fn degrees(&self) -> bool {
        self.degrees && self.mode != Mode::Analysis
    }

    pub(super) fn complex(&self) -> bool {
        self.mode == Mode::Imaginary
    }

    /// The physical constant of that name, unless the mode has none.
    pub(super) fn physical_constant(&self, key: &str) -> Option<PhysicalQuantity> {
        if matches!(self.mode, Mode::Algebra | Mode::Imaginary | Mode::Analysis | Mode::Linear | Mode::BaseN | Mode::Matrix | Mode::Statistics | Mode::Distributions | Mode::Verify | Mode::Geometry | Mode::Logic) { None } else { physical_constant(key) }
    }

    /// An evaluator that sees the same variables plus `scope`, which the caller updates between
    /// samples.
    fn scoped<'b>(&self, scope: &'b Variables) -> Evaluator<'b> {
        Evaluator { variables: scope, precision: self.precision, numeric_digits: self.numeric_digits, mode: self.mode, degrees: self.degrees, rounding: self.rounding }
    }

    pub(crate) fn scoped_over<'b>(&self, scope: &'b Variables) -> Evaluator<'b> {
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
            mode: self.mode,
            degrees: self.degrees,
            rounding: self.rounding,
        }
    }

    /// The node's value. An error points at the innermost node it came from, so the input can
    /// be marked there.
    pub fn evaluate(&self, node: &Node) -> Result<PhysicalQuantity> {
        self.evaluate_node(node).map_err(|error| error.at_node(node))
    }

    fn evaluate_node(&self, node: &Node) -> Result<PhysicalQuantity> {
        match node {
            Node::Number { value, .. } => Ok(PhysicalQuantity::dimensionless(Value::from_decimal(value))),
            Node::Unit { unit, .. } => Ok(PhysicalQuantity::new(Value::one(), unit.clone())),
            Node::Constant { name, .. } => constant(name).map(PhysicalQuantity::dimensionless),
            Node::Variable { name, index, .. } => {
                let key = Node::variable_key(name, index);
                if let Some(value) = self.variables.get(&key) {
                    return Ok(value.clone());
                }
                if let Some(constant) = self.physical_constant(&key) {
                    return Ok(constant);
                }
                Err(QuantusError::UnresolvedVariable(if index.is_empty() {
                    msg!("core.eval.evaluator.variable_without_subscript"
                    )
                } else {
                    msg!("core.eval.evaluator.variable_undefined")
                }))
            }
            Node::Group { inner, .. } => self.evaluate(inner),
            Node::Negation { operand, .. } => Ok(self.evaluate(operand)?.neg()),
            Node::Fraction { numerator, denominator, .. } => {
                let numerator = self.evaluate(numerator)?;
                let divisor = self.evaluate(denominator)?;
                numerator.div(&divisor).map_err(|error| by_zero(error, &divisor, denominator))
            }
            Node::Power { base, exponent, .. } => self.evaluate(base)?.pow(&self.evaluate(exponent)?),
            Node::Root { radicand, degree, .. } => {
                let degree = self.degree(degree.as_deref())?;
                self.evaluate(radicand)?.root(degree)
            }
            Node::Binary { operator, left, right: right_node, .. } => {
                let left = self.evaluate(left)?;
                let right = self.evaluate(right_node)?;
                match operator {
                    Operator::Plus => left.add(&right),
                    Operator::Minus => left.sub(&right),
                    Operator::Times | Operator::ImplicitTimes => Ok(left.mul(&right)),
                    Operator::Divide => left.div(&right).map_err(|error| by_zero(error, &right, right_node)),
                }
            }
            Node::Abs { operand, .. } => {
                let quantity = self.evaluate(operand)?;
                if self.complex() && quantity.value.contains_imaginary() {
                    if let Some(modulus) = quantity.value.modulus() {
                        return Ok(PhysicalQuantity::new(modulus?, quantity.unit));
                    }
                }
                if quantity.value.is_negative()? { Ok(quantity.neg()) } else { Ok(quantity) }
            }
            Node::Degrees { operand, .. } => {
                let angle = self.plain_number(operand, &msg!("core.eval.evaluator.angle_degrees"))?;
                // Degree mode computes in degrees, so the sign changes nothing there.
                let angle = if self.degrees() { angle } else { angles::to_radians(&angle)? };
                Ok(PhysicalQuantity::dimensionless(angle))
            }
            Node::Binomial { n, k, .. } => {
                let n = self.whole_number(n, "binom")?;
                let k = self.whole_number(k, "binom")?;
                binomial(n, k).map(|value| PhysicalQuantity::dimensionless(Value::Rat(RBig::from(value))))
            }
            Node::Function { name, argument, .. } if self.complex() && is_complex_function(name) => {
                let argument = self.plain_number(argument, &msg!("core.eval.evaluator.argument"))?;
                let unknown = || QuantusError::other(msg!("core.eval.evaluator.needs_number_form_bi"));
                let value = match name.as_str() {
                    "Re" => argument.real_part().ok_or_else(unknown)?,
                    "Im" => argument.imaginary_part().ok_or_else(unknown)?,
                    "overline" => argument.conjugate().ok_or_else(unknown)?,
                    _ if self.degrees() => angles::to_degrees(&argument.argument().ok_or_else(unknown)??)?,
                    _ => argument.argument().ok_or_else(unknown)??,
                };
                Ok(PhysicalQuantity::dimensionless(value))
            }
            Node::Function { name, argument, .. } => {
                let function = MathFunction::from_command(name)
                    .ok_or_else(|| QuantusError::other(msg!("core.eval.evaluator.unknown_function")))?;
                let in_degrees = self.degrees() && angles::takes_angle(name) && !angles::is_marked(argument);
                let mut argument = self.plain_number(argument, &msg!("core.eval.evaluator.argument"))?;
                if in_degrees {
                    argument = angles::to_radians(&argument)?;
                }
                let mut applied = if self.complex() { argument.apply_complex(function) } else { argument.apply(function) }?;
                if self.degrees() && angles::gives_angle(name) {
                    if applied.contains_imaginary() {
                        return Err(QuantusError::other(msg!("core.eval.evaluator.complex_angle_in_degrees"
                        )));
                    }
                    applied = angles::to_degrees(&applied)?;
                }
                Ok(PhysicalQuantity::dimensionless(applied))
            }
            Node::Sum { index, from, to, body, product: false, .. } => self.sum(index, from, to, body),
            Node::Sum { index, from, to, body, product: true, .. } => self.product(index, from, to, body),
            Node::Integral { variable, lower, upper, body, .. } => self.integral(variable, lower, upper, body),
            Node::Antiderivative { .. } => Err(QuantusError::other(msg!("core.eval.evaluator.antiderivative_alone"))),
            Node::Limit { variable, target, body, side, .. } => self.limit(variable, target, body, *side),
            Node::Derivative { variable, body, at, given, .. } => self.derivative(variable, body, at.as_deref(), given),
            Node::Equation { .. } => {
                self.solve(node)?.values.into_iter().next().ok_or_else(|| QuantusError::other(msg!("core.common.equation_has_no_solution")))
            }
            Node::System { .. } => Err(QuantusError::other(
                msg!("core.eval.evaluator.system_inside_calculation"),
            )),
            Node::Matrix { .. } => Err(QuantusError::other(
                msg!("core.eval.evaluator.matrix_not_number"),
            )),
            Node::Determinant { operand, .. } => self.determinant(operand),
            Node::Call { name, arguments, .. } => self.call(name, arguments),
        }
    }

    /// Whether `left operator right` holds: `=`, `<`, `>`, `\le`, `\ge` or `\neq`. Both sides
    /// must have the same dimension; equal means equal to 40 digits, so `\sqrt{2}^2 = 2` holds.
    pub(crate) fn compare(&self, left: &Node, operator: &str, right: &Node) -> Result<bool> {
        let left = self.evaluate(left)?;
        let right = self.evaluate(right)?.to(&left.unit)?;
        let difference = right.value.sub(&left.value);
        let sign = match difference.as_rational() {
            Some(exact) => if exact.is_zero() { 0 } else if *exact < RBig::ZERO { -1 } else { 1 },
            None => match difference.decimal(40) {
                Ok(decimal) => {
                    let size = decimal.to_rational();
                    let tiny = RBig::from_parts(IBig::ONE, UBig::from(10u8).pow(35));
                    if size.clone().abs() < tiny { 0 } else if decimal.is_negative() { -1 } else { 1 }
                }
                // A complex difference: only equal or not.
                Err(_) => {
                    let zero = difference.re_im().is_some_and(|(re, im)| {
                        [re, im].iter().all(|part| part.decimal(40).is_ok_and(|d| d.to_rational().abs() < RBig::from_parts(IBig::ONE, UBig::from(10u8).pow(35))))
                    });
                    if !zero && !matches!(operator, "=" | "\\neq") {
                        return Err(QuantusError::other(msg!("core.eval.evaluator.complex_no_order")));
                    }
                    if zero { 0 } else { 2 }
                }
            },
        };
        // sign is that of right - left.
        Ok(match operator {
            "=" => sign == 0,
            "\\neq" => sign != 0,
            "<" => sign == 1,
            ">" => sign == -1,
            "\\le" => sign >= 0 && sign != 2,
            "\\ge" => sign <= 0,
            other => return Err(QuantusError::other(msg!("core.eval.evaluator.unknown_comparison"))),
        })
    }

    /// A dimensionless operand's value; `what` names the construct for the error, which points
    /// at the operand.
    fn plain_number(&self, node: &Node, what: &str) -> Result<Value> {
        let quantity = self.evaluate(node)?;
        if !quantity.is_dimensionless() {
            return Err(QuantusError::other(msg!("core.eval.evaluator.must_be_plain")).at_node(node));
        }
        quantity.dimensionless_value()
    }

    fn whole_number(&self, node: &Node, command: &str) -> Result<i64> {
        let value = self.plain_number(node, &msg!("core.eval.evaluator.argument_of_command"))?;
        value.to_i64().filter(|n| *n >= 0).ok_or_else(|| {
            QuantusError::other(msg!("core.eval.evaluator.arguments_natural")).at_node(node)
        })
    }

    fn whole_integer(&self, node: &Node, what: &str) -> Result<i64> {
        let value = self.plain_number(node, what)?;
        value
            .to_i64()
            .ok_or_else(|| QuantusError::other(msg!("core.eval.evaluator.must_be_whole")).at_node(node))
    }

    // ---- constructs that bind a variable ------------------------------------------------

    fn sum(&self, index: &str, from: &Node, to: &Node, body: &Node) -> Result<PhysicalQuantity> {
        if infinity_sign(to) == Some(1) {
            return self.infinite_sum(index, from, body);
        }
        let from = self.whole_integer(from, &msg!("core.eval.evaluator.lower_limit_sum"))?;
        let to = self.whole_integer(to, &msg!("core.eval.evaluator.upper_limit_sum"))?;
        if to.saturating_sub(from) >= MAX_SUM_TERMS {
            return Err(QuantusError::other(msg!("core.eval.evaluator.sum_limited_terms")));
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

    /// `\sum_{k=a}^{\infty}`, numerically to at most 30 digits like an integral; how depends on
    /// the terms, see [`super::series`]. The terms must be numbers without a unit.
    fn infinite_sum(&self, index: &str, from: &Node, body: &Node) -> Result<PhysicalQuantity> {
        let from = self.whole_integer(from, &msg!("core.eval.evaluator.lower_limit_sum"))?;
        let digits = self.numeric_digits + 10;
        let total = match super::inequality::rational_function(self, body, index, &mut Vec::new()) {
            Ok((p, _)) if p.iter().all(|c| *c == RBig::ZERO) => RBig::ZERO,
            Ok((p, q)) => {
                let n = series::rational_tail_start(&p, &q, from)?;
                let head = series::rational_partial_sum(&p, &q, from, n)?;
                let lower = Node::Number { value: Decimal::from_i64(n), position: Node::SYNTHETIC };
                let upper = Node::Constant { name: "\\infty".into(), position: Node::SYNTHETIC };
                let integral = self.integral(index, &lower, &upper, body)?.value.decimal(digits)?.to_rational();
                head + integral + series::euler_maclaurin_correction(&p, &q, n)
            }
            Err(_) => {
                let mut scope = self.variables.clone();
                let mut term = |k: i64| -> Result<RBig> {
                    scope.insert(index.to_string(), PhysicalQuantity::dimensionless(Value::int(k)));
                    let quantity = self.scoped(&scope).evaluate(body)?;
                    if !quantity.unit.is_one() {
                        return Err(QuantusError::other(msg!("core.eval.evaluator.infinite_sum_unit")));
                    }
                    Ok(quantity.value.decimal(digits + 10)?.to_rational())
                };
                series::numeric_series(&mut term, from, digits)?
            }
        };
        let value = Value::from_decimal(&Value::Rat(total).decimal(self.numeric_digits)?);
        Ok(PhysicalQuantity::dimensionless(value))
    }

    /// `\prod`: the terms multiplied, units and all; an empty product is 1.
    fn product(&self, index: &str, from: &Node, to: &Node, body: &Node) -> Result<PhysicalQuantity> {
        let from = self.whole_integer(from, &msg!("core.eval.evaluator.lower_limit_product"))?;
        let to = self.whole_integer(to, &msg!("core.eval.evaluator.upper_limit_product"))?;
        if to.saturating_sub(from) >= MAX_SUM_TERMS {
            return Err(QuantusError::other(msg!("core.eval.evaluator.product_limited_factors")));
        }
        let mut scope = self.variables.clone();
        let mut total = PhysicalQuantity::dimensionless(Value::one());
        for i in from..=to {
            scope.insert(index.to_string(), PhysicalQuantity::dimensionless(Value::int(i)));
            total = total.mul(&self.scoped(&scope).evaluate(body)?);
        }
        Ok(total)
    }

    /// Numerical, to the calculator's precision. The variable carries the unit of the limits,
    /// and the result the unit of the integrand times that.
    fn integral(&self, variable: &str, lower: &Node, upper: &Node, body: &Node) -> Result<PhysicalQuantity> {
        let (lower_inf, upper_inf) = (infinity_sign(lower), infinity_sign(upper));
        if lower_inf.is_some() || upper_inf.is_some() {
            return self.improper_integral(variable, lower, upper, body, lower_inf, upper_inf);
        }
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

        // A kink (|x - 1| at 1) slows tanh-sinh to a crawl; integrating up to it and on from it
        // keeps every piece smooth.
        let mut points = vec![a.clone()];
        points.extend(self.kinks(variable, body, &lower, &a, &b, digits)?);
        points.push(b.clone());
        let mut integrand_unit = None;
        let mut parts = Vec::new();
        let mut complex_seen = false;
        // The real part, and the imaginary part if a sample had one (Imaginary mode only).
        for part in [Part::Real, Part::Imaginary] {
            if part == Part::Imaginary && !complex_seen {
                break;
            }
            let mut scope = self.variables.clone();
            let mut sample = |x: &Decimal| -> Result<Decimal> {
                scope.insert(variable.to_string(), PhysicalQuantity::new(Value::from_decimal(x), lower.unit.clone()));
                let y = self.scoped_deeper(&scope).evaluate(body)?;
                let y = match &integrand_unit {
                    None => {
                        integrand_unit = Some(y.unit.clone());
                        y
                    }
                    Some(unit) => y.to(unit)?,
                };
                self.integrand_part(&y.value, part, digits + 10, &mut complex_seen)
            };
            let mut result = RBig::ZERO;
            for piece in points.windows(2) {
                result += numeric::integrate(&piece[0], &piece[1], digits, &mut sample)?.to_rational();
            }
            parts.push(result);
        }
        let unit = integrand_unit.unwrap_or(crate::units::Unit::ONE);
        let value = self.integral_value(parts)?;
        Ok(PhysicalQuantity::new(value, unit).mul(&PhysicalQuantity::new(Value::one(), lower.unit)))
    }

    /// The part of an integrand's sample that is being integrated. Outside Imaginary mode the
    /// value must be real; in it, a sample with an imaginary part sets `complex_seen`, so the
    /// imaginary part is integrated in a second pass.
    fn integrand_part(&self, value: &Value, part: Part, digits: usize, complex_seen: &mut bool) -> Result<Decimal> {
        if !self.complex() || !value.contains_imaginary() {
            return match part {
                Part::Real => value.decimal(digits),
                Part::Imaginary => Ok(Decimal::from_i64(0)),
            };
        }
        let Some((re, im)) = value.re_im() else { return value.decimal(digits) };
        if !im.is_zero() {
            *complex_seen = true;
        }
        match part {
            Part::Real => re.decimal(digits),
            Part::Imaginary => im.decimal(digits),
        }
    }

    /// The integral's value from its real part and, if there is one, its imaginary part. Both
    /// come from one integral whose precision is relative to its modulus, so a part below that
    /// precision next to the other is 0 (the real part of `\int_0^{\pi} e^{ix}\,dx`).
    fn integral_value(&self, parts: Vec<RBig>) -> Result<Value> {
        let rounded = |r: RBig| -> Result<Value> { Ok(Value::from_decimal(&Value::Rat(r).decimal(self.numeric_digits)?)) };
        let mut parts = parts.into_iter();
        let re = parts.next().unwrap_or(RBig::ZERO);
        let Some(im) = parts.next() else { return rounded(re) };
        let noise = |part: &RBig, other: &RBig| {
            let scale = RBig::from_parts(IBig::ONE, UBig::from(10u8).pow(self.numeric_digits));
            part.clone().abs() < other.clone().abs() * scale
        };
        let (re, im) = match (noise(&re, &im), noise(&im, &re)) {
            (true, _) => (RBig::ZERO, im),
            (_, true) => (re, RBig::ZERO),
            _ => (re, im),
        };
        Ok(rounded(re)?.add(&rounded(im)?.mul(&Value::I)))
    }

    /// An integral with an infinite bound, mapped onto a finite interval: x = a + t/(1-t) on
    /// [0, 1) for [a, ∞), x = b - t/(1-t) for (-∞, b], x = t/(1-t²) on (-1, 1) for (-∞, ∞),
    /// with the Jacobian in the integrand. Far out, where the integrand of a convergent integral
    /// has vanished, a sample that cannot be computed (e^(-x²) at x = 10^80) counts as 0.
    fn improper_integral(
        &self,
        variable: &str,
        lower: &Node,
        upper: &Node,
        body: &Node,
        lower_inf: Option<i64>,
        upper_inf: Option<i64>,
    ) -> Result<PhysicalQuantity> {
        // Orient the bounds so that an infinite lower bound is -∞ and an infinite upper one +∞.
        let (lower, upper, lower_inf, upper_inf, sign) = match (lower_inf, upper_inf) {
            (Some(l), Some(u)) if l == u => {
                return Ok(PhysicalQuantity::dimensionless(Value::zero()))
            }
            (Some(1), _) | (_, Some(-1)) => (upper, lower, upper_inf, lower_inf, -1),
            _ => (lower, upper, lower_inf, upper_inf, 1),
        };
        let finite = match (lower_inf, upper_inf) {
            (None, _) => Some(self.evaluate(lower)?),
            (_, None) => Some(self.evaluate(upper)?),
            _ => None,
        };
        let variable_unit = finite
            .as_ref()
            .map(|q| q.unit.clone())
            .unwrap_or(crate::units::Unit::ONE);
        let anchor = match &finite {
            Some(q) => q.value.decimal(self.numeric_digits + 20)?.to_rational(),
            None => RBig::ZERO,
        };
        let digits = self.numeric_digits + 8;
        let one = RBig::ONE;
        let far = RBig::from(IBig::from(10).pow(20));

        let mut integrand_unit = None;
        let mut parts = Vec::new();
        let mut complex_seen = false;
        for part in [Part::Real, Part::Imaginary] {
            if part == Part::Imaginary && !complex_seen {
                break;
            }
            let mut scope = self.variables.clone();
            let mut sample = |t: &Decimal| -> Result<Decimal> {
                let t = t.to_rational();
                let (x, jacobian) = match (lower_inf, upper_inf) {
                    (None, Some(_)) => {
                        let s = &one - &t;
                        (&anchor + &t / &s, &one / (&s * &s))
                    }
                    (Some(_), None) => {
                        let s = &one - &t;
                        (&anchor - &t / &s, &one / (&s * &s))
                    }
                    _ => {
                        let s = &one - &t * &t;
                        (&t / &s, (&one + &t * &t) / (&s * &s))
                    }
                };
                // Close to t = 1 the exact x and Jacobian have hundreds of digits; the working
                // precision is all the integrand and the sum need.
                let x = Value::Rat(x).decimal(digits + 10)?.to_rational();
                let jacobian = Value::Rat(jacobian).decimal(digits + 10)?;
                scope.insert(
                    variable.to_string(),
                    PhysicalQuantity::new(Value::Rat(x.clone()), variable_unit.clone()),
                );
                let y = match self.scoped_deeper(&scope).evaluate(body) {
                    Ok(y) => y,
                    Err(_) if x.clone().abs() > far => return Ok(Decimal::from_i64(0)),
                    Err(e) => return Err(e),
                };
                let y = match &integrand_unit {
                    None => {
                        integrand_unit = Some(y.unit.clone());
                        y
                    }
                    Some(unit) => y.to(unit)?,
                };
                // Decimals, not rationals: the integrand can be 10^-800000 far out, and the exact
                // fraction of that is a number with 800,000 digits.
                let value = match self.integrand_part(&y.value, part, digits + 10, &mut complex_seen) {
                    Ok(v) => v,
                    Err(_) if x.clone().abs() > far => return Ok(Decimal::from_i64(0)),
                    Err(e) => return Err(e),
                };
                let product = Decimal::new(value.unscaled() * jacobian.unscaled(), value.scale() + jacobian.scale());
                Ok(product.round(digits + 10).normalized())
            };
            let (a, b) = match (lower_inf, upper_inf) {
                (Some(_), Some(_)) => (Decimal::from_i64(-1), Decimal::from_i64(1)),
                (None, Some(_)) => (Decimal::from_i64(0), Decimal::from_i64(1)),
                // x = anchor - t/(1-t) runs from -∞ (t → 1) to the anchor (t = 0).
                _ => (Decimal::from_i64(0), Decimal::from_i64(1)),
            };
            parts.push(numeric::integrate(&a, &b, digits, &mut sample)?.to_rational() * RBig::from(sign));
        }
        let unit = integrand_unit.unwrap_or(crate::units::Unit::ONE);
        let value = self.integral_value(parts)?;
        Ok(PhysicalQuantity::new(value, unit)
            .mul(&PhysicalQuantity::new(Value::one(), variable_unit)))
    }

    /// `n!`, floor, ceiling, max, min and mod. Floor, ceiling and mod keep the unit of their
    /// argument; max and min need arguments of one dimension and answer in the first one's unit.
    fn call(&self, name: &str, arguments: &[Node]) -> Result<PhysicalQuantity> {
        let real = |q: &PhysicalQuantity| -> Result<RBig> { Ok(q.value.decimal(self.precision + 10)?.to_rational()) };
        match (name, arguments) {
            ("polar", [r, theta]) => {
                if !self.complex() {
                    return Err(QuantusError::other(msg!("core.eval.evaluator.polar_needs_complex_mode")));
                }
                let modulus = self.evaluate(r)?;
                let mut angle = self.plain_number(theta, &msg!("core.eval.evaluator.polar_angle"))?;
                if self.degrees() && !angles::is_marked(theta) {
                    angle = angles::to_radians(&angle)?;
                }
                let (cos, sin) = (angle.apply(MathFunction::Cos)?, angle.apply(MathFunction::Sin)?);
                let direction = cos.add(&sin.mul(&Value::I));
                Ok(PhysicalQuantity::new(modulus.value.mul(&direction), modulus.unit))
            }
            ("factorial", [n]) => {
                // Not whole: Γ(n + 1), so (½)! = √π/2.
                let value = self.plain_number(n, &msg!("core.eval.evaluator.argument_factorial"))?;
                if !value.as_rational().is_some_and(|r| *r.denominator() == UBig::ONE) {
                    let gamma = value.add(&Value::one()).apply(MathFunction::Gamma)?;
                    return Ok(PhysicalQuantity::dimensionless(gamma));
                }
                if value.is_negative()? {
                    return Err(QuantusError::other(msg!("core.eval.evaluator.factorial_pole")));
                }
                let n = self.whole_number(n, "factorial")?;
                if n > 10_000 {
                    return Err(QuantusError::other(msg!("core.eval.evaluator.too_large_compute_exactly")));
                }
                let product = (2..=n).fold(IBig::ONE, |p, k| p * IBig::from(k));
                Ok(PhysicalQuantity::dimensionless(Value::Rat(RBig::from(product))))
            }
            (relation, operands) if relation.starts_with("relation ") => {
                let mut holds = true;
                for (operator, pair) in relation.split(' ').skip(1).zip(operands.windows(2)) {
                    holds &= self.compare(&pair[0], operator, &pair[1])?;
                }
                Ok(PhysicalQuantity::dimensionless(Value::int(i64::from(holds))))
            }
            ("convert", [value, target]) => {
                let value = self.evaluate(value)?;
                let target = self.evaluate(target)?;
                if !target.value.as_rational().is_some_and(|r| *r == RBig::ONE) || target.unit.is_one() {
                    return Err(QuantusError::other(msg!("core.eval.evaluator.unit_alone_after_arrow")));
                }
                value.to(&target.unit)
            }
            ("nPr", [n, r]) => {
                let (n, r) = (self.whole_number(n, "nPr")?, self.whole_number(r, "nPr")?);
                if r > n {
                    return Err(QuantusError::other(format!("nPr needs r ≤ n, but r = {r} and n = {n}.")));
                }
                if n > 10_000 {
                    return Err(QuantusError::other(msg!("core.eval.evaluator.permutations_too_large")));
                }
                let product = (n - r + 1..=n).fold(IBig::ONE, |p, k| p * IBig::from(k));
                Ok(PhysicalQuantity::dimensionless(Value::Rat(RBig::from(product))))
            }
            ("gcd" | "lcm", [first, rest @ ..]) => {
                let whole = |node: &Node| -> Result<IBig> {
                    let value = self.plain_number(node, &msg!("core.eval.evaluator.argument_of_name"))?;
                    value.to_i64().map(IBig::from).ok_or_else(|| {
                        QuantusError::other(msg!("core.eval.evaluator.arguments_whole"))
                    })
                };
                let mut result = dashu::base::UnsignedAbs::unsigned_abs(whole(first)?);
                for argument in rest {
                    let next = dashu::base::UnsignedAbs::unsigned_abs(whole(argument)?);
                    result = if name == "gcd" {
                        dashu::base::Gcd::gcd(result, next)
                    } else if result == UBig::ZERO || next == UBig::ZERO {
                        UBig::ZERO
                    } else {
                        let divisor = dashu::base::Gcd::gcd(result.clone(), next.clone());
                        result / divisor * next
                    };
                }
                Ok(PhysicalQuantity::dimensionless(Value::Rat(RBig::from(IBig::from(result)))))
            }
            ("Int", [x]) => {
                // Toward zero, keeping the unit.
                let q = self.evaluate(x)?;
                let r = match q.value.as_rational() {
                    Some(r) => r.clone(),
                    None => real(&q)?,
                };
                Ok(PhysicalQuantity::new(Value::Rat(RBig::from(r.trunc())), q.unit))
            }
            ("Rnd" | "RndFix", [x, places @ ..]) => {
                let q = self.evaluate(x)?;
                let decimal = q.value.decimal(self.precision + 10)?;
                let rounded = match (name, places, self.rounding) {
                    ("RndFix", [places], _) => {
                        let places = self.whole_number(places, "RndFix")?;
                        crate::latex::numbers::to_places(&decimal, places.min(1000))
                    }
                    ("RndFix", _, _) => return Err(QuantusError::other(msg!("core.eval.evaluator.rndfix_arguments"))),
                    (_, [], NumberFormat::Fix { digits }) => crate::latex::numbers::to_places(&decimal, i64::from(digits)),
                    (_, [], NumberFormat::Sci { digits }) => decimal.round(if digits == 0 { 10 } else { usize::from(digits) }),
                    // Norm rounds to the ten digits the display shows.
                    (_, [], _) => decimal.round(10),
                    _ => return Err(QuantusError::other(msg!("core.eval.evaluator.rnd_arguments"))),
                };
                Ok(PhysicalQuantity::new(Value::Rat(rounded.to_rational()), q.unit))
            }
            ("divR", [a, b]) => {
                let (a, b) = (self.whole_integer(a, "The dividend of ÷R")?, self.whole_integer(b, &msg!("core.eval.evaluator.remainder_divisor"))?);
                if b == 0 {
                    return Err(QuantusError::other(msg!("core.common.division_zero")));
                }
                Ok(PhysicalQuantity::dimensionless(Value::int(a / b)))
            }
            ("floor" | "ceil", [x]) => {
                let q = self.evaluate(x)?;
                let r = match q.value.as_rational() {
                    Some(r) => r.clone(),
                    None => real(&q)?,
                };
                let rounded = if name == "floor" { r.floor() } else { r.ceil() };
                Ok(PhysicalQuantity::new(Value::Rat(RBig::from(rounded)), q.unit))
            }
            ("mod", [a, b]) => {
                let (a, b) = (self.evaluate(a)?, self.evaluate(b)?);
                let b = b.to(&a.unit)?;
                let (ra, rb) = (real(&a)?, real(&b)?);
                if rb.is_zero() {
                    return Err(QuantusError::other(msg!("core.common.division_zero")));
                }
                let quotient = RBig::from((&ra / &rb).floor());
                Ok(PhysicalQuantity::new(Value::Rat(ra - rb * quotient), a.unit))
            }
            ("max" | "min", [first, rest @ ..]) => {
                let mut best = self.evaluate(first)?;
                let mut best_value = real(&best)?;
                for argument in rest {
                    let q = self.evaluate(argument)?.to(&best.unit)?;
                    let value = real(&q)?;
                    if (name == "max" && value > best_value) || (name == "min" && value < best_value) {
                        best_value = value;
                        best = q;
                    }
                }
                Ok(best)
            }
            _ => Err(QuantusError::other(msg!("core.eval.evaluator.unknown_function"))),
        }
    }

    /// Points strictly between `a` and `b` where the operand of an absolute value in `body`
    /// changes sign, ordered from `a` to `b`. Found on a coarse grid and refined by bisection.
    fn kinks(&self, variable: &str, body: &Node, lower: &PhysicalQuantity, a: &Decimal, b: &Decimal, digits: usize) -> Result<Vec<Decimal>> {
        const GRID: i64 = 64;
        let mut operands = Vec::new();
        collect_abs_operands(body, variable, &mut operands);
        if operands.is_empty() {
            return Ok(Vec::new());
        }
        let (ra, rb) = (a.to_rational(), b.to_rational());
        let width = &rb - &ra;
        let tolerance = width.clone().abs() / RBig::from(IBig::from(10).pow(digits + 4));
        let mut scope = self.variables.clone();
        let mut sign = |operand: &Node, x: &RBig| -> Option<i32> {
            scope.insert(variable.to_string(), PhysicalQuantity::new(Value::Rat(x.clone()), lower.unit.clone()));
            let d = self.scoped_deeper(&scope).evaluate(operand).ok()?.value.decimal(digits).ok()?;
            Some(if d.is_zero() { 0 } else if d.is_negative() { -1 } else { 1 })
        };
        let mut found: Vec<RBig> = Vec::new();
        for operand in operands {
            let mut previous: Option<(RBig, i32)> = None;
            for i in 0..=GRID {
                let x = &ra + &width * RBig::from_parts(IBig::from(i), UBig::from(GRID as u64));
                let Some(s) = sign(operand, &x) else {
                    previous = None;
                    continue;
                };
                if s == 0 {
                    if i != 0 && i != GRID {
                        found.push(x.clone());
                    }
                } else if let Some((lo, ls)) = &previous {
                    if *ls != 0 && *ls != s {
                        let (mut lo, mut hi) = (lo.clone(), x.clone());
                        while (&hi - &lo).abs() > tolerance {
                            let mid = (&lo + &hi) / RBig::from(2);
                            match sign(operand, &mid) {
                                Some(m) if m == *ls => lo = mid,
                                Some(0) => {
                                    lo = mid.clone();
                                    hi = mid;
                                }
                                Some(_) => hi = mid,
                                None => break,
                            }
                        }
                        found.push((lo + hi) / RBig::from(2));
                    }
                }
                previous = Some((x, s));
            }
        }
        let ascending = ra < rb;
        found.sort();
        if !ascending {
            found.reverse();
        }
        found.dedup();
        found.into_iter().map(|x| Value::Rat(x).decimal(digits + 8)).collect()
    }

    /// Exact, by cofactor expansion. The entries have to agree in dimension; the result carries
    /// that unit to the power of the matrix size.
    fn determinant(&self, operand: &Node) -> Result<PhysicalQuantity> {
        let mut operand = operand;
        while let Node::Group { inner, .. } = operand {
            operand = inner;
        }
        let Node::Matrix { rows, .. } = operand else {
            return Err(QuantusError::other(msg!("core.eval.evaluator.needs_matrix_pmatrix_pmatrix")));
        };
        let size = rows.len();
        if rows.iter().any(|row| row.len() != size) {
            return Err(QuantusError::other(msg!("core.eval.evaluator.determinant_not_square",
                rows[0].len()
            )));
        }
        if size > MAX_DETERMINANT_SIZE {
            return Err(QuantusError::other(msg!("core.eval.evaluator.determinants_limited_matrices")));
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
    /// `given` are the point's other variables (`|_{x=2, z=3}` for `\partial/\partial x`), taken
    /// as fixed while differentiating and put in with the point.
    fn derivative(&self, variable: &str, body: &Node, at: Option<&Node>, given: &[(String, Node)]) -> Result<PhysicalQuantity> {
        let derived = {
            let _unit = angles::DegreeScope::new(self.degrees());
            derivative::differentiate(body, variable)?
        };
        let mut scope = self.variables.clone();
        for (name, value) in given {
            scope.insert(name.clone(), self.evaluate(value)?);
        }
        match at {
            Some(at) => {
                scope.insert(variable.to_string(), self.evaluate(at)?);
            }
            None if !scope.contains_key(variable) => {
                return Err(QuantusError::UnresolvedVariable(msg!("core.eval.evaluator.derivative_needs_point"
                )));
            }
            None => {}
        }
        self.scoped(&scope).evaluate(&derived)
    }

    /// Two-sided and numerical: the body is sampled just either side of the target and the
    /// samples must agree, which also rules out a limit that diverges.
    /// Numerical: the body at points ever closer to the target (from both sides unless `side`
    /// picks one) must agree. At ±∞ the points are ±10^k for large k, i.e. x = 1/t with t → 0⁺.
    fn limit(&self, variable: &str, target: &Node, body: &Node, side: i8) -> Result<PhysicalQuantity> {
        let infinite = infinity_sign(target);
        let target = match infinite {
            Some(_) => PhysicalQuantity::dimensionless(Value::zero()),
            None => self.evaluate(target)?,
        };
        let digits = self.precision;
        let working = 3 * digits + 30;
        let centre = target.value.decimal(working)?.to_rational();
        let step = |exponent: usize| Value::Rat(RBig::from_parts(IBig::ONE, UBig::from(10u8).pow(exponent)));
        let directions: &[i64] = match side {
            1 => &[1],
            -1 => &[-1],
            _ => &[1, -1],
        };

        let mut scope = self.variables.clone();
        let mut unit = None;
        let mut samples = Vec::new();
        for exponent in [digits + 5, digits + 4] {
            for direction in directions {
                let x = match infinite {
                    Some(sign) => Value::Rat(RBig::from(IBig::from(10).pow(exponent) * IBig::from(sign))),
                    None => Value::Rat(centre.clone()).add(&step(exponent).mul(&Value::int(*direction))),
                };
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
            return Err(QuantusError::other(if side == 0 && infinite.is_none() {
                msg!("core.eval.evaluator.limit_sides_disagree")
            } else {
                msg!("core.eval.evaluator.limit_does_not_settle")
            }));
        }
        let mean = samples.iter().fold(RBig::ZERO, |sum, s| sum + s) / RBig::from(samples.len());
        // Only the digits the samples agree on are kept: x ln x at 0 is 0, not -6.8·10^-53.
        let whole_digits = scale.floor().to_string().trim_start_matches('-').len() as i64;
        let quantum_exponent = whole_digits - 1 - digits.saturating_sub(5) as i64;
        let quantum = if quantum_exponent >= 0 {
            RBig::from(IBig::from(10).pow(quantum_exponent as usize))
        } else {
            RBig::from_parts(IBig::ONE, UBig::from(10u8).pow((-quantum_exponent) as usize))
        };
        let half = RBig::from_parts(IBig::ONE, UBig::from(2u8));
        let rounded = RBig::from((&mean / &quantum + half).floor()) * &quantum;
        let value = Value::from_decimal(&Value::Rat(rounded).decimal(digits)?);
        Ok(PhysicalQuantity::new(value, unit.unwrap_or(crate::units::Unit::ONE)))
    }

    fn degree(&self, degree: Option<&Node>) -> Result<u32> {
        let Some(degree) = degree else { return Ok(2) };
        let quantity = self.evaluate(degree)?;
        if !quantity.is_dimensionless() {
            return Err(QuantusError::other(
                msg!("core.eval.evaluator.root_degree_plain"),
            ));
        }
        let value = quantity.dimensionless_value()?;
        match value.to_i64() {
            Some(n) if n > 0 && n <= i64::from(u32::MAX) => Ok(n as u32),
            _ => Err(QuantusError::other(msg!("core.eval.evaluator.root_degree_whole"
            ))),
        }
    }
}

/// `Some(1)` for `\\infty` (also `+\\infty`), `Some(-1)` for `-\\infty`.
fn infinity_sign(node: &Node) -> Option<i64> {
    match node {
        Node::Constant { name, .. } if name == "\\infty" => Some(1),
        Node::Group { inner, .. } => infinity_sign(inner),
        Node::Negation { operand, .. } => infinity_sign(operand).map(|s| -s),
        _ => None,
    }
}

/// The operands of `|…|` in `node` that depend on `variable`.
fn collect_abs_operands<'n>(node: &'n Node, variable: &str, out: &mut Vec<&'n Node>) {
    if let Node::Abs { operand, .. } = node {
        if mentions(operand, variable) {
            out.push(operand);
        }
    }
    for child in node.children() {
        collect_abs_operands(child, variable, out);
    }
}

fn mentions(node: &Node, variable: &str) -> bool {
    matches!(node, Node::Variable { name, index, .. } if index.is_empty() && name == variable)
        || node.children().into_iter().any(|c| mentions(c, variable))
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
        return Err(QuantusError::other(msg!("core.eval.evaluator.binom_index_limit")));
    }
    let mut result = IBig::ONE;
    for i in 0..k {
        result = result * IBig::from(n - i) / IBig::from(i + 1);
    }
    Ok(result)
}

/// Reserved constant names (F2). They are looked up after the unit table, so a symbol that is
/// a unit stays a unit. The values are symbolic: π stays π until the last numeric evaluation.
/// Physical constants (CODATA 2018), by the name they are written with: `c_0`, `k_B`, `\hbar`;
/// the table is [`crate::constants`]. A session variable of the same name takes precedence.
pub(crate) fn physical_constant(key: &str) -> Option<PhysicalQuantity> {
    crate::constants::find(key)
}

/// The functions of a complex number, which are no `MathFunction`: real part, imaginary part,
/// conjugate and argument.
fn is_complex_function(name: &str) -> bool {
    matches!(name, "Re" | "Im" | "overline" | "arg")
}

fn constant(name: &str) -> Result<Value> {
    match name {
        "\\pi" => Ok(Value::Pi),
        "e" => Ok(Value::E),
        "i" => Ok(Value::I),
        "\\infty" => Err(QuantusError::other(
            msg!("core.eval.evaluator.infinity_not_number"),
        )),
        _ => Err(QuantusError::other(msg!("core.eval.evaluator.unknown_constant"))),
    }
}

/// A division that failed because the divisor is zero points at the divisor and says so.
fn by_zero(error: QuantusError, divisor: &PhysicalQuantity, node: &Node) -> QuantusError {
    if divisor.value.is_zero() {
        QuantusError::other(msg!("core.eval.evaluator.division_zero_here")).at_node(node)
    } else {
        error
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
        assert!(matches!(error.unlocated(), QuantusError::UnresolvedVariable(_)));
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

/// The part of a complex integrand that one pass of a numerical integral takes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Part {
    Real,
    Imaginary,
}
