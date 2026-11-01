//! The real engine on the PC: `quantus-core` behind the UI's `Backend`, for testing the screen
//! with real results before the engine runs on the board.

use quantus_core::api::{self, CalculationRequest};
use quantus_core::session::SessionStore;
use quantus_core::settings::{self as core, Mode as CoreMode};
use quantus_ui::app::{Mode, Settings};
use quantus_ui::backend::Backend;
use quantus_ui::settings as ui;
use quantus_ui::text::Lang;

#[derive(Default)]
pub struct Engine {
    store: SessionStore,
}

impl Backend for Engine {
    fn evaluate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> Result<String, String> {
        // Analysis and Linear have their own calls (curve sketch, systems) and no screen for them
        // yet; here they evaluate plain inputs.
        let mode = match mode {
            Mode::Algebra => CoreMode::Algebra,
            Mode::Physics => CoreMode::Physics,
            Mode::Imaginary => CoreMode::Imaginary,
            Mode::Analysis => CoreMode::Analysis,
            Mode::Linear => CoreMode::Linear,
            // The app does not ask for these (`Mode::engine`).
            _ => return Err("Noch nicht verfügbar".to_string()),
        };
        let core = core_settings(mode, settings);
        api::set_settings(&mut self.store, core).map_err(|error| error.to_string())?;
        let request = CalculationRequest {
            latex: latex.to_string(),
            session_id: None,
            precision: Some(settings.digits.into()),
            ..Default::default()
        };
        let response = api::evaluate(&mut self.store, &request).map_err(|error| error.to_string())?;
        // A decimal with digits after the comma is rounded; mark it like the stub does.
        if settings.decimal && response.result.contains("{,}") {
            Ok(format!(r"\approx {}", response.result))
        } else {
            Ok(response.result)
        }
    }
}

/// The engine's settings for the hui's: every setting the engine has. The others (the device,
/// the connections, and what the engine does not have yet) wait for the engine or the firmware.
fn core_settings(mode: CoreMode, settings: &Settings) -> core::Settings {
    core::Settings {
        mode,
        precision: settings.digits.into(),
        binomial_form: match settings.binomial_form {
            ui::BinomialForm::Auto => core::BinomialForm::Auto,
            ui::BinomialForm::Expanded => core::BinomialForm::Expanded,
            ui::BinomialForm::Binomial => core::BinomialForm::Binomial,
        },
        complete_square: settings.complete_square,
        denominator_form: match settings.denominator_form {
            ui::DenominatorForm::Auto => core::DenominatorForm::Auto,
            ui::DenominatorForm::Expanded => core::DenominatorForm::Expanded,
            ui::DenominatorForm::Factored => core::DenominatorForm::Factored,
        },
        conditional_identities: match settings.conditional_identities {
            ui::ConditionalIdentities::WithCondition => core::ConditionalIdentities::WithCondition,
            ui::ConditionalIdentities::AlwaysValid => core::ConditionalIdentities::AlwaysValid,
        },
        exact_roots: !settings.decimal,
        a_and_y_as_variables: settings.a_and_y_as_variables,
        letter_products: settings.letter_products,
        extra_unit_letters: settings.extra_unit_letters,
        language: match settings.language {
            Lang::De => core::Language::De,
            Lang::En => core::Language::En,
        },
        quadratic_formula: match settings.quadratic_formula {
            ui::QuadraticFormula::Auto => core::QuadraticFormula::Auto,
            ui::QuadraticFormula::Pq => core::QuadraticFormula::Pq,
            ui::QuadraticFormula::Abc => core::QuadraticFormula::Abc,
        },
        step_detail: match settings.step_detail {
            ui::StepDetail::None => core::StepDetail::None,
            ui::StepDetail::Brief => core::StepDetail::Brief,
            ui::StepDetail::Standard => core::StepDetail::Standard,
            ui::StepDetail::Detailed => core::StepDetail::Detailed,
            ui::StepDetail::Maximal => core::StepDetail::Maximal,
        },
        cancel_marks: settings.cancel_marks,
        elimination: match settings.elimination {
            ui::Elimination::Gauss => core::Elimination::Gauss,
            ui::Elimination::GaussJordan => core::Elimination::GaussJordan,
        },
        unknown_names: match settings.unknown_names {
            ui::UnknownNames::Indexed => core::UnknownNames::Indexed,
            ui::UnknownNames::Letters => core::UnknownNames::Letters,
        },
        linear_complex: settings.linear_complex,
        angle_unit: match settings.angle {
            ui::Angle::Degrees => core::AngleUnit::Degree,
            ui::Angle::Radians => core::AngleUnit::Radian,
        },
        number_format: match settings.format {
            ui::NumberFormat::Norm1 => core::NumberFormat::Norm1,
            ui::NumberFormat::Norm2 => core::NumberFormat::Norm2,
            ui::NumberFormat::Fix => core::NumberFormat::Fix { digits: settings.format_digits },
            ui::NumberFormat::Sci => core::NumberFormat::Sci { digits: settings.format_digits },
        },
        decimal_prefixes: settings.prefixes,
        digit_grouping: settings.grouping,
        fraction_form: match settings.fraction {
            ui::FractionForm::Improper => core::FractionForm::Improper,
            ui::FractionForm::Mixed => core::FractionForm::Mixed,
        },
        recurring: settings.recurring,
        complex_form: match settings.complex {
            ui::ComplexForm::Cartesian => core::ComplexForm::Rectangular,
            ui::ComplexForm::Polar => core::ComplexForm::Polar,
        },
        // The hui's Basis-N has no screen yet to choose the base.
        number_base: core::NumberBase::Dec,
    }
}
