//! The Tauri commands: each locks the session store and calls `quantus_core::api`. The engine
//! keeps no thread-local state, so only the store needs the mutex.

use std::sync::Mutex;

use quantus_core::api::{self, ApiError, CalculationRequest, CalculationResponse, VariableResponse};
use quantus_core::num::DEFAULT_PRECISION;
use quantus_core::session::SessionStore;
use serde::Serialize;
use tauri::State;

#[derive(Default)]
pub struct Engine(Mutex<SessionStore>);

impl Engine {
    fn with<T>(&self, f: impl FnOnce(&mut SessionStore) -> T) -> T {
        // A poisoned lock only means an earlier command panicked; the store itself is intact.
        let mut store = self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        f(&mut store)
    }
}

#[derive(Serialize)]
pub struct EngineInfo {
    pub application: &'static str,
    pub precision: usize,
}

#[tauri::command]
pub fn engine_info() -> EngineInfo {
    EngineInfo { application: "Quantus", precision: DEFAULT_PRECISION }
}

#[tauri::command]
pub fn evaluate(engine: State<Engine>, request: CalculationRequest) -> Result<CalculationResponse, ApiError> {
    engine.with(|store| api::evaluate(store, &request))
}

#[tauri::command]
pub fn define_variable(engine: State<Engine>, request: CalculationRequest) -> Result<VariableResponse, ApiError> {
    engine.with(|store| api::define_variable(store, &request))
}

#[tauri::command]
pub fn list_variables(engine: State<Engine>, session_id: Option<String>) -> Vec<VariableResponse> {
    engine.with(|store| api::list_variables(store, session_id.as_deref()))
}

#[tauri::command]
pub fn delete_variable(engine: State<Engine>, key: String, session_id: Option<String>) {
    engine.with(|store| api::delete_variable(store, &key, session_id.as_deref()))
}
