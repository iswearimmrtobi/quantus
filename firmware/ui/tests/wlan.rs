//! SETUP → Verbindungen → WLAN against a backend that finds two networks and notes what it is
//! asked.

use std::cell::RefCell;
use std::rc::Rc;

use quantus_ui::app::{App, Mode, Screen, Settings};
use quantus_ui::backend::{Backend, Network, Stub, WlanStatus};
use quantus_ui::font::Scale;
use quantus_ui::key::Key;
use quantus_ui::menu::{Edit, Menu, TextFor};
use quantus_ui::render::Frame;
use quantus_ui::text;
use quantus_ui::tree;

#[derive(Debug, PartialEq, Eq)]
enum Asked {
    Scan,
    Connect(String, Option<String>),
    Forget(String),
    Radio(bool),
}

#[derive(Clone, Default)]
struct Radio {
    asked: Rc<RefCell<Vec<Asked>>>,
    /// Scans answered so far: the first `networks` after a scan says it still looks.
    looked: Rc<RefCell<bool>>,
}

impl Backend for Radio {
    fn evaluate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> Result<String, String> {
        Stub.evaluate(mode, settings, latex)
    }

    fn use_settings(&mut self, settings: &Settings) {
        self.asked.borrow_mut().push(Asked::Radio(settings.wlan));
    }

    fn wlan_status(&mut self) -> WlanStatus {
        WlanStatus::Connected { ssid: "Heim".into(), ip: "192.168.1.23".into(), rssi: -48 }
    }

    fn scan(&mut self) {
        self.asked.borrow_mut().push(Asked::Scan);
        *self.looked.borrow_mut() = false;
    }

    fn networks(&mut self) -> Option<Vec<Network>> {
        if !std::mem::replace(&mut *self.looked.borrow_mut(), true) {
            return None;
        }
        Some(vec![
            Network { ssid: "Heim".into(), rssi: -48, secured: true, saved: true },
            Network { ssid: "Labor".into(), rssi: -71, secured: true, saved: false },
            Network { ssid: "Café".into(), rssi: -80, secured: false, saved: false },
        ])
    }

    fn connect(&mut self, ssid: &str, password: Option<&str>) {
        self.asked.borrow_mut().push(Asked::Connect(ssid.into(), password.map(str::to_string)));
    }

    fn saved_networks(&mut self) -> Vec<String> {
        vec!["Heim".into(), "Büro".into()]
    }

    fn forget(&mut self, ssid: &str) {
        self.asked.borrow_mut().push(Asked::Forget(ssid.into()));
    }
}

fn press(app: &mut App<Radio>, keys: &[Key]) {
    for &key in keys {
        app.key(key);
        // The second tick brings the networks found.
        app.tick(0);
        app.tick(0);
        let _ = Frame::new(app);
    }
}

fn typed(text: &str) -> Vec<Key> {
    text.chars().filter_map(Key::from_char).collect()
}

/// The WLAN page: Verbindungen (↑ wraps to the second page of SETUP), then its 2nd entry.
fn wlan_page() -> (App<Radio>, Radio) {
    let backend = Radio::default();
    let mut app = App::new(backend.clone());
    app.screen = Screen::Menu(Menu::new(&tree::SETUP));
    press(&mut app, &[Key::Up, Key::Digit(2), Key::Digit(2)]);
    (app, backend)
}

fn edit(app: &App<Radio>) -> Option<&Edit> {
    let Screen::Menu(menu) = &app.screen else { panic!("the menu stays") };
    menu.edit.as_ref()
}

#[test]
fn networks_wait_for_wlan_on() {
    let (mut app, backend) = wlan_page();
    press(&mut app, &[Key::Digit(2)]);
    assert!(matches!(edit(&app), Some(Edit::Message { message, .. }) if *message == text::WLAN_OFF));
    assert!(!backend.asked.borrow().contains(&Asked::Scan));
}

#[test]
fn a_new_network_asks_for_its_password_and_connects() {
    let (mut app, backend) = wlan_page();
    // WLAN → Ein, then Netzwerke.
    press(&mut app, &[Key::Digit(1), Key::Digit(1)]);
    assert_eq!(backend.asked.borrow().last(), Some(&Asked::Radio(true)));
    press(&mut app, &[Key::Digit(2)]);
    assert!(matches!(edit(&app), Some(Edit::Networks { networks: Some(found), .. }) if found.len() == 3));
    assert!(backend.asked.borrow().contains(&Asked::Scan));
    press(&mut app, &[Key::Digit(2)]);
    assert!(matches!(edit(&app), Some(Edit::Text { purpose: TextFor::Password(ssid), .. }) if ssid == "Labor"));
    // Too short first; the point and @ are typed as they are; S⇔D shows it.
    press(&mut app, &typed("a.b@"));
    press(&mut app, &[Key::Ok]);
    assert!(matches!(edit(&app), Some(Edit::Text { error: true, .. })));
    press(&mut app, &[Key::SwapForm]);
    assert!(matches!(edit(&app), Some(Edit::Text { plain: true, .. })));
    press(&mut app, &typed("Geheim-1"));
    press(&mut app, &[Key::Ok]);
    assert_eq!(backend.asked.borrow().last(), Some(&Asked::Connect("Labor".into(), Some("a.b@Geheim-1".into()))));
    assert!(matches!(edit(&app), Some(Edit::WlanStatus { status, .. }) if status.connected()));
    assert!(app.wlan_connected);
}

#[test]
fn a_saved_or_open_network_connects_at_once() {
    let (mut app, backend) = wlan_page();
    press(&mut app, &[Key::Digit(1), Key::Digit(1), Key::Digit(2), Key::Digit(1)]);
    assert_eq!(backend.asked.borrow().last(), Some(&Asked::Connect("Heim".into(), None)));
    press(&mut app, &[Key::Left, Key::Digit(2), Key::Digit(3)]);
    assert_eq!(backend.asked.borrow().last(), Some(&Asked::Connect("Café".into(), None)));
}

#[test]
fn a_saved_network_is_forgotten_after_asking() {
    let (mut app, backend) = wlan_page();
    press(&mut app, &[Key::Digit(3), Key::Down, Key::Ok]);
    assert!(matches!(edit(&app), Some(Edit::Forget { ssid, .. }) if ssid == "Büro"));
    press(&mut app, &[Key::Ok]);
    assert_eq!(backend.asked.borrow().last(), Some(&Asked::Forget("Büro".into())));
    assert!(matches!(edit(&app), Some(Edit::Message { message, .. }) if *message == text::DONE));
}

#[test]
fn the_exam_mode_switches_the_radio_off() {
    let (mut app, backend) = wlan_page();
    press(&mut app, &[Key::Digit(1), Key::Digit(1)]);
    assert!(app.wlan_connected);
    // Back to Verbindungen, Prüfungsmodus → Ein.
    press(&mut app, &[Key::Left, Key::Digit(1), Key::Digit(1)]);
    assert_eq!(backend.asked.borrow().last(), Some(&Asked::Radio(false)));
    // The status follows within half a second.
    app.tick(1000);
    assert!(!app.wlan_connected);
}

#[test]
fn the_status_shows_the_connection() {
    let (mut app, _) = wlan_page();
    press(&mut app, &[Key::Digit(4)]);
    assert!(matches!(edit(&app), Some(Edit::WlanStatus { .. })));
    let _ = Frame::new(&app);
}

#[test]
fn the_texts_have_their_glyphs() {
    for text in [text::PASSWORD_HINT, text::SEARCHING, text::WLAN_SEARCHING, text::CONNECTING] {
        for lang in [text::Lang::De, text::Lang::En] {
            for c in text.get(lang).chars().filter(|c| *c != ' ') {
                assert!(Scale::Small.width(&c.to_string()) > 0 || Scale::Normal.width(&c.to_string()) > 0, "{c:?}");
            }
        }
    }
}
