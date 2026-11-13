use quantus_core::api::{self, CalculationRequest, LinearSystemRequest, ModeInputs, NewSessionRequest};
use quantus_core::session::{default_name, SessionStore};
use quantus_core::settings::{Language, Mode, Settings};

fn run(store: &mut SessionStore, id: &str, latex: &str) -> String {
    let request = CalculationRequest { latex: latex.into(), session_id: Some(id.into()), at: Some(1_000), ..Default::default() };
    api::evaluate(store, &request).map(|r| r.result).unwrap_or_else(|e| format!("error: {e}"))
}

#[test]
fn a_new_session_is_named_after_the_time() {
    // 2026-11-01 13:30 UTC is 14:30 in Berlin (CET, +60 minutes).
    let now = 1_793_539_800_000;
    assert_eq!(default_name(Some(now), 60, Language::De), "Sitzung 2026-11-01 14:30");
    assert_eq!(default_name(None, 0, Language::En), "Session");
    let mut store = SessionStore::new();
    let info = api::new_session(&mut store, &NewSessionRequest { now: Some(now), utc_offset_minutes: 60, ..Default::default() });
    assert_eq!(info.name, "Session 2026-11-01 14:30");
    assert_eq!(info.id, "session-1");
    let second = api::new_session(&mut store, &NewSessionRequest { name: Some("Physik".into()), ..Default::default() });
    assert_eq!((second.id.as_str(), second.name.as_str()), ("session-2", "Physik"));
}

#[test]
fn a_session_round_trips_through_json_with_exact_values() {
    let mut store = SessionStore::new();
    let id = api::new_session(&mut store, &NewSessionRequest { name: Some("Test".into()), now: Some(5), ..Default::default() }).id;
    run(&mut store, &id, r"r_1 = \frac{1}{3}\,\mathrm{m}");
    run(&mut store, &id, r"r_2 = 3 r_1");
    run(&mut store, &id, r"E_1 = m_e c_0^2");
    store.settings = Settings { mode: Mode::Matrix, ..Settings::default() };
    run(&mut store, &id, r"A = \begin{pmatrix} 1 & \frac{1}{2} \\ 0 & 1 \end{pmatrix}");
    store.settings = Settings::default();
    let inputs = ModeInputs { linear: Some(LinearSystemRequest::Equations("x + y = 2".into())), ..Default::default() };
    api::set_mode_inputs(&mut store, Some(&id), inputs);
    let json = api::export_session(&mut store, Some(&id));
    assert!(json.contains("\"version\": 1"), "{json}");

    let mut other = SessionStore::new();
    let info = api::import_session(&mut other, &json).unwrap();
    assert_eq!((info.id.as_str(), info.name.as_str(), info.entries), (id.as_str(), "Test", 4));
    // r_2 is exactly 1 m again, worked out from its definition.
    assert_eq!(run(&mut other, &id, r"r_2"), r"1\,\mathrm{m}");
    assert_eq!(run(&mut other, &id, r"\frac{r_1}{r_2}"), "0{,}33333333333333333333333333333333333333333333333333");
    other.settings = Settings { mode: Mode::Matrix, ..Settings::default() };
    assert_eq!(run(&mut other, &id, "A^{2}"), r"\begin{pmatrix} 1 & 1 \\ 0 & 1 \end{pmatrix}");
    assert!(matches!(api::get_mode_inputs(&mut other, Some(&id)).linear, Some(LinearSystemRequest::Equations(_))));
    // Importing it again gives a new id.
    let again = api::import_session(&mut other, &json).unwrap();
    assert_ne!(again.id, id);
}

#[test]
fn the_history_is_kept_and_capped() {
    let mut store = SessionStore::new();
    for n in 0..205 {
        run(&mut store, "default", &format!("{n} + 1"));
    }
    let history = api::history(&mut store, None);
    assert_eq!(history.len(), 200);
    assert_eq!(history[0].input, "5 + 1");
    assert_eq!(history[199].response.result, "205");
    api::clear_history(&mut store, None);
    assert!(api::history(&mut store, None).is_empty());
}

#[test]
fn rename_duplicate_delete_and_newer_formats() {
    let mut store = SessionStore::new();
    let id = api::new_session(&mut store, &NewSessionRequest { name: Some("A".into()), ..Default::default() }).id;
    run(&mut store, &id, "x_1 = 2");
    api::rename_session(&mut store, &id, "B").unwrap();
    let copy = api::duplicate_session(&mut store, &id, Some(9)).unwrap();
    assert_eq!(copy.name, "B (copy)");
    assert_eq!(run(&mut store, &copy.id, "x_1 \\cdot 3"), "6");
    assert!(api::delete_session(&mut store, &id));
    assert!(api::rename_session(&mut store, &id, "C").is_err());
    let newer = api::export_session(&mut store, Some(&copy.id)).replace("\"version\": 1", "\"version\": 7");
    assert!(api::import_session(&mut store, &newer).unwrap_err().error.contains("newer Quantus"));
}

#[test]
fn ans_and_pre_ans_are_the_last_two_results() {
    let mut store = SessionStore::new();
    run(&mut store, "default", r"2\,\mathrm{m} \cdot 3");
    run(&mut store, "default", "10");
    // Ans is 10, PreAns 6 m: they do not add.
    assert!(run(&mut store, "default", r"\mathrm{Ans} + \mathrm{PreAns}").starts_with("error: Incompatible units"));
    let mut store = SessionStore::new();
    run(&mut store, "default", "4");
    run(&mut store, "default", "6");
    assert_eq!(run(&mut store, "default", r"\mathrm{Ans} \cdot \mathrm{PreAns}"), "24");
    // Ans is now 24, PreAns 6.
    assert_eq!(run(&mut store, "default", r"\operatorname{Ans} - \operatorname{PreAns}"), "18");
    // An algebraic result leaves them alone.
    store.settings = Settings { mode: Mode::Algebra, ..Settings::default() };
    run(&mut store, "default", "a + a");
    assert_eq!(run(&mut store, "default", r"\mathrm{Ans}"), "18");
    // Saved with the session.
    let json = api::export_session(&mut store, None);
    let mut other = SessionStore::new();
    let id = api::import_session(&mut other, &json).unwrap().id;
    assert_eq!(run(&mut other, &id, r"\mathrm{PreAns}"), "18");
}

#[test]
fn the_geometry_scene_is_kept_per_session_apart_from_the_variables() {
    use quantus_core::api::GeometryRequest;
    let mut store = SessionStore::new();
    let id = api::new_session(&mut store, &NewSessionRequest { name: Some("Geo".into()), ..Default::default() }).id;
    run(&mut store, &id, "A_1 = 5");
    let scene: GeometryRequest = serde_json::from_value(serde_json::json!({
        "scene": { "objects": [
            { "name": "A_1", "kind": "point", "at": ["1", "2", "3"] },
            { "kind": "typed", "latex": "E: x + y + z = 1" }
        ] },
        "task": { "kind": "distance", "a": "A_1", "b": "E" }
    }))
    .unwrap();
    api::set_mode_inputs(&mut store, Some(&id), ModeInputs { geometry: Some(scene.clone()), ..Default::default() });

    let mut other = SessionStore::new();
    api::import_session(&mut other, &api::export_session(&mut store, Some(&id))).unwrap();
    let kept = api::get_mode_inputs(&mut other, Some(&id)).geometry.unwrap();
    assert_eq!(kept, scene);
    // The point A_1 is not the variable A_1, and the variable list has no point in it.
    assert_eq!(run(&mut other, &id, "A_1"), "5");
    assert_eq!(api::list_variables(&mut other, Some(&id)).len(), 1);
    let answer = api::geometry(&other.settings, &kept).unwrap().task.unwrap();
    assert_eq!(answer.result.last().unwrap(), r"d = \frac{5\sqrt{3}}{3}");
    // Another session has no scene.
    assert!(api::get_mode_inputs(&mut other, Some("fresh")).geometry.is_none());
}

#[test]
fn the_logic_scene_is_kept_per_session() {
    use quantus_core::api::LogicRequest;
    let mut store = SessionStore::new();
    let id = api::new_session(&mut store, &NewSessionRequest { name: Some("Logik".into()), ..Default::default() }).id;
    let scene: LogicRequest = serde_json::from_value(serde_json::json!({
        "scene": {
            "inputs": ["A", "B"],
            "outputs": [
                { "name": "f", "source": { "kind": "table", "values": ["0", "1", "1", "-"] }, "chosen": 0 },
                { "source": { "kind": "expression", "latex": r"g = A \oplus B" } }
            ],
            "row": 2,
            "editor": { "generated": true, "positions": { "G1": [4, 2] } }
        },
        "task": { "kind": "checkForm", "output": "f", "latex": "A + B" }
    }))
    .unwrap();
    api::set_mode_inputs(&mut store, Some(&id), ModeInputs { logic: Some(scene.clone()), ..Default::default() });

    let mut other = SessionStore::new();
    api::import_session(&mut other, &api::export_session(&mut store, Some(&id))).unwrap();
    let kept = api::get_mode_inputs(&mut other, Some(&id)).logic.unwrap();
    assert_eq!(kept, scene);
    let answer = api::logic(&other.settings, &kept).unwrap().task.unwrap();
    assert_eq!((answer.equal, answer.minimal), (Some(true), Some(true)));
    assert!(api::get_mode_inputs(&mut other, Some("fresh")).logic.is_none());
}
