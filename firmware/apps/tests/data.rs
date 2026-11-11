//! The saved data of an app (apps.md §2): JSON and back, the limit, kept across opening it again.

use quantus_apps::json;
use quantus_apps::{ScriptApplet, Value};
use quantus_ui::app::App;
use quantus_ui::applet::AppInfo;
use quantus_ui::backend::Stub;
use quantus_ui::key::Key;
use quantus_ui::render::{Frame, Shape};

#[test]
fn nested_values_go_to_json_and_back() {
    let value = Value::Map(vec![
        ("best".into(), Value::Int(2048)),
        ("board".into(), Value::Array(vec![Value::Array(vec![Value::Int(2), Value::Unit]), Value::Array(vec![])])),
        ("name".into(), Value::Str("Zoë \"Ä\"\n\\".into())),
        ("ratio".into(), Value::Float(0.5)),
        ("whole".into(), Value::Float(3.0)),
        ("on".into(), Value::Bool(true)),
        ("empty".into(), Value::Map(vec![])),
    ]);
    let text = json::write(&value);
    assert_eq!(json::read(&text), Some(value));
    assert_eq!(json::read(" { \"a\" : [ 1 , -2.5e1 ] } "), Some(Value::Map(vec![("a".into(), Value::Array(vec![Value::Int(1), Value::Float(-25.0)]))])));
    for bad in ["", "{", "[1,]", "{\"a\" 1}", "nul", "1 2", &"[".repeat(100)] {
        assert_eq!(json::read(bad), None, "{bad}");
    }
}

fn open(app: &mut App<Stub>, source: &str) {
    let info = AppInfo { id: "keeper".into(), name: "Keeper".into(), api: 1, usable: true, ..AppInfo::default() };
    app.open_applet(Box::new(ScriptApplet::new(source)), info);
}

fn texts(app: &App<Stub>) -> Vec<String> {
    Frame::new(app).0.iter().filter_map(|shape| match shape {
        Shape::Text(_, text, ..) => Some(text.clone()),
        _ => None,
    }).collect()
}

const KEEPER: &str = r#"
    local n
    function init() local d = load(); n = d and d.n or 0 end
    function key(k) n = n + 1; save({n = n, list = {1, 2.5, "x", true}}) end
    function draw() text(0, 0, "n " .. n, "text") end
"#;

#[test]
fn the_data_outlast_the_app() {
    // The stub keeps no data: the hui does, until it is switched off.
    let mut app = App::new(Stub);
    open(&mut app, KEEPER);
    app.key(Key::Digit(1));
    app.key(Key::Digit(1));
    assert!(texts(&app).contains(&"n 2".to_string()));
    app.key(Key::Ac);
    open(&mut app, KEEPER);
    assert!(texts(&app).contains(&"n 2".to_string()), "{:?}", texts(&app));
}

#[test]
fn over_64_kb_is_an_error_of_the_script() {
    let mut app = App::new(Stub);
    open(&mut app, r#"function key(k) local a = {}; for i = 1, 9000 do a[i] = 1234567 end; save(a) end function draw() text(0, 0, "drawn", "text") end"#);
    app.key(Key::Digit(1));
    let shown = texts(&app);
    assert!(shown.iter().any(|text| text.contains("64 KB")), "{shown:?}");
}
