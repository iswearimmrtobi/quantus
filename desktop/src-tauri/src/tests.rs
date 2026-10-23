//! The commands through Tauri's own IPC layer, as the renderer's `invoke` reaches them.

use serde_json::{json, Value};
use tauri::test::{mock_builder, mock_context, noop_assets, MockRuntime};
use tauri::webview::InvokeRequest;
use tauri::{WebviewWindow, WebviewWindowBuilder};

fn window() -> WebviewWindow<MockRuntime> {
    if std::env::var("XDG_DATA_HOME").is_err() {
        std::env::set_var("XDG_DATA_HOME", std::env::temp_dir());
    }
    let app = super::app(mock_builder())
        .build(mock_context(noop_assets()))
        .unwrap();
    WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap()
}

fn invoke(window: &WebviewWindow<MockRuntime>, command: &str, body: Value) -> Result<Value, Value> {
    let request = InvokeRequest {
        cmd: command.into(),
        callback: tauri::ipc::CallbackFn(0),
        error: tauri::ipc::CallbackFn(1),
        url: "tauri://localhost".parse().unwrap(),
        body: tauri::ipc::InvokeBody::Json(body),
        headers: Default::default(),
        invoke_key: tauri::test::INVOKE_KEY.to_string(),
    };
    tauri::test::get_ipc_response(window, request).map(|body| body.deserialize().unwrap())
}

fn evaluate(window: &WebviewWindow<MockRuntime>, latex: &str) -> Result<Value, Value> {
    invoke(
        window,
        "evaluate",
        json!({ "request": { "latex": latex, "sessionId": "desktop" } }),
    )
}

fn keys(window: &WebviewWindow<MockRuntime>) -> Vec<String> {
    invoke(window, "list_variables", json!({ "sessionId": "desktop" }))
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["key"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn milestone_result_reaches_the_renderer() {
    let w = window();
    let calc = evaluate(&w, r"F_1 = 2kg \cdot 10\frac{m}{s^2}").unwrap();
    assert_eq!(calc["latex"], r"F_{1} = 20\,\mathrm{N}");
    assert_eq!(calc["quantityName"], "force");
    assert_eq!(calc["dimension"], "[L]·[M]/[T]²");
    let kinds: Vec<&str> = calc["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["symbolic", "result"]);
    assert_eq!(calc["variable"]["key"], "F_1");
}

#[test]
fn variables_are_defined_reused_and_forgotten() {
    let w = window();
    evaluate(&w, r"m_1 = 2kg").unwrap();
    evaluate(&w, r"a_1 = 10\frac{m}{s^2}").unwrap();
    assert_eq!(keys(&w), ["m_1", "a_1"]);

    let reused = evaluate(&w, r"F = m_1 \cdot a_1").unwrap();
    assert_eq!(reused["result"], r"20\,\mathrm{N}");
    assert_eq!(
        reused["steps"].as_array().unwrap().len(),
        3,
        "variables add the substituted step"
    );

    invoke(
        &w,
        "delete_variable",
        json!({ "key": "m_1", "sessionId": "desktop" }),
    )
    .unwrap();
    assert_eq!(keys(&w), ["a_1"]);
}

#[test]
fn parse_errors_carry_a_position() {
    let error = evaluate(&window(), r"2 + * 3").unwrap_err();
    assert!(
        error["error"].as_str().unwrap().contains("position"),
        "{error}"
    );
    assert!(error["position"].is_u64(), "{error}");
}

#[test]
fn dimension_errors_have_text_and_no_position() {
    let error = evaluate(&window(), r"1m + 1s").unwrap_err();
    assert!(error["position"].is_null(), "{error}");
    assert!(error["error"].as_str().unwrap().len() > 10, "{error}");
}

#[test]
fn engine_info_reports_the_default_precision() {
    let info = invoke(&window(), "engine_info", json!({})).unwrap();
    assert_eq!(info["precision"], 50);
}

#[test]
fn menu_has_an_item_for_every_forwarded_shortcut() {
    let app = super::app(mock_builder())
        .build(mock_context(noop_assets()))
        .unwrap();
    let menu = super::shell::build_menu(app.handle()).unwrap();
    let ids: Vec<String> = menu
        .items()
        .unwrap()
        .iter()
        .filter_map(|top| top.as_submenu())
        .flat_map(|submenu| submenu.items().unwrap())
        .map(|entry| entry.id().as_ref().to_string())
        .collect();
    for id in super::shell::FORWARDED {
        assert!(ids.iter().any(|found| found == id), "menu item {id} missing");
    }
}

#[test]
fn algebraic_simplification_reaches_the_renderer() {
    let w = window();
    let calc = evaluate(&w, r"3 / ((x + 3)/(x - 2)) + ((x - 102)/(x - 4))/(x + 3)").unwrap();
    assert_eq!(calc["result"], r"\frac{3x - 26}{x - 4}");
    assert_eq!(
        calc["domainRestrictions"],
        json!(["x \\neq -3, 2, 4"])
    );
    let steps = calc["steps"].as_array().unwrap();
    assert!(steps.len() >= 5);
    let algebraic_step = steps.iter().find(|s| s["kind"] == "algebraic").unwrap();
    assert!(algebraic_step["explanation"].is_string());
}

#[test]
fn settings_round_trip_and_change_the_result() {
    let w = window();
    let defaults = invoke(&w, "get_settings", json!({})).unwrap();
    assert_eq!(defaults["binomialForm"], "auto");
    assert_eq!(evaluate(&w, "a^2+2ab+b^2").unwrap()["result"], "(a + b)^{2}");

    let mut changed = defaults.clone();
    changed["binomialForm"] = json!("expanded");
    changed["precision"] = json!(7);
    assert_eq!(invoke(&w, "set_settings", json!({ "settings": changed })).unwrap(), changed);
    assert_eq!(invoke(&w, "get_settings", json!({})).unwrap(), changed);
    assert_eq!(evaluate(&w, "(a+b)^2").unwrap()["result"], r"a^{2} + 2 \cdot a \cdot b + b^{2}");
    assert_eq!(invoke(&w, "engine_info", json!({})).unwrap()["precision"], 7);
}

#[test]
fn the_mode_round_trips_and_decides_whether_constants_are_known() {
    let w = window();
    let mut settings = invoke(&w, "get_settings", json!({})).unwrap();
    assert_eq!(settings["mode"], "physics");
    assert!(evaluate(&w, r"2 \cdot c_0").is_ok());

    settings["mode"] = json!("algebra");
    assert_eq!(invoke(&w, "set_settings", json!({ "settings": settings })).unwrap(), settings);
    assert_eq!(invoke(&w, "get_settings", json!({})).unwrap()["mode"], "algebra");
    assert!(evaluate(&w, r"2 \cdot c_0").is_err());
}

#[test]
fn invalid_settings_are_rejected_and_keep_the_old_ones() {
    let w = window();
    let error = invoke(&w, "set_settings", json!({ "settings": { "precision": 0 } })).unwrap_err();
    assert!(error["error"].as_str().unwrap().contains("between 1 and"), "{error}");
    assert_eq!(invoke(&w, "get_settings", json!({})).unwrap()["precision"], 50);
}

#[test]
fn settings_files_are_written_and_read_back() {
    use quantus_core::settings::{DenominatorForm, Settings};
    let dir = std::env::temp_dir().join(format!("quantus-settings-{}", std::process::id()));
    let path = dir.join("nested").join("settings.json");
    assert_eq!(super::commands::load_settings(&path).unwrap(), Settings::default());

    let settings = Settings { denominator_form: DenominatorForm::Factored, ..Settings::default() };
    super::commands::save_settings(&path, &settings).unwrap();
    assert_eq!(super::commands::load_settings(&path).unwrap(), settings);

    std::fs::write(&path, r#"{ "precision": 0 }"#).unwrap();
    assert!(super::commands::load_settings(&path).is_err());
    std::fs::remove_dir_all(dir).unwrap();
}

fn cable() -> Value {
    json!({
        "nodes": [0, 1, 2, 3],
        "elements": [
            { "id": "V1", "kind": "voltageSource", "a": 1, "b": 0, "u": "230V" },
            { "id": "W1", "kind": "wire", "a": 1, "b": 2, "material": "copper", "length": "100m", "area": "2mm^2" },
            { "id": "R1", "kind": "resistor", "a": 2, "b": 3, "r": "10\\Omega" },
            { "id": "W2", "kind": "wire", "a": 3, "b": 0, "material": "copper", "length": "100m", "area": "2mm^2" },
        ],
    })
}

#[test]
fn a_circuit_is_solved_through_the_command_at_the_set_precision() {
    let w = window();
    let mut settings = invoke(&w, "get_settings", json!({})).unwrap();
    settings["precision"] = json!(5);
    invoke(&w, "set_settings", json!({ "settings": settings })).unwrap();

    let solved = invoke(&w, "solve_circuit", json!({ "request": cable() })).unwrap();
    let wire = solved["elements"].as_array().unwrap().iter().find(|e| e["id"] == "W1").unwrap();
    assert_eq!(wire["voltage"]["result"], r"16{,}877\,\mathrm{V}");
    assert_eq!(wire["voltage"]["status"], "known");
    assert_eq!(wire["notes"], json!(["bei 20 °C"]));
    assert!(solved["reverse"].is_null());
}

#[test]
fn circuit_errors_reach_the_renderer_like_other_errors() {
    let mut request = cable();
    request["elements"][2]["r"] = json!("0\\Omega");
    request["elements"][1]["material"] = json!("unobtainium");
    let error = invoke(&window(), "solve_circuit", json!({ "request": request })).unwrap_err();
    assert_eq!(error["error"], "W1: unbekanntes Material „unobtainium“");
    assert!(error["position"].is_null());
}

fn saved_circuit() -> Value {
    json!({
        "version": 1,
        "nodes": [0, 1],
        "elements": [
            { "id": "V1", "kind": "voltageSource", "a": 1, "b": 0, "u": "12V", "x": 100, "y": 100, "rotation": 0 },
            { "id": "W1", "kind": "wire", "a": 1, "b": 0, "x": 20, "y": 40, "x2": 60, "y2": 40 },
            { "id": "GND1", "kind": "ground", "a": 0, "b": 0, "x": 0, "y": 0, "rotation": 0 },
        ],
    })
}

#[test]
fn a_circuit_is_saved_and_loaded_unchanged() {
    let w = window();
    let dir = std::env::temp_dir().join(format!("quantus-circuit-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("schaltung.json").to_string_lossy().into_owned();

    invoke(&w, "save_circuit", json!({ "path": path, "circuit": saved_circuit() })).unwrap();
    let loaded = invoke(&w, "load_circuit", json!({ "path": path })).unwrap();
    assert_eq!(loaded, saved_circuit());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_broken_circuit_file_is_reported_not_loaded() {
    let w = window();
    let dir = std::env::temp_dir().join(format!("quantus-broken-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let load = |text: &str| {
        let path = dir.join("kaputt.json");
        std::fs::write(&path, text).unwrap();
        invoke(&w, "load_circuit", json!({ "path": path.to_string_lossy() })).unwrap_err()
    };

    assert!(load("{ nicht json")["error"].as_str().unwrap().starts_with("Keine gültige Schaltungsdatei"));
    assert_eq!(
        load(r#"{ "elements": [] }"#)["error"],
        "Das ist keine Schaltungsdatei: die Version fehlt."
    );
    assert_eq!(
        load(r#"{ "version": 2, "elements": [] }"#)["error"],
        "Schaltungsdatei hat die unbekannte Version 2."
    );
    let bad_kind = r#"{ "version": 1, "elements": [{ "id": "X1", "kind": "diode", "x": 0, "y": 0 }] }"#;
    assert_eq!(load(bad_kind)["error"], "X1: unbekannte Bauteilart „diode“.");
    let no_position = r#"{ "version": 1, "elements": [{ "id": "R1", "kind": "resistor", "x": 0 }] }"#;
    assert_eq!(load(no_position)["error"], "R1: die Position (y) fehlt.");
    let twice = r#"{ "version": 1, "elements": [
        { "id": "R1", "kind": "resistor", "x": 0, "y": 0 }, { "id": "R1", "kind": "resistor", "x": 0, "y": 0 }] }"#;
    assert_eq!(load(twice)["error"], "Die ID R1 kommt mehrfach vor.");

    let missing = invoke(&w, "load_circuit", json!({ "path": dir.join("gibt-es-nicht.json").to_string_lossy() }));
    assert!(missing.unwrap_err()["error"].as_str().unwrap().contains("konnte nicht gelesen werden"));
    std::fs::remove_dir_all(dir).unwrap();
}
