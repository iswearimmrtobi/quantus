//! The Duo's input screen (`quantus_ui::pad`): taps type keys, the soft keys follow the mode,
//! SHIFT shows its legends, and it draws on its own 480×320 panel beside the main screen.

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use quantus_ui::app::App;
use quantus_ui::backend::Stub;
use quantus_ui::key::Key;
use quantus_ui::layer::Layer;
use quantus_ui::pad::{Action, Pad, PadKey, HEIGHT, WIDTH};
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
    for label in ["1", "2", "+", "3", "EXE"] {
        tap(&mut app, label);
    }
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
fn the_soft_keys_are_on_the_pad_not_the_screen() {
    let mut app = duo();
    for key in [Key::Digit(1), Key::Divide, Key::Digit(3), Key::Exe] {
        app.key(key);
    }
    assert!(!soft::shows(&app));
    let keys = keys(&app);
    let first = keys.iter().find(|key| key.action == Action::Soft(0)).expect("a soft key");
    assert!(!first.label.is_empty());
    // On the handheld the same bar shows in the strips.
    app.pad = None;
    assert!(soft::shows(&app));
}

#[test]
fn shift_shows_the_legends_until_a_key() {
    let mut app = duo();
    tap(&mut app, "SHIFT");
    assert_eq!(app.modifier.layer(), Layer::Shift);
    // sin shows sin⁻¹ in the SHIFT colour.
    let frame = pad::frame(&app);
    let shift = quantus_ui::render::colors::shift();
    assert!(frame.0.iter().any(|shape| matches!(shape, Shape::Math(_, _, color) if *color == shift)));
    tap(&mut app, "sin");
    assert_eq!(app.modifier.layer(), Layer::Base);
    assert!(app.editor.to_latex().contains("sin^{-1}") || app.editor.to_latex().contains("arcsin"), "{}", app.editor.to_latex());
}

#[test]
fn the_page_key_turns_the_function_keys() {
    let mut app = duo();
    assert!(keys(&app).iter().any(|key| key.label == "sin"));
    tap(&mut app, "1/2");
    assert_eq!(app.pad.as_ref().unwrap().page, 1);
    assert!(keys(&app).iter().any(|key| key.label == "CONST"));
    assert!(!keys(&app).iter().any(|key| key.label == "sin"));
}

#[test]
fn every_key_lies_on_the_pad_apart_from_the_others() {
    let app = duo();
    let keys = keys(&app);
    let area = embedded_graphics::primitives::Rectangle::new(Point::zero(), Size::new(WIDTH, HEIGHT));
    for (index, key) in keys.iter().enumerate() {
        assert_eq!(area.intersection(&key.rect), key.rect, "{} off the pad", key.label);
        for other in &keys[index + 1..] {
            assert!(key.rect.intersection(&other.rect).is_zero_sized(), "{} on {}", key.label, other.label);
        }
    }
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
