//! A backend that calculates elsewhere, like the engine on its own thread on the board: the app
//! turns the spinner until the answer comes, and AC drops it.

use std::cell::RefCell;
use std::rc::Rc;

use quantus_ui::app::{App, Mode, Settings};
use quantus_ui::backend::Backend;
use quantus_ui::key::Key;

/// Answers only when the test says so, through `answer`.
#[derive(Clone, Default)]
struct Later(Rc<RefCell<State>>);

#[derive(Default)]
struct State {
    /// What was started and not yet answered or dropped.
    asked: Option<String>,
    /// The answer `poll` hands out next.
    ready: Option<Result<String, String>>,
    started: usize,
    dropped: usize,
}

impl Later {
    /// Answers the calculation that runs now with what it was asked, as the stub does.
    fn answer(&self) {
        let mut state = self.0.borrow_mut();
        let latex = state.asked.take().expect("a calculation runs");
        state.ready = Some(Ok(latex));
    }
}

impl Backend for Later {
    fn evaluate(&mut self, _mode: Mode, _settings: &Settings, latex: &str) -> Result<String, String> {
        Ok(latex.to_string())
    }

    fn start(&mut self, _mode: Mode, settings: &Settings, latex: &str) -> Option<Result<String, String>> {
        let mut state = self.0.borrow_mut();
        state.started += 1;
        let latex = if settings.decimal { format!(r"\approx {latex}") } else { latex.to_string() };
        state.asked = Some(latex);
        None
    }

    fn poll(&mut self) -> Option<Result<String, String>> {
        self.0.borrow_mut().ready.take()
    }

    fn drop_pending(&mut self) {
        let mut state = self.0.borrow_mut();
        state.dropped += 1;
        state.asked = None;
    }
}

fn app() -> (App<Later>, Later) {
    let backend = Later::default();
    (App::new(backend.clone()), backend)
}

fn press(app: &mut App<Later>, keys: &[Key]) {
    keys.iter().for_each(|&key| app.key(key));
}

#[test]
fn the_spinner_turns_until_the_answer_comes() {
    let (mut app, backend) = app();
    press(&mut app, &[Key::Digit(2), Key::Plus, Key::Digit(3), Key::Exe]);
    assert!(app.busy);
    assert!(app.history.is_empty());
    assert_eq!(app.shown, None);

    app.tick(10);
    assert!(app.busy, "still calculating");
    backend.answer();
    assert!(app.tick(20));
    assert!(!app.busy);
    assert_eq!(app.history.len(), 1);
    assert_eq!(app.shown, Some(0));
    assert_eq!(app.editor.to_latex(), "2+3");
}

#[test]
fn keys_wait_while_it_calculates() {
    let (mut app, backend) = app();
    press(&mut app, &[Key::Digit(7), Key::Exe]);
    press(&mut app, &[Key::Digit(1), Key::Exe, Key::Del, Key::SwapForm]);
    assert_eq!(app.editor.to_latex(), "7", "the input stays as it was sent");
    assert_eq!(backend.0.borrow().started, 1, "EXE does not start a second calculation");
    backend.answer();
    app.tick(10);
    assert_eq!(app.history.len(), 1);
}

#[test]
fn ac_drops_the_answer_and_keeps_the_input() {
    let (mut app, backend) = app();
    press(&mut app, &[Key::Digit(7), Key::Exe, Key::Ac]);
    assert!(!app.busy);
    assert_eq!(backend.0.borrow().dropped, 1);
    assert_eq!(app.editor.to_latex(), "7");
    assert_eq!(app.shown, None);

    // An answer that comes anyway is not shown.
    backend.0.borrow_mut().ready = Some(Ok("7".to_string()));
    app.tick(10);
    assert!(app.history.is_empty());

    // The input can be sent again.
    press(&mut app, &[Key::Exe]);
    assert!(app.busy);
    assert_eq!(backend.0.borrow().started, 2);
}

#[test]
fn the_other_form_comes_later_too() {
    let (mut app, backend) = app();
    press(&mut app, &[Key::Digit(7), Key::Exe]);
    backend.answer();
    app.tick(10);
    press(&mut app, &[Key::SwapForm]);
    assert!(app.busy);
    assert!(!app.history[0].decimal);
    backend.answer();
    app.tick(20);
    assert!(!app.busy);
    assert!(app.history[0].decimal);
    assert_eq!(app.history.len(), 1);
}

#[test]
fn the_menus_open_while_it_calculates() {
    let (mut app, _backend) = app();
    press(&mut app, &[Key::Digit(7), Key::Exe, Key::Mode]);
    assert!(matches!(app.screen, quantus_ui::app::Screen::Modes { .. }));
    assert!(app.busy);
}
