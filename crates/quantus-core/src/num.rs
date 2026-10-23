//! Exact arithmetic and the decimal output boundary — what Symja did on the JVM side.
//!
//! [`Value`] keeps numbers exact: literals enter as rationals, π and e stay symbols, roots
//! that are not whole stay roots. Only [`Value::decimal`] turns a value into digits, with the
//! precision the caller asks for (F3).
//!
//! Rounding mirrors the Kotlin engine on purpose: evaluate with a few guard digits (the digits
//! are *truncated*, as Apfloat did), then round half-up to the requested precision.

use std::fmt;

use astro_float::{BigFloat, Consts, Radix, RoundingMode};
use dashu::base::{Abs, Sign, UnsignedAbs};
use dashu::integer::{IBig, UBig};
use dashu::rational::RBig;

use crate::error::{QuantusError, Result};

/// Default digit count per F3.
pub const DEFAULT_PRECISION: usize = 50;
pub const MAX_PRECISION: usize = 10_000;
/// Extra digits computed so the last reported digit is rounded, not truncated.
const GUARD_DIGITS: usize = 5;
/// Largest exponent for an exact integer power; beyond this the result is not worth computing.
const MAX_EXACT_EXPONENT: u32 = 100_000;

// ---------------------------------------------------------------------------------------
// Decimal
// ---------------------------------------------------------------------------------------

/// An arbitrary-precision decimal: `unscaled * 10^(-scale)`, the same model as Java's
/// `BigDecimal`. Trailing zeros are not significant here; [`Decimal::normalized`] strips them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decimal {
    unscaled: IBig,
    scale: i64,
}

impl Decimal {
    pub fn new(unscaled: IBig, scale: i64) -> Self {
        Decimal { unscaled, scale }
    }

    pub fn from_i64(value: i64) -> Self {
        Decimal::new(IBig::from(value), 0)
    }

    /// Parses `[-]digits[.digits][e[+-]digits]`. A decimal comma is *not* accepted here — the
    /// lexer normalises that to a point.
    pub fn parse(text: &str) -> Option<Decimal> {
        let text = text.trim();
        let (mantissa, exponent) = match text.find(['e', 'E']) {
            Some(i) => (&text[..i], text[i + 1..].parse::<i64>().ok()?),
            None => (text, 0),
        };
        let (negative, mantissa) = match mantissa.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, mantissa.strip_prefix('+').unwrap_or(mantissa)),
        };
        let (int_part, frac_part) = mantissa.split_once('.').unwrap_or((mantissa, ""));
        if int_part.is_empty() && frac_part.is_empty() {
            return None;
        }
        if !int_part.chars().chain(frac_part.chars()).all(|c| c.is_ascii_digit()) {
            return None;
        }
        let digits = format!("{int_part}{frac_part}");
        let mut unscaled: IBig = digits.parse().ok()?;
        if negative {
            unscaled = -unscaled;
        }
        Some(Decimal::new(unscaled, frac_part.len() as i64 - exponent))
    }

    pub fn unscaled(&self) -> &IBig {
        &self.unscaled
    }

    pub fn scale(&self) -> i64 {
        self.scale
    }

    pub fn is_zero(&self) -> bool {
        self.unscaled == IBig::ZERO
    }

    pub fn is_negative(&self) -> bool {
        self.unscaled.sign() == Sign::Negative
    }

    /// Number of digits of the unscaled value (`BigDecimal.precision()`).
    pub fn precision(&self) -> usize {
        digit_count(&self.unscaled)
    }

    /// The same value without trailing zeros in the unscaled part (`stripTrailingZeros`).
    pub fn normalized(&self) -> Decimal {
        if self.is_zero() {
            return Decimal::new(IBig::ZERO, 0);
        }
        let ten = IBig::from(10);
        let mut unscaled = self.unscaled.clone();
        let mut scale = self.scale;
        loop {
            let (quotient, remainder) = (&unscaled / &ten, &unscaled % &ten);
            if remainder != IBig::ZERO {
                break;
            }
            unscaled = quotient;
            scale -= 1;
        }
        Decimal::new(unscaled, scale)
    }

    /// Rounds to `precision` significant digits, half away from zero (`RoundingMode.HALF_UP`).
    pub fn round(&self, precision: usize) -> Decimal {
        let digits = self.precision();
        if self.is_zero() || digits <= precision {
            return self.clone();
        }
        let drop = digits - precision;
        let divisor = pow10(drop);
        let negative = self.is_negative();
        let magnitude: IBig = self.unscaled.clone().abs();
        let (mut quotient, remainder) = (&magnitude / &divisor, &magnitude % &divisor);
        if &remainder * IBig::from(2) >= divisor {
            quotient += IBig::ONE;
        }
        if negative {
            quotient = -quotient;
        }
        // Rounding up can carry into a new digit (999.9 -> 1000); that is still one more
        // digit than `precision` only as a trailing zero, which `normalized` drops later.
        Decimal::new(quotient, self.scale - drop as i64)
    }

    /// The exact rational value.
    pub fn to_rational(&self) -> RBig {
        if self.scale >= 0 {
            RBig::from_parts(self.unscaled.clone(), unsigned(pow10(self.scale as usize)))
        } else {
            RBig::from(&self.unscaled * pow10((-self.scale) as usize))
        }
    }

    /// Plain notation, no exponent: `20`, `0.5`, `-1234.5`.
    pub fn to_plain_string(&self) -> String {
        let negative = self.is_negative();
        let digits = self.unscaled.clone().abs().to_string();
        let body = if self.scale <= 0 {
            let mut s = digits;
            if s != "0" {
                s.push_str(&"0".repeat((-self.scale) as usize));
            }
            s
        } else {
            let scale = self.scale as usize;
            let padded = if digits.len() <= scale {
                format!("{}{}", "0".repeat(scale - digits.len() + 1), digits)
            } else {
                digits
            };
            let (int_part, frac_part) = padded.split_at(padded.len() - scale);
            format!("{int_part}.{frac_part}")
        };
        if negative {
            format!("-{body}")
        } else {
            body
        }
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_plain_string())
    }
}

fn pow10(exponent: usize) -> IBig {
    IBig::from(10).pow(exponent)
}

fn unsigned(value: IBig) -> UBig {
    value.unsigned_abs()
}

fn digit_count(value: &IBig) -> usize {
    if *value == IBig::ZERO {
        1
    } else {
        value.clone().abs().to_string().len()
    }
}

// ---------------------------------------------------------------------------------------
// Value
// ---------------------------------------------------------------------------------------

/// An exact numeric value. Built through the constructors below, which fold what can be folded
/// exactly (rational arithmetic, whole roots) and leave the rest symbolic.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Rat(RBig),
    Pi,
    E,
    /// The imaginary unit. Complex values exist, but cannot be shown as a decimal.
    I,
    Add(Vec<Value>),
    Mul(Vec<Value>),
    Pow(Box<Value>, Box<Value>),
    /// A transcendental function of a value that did not fold to something exact.
    Func(MathFunction, Box<Value>),
}

/// The functions a [`Value`] can be wrapped in. Trigonometry takes radians.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathFunction {
    Sin,
    Cos,
    Tan,
    /// Natural logarithm.
    Ln,
    /// Base-10 logarithm.
    Log,
    Exp,
    Sinh,
    Cosh,
    Tanh,
    Cot,
    Sec,
    Csc,
    Arcsin,
    Arccos,
    Arctan,
}

impl MathFunction {
    pub fn from_command(name: &str) -> Option<MathFunction> {
        Some(match name {
            "sin" => MathFunction::Sin,
            "cos" => MathFunction::Cos,
            "tan" => MathFunction::Tan,
            "ln" => MathFunction::Ln,
            "log" | "lg" => MathFunction::Log,
            "exp" => MathFunction::Exp,
            "sinh" => MathFunction::Sinh,
            "cosh" => MathFunction::Cosh,
            "tanh" => MathFunction::Tanh,
            "cot" => MathFunction::Cot,
            "sec" => MathFunction::Sec,
            "csc" => MathFunction::Csc,
            "arcsin" => MathFunction::Arcsin,
            "arccos" => MathFunction::Arccos,
            "arctan" => MathFunction::Arctan,
            _ => return None,
        })
    }

    fn display_name(self) -> &'static str {
        match self {
            MathFunction::Sin => "Sin",
            MathFunction::Cos => "Cos",
            MathFunction::Tan => "Tan",
            MathFunction::Ln => "Ln",
            MathFunction::Log => "Log10",
            MathFunction::Exp => "Exp",
            MathFunction::Sinh => "Sinh",
            MathFunction::Cosh => "Cosh",
            MathFunction::Tanh => "Tanh",
            MathFunction::Cot => "Cot",
            MathFunction::Sec => "Sec",
            MathFunction::Csc => "Csc",
            MathFunction::Arcsin => "ArcSin",
            MathFunction::Arccos => "ArcCos",
            MathFunction::Arctan => "ArcTan",
        }
    }
}

impl Value {
    pub fn zero() -> Value {
        Value::Rat(RBig::ZERO)
    }

    pub fn one() -> Value {
        Value::Rat(RBig::ONE)
    }

    pub fn int(value: i64) -> Value {
        Value::Rat(RBig::from(value))
    }

    pub fn ratio(numerator: i64, denominator: u64) -> Value {
        Value::Rat(RBig::from_parts(IBig::from(numerator), UBig::from(denominator)))
    }

    /// The exact value of a decimal literal: `9.81` is `981/100`, never a double.
    pub fn from_decimal(decimal: &Decimal) -> Value {
        Value::Rat(decimal.to_rational())
    }

    pub fn as_rational(&self) -> Option<&RBig> {
        match self {
            Value::Rat(r) => Some(r),
            _ => None,
        }
    }

    pub fn is_zero(&self) -> bool {
        matches!(self, Value::Rat(r) if *r == RBig::ZERO)
    }

    /// Is the value below zero? Symbolic values are decided on their decimal expansion;
    /// complex ones have no sign.
    pub fn is_negative(&self) -> Result<bool> {
        match self {
            Value::Rat(r) => Ok(r.numerator().sign() == Sign::Negative),
            _ => Ok(self.decimal(20)?.is_negative()),
        }
    }

    pub fn is_one(&self) -> bool {
        matches!(self, Value::Rat(r) if *r == RBig::ONE)
    }

    /// The value as a machine integer, if it is a whole number that fits.
    pub fn to_i64(&self) -> Option<i64> {
        let r = self.as_rational()?;
        if *r.denominator() != UBig::ONE {
            return None;
        }
        i64::try_from(r.numerator().clone()).ok()
    }

    pub fn add(&self, other: &Value) -> Value {
        let mut constant = RBig::ZERO;
        // Like terms are collected: √3/2 - √3/2 is exactly 0, not a rounding residue.
        let mut collected: Vec<(RBig, Value)> = Vec::new();
        let mut push = |t: &Value| {
            let (coefficient, rest) = t.split_coefficient();
            match collected.iter_mut().find(|(_, r)| *r == rest) {
                Some((c, _)) => *c += coefficient,
                None => collected.push((coefficient, rest)),
            }
        };
        for term in [self, other] {
            match term {
                Value::Rat(r) => constant += r,
                Value::Add(inner) => {
                    for t in inner {
                        match t {
                            Value::Rat(r) => constant += r,
                            other => push(other),
                        }
                    }
                }
                other => push(other),
            }
        }
        let mut terms: Vec<Value> = collected
            .into_iter()
            .filter(|(c, _)| *c != RBig::ZERO)
            .map(|(c, rest)| Value::Rat(c).mul(&rest))
            .collect();
        if constant != RBig::ZERO || terms.is_empty() {
            terms.insert(0, Value::Rat(constant));
        }
        if terms.len() == 1 {
            terms.remove(0)
        } else {
            Value::Add(terms)
        }
    }

    /// `3·√2·π` as (3, √2·π); a term without a rational factor has coefficient 1.
    fn split_coefficient(&self) -> (RBig, Value) {
        match self {
            Value::Mul(factors) => match factors.as_slice() {
                [Value::Rat(c), rest @ ..] => {
                    let rest = if rest.len() == 1 { rest[0].clone() } else { Value::Mul(rest.to_vec()) };
                    (c.clone(), rest)
                }
                _ => (RBig::ONE, self.clone()),
            },
            other => (RBig::ONE, other.clone()),
        }
    }

    pub fn neg(&self) -> Value {
        Value::int(-1).mul(self)
    }

    pub fn sub(&self, other: &Value) -> Value {
        self.add(&other.neg())
    }

    pub fn mul(&self, other: &Value) -> Value {
        let mut constant = RBig::ONE;
        // Factors with the same base are merged: √3·√3 is 3, √3/√3 is 1.
        let mut powers: Vec<(Value, Value)> = Vec::new();
        let mut push = |f: &Value| {
            let (base, exponent) = match f {
                Value::Pow(base, exponent) if matches!(**base, Value::Rat(_)) => ((**base).clone(), (**exponent).clone()),
                other => (other.clone(), Value::one()),
            };
            match powers.iter_mut().find(|(b, _)| *b == base) {
                Some((_, e)) => *e = e.add(&exponent),
                None => powers.push((base, exponent)),
            }
        };
        for factor in [self, other] {
            match factor {
                Value::Rat(r) => constant *= r,
                Value::Mul(inner) => {
                    for f in inner {
                        match f {
                            Value::Rat(r) => constant *= r,
                            other => push(other),
                        }
                    }
                }
                other => push(other),
            }
        }
        if constant == RBig::ZERO {
            return Value::zero();
        }
        let mut factors: Vec<Value> = Vec::new();
        for (base, exponent) in powers {
            let merged = if exponent.is_one() {
                base
            } else {
                match base.pow(&exponent) {
                    Ok(v) => v,
                    Err(_) => Value::Pow(Box::new(base), Box::new(exponent)),
                }
            };
            match merged {
                Value::Rat(r) => constant *= r,
                other => factors.push(other),
            }
        }
        if constant == RBig::ZERO {
            return Value::zero();
        }
        // A canonical order, so equal products compare equal when terms are collected.
        factors.sort_by_cached_key(|f| f.to_string());
        if constant != RBig::ONE || factors.is_empty() {
            factors.insert(0, Value::Rat(constant));
        }
        if factors.len() == 1 {
            factors.remove(0)
        } else {
            Value::Mul(factors)
        }
    }

    pub fn div(&self, other: &Value) -> Result<Value> {
        if other.is_zero() {
            return Err(QuantusError::other("Division by zero."));
        }
        Ok(self.mul(&other.pow(&Value::int(-1))?))
    }

    pub fn pow(&self, exponent: &Value) -> Result<Value> {
        if exponent.is_zero() {
            return if self.is_zero() {
                Err(QuantusError::other("0 to the power of 0 is undefined."))
            } else {
                Ok(Value::one())
            };
        }
        if exponent.is_one() {
            return Ok(self.clone());
        }
        if let (Value::Rat(base), Value::Rat(exp)) = (self, exponent) {
            if let Some(exact) = rational_power(base, exp)? {
                return Ok(Value::Rat(exact));
            }
        }
        Ok(Value::Pow(Box::new(self.clone()), Box::new(exponent.clone())))
    }

    /// `function(self)`: exact where the result is a known closed form (`sin 0`, `cos(\pi/3)`,
    /// `\ln e`, `\log 100`), symbolic otherwise. Arguments outside the domain are an error.
    pub fn apply(&self, function: MathFunction) -> Result<Value> {
        match function {
            MathFunction::Sin | MathFunction::Cos | MathFunction::Tan => {
                if let Some(exact) = self.exact_trig(function)? {
                    return Ok(exact);
                }
            }
            MathFunction::Ln | MathFunction::Log => {
                if self.contains_imaginary() {
                    return Err(self.not_real());
                }
                if self.is_zero() || self.is_negative()? {
                    return Err(QuantusError::other(format!(
                        "The logarithm is only defined for positive numbers, but the argument was '{self}'."
                    )));
                }
                if let Some(exact) = self.exact_log(function) {
                    return Ok(exact);
                }
            }
            // The rest are built from e^x, sine and cosine, so their exact values come for free.
            MathFunction::Exp => {
                if let Value::Func(MathFunction::Ln, inner) = self {
                    return Ok((**inner).clone());
                }
                return Value::E.pow(self);
            }
            MathFunction::Sinh | MathFunction::Cosh | MathFunction::Tanh => {
                let (plus, minus) = (Value::E.pow(self)?, Value::E.pow(&self.neg())?);
                let (sinh, cosh) = (plus.sub(&minus).div(&Value::int(2))?, plus.add(&minus).div(&Value::int(2))?);
                return match function {
                    MathFunction::Sinh => Ok(sinh),
                    MathFunction::Cosh => Ok(cosh),
                    _ => sinh.div(&cosh),
                };
            }
            MathFunction::Cot | MathFunction::Sec | MathFunction::Csc => {
                let (sin, cos) = (self.apply(MathFunction::Sin)?, self.apply(MathFunction::Cos)?);
                let (top, bottom) = match function {
                    MathFunction::Cot => (cos, sin),
                    MathFunction::Sec => (Value::one(), cos),
                    _ => (Value::one(), sin),
                };
                if bottom.is_zero() {
                    return Err(QuantusError::other(format!(
                        "{} is undefined at '{self}'.",
                        function.display_name()
                    )));
                }
                return top.div(&bottom);
            }
            MathFunction::Arcsin | MathFunction::Arccos | MathFunction::Arctan => {
                if self.contains_imaginary() {
                    return Err(self.not_real());
                }
                if let Some(exact) = self.exact_arc(function)? {
                    return Ok(exact);
                }
            }
        }
        Ok(Value::Func(function, Box::new(self.clone())))
    }

    /// arcsin, arccos and arctan at the values where they are a rational multiple of π, by
    /// comparing 60 digits with the known ones; outside [-1, 1] arcsin and arccos are not real.
    fn exact_arc(&self, function: MathFunction) -> Result<Option<Value>> {
        let x = self.decimal(60)?.to_rational();
        if function != MathFunction::Arctan && x.clone().abs() > RBig::ONE {
            return Err(QuantusError::other(format!(
                "{} is only defined between -1 and 1, but the argument was '{self}'.",
                function.display_name()
            )));
        }
        let tolerance = RBig::from_parts(IBig::ONE, UBig::from(10u8).pow(55));
        // (multiple of π, value) for the principal branch.
        let candidates: Vec<(RBig, Value)> = match function {
            MathFunction::Arctan => [(0, 1), (1, 6), (1, 4), (1, 3), (-1, 6), (-1, 4), (-1, 3)]
                .iter()
                .map(|(n, d)| {
                    let r = RBig::from_parts(IBig::from(*n), UBig::from(*d as u64));
                    let angle = Value::Rat(r.clone()).mul(&Value::Pi);
                    (r, angle.apply(MathFunction::Tan).unwrap_or(Value::zero()))
                })
                .collect(),
            _ => [(0, 1), (1, 6), (1, 4), (1, 3), (1, 2), (-1, 6), (-1, 4), (-1, 3), (-1, 2)]
                .iter()
                .map(|(n, d)| {
                    let r = RBig::from_parts(IBig::from(*n), UBig::from(*d as u64));
                    (r.clone(), Value::Rat(r).mul(&Value::Pi).apply(MathFunction::Sin).unwrap_or(Value::zero()))
                })
                .collect(),
        };
        for (multiple, value) in candidates {
            let known = value.decimal(60)?.to_rational();
            if (&known - &x).abs() <= tolerance {
                // arccos x = π/2 - arcsin x
                let multiple = if function == MathFunction::Arccos {
                    RBig::from_parts(IBig::ONE, UBig::from(2u8)) - multiple
                } else {
                    multiple
                };
                return Ok(Some(Value::Rat(multiple).mul(&Value::Pi)));
            }
        }
        Ok(None)
    }

    /// `self` as a rational multiple of π, if it is one (`\pi`, `\frac{\pi}{6}`, `0`).
    fn pi_multiple(&self) -> Option<RBig> {
        match self {
            Value::Rat(r) if *r == RBig::ZERO => Some(RBig::ZERO),
            Value::Pi => Some(RBig::ONE),
            Value::Mul(factors) => match factors.as_slice() {
                [Value::Rat(r), Value::Pi] | [Value::Pi, Value::Rat(r)] => Some(r.clone()),
                _ => None,
            },
            _ => None,
        }
    }

    fn exact_trig(&self, function: MathFunction) -> Result<Option<Value>> {
        let Some(r) = self.pi_multiple() else { return Ok(None) };
        // sin(πr) = cos(π(1/2 - r)).
        let half = RBig::from_parts(IBig::ONE, UBig::from(2u8));
        let cos_of = |r: &RBig| cos_of_pi_multiple(r);
        Ok(match function {
            MathFunction::Cos => cos_of(&r),
            MathFunction::Sin => cos_of(&(half - &r)),
            _ => {
                let (sin, cos) = (cos_of(&(half - &r)), cos_of(&r));
                match (sin, cos) {
                    (_, Some(c)) if c.is_zero() => {
                        return Err(QuantusError::other("The tangent is undefined at odd multiples of \u{3c0}/2."))
                    }
                    (Some(s), Some(c)) => Some(s.div(&c)?),
                    _ => tan_of_quarter_pi_multiple(&r),
                }
            }
        })
    }

    fn exact_log(&self, function: MathFunction) -> Option<Value> {
        match (function, self) {
            (_, Value::Rat(r)) if *r == RBig::ONE => Some(Value::zero()),
            (MathFunction::Ln, Value::E) => Some(Value::one()),
            (MathFunction::Ln, Value::Pow(base, exponent)) if **base == Value::E => Some((**exponent).clone()),
            (MathFunction::Log, Value::Rat(r)) => {
                let ten = RBig::from(10);
                (-50i64..=50).find(|k| *r == ten.pow(*k as isize)).map(Value::int)
            }
            _ => None,
        }
    }

    /// The value as a decimal with `precision` significant digits, correctly rounded.
    pub fn decimal(&self, precision: usize) -> Result<Decimal> {
        if !(1..=MAX_PRECISION).contains(&precision) {
            return Err(QuantusError::other(format!(
                "precision must be between 1 and {MAX_PRECISION}, was: {precision}"
            )));
        }
        let working = precision + GUARD_DIGITS;
        let truncated = match self {
            Value::Rat(r) => rational_digits(r, working),
            _ => self.irrational_digits(working)?,
        };
        Ok(truncated.round(precision).normalized())
    }

    fn irrational_digits(&self, digits: usize) -> Result<Decimal> {
        if self.contains_imaginary() {
            return Err(self.not_real());
        }
        // Binary precision with room to spare: roughly log2(10) bits per digit.
        let bits = ((digits + 10) as f64 * std::f64::consts::LOG2_10).ceil() as usize + 64;
        let mut consts = Consts::new().map_err(|_| QuantusError::other("could not initialise constants"))?;
        let result = self.eval_float(bits, &mut consts)?;
        if result.is_nan() || result.is_inf() {
            return Err(self.not_real());
        }
        if result.is_zero() {
            return Ok(Decimal::new(IBig::ZERO, 0));
        }
        let text = result
            .format(Radix::Dec, RoundingMode::ToEven, &mut consts)
            .map_err(|_| QuantusError::other("could not format a number"))?;
        parse_float_text(&text, digits).ok_or_else(|| QuantusError::other("could not read a computed number"))
    }

    fn eval_float(&self, bits: usize, consts: &mut Consts) -> Result<BigFloat> {
        let rm = RoundingMode::ToEven;
        Ok(match self {
            Value::Rat(r) => {
                let numerator = bigfloat_from_int(r.numerator(), bits, consts);
                if *r.denominator() == UBig::ONE {
                    numerator
                } else {
                    let denominator = bigfloat_from_int(&IBig::from(r.denominator().clone()), bits, consts);
                    numerator.div(&denominator, bits, rm)
                }
            }
            Value::Pi => consts.pi(bits, rm),
            Value::E => consts.e(bits, rm),
            Value::I => return Err(self.not_real()),
            Value::Add(terms) => {
                let mut sum = BigFloat::from_u8(0, bits);
                for t in terms {
                    sum = sum.add(&t.eval_float(bits, consts)?, bits, rm);
                }
                sum
            }
            Value::Mul(factors) => {
                let mut product = BigFloat::from_u8(1, bits);
                for f in factors {
                    product = product.mul(&f.eval_float(bits, consts)?, bits, rm);
                }
                product
            }
            Value::Func(function, argument) => {
                let x = argument.eval_float(bits, consts)?;
                let result = match function {
                    MathFunction::Sin => x.sin(bits, rm, consts),
                    MathFunction::Cos => x.cos(bits, rm, consts),
                    MathFunction::Tan => x.tan(bits, rm, consts),
                    MathFunction::Ln => x.ln(bits, rm, consts),
                    MathFunction::Log => x.log10(bits, rm, consts),
                    MathFunction::Arcsin => x.asin(bits, rm, consts),
                    MathFunction::Arccos => x.acos(bits, rm, consts),
                    MathFunction::Arctan => x.atan(bits, rm, consts),
                    // Built from exp, sin and cos by `apply`, so never wrapped.
                    MathFunction::Exp => x.exp(bits, rm, consts),
                    MathFunction::Sinh => x.sinh(bits, rm, consts),
                    MathFunction::Cosh => x.cosh(bits, rm, consts),
                    MathFunction::Tanh => x.tanh(bits, rm, consts),
                    MathFunction::Cot => x.tan(bits, rm, consts).reciprocal(bits, rm),
                    MathFunction::Sec => x.cos(bits, rm, consts).reciprocal(bits, rm),
                    MathFunction::Csc => x.sin(bits, rm, consts).reciprocal(bits, rm),
                };
                if result.is_nan() || result.is_inf() {
                    return Err(self.not_real());
                }
                result
            }
            Value::Pow(base, exponent) => {
                let b = base.eval_float(bits, consts)?;
                let e = exponent.eval_float(bits, consts)?;
                if !b.is_negative() {
                    // Far outside what is worth computing (e^(-10^80), or e^(-2·10^8), in an improper integral)
                    // `pow` crawls; the result is 0 or too large anyway.
                    if !b.is_zero() {
                        let magnitude = b.ln(bits, rm, consts).mul(&e, bits, rm);
                        let limit = BigFloat::from_i64(1_000_000, bits);
                        if magnitude.cmp(&limit.neg()).is_some_and(|o| o < 0) {
                            return Ok(BigFloat::from_u8(0, bits));
                        }
                        if magnitude.cmp(&limit).is_some_and(|o| o > 0) {
                            return Err(QuantusError::other(format!(
                                "'{self}' is too large to compute."
                            )));
                        }
                    }
                    return Ok(b.pow(&e, bits, rm, consts));
                }
                // A negative base has a real power only for a rational exponent with an odd
                // denominator: (-2)^(1/3) = -(2^(1/3)), (-2)^(2/3) = 2^(2/3).
                let Some(r) = exponent.as_rational() else { return Err(self.not_real()) };
                if *r.denominator() == UBig::ONE {
                    b.pow(&e, bits, rm, consts)
                } else if r.denominator() % UBig::from(2u8) == UBig::ZERO {
                    return Err(self.not_real());
                } else {
                    let magnitude = b.abs().pow(&e, bits, rm, consts);
                    if r.numerator() % IBig::from(2) == IBig::ZERO { magnitude } else { magnitude.neg() }
                }
            }
        })
    }

    fn contains_imaginary(&self) -> bool {
        match self {
            Value::I => true,
            Value::Add(items) | Value::Mul(items) => items.iter().any(Value::contains_imaginary),
            Value::Pow(base, exponent) => base.contains_imaginary() || exponent.contains_imaginary(),
            Value::Func(_, argument) => argument.contains_imaginary(),
            _ => false,
        }
    }

    fn not_real(&self) -> QuantusError {
        QuantusError::NotReal(format!("'{self}' is not a real number and cannot be shown as a decimal."))
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Rat(r) => {
                if *r.denominator() == UBig::ONE {
                    write!(f, "{}", r.numerator())
                } else {
                    write!(f, "{}/{}", r.numerator(), r.denominator())
                }
            }
            Value::Pi => f.write_str("Pi"),
            Value::E => f.write_str("E"),
            Value::I => f.write_str("I"),
            Value::Add(items) => write_joined(f, items, "+"),
            Value::Mul(items) => write_joined(f, items, "*"),
            Value::Func(function, argument) => write!(f, "{}({argument})", function.display_name()),
            Value::Pow(base, exponent) if **exponent == Value::ratio(1, 2) => write!(f, "Sqrt({base})"),
            Value::Pow(base, exponent) => write!(f, "({base})^({exponent})"),
        }
    }
}

fn write_joined(f: &mut fmt::Formatter<'_>, items: &[Value], separator: &str) -> fmt::Result {
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            f.write_str(separator)?;
        }
        write!(f, "{item}")?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------------------

/// `cos(πr)` when it has a closed form: 0, ±1/2, ±1, ±√2/2 or ±√3/2.
fn cos_of_pi_multiple(r: &RBig) -> Option<Value> {
    let two = RBig::from(2);
    // Into [0, 2): r - 2·floor(r/2). Dividing floor(r) by 2 would truncate towards zero and
    // leave negative r unreduced.
    let mut r = r - &two * RBig::from((r / &two).floor());
    if r > RBig::ONE {
        r = two - r; // cos is symmetric around π
    }
    let numerator = i64::try_from(r.numerator().clone()).ok()?;
    let denominator = u64::try_from(r.denominator().clone()).ok()?;
    match (numerator, denominator) {
        (0, _) => Some(Value::one()),
        (1, 1) => Some(Value::int(-1)),
        (1, 2) => Some(Value::zero()),
        (1, 3) => Some(Value::ratio(1, 2)),
        (2, 3) => Some(Value::ratio(-1, 2)),
        (1, 4) => Some(half_root(2, 1)),
        (3, 4) => Some(half_root(2, -1)),
        (1, 6) => Some(half_root(3, 1)),
        (5, 6) => Some(half_root(3, -1)),
        _ => None,
    }
}

/// `sign·√n/2`
fn half_root(n: i64, sign: i64) -> Value {
    Value::ratio(sign, 2).mul(&Value::Pow(Box::new(Value::int(n)), Box::new(Value::ratio(1, 2))))
}

/// `tan(πr)` for the odd quarter turns, where sine and cosine are not rational.
fn tan_of_quarter_pi_multiple(r: &RBig) -> Option<Value> {
    let quarters = r * RBig::from(4);
    if *quarters.denominator() != UBig::ONE {
        return None;
    }
    // tan has period π: 0 at 0, 1 at π/4, -1 at 3π/4.
    match i64::try_from(quarters.numerator().clone()).ok()?.rem_euclid(4) {
        0 | 2 => Some(Value::zero()),
        1 => Some(Value::one()),
        3 => Some(Value::int(-1)),
        _ => None,
    }
}

/// `base^exponent` when it stays rational, `None` when it has to stay symbolic.
fn rational_power(base: &RBig, exponent: &RBig) -> Result<Option<RBig>> {
    let numerator = exponent.numerator();
    let denominator = exponent.denominator();
    if *denominator == UBig::ONE {
        let Ok(n) = i32::try_from(numerator.clone()) else {
            return Ok(None);
        };
        if n.unsigned_abs() > MAX_EXACT_EXPONENT {
            return Ok(None);
        }
        if *base == RBig::ZERO {
            return if n > 0 {
                Ok(Some(RBig::ZERO))
            } else {
                Err(QuantusError::other("Division by zero."))
            };
        }
        // `RBig::pow` takes negative exponents and inverts for us.
        return Ok(Some(base.pow(n as isize)));
    }
    // A whole root: (a/b)^(p/q) with both a and b perfect q-th powers. A negative base has a
    // real root only for odd q: (-27)^(1/3) = -3.
    let (Ok(p), Ok(q)) = (i32::try_from(numerator.clone()), usize::try_from(denominator.clone())) else {
        return Ok(None);
    };
    let negative = base.numerator().sign() == Sign::Negative;
    if negative && q % 2 == 0 {
        return Ok(None);
    }
    if p.unsigned_abs() > MAX_EXACT_EXPONENT {
        return Ok(None);
    }
    let a = base.numerator().clone().unsigned_abs();
    let b = base.denominator().clone();
    let (root_a, root_b) = (a.nth_root(q), b.nth_root(q));
    if root_a.pow(q) != a || root_b.pow(q) != b {
        return Ok(None);
    }
    let root = RBig::from_parts(if negative { -IBig::from(root_a) } else { IBig::from(root_a) }, root_b);
    if root == RBig::ZERO {
        return if p > 0 { Ok(Some(RBig::ZERO)) } else { Err(QuantusError::other("Division by zero.")) };
    }
    Ok(Some(root.pow(p as isize)))
}

/// The first `digits` significant digits of a rational, truncated (not rounded).
fn rational_digits(value: &RBig, digits: usize) -> Decimal {
    if *value == RBig::ZERO {
        return Decimal::new(IBig::ZERO, 0);
    }
    let negative = value.numerator().sign() == Sign::Negative;
    let numerator = value.numerator().clone().abs();
    let denominator = IBig::from(value.denominator().clone());
    // Scale so the integer quotient has at least `digits` digits.
    let estimate = digit_count(&numerator) as i64 - digit_count(&denominator) as i64;
    let scale = (digits as i64 - estimate + 1).max(0);
    let quotient = &numerator * pow10(scale as usize) / &denominator;
    let mut decimal = Decimal::new(quotient, scale);
    let excess = decimal.precision().saturating_sub(digits);
    if excess > 0 {
        let quotient = decimal.unscaled() / pow10(excess);
        decimal = Decimal::new(quotient, decimal.scale - excess as i64);
    }
    if negative {
        decimal = Decimal::new(-decimal.unscaled, decimal.scale);
    }
    decimal
}

fn bigfloat_from_int(value: &IBig, bits: usize, consts: &mut Consts) -> BigFloat {
    BigFloat::parse(&value.to_string(), Radix::Dec, bits, RoundingMode::ToEven, consts)
}

/// Reads astro-float's `d.ddd...e+N` output and keeps at most `digits` significant digits
/// (truncated), as a [`Decimal`].
pub(crate) fn parse_float_text(text: &str, digits: usize) -> Option<Decimal> {
    let (mantissa, exponent) = text.split_once('e')?;
    let exponent: i64 = exponent.parse().ok()?;
    let (negative, mantissa) = match mantissa.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, mantissa),
    };
    let (int_part, frac_part) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let mut all = format!("{int_part}{frac_part}");
    let mut scale = frac_part.len() as i64 - exponent;
    if all.len() > digits {
        scale -= (all.len() - digits) as i64;
        all.truncate(digits);
    }
    let mut unscaled: IBig = all.parse().ok()?;
    if negative {
        unscaled = -unscaled;
    }
    Some(Decimal::new(unscaled, scale))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dec(text: &str) -> Decimal {
        Decimal::parse(text).unwrap()
    }

    fn rat(text: &str) -> Value {
        Value::from_decimal(&dec(text))
    }

    #[test]
    fn decimal_literals_are_exact() {
        // 9.81 is 981/100, never a double
        assert_eq!(rat("9.81"), Value::ratio(981, 100));
        assert_eq!(rat("0.5").decimal(50).unwrap().to_plain_string(), "0.5");
        assert_eq!(rat("20").decimal(50).unwrap().to_plain_string(), "20");
    }

    #[test]
    fn one_third_has_fifty_digits() {
        let third = Value::int(1).div(&Value::int(3)).unwrap().decimal(50).unwrap();
        assert_eq!(third.precision(), 50);
        assert_eq!(
            third.to_plain_string(),
            "0.33333333333333333333333333333333333333333333333333"
        );
    }

    #[test]
    fn last_digit_is_rounded_not_truncated() {
        // 5/18 = 0.2777...; Apfloat truncated the last digit, the guard digits fix that.
        let value = Value::int(5).div(&Value::int(18)).unwrap().decimal(50).unwrap();
        assert!(value.to_plain_string().ends_with("778"), "{value}");
    }

    #[test]
    fn rounding_respects_requested_precision() {
        let sqrt2 = Value::int(2).pow(&Value::ratio(1, 2)).unwrap().decimal(6).unwrap();
        assert_eq!(sqrt2.to_plain_string(), "1.41421");
        let pi = Value::Pi.decimal(10).unwrap();
        assert_eq!(pi.to_plain_string(), "3.141592654");
    }

    #[test]
    fn whole_roots_stay_exact() {
        assert_eq!(Value::int(4).pow(&Value::ratio(1, 2)).unwrap(), Value::int(2));
        assert_eq!(Value::int(8).pow(&Value::ratio(1, 3)).unwrap(), Value::int(2));
        assert_eq!(
            Value::ratio(1, 4).pow(&Value::ratio(1, 2)).unwrap(),
            Value::ratio(1, 2)
        );
        // not whole: stays a root
        assert!(matches!(
            Value::int(2).pow(&Value::ratio(1, 2)).unwrap(),
            Value::Pow(..)
        ));
    }

    #[test]
    fn rational_arithmetic_is_exact() {
        let sum = Value::ratio(1, 3).add(&Value::ratio(1, 6));
        assert_eq!(sum, Value::ratio(1, 2));
        let product = Value::ratio(2, 3).mul(&Value::int(3));
        assert_eq!(product, Value::int(2));
    }

    #[test]
    fn pi_stays_symbolic_until_the_decimal() {
        let circumference = Value::int(2).mul(&Value::Pi);
        assert!(matches!(circumference, Value::Mul(_)));
        let d = circumference.decimal(5).unwrap();
        assert_eq!(d.to_plain_string(), "6.2832");
    }

    #[test]
    fn large_results_keep_their_digits() {
        let c = Value::int(299_792_458);
        let squared = c.pow(&Value::int(2)).unwrap();
        assert_eq!(squared.decimal(50).unwrap().to_plain_string(), "89875517873681764");
    }

    #[test]
    fn complex_values_are_not_real() {
        let err = Value::I.decimal(10).unwrap_err();
        assert!(matches!(err, QuantusError::NotReal(_)));
        let err = Value::int(-4).pow(&Value::ratio(1, 2)).unwrap().decimal(10).unwrap_err();
        assert!(matches!(err, QuantusError::NotReal(_)));
    }

    #[test]
    fn division_by_zero_is_an_error() {
        assert!(Value::int(1).div(&Value::zero()).is_err());
    }

    #[test]
    fn precision_bounds() {
        assert!(Value::int(1).decimal(0).is_err());
        assert!(Value::int(1).decimal(MAX_PRECISION + 1).is_err());
    }

    #[test]
    fn decimal_parsing_and_printing() {
        assert_eq!(dec("9.81").to_plain_string(), "9.81");
        assert_eq!(dec("-0.05").to_plain_string(), "-0.05");
        assert_eq!(dec("1e3").to_plain_string(), "1000");
        assert_eq!(dec("1.50").normalized().to_plain_string(), "1.5");
        assert!(Decimal::parse("abc").is_none());
    }
}
