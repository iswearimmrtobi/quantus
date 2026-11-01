//! The alternative forms of a result: the same value written another way (°′″ for an angle,
//! and so on). The shell switches between them (S⇔D on the handheld, a switch on the desktop)
//! without asking the engine again.

use dashu::integer::IBig;
use dashu::rational::RBig;
use serde::{Deserialize, Serialize};

use crate::latex::numbers;
use crate::num::Decimal;

/// What a form shows.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FormKind {
    /// Degrees, minutes and seconds: `2^{\circ}20'30''`.
    Sexagesimal,
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
