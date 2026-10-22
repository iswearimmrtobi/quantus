//! Numbers in LaTeX, written the way a physics class writes them (F6): `19{,}62`.
//!
//! The comma is typeset as `{,}` so LaTeX keeps it tight instead of adding the spacing it would
//! put after a list separator. Very large and very small magnitudes switch to scientific
//! notation with a proper `\cdot 10^{…}`.

use crate::num::Decimal;

const DECIMAL_SEPARATOR: &str = "{,}";
/// Magnitudes from 1e15 up and below 1e-4 are written in scientific notation.
const SCIENTIFIC_UPPER: i64 = 15;
const SCIENTIFIC_LOWER: i64 = -4;

pub fn format(value: &Decimal) -> String {
    let stripped = value.normalized();
    if stripped.is_zero() {
        return "0".to_string();
    }
    let exponent = stripped.precision() as i64 - stripped.scale() - 1;
    if exponent >= SCIENTIFIC_UPPER || exponent < SCIENTIFIC_LOWER {
        scientific(&stripped, exponent)
    } else {
        plain(&stripped)
    }
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

    #[test]
    fn scientific_mantissa_keeps_its_digits_and_sign() {
        assert_eq!(latex("6.674e-11"), "6{,}674 \\cdot 10^{-11}");
        assert_eq!(latex("-1.5e20"), "-1{,}5 \\cdot 10^{20}");
        assert_eq!(latex("12300000000000000000"), "1{,}23 \\cdot 10^{19}");
    }
}
