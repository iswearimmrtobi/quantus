//! The Tauri commands: each locks the session store and calls `quantus_core::api`. The engine
//! keeps no thread-local state, so only the store needs the mutex.
//!
//! The settings live in the store as well and are kept in a JSON file in the app's config
//! directory, which [`restore_settings`] reads at startup and `set_settings` rewrites. Each
//! session is a file of its own in `sessions/` next to it: [`restore_sessions`] reads them all,
//! and every command that changes a session writes its file again.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::{fs, io};

use quantus_core::api::{
    self, AnalysisRequest, AnalysisResponse, ApiError, AreaRequest, AreaResponse, CalculationRequest,
    CalcRequest, CalculationResponse, HistoryEntry, ModeInputs, NewSessionRequest, SessionInfo, CircuitRequest, FreeVariable, SolveRequest, CircuitResponse, ConstantCategory, DistributionRequest, DistributionResponse, EngineeringRequest, EngineeringResponse, GeometryRequest, GeometryResponse, LogicRequest, LogicResponse, ViewRequest, ViewResponse, KirchhoffRequest, KirchhoffResponse,
    LinearSystemRequest, LinearSystemResponse, SampleRequest, SampleResponse, StatisticsRequest, StatisticsResponse, TableRequest, TableResponse, TangentRequest, TangentResponse, UnitCategory,
    VariableResponse,
};
use quantus_core::session::{SessionStore, DEFAULT_SESSION};
use quantus_core::settings::{Language, Settings};
use quantus_texts::tr;
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Manager, Runtime, State};

#[derive(Default)]
pub struct Engine {
    store: Mutex<SessionStore>,
    /// Where the settings are saved; unset (as in the tests) means they are not saved at all.
    settings_file: OnceLock<PathBuf>,
    /// Where the sessions are saved, one file each; unset means they are kept in memory only.
    sessions_dir: OnceLock<PathBuf>,
}

impl Engine {
    pub fn language(&self) -> Language {
        self.with(|store| store.settings.language)
    }

    fn with<T>(&self, f: impl FnOnce(&mut SessionStore) -> T) -> T {
        // A poisoned lock only means an earlier command panicked; the store itself is intact.
        let mut store = self
            .store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        f(&mut store)
    }

    /// Like [`Engine::with`], then writes the session `id` (the default one for `None`) to its file.
    fn with_saved<T>(&self, id: Option<&str>, f: impl FnOnce(&mut SessionStore) -> T) -> T {
        self.with(|store| {
            let result = f(store);
            if let Some(dir) = self.sessions_dir.get() {
                let id = id.filter(|id| !id.trim().is_empty()).unwrap_or(DEFAULT_SESSION);
                if let Err(error) = save_session(dir, store, id) {
                    eprintln!("Quantus: the session {id} could not be saved: {error}");
                }
            }
            result
        })
    }
}

/// The file of session `id` in `dir`, or `None` for an id that is no plain file name.
pub fn session_path(dir: &Path, id: &str) -> Option<PathBuf> {
    let plain = !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    plain.then(|| dir.join(format!("{id}.json")))
}

/// Writes session `id` to its file; a session the store does not have is left alone.
pub fn save_session(dir: &Path, store: &mut SessionStore, id: &str) -> io::Result<()> {
    let Some(path) = session_path(dir, id) else { return Ok(()) };
    if store.get(id).is_none() {
        return Ok(());
    }
    fs::create_dir_all(dir)?;
    fs::write(path, api::export_session(store, Some(id)))
}

/// Reads every session file in `dir` into the store; a broken one is reported and skipped.
pub fn load_sessions(dir: &Path, store: &mut SessionStore) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for path in entries.flatten().map(|entry| entry.path()) {
        if path.extension().is_none_or(|extension| extension != "json") {
            continue;
        }
        let read = fs::read_to_string(&path).map_err(|e| e.to_string());
        if let Err(error) = read.and_then(|json| api::import_session(store, &json).map_err(|e| e.error)) {
            eprintln!("Quantus: ignoring {}: {error}", path.display());
        }
    }
}

/// Loads the saved sessions and remembers their directory for later saves. Call it after
/// [`restore_settings`]: reading a session evaluates its variables with the settings.
pub fn restore_sessions<R: Runtime>(app: &AppHandle<R>) {
    let Ok(dir) = app.path().app_config_dir() else { return };
    let dir = dir.join("sessions");
    let engine = app.state::<Engine>();
    engine.with(|store| load_sessions(&dir, store));
    let _ = engine.sessions_dir.set(dir);
}

/// Reads the settings file; a missing file means the defaults.
pub fn load_settings(path: &Path) -> Result<Settings, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Settings::default()),
        Err(error) => return Err(error.to_string()),
    };
    let settings: Settings = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    settings.validate().map_err(|e| e.to_string())?;
    Ok(settings)
}

pub fn save_settings(path: &Path, settings: &Settings) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let text = serde_json::to_string_pretty(settings).map_err(io::Error::other)?;
    fs::write(path, text)
}

/// Loads the saved settings into the engine and remembers the file for later saves. A broken
/// file is reported and replaced by the defaults rather than stopping the app.
pub fn restore_settings<R: Runtime>(app: &AppHandle<R>) {
    let Ok(dir) = app.path().app_config_dir() else {
        eprintln!("Quantus: no config directory, settings will not be saved");
        return;
    };
    let path = dir.join("settings.json");
    let settings = load_settings(&path).unwrap_or_else(|error| {
        eprintln!("Quantus: ignoring {}: {error}", path.display());
        Settings::default()
    });
    let engine = app.state::<Engine>();
    engine.with(|store| store.settings = settings);
    let _ = engine.settings_file.set(path);
}

#[derive(Serialize)]
pub struct EngineInfo {
    pub application: &'static str,
    pub precision: usize,
}

#[tauri::command]
pub fn engine_info(engine: State<Engine>) -> EngineInfo {
    EngineInfo {
        application: "Quantus",
        precision: engine.with(|store| store.settings.precision),
    }
}

#[tauri::command]
pub fn evaluate(
    engine: State<Engine>,
    request: CalculationRequest,
) -> Result<CalculationResponse, ApiError> {
    engine.with_saved(request.session_id.as_deref(), |store| api::evaluate(store, &request))
}

#[tauri::command]
pub fn define_variable(
    engine: State<Engine>,
    request: CalculationRequest,
) -> Result<VariableResponse, ApiError> {
    engine.with_saved(request.session_id.as_deref(), |store| api::define_variable(store, &request))
}

/// The solver is stateless, so only the settings are taken from the store; the lock is not held
/// while solving.
#[tauri::command]
pub fn solve_circuit(
    engine: State<Engine>,
    request: CircuitRequest,
) -> Result<CircuitResponse, ApiError> {
    let settings = engine.with(|store| api::get_settings(store));
    api::solve_circuit(&settings, &request)
}

/// A Geometry scene and a task on it. Stateless: the scene is saved with the mode inputs.
#[tauri::command]
pub fn geometry(engine: State<Engine>, request: GeometryRequest) -> Result<GeometryResponse, ApiError> {
    let settings = engine.with(|store| api::get_settings(store));
    api::geometry(&settings, &request)
}

/// The Geometry scene projected for the 3D view, at the camera's angles and zoom.
#[tauri::command]
pub fn geometry_view(engine: State<Engine>, request: ViewRequest) -> Result<ViewResponse, ApiError> {
    let settings = engine.with(|store| api::get_settings(store));
    api::geometry_view(&settings, &request)
}

/// A Logic scene and a task on it: every output's table, forms, KV diagram and gates, and the
/// circuit's figures. Stateless: the scene is saved with the mode inputs.
#[tauri::command]
pub fn logic(engine: State<Engine>, request: LogicRequest) -> Result<LogicResponse, ApiError> {
    let settings = engine.with(|store| api::get_settings(store));
    api::logic(&settings, &request)
}

/// A resistor network by the junction and loop rules. Stateless like `solve_circuit`.
#[tauri::command]
pub fn solve_kirchhoff(
    engine: State<Engine>,
    request: KirchhoffRequest,
) -> Result<KirchhoffResponse, ApiError> {
    let settings = engine.with(|store| api::get_settings(store));
    api::kirchhoff(&settings, &request)
}

/// The curve sketch of a function. Like `solve_circuit` it touches no session.
#[tauri::command]
pub fn analyze(engine: State<Engine>, request: AnalysisRequest) -> Result<AnalysisResponse, ApiError> {
    let settings = engine.with(|store| api::get_settings(store));
    api::analyze(&settings, &request)
}

/// A system of linear equations, typed or as a matrix. Like `analyze` it touches no session.
#[tauri::command]
pub fn solve_linear_system(
    engine: State<Engine>,
    request: LinearSystemRequest,
) -> Result<LinearSystemResponse, ApiError> {
    let settings = engine.with(|store| api::get_settings(store));
    api::linear_system(&settings, &request)
}

/// A result in engineering notation, one ENG or ←ENG step at a time. It touches no session.
#[tauri::command]
pub fn engineering(engine: State<Engine>, request: EngineeringRequest) -> Result<EngineeringResponse, ApiError> {
    let settings = engine.with(|store| api::get_settings(store));
    api::engineering(&settings, &request)
}

/// The statistics of a data list. It touches no session.
#[tauri::command]
pub fn statistics(engine: State<Engine>, request: StatisticsRequest) -> Result<StatisticsResponse, ApiError> {
    let settings = engine.with(|store| api::get_settings(store));
    api::statistics(&settings, &request)
}

/// A probability of a distribution. It touches no session.
#[tauri::command]
pub fn distribution(engine: State<Engine>, request: DistributionRequest) -> Result<DistributionResponse, ApiError> {
    let settings = engine.with(|store| api::get_settings(store));
    api::distribution(&settings, &request)
}

/// The physical constants by category, named in the user's language.
#[tauri::command]
pub fn constants(engine: State<Engine>) -> Vec<ConstantCategory> {
    let settings = engine.with(|store| api::get_settings(store));
    api::constants(settings.language)
}

/// The units a conversion can aim at, by category.
#[tauri::command]
pub fn conversion_units(engine: State<Engine>) -> Vec<UnitCategory> {
    let settings = engine.with(|store| api::get_settings(store));
    api::conversion_units(settings.language)
}

/// The constants and units that fit a quick-search query, best first.
#[tauri::command]
pub fn search_library(engine: State<Engine>, request: api::LibrarySearchRequest) -> Vec<api::LibraryHit> {
    let settings = engine.with(|store| api::get_settings(store));
    api::search_library(settings.language, &request)
}

/// The letters of an input that have no value yet (CALC).
#[tauri::command]
pub fn free_variables(engine: State<Engine>, request: CalculationRequest) -> Result<Vec<FreeVariable>, ApiError> {
    engine.with(|store| api::free_variables(store, &request))
}

/// CALC: the values become session variables, then the input is evaluated.
#[tauri::command]
pub fn calc(engine: State<Engine>, request: CalcRequest) -> Result<CalculationResponse, ApiError> {
    engine.with_saved(request.session_id.as_deref(), |store| api::calc(store, &request))
}

/// SOLVE for a named letter.
#[tauri::command]
pub fn solve_for(engine: State<Engine>, request: SolveRequest) -> Result<CalculationResponse, ApiError> {
    engine.with_saved(request.session_id.as_deref(), |store| api::solve_for(store, &request))
}

/// A new session, named after the time unless a name is given.
#[tauri::command]
pub fn new_session(engine: State<Engine>, request: NewSessionRequest) -> SessionInfo {
    let info = engine.with(|store| api::new_session(store, &request));
    engine.with_saved(Some(&info.id), |_| info.clone())
}

/// Marks a session as the one in use, so the next start opens it.
#[tauri::command]
pub fn open_session(engine: State<Engine>, id: String, now: Option<i64>) -> Result<SessionInfo, ApiError> {
    engine.with_saved(Some(&id), |store| {
        let language = store.settings.language;
        let session = store
            .get_mut(&id)
            .ok_or_else(|| ApiError::new(tr!(language, "core.session.there_no_session")))?;
        if now.is_some() {
            session.last_used = now;
        }
        Ok(api::list_sessions(store).into_iter().find(|info| info.id == id).expect("the session exists"))
    })
}

#[tauri::command]
pub fn list_sessions(engine: State<Engine>) -> Vec<SessionInfo> {
    engine.with(|store| api::list_sessions(store))
}

#[tauri::command]
pub fn rename_session(engine: State<Engine>, id: String, name: String) -> Result<SessionInfo, ApiError> {
    engine.with_saved(Some(&id), |store| api::rename_session(store, &id, &name))
}

#[tauri::command]
pub fn duplicate_session(engine: State<Engine>, id: String, now: Option<i64>) -> Result<SessionInfo, ApiError> {
    let info = engine.with(|store| api::duplicate_session(store, &id, now))?;
    Ok(engine.with_saved(Some(&info.id), |_| info.clone()))
}

#[tauri::command]
pub fn delete_session(engine: State<Engine>, id: String) -> bool {
    let deleted = engine.with(|store| api::delete_session(store, &id));
    if let Some(path) = engine.sessions_dir.get().and_then(|dir| session_path(dir, &id)) {
        let _ = fs::remove_file(path);
    }
    deleted
}

/// The session as JSON, for the frontend to write to a file.
#[tauri::command]
pub fn export_session(engine: State<Engine>, session_id: Option<String>) -> String {
    engine.with(|store| api::export_session(store, session_id.as_deref()))
}

#[tauri::command]
pub fn import_session(engine: State<Engine>, json: String) -> Result<SessionInfo, ApiError> {
    let info = engine.with(|store| api::import_session(store, &json))?;
    Ok(engine.with_saved(Some(&info.id), |_| info.clone()))
}

#[tauri::command]
pub fn history(engine: State<Engine>, session_id: Option<String>) -> Vec<HistoryEntry> {
    engine.with(|store| api::history(store, session_id.as_deref()))
}

#[tauri::command]
pub fn writeup(engine: State<Engine>, session_id: Option<String>) -> quantus_core::writeup::Writeup {
    engine.with(|store| api::writeup(store, session_id.as_deref()))
}

#[tauri::command]
pub fn clear_history(engine: State<Engine>, session_id: Option<String>) {
    engine.with_saved(session_id.as_deref(), |store| api::clear_history(store, session_id.as_deref()))
}

#[tauri::command]
pub fn get_mode_inputs(engine: State<Engine>, session_id: Option<String>) -> ModeInputs {
    engine.with(|store| api::get_mode_inputs(store, session_id.as_deref()))
}

#[tauri::command]
pub fn set_mode_inputs(engine: State<Engine>, session_id: Option<String>, inputs: ModeInputs) {
    engine.with_saved(session_id.as_deref(), |store| api::set_mode_inputs(store, session_id.as_deref(), inputs))
}

/// Fresh curve segments while the graph is panned or zoomed.
#[tauri::command]
pub fn sample_function(engine: State<Engine>, request: SampleRequest) -> Result<SampleResponse, ApiError> {
    let settings = engine.with(|store| api::get_settings(store));
    api::sample_function(&settings, &request)
}

#[tauri::command]
pub fn tangent(engine: State<Engine>, request: TangentRequest) -> Result<TangentResponse, ApiError> {
    let settings = engine.with(|store| api::get_settings(store));
    api::tangent(&settings, &request)
}

#[tauri::command]
pub fn area_under_curve(engine: State<Engine>, request: AreaRequest) -> Result<AreaResponse, ApiError> {
    let settings = engine.with(|store| api::get_settings(store));
    api::area(&settings, &request)
}

#[tauri::command]
pub fn value_table(engine: State<Engine>, request: TableRequest) -> Result<TableResponse, ApiError> {
    let settings = engine.with(|store| api::get_settings(store));
    api::value_table(&settings, &request)
}

#[tauri::command]
pub fn list_variables(engine: State<Engine>, session_id: Option<String>) -> Vec<VariableResponse> {
    engine.with(|store| api::list_variables(store, session_id.as_deref()))
}

#[tauri::command]
pub fn delete_variable(engine: State<Engine>, key: String, session_id: Option<String>) {
    engine.with_saved(session_id.as_deref(), |store| api::delete_variable(store, &key, session_id.as_deref()))
}

#[tauri::command]
pub fn get_settings(engine: State<Engine>) -> Settings {
    engine.with(|store| api::get_settings(store))
}

#[tauri::command]
pub fn set_settings<R: Runtime>(
    app: AppHandle<R>,
    engine: State<Engine>,
    settings: Settings,
) -> Result<Settings, ApiError> {
    let before = engine.language();
    let saved = engine.with(|store| api::set_settings(store, settings))?;
    #[cfg(desktop)]
    if saved.language != before {
        crate::shell::set_menu_language(&app, saved.language);
    }
    #[cfg(mobile)]
    let _ = (app, before);
    if let Some(path) = engine.settings_file.get() {
        save_settings(path, &saved)
            .map_err(|error| ApiError::new(tr!(saved.language, "shell.commands.settings_could_not_saved")))?;
    }
    Ok(saved)
}

/// The version of the circuit file this build writes and reads.
const CIRCUIT_FILE_VERSION: u64 = 1;

const ELEMENT_KINDS: [&str; 8] = [
    "resistor",
    "wire",
    "voltageSource",
    "currentSource",
    "potentiometer",
    "powerLoad",
    "ground",
    "terminal",
];

fn file_error(message: impl std::fmt::Display) -> ApiError {
    ApiError::new(message.to_string())
}

/// Checks the shape of a circuit file: the version, and for each element an ID, a known kind and
/// the editor's position, and that `reversed` (the Kirchhoff mode's flipped current) is a
/// yes/no if it is there. The parameter texts are not checked here; the solver reads them.
pub fn validate_circuit(circuit: &Value, language: Language) -> Result<(), ApiError> {
    let version = circuit.get("version").and_then(Value::as_u64);
    if version != Some(CIRCUIT_FILE_VERSION) {
        return Err(file_error(match version {
            Some(other) => tr!(
                language,
                "shell.commands.circuit_file_has_unknown_version"),
            None => tr!(
                language,
                "shell.commands.not_circuit_file_version_missing"),
        }));
    }
    let elements = circuit
        .get("elements")
        .and_then(Value::as_array)
        .ok_or_else(|| file_error(tr!(language, "shell.commands.circuit_file_has_no_list")))?;
    let mut ids = std::collections::HashSet::new();
    for (index, element) in elements.iter().enumerate() {
        let number = index + 1;
        let id = element
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| !id.trim().is_empty())
            .ok_or_else(|| file_error(tr!(language, "shell.commands.part_id_missing")))?;
        if !ids.insert(id) {
            return Err(file_error(tr!(language, "shell.commands.id_occurs_more_than_once")));
        }
        let kind = element.get("kind").and_then(Value::as_str).unwrap_or_default();
        if !ELEMENT_KINDS.contains(&kind) {
            return Err(file_error(tr!(language, "shell.commands.unknown_kind_part")));
        }
        for key in ["x", "y"] {
            if !element.get(key).is_some_and(Value::is_number) {
                return Err(file_error(tr!(language, "shell.commands.position_missing")));
            }
        }
        if element.get("reversed").is_some_and(|reversed| !reversed.is_boolean()) {
            return Err(file_error(tr!(
                language,
                "shell.commands.reversed_must_true_false")));
        }
    }
    Ok(())
}

/// Reads a circuit file. A broken one is reported instead of being returned, so the caller keeps
/// the circuit it has.
#[cfg(test)]
pub fn read_circuit(path: &Path, language: Language) -> Result<Value, ApiError> {
    parse_circuit(fs::read_to_string(path), &path.display().to_string(), language)
}

/// `text` is what reading the file `shown` gave.
fn parse_circuit(text: io::Result<String>, shown: &str, language: Language) -> Result<Value, ApiError> {
    let text = text
        .map_err(|error| file_error(tr!(
            language,
            "shell.commands.could_not_read",
            shown
        )))?;
    let circuit: Value = serde_json::from_str(&text)
        .map_err(|error| file_error(tr!(language, "shell.commands.not_valid_circuit_file")))?;
    validate_circuit(&circuit, language)?;
    Ok(circuit)
}

#[tauri::command]
pub fn load_circuit<R: Runtime>(app: AppHandle<R>, engine: State<Engine>, path: String) -> Result<Value, ApiError> {
    parse_circuit(read_picked(&app, &path), &path, engine.language())
}

#[tauri::command]
pub fn save_circuit<R: Runtime>(
    app: AppHandle<R>,
    engine: State<Engine>,
    path: String,
    circuit: Value,
) -> Result<(), ApiError> {
    let language = engine.language();
    validate_circuit(&circuit, language)?;
    let text = serde_json::to_string_pretty(&circuit).map_err(file_error)?;
    write_picked(&app, &path, &text)
        .map_err(|error| file_error(tr!(
            language,
            "shell.commands.could_not_saved",
            path
        )))
}

/// A file from the open dialog: a path on the desktop.
#[cfg(desktop)]
fn read_picked<R: Runtime>(_app: &AppHandle<R>, path: &str) -> io::Result<String> {
    fs::read_to_string(path)
}

#[cfg(desktop)]
fn write_picked<R: Runtime>(_app: &AppHandle<R>, path: &str, text: &str) -> io::Result<()> {
    fs::write(path, text)
}

/// On Android the dialog hands out `content://` URIs, which only the fs plugin can open.
#[cfg(mobile)]
fn read_picked<R: Runtime>(app: &AppHandle<R>, path: &str) -> io::Result<String> {
    use tauri_plugin_fs::{FilePath, FsExt};
    let path: FilePath = path.parse().map_err(io::Error::other)?;
    app.fs().read_to_string(path)
}

#[cfg(mobile)]
fn write_picked<R: Runtime>(app: &AppHandle<R>, path: &str, text: &str) -> io::Result<()> {
    use io::Write;
    use tauri_plugin_fs::{FilePath, FsExt, OpenOptions};
    let path: FilePath = path.parse().map_err(io::Error::other)?;
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    app.fs().open(path, options)?.write_all(text.as_bytes())
}
