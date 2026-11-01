use quantus_ui::app::{App, Screen};
use quantus_ui::backend::Stub;
use quantus_ui::font::Scale;
use quantus_ui::key::Key;
use quantus_ui::menu::{self, Edit, Kind, Page};
use quantus_ui::render::{Frame, WIDTH};
use quantus_ui::settings::{Settings, StepDetail};
use quantus_ui::text::Lang;
use quantus_ui::tree;

fn press(app: &mut App<Stub>, keys: &[Key]) {
    keys.iter().for_each(|&key| app.key(key));
}

fn setup() -> App<Stub> {
    let mut app = App::new(Stub);
    press(&mut app, &[Key::Shift, Key::Mode]);
    assert!(matches!(app.screen, Screen::Menu(_)));
    app
}

fn menu(app: &App<Stub>) -> &menu::Menu {
    let Screen::Menu(menu) = &app.screen else { panic!("no menu open") };
    menu
}

#[test]
fn significant_digits_are_typed_and_checked() {
    let mut app = setup();
    // Allgemein → Signifikante Stellen, 12.
    press(&mut app, &[Key::Digit(1), Key::Digit(3), Key::Digit(1), Key::Digit(2), Key::Exe]);
    assert_eq!(app.settings.digits, 12);
    assert!(menu(&app).edit.is_none(), "back on the page");

    // 99 is out of range: refused, the input stays open and marks it.
    press(&mut app, &[Key::Digit(3), Key::Digit(9), Key::Digit(9), Key::Digit(9), Key::Exe]);
    assert_eq!(app.settings.digits, 12);
    assert!(matches!(menu(&app).edit, Some(Edit::Number { error: true, .. })));
    press(&mut app, &[Key::Del, Key::Del, Key::Digit(5), Key::Exe]);
    assert_eq!(app.settings.digits, 5);

    // ← goes back one level, AC closes.
    press(&mut app, &[Key::Left]);
    assert_eq!(menu(&app).stack.len(), 1);
    press(&mut app, &[Key::Ac]);
    assert_eq!(app.screen, Screen::Calc);
}

#[test]
fn a_choice_submenu_sets_its_value() {
    let mut app = setup();
    // Rechenweg → Rechenweg → Ausführlich (4th), by the cursor.
    press(&mut app, &[Key::Down, Key::Down, Key::Exe, Key::Exe, Key::Down, Key::Exe]);
    assert_eq!(app.settings.step_detail, StepDetail::Detailed);
    // By digit: Kurz.
    press(&mut app, &[Key::Digit(1), Key::Digit(2)]);
    assert_eq!(app.settings.step_detail, StepDetail::Brief);
}

#[test]
fn fix_asks_for_its_digits() {
    let mut app = setup();
    press(&mut app, &[Key::Digit(1), Key::Digit(4), Key::Digit(3), Key::Digit(3), Key::Exe]);
    assert_eq!((app.settings.get(quantus_ui::settings::Setting::Format), app.settings.format_digits), (2, 3));
    let entry = &match &tree::SETUP.entries[0].kind { Kind::Page(page) => &page.entries[3], _ => unreachable!() };
    assert_eq!(menu::value(entry, &app.settings, Lang::De).as_deref(), Some("Fix 3"));
}

#[test]
fn reset_memory_empties_the_history() {
    let mut app = setup();
    press(&mut app, &[Key::Ac, Key::Digit(7), Key::Exe]);
    assert_eq!(app.history.len(), 1);
    press(&mut app, &[Key::Shift, Key::Mode, Key::Digit(6), Key::Digit(4), Key::Digit(2), Key::Exe]);
    assert!(app.history.is_empty() && app.editor.is_empty());

    app.settings.digits = 3;
    press(&mut app, &[Key::Left, Key::Digit(1), Key::Digit(1)]);
    assert_eq!(app.settings, Settings::default());
}

#[test]
fn the_exam_mode_keeps_the_radios_off() {
    let mut app = setup();
    // Verbindungen → WLAN → WLAN → Ein.
    press(&mut app, &[Key::Up, Key::Digit(1), Key::Digit(2), Key::Digit(1), Key::Digit(1)]);
    assert!(app.settings.wlan);
    // Back, Prüfungsmodus → Ein switches it off and keeps it off.
    press(&mut app, &[Key::Left, Key::Digit(1), Key::Digit(1)]);
    assert!(app.settings.exam && !app.settings.wlan);
    press(&mut app, &[Key::Digit(2), Key::Digit(1), Key::Digit(1)]);
    assert!(!app.settings.wlan);
    assert!(matches!(menu(&app).edit, Some(Edit::Message { .. })));
}

#[test]
fn the_language_switches_the_texts() {
    let mut app = setup();
    assert_eq!(menu(&app).title().get(app.settings.language), "Setup");
    press(&mut app, &[Key::Digit(1), Key::Digit(1), Key::Digit(2)]);
    assert_eq!(app.settings.language, Lang::En);
    assert_eq!(menu(&app).title().get(app.settings.language), "General");
}

#[test]
fn every_page_draws() {
    let mut app = setup();
    for _ in 0..7 {
        let _ = Frame::new(&app);
        press(&mut app, &[Key::Exe]);
        let _ = Frame::new(&app);
        press(&mut app, &[Key::Exe]);
        let _ = Frame::new(&app);
        press(&mut app, &[Key::Left, Key::Left, Key::Down]);
    }
}

/// Every label and value fits its line next to each other, and every character has a glyph.
#[test]
fn every_entry_fits_its_line() {
    fn walk(page: &'static Page, pages: &mut Vec<&'static Page>) {
        pages.push(page);
        for entry in page.entries {
            if let Kind::Page(sub) = &entry.kind {
                walk(sub, pages);
            }
        }
    }
    let mut pages = Vec::new();
    walk(&tree::SETUP, &mut pages);
    walk(&tree::SESSIONS, &mut pages);
    for mode in quantus_ui::app::Mode::ALL {
        walk(tree::optn(mode), &mut pages);
    }
    let settings = Settings::default();
    let mut wide = Vec::new();
    for lang in [Lang::De, Lang::En] {
        for page in &pages {
            for entry in page.entries {
                let label = entry.label.get(lang);
                let mut texts = vec![label.to_string()];
                if let Kind::Choice(_, opts) = &entry.kind {
                    texts.extend(opts.iter().map(|opt| opt.text.get(lang).to_string()));
                }
                for text in &texts {
                    for c in text.chars().filter(|c| *c != ' ') {
                        assert!(Scale::Normal.width(&c.to_string()) > 0, "no glyph for {c:?} in {text:?}");
                    }
                }
                // The widest value any choice can show.
                let value = match &entry.kind {
                    Kind::Choice(_, opts) => {
                        opts.iter().map(|opt| Scale::Small.width(opt.text.get(lang))).max().unwrap_or(0)
                    }
                    _ => menu::value(entry, &settings, lang).map_or(0, |value| Scale::Small.width(&value)),
                };
                // As `lines_page` draws it: the label at 34 from the margin, the value 14 before the
                // scroll bar, an arrow for a page, and a gap of 12 between them.
                let arrow = if matches!(entry.kind, Kind::Page(_)) { 16 } else { 0 };
                let room = WIDTH as i32 - 2 * 10 - 8 - 34 - 14 - arrow - 12;
                let width = Scale::Normal.width(label) + value;
                if width > room {
                    wide.push(format!("{label:?} by {}", width - room));
                }
            }
        }
    }
    assert!(wide.is_empty(), "too wide: {wide:?}");
}

#[test]
fn the_top_bar_shows_the_exam_mode_and_the_radios() {
    use quantus_ui::render::Shape;
    // The WLAN icon is the only arc on the screen, the Bluetooth rune the only path.
    let shown = |app: &App<Stub>| -> (Vec<String>, bool, bool) {
        let shapes = Frame::new(app).0;
        let texts = shapes
            .iter()
            .filter_map(|shape| match shape {
                Shape::Text(_, text, _, _) => Some(text.clone()),
                _ => None,
            })
            .collect();
        let wlan = shapes.iter().any(|shape| matches!(shape, Shape::Arc(..)));
        let bluetooth = shapes.iter().any(|shape| matches!(shape, Shape::Path(..)));
        (texts, wlan, bluetooth)
    };
    let mut app = App::new(Stub);
    assert!(matches!(shown(&app), (_, false, false)));
    app.settings.wlan = true;
    app.settings.bluetooth = true;
    let (texts, wlan, bluetooth) = shown(&app);
    assert!(wlan && bluetooth);
    assert!(texts.contains(&"D".to_string()));
    app.settings.set(quantus_ui::settings::Setting::Exam, 1);
    let (texts, wlan, bluetooth) = shown(&app);
    assert!(texts.contains(&"PRÜFUNG".to_string()) && !wlan && !bluetooth);
}

#[test]
fn the_battery_shows_a_bolt_while_charging() {
    let fills = |app: &App<Stub>| {
        Frame::new(app).0.iter().filter(|shape| matches!(shape, quantus_ui::render::Shape::Fill(..))).count()
    };
    let mut app = App::new(Stub);
    app.battery = Some(40);
    let before = fills(&app);
    app.charging = true;
    assert!(fills(&app) > before);
}

#[test]
fn a_spinner_turns_in_the_top_bar_while_busy() {
    let dots = |app: &App<Stub>| {
        Frame::new(app).0.iter().filter(|shape| matches!(shape, quantus_ui::render::Shape::Circle(..))).count()
    };
    let mut app = App::new(Stub);
    let idle = dots(&app);
    assert!(!app.tick(200), "the spinner stands still while idle");
    app.busy = true;
    assert_eq!(dots(&app), idle + 8);
    let spin = app.spin;
    assert!(app.tick(400));
    assert_ne!(app.spin, spin);
}
