//! The settings survive a restart: written as text, read back, handed to the backend.

use std::cell::RefCell;
use std::rc::Rc;

use quantus_ui::app::{App, Mode, Screen, Settings};
use quantus_ui::backend::{Backend, Stub};
use quantus_ui::key::Key;
use quantus_ui::menu::Menu;
use quantus_ui::font::Scale;
use quantus_ui::settings::{Angle, FontSize, NumberFormat};
use quantus_ui::text::Lang;
use quantus_ui::tree;

#[test]
fn the_text_gives_back_the_settings() {
    let settings = Settings {
        language: Lang::En,
        angle: Angle::Radians,
        digits: 7,
        format: NumberFormat::Sci,
        format_digits: 4,
        decimal: true,
        wlan: true,
        time_from_network: false,
        font_size: FontSize::Large,
        ..Settings::default()
    };
    assert_eq!(Settings::from_text(&settings.to_text()), settings);
}

#[test]
fn the_font_size_starts_small_and_an_old_one_is_not_read() {
    assert_eq!(Settings::default().font_size, FontSize::Small);
    assert_eq!(FontSize::Small.scale(), Scale::Normal);
    assert_eq!(FontSize::Large.scale(), Scale::Huge);
    // Before, 1 was Klein.
    assert_eq!(Settings::from_text("quantus-settings 1\nfont_size=1\n").font_size, FontSize::Small);
}

#[test]
fn unknown_or_broken_lines_keep_the_defaults() {
    let text = "quantus-settings 1\ndigits=7\ncolour=blue\ndecimal=yes\nno line\n";
    assert_eq!(Settings::from_text(text), Settings { digits: 7, ..Settings::default() });
    assert_eq!(Settings::from_text("something else\ndigits=7\n"), Settings::default());
}

#[test]
fn a_saved_exam_mode_keeps_the_radios_off() {
    let text = "quantus-settings 1\nwlan=1\nexam=1\nbluetooth=1\n";
    let settings = Settings::from_text(text);
    assert!(settings.exam && !settings.wlan && !settings.bluetooth);
}

/// Starts with the settings it keeps and notes every set it is given.
#[derive(Clone, Default)]
struct Kept(Rc<RefCell<Vec<Settings>>>);

impl Backend for Kept {
    fn evaluate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> quantus_ui::backend::Answer {
        Stub.evaluate(mode, settings, latex)
    }

    fn load_settings(&mut self) -> Option<Settings> {
        Some(Settings { digits: 5, ..Settings::default() })
    }

    fn use_settings(&mut self, settings: &Settings) {
        self.0.borrow_mut().push(*settings);
    }
}

#[test]
fn the_app_starts_with_the_kept_settings_and_hands_on_each_change() {
    let backend = Kept::default();
    let mut app = App::new(backend.clone());
    assert_eq!(app.settings.digits, 5);
    assert_eq!(backend.0.borrow().len(), 1);
    app.tick(0);
    assert_eq!(backend.0.borrow().len(), 1, "nothing changed");
    // Verbindungen → WLAN → WLAN → Ein.
    app.screen = Screen::Menu(Menu::new(&tree::SETUP));
    for key in [Key::Up, Key::Digit(2), Key::Digit(2), Key::Digit(1), Key::Digit(1)] {
        app.key(key);
    }
    app.tick(1);
    let given = backend.0.borrow();
    assert_eq!(given.len(), 2);
    assert!(given[1].wlan && given[1].digits == 5);
}
