//! The engine behind the UI: `quantus-core` as the UI's `Backend`. The simulator calls it
//! directly; the handheld runs it on a thread of its own, so the screen stays live meanwhile.
//!
//! The engine keeps the sessions. Given a folder (`sessions/` on the SD card), it reads every
//! session file in it at start and writes a session's file again after each change, the same
//! files the desktop keeps in its config directory. `current.txt` there names the session that
//! was open, which opens again on the next start.

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use quantus_core::api::{self, CalcRequest, CalculationRequest, CalculationResponse, NewSessionRequest};
use quantus_core::forms::{FormKind, ResultForm};
use quantus_core::session::SessionStore;
use quantus_core::settings::{self as core, Mode as CoreMode};
use quantus_ui::app::{Mode, Settings};
use quantus_ui::backend::{Backend, Opened, SessionCommand, SessionInfo};
use quantus_ui::settings as ui;
use quantus_ui::text::Lang;

/// The file in the sessions folder that names the open session.
pub const CURRENT: &str = "current.txt";

pub struct Engine {
    store: SessionStore,
    /// The id of the open session: calculations go into it.
    current: String,
    /// Where the session files are; `None` keeps them in memory only.
    dir: Option<PathBuf>,
}

/// Sessions in memory only.
impl Default for Engine {
    fn default() -> Self {
        Engine::new(None)
    }
}

impl Engine {
    /// The engine with the sessions in `dir`, on the one open last; the first in the list if
    /// that one is gone, a new one if there is none.
    pub fn new(dir: Option<PathBuf>) -> Self {
        // The handheld's settings until the first calculation brings the user's: German, so the
        // first session is a "Sitzung".
        let mut store = SessionStore::new();
        store.settings = core_settings(CoreMode::Physics, &Settings::default());
        if let Some(dir) = &dir {
            load_sessions(dir, &mut store);
        }
        let remembered = dir
            .as_ref()
            .and_then(|dir| fs::read_to_string(dir.join(CURRENT)).ok())
            .map(|id| id.trim().to_string())
            .filter(|id| store.get(id).is_some());
        let first = store.sessions().first().map(|session| session.id.clone());
        let mut engine = Engine { store, current: String::new(), dir };
        engine.current = match remembered.or(first) {
            Some(id) => id,
            None => engine.create(),
        };
        engine.keep_current();
        engine
    }

    /// The id of the open session.
    pub fn current(&self) -> &str {
        &self.current
    }

    /// A new, empty session, "Sitzung 3" with the lowest number free (the handheld has no clock
    /// for the desktop's "Sitzung 2026-11-01 14:30"), saved; returns its id.
    fn create(&mut self) -> String {
        let word = if self.store.settings.language == core::Language::De { "Sitzung" } else { "Session" };
        let names: HashSet<String> = self.store.sessions().into_iter().map(|session| session.name.clone()).collect();
        let name = (1..).map(|n| format!("{word} {n}")).find(|name| !names.contains(name)).expect("numbers are endless");
        let request = NewSessionRequest { name: Some(name), ..Default::default() };
        let id = api::new_session(&mut self.store, &request).id;
        self.save(&id);
        id
    }

    /// Writes the file of session `id`; a failure is reported, the session stays in memory.
    fn save(&self, id: &str) {
        let Some(dir) = &self.dir else { return };
        if let Err(error) = save_session(dir, &self.store, id) {
            eprintln!("the session {id} could not be saved: {error}");
        }
    }

    /// Notes the open session in [`CURRENT`].
    fn keep_current(&self) {
        let Some(dir) = &self.dir else { return };
        if let Err(error) = fs::create_dir_all(dir).and_then(|()| fs::write(dir.join(CURRENT), &self.current)) {
            eprintln!("the open session could not be noted: {error}");
        }
    }

    /// The open session and its history, the results written as `settings` say.
    fn opened(&self, settings: &Settings) -> Opened {
        let session = self.store.get(&self.current).expect("the open session exists");
        Opened {
            info: SessionInfo { id: session.id.clone(), name: session.name.clone() },
            history: session.history().iter().map(|entry| (entry.input.clone(), shown(&entry.response, settings))).collect(),
        }
    }

    /// Evaluates `latex` in the open session; `record` puts it into the history (S⇔D does not).
    fn calculate(&mut self, mode: Mode, settings: &Settings, latex: &str, record: bool) -> Result<String, String> {
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
        let (latex, session_id, precision) = (latex.to_string(), Some(self.current.clone()), Some(settings.digits.into()));
        let response = if record {
            let request = CalculationRequest { latex, session_id, precision, ..Default::default() };
            api::evaluate(&mut self.store, &request)
        } else {
            api::calc(&mut self.store, &CalcRequest { latex, values: Vec::new(), session_id, precision })
        };
        let response = response.map_err(|error| error.to_string())?;
        // A definition and Ans changed, and the history if recorded.
        self.save(&self.current);
        Ok(shown(&response, settings))
    }
}

impl Backend for Engine {
    fn evaluate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> Result<String, String> {
        self.calculate(mode, settings, latex, true)
    }

    fn recalculate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> Option<Result<String, String>> {
        Some(self.calculate(mode, settings, latex, false))
    }

    fn sessions(&mut self) -> Vec<SessionInfo> {
        api::list_sessions(&self.store).into_iter().map(|info| SessionInfo { id: info.id, name: info.name }).collect()
    }

    fn session(&mut self, command: SessionCommand, settings: &Settings) -> Result<Opened, String> {
        // The language names a new session and a copy.
        api::set_settings(&mut self.store, core_settings(CoreMode::Physics, settings)).map_err(|error| error.to_string())?;
        let current = self.current.clone();
        match command {
            SessionCommand::Current => {}
            SessionCommand::New => self.current = self.create(),
            SessionCommand::Open(id) => {
                if self.store.get(&id).is_none() {
                    return Err(format!("There is no session '{id}'."));
                }
                self.current = id;
            }
            SessionCommand::Rename(name) => {
                api::rename_session(&mut self.store, &current, &name).map_err(|error| error.error)?;
                self.save(&current);
            }
            SessionCommand::Duplicate => {
                let copy = api::duplicate_session(&mut self.store, &current, None).map_err(|error| error.error)?;
                self.save(&copy.id);
                self.current = copy.id;
            }
            SessionCommand::Delete => {
                api::delete_session(&mut self.store, &current);
                if let Some(path) = self.dir.as_ref().and_then(|dir| session_path(dir, &current)) {
                    if let Err(error) = fs::remove_file(&path) {
                        eprintln!("{} could not be deleted: {error}", path.display());
                    }
                }
                let next = self.store.sessions().first().map(|session| session.id.clone());
                self.current = match next {
                    Some(id) => id,
                    None => self.create(),
                };
            }
            SessionCommand::ClearHistory => {
                api::clear_history(&mut self.store, Some(&current));
                self.save(&current);
            }
        }
        if self.current != current {
            self.keep_current();
        }
        Ok(self.opened(settings))
    }
}

/// A response as the handheld shows it. The engine writes a rational as a decimal and gives the
/// fraction among its forms; exact, the handheld shows the fraction, as a Casio does.
fn shown(response: &CalculationResponse, settings: &Settings) -> String {
    if !settings.decimal {
        if let Some(fraction) = fraction(&response.forms) {
            return fraction.to_string();
        }
    }
    // A decimal with digits after the comma is rounded; mark it like the stub does.
    if settings.decimal && response.result.contains("{,}") {
        format!(r"\approx {}", response.result)
    } else {
        response.result.clone()
    }
}

/// The file of session `id` in `dir`, or `None` for an id that is no plain file name.
pub fn session_path(dir: &Path, id: &str) -> Option<PathBuf> {
    let plain = !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    plain.then(|| dir.join(format!("{id}.json")))
}

/// Writes session `id` to its file; a session the store does not have is left alone.
fn save_session(dir: &Path, store: &SessionStore, id: &str) -> io::Result<()> {
    let (Some(path), Some(session)) = (session_path(dir, id), store.get(id)) else { return Ok(()) };
    fs::create_dir_all(dir)?;
    fs::write(path, session.to_json())
}

/// Reads every session file in `dir` into the store; a broken one is reported and skipped.
fn load_sessions(dir: &Path, store: &mut SessionStore) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for path in entries.flatten().map(|entry| entry.path()) {
        if path.extension().is_none_or(|extension| !extension.eq_ignore_ascii_case("json")) {
            continue;
        }
        let read = fs::read_to_string(&path).map_err(|error| error.to_string());
        if let Err(error) = read.and_then(|json| api::import_session(store, &json).map_err(|error| error.error)) {
            eprintln!("ignoring {}: {error}", path.display());
        }
    }
}

/// A fraction with more digits than this is shown as a decimal, as on a Casio:
/// `\frac{1}{7}+\frac{1}{123456}` gives `0{,}14…`, not a fraction of twelve digits.
const FRACTION_DIGITS: usize = 10;

/// The fraction among a result's forms, improper or mixed as the setting says (the engine puts
/// that one first), if it is short enough to read.
fn fraction(forms: &[ResultForm]) -> Option<&str> {
    let form = forms.iter().find(|form| matches!(form.kind, FormKind::Fraction | FormKind::MixedFraction))?;
    // The number alone, without the unit after `\,`, whose exponents are digits too.
    let number = form.latex.split("\\,").next().unwrap_or_default();
    let digits = number.chars().filter(char::is_ascii_digit).count();
    (digits <= FRACTION_DIGITS).then_some(form.latex.as_str())
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
