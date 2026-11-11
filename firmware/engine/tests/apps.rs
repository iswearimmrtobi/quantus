//! Apps against the real engine (apps.md §4.6): `calc` leaves the history alone.

use quantus_engine::Engine;
use quantus_ui::app::{Mode, Settings};
use quantus_ui::backend::{Backend, SessionCommand};

/// How many calculations the open session has.
fn history(engine: &mut Engine) -> usize {
    engine.session(SessionCommand::Current, &Settings::default()).map(|opened| opened.history.len()).unwrap_or(0)
}

#[test]
fn app_calc_works_out_without_the_history() {
    let mut engine = Engine::default();
    let settings = Settings::default();
    engine.evaluate(Mode::Physics, &settings, "1+1").unwrap();
    let before = history(&mut engine);
    assert_eq!(engine.app_calc(Mode::Algebra, &settings, r"\frac{1}{2}+\frac{1}{3}"), Some(Ok(r"\frac{5}{6}".to_string())));
    assert!(matches!(engine.app_calc(Mode::Algebra, &settings, "1+"), Some(Err(_))));
    assert_eq!(history(&mut engine), before);
}

use quantus_apps::ScriptApplet;
use quantus_ui::app::App;
use quantus_ui::applet::AppInfo;
use quantus_ui::key::Key;
use quantus_ui::render::{Frame, Shape};

fn texts(app: &App<Engine>) -> Vec<String> {
    Frame::new(app).0.iter().filter_map(|shape| match shape {
        Shape::Text(_, text, ..) => Some(text.clone()),
        _ => None,
    }).collect()
}

#[test]
fn a_script_adds_two_lengths_and_the_history_stays() {
    let mut app = App::new(Engine::default());
    app.key(Key::Digit(7));
    app.key(Key::Exe);
    let history = app.history.len();
    let source = r#"
        fn init() { this.t = ""; }
        fn key(k) {
            let a = calc("3\\,\\mathrm{m}+20\\,\\mathrm{cm}");
            let b = calc("1+");
            let c = calc(a.value + "\\cdot 2");
            this.t = `${a.ok} ${a.latex}|${b.ok} ${b.error != ()}|${c.latex}`;
        }
        fn draw() { text(0, 0, this.t, "text"); }
    "#;
    let info = AppInfo { id: "t".into(), name: "T".into(), api: 1, usable: true, ..AppInfo::default() };
    app.open_applet(Box::new(ScriptApplet::new(source)), info);
    app.key(Key::Digit(1));
    let shown = texts(&app);
    let line = shown.iter().find(|text| text.starts_with("true")).unwrap_or_else(|| panic!("{shown:?}"));
    let (sum, rest) = line.split_once('|').unwrap();
    assert!(sum.contains(r"\mathrm{m}") && (sum.contains("16") || sum.contains("3{,}2")), "{line}");
    assert!(rest.starts_with("false true|"), "{line}");
    assert!(rest.contains(r"\mathrm{m}"), "{line}");
    app.key(Key::Ac);
    assert_eq!(app.history.len(), history);
}

#[test]
fn var_reads_the_sessions_variables_and_insert_hands_back() {
    use quantus_ui::backend::VariableCommand;
    let settings = Settings::default();
    let store = VariableCommand::Store { mode: Mode::Physics, key: "a".into(), latex: "5".into() };
    // Through the app's backend, as STO does.
    let mut engine = Engine::default();
    engine.variables(store.clone(), &settings).expect("stored");
    let mut app = App::new(engine);
    let source = r#"
        fn init() { this.t = `${variable("a")} ${variable("nothing")}`; }
        fn key(k) { insert("\\frac{1}{2}"); }
        fn draw() { text(0, 0, this.t, "text"); }
    "#;
    let info = AppInfo { id: "t".into(), name: "T".into(), api: 1, usable: true, ..AppInfo::default() };
    app.open_applet(Box::new(ScriptApplet::new(source)), info);
    assert!(texts(&app).contains(&"5 ".to_string()), "{:?}", texts(&app));
    app.key(Key::Digit(1));
    // Left, back in the calculation, the value at its cursor.
    assert_eq!(app.screen, quantus_ui::app::Screen::Calc);
    assert_eq!(app.editor.to_latex(), r"\frac{1}{2}");
}
