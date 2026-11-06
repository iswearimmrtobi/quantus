//! The remote engine: one JSON request in, one JSON response out, so a handheld can hand a
//! calculation to a Quantus engine elsewhere (the desktop app, a server) over WLAN or Bluetooth.
//! The transport is the shell's; this is only the format, and [`dispatch`], which a listener
//! calls with what it received.
//!
//! A request names an engine call (`evaluate`, `linearSystem`, …) and passes that call's own
//! request as `params`, the same JSON the desktop sends. It may bring the caller's `settings`
//! and `session` (a [`SessionFile`]): the call then runs on that session under those settings,
//! not on the listener's, and the response returns the session as it is afterwards, so the
//! handheld keeps its variables and history.
//!
//! ```json
//! { "version": 1, "id": 7, "method": "evaluate", "params": { "latex": "2 + 3" } }
//! { "version": 1, "id": 7, "result": { "result": "5", … } }
//! ```

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

use crate::api::{self, ApiError};
use crate::session::{Session, SessionFile, SessionStore};
use crate::settings::Settings;

/// The version of the format this engine speaks.
pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteRequest {
    pub version: u32,
    /// Chosen by the caller and echoed in the response, to match the two.
    #[serde(default)]
    pub id: Json,
    pub method: String,
    #[serde(default)]
    pub params: Json,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<Settings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionFile>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteResponse {
    pub version: u32,
    #[serde(default)]
    pub id: Json,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Json>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiError>,
    /// The caller's session after the call, when it sent one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionFile>,
}

fn error(message: impl Into<String>) -> ApiError {
    ApiError { error: message.into(), position: None }
}

/// Answers a request given as JSON with a response as JSON; `store` is the listener's own
/// engine, used when the request brings no session of its own.
pub fn dispatch(store: &mut SessionStore, request: &str) -> String {
    let response = match serde_json::from_str::<RemoteRequest>(request) {
        Ok(request) => handle(store, request),
        Err(e) => RemoteResponse {
            version: PROTOCOL_VERSION,
            id: Json::Null,
            result: None,
            error: Some(error(format!("Not a Quantus request: {e}"))),
            session: None,
        },
    };
    serde_json::to_string(&response).expect("a response serialises")
}

/// Answers a request.
pub fn handle(store: &mut SessionStore, request: RemoteRequest) -> RemoteResponse {
    let id = request.id.clone();
    let respond = |result: Result<Json, ApiError>, session: Option<SessionFile>| match result {
        Ok(json) => RemoteResponse { version: PROTOCOL_VERSION, id: id.clone(), result: Some(json), error: None, session },
        Err(e) => RemoteResponse { version: PROTOCOL_VERSION, id: id.clone(), result: None, error: Some(e), session },
    };
    if request.version > PROTOCOL_VERSION {
        return respond(Err(error(format!("This engine speaks version {PROTOCOL_VERSION}, not {}.", request.version))), None);
    }
    if request.session.is_none() && request.settings.is_none() {
        return respond(call(store, &request.method, request.params, None), None);
    }
    // The caller's own session and settings, on an engine of their own.
    let mut local = SessionStore::new();
    local.settings = request.settings.clone().unwrap_or_else(|| store.settings.clone());
    if let Err(e) = local.settings.validate() {
        return respond(Err(e.into()), None);
    }
    let session_id = match request.session {
        Some(file) => match Session::from_file(file, &local.settings) {
            Ok(session) => {
                let id = session.id.clone();
                local.insert(session);
                Some(id)
            }
            Err(e) => return respond(Err(e.into()), None),
        },
        None => None,
    };
    let result = call(&mut local, &request.method, request.params, session_id.as_deref());
    let session = session_id.map(|id| local.session(Some(&id)).to_file());
    respond(result, session)
}

fn params<T: DeserializeOwned>(params: Json) -> Result<T, ApiError> {
    let params = if params.is_null() { Json::Object(Default::default()) } else { params };
    serde_json::from_value(params).map_err(|e| error(format!("The parameters do not fit: {e}")))
}

fn json(value: impl Serialize) -> Result<Json, ApiError> {
    serde_json::to_value(value).map_err(|e| error(e.to_string()))
}

/// The engine call `method` with its request in `params`; a call on a session uses `session`
/// unless the request names one.
fn call(store: &mut SessionStore, method: &str, raw: Json, session: Option<&str>) -> Result<Json, ApiError> {
    // Session calls fall back to the session the request brought.
    let with_session = |mut raw: Json| {
        if let (Some(id), Json::Object(map)) = (session, &mut raw) {
            map.entry("sessionId").or_insert_with(|| Json::String(id.to_string()));
        }
        if raw.is_null() {
            if let Some(id) = session {
                return serde_json::json!({ "sessionId": id });
            }
        }
        raw
    };
    let settings = store.settings.clone();
    match method {
        "evaluate" => json(api::evaluate(store, &params(with_session(raw))?)?),
        "defineVariable" => json(api::define_variable(store, &params(with_session(raw))?)?),
        "freeVariables" => json(api::free_variables(store, &params(with_session(raw))?)?),
        "calc" => json(api::calc(store, &params(with_session(raw))?)?),
        "solveFor" => json(api::solve_for(store, &params(with_session(raw))?)?),
        "listVariables" => {
            let request: api::CalculationRequest = params(with_session(raw))?;
            json(api::list_variables(store, request.session_id.as_deref()))
        }
        "history" => {
            let request: api::CalculationRequest = params(with_session(raw))?;
            json(api::history(store, request.session_id.as_deref()))
        }
        "engineering" => json(api::engineering(&settings, &params(raw)?)?),
        "analyze" => json(api::analyze(&settings, &params(raw)?)?),
        "sampleFunction" => json(api::sample_function(&settings, &params(raw)?)?),
        "tangent" => json(api::tangent(&settings, &params(raw)?)?),
        "area" => json(api::area(&settings, &params(raw)?)?),
        "valueTable" => json(api::value_table(&settings, &params(raw)?)?),
        "linearSystem" => json(api::linear_system(&settings, &params(raw)?)?),
        "kirchhoff" => json(api::kirchhoff(&settings, &params(raw)?)?),
        "solveCircuit" => json(api::solve_circuit(&settings, &params(raw)?)?),
        "statistics" => json(api::statistics(&settings, &params(raw)?)?),
        "distribution" => json(api::distribution(&settings, &params(raw)?)?),
        "constants" => json(api::constants(settings.language)),
        "conversionUnits" => json(api::conversion_units(settings.language)),
        "searchLibrary" => json(api::search_library(settings.language, &params(raw)?)),
        "getSettings" => json(api::get_settings(store)),
        "ping" => json(serde_json::json!({ "version": PROTOCOL_VERSION })),
        other => Err(error(format!("Unknown method '{other}'."))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn send(store: &mut SessionStore, request: Json) -> RemoteResponse {
        serde_json::from_str(&dispatch(store, &request.to_string())).unwrap()
    }

    #[test]
    fn a_calculation_answers_with_its_id() {
        let mut store = SessionStore::new();
        let response = send(&mut store, json!({ "version": 1, "id": 7, "method": "evaluate", "params": { "latex": "2 + 3" } }));
        assert_eq!(response.id, json!(7));
        assert_eq!(response.result.unwrap()["result"], "5");
        assert!(response.error.is_none() && response.session.is_none());
    }

    #[test]
    fn errors_come_back_as_errors() {
        let mut store = SessionStore::new();
        let parse = send(&mut store, json!({ "version": 1, "id": "a", "method": "evaluate", "params": { "latex": "2 +" } }));
        assert!(parse.result.is_none() && parse.error.unwrap().position.is_some());
        let unknown = send(&mut store, json!({ "version": 1, "method": "fly" }));
        assert!(unknown.error.unwrap().error.contains("Unknown method"));
        let newer = send(&mut store, json!({ "version": 9, "method": "ping" }));
        assert!(newer.error.unwrap().error.contains("speaks version 1"));
        let garbage: RemoteResponse = serde_json::from_str(&dispatch(&mut store, "{")).unwrap();
        assert!(garbage.error.is_some());
    }

    #[test]
    fn the_callers_session_and_settings_travel_along() {
        // The handheld's session with x_1 = 4, its settings in German.
        let mut handheld = SessionStore::new();
        api::evaluate(&mut handheld, &api::CalculationRequest { latex: "x_1 = 4".into(), ..Default::default() }).unwrap();
        let file = handheld.session(None).to_file();
        let settings = Settings { language: crate::settings::Language::De, ..Settings::default() };
        let mut desktop = SessionStore::new();
        let request = json!({
            "version": 1,
            "id": 1,
            "method": "evaluate",
            "params": { "latex": "x_1 \\cdot 2 = 8" },
            "settings": settings,
            "session": file,
        });
        let response = send(&mut desktop, request);
        assert_eq!(response.result.unwrap()["result"], "\\text{wahr}");
        let session = response.session.unwrap();
        assert_eq!(session.history.len(), 2);
        // The listener's own session is untouched.
        assert!(desktop.session(None).history().is_empty());
    }

    #[test]
    fn every_stateless_method_takes_its_request() {
        let mut store = SessionStore::new();
        let calls = [
            ("linearSystem", json!({ "equations": "x + y = 2; x - y = 0" })),
            ("statistics", json!({ "x": ["1", "2", "3"] })),
            ("distribution", json!({ "kind": "binomialDensity", "n": "2", "p": "\\frac{1}{2}", "k": ["1"] })),
            ("engineering", json!({ "value": "1234", "steps": 0 })),
            ("analyze", json!({ "latex": "x^2" })),
            ("constants", Json::Null),
            ("conversionUnits", Json::Null),
            ("searchLibrary", json!({ "query": "steff" })),
            ("getSettings", Json::Null),
            ("freeVariables", json!({ "latex": "a + b" })),
            ("solveFor", json!({ "latex": "2x = 4", "variable": "x" })),
            ("calc", json!({ "latex": "y_1 + 1", "values": [{ "key": "y_1", "latex": "2" }] })),
            ("history", Json::Null),
        ];
        for (method, params) in calls {
            let response = send(&mut store, json!({ "version": 1, "method": method, "params": params }));
            assert!(response.error.is_none(), "{method}: {:?}", response.error);
        }
    }
}
