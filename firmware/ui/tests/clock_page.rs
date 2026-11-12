//! SETUP → Verbindungen → Uhrzeit: the date and the time typed set the clock.

use std::cell::RefCell;
use std::rc::Rc;

use quantus_ui::app::{App, Mode, Screen, Settings};
use quantus_ui::backend::{Backend, Stub};
use quantus_ui::clock::{self, Zone};
use quantus_ui::key::Key;
use quantus_ui::menu::{self, Edit, Menu, TimeField};
use quantus_ui::render::Frame;
use quantus_ui::text::Lang;
use quantus_ui::tree;

/// Notes the times it is set to, and leaves the clock alone.
#[derive(Clone, Default)]
struct Hand(Rc<RefCell<Vec<i64>>>);

impl Backend for Hand {
    fn evaluate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> quantus_ui::backend::Answer {
        Stub.evaluate(mode, settings, latex)
    }

    fn set_time(&mut self, utc_ms: i64) {
        self.0.borrow_mut().push(utc_ms);
    }
}

fn press(app: &mut App<Hand>, keys: &[Key]) {
    for &key in keys {
        app.key(key);
        let _ = Frame::new(app);
    }
}

fn digits(text: &str) -> Vec<Key> {
    text.bytes().map(|b| Key::Digit(b - b'0')).collect()
}

/// The Uhrzeit page: Verbindungen (↑ wraps to the second page of SETUP), then its 5th entry.
fn clock_page() -> (App<Hand>, Hand) {
    let backend = Hand::default();
    let mut app = App::new(backend.clone());
    app.screen = Screen::Menu(Menu::new(&tree::SETUP));
    press(&mut app, &[Key::Up, Key::Digit(2), Key::Digit(5)]);
    (app, backend)
}

fn edit(app: &App<Hand>) -> Option<&Edit> {
    let Screen::Menu(menu) = &app.screen else { panic!("the menu stays") };
    menu.edit.as_ref()
}

#[test]
fn a_date_typed_sets_the_clock_and_the_source_to_by_hand() {
    let (mut app, backend) = clock_page();
    assert!(app.settings.time_from_network);
    press(&mut app, &[Key::Digit(1)]);
    assert!(matches!(edit(&app), Some(Edit::Time { field: TimeField::Date, .. })));
    press(&mut app, &digits("02112026"));
    press(&mut app, &[Key::Ok]);
    assert_eq!(edit(&app), None);
    let set = backend.0.borrow();
    // Noon in Berlin, the clock not being set before (the backend leaves it alone).
    if clock::now().is_none() {
        assert_eq!(set.as_slice(), [clock::to_utc(2026, 11, 2, 12, 0, Zone::Berlin)]);
    }
    let local = clock::local(set[0], Zone::Berlin);
    assert_eq!((local.year, local.month, local.day), (2026, 11, 2));
    assert!(!app.settings.time_from_network);
}

#[test]
fn a_date_that_does_not_exist_is_refused() {
    let (mut app, backend) = clock_page();
    press(&mut app, &[Key::Digit(1)]);
    press(&mut app, &digits("29022027"));
    press(&mut app, &[Key::Ok]);
    assert!(matches!(edit(&app), Some(Edit::Time { error: true, .. })));
    // Too short, too.
    press(&mut app, &[Key::Del, Key::Del, Key::Ok]);
    assert!(matches!(edit(&app), Some(Edit::Time { error: true, .. })));
    assert!(backend.0.borrow().is_empty());
}

#[test]
fn the_time_typed_sets_the_clock_on_its_date() {
    let (mut app, backend) = clock_page();
    press(&mut app, &[Key::Digit(2)]);
    press(&mut app, &digits("2561"));
    press(&mut app, &[Key::Ok]);
    assert!(matches!(edit(&app), Some(Edit::Time { error: true, .. })), "25:61 is no time");
    press(&mut app, &[Key::Del, Key::Del, Key::Del, Key::Del]);
    press(&mut app, &digits("0730"));
    press(&mut app, &[Key::Ok]);
    match clock::now() {
        // The system clock of the PC running the test is set.
        Some(now) => {
            let today = clock::local(now, Zone::Berlin);
            let set = clock::local(backend.0.borrow()[0], Zone::Berlin);
            assert_eq!((set.day, set.hour, set.minute), (today.day, 7, 30));
        }
        None => assert!(matches!(edit(&app), Some(Edit::Message { .. }))),
    }
}

#[test]
fn the_field_shows_what_is_still_to_type() {
    assert_eq!(TimeField::Date.show("021"), "02.1_.____");
    assert_eq!(TimeField::Clock.show(""), "__:__");
    assert_eq!(menu::typed_date("31042026"), None);
    assert_eq!(menu::typed_date("29022028"), Some((2028, 2, 29)));
    assert_eq!(menu::typed_clock("2359"), Some((23, 59)));
}

#[test]
fn the_page_shows_the_date_with_its_weekday() {
    let monday = clock::to_utc(2026, 11, 2, 14, 30, Zone::Berlin);
    let local = clock::local(monday, Zone::Berlin);
    assert_eq!(TimeField::Date.format(&local, Lang::De), "Mo 02.11.2026");
    assert_eq!(TimeField::Date.format(&local, Lang::En), "Mon 02.11.2026");
    assert_eq!(TimeField::Clock.format(&local, Lang::De), "14:30");
}
