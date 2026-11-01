//! The real engine on the PC: `quantus-core` behind the UI's `Backend`, for testing the screen
//! with real results before the engine runs on the board.

use quantus_core::api::{self, CalculationRequest};
use quantus_core::session::SessionStore;
use quantus_core::settings::Mode as CoreMode;
use quantus_ui::app::{Mode, Settings};
use quantus_ui::backend::Backend;

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
        };
        let core = quantus_core::settings::Settings {
            mode,
            exact_roots: !settings.decimal,
            ..self.store.settings.clone()
        };
        api::set_settings(&mut self.store, core).map_err(|error| error.to_string())?;
        let request = CalculationRequest {
            latex: latex.to_string(),
            session_id: None,
            precision: Some(settings.digits.into()),
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
