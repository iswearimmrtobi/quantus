//! What the user may change about how inputs are read and results are written. Every default
//! is the engine's built-in behaviour, so a missing or partial settings file changes nothing.

use serde::{Deserialize, Serialize};

use crate::algebra::pipeline::OutputStyle;
use crate::latex::parser::ParseOptions;
use crate::num::{DEFAULT_PRECISION, MAX_PRECISION};
use crate::{QuantusError, Result};

/// Which way the binomial formulas are applied to a polynomial result.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
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
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
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

/// What the algebra does with an identity that only holds under a condition, such as
/// `\sqrt{a} \sqrt{b} = \sqrt{ab}` for `a, b \geq 0`.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum ConditionalIdentities {
    /// Apply it and add the condition to the domain of the result.
    #[default]
    WithCondition,
    /// Apply it only where the condition holds anyway (`\sqrt{2} \sqrt{3} = \sqrt{6}`).
    AlwaysValid,
}

/// Which formula solves a quadratic equation in the analysis mode's working.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum QuadraticFormula {
    /// The pq formula in German, the quadratic (abc) formula in English.
    #[default]
    Auto,
    /// `x_{1,2} = -\frac{p}{2} \pm \sqrt{(\frac{p}{2})^2 - q}` after dividing by a.
    Pq,
    /// `x_{1,2} = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}`.
    Abc,
}

/// How much working the step list shows. Each level contains every step of the levels before it.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "lowercase")]
pub enum StepDetail {
    /// Only the result; the engine still computes the steps at `Brief`, the list is hidden.
    None,
    /// One step per rule.
    Brief,
    /// What a student writes in their notebook: the line that shows the idea of the rule.
    #[default]
    Standard,
    /// Every intermediate expression of a rule: the formula with the values put in, the factor
    /// written out before it is cancelled.
    Detailed,
    /// Also the bookkeeping: signs, the order of terms, the domain notes of cancelled factors.
    Maximal,
}

impl StepDetail {
    /// Whether a step of `level` is shown at this setting. `None` shows what `Brief` shows.
    pub fn shows(self, level: StepDetail) -> bool {
        level <= self.max(StepDetail::Brief)
    }
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
    /// Inputs are read as in `Algebra`, but `i` is the imaginary unit and results may be complex.
    Imaginary,
    /// The circuit calculator. Inputs are read as in `Physics`.
    Electronics,
    /// The curve sketch of a function of one variable. Inputs are read as in `Algebra`.
    Analysis,
}

/// The language of the texts the engine writes itself (the electronics notes and errors).
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    En,
    De,
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
    pub conditional_identities: ConditionalIdentities,
    /// Results with roots are written as `\sqrt{2}` rather than as a decimal.
    pub exact_roots: bool,
    /// `a` and `y` are variables rather than the units annum and year.
    pub a_and_y_as_variables: bool,
    /// A run of letters like `ab` in `2ab` is the product `a \cdot b`.
    pub letter_products: bool,
    /// `t`, `u` and `in` are the tonne, the atomic mass unit and the inch rather than variables.
    pub extra_unit_letters: bool,
    pub language: Language,
    pub quadratic_formula: QuadraticFormula,
    pub step_detail: StepDetail,
    /// Whether a cancelled factor is struck through (`\cancel`) rather than coloured.
    pub cancel_marks: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            mode: Mode::Physics,
            precision: DEFAULT_PRECISION,
            binomial_form: BinomialForm::Auto,
            complete_square: true,
            denominator_form: DenominatorForm::Auto,
            conditional_identities: ConditionalIdentities::WithCondition,
            exact_roots: false,
            a_and_y_as_variables: true,
            letter_products: true,
            extra_unit_letters: false,
            language: Language::En,
            quadratic_formula: QuadraticFormula::Auto,
            step_detail: StepDetail::Standard,
            cancel_marks: true,
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
            units: !matches!(self.mode, Mode::Algebra | Mode::Imaginary | Mode::Analysis),
            imaginary_i: self.mode == Mode::Imaginary,
        }
    }

    /// Whether quadratics are solved with the pq formula rather than the abc formula.
    pub fn uses_pq_formula(&self) -> bool {
        match self.quadratic_formula {
            QuadraticFormula::Auto => self.language == Language::De,
            QuadraticFormula::Pq => true,
            QuadraticFormula::Abc => false,
        }
    }

    pub fn output_style(&self) -> OutputStyle {
        OutputStyle {
            binomial: self.binomial_form,
            complete_square: self.complete_square,
            denominator: self.denominator_form,
            conditions: self.conditional_identities,
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
                "conditionalIdentities": "withCondition",
                "exactRoots": false,
                "aAndYAsVariables": true,
                "letterProducts": true,
                "extraUnitLetters": false,
                "language": "en",
                "quadraticFormula": "auto",
                "stepDetail": "standard",
                "cancelMarks": true
            })
        );
    }

    #[test]
    fn the_mode_decides_units_and_the_imaginary_unit() {
        let options = |mode| Settings { mode, ..Settings::default() }.parse_options();
        assert_eq!((options(Mode::Algebra).units, options(Mode::Algebra).imaginary_i), (false, false));
        assert_eq!((options(Mode::Physics).units, options(Mode::Physics).imaginary_i), (true, false));
        assert_eq!((options(Mode::Imaginary).units, options(Mode::Imaginary).imaginary_i), (false, true));
        assert_eq!((options(Mode::Electronics).units, options(Mode::Electronics).imaginary_i), (true, false));
        assert_eq!((options(Mode::Analysis).units, options(Mode::Analysis).imaginary_i), (false, false));
        assert_eq!(serde_json::to_value(Mode::Analysis).unwrap(), json!("analysis"));
    }

    #[test]
    fn the_quadratic_formula_follows_the_language_unless_chosen() {
        let with = |quadratic_formula, language| Settings { quadratic_formula, language, ..Settings::default() }.uses_pq_formula();
        assert!(with(QuadraticFormula::Auto, Language::De) && !with(QuadraticFormula::Auto, Language::En));
        assert!(with(QuadraticFormula::Pq, Language::En) && !with(QuadraticFormula::Abc, Language::De));
        let parsed: Settings = serde_json::from_value(json!({ "quadraticFormula": "pq" })).unwrap();
        assert_eq!(parsed.quadratic_formula, QuadraticFormula::Pq);
    }

    #[test]
    fn conditional_identities_reach_the_output_style() {
        let parsed: Settings = serde_json::from_value(json!({ "conditionalIdentities": "alwaysValid" })).unwrap();
        assert_eq!(parsed.conditional_identities, ConditionalIdentities::AlwaysValid);
        assert_eq!(parsed.output_style().conditions, ConditionalIdentities::AlwaysValid);
        assert_eq!(Settings::default().output_style().conditions, ConditionalIdentities::WithCondition);
    }

    #[test]
    fn each_detail_level_shows_the_levels_before_it() {
        use StepDetail::*;
        assert!(Standard.shows(Brief) && Standard.shows(Standard) && !Standard.shows(Detailed));
        assert!(None.shows(Brief) && !None.shows(Standard));
        assert!(Maximal.shows(Maximal));
        let parsed: Settings = serde_json::from_value(json!({ "stepDetail": "maximal", "cancelMarks": false })).unwrap();
        assert_eq!((parsed.step_detail, parsed.cancel_marks), (Maximal, false));
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
