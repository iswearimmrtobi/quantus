//! The Tauri commands: each locks the session store and calls `quantus_core::api`. The engine
//! keeps no thread-local state, so only the store needs the mutex.
//!
//! The settings live in the store as well and are kept in a JSON file in the app's config
//! directory, which [`restore_settings`] reads at startup and `set_settings` rewrites.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::{fs, io};

use quantus_core::api::{
    self, ApiError, CalculationRequest, CalculationResponse, CircuitRequest, CircuitResponse,
    VariableResponse,
};
use quantus_core::session::SessionStore;
use quantus_core::settings::Settings;
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Manager, Runtime, State};

#[derive(Default)]
pub struct Engine {
    store: Mutex<SessionStore>,
    /// Where the settings are saved; unset (as in the tests) means they are not saved at all.
    settings_file: OnceLock<PathBuf>,
}

impl Engine {
    fn with<T>(&self, f: impl FnOnce(&mut SessionStore) -> T) -> T {
        // A poisoned lock only means an earlier command panicked; the store itself is intact.
        let mut store = self
            .store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        f(&mut store)
    }
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
    engine.with(|store| api::evaluate(store, &request))
}

#[tauri::command]
pub fn define_variable(
    engine: State<Engine>,
    request: CalculationRequest,
) -> Result<VariableResponse, ApiError> {
    engine.with(|store| api::define_variable(store, &request))
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

#[tauri::command]
pub fn list_variables(engine: State<Engine>, session_id: Option<String>) -> Vec<VariableResponse> {
    engine.with(|store| api::list_variables(store, session_id.as_deref()))
}

#[tauri::command]
pub fn delete_variable(engine: State<Engine>, key: String, session_id: Option<String>) {
    engine.with(|store| api::delete_variable(store, &key, session_id.as_deref()))
}

#[tauri::command]
pub fn get_settings(engine: State<Engine>) -> Settings {
    engine.with(|store| api::get_settings(store))
}

#[tauri::command]
pub fn set_settings(engine: State<Engine>, settings: Settings) -> Result<Settings, ApiError> {
    let saved = engine.with(|store| api::set_settings(store, settings))?;
    if let Some(path) = engine.settings_file.get() {
        save_settings(path, &saved).map_err(|error| ApiError {
            error: format!("Einstellungen konnten nicht gespeichert werden: {error}"),
            position: None,
        })?;
    }
    Ok(saved)
}

/// The version of the circuit file this build writes and reads.
const CIRCUIT_FILE_VERSION: u64 = 1;

const ELEMENT_KINDS: [&str; 7] = [
    "resistor",
    "wire",
    "voltageSource",
    "currentSource",
    "potentiometer",
    "powerLoad",
    "ground",
];

fn file_error(message: impl std::fmt::Display) -> ApiError {
    ApiError {
        error: message.to_string(),
        position: None,
    }
}

/// Checks the shape of a circuit file: the version, and for each element an ID, a known kind and
/// the editor's position. The parameter texts are not checked here; the solver reads them.
pub fn validate_circuit(circuit: &Value) -> Result<(), ApiError> {
    let version = circuit.get("version").and_then(Value::as_u64);
    if version != Some(CIRCUIT_FILE_VERSION) {
        return Err(file_error(match version {
            Some(other) => format!("Schaltungsdatei hat die unbekannte Version {other}."),
            None => "Das ist keine Schaltungsdatei: die Version fehlt.".to_string(),
        }));
    }
    let elements = circuit
        .get("elements")
        .and_then(Value::as_array)
        .ok_or_else(|| file_error("Schaltungsdatei ohne Bauteilliste."))?;
    let mut ids = std::collections::HashSet::new();
    for (index, element) in elements.iter().enumerate() {
        let number = index + 1;
        let id = element
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| !id.trim().is_empty())
            .ok_or_else(|| file_error(format!("Bauteil {number}: die ID fehlt.")))?;
        if !ids.insert(id) {
            return Err(file_error(format!("Die ID {id} kommt mehrfach vor.")));
        }
        let kind = element.get("kind").and_then(Value::as_str).unwrap_or_default();
        if !ELEMENT_KINDS.contains(&kind) {
            return Err(file_error(format!("{id}: unbekannte Bauteilart „{kind}“.")));
        }
        for key in ["x", "y"] {
            if !element.get(key).is_some_and(Value::is_number) {
                return Err(file_error(format!("{id}: die Position ({key}) fehlt.")));
            }
        }
    }
    Ok(())
}

/// Reads a circuit file. A broken one is reported instead of being returned, so the caller keeps
/// the circuit it has.
pub fn read_circuit(path: &Path) -> Result<Value, ApiError> {
    let text = fs::read_to_string(path)
        .map_err(|error| file_error(format!("{} konnte nicht gelesen werden: {error}", path.display())))?;
    let circuit: Value = serde_json::from_str(&text)
        .map_err(|error| file_error(format!("Keine gültige Schaltungsdatei: {error}")))?;
    validate_circuit(&circuit)?;
    Ok(circuit)
}

pub fn write_circuit(path: &Path, circuit: &Value) -> Result<(), ApiError> {
    validate_circuit(circuit)?;
    let text = serde_json::to_string_pretty(circuit).map_err(file_error)?;
    fs::write(path, text)
        .map_err(|error| file_error(format!("{} konnte nicht gespeichert werden: {error}", path.display())))
}

#[tauri::command]
pub fn load_circuit(path: String) -> Result<Value, ApiError> {
    read_circuit(Path::new(&path))
}

#[tauri::command]
pub fn save_circuit(path: String, circuit: Value) -> Result<(), ApiError> {
    write_circuit(Path::new(&path), &circuit)
}
