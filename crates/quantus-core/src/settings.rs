//! What the user may change about how inputs are read and results are written. Every default
//! is the engine's built-in behaviour, so a missing or partial settings file changes nothing.

use serde::{Deserialize, Serialize};

use crate::algebra::pipeline::OutputStyle;
use crate::latex::parser::ParseOptions;
use crate::num::{DEFAULT_PRECISION, MAX_PRECISION};
use crate::{QuantusError, Result};

/// Which way the binomial formulas are applied to a polynomial result.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BinomialForm {
    /// Binomial form, unless the input itself was compact (`(a + b)^2`): then the expansion is
    /// what was asked for.
    #[default]
    Auto,
    /// Always the expanded polynomial.
    Expanded,
    /// Always the binomial form where one exists.
    Binomial,
}

/// How the denominator of a rational result is written.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DenominatorForm {
    /// Factored when it splits into several (or repeated) factors, expanded otherwise.
    #[default]
    Auto,
    /// Always expanded.
    Expanded,
    /// Always factored, including a constant content: `2(x + 2)`.
    Factored,
}

/// What the calculator is used for. It decides which symbols mean something on their own.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Letters are variables: no units, no physical constants, and `i` is a variable.
    Algebra,
    /// Units and physical constants are known; `i` is a variable.
    #[default]
    Physics,
    /// `i` is the imaginary unit and results may be complex.
    Imaginary,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub mode: Mode,
    /// Significant digits of numeric results.
    pub precision: usize,
    pub binomial_form: BinomialForm,
    /// Whether a polynomial without a binomial form is written by completing the square.
    pub complete_square: bool,
    pub denominator_form: DenominatorForm,
    /// `a` and `y` are variables rather than the units annum and year.
    pub a_and_y_as_variables: bool,
    /// A run of letters like `ab` in `2ab` is the product `a \cdot b`.
    pub letter_products: bool,
    /// `t`, `u` and `in` are the tonne, the atomic mass unit and the inch rather than variables.
    pub extra_unit_letters: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            mode: Mode::Physics,
            precision: DEFAULT_PRECISION,
            binomial_form: BinomialForm::Auto,
            complete_square: true,
            denominator_form: DenominatorForm::Auto,
            a_and_y_as_variables: true,
            letter_products: true,
            extra_unit_letters: false,
        }
    }
}

impl Settings {
    pub fn validate(&self) -> Result<()> {
        if !(1..=MAX_PRECISION).contains(&self.precision) {
            return Err(QuantusError::other(format!(
                "The precision must be between 1 and {MAX_PRECISION} digits, but was {}.",
                self.precision
            )));
        }
        Ok(())
    }

    pub fn parse_options(&self) -> ParseOptions {
        ParseOptions {
            a_and_y_as_variables: self.a_and_y_as_variables,
            letter_products: self.letter_products,
            extra_unit_letters: self.extra_unit_letters,
            units: self.mode != Mode::Algebra,
            imaginary_i: self.mode == Mode::Imaginary,
        }
    }

    pub fn output_style(&self) -> OutputStyle {
        OutputStyle {
            binomial: self.binomial_form,
            complete_square: self.complete_square,
            denominator: self.denominator_form,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_partial_file_keeps_the_other_defaults() {
        let settings: Settings = serde_json::from_value(json!({ "binomialForm": "expanded" })).unwrap();
        assert_eq!(settings, Settings { binomial_form: BinomialForm::Expanded, ..Settings::default() });
    }

    #[test]
    fn serializes_in_camel_case_with_lowercase_choices() {
        let value = serde_json::to_value(Settings::default()).unwrap();
        assert_eq!(
            value,
            json!({
                "mode": "physics",
                "precision": 50,
                "binomialForm": "auto",
                "completeSquare": true,
                "denominatorForm": "auto",
                "aAndYAsVariables": true,
                "letterProducts": true,
                "extraUnitLetters": false
            })
        );
    }

    #[test]
    fn the_mode_decides_units_and_the_imaginary_unit() {
        let options = |mode| Settings { mode, ..Settings::default() }.parse_options();
        assert_eq!((options(Mode::Algebra).units, options(Mode::Algebra).imaginary_i), (false, false));
        assert_eq!((options(Mode::Physics).units, options(Mode::Physics).imaginary_i), (true, false));
        assert_eq!((options(Mode::Imaginary).units, options(Mode::Imaginary).imaginary_i), (true, true));
    }

    #[test]
    fn precision_is_range_checked() {
        assert!(Settings::default().validate().is_ok());
        for bad in [0, MAX_PRECISION + 1] {
            let error = Settings { precision: bad, ..Settings::default() }.validate().unwrap_err();
            assert!(error.to_string().contains("between 1 and"), "{bad}");
        }
    }
}
