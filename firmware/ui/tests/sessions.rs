//! The Sitzungen menu against a backend that keeps two sessions and notes what it is asked.

use std::cell::RefCell;
use std::rc::Rc;

use quantus_ui::app::{App, Mode, Screen, Settings};
use quantus_ui::backend::{Backend, Opened, SessionCommand, SessionInfo, Stub};
use quantus_ui::key::Key;
use quantus_ui::menu::{Edit, Menu};
use quantus_ui::render::Frame;
use quantus_ui::tree;

#[derive(Clone, Default)]
struct Two(Rc<RefCell<Vec<SessionCommand>>>);

fn info(id: &str, name: &str) -> SessionInfo {
    SessionInfo { id: id.to_string(), name: name.to_string() }
}

impl Backend for Two {
    fn evaluate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> Result<String, String> {
        Stub.evaluate(mode, settings, latex)
    }

    fn sessions(&mut self) -> Vec<SessionInfo> {
        vec![info("a", "Sitzung 1"), info("b", "Labor")]
    }

    fn session(&mut self, command: SessionCommand, _settings: &Settings) -> Result<Opened, String> {
        self.0.borrow_mut().push(command.clone());
        Ok(match command {
            SessionCommand::Open(id) if id == "b" => Opened { info: info("b", "Labor"), history: vec![("1+2".into(), "3".into())], ..Opened::default() },
            SessionCommand::Rename(name) => Opened { info: info("a", &name), history: Vec::new(), ..Opened::default() },
            SessionCommand::New => Opened { info: info("c", "Sitzung 2"), history: Vec::new(), ..Opened::default() },
            _ => Opened { info: info("a", "Sitzung 1"), history: vec![("2+2".into(), "4".into())], ..Opened::default() },
        })
    }
}

fn sessions_menu() -> (App<Two>, Two) {
    let backend = Two::default();
    let mut app = App::new(backend.clone());
    app.screen = Screen::Menu(Menu::new(&tree::SESSIONS));
    (app, backend)
}

fn press(app: &mut App<Two>, keys: &[Key]) {
    for &key in keys {
        app.key(key);
        let _ = Frame::new(app);
    }
}

#[test]
fn the_open_session_and_its_history_come_at_start() {
    let (app, backend) = sessions_menu();
    assert_eq!(backend.0.borrow().as_slice(), [SessionCommand::Current]);
    assert_eq!(app.session.as_ref().unwrap().name, "Sitzung 1");
    assert_eq!(app.history.len(), 1);
    assert_eq!(app.history[0].input.to_latex(), "2+2");
}

#[test]
fn open_lists_the_sessions_on_the_open_one_and_opens_the_chosen() {
    let (mut app, backend) = sessions_menu();
    press(&mut app, &[Key::Digit(2)]);
    let Screen::Menu(menu) = &app.screen else { panic!("the menu stays") };
    assert!(matches!(&menu.edit, Some(Edit::Sessions { selected: 0, sessions, .. }) if sessions.len() == 2));
    press(&mut app, &[Key::Down, Key::Ok]);
    assert_eq!(backend.0.borrow().last(), Some(&SessionCommand::Open("b".into())));
    assert_eq!(app.screen, Screen::Calc);
    assert_eq!(app.session.as_ref().unwrap().name, "Labor");
    assert_eq!(app.history[0].input.to_latex(), "1+2");
}

#[test]
fn rename_starts_from_the_name_and_refuses_an_empty_one() {
    let (mut app, backend) = sessions_menu();
    press(&mut app, &[Key::Digit(3)]);
    let Screen::Menu(menu) = &app.screen else { panic!() };
    assert!(matches!(&menu.edit, Some(Edit::Text { typed, .. }) if typed == "Sitzung 1"));
    press(&mut app, &[Key::Del; 9]);
    press(&mut app, &[Key::Ok]);
    let Screen::Menu(menu) = &app.screen else { panic!() };
    assert!(matches!(&menu.edit, Some(Edit::Text { error: true, .. })));
    press(&mut app, &[Key::Letter('L'), Key::Letter('a'), Key::Space, Key::Digit(2), Key::Ok]);
    assert_eq!(backend.0.borrow().last(), Some(&SessionCommand::Rename("La 2".into())));
    assert_eq!(app.session.as_ref().unwrap().name, "La 2");
    // Back on the Sitzungen page.
    let Screen::Menu(menu) = &app.screen else { panic!() };
    assert_eq!(menu.edit, None);
}

#[test]
fn new_and_delete_open_a_session_on_the_calculation_screen() {
    let (mut app, backend) = sessions_menu();
    press(&mut app, &[Key::Digit(1)]);
    assert_eq!(app.screen, Screen::Calc);
    assert_eq!(app.session.as_ref().unwrap().name, "Sitzung 2");
    assert!(app.history.is_empty());
    app.screen = Screen::Menu(Menu::new(&tree::SESSIONS));
    // Löschen asks first.
    press(&mut app, &[Key::Digit(5)]);
    assert_ne!(backend.0.borrow().last(), Some(&SessionCommand::Delete));
    press(&mut app, &[Key::Ok]);
    assert_eq!(backend.0.borrow().last(), Some(&SessionCommand::Delete));
    assert_eq!(app.screen, Screen::Calc);
}

#[test]
fn without_sessions_the_menu_says_so() {
    let mut app = App::new(Stub);
    app.screen = Screen::Menu(Menu::new(&tree::SESSIONS));
    app.key(Key::Digit(1));
    let Screen::Menu(menu) = &app.screen else { panic!() };
    assert!(matches!(menu.edit, Some(Edit::Message { .. })));
    assert_eq!(app.session, None);
}
