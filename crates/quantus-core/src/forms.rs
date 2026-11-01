//! The alternative forms of a result: the same value written another way (°′″ for an angle,
//! and so on). The shell switches between them (S⇔D on the handheld, a switch on the desktop)
//! without asking the engine again.

use dashu::base::UnsignedAbs;
use dashu::integer::{IBig, UBig};
use dashu::rational::RBig;
use serde::{Deserialize, Serialize};

use crate::latex::numbers;
use crate::num::Decimal;

/// What a form shows.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FormKind {
    /// An improper fraction: `\frac{7}{3}`.
    Fraction,
    /// A mixed number: `2\frac{1}{3}`.
    MixedFraction,
    /// A recurring decimal: `2{,}\overline{3}`.
    Recurring,
    /// The decimal number.
    Decimal,
    /// The exact value with roots or π: `2\sqrt{3}`.
    Exact,
    /// The prime factors: `2^{3} \cdot 3 \cdot 5`.
    PrimeFactors,
    /// Degrees, minutes and seconds: `2^{\circ}20'30''`.
    Sexagesimal,
    /// A complex number as `a + bi`.
    Rectangular,
    /// A complex number as `r\angle\theta`.
    Polar,
    /// A Basis-N value in decimal, hexadecimal, binary or octal.
    Dec,
    Hex,
    Bin,
    Oct,
}

/// One form of a result, as LaTeX.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResultForm {
    pub kind: FormKind,
    pub latex: String,
}

impl ResultForm {
    pub fn new(kind: FormKind, latex: impl Into<String>) -> Self {
        ResultForm { kind, latex: latex.into() }
    }
}

/// `\frac{7}{3}`, `-\frac{7}{3}`; `None` for an integer.
pub fn fraction(value: &RBig) -> Option<String> {
    let (numerator, denominator) = (value.numerator(), value.denominator());
    if *denominator == UBig::ONE {
        return None;
    }
    let sign = if *numerator < IBig::ZERO { "-" } else { "" };
    Some(format!("{sign}\\frac{{{}}}{{{denominator}}}", numerator.unsigned_abs()))
}

/// `2\frac{1}{3}`, `-2\frac{1}{3}`; `None` for an integer or a proper fraction.
pub fn mixed_fraction(value: &RBig) -> Option<String> {
    let (numerator, denominator) = (value.numerator(), value.denominator());
    let magnitude = numerator.unsigned_abs();
    let (whole, rest) = (&magnitude / denominator, &magnitude % denominator);
    if whole == UBig::ZERO || rest == UBig::ZERO {
        return None;
    }
    let sign = if *numerator < IBig::ZERO { "-" } else { "" };
    Some(format!("{sign}{whole}\\frac{{{rest}}}{{{denominator}}}"))
}

/// The longest period a recurring decimal is written with.
const MAX_PERIOD: usize = 100;

/// A fraction whose decimal does not end, with its period overlined: `\frac{1}{6}` →
/// `0{,}1\overline{6}`. `None` when the decimal ends or the period is too long.
pub fn recurring(value: &RBig) -> Option<String> {
    let (numerator, denominator) = (value.numerator(), value.denominator());
    let mut rest = denominator.clone();
    for p in [2u8, 5] {
        while &rest % UBig::from(p) == UBig::ZERO {
            rest /= UBig::from(p);
        }
    }
    if rest == UBig::ONE {
        return None;
    }
    let magnitude = numerator.unsigned_abs();
    let whole = &magnitude / denominator;
    let mut remainder = &magnitude % denominator;
    let mut seen: Vec<UBig> = Vec::new();
    let mut digits = String::new();
    // Long division until a remainder repeats: the digits since its first time are the period.
    while !seen.contains(&remainder) {
        if seen.len() > 2 * MAX_PERIOD {
            return None;
        }
        seen.push(remainder.clone());
        remainder *= UBig::from(10u8);
        digits.push_str(&(&remainder / denominator).to_string());
        remainder %= denominator;
    }
    let start = seen.iter().position(|r| *r == remainder).expect("the remainder was seen");
    if digits.len() - start > MAX_PERIOD {
        return None;
    }
    let sign = if *numerator < IBig::ZERO { "-" } else { "" };
    Some(format!("{sign}{whole}{{,}}{}\\overline{{{}}}", &digits[..start], &digits[start..]))
}

/// The largest number factored into primes.
const MAX_FACTORED: u64 = 1_000_000_000_000_000;

/// The prime factors of a whole number above 1: `120` → `2^{3} \cdot 3 \cdot 5`; a fraction
/// factors above and below the line. `None` for 0, ±1 and numbers beyond 10¹⁵.
pub fn prime_factors(value: &RBig) -> Option<String> {
    let (numerator, denominator) = (value.numerator(), value.denominator());
    let top = u64::try_from(numerator.unsigned_abs()).ok().filter(|n| *n <= MAX_FACTORED)?;
    let bottom = u64::try_from(denominator.clone()).ok().filter(|n| *n <= MAX_FACTORED)?;
    if top == 0 || (top == 1 && bottom == 1) {
        return None;
    }
    let sign = if *numerator < IBig::ZERO { "-" } else { "" };
    let written = |n: u64| -> String {
        if n == 1 {
            return "1".to_string();
        }
        factorize(n)
            .into_iter()
            .map(|(p, k)| if k == 1 { p.to_string() } else { format!("{p}^{{{k}}}") })
            .collect::<Vec<_>>()
            .join(" \\cdot ")
    };
    Some(if bottom == 1 {
        format!("{sign}{}", written(top))
    } else {
        format!("{sign}\\frac{{{}}}{{{}}}", written(top), written(bottom))
    })
}

/// Trial division: primes with their exponents, ascending.
fn factorize(mut n: u64) -> Vec<(u64, u32)> {
    let mut factors = Vec::new();
    let mut divisor = 2u64;
    while divisor * divisor <= n {
        let mut exponent = 0;
        while n.is_multiple_of(divisor) {
            n /= divisor;
            exponent += 1;
        }
        if exponent > 0 {
            factors.push((divisor, exponent));
        }
        divisor += if divisor == 2 { 1 } else { 2 };
    }
    if n > 1 {
        factors.push((n, 1));
    }
    factors
}

/// Decimal places the seconds are written to at most.
const SECOND_DECIMALS: i64 = 4;

/// An angle in degrees as degrees, minutes and seconds: `2{,}3416` → `2^{\circ}20'29{,}76''`.
/// Minutes and seconds that are zero are still written, as on a calculator.
pub fn sexagesimal(degrees: &Decimal) -> String {
    let value = degrees.to_rational();
    let negative = value < RBig::ZERO;
    let value = if negative { -value } else { value };
    let scale = IBig::from(10).pow(SECOND_DECIMALS as usize);
    // Everything in units of 10^-4 seconds, rounded once, so the carries are exact.
    let ticks = (value * RBig::from(3600) * RBig::from(scale.clone())).round();
    let per_degree = IBig::from(3600) * &scale;
    let per_minute = IBig::from(60) * &scale;
    let whole = &ticks / &per_degree;
    let rest = &ticks % &per_degree;
    let minutes = &rest / &per_minute;
    let seconds = Decimal::new(&rest % &per_minute, SECOND_DECIMALS).normalized();
    let sign = if negative && ticks != IBig::ZERO { "-" } else { "" };
    format!("{sign}{whole}^{{\\circ}}{minutes}'{}''", numbers::format(&seconds))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dms(text: &str) -> String {
        sexagesimal(&Decimal::parse(text).unwrap())
    }

    fn rational(n: i64, d: u64) -> RBig {
        RBig::from_parts(IBig::from(n), UBig::from(d))
    }

    #[test]
    fn fractions_improper_and_mixed() {
        assert_eq!(fraction(&rational(7, 3)).unwrap(), "\\frac{7}{3}");
        assert_eq!(fraction(&rational(-7, 3)).unwrap(), "-\\frac{7}{3}");
        assert_eq!(fraction(&rational(4, 1)), None);
        assert_eq!(mixed_fraction(&rational(7, 3)).unwrap(), "2\\frac{1}{3}");
        assert_eq!(mixed_fraction(&rational(-7, 3)).unwrap(), "-2\\frac{1}{3}");
        assert_eq!(mixed_fraction(&rational(1, 3)), None);
    }

    #[test]
    fn recurring_decimals_overline_their_period() {
        assert_eq!(recurring(&rational(1, 3)).unwrap(), "0{,}\\overline{3}");
        assert_eq!(recurring(&rational(1, 6)).unwrap(), "0{,}1\\overline{6}");
        assert_eq!(recurring(&rational(-22, 7)).unwrap(), "-3{,}\\overline{142857}");
        assert_eq!(recurring(&rational(1, 12)).unwrap(), "0{,}08\\overline{3}");
        assert_eq!(recurring(&rational(1, 8)), None);
        // 1/197 has a period of 98 digits, 1/199 of 99, 1/211 of 30, 1/223 of 222.
        assert!(recurring(&rational(1, 199)).is_some());
        assert_eq!(recurring(&rational(1, 223)), None);
    }

    #[test]
    fn prime_factors_of_whole_numbers_and_fractions() {
        assert_eq!(prime_factors(&rational(120, 1)).unwrap(), "2^{3} \\cdot 3 \\cdot 5");
        assert_eq!(prime_factors(&rational(97, 1)).unwrap(), "97");
        assert_eq!(prime_factors(&rational(-12, 35)).unwrap(), "-\\frac{2^{2} \\cdot 3}{5 \\cdot 7}");
        assert_eq!(prime_factors(&rational(1, 4)).unwrap(), "\\frac{1}{2^{2}}");
        assert_eq!(prime_factors(&rational(1, 1)), None);
        assert_eq!(prime_factors(&rational(0, 1)), None);
        assert_eq!(prime_factors(&rational(999_999_999_989, 1)).unwrap(), "999999999989");
    }

    #[test]
    fn degrees_split_into_minutes_and_seconds() {
        assert_eq!(dms("2.341666666666666667"), "2^{\\circ}20'30''");
        assert_eq!(dms("30"), "30^{\\circ}0'0''");
        assert_eq!(dms("0.5"), "0^{\\circ}30'0''");
        assert_eq!(dms("-1.25"), "-1^{\\circ}15'0''");
    }

    #[test]
    fn seconds_keep_four_decimals_and_carry() {
        assert_eq!(dms("1.0001"), "1^{\\circ}0'0{,}36''");
        assert_eq!(dms("1.99999999999"), "2^{\\circ}0'0''");
    }
}
