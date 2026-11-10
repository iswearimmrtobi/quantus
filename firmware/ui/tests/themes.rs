use std::cell::RefCell;
use std::rc::Rc;

use quantus_ui::app::{App, Mode, Screen};
use quantus_ui::backend::{Backend, Stub};
use quantus_ui::key::Key;
use quantus_ui::menu::Edit;
use quantus_ui::render::colors::{self, Palette};
use quantus_ui::render::Frame;
use quantus_ui::settings::Settings;

/// A card with two themes of the user's, one of them broken, and a store for the theme kept.
struct Themed {
    kept: Rc<RefCell<Option<String>>>,
}

impl Backend for Themed {
    fn evaluate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> Result<String, String> {
        Stub.evaluate(mode, settings, latex)
    }

    fn themes(&mut self) -> Vec<(String, String)> {
        vec![
            ("sunset".to_string(), "name = { en = \"Sunset\", de = \"Abendrot\" }\n[colors]\nbackground = \"#402010\"".to_string()),
            ("broken".to_string(), "[colors]\ntext = \"red\"".to_string()),
        ]
    }

    fn load_theme(&mut self) -> Option<String> {
        self.kept.borrow().clone()
    }

    fn keep_theme(&mut self, id: &str) {
        *self.kept.borrow_mut() = Some(id.to_string());
    }
}

fn press(app: &mut App<Themed>, keys: &[Key]) {
    keys.iter().for_each(|&key| app.key(key));
}

fn edit(app: &App<Themed>) -> Option<&Edit> {
    let Screen::Menu(menu) = &app.screen else { panic!("no menu open") };
    menu.edit.as_ref()
}

/// SETUP → Gerät (on the second page) → Farbschema.
const OPEN: [Key; 5] = [Key::Shift, Key::Mode, Key::Up, Key::Digit(1), Key::Digit(4)];

#[test]
fn the_theme_list_previews_keeps_and_restores() {
    let kept = Rc::new(RefCell::new(None));
    let mut app = App::new(Themed { kept: kept.clone() });
    assert_eq!(app.theme.id, "dark");
    assert_eq!(colors::get(), Palette::DARK);

    press(&mut app, &OPEN);
    let Some(Edit::Themes { names, selected: 0, .. }) = edit(&app) else { panic!("no themes: {:?}", edit(&app)) };
    assert_eq!(names, &["Dunkel", "Hell", "broken (!)", "Abendrot"]);
    let _ = Frame::new(&app);

    // ↓ shows the light theme at once, ← goes back to the dark one.
    press(&mut app, &[Key::Down]);
    assert_eq!(colors::background(), colors::rgb(0xf6, 0xf5, 0xfa));
    press(&mut app, &[Key::Left]);
    assert_eq!(colors::get(), Palette::DARK);
    assert!(kept.borrow().is_none());

    // AC closes the whole menu and drops the preview too.
    press(&mut app, &[Key::Digit(4), Key::Down, Key::Ac]);
    assert!(matches!(app.screen, Screen::Calc));
    assert_eq!(colors::get(), Palette::DARK);
    press(&mut app, &OPEN);
    press(&mut app, &[Key::Left]);

    // 4 keeps Abendrot: its own background over the dark theme's other colours.
    press(&mut app, &[Key::Digit(4), Key::Digit(4)]);
    assert_eq!(app.theme.id, "sunset");
    assert_eq!(kept.borrow().as_deref(), Some("sunset"));
    assert_eq!(colors::background(), colors::rgb(0x40, 0x20, 0x10));
    assert_eq!(colors::text(), Palette::DARK.text);

    // The broken file says why and changes nothing.
    press(&mut app, &[Key::Digit(4), Key::Digit(3)]);
    let Some(Edit::Note { note, .. }) = edit(&app) else { panic!("no note: {:?}", edit(&app)) };
    assert!(note.contains("broken.toml") && note.contains("colors.text"), "{note}");
    assert_eq!(app.theme.id, "sunset");
    assert_eq!(colors::background(), colors::rgb(0x40, 0x20, 0x10));
}

#[test]
fn the_kept_theme_comes_back_at_the_start() {
    let kept = Rc::new(RefCell::new(Some("light".to_string())));
    let app = App::new(Themed { kept });
    assert_eq!(app.theme.id, "light");
    assert_eq!(colors::get(), Palette::from_theme(&quantus_themes::built_in("light").unwrap()));

    // One that is gone, as on another card, falls back to the default.
    let kept = Rc::new(RefCell::new(Some("gone".to_string())));
    let app = App::new(Themed { kept });
    assert_eq!(app.theme.id, "dark");
    assert_eq!(colors::get(), Palette::DARK);
}

#[test]
fn the_dark_palette_is_the_dark_theme() {
    assert_eq!(Palette::from_theme(&quantus_themes::default_theme()), Palette::DARK);
}
