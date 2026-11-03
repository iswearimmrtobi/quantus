//! Kurvendiskussion: EXE sketches the function on the input line, its pages turn with ←/→, and
//! OPTN opens its tasks.

use std::cell::RefCell;
use std::rc::Rc;

use quantus_ui::app::{App, Mode, Settings};
use quantus_ui::backend::{Backend, Reply, Request, Stub};
use quantus_ui::key::Key;

/// Keeps what it was asked and answers as the stub does, now or later.
#[derive(Clone, Default)]
struct Asked(Rc<RefCell<State>>);

#[derive(Default)]
struct State {
    asked: Vec<Request>,
    later: bool,
    ready: Option<Result<Reply, String>>,
}

impl Backend for Asked {
    fn evaluate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> Result<String, String> {
        Stub.evaluate(mode, settings, latex)
    }

    fn start_solve(&mut self, settings: &Settings, request: &Request) -> Option<Result<Reply, String>> {
        let reply = Stub.solve(settings, request);
        let mut state = self.0.borrow_mut();
        state.asked.push(request.clone());
        if state.later {
            state.ready = Some(reply);
            return None;
        }
        Some(reply)
    }

    fn poll_solve(&mut self) -> Option<Result<Reply, String>> {
        self.0.borrow_mut().ready.take()
    }

    fn drop_pending(&mut self) {
        self.0.borrow_mut().ready = None;
    }
}

fn app() -> (App<Asked>, Asked) {
    let backend = Asked::default();
    let mut app = App::new(backend.clone());
    app.mode = Mode::Analysis;
    (app, backend)
}

fn type_text(app: &mut App<Asked>, text: &str) {
    for c in text.chars() {
        app.key(Key::from_char(c).expect("a key"));
    }
}

#[test]
fn exe_sketches_the_function_and_the_pages_turn() {
    let (mut app, backend) = app();
    type_text(&mut app, "x^2");
    app.key(Key::Exe);
    assert_eq!(backend.0.borrow().asked, [Request::Analysis { f: "x^{2}".into() }]);
    let answer = app.answer.as_ref().expect("the sketch is shown");
    // The graph, then the stub's two sections.
    assert_eq!((answer.page, answer.pages()), (0, 3));
    app.key(Key::Right);
    app.key(Key::Right);
    app.key(Key::Right);
    assert_eq!(app.answer.as_ref().unwrap().page, 2);
    app.key(Key::Left);
    assert_eq!(app.answer.as_ref().unwrap().page, 1);
    // AC goes back to the input, as it was.
    app.key(Key::Ac);
    assert!(app.answer.is_none());
    assert_eq!(app.editor.to_latex(), "x^{2}");
    // Nothing went into the history.
    assert!(app.history.is_empty());
}

#[test]
fn a_key_to_type_leaves_the_sketch_and_types() {
    let (mut app, _) = app();
    type_text(&mut app, "x");
    app.key(Key::Exe);
    app.key(Key::Plus);
    assert!(app.answer.is_none());
    assert_eq!(app.editor.to_latex(), "x+");
}

#[test]
fn the_spinner_turns_while_the_sketch_is_worked_out_and_ac_drops_it() {
    let (mut app, backend) = app();
    backend.0.borrow_mut().later = true;
    type_text(&mut app, "x");
    app.key(Key::Exe);
    assert!(app.busy && app.answer.is_none());
    app.key(Key::Ac);
    assert!(!app.busy);
    app.tick(1000);
    assert!(app.answer.is_none());

    app.key(Key::Exe);
    app.tick(2000);
    assert!(app.answer.is_some() && !app.busy);
}

#[test]
fn another_mode_leaves_the_sketch() {
    let (mut app, _) = app();
    type_text(&mut app, "x");
    app.key(Key::Exe);
    app.key(Key::Mode);
    app.key(Key::Digit(2));
    assert_eq!(app.mode, Mode::Algebra);
    assert!(app.answer.is_none());
}

/// Every text drawn on the screen.
fn texts(app: &App<Asked>) -> Vec<String> {
    let frame = quantus_ui::render::Frame::new(app);
    let debug = format!("{:?}", frame.0);
    debug.split("Text(").skip(1).filter_map(|part| part.split('"').nth(1).map(String::from)).collect()
}

#[test]
fn a_section_page_shows_its_title_and_where_it_stands() {
    let (mut app, _) = app();
    type_text(&mut app, "x");
    app.key(Key::Exe);
    assert!(texts(&app).iter().any(|text| text == "1/3"));
    app.key(Key::Right);
    let shown = texts(&app);
    assert!(shown.iter().any(|text| text == "f(x)") && shown.iter().any(|text| text == "2/3"), "{shown:?}");
    // The stub's second section failed: it says so, and why.
    app.key(Key::Right);
    let shown = texts(&app);
    assert!(shown.iter().any(|text| text == "fehlgeschlagen") && shown.iter().any(|text| text == "Stub"), "{shown:?}");
}
