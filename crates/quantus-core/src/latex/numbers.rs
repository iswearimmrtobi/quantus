//! Numbers in LaTeX, written the way a physics class writes them (F6): `19{,}62`.
//!
//! The comma is typeset as `{,}` so LaTeX keeps it tight instead of adding the spacing it would
//! put after a list separator. Very large and very small magnitudes switch to scientific
//! notation with a proper `\cdot 10^{…}`.

use dashu::base::Abs;
use dashu::integer::IBig;

use crate::num::Decimal;
use crate::settings::NumberFormat;

/// How a decimal result is written, from the settings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Style {
    pub format: NumberFormat,
    /// A plain number gets a decimal prefix: `4{,}7\,\mathrm{k}`.
    pub prefixes: bool,
    /// The digits before the decimal comma are grouped by three with a thin space, from five
    /// digits on: `12\,345{,}6`.
    pub grouping: bool,
}

/// Integer parts this long or longer are grouped.
const GROUPED_FROM: usize = 5;

/// The leading digits of `latex` (after a sign) grouped by three with `\,`.
fn grouped(latex: String) -> String {
    let (sign, rest) = latex.strip_prefix('-').map_or(("", latex.as_str()), |rest| ("-", rest));
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits < GROUPED_FROM {
        return latex;
    }
    let (whole, tail) = rest.split_at(digits);
    let mut out = String::from(sign);
    for (i, digit) in whole.chars().enumerate() {
        if i > 0 && (digits - i) % 3 == 0 {
            out.push_str("\\,");
        }
        out.push(digit);
    }
    out + tail
}

/// The Casio's decimal prefixes, by their power of ten.
const PREFIXES: [(i64, &str); 11] = [
    (-15, "f"),
    (-12, "p"),
    (-9, "n"),
    (-6, "\\mu"),
    (-3, "m"),
    (3, "k"),
    (6, "M"),
    (9, "G"),
    (12, "T"),
    (15, "P"),
    (18, "E"),
];

fn prefix(exponent: i64) -> Option<&'static str> {
    PREFIXES.iter().find(|(e, _)| *e == exponent).map(|(_, symbol)| *symbol)
}

/// `value / 10^exponent`, exactly.
fn shifted(value: &Decimal, exponent: i64) -> Decimal {
    Decimal::new(value.unscaled().clone(), value.scale() + exponent).normalized()
}

/// A plain number with a decimal prefix where one fits (`4700` → `4{,}7\,\mathrm{k}`), the
/// mantissa in the number format; without one, the number as [`format_with`] writes it.
pub fn with_prefix(value: &Decimal, style: &Style) -> String {
    let plain_style = Style { prefixes: false, ..*style };
    let stripped = value.normalized();
    if stripped.is_zero() {
        return format_with(value, &plain_style);
    }
    let exponent = exponent_of(&stripped).div_euclid(3) * 3;
    match prefix(exponent) {
        Some(symbol) => {
            let mantissa = shifted(&stripped, exponent);
            // Fix keeps its places on the mantissa; Sci would undo the prefix.
            let mantissa = match style.format {
                NumberFormat::Sci { .. } => format(&mantissa),
                _ => format_with(&mantissa, &plain_style),
            };
            format!("{mantissa}\\,\\mathrm{{{symbol}}}")
        }
        None => format_with(value, &plain_style),
    }
}

/// Engineering notation: the exponent a multiple of three, moved by three per step (ENG, and
/// ←ENG for negative steps): `1234` is `1{,}234 \cdot 10^{3}`, one step on `0{,}001234 \cdot 10^{6}`.
/// With `prefixes` a prefix stands for the power where one exists.
pub fn engineering(value: &Decimal, steps: i32, prefixes: bool) -> String {
    let stripped = value.normalized();
    if stripped.is_zero() {
        return "0".to_string();
    }
    let exponent = exponent_of(&stripped).div_euclid(3) * 3 + 3 * i64::from(steps);
    let mantissa = plain(&shifted(&stripped, exponent));
    if prefixes {
        if exponent == 0 {
            return mantissa;
        }
        if let Some(symbol) = prefix(exponent) {
            return format!("{mantissa}\\,\\mathrm{{{symbol}}}");
        }
    }
    format!("{mantissa} \\cdot 10^{{{exponent}}}")
}

const DECIMAL_SEPARATOR: &str = "{,}";
/// Magnitudes from 1e15 up and below 1e-4 are written in scientific notation.
const PLAIN_EXPONENTS: std::ops::Range<i64> = -4..15;

pub fn format(value: &Decimal) -> String {
    let stripped = value.normalized();
    if stripped.is_zero() {
        return "0".to_string();
    }
    let exponent = stripped.precision() as i64 - stripped.scale() - 1;
    if !PLAIN_EXPONENTS.contains(&exponent) {
        scientific(&stripped, exponent)
    } else {
        plain(&stripped)
    }
}

/// A decimal result in the user's number format: today's rule (`auto`), the Casio's Norm 1 and
/// Norm 2, a fixed number of decimal places, or scientific notation with a number of significant
/// digits.
pub fn format_with(value: &Decimal, style: &Style) -> String {
    let latex = match style.format {
        NumberFormat::Auto => format(value),
        NumberFormat::Norm1 => normal(value, -2),
        NumberFormat::Norm2 => normal(value, -9),
        NumberFormat::Fix { digits } => fixed(value, i64::from(digits.min(9))),
        NumberFormat::Sci { digits } => sci(value, if digits == 0 { 10 } else { usize::from(digits.min(9)) }),
    };
    if style.grouping { grouped(latex) } else { latex }
}

/// Plain for exponents from `smallest` to 9, scientific outside.
fn normal(value: &Decimal, smallest: i64) -> String {
    let stripped = value.normalized();
    if stripped.is_zero() {
        return "0".to_string();
    }
    let exponent = exponent_of(&stripped);
    if exponent < smallest || exponent >= 10 { scientific(&stripped, exponent) } else { plain(&stripped) }
}

/// Exactly `places` decimal places, rounded half away from zero.
fn fixed(value: &Decimal, places: i64) -> String {
    let rounded = to_places(value, places);
    // A negative value that rounds to zero loses its sign.
    let rounded = if rounded.is_zero() { Decimal::new(IBig::ZERO, places) } else { rounded };
    plain_padded(&rounded, places)
}

/// `digits` significant digits as `m \cdot 10^{e}`, trailing zeros kept.
fn sci(value: &Decimal, digits: usize) -> String {
    if value.is_zero() {
        return format!("{} \\cdot 10^{{0}}", plain_padded(&Decimal::new(IBig::ZERO, digits as i64 - 1), digits as i64 - 1));
    }
    let rounded = value.round(digits).normalized();
    let exponent = exponent_of(&rounded);
    let mantissa = Decimal::new(rounded.unscaled().clone(), rounded.precision() as i64 - 1);
    format!("{} \\cdot 10^{{{exponent}}}", plain_padded(&mantissa, digits as i64 - 1))
}

fn exponent_of(value: &Decimal) -> i64 {
    value.precision() as i64 - value.scale() - 1
}

/// `value` rounded to `places` decimal places, half away from zero, with exactly that scale.
pub(crate) fn to_places(value: &Decimal, places: i64) -> Decimal {
    let scale = value.scale();
    if scale <= places {
        let factor = IBig::from(10).pow((places - scale) as usize);
        return Decimal::new(value.unscaled() * factor, places);
    }
    let divisor = IBig::from(10).pow((scale - places) as usize);
    let magnitude = value.unscaled().clone().abs();
    let (mut quotient, remainder) = (&magnitude / &divisor, &magnitude % &divisor);
    if remainder * IBig::from(2) >= divisor {
        quotient += IBig::ONE;
    }
    Decimal::new(if value.is_negative() { -quotient } else { quotient }, places)
}

/// The value written out with at least `places` decimal places (trailing zeros kept).
fn plain_padded(value: &Decimal, places: i64) -> String {
    let normalized = value.normalized();
    let padded = if normalized.scale() < places { to_places(&normalized, places) } else { normalized };
    plain(&padded)
}

fn plain(value: &Decimal) -> String {
    value.to_plain_string().replace('.', DECIMAL_SEPARATOR)
}

fn scientific(value: &Decimal, exponent: i64) -> String {
    let mantissa = Decimal::new(value.unscaled().clone(), value.precision() as i64 - 1);
    format!("{} \\cdot 10^{{{exponent}}}", plain(&mantissa.normalized()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn latex(text: &str) -> String {
        format(&Decimal::parse(text).unwrap())
    }

    #[test]
    fn plain_numbers_use_the_braced_comma() {
        assert_eq!(latex("19.62"), "19{,}62");
        assert_eq!(latex("20"), "20");
        assert_eq!(latex("0.5"), "0{,}5");
        assert_eq!(latex("-9.81"), "-9{,}81");
    }

    #[test]
    fn trailing_zeros_and_zero_collapse() {
        assert_eq!(latex("2.500"), "2{,}5");
        assert_eq!(latex("100"), "100");
        assert_eq!(latex("0.000"), "0");
    }

    #[test]
    fn the_scientific_thresholds_are_1e15_and_1e_minus_4() {
        assert_eq!(latex("999999999999999"), "999999999999999");
        assert_eq!(latex("1000000000000000"), "1 \\cdot 10^{15}");
        assert_eq!(latex("0.0001"), "0{,}0001");
        assert_eq!(latex("0.00001"), "1 \\cdot 10^{-5}");
    }

    fn styled(text: &str, format: NumberFormat) -> String {
        format_with(&Decimal::parse(text).unwrap(), &Style { format, prefixes: false, grouping: false })
    }

    #[test]
    fn grouping_splits_long_integer_parts_by_three() {
        let grouped = |text: &str, format| format_with(&Decimal::parse(text).unwrap(), &Style { format, prefixes: false, grouping: true });
        assert_eq!(grouped("12345.678", NumberFormat::Auto), "12\\,345{,}678");
        assert_eq!(grouped("-1234567", NumberFormat::Auto), "-1\\,234\\,567");
        assert_eq!(grouped("1234", NumberFormat::Auto), "1234");
        assert_eq!(grouped("100000", NumberFormat::Fix { digits: 1 }), "100\\,000{,}0");
        assert_eq!(grouped("1e20", NumberFormat::Auto), "1 \\cdot 10^{20}");
    }

    #[test]
    fn prefixes_take_the_nearest_power_of_a_thousand_below() {
        let prefixed = |text: &str, format| with_prefix(&Decimal::parse(text).unwrap(), &Style { format, prefixes: true, grouping: false });
        assert_eq!(prefixed("4700", NumberFormat::Auto), "4{,}7\\,\\mathrm{k}");
        assert_eq!(prefixed("0.5", NumberFormat::Auto), "500\\,\\mathrm{m}");
        assert_eq!(prefixed("0.0000022", NumberFormat::Auto), "2{,}2\\,\\mathrm{\\mu}");
        assert_eq!(prefixed("12.5", NumberFormat::Auto), "12{,}5");
        assert_eq!(prefixed("-3300000", NumberFormat::Fix { digits: 2 }), "-3{,}30\\,\\mathrm{M}");
        assert_eq!(prefixed("1e30", NumberFormat::Auto), "1 \\cdot 10^{30}");
    }

    #[test]
    fn engineering_steps_by_three() {
        let eng = |text: &str, steps, prefixes| engineering(&Decimal::parse(text).unwrap(), steps, prefixes);
        assert_eq!(eng("1234", 0, false), "1{,}234 \\cdot 10^{3}");
        assert_eq!(eng("1234", 1, false), "0{,}001234 \\cdot 10^{6}");
        assert_eq!(eng("1234", -1, false), "1234 \\cdot 10^{0}");
        assert_eq!(eng("1234", -2, false), "1234000 \\cdot 10^{-3}");
        assert_eq!(eng("0.00056", 0, false), "560 \\cdot 10^{-6}");
        assert_eq!(eng("0.00056", 0, true), "560\\,\\mathrm{\\mu}");
        assert_eq!(eng("56", 0, true), "56");
    }

    #[test]
    fn norm_1_and_2_switch_at_their_thresholds() {
        assert_eq!(styled("0.01", NumberFormat::Norm1), "0{,}01");
        assert_eq!(styled("0.005", NumberFormat::Norm1), "5 \\cdot 10^{-3}");
        assert_eq!(styled("0.005", NumberFormat::Norm2), "0{,}005");
        assert_eq!(styled("9999999999", NumberFormat::Norm1), "9999999999");
        assert_eq!(styled("10000000000", NumberFormat::Norm2), "1 \\cdot 10^{10}");
    }

    #[test]
    fn fix_keeps_its_decimal_places() {
        assert_eq!(styled("1.5", NumberFormat::Fix { digits: 2 }), "1{,}50");
        assert_eq!(styled("2.345", NumberFormat::Fix { digits: 2 }), "2{,}35");
        assert_eq!(styled("-2.345", NumberFormat::Fix { digits: 0 }), "-2");
        assert_eq!(styled("-0.001", NumberFormat::Fix { digits: 2 }), "0{,}00");
        assert_eq!(styled("123456", NumberFormat::Fix { digits: 1 }), "123456{,}0");
    }

    #[test]
    fn sci_keeps_its_significant_digits() {
        assert_eq!(styled("12345", NumberFormat::Sci { digits: 3 }), "1{,}23 \\cdot 10^{4}");
        assert_eq!(styled("0.5", NumberFormat::Sci { digits: 2 }), "5{,}0 \\cdot 10^{-1}");
        assert_eq!(styled("9.99", NumberFormat::Sci { digits: 2 }), "1{,}0 \\cdot 10^{1}");
        assert_eq!(styled("1", NumberFormat::Sci { digits: 0 }), "1{,}000000000 \\cdot 10^{0}");
        assert_eq!(styled("0", NumberFormat::Sci { digits: 3 }), "0{,}00 \\cdot 10^{0}");
    }

    #[test]
    fn scientific_mantissa_keeps_its_digits_and_sign() {
        assert_eq!(latex("6.674e-11"), "6{,}674 \\cdot 10^{-11}");
        assert_eq!(latex("-1.5e20"), "-1{,}5 \\cdot 10^{20}");
        assert_eq!(latex("12300000000000000000"), "1{,}23 \\cdot 10^{19}");
    }
}
