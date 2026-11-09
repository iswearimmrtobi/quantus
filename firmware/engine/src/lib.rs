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

use quantus_core::api::{self, Assignment, CalcRequest, CalculationRequest, CalculationResponse, EngineeringRequest, NewSessionRequest, SolveRequest};
use quantus_core::forms::{FormKind, ResultForm};
use quantus_core::session::SessionStore;
use quantus_core::settings::{self as core, Mode as CoreMode};
use quantus_ui::app::{Mode, Settings};
use quantus_ui::backend::{
    Backend, Catalogue, Group, Item, Opened, Reply, Request, SessionCommand, SessionInfo, Stored, Units, Variable, VariableCommand,
};
use quantus_ui::clock;
use quantus_ui::settings as ui;
use quantus_ui::text::{self, Lang};

mod analysis;
mod sheet;

/// The file in the sessions folder that names the open session.
pub const CURRENT: &str = "current.txt";

pub struct Engine {
    store: SessionStore,
    /// The id of the open session: calculations go into it.
    current: String,
    /// Where the session files are; `None` keeps them in memory only.
    dir: Option<PathBuf>,
    /// The time zone of the last settings given, for the names of new sessions.
    zone: ui::Zone,
    /// Now, in milliseconds since 1970 (UTC), if the clock is set: [`clock::now`].
    clock: fn() -> Option<i64>,
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
        Engine::with_clock(dir, clock::now)
    }

    /// [`Engine::new`] with another clock than the handheld's, for tests: `|| None` for one not
    /// set.
    pub fn with_clock(dir: Option<PathBuf>, clock: fn() -> Option<i64>) -> Self {
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
        let mut engine = Engine { store, current: String::new(), dir, zone: Settings::default().zone, clock };
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

    /// A new, empty session, saved; returns its id. Named after the local time like the
    /// desktop's ("Sitzung 2026-11-02 14:30") once the clock is set, before that "Sitzung 3"
    /// with the lowest number free.
    fn create(&mut self) -> String {
        let request = match (self.clock)() {
            Some(now) => {
                let utc_offset_minutes = self.zone.offset_minutes(now);
                NewSessionRequest { name: None, now: Some(now), utc_offset_minutes }
            }
            None => {
                let word = quantus_texts::tr!(self.store.settings.language, "core.session.session");
                let names: HashSet<String> = self.store.sessions().into_iter().map(|session| session.name.clone()).collect();
                let name = (1..).map(|n| format!("{word} {n}")).find(|name| !names.contains(name)).expect("numbers are endless");
                NewSessionRequest { name: Some(name), ..Default::default() }
            }
        };
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
            inputs: sheet::kept(&session.mode_inputs),
        }
    }

    /// The engine's settings for `mode` set in the store, for a call on the open session.
    fn prepare(&mut self, mode: Mode, settings: &Settings) -> Result<(), String> {
        let mode = core_mode(mode).ok_or_else(|| text::NOT_AVAILABLE.get(settings.language).to_string())?;
        self.zone = settings.zone;
        api::set_settings(&mut self.store, core_settings(mode, settings)).map(|_| ()).map_err(|error| error.to_string())
    }

    /// What the calculation screen asks of the open session: the letters of an input, CALC,
    /// SOLVE.
    fn formula(&mut self, settings: &Settings, request: &Request) -> Result<Reply, String> {
        let (session_id, precision) = (Some(self.current.clone()), Some(i64::from(settings.digits)));
        match request {
            Request::Variables { mode, latex } => {
                self.prepare(*mode, settings)?;
                let request = CalculationRequest { latex: latex.clone(), session_id, ..Default::default() };
                let variables = api::free_variables(&mut self.store, &request).map_err(|error| error.error)?;
                Ok(Reply::Variables(variables.into_iter().map(|v| Variable { key: v.key, symbol: v.symbol }).collect()))
            }
            Request::Calc { mode, latex, values } => {
                self.prepare(*mode, settings)?;
                let values = values.iter().map(|(key, latex)| Assignment { key: key.clone(), latex: latex.clone() }).collect();
                let request = CalcRequest { latex: latex.clone(), values, session_id, precision };
                let response = api::calc(&mut self.store, &request).map_err(|error| error.error);
                // The values given are variables of the session now.
                self.save(&self.current);
                Ok(Reply::Result(shown(&response?, settings)))
            }
            Request::Solve { mode, latex, variable, values, start } => {
                self.prepare(*mode, settings)?;
                let values = values.iter().map(|(key, latex)| Assignment { key: key.clone(), latex: latex.clone() }).collect();
                let request = SolveRequest { latex: latex.clone(), variable: variable.clone(), values, start: start.clone(), session_id, precision };
                let response = api::solve_for(&mut self.store, &request).map_err(|error| error.error);
                self.save(&self.current);
                Ok(Reply::Result(response?.result))
            }
            _ => Err(text::NOT_AVAILABLE.get(settings.language).to_string()),
        }
    }

    /// Evaluates `latex` in the open session; `record` puts it into the history (S⇔D does not).
    fn calculate(&mut self, mode: Mode, settings: &Settings, latex: &str, record: bool) -> Result<String, String> {
        self.prepare(mode, settings)?;
        let (latex, session_id, precision) = (latex.to_string(), Some(self.current.clone()), Some(settings.digits.into()));
        let response = if record {
            let request = CalculationRequest { latex, session_id, precision, at: (self.clock)() };
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

    fn engineering(&mut self, mode: Mode, settings: &Settings, latex: &str, steps: i32) -> Option<Result<String, String>> {
        let work = |engine: &mut Engine| {
            engine.prepare(mode, settings)?;
            let request = CalcRequest { latex: latex.to_string(), values: Vec::new(), session_id: Some(engine.current.clone()), precision: Some(settings.digits.into()) };
            let response = api::calc(&mut engine.store, &request).map_err(|error| error.to_string())?;
            let core = core_settings(core_mode(mode).ok_or_else(|| text::NOT_AVAILABLE.get(settings.language).to_string())?, settings);
            let request = EngineeringRequest { value: response.value, imaginary: response.imaginary, unit_latex: response.unit_latex, steps };
            api::engineering(&core, &request).map(|response| response.latex).map_err(|error| error.to_string())
        };
        Some(work(self))
    }

    /// Keeps the sheet in the open session, then works it out; or what the calculation screen
    /// asks of the session.
    fn solve(&mut self, settings: &Settings, request: &Request) -> Result<Reply, String> {
        if matches!(request, Request::Variables { .. } | Request::Calc { .. } | Request::Solve { .. }) {
            return self.formula(settings, request);
        }
        self.zone = settings.zone;
        let current = Some(self.current.as_str());
        let mut inputs = api::get_mode_inputs(&mut self.store, current);
        sheet::keep(&mut inputs, request);
        api::set_mode_inputs(&mut self.store, current, inputs);
        self.save(&self.current);
        sheet::solve(settings, request, |mode| core_settings(mode, settings))
    }

    fn catalogue(&mut self, catalogue: Catalogue, lang: Lang) -> Vec<Group> {
        let language = lang;
        match catalogue {
            Catalogue::Constants => api::constants(language)
                .into_iter()
                .map(|category| Group {
                    name: category.name,
                    items: category.constants.into_iter().map(|c| Item { name: c.name, latex: c.latex }).collect(),
                })
                .collect(),
            Catalogue::Units => api::conversion_units(language)
                .into_iter()
                .map(|category| Group {
                    name: category.name,
                    items: category.units.into_iter().map(|unit| Item { name: unit.name, latex: unit.latex }).collect(),
                })
                .collect(),
            // The app lists these itself, from the OPTN tree.
            Catalogue::Functions => Vec::new(),
        }
    }

    fn unit_suggestions(&mut self, before: &str, recent: &[String], result: Option<&str>) -> Units {
        let request = api::UnitSuggestionRequest { before: before.to_string(), recent: recent.to_vec(), result: result.map(str::to_string) };
        let found = api::unit_suggestions(&request);
        let items = |list: Vec<api::UnitSuggestion>| list.into_iter().map(|unit| Item { name: unit.label, latex: unit.latex }).collect();
        Units { best: items(found.best), more: items(found.more) }
    }

    fn variables(&mut self, command: VariableCommand, settings: &Settings) -> Result<Vec<Stored>, String> {
        let current = Some(self.current.clone());
        match command {
            VariableCommand::List => {}
            // Kept as CALC keeps a value: any name, not only one with an index.
            VariableCommand::Store { mode, key, latex } => {
                self.prepare(mode, settings)?;
                let values = vec![Assignment { key: key.clone(), latex }];
                let request = CalcRequest { latex: key, values, session_id: current.clone(), precision: Some(settings.digits.into()) };
                let stored = api::calc(&mut self.store, &request).map(|_| ()).map_err(|error| error.error);
                self.save(&self.current);
                stored?;
            }
            VariableCommand::Delete(key) => {
                api::delete_variable(&mut self.store, &key, current.as_deref());
                self.save(&self.current);
            }
        }
        let variables = api::list_variables(&mut self.store, current.as_deref());
        let variables = variables.into_iter().map(|v| Stored { key: v.key, symbol: v.symbol, value: v.latex });
        // The matrices of Matrizen after them.
        let session = self.store.session(current.as_deref());
        let matrices = session.matrix_definitions().iter().map(|(key, latex)| Stored {
            key: key.clone(),
            symbol: quantus_core::session::symbol_of(key),
            value: latex.clone(),
        });
        Ok(variables.chain(matrices.collect::<Vec<_>>()).collect())
    }

    fn sessions(&mut self) -> Vec<SessionInfo> {
        api::list_sessions(&self.store).into_iter().map(|info| SessionInfo { id: info.id, name: info.name }).collect()
    }

    fn session(&mut self, command: SessionCommand, settings: &Settings) -> Result<Opened, String> {
        // The language and the time zone name a new session and a copy.
        self.zone = settings.zone;
        api::set_settings(&mut self.store, core_settings(CoreMode::Physics, settings)).map_err(|error| error.to_string())?;
        let current = self.current.clone();
        match command {
            SessionCommand::Current => {}
            SessionCommand::New => self.current = self.create(),
            SessionCommand::Open(id) => {
                if self.store.get(&id).is_none() {
                    return Err(quantus_texts::tr!(settings.language, "core.session.there_no_session"));
                }
                self.current = id;
            }
            SessionCommand::Rename(name) => {
                api::rename_session(&mut self.store, &current, &name).map_err(|error| error.error)?;
                self.save(&current);
            }
            SessionCommand::Duplicate => {
                let copy = api::duplicate_session(&mut self.store, &current, (self.clock)()).map_err(|error| error.error)?;
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

/// The engine's mode for a mode of the hui's that evaluates inputs. Analysis has its own call
/// (the curve sketch), and so do the sheets; here it and Linear with typed equations evaluate
/// plain inputs.
fn core_mode(mode: Mode) -> Option<CoreMode> {
    Some(match mode {
        Mode::Algebra => CoreMode::Algebra,
        Mode::Physics => CoreMode::Physics,
        Mode::Imaginary => CoreMode::Imaginary,
        Mode::Analysis => CoreMode::Analysis,
        Mode::Linear => CoreMode::Linear,
        Mode::Verify => CoreMode::Verify,
        Mode::BaseN => CoreMode::BaseN,
        Mode::Matrix => CoreMode::Matrix,
        // The app does not ask for these (`Mode::engine`).
        _ => return None,
    })
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
        language: settings.language,
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
        // The handheld has no Kirchhoff mode.
        star_delta: core::StarDelta::NodeRule,
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
        number_base: match settings.number_base {
            ui::NumberBase::Dec => core::NumberBase::Dec,
            ui::NumberBase::Hex => core::NumberBase::Hex,
            ui::NumberBase::Bin => core::NumberBase::Bin,
            ui::NumberBase::Oct => core::NumberBase::Oct,
        },
    }
}
