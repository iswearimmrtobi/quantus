//! The commands through Tauri's own IPC layer, as the renderer's `invoke` reaches them.

use quantus_core::settings::Language;
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
    // The app's own origin; any other one is refused by the permission check.
    let origin = if cfg!(windows) { "http://tauri.localhost" } else { "tauri://localhost" };
    let request = InvokeRequest {
        cmd: command.into(),
        callback: tauri::ipc::CallbackFn(0),
        error: tauri::ipc::CallbackFn(1),
        url: origin.parse().unwrap(),
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
    // At the default detail, numbers and units are multiplied before the unit gets its name.
    assert_eq!(kinds, ["symbolic", "algebraic", "result"]);
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
        4,
        "variables add the substituted step, before numbers and units are multiplied"
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
    for language in [Language::En, Language::De] {
        let menu = super::shell::build_menu(app.handle(), language).unwrap();
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
    assert_eq!(wire["notes"], json!(["at 20 °C"]));
    assert!(solved["reverse"].is_null());
}

#[test]
fn circuit_errors_reach_the_renderer_like_other_errors() {
    let mut request = cable();
    request["elements"][2]["r"] = json!("0\\Omega");
    request["elements"][1]["material"] = json!("unobtainium");
    let error = invoke(&window(), "solve_circuit", json!({ "request": request })).unwrap_err();
    assert_eq!(error["error"], "W1: unknown material “unobtainium”");
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

    assert!(load("{ nicht json")["error"].as_str().unwrap().starts_with("Not a valid circuit file"));
    assert_eq!(
        load(r#"{ "elements": [] }"#)["error"],
        "This is not a circuit file: the version is missing."
    );
    assert_eq!(
        load(r#"{ "version": 2, "elements": [] }"#)["error"],
        "The circuit file has the unknown version 2."
    );
    let bad_kind = r#"{ "version": 1, "elements": [{ "id": "X1", "kind": "diode", "x": 0, "y": 0 }] }"#;
    assert_eq!(load(bad_kind)["error"], "X1: unknown kind of part “diode”.");
    let no_position = r#"{ "version": 1, "elements": [{ "id": "R1", "kind": "resistor", "x": 0 }] }"#;
    assert_eq!(load(no_position)["error"], "R1: the position (y) is missing.");
    let twice = r#"{ "version": 1, "elements": [
        { "id": "R1", "kind": "resistor", "x": 0, "y": 0 }, { "id": "R1", "kind": "resistor", "x": 0, "y": 0 }] }"#;
    assert_eq!(load(twice)["error"], "The ID R1 occurs more than once.");

    let missing = invoke(&w, "load_circuit", json!({ "path": dir.join("gibt-es-nicht.json").to_string_lossy() }));
    assert!(missing.unwrap_err()["error"].as_str().unwrap().contains("could not be read"));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn circuit_file_errors_follow_the_language() {
    let dir = std::env::temp_dir().join(format!("quantus-german-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("leer.json");
    std::fs::write(&path, r#"{ "elements": [] }"#).unwrap();
    let message = |language| super::commands::read_circuit(&path, language).unwrap_err().error;
    assert_eq!(message(Language::En), "This is not a circuit file: the version is missing.");
    assert_eq!(message(Language::De), "Das ist keine Schaltungsdatei: die Version fehlt.");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_function_is_analysed_through_the_command_and_the_mode_round_trips() {
    // German texts are covered in quantus-core: switching the language here would rebuild the
    // menu, which the mock runtime cannot.
    let w = window();
    let sketch = invoke(&w, "analyze", json!({ "request": { "latex": "f(x) = x^3 - 3x" } })).unwrap();
    assert_eq!(sketch["function"], "f(x) = x^{3} - 3x");
    assert_eq!(sketch["sections"].as_array().unwrap().len(), 11);
    assert_eq!(sketch["sections"][1]["title"], "Domain");
    let max = sketch["points"].as_array().unwrap().iter().find(|p| p["kind"] == "max").unwrap().clone();
    assert_eq!((max["name"].clone(), max["x"].clone()), (json!("Max"), json!(-1.0)));

    let mut settings = invoke(&w, "get_settings", json!({})).unwrap();
    settings["mode"] = json!("analysis");
    settings["quadraticFormula"] = json!("pq");
    let saved = invoke(&w, "set_settings", json!({ "settings": settings })).unwrap();
    assert_eq!((saved["mode"].clone(), saved["quadraticFormula"].clone()), (json!("analysis"), json!("pq")));
    // Analysis mode reads like algebra mode: no physical constants.
    assert!(evaluate(&w, r"2 \cdot c_0").is_err());
}

#[test]
fn the_analysis_tasks_reach_the_renderer() {
    let w = window();
    let sampled = invoke(&w, "sample_function", json!({ "request": { "latex": "\\tan x", "xMin": -4.0, "xMax": 4.0, "samples": 200, "yMin": -5.0, "yMax": 5.0 } })).unwrap();
    assert_eq!(sampled["segments"].as_array().unwrap().len(), 3);
    let tangent = invoke(&w, "tangent", json!({ "request": { "latex": "x^2", "x0": "0" } })).unwrap();
    assert_eq!(tangent["normal"]["kind"], "vertical");
    let area = invoke(&w, "area_under_curve", json!({ "request": { "latex": "x^2", "from": "0", "to": "3" } })).unwrap();
    assert_eq!(area["area"], "9");
    let table = invoke(&w, "value_table", json!({ "request": { "latex": "\\ln x", "from": "0", "to": "1", "step": "1" } })).unwrap();
    assert_eq!(
        table["rows"],
        json!([{ "x": "0", "y": null, "xValue": 0.0, "yValue": null }, { "x": "1", "y": "0", "xValue": 1.0, "yValue": 0.0 }])
    );
}

#[test]
fn analysis_errors_reach_the_renderer_like_other_errors() {
    let w = window();
    let parse = invoke(&w, "analyze", json!({ "request": { "latex": "f(x) = x^2 +" } })).unwrap_err();
    assert!(parse["position"].as_u64().is_some());
    let improper = invoke(&w, "area_under_curve", json!({ "request": { "latex": "\\frac{1}{x}", "from": "-1", "to": "1" } })).unwrap_err();
    assert!(improper["error"].as_str().unwrap().contains("Improper integral"));
    assert!(improper["position"].is_null());
    let tangent = invoke(&w, "tangent", json!({ "request": { "latex": "\\ln x", "x0": "-1" } })).unwrap_err();
    assert!(tangent["error"].as_str().unwrap().contains("not differentiable"));
    let table = invoke(&w, "value_table", json!({ "request": { "latex": "x", "from": "0", "to": "1", "step": "-1" } })).unwrap_err();
    assert!(table["error"].as_str().unwrap().contains("step"));
    let sample = invoke(&w, "sample_function", json!({ "request": { "latex": "x^2 + a", "xMin": -1.0, "xMax": 1.0, "samples": 10 } })).unwrap_err();
    assert!(sample["error"].as_str().unwrap().contains("function families"));
}

#[test]
fn the_analysis_follows_the_set_precision() {
    let w = window();
    let mut settings = invoke(&w, "get_settings", json!({})).unwrap();
    settings["precision"] = json!(3);
    invoke(&w, "set_settings", json!({ "settings": settings })).unwrap();
    let sketch = invoke(&w, "analyze", json!({ "request": { "latex": "x^3 - 3x" } })).unwrap();
    let zero = sketch["points"].as_array().unwrap().iter().find(|p| p["kind"] == "zero").unwrap().clone();
    assert!(zero["latex"].as_str().unwrap().ends_with("\\approx Z_{1}\\left(-1{,}73 \\mid 0\\right)"), "{}", zero["latex"]);
    let precise = invoke(&w, "analyze", json!({ "request": { "latex": "x^3 - 3x", "precision": 8 } })).unwrap();
    assert!(precise["points"][0]["latex"].as_str().unwrap().contains("-1{,}7320508"));
    let area = invoke(&w, "area_under_curve", json!({ "request": { "latex": "e^{x^2}", "from": "0", "to": "1" } })).unwrap();
    assert_eq!(area["area"], "\\approx 1{,}46");
}

#[test]
fn a_linear_system_is_solved_through_the_command_without_session_variables() {
    let w = window();
    // A stored k_1 is ignored by the linear systems mode: it stays an unknown.
    invoke(&w, "evaluate", json!({ "request": { "latex": "k_1 = 5", "sessionId": "desktop" } })).unwrap();
    let typed = invoke(&w, "solve_linear_system", json!({ "request": { "equations": "k_1 + x = 3; k_1 - x = 1" } })).unwrap();
    assert_eq!(typed["result"], "x = 1 \\land k_{1} = 2");
    assert_eq!(typed["matrix"], json!([["1", "1", "3"], ["-1", "1", "1"]]));

    let mut settings = invoke(&w, "get_settings", json!({})).unwrap();
    settings["mode"] = json!("linear");
    settings["unknownNames"] = json!("letters");
    let saved = invoke(&w, "set_settings", json!({ "settings": settings })).unwrap();
    assert_eq!((saved["mode"].clone(), saved["linearComplex"].clone()), (json!("linear"), json!(false)));
    let given = invoke(&w, "solve_linear_system", json!({ "request": { "matrix": { "rows": [["2", "1", "5"], ["1", "-1", ""]] } } })).unwrap();
    assert_eq!(given["equations"], json!(["2x + y = 5", "x - y = 0"]));
    assert_eq!(given["result"], "x = \\frac{5}{3} \\land y = \\frac{5}{3}");
    assert!(given["alignBlock"].as_str().unwrap().starts_with("\\begin{align*}"));

    let error = invoke(&w, "solve_linear_system", json!({ "request": { "matrix": { "rows": [["2", "m", "5"]] } } })).unwrap_err();
    assert!(error["error"].as_str().unwrap().starts_with("Row 1, entry 2:"));
}

/// The circuits in `test/circuits/`, loaded and solved the way the editor does it: the file goes
/// through `load_circuit`, the ground symbols stay behind and the rest is the request.
#[test]
fn the_test_circuits_solve_to_their_documented_values() {
    let w = window();
    let mut settings = invoke(&w, "get_settings", json!({})).unwrap();
    settings["precision"] = json!(6);
    invoke(&w, "set_settings", json!({ "settings": settings })).unwrap();
    let solve = |name: &str| {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test/circuits").join(format!("{name}.json"));
        let mut request = invoke(&w, "load_circuit", json!({ "path": path })).unwrap();
        request["elements"].as_array_mut().unwrap().retain(|e| e["kind"] != "ground");
        invoke(&w, "solve_circuit", json!({ "request": request }))
    };
    // (circuit, element, quantity, result)
    let expected = [
        ("divider", "R1", "current", r"4\,\mathrm{mA}"),
        ("divider", "R2", "voltage", r"8\,\mathrm{V}"),
        ("parallel", "R1", "current", r"100\,\mathrm{mA}"),
        ("parallel", "R2", "current", r"25\,\mathrm{mA}"),
        ("parallel", "V1", "current", r"-125\,\mathrm{mA}"),
        ("precision", "R1", "current", r"3{,}33333\,\mathrm{A}"),
        ("current-source", "R1", "voltage", r"8\,\mathrm{V}"),
        ("current-source", "R1", "current", r"1{,}6\,\mathrm{A}"),
        ("current-source", "R2", "current", r"400\,\mathrm{mA}"),
        ("potentiometer", "R1", "voltage", r"6{,}31579\,\mathrm{V}"),
        ("two-sources", "R3", "voltage", r"6{,}25\,\mathrm{V}"),
        ("two-sources", "R1", "current", r"3{,}75\,\mathrm{mA}"),
        ("two-sources", "R2", "current", r"625\,\mathrm{\mu A}"),
        ("wheatstone", "R5", "current", r"0\,\mathrm{A}"),
        ("wheatstone", "R1", "voltage", r"3{,}33333\,\mathrm{V}"),
        ("cable", "L1", "power", r"2\,\mathrm{kW}"),
        ("cable", "C1", "voltage", r"6{,}33116\,\mathrm{V}"),
        ("wheatstone", "R5", "resistance", r"500\,\mathrm{\Omega }"),
        ("cable", "L1", "resistance", r"23{,}6178\,\mathrm{\Omega }"),
        ("cable", "C1", "resistance", r"688\,\mathrm{m\Omega }"),
        ("relative", "R1", "voltage", r"25\,\%"),
        ("relative", "R2", "voltage", r"75\,\%"),
    ];
    for (circuit, id, quantity, result) in expected {
        let solved = solve(circuit).unwrap();
        let element = solved["elements"].as_array().unwrap().iter().find(|e| e["id"] == id).unwrap();
        assert_eq!(element[quantity]["result"], result, "{circuit}: {quantity} of {id}");
    }
    assert_eq!(solve("relative").unwrap()["elements"][2]["voltage"]["status"], "relative");
    assert!(solve("divider").unwrap()["elements"][0]["resistance"].is_null(), "a source has no resistance");
    assert_eq!(solve("short-circuit").unwrap_err()["error"], "short circuit");
}

/// One source and one resistor in a square, as the editor sends it: terminal points in grid steps.
fn square(reversed: bool) -> Value {
    json!({ "elements": [
        { "id": "V1", "kind": "voltageSource", "u": "10V", "pa": { "x": 0, "y": 4 }, "pb": { "x": 0, "y": 0 }, "reversed": reversed },
        { "id": "R1", "kind": "resistor", "r": "5\\Omega", "pa": { "x": 0, "y": 0 }, "pb": { "x": 4, "y": 0 } },
        { "id": "W1", "kind": "wire", "pa": { "x": 4, "y": 0 }, "pb": { "x": 4, "y": 4 } },
        { "id": "W2", "kind": "wire", "pa": { "x": 4, "y": 4 }, "pb": { "x": 0, "y": 4 } },
        { "id": "GND1", "kind": "ground", "pa": { "x": 0, "y": 4 }, "pb": { "x": 0, "y": 4 } },
    ] })
}

#[test]
fn a_kirchhoff_network_is_solved_through_the_command() {
    let w = window();
    let solved = invoke(&w, "solve_kirchhoff", json!({ "request": square(false) })).unwrap();
    assert_eq!(solved["result"], json!(["I_{1} = 2\\,\\mathrm{A}"]));
    assert_eq!(solved["loops"][0]["name"], "L_{1}");
    assert_eq!(solved["branches"][0]["lead"], "V1");
    assert!(solved["error"].is_null());
    let flipped = invoke(&w, "solve_kirchhoff", json!({ "request": square(true) })).unwrap();
    assert_eq!(flipped["result"], json!(["I_{1} = -2\\,\\mathrm{A}"]));

    let mut missing = square(false);
    missing["elements"][1]["r"] = json!("");
    let error = invoke(&w, "solve_kirchhoff", json!({ "request": missing })).unwrap_err();
    assert!(error["error"].as_str().unwrap().contains("R1"), "{error}");
}

#[test]
fn a_circuit_file_may_carry_a_flipped_current() {
    let mut circuit = saved_circuit();
    circuit["elements"][0]["reversed"] = json!(true);
    assert!(super::commands::validate_circuit(&circuit, Language::En).is_ok());
    circuit["elements"][0]["reversed"] = json!("yes");
    assert_eq!(
        super::commands::validate_circuit(&circuit, Language::En).unwrap_err().error,
        "V1: “reversed” must be true or false."
    );
}
