//! The IEEE-754 view of Basis-N: `f32(x)` and `f64(x)` turn a decimal number into its pattern,
//! and a 32- or 64-bit pattern is broken down into sign, exponent and mantissa, with the float's
//! exact value (every float is a finite decimal) and, for a number typed into `f32(…)`, the
//! rounding error.

use dashu::base::UnsignedAbs;
use dashu::integer::{IBig, UBig};
use dashu::rational::RBig;
use serde::{Deserialize, Serialize};

use crate::text::msg;
use crate::{QuantusError, Result};

/// The two formats.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    F32,
    F64,
}

impl Format {
    /// `f32` or `f64`, in small letters: `F32` is a hexadecimal number.
    pub fn of_name(name: &str) -> Option<Format> {
        match name {
            "f32" => Some(Format::F32),
            "f64" => Some(Format::F64),
            _ => None,
        }
    }

    pub fn of_width(width: u32) -> Option<Format> {
        match width {
            32 => Some(Format::F32),
            64 => Some(Format::F64),
            _ => None,
        }
    }

    pub fn width(self) -> u32 {
        match self {
            Format::F32 => 32,
            Format::F64 => 64,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Format::F32 => "f32",
            Format::F64 => "f64",
        }
    }

    fn exponent_width(self) -> u32 {
        match self {
            Format::F32 => 8,
            Format::F64 => 11,
        }
    }

    fn mantissa_width(self) -> u32 {
        self.width() - 1 - self.exponent_width()
    }

    fn bias(self) -> i32 {
        (1 << (self.exponent_width() - 1)) - 1
    }
}

/// What kind of number a pattern is.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FloatClass {
    Normal,
    Subnormal,
    Zero,
    Infinity,
    Nan,
}

/// A 32- or 64-bit pattern read as a float.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IeeeView {
    /// Whether the sign bit is set.
    pub sign: bool,
    pub exponent_bits: String,
    /// The exponent without the bias (for a subnormal and zero the smallest one, 1 − bias);
    /// none for infinity and NaN.
    pub exponent: Option<i32>,
    pub mantissa_bits: String,
    pub class: FloatClass,
    /// The float's exact value as LaTeX: `0{,}100000001490116119384765625`, `\infty`.
    pub exact: String,
    /// The number typed into `f32(…)`/`f64(…)`, when the result came straight from it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typed: Option<String>,
    /// exact − typed, as LaTeX.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// The decimal number typed as the argument of `f32(…)`, as plain text for `str::parse`:
/// digits, `.` or `{,}` or `,`, a sign, an exponent as `\cdot 10^{n}`, `\times 10^n` or `e`n;
/// also `\infty` and `NaN`.
pub fn plain_number(latex: &str) -> Option<String> {
    let mut text = latex.to_string();
    for (from, to) in [
        ("\\left", ""),
        ("\\right", ""),
        ("\\,", ""),
        ("\\ ", ""),
        ("{,}", "."),
        (",", "."),
        ("\\cdot", "e"),
        ("\\times", "e"),
        ("\\infty", "inf"),
        ("\u{2212}", "-"),
        ("\\mathrm", ""),
        ("\\operatorname", ""),
        ("{", ""),
        ("}", ""),
        (" ", ""),
    ] {
        text = text.replace(from, to);
    }
    // `1.5e10^3` (from `\cdot 10^{3}`) is `1.5e3`.
    if let Some(at) = text.find("e10^") {
        text.replace_range(at..at + 4, "e");
    }
    let text = text.replace('E', "e");
    let plain = !text.is_empty()
        && (text.chars().all(|c| c.is_ascii_digit() || matches!(c, '.' | 'e' | '+' | '-'))
            || ["inf", "-inf", "+inf", "nan", "NaN"].contains(&text.as_str()));
    plain.then_some(text)
}

/// The pattern of the decimal number `text` (from [`plain_number`]) in `format`, rounded to the
/// nearest (ties to even).
pub fn pattern(text: &str, format: Format) -> Result<u64> {
    let bad = || QuantusError::other(msg!("core.basen.not_float", text));
    Ok(match format {
        Format::F32 => u64::from(text.parse::<f32>().map_err(|_| bad())?.to_bits()),
        Format::F64 => text.parse::<f64>().map_err(|_| bad())?.to_bits(),
    })
}

/// The error for a float in a word too narrow for it.
pub fn needs_bits(format: Format) -> QuantusError {
    QuantusError::other(msg!("core.basen.float_needs_bits", format.name(), format.width()))
}

/// The pattern `bits` (its low `format.width()` bits) read as a float; `typed` the number typed
/// into `f32(…)` when it came straight from there. A value longer than `precision` significant
/// digits is cut with `\ldots`.
pub fn view(bits: u64, format: Format, typed: Option<&str>, precision: usize) -> IeeeView {
    let (exponent_width, mantissa_width) = (format.exponent_width(), format.mantissa_width());
    let sign = bits >> (format.width() - 1) & 1 == 1;
    let raw_exponent = (bits >> mantissa_width) & ((1 << exponent_width) - 1);
    let mantissa = bits & ((1 << mantissa_width) - 1);
    let all_ones = (1 << exponent_width) - 1;
    let class = match (raw_exponent, mantissa) {
        (0, 0) => FloatClass::Zero,
        (0, _) => FloatClass::Subnormal,
        (e, 0) if e == all_ones => FloatClass::Infinity,
        (e, _) if e == all_ones => FloatClass::Nan,
        _ => FloatClass::Normal,
    };
    let exponent = match class {
        FloatClass::Normal => Some(raw_exponent as i32 - format.bias()),
        FloatClass::Subnormal | FloatClass::Zero => Some(1 - format.bias()),
        FloatClass::Infinity | FloatClass::Nan => None,
    };
    let minus = if sign { "-" } else { "" };
    let value = exact_value(format, class, raw_exponent, mantissa, sign);
    let exact = match (&value, class) {
        (_, FloatClass::Nan) => "\\mathrm{NaN}".to_string(),
        (_, FloatClass::Infinity) => format!("{minus}\\infty"),
        (_, FloatClass::Zero) => format!("{minus}0"),
        (Some(value), _) => decimal(value, precision),
        (None, _) => String::new(),
    };
    let typed_value = typed.and_then(rational_of);
    let error = match (&value, &typed_value) {
        (Some(value), Some(typed)) => Some(decimal(&(value - typed), precision)),
        _ => None,
    };
    IeeeView {
        sign,
        exponent_bits: format!("{raw_exponent:0exponent_width$b}", exponent_width = exponent_width as usize),
        exponent,
        mantissa_bits: format!("{mantissa:0mantissa_width$b}", mantissa_width = mantissa_width as usize),
        class,
        exact,
        typed: typed.map(decimal_latex),
        error,
    }
}

/// The exact value of a finite float: mantissa · 2^exponent.
fn exact_value(format: Format, class: FloatClass, raw_exponent: u64, mantissa: u64, sign: bool) -> Option<RBig> {
    let mantissa_width = format.mantissa_width() as i32;
    let (significand, exponent) = match class {
        FloatClass::Normal => (mantissa | 1 << mantissa_width, raw_exponent as i32 - format.bias() - mantissa_width),
        FloatClass::Subnormal | FloatClass::Zero => (mantissa, 1 - format.bias() - mantissa_width),
        FloatClass::Infinity | FloatClass::Nan => return None,
    };
    let significand = IBig::from(significand);
    let significand = if sign { -significand } else { significand };
    let power = UBig::from(2u8).pow(exponent.unsigned_abs() as usize);
    Some(if exponent >= 0 {
        RBig::from(significand * IBig::from(power))
    } else {
        RBig::from_parts(significand, power)
    })
}

/// The decimal number `text` (from [`plain_number`]) as an exact rational; none for infinity
/// and NaN.
fn rational_of(text: &str) -> Option<RBig> {
    let (mantissa, exponent) = match text.split_once('e') {
        Some((mantissa, exponent)) => (mantissa, exponent.parse::<i64>().ok()?),
        None => (text, 0),
    };
    let (negative, mantissa) = match mantissa.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, mantissa.strip_prefix('+').unwrap_or(mantissa)),
    };
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits = format!("{whole}{fraction}");
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let digits: IBig = digits.parse().ok()?;
    let digits = if negative { -digits } else { digits };
    let scale = exponent - fraction.len() as i64;
    let ten = UBig::from(10u8).pow(scale.unsigned_abs() as usize);
    Some(if scale >= 0 { RBig::from(digits * IBig::from(ten)) } else { RBig::from_parts(digits, ten) })
}

/// A rational whose denominator has no prime factors but 2 and 5, as its finite decimal in LaTeX
/// (`{,}` as the decimal comma), cut after `precision` significant digits with `\ldots`.
fn decimal(value: &RBig, precision: usize) -> String {
    let (numerator, denominator) = (value.numerator(), value.denominator());
    let (mut twos, mut fives, mut rest) = (0usize, 0usize, denominator.clone());
    let (two, five) = (UBig::from(2u8), UBig::from(5u8));
    while &rest % &two == UBig::ZERO {
        rest /= &two;
        twos += 1;
    }
    while &rest % &five == UBig::ZERO {
        rest /= &five;
        fives += 1;
    }
    debug_assert_eq!(rest, UBig::ONE, "a float is a finite decimal");
    let places = twos.max(fives);
    let scaled = numerator.unsigned_abs() * (UBig::from(10u8).pow(places) / denominator);
    let digits = scaled.to_string();
    let digits = if digits.len() <= places { format!("{}{digits}", "0".repeat(places + 1 - digits.len())) } else { digits };
    let (whole, fraction) = digits.split_at(digits.len() - places);
    let sign = if *numerator < IBig::ZERO { "-" } else { "" };
    // Significant digits: from the first non-zero one.
    let significant = format!("{whole}{fraction}").trim_start_matches('0').len();
    let lead = whole.trim_start_matches('0').len();
    if significant > precision && lead > precision {
        // A whole number longer than the precision: `1{,}7976\ldots \cdot 10^{308}`.
        let kept = whole[1..precision].trim_end_matches('0');
        let kept = if kept.is_empty() { String::new() } else { format!("{{,}}{kept}") };
        return format!("{sign}{}{kept}\\ldots \\cdot 10^{{{}}}", &whole[..1], lead - 1);
    }
    let (fraction, cut) = if significant > precision {
        {
            let leading_zeros = if lead == 0 { fraction.len() - fraction.trim_start_matches('0').len() } else { 0 };
            (fraction[..(precision - lead + leading_zeros).min(fraction.len())].to_string(), true)
        }
    } else {
        (fraction.to_string(), false)
    };
    // Zeros at the end are digits of the value when it is cut.
    let fraction = if cut { fraction.as_str() } else { fraction.trim_end_matches('0') };
    let number = if fraction.is_empty() { whole.to_string() } else { format!("{whole}{{,}}{fraction}") };
    format!("{sign}{number}{}", if cut { "\\ldots" } else { "" })
}

/// The typed number in LaTeX: `0{,}1`, `1 \cdot 10^{39}`.
fn decimal_latex(text: &str) -> String {
    let (mantissa, exponent) = match text.split_once('e') {
        Some((mantissa, exponent)) => (mantissa, Some(exponent.trim_start_matches('+'))),
        None => (text, None),
    };
    let mantissa = match mantissa {
        "inf" | "+inf" => "\\infty".to_string(),
        "-inf" => "-\\infty".to_string(),
        "nan" | "NaN" => "\\mathrm{NaN}".to_string(),
        _ => mantissa.replace('.', "{,}"),
    };
    match exponent {
        Some(exponent) => format!("{mantissa} \\cdot 10^{{{exponent}}}"),
        None => mantissa,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn of(text: &str, format: Format) -> IeeeView {
        let plain = plain_number(text).unwrap();
        view(pattern(&plain, format).unwrap(), format, Some(&plain), 50)
    }

    #[test]
    fn a_tenth_is_not_exact() {
        let tenth = of("0{,}1", Format::F32);
        assert_eq!(pattern("0.1", Format::F32).unwrap(), 0x3DCC_CCCD);
        assert_eq!(tenth.exact, "0{,}100000001490116119384765625");
        assert_eq!(tenth.error.as_deref(), Some("0{,}000000001490116119384765625"));
        assert_eq!((tenth.sign, tenth.exponent, tenth.class), (false, Some(-4), FloatClass::Normal));
        assert_eq!((tenth.exponent_bits.as_str(), tenth.mantissa_bits.as_str()), ("01111011", "10011001100110011001101"));
        let tenth = view(pattern("0.1", Format::F64).unwrap(), Format::F64, None, 60);
        assert_eq!(tenth.exact, "0{,}1000000000000000055511151231257827021181583404541015625");
        // 55 significant digits, cut at the default precision of 50.
        assert!(of("0.1", Format::F64).exact.ends_with("34045410\\ldots"));
        let short = view(pattern("0.1", Format::F64).unwrap(), Format::F64, None, 10);
        assert_eq!(short.exact, "0{,}1000000000\\ldots");
    }

    #[test]
    fn the_special_classes() {
        let zero = of("-0", Format::F32);
        assert_eq!((zero.class, zero.exact.as_str(), zero.sign), (FloatClass::Zero, "-0", true));
        let big = of("1 \\cdot 10^{39}", Format::F32);
        assert_eq!((big.class, big.exact.as_str()), (FloatClass::Infinity, "\\infty"));
        assert_eq!(big.typed.as_deref(), Some("1 \\cdot 10^{39}"));
        assert_eq!(big.error, None);
        let smallest = view(1, Format::F32, None, 50);
        assert_eq!(smallest.class, FloatClass::Subnormal);
        assert_eq!(smallest.exponent, Some(-126));
        assert!(smallest.exact.starts_with("0{,}000000000000000000000000000000000000000000001401298464"), "{}", smallest.exact);
        let nan = view(0x7FC0_0000, Format::F32, None, 50);
        assert_eq!((nan.class, nan.exact.as_str(), nan.exponent), (FloatClass::Nan, "\\mathrm{NaN}", None));
        let one = view(0x3F80_0000, Format::F32, None, 50);
        assert_eq!((one.exact.as_str(), one.exponent), ("1", Some(0)));
        assert_eq!(view(0xC120_0000, Format::F32, None, 50).exact, "-10");
        // A big whole number past the precision.
        assert_eq!(view(f64::MAX.to_bits(), Format::F64, None, 5).exact, "1{,}7976\\ldots \\cdot 10^{308}");
        assert_eq!(view(f32::MAX.to_bits() as u64, Format::F32, None, 50).exact, "340282346638528859811704183484516925440");
    }

    #[test]
    fn typed_numbers_are_read_as_decimals() {
        assert_eq!(plain_number("-1{,}5 \\cdot 10^{-3}").as_deref(), Some("-1.5e-3"));
        assert_eq!(plain_number("2.5e3").as_deref(), Some("2.5e3"));
        assert_eq!(plain_number("\\infty").as_deref(), Some("inf"));
        assert_eq!(plain_number("1+2"), Some("1+2".into()));
        assert!(pattern("1+2", Format::F32).is_err());
        assert_eq!(plain_number("x"), None);
    }
}
