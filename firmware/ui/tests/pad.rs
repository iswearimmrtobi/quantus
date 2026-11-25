//! The Duo's input screen (`quantus_ui::pad`): taps type keys, the soft keys follow the mode,
//! SHIFT shows its legends, and it draws on its own 480×320 panel beside the main screen.

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use quantus_ui::app::App;
use quantus_ui::backend::Stub;
use quantus_ui::key::Key;
use quantus_ui::layer::Layer;
use quantus_ui::app::{Mode, Screen};
use quantus_ui::pad::{Action, Look, Pad, PadKey, HEIGHT, WIDTH};
use quantus_ui::profile::{self, Profile};
use quantus_ui::render::{pad, Frame, Renderer, Shape};
use quantus_ui::soft;
use quantus_ui::touch::Touch;

fn keys(app: &App<Stub>) -> Vec<PadKey> {
    app.pad.as_ref().unwrap().keys(app)
}

/// Taps the key labelled `label` in its middle.
fn tap(app: &mut App<Stub>, label: &str) {
    let key = keys(app).into_iter().find(|key| key.label == label).unwrap_or_else(|| panic!("no key {label}"));
    app.pad_touch(Touch::Press(key.rect.center()));
    app.pad_touch(Touch::Release);
}

fn duo() -> App<Stub> {
    let mut app = App::new(Stub);
    app.pad = Some(Pad::new());
    app
}

#[test]
fn taps_type_a_calculation() {
    let mut app = duo();
    for label in ["1", "2", "+", "3"] {
        tap(&mut app, label);
    }
    // EXE is a hardware button.
    app.key(Key::Exe);
    assert_eq!(app.history.len(), 1);
    assert_eq!(app.history[0].input.to_latex(), "12+3");
}

#[test]
fn a_key_slid_off_does_nothing() {
    let mut app = duo();
    let keys = keys(&app);
    let seven = keys.iter().find(|key| key.label == "7").unwrap();
    let eight = keys.iter().find(|key| key.label == "8").unwrap();
    app.pad_touch(Touch::Press(seven.rect.center()));
    app.pad_touch(Touch::Drag(eight.rect.center()));
    app.pad_touch(Touch::Release);
    assert!(app.editor.is_empty());
}

#[test]
fn the_soft_keys_are_mode_keys_not_strips() {
    let mut app = duo();
    for key in [Key::Digit(1), Key::Divide, Key::Digit(3), Key::Exe] {
        app.key(key);
    }
    assert!(!soft::shows(&app));
    // Over the result its forms lead the mode keys, and act as the strip would.
    let swap = keys(&app).into_iter().find(|key| matches!(&key.action, Action::Slot(_)) && key.label.contains("Leftrightarrow")).expect("S⇔D");
    assert_eq!(swap.rect.top_left.y, keys(&app).iter().map(|key| key.rect.top_left.y).filter(|&y| y > 2).min().unwrap(), "S⇔D leads");
    // A soft key acts as its strip would: Basis-N's HEX sets the base.
    let mut base_n = duo();
    base_n.mode = Mode::BaseN;
    tap(&mut base_n, "HEX");
    assert_eq!(base_n.settings.number_base, quantus_ui::settings::NumberBase::Hex);
    // On the handheld the same bar shows in the strips.
    app.pad = None;
    assert!(soft::shows(&app));
}

#[test]
fn shift_shows_the_legends_until_a_key() {
    let mut app = duo();
    tap(&mut app, "SHIFT");
    assert_eq!(app.modifier.layer(), Layer::Shift);
    // √ shows ⁿ√ in the SHIFT colour.
    let frame = pad::frame(&app);
    let shift = quantus_ui::render::colors::shift();
    assert!(frame.0.iter().any(|shape| matches!(shape, Shape::Math(_, _, color) if *color == shift)));
    tap(&mut app, "√");
    assert_eq!(app.modifier.layer(), Layer::Base);
    assert!(app.editor.to_latex().contains(r"\sqrt["), "{}", app.editor.to_latex());
}

/// Every key of the pad as `app` shows it, page by page, until the first page comes back.
fn all_pages(app: &mut App<Stub>) -> Vec<Vec<PadKey>> {
    let mut pages = vec![keys(app)];
    while let Some(page) = pages.last().unwrap().iter().find(|key| key.action == Action::Page).cloned() {
        app.pad_touch(Touch::Press(page.rect.center()));
        app.pad_touch(Touch::Release);
        let next = keys(app);
        if next == pages[0] {
            break;
        }
        assert!(pages.len() < 10, "the pages do not come round");
        pages.push(next);
    }
    pages
}

#[test]
fn one_page_key_turns_the_mode_keys() {
    let mut app = duo();
    let pages = all_pages(&mut app);
    assert!(pages.len() >= 2);
    for page in &pages {
        assert_eq!(page.iter().filter(|key| key.action == Action::Page).count(), 1);
    }
    // The core on page 1, the functions later.
    for label in ["(", ")", "(−)", "a/b", "xʸ", "√"] {
        assert!(pages[0].iter().any(|key| key.label == label), "{label} not on page 1");
    }
    assert!(!pages[0].iter().any(|key| key.label == "sin"));
    assert!(pages[1..].iter().flatten().any(|key| key.label == "sin"));
}

/// The modes and screens whose keys are checked for size and room.
fn contexts() -> Vec<App<Stub>> {
    let mut list = Vec::new();
    for mode in [Mode::Physics, Mode::Algebra, Mode::Imaginary, Mode::Analysis, Mode::Matrix, Mode::BaseN, Mode::Verify, Mode::Table, Mode::Statistics] {
        let mut app = duo();
        app.mode = mode;
        list.push(app);
    }
    let mut menu = duo();
    menu.key(Key::Optn);
    list.push(menu);
    list
}

#[test]
fn every_key_is_large_enough_and_apart_from_the_others() {
    let area = embedded_graphics::primitives::Rectangle::new(Point::zero(), Size::new(WIDTH, HEIGHT));
    for mut app in contexts() {
        for keys in all_pages(&mut app) {
            for (index, key) in keys.iter().enumerate() {
                assert_eq!(area.intersection(&key.rect), key.rect, "{} off the pad", key.label);
                assert!(key.rect.size.width >= 62 && key.rect.size.height >= 40, "{} is {:?}", key.label, key.rect.size);
                for other in &keys[index + 1..] {
                    assert!(key.rect.intersection(&other.rect).is_zero_sized(), "{} on {}", key.label, other.label);
                }
            }
        }
    }
}

#[test]
fn the_number_block_is_larger_than_the_old_function_keys() {
    let app = duo();
    let seven = keys(&app).into_iter().find(|key| key.label == "7").unwrap();
    assert!(seven.rect.size.width >= 62 && seven.rect.size.height >= 60, "{:?}", seven.rect.size);
    // The modifiers on top, taller than the old 34 px row.
    let shift = keys(&app).into_iter().find(|key| key.label == "SHIFT").unwrap();
    assert_eq!(shift.rect.top_left.y, 2);
    assert!(shift.rect.size.height >= 40);
}

#[test]
fn base_n_types_hex_digits_and_fades_the_others() {
    let mut app = duo();
    app.mode = Mode::BaseN;
    app.settings.number_base = quantus_ui::settings::NumberBase::Hex;
    tap(&mut app, "F");
    assert!(app.editor.to_latex().contains('F'), "{}", app.editor.to_latex());
    app.settings.number_base = quantus_ui::settings::NumberBase::Bin;
    let keys = keys(&app);
    let look = |label: &str| keys.iter().find(|key| key.label == label).unwrap().look;
    assert_eq!(look("1"), Look::Plain);
    assert_eq!(look("2"), Look::Faint);
    assert_eq!(look("A"), Look::Faint);
}

#[test]
fn unit_is_faint_outside_physics() {
    let mut app = duo();
    let unit = |app: &App<Stub>| keys(app).into_iter().find(|key| key.label == "UNIT").unwrap();
    assert_eq!(unit(&app).look, Look::Plain);
    app.mode = Mode::Algebra;
    assert_eq!(unit(&app).look, Look::Faint);
    assert_eq!(unit(&app).action, Action::None);
}

#[test]
fn times_ten_types_a_power_of_ten() {
    let mut app = duo();
    tap(&mut app, "3");
    tap(&mut app, "×10ˣ");
    tap(&mut app, "8");
    // The editor writes × as \cdot.
    assert_eq!(app.editor.to_latex(), r"3\cdot10^{8}");
}

#[test]
fn a_line_of_text_gets_the_letter_keyboard() {
    let mut app = duo();
    app.key(Key::Catalog);
    assert!(app.text_entry());
    assert!(keys(&app).iter().any(|key| key.label == "q"));
    for label in ["s", "i", "n"] {
        tap(&mut app, label);
    }
    let Screen::Menu(menu) = &app.screen else { panic!("the catalogue") };
    assert!(matches!(&menu.edit, Some(quantus_ui::menu::Edit::Catalogue { query, .. }) if query == "sin"), "{:?}", menu.edit);
}

#[test]
fn an_analysis_graph_gets_its_view_keys() {
    let mut app = duo();
    app.mode = Mode::Analysis;
    // The view keys show while an answer is on the screen.
    assert!(!keys(&app).iter().any(|key| key.label == quantus_ui::text::PAD_CLOSE.get(app.settings.language)));
}

/// Counts what is drawn off a 480×320 panel.
struct Panel(usize);

impl OriginDimensions for Panel {
    fn size(&self) -> Size {
        Size::new(WIDTH, HEIGHT)
    }
}

impl DrawTarget for Panel {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I: IntoIterator<Item = Pixel<Rgb565>>>(&mut self, pixels: I) -> Result<(), Self::Error> {
        let size = self.size();
        self.0 += pixels.into_iter().filter(|Pixel(point, _)| !(point.x >= 0 && point.y >= 0 && (point.x as u32) < size.width && (point.y as u32) < size.height)).count();
        Ok(())
    }
}

#[test]
fn the_pad_draws_at_its_own_size_beside_the_main_screen() {
    profile::set(Profile::DUO);
    let app = duo();
    let mut panel = Panel(0);
    let sent = Renderer::new().render_on(&pad::frame(&app), &mut panel, Profile::HANDHELD).unwrap();
    assert_eq!(sent, 10, "every band of 32 rows");
    assert_eq!(panel.0, 0);
    // The main screen still draws for the Duo's profile.
    let main = Frame::new(&app);
    assert!(!main.0.is_empty());
}

/// Touches each target where a panel that reads twice as far and 10 off would report it.
fn touch_targets(app: &mut App<Stub>, targets: &[Point], pad: bool) {
    for &target in targets {
        let raw = Point::new(2 * target.x + 10, 2 * target.y + 10);
        let touch = |app: &mut App<Stub>, touch| if pad { app.pad_touch(touch) } else { app.touch(touch) };
        touch(app, Touch::Press(raw));
        touch(app, Touch::Release);
    }
}

#[test]
fn the_pad_calibrates_on_its_own_targets() {
    let mut app = duo();
    app.calibrate_pad();
    assert!(app.pad_calibrating());
    let frame = pad::frame(&app);
    let heading = quantus_ui::text::CALIBRATE_PAD.get(app.settings.language);
    assert!(frame.0.iter().any(|shape| matches!(shape, Shape::Text(_, text, ..) if text == heading)));
    touch_targets(&mut app, &Pad::targets(), true);
    assert!(!app.pad_calibrating());
    let pad = app.pad.as_mut().unwrap();
    let calibration = pad.take_calibration().expect("a calibration to store");
    assert_eq!(calibration.map(Point::new(2 * 100 + 10, 2 * 60 + 10)), Point::new(100, 60));
    // The main screen's calibration is untouched, and the keys work through the new one.
    assert_eq!(app.calibration, quantus_ui::touch::Calibration::default());
    let seven = keys(&app).into_iter().find(|key| key.label == "7").unwrap().rect.center();
    app.pad_touch(Touch::Press(Point::new(2 * seven.x + 10, 2 * seven.y + 10)));
    app.pad_touch(Touch::Release);
    assert_eq!(app.editor.to_latex(), "7");
}

#[test]
fn the_main_calibration_goes_on_to_the_pad() {
    let mut app = duo();
    app.calibrate();
    touch_targets(&mut app, &quantus_ui::touch::TARGETS, false);
    assert!(app.take_calibration().is_some());
    assert!(app.pad_calibrating());
}

#[test]
fn ac_leaves_the_pad_calibration_as_it_was() {
    let mut app = duo();
    app.calibrate_pad();
    app.key(Key::Ac);
    assert!(!app.pad_calibrating());
    assert!(app.pad.as_mut().unwrap().take_calibration().is_none());
}

#[test]
fn the_handheld_has_no_pad_to_calibrate() {
    let mut app = App::new(Stub);
    app.calibrate_pad();
    assert!(!app.pad_calibrating());
    assert!(pad::frame(&app).0.is_empty());
}
