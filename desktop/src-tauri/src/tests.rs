//! The commands through Tauri's own IPC layer, as the renderer's `invoke` reaches them.

use serde_json::{json, Value};
use tauri::test::{mock_builder, mock_context, noop_assets, MockRuntime};
use tauri::webview::InvokeRequest;
use tauri::{WebviewWindow, WebviewWindowBuilder};

fn window() -> WebviewWindow<MockRuntime> {
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
