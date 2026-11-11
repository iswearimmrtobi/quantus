//! calc from a script while the engine works on its own thread: the spinner, the answer later,
//! AC dropping it (apps.md §4.6).

use std::cell::Cell;
use std::rc::Rc;

use quantus_apps::ScriptApplet;
use quantus_ui::app::{App, Mode, Screen, Settings};
use quantus_ui::applet::AppInfo;
use quantus_ui::backend::{Backend, Stub};
use quantus_ui::key::Key;
use quantus_ui::render::{Frame, Shape};

/// An engine that answers on the second poll, and counts what was dropped.
struct Slow {
    polls: u32,
    dropped: Rc<Cell<u32>>,
    mode: Rc<Cell<Option<Mode>>>,
}

impl Backend for Slow {
    fn evaluate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> Result<String, String> {
        Stub.evaluate(mode, settings, latex)
    }

    fn app_calc(&mut self, mode: Mode, _settings: &Settings, _input: &str) -> Option<Result<String, String>> {
        self.mode.set(Some(mode));
        self.polls = 0;
        None
    }

    fn poll(&mut self) -> Option<Result<String, String>> {
        self.polls += 1;
        (self.polls >= 2).then(|| Ok(r"\approx 4{,}2".to_string()))
    }

    fn drop_pending(&mut self) {
        self.dropped.set(self.dropped.get() + 1);
    }
}

const SOURCE: &str = r#"
    local t = "-"
    function key(k) local r = calc("6\\cdot 0{,}7"); t = tostring(r.ok) .. " " .. r.value .. " " .. r.latex end
    function draw() text(0, 0, t, "text") end
"#;

/// The mode a calculation was asked in.
type Asked = Rc<Cell<Option<Mode>>>;

fn open() -> (App<Slow>, Rc<Cell<u32>>, Asked) {
    let (dropped, mode) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(None)));
    let mut app = App::new(Slow { polls: 0, dropped: dropped.clone(), mode: mode.clone() });
    // A mode without the engine: the app calculates in Physik.
    app.mode = Mode::Table;
    let info = AppInfo { id: "t".into(), name: "T".into(), api: 1, usable: true, ..AppInfo::default() };
    app.open_applet(Box::new(ScriptApplet::new(SOURCE)), info);
    (app, dropped, mode)
}

fn texts(app: &App<Slow>) -> Vec<String> {
    Frame::new(app).0.iter().filter_map(|shape| match shape {
        Shape::Text(_, text, ..) => Some(text.clone()),
        _ => None,
    }).collect()
}

#[test]
fn the_answer_comes_later_and_the_spinner_turns_meanwhile() {
    let (mut app, _, mode) = open();
    app.key(Key::Digit(1));
    assert_eq!(mode.get(), Some(Mode::Physics));
    app.tick(0);
    assert!(app.busy);
    // Keys wait.
    app.key(Key::Digit(2));
    assert!(texts(&app).contains(&"-".to_string()));
    app.tick(10);
    assert!(!app.busy);
    assert!(texts(&app).contains(&r"true 4{,}2 \approx 4{,}2".to_string()), "{:?}", texts(&app));
}

#[test]
fn ac_drops_the_calculation_and_leaves() {
    let (mut app, dropped, _) = open();
    app.key(Key::Digit(1));
    app.tick(0);
    app.key(Key::Ac);
    assert_eq!(dropped.get(), 1);
    assert!(!app.busy);
    assert_eq!(app.screen, Screen::Calc);
    assert!(app.running.is_none());
}

#[test]
fn the_line_with_calc_waits_for_the_engine() {
    let (dropped, mode) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(None)));
    let mut app = App::new(Slow { polls: 0, dropped, mode });
    let source = r#"
        local t = "-"
        function init() input("line", {calc = true}) end
        function entered(r) t = r.value end
        function draw() text(0, 0, t, "text") end
    "#;
    let info = AppInfo { id: "t".into(), name: "T".into(), api: 1, usable: true, ..AppInfo::default() };
    app.open_applet(Box::new(ScriptApplet::new(source)), info);
    app.key(Key::Digit(4));
    app.key(Key::Exe);
    app.tick(0);
    assert!(app.busy);
    app.tick(10);
    assert!(texts(&app).contains(&"4{,}2".to_string()), "{:?}", texts(&app));
}
