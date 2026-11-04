use embedded_graphics::prelude::*;
use quantus_ui::app::{App, Mode, Screen};
use quantus_ui::backend::{Backend, Stub};
use quantus_ui::menu::{Edit, Menu};
use quantus_ui::render::{back_area, dot_rect, line_rect, number_button, tile_rect, Frame, HEIGHT, WIDTH};
use quantus_ui::settings::Settings;
use quantus_ui::key::Key;
use quantus_ui::touch::{Target, Touch, TARGETS};
use quantus_ui::wallpaper::Wallpaper;

fn tap<B: Backend>(app: &mut App<B>, point: Point) {
    app.touch(Touch::Press(point));
    app.touch(Touch::Release);
    // Every screen a tap leads to draws.
    let _ = Frame::new(app);
}

/// A finger dragged from `from` to `to`, the screen drawn on the way.
fn swipe<B: Backend>(app: &mut App<B>, from: Point, to: Point) {
    app.touch(Touch::Press(from));
    for step in 1..=4 {
        app.touch(Touch::Drag(from + (to - from) * step / 4));
        let _ = Frame::new(app);
    }
    app.touch(Touch::Release);
}

/// The middle of line `offset` of a menu list.
fn line(offset: usize) -> Point {
    Point::new(100, line_rect(offset, WIDTH as i32).center().y)
}

fn menu<B>(app: &App<B>) -> &Menu {
    let Screen::Menu(menu) = &app.screen else { panic!("no menu open") };
    menu
}

const BACK: Point = Point::new(20, 12);

/// SHIFT, MODE: the setup.
fn setup<B: Backend>(app: &mut App<B>) {
    app.key(Key::Shift);
    app.key(Key::Mode);
}

#[test]
fn back_closes_the_setup() {
    let mut app = App::new(Stub);
    setup(&mut app);
    assert_eq!(menu(&app).level().page.title.de, "Setup");
    // Allgemein, and back.
    tap(&mut app, line(0));
    assert_eq!(menu(&app).stack.len(), 2);
    tap(&mut app, BACK);
    assert_eq!(menu(&app).stack.len(), 1);
    tap(&mut app, BACK);
    assert_eq!(app.screen, Screen::Calc);
    // A tap on nothing does nothing.
    assert!(back_area().contains(BACK));
    tap(&mut app, BACK);
    assert_eq!(app.screen, Screen::Calc);
}

#[test]
fn a_choice_is_tapped() {
    let mut app = App::new(Stub);
    setup(&mut app);
    // Allgemein → Winkeleinheit → Bogenmaß.
    tap(&mut app, line(0));
    tap(&mut app, line(1));
    assert!(matches!(menu(&app).edit, Some(Edit::Choice { .. })));
    tap(&mut app, line(1));
    assert_eq!(app.settings.get(quantus_ui::settings::Setting::Angle), 1);
    assert!(menu(&app).edit.is_none(), "back on the page");
}

#[test]
fn a_number_is_stepped_and_set() {
    let mut app = App::new(Stub);
    let digits = Settings::default().digits;
    setup(&mut app);
    // Allgemein → Signifikante Stellen (1 to 15).
    tap(&mut app, line(0));
    tap(&mut app, line(2));
    assert!(matches!(menu(&app).edit, Some(Edit::Number { .. })));
    tap(&mut app, number_button(1).center());
    tap(&mut app, number_button(1).center());
    tap(&mut app, number_button(0).center());
    tap(&mut app, number_button(2).center());
    assert_eq!(app.settings.digits, digits + 1);

    // Kept in the range.
    tap(&mut app, line(2));
    for _ in 0..40 {
        tap(&mut app, number_button(1).center());
    }
    let Some(Edit::Number { typed, .. }) = &menu(&app).edit else { panic!("no number open") };
    assert_eq!(typed, "15");
    tap(&mut app, number_button(2).center());
    assert_eq!(app.settings.digits, 15);
}

#[test]
fn the_mode_tiles_page_by_swipe_and_by_dot() {
    let mut app = App::new(Stub);
    app.key(Key::Mode);
    assert_eq!(app.screen, Screen::Modes { selected: 0 });
    let middle = Point::new(WIDTH as i32 / 2, HEIGHT as i32 / 2);
    swipe(&mut app, middle, middle - Point::new(120, 0));
    assert_eq!(app.screen, Screen::Modes { selected: 6 });
    swipe(&mut app, middle, middle + Point::new(120, 0));
    assert_eq!(app.screen, Screen::Modes { selected: 0 });
    tap(&mut app, dot_rect(1).center());
    assert_eq!(app.screen, Screen::Modes { selected: 6 });
    // The fifth tile of the second page: Prüfen.
    tap(&mut app, tile_rect(4).center());
    assert_eq!((app.mode, &app.screen), (Mode::Verify, &Screen::Calc));
    // The last tile opens the sessions; back closes the tiles.
    app.key(Key::Mode);
    tap(&mut app, dot_rect(1).center());
    tap(&mut app, tile_rect(5).center());
    assert_eq!(menu(&app).level().page.title.de, "Sitzungen");
    tap(&mut app, BACK);
    assert_eq!(app.screen, Screen::Calc);
    app.key(Key::Mode);
    assert!(matches!(app.screen, Screen::Modes { .. }));
    tap(&mut app, BACK);
    assert_eq!(app.screen, Screen::Calc);
}

#[test]
fn a_long_list_follows_the_finger_and_turns_the_page() {
    let mut app = App::new(Stub);
    setup(&mut app);
    // SETUP has seven entries, five to a page. A short drag springs back.
    swipe(&mut app, line(3), line(3) - Point::new(0, 25));
    assert_eq!(menu(&app).level().selected, 0);
    // While it drags, the list moves with the finger.
    app.touch(Touch::Press(line(3)));
    app.touch(Touch::Drag(line(1)));
    assert_eq!(app.drag().map(|drag| drag.y), Some(line(1).y - line(3).y));
    app.touch(Touch::Release);
    assert_eq!(app.drag(), None);
    assert_eq!(menu(&app).level().selected, 5, "on the second page");
    swipe(&mut app, line(1), line(3));
    assert_eq!(menu(&app).level().selected, 0);
    // Verbindungen, the second line of the second page.
    swipe(&mut app, line(3), line(1));
    tap(&mut app, line(1));
    assert_eq!(menu(&app).level().page.title.de, "Verbindungen");
}

#[test]
fn what_the_finger_is_on_lights_up_until_it_drags_away() {
    let mut app = App::new(Stub);
    app.key(Key::Mode);
    let tile = tile_rect(0).center();
    app.touch(Touch::Press(tile));
    assert_eq!(app.pressed(), Some(Target::Tile(0)));
    app.touch(Touch::Drag(tile + Point::new(0, -60)));
    assert_eq!(app.pressed(), None);
    app.touch(Touch::Release);
    assert_eq!(app.screen, Screen::Modes { selected: 0 }, "a drag is no tap");
}

#[test]
fn optn_inserts_a_function() {
    let mut app = App::new(Stub);
    app.key(Key::Optn);
    // Hyperbol. Fkt., the second line of the second page → sinh.
    swipe(&mut app, line(3), line(1));
    tap(&mut app, line(1));
    tap(&mut app, line(0));
    assert_eq!(app.screen, Screen::Calc);
    assert_eq!(app.editor.to_latex(), r"\sinh()");
}

/// Two pictures on the card.
struct Card;

impl Backend for Card {
    fn evaluate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> Result<String, String> {
        Stub.evaluate(mode, settings, latex)
    }

    fn wallpapers(&mut self) -> Vec<String> {
        vec!["a".to_string(), "b".to_string()]
    }

    fn picture(&mut self, _name: &str) -> Option<Wallpaper> {
        static BLACK: [u8; (WIDTH * HEIGHT * 2) as usize] = [0; (WIDTH * HEIGHT * 2) as usize];
        Wallpaper::from_raw(&BLACK, &BLACK)
    }
}

#[test]
fn a_wallpaper_is_shown_on_the_first_tap_and_kept_on_the_second() {
    let mut app = App::new(Card);
    setup(&mut app);
    // Gerät, the first line of the second page → Hintergrundbild → Bild.
    swipe(&mut app, line(3), line(1));
    tap(&mut app, line(0));
    tap(&mut app, line(2));
    tap(&mut app, line(0));
    assert!(matches!(menu(&app).edit, Some(Edit::Pictures { .. })));
    tap(&mut app, line(2));
    assert_eq!(app.preview.as_ref().map(|(index, _)| *index), Some(2));
    assert_eq!(app.wallpaper_name, None);
    tap(&mut app, line(2));
    assert_eq!(app.wallpaper_name.as_deref(), Some("b"));
    assert!(app.preview.is_none());
}


/// What the panel of the board might read at a screen point: the axes swapped, one turned
/// round, and scaled to 12 bits.
fn panel(at: Point) -> Point {
    Point::new(300 + at.y * 11, 3900 - at.x * 7)
}

fn hold<B: Backend>(app: &mut App<B>, raw: Point) {
    app.touch(Touch::Press(raw));
    app.touch(Touch::Drag(raw + Point::new(3, -2)));
    app.touch(Touch::Drag(raw - Point::new(3, -2)));
    app.touch(Touch::Release);
    let _ = Frame::new(app);
}

#[test]
fn the_calibration_learns_the_panel() {
    let mut app = App::new(Stub);
    setup(&mut app);
    // Gerät, on the second page → Touch kalibrieren, its seventh line.
    swipe(&mut app, line(3), line(1));
    tap(&mut app, line(0));
    swipe(&mut app, line(3), line(1));
    tap(&mut app, line(1));
    assert!(matches!(app.screen, Screen::Calibrate(..)));
    for target in TARGETS {
        hold(&mut app, panel(target));
    }
    let calibration = app.take_calibration().expect("a calibration");
    assert_eq!(app.take_calibration(), None, "taken once");
    assert_eq!(menu(&app).level().page.title.de, "Gerät", "back where it came from");
    let off = calibration.map(panel(Point::new(100, 200))) - Point::new(100, 200);
    assert!(off.x.abs() <= 1 && off.y.abs() <= 1, "{off:?}");

    // From now on the panel's readings land where they were meant.
    tap(&mut app, panel(Point::new(20, 12)));
    tap(&mut app, panel(Point::new(20, 12)));
    assert_eq!(app.screen, Screen::Calc);
    app.key(Key::Mode);
    tap(&mut app, panel(dot_rect(1).center()));
    assert_eq!(app.screen, Screen::Modes { selected: 6 });
}

#[test]
fn a_calibration_too_far_off_is_done_again_and_ac_leaves_it() {
    let mut app = App::new(Stub);
    app.calibrate();
    // The middle pressed at a corner.
    for target in TARGETS.iter().take(4).copied().chain([Point::new(400, 300)]) {
        hold(&mut app, panel(target));
    }
    let Screen::Calibrate(state, _) = &app.screen else { panic!("left the calibration") };
    assert!(state.retry && state.readings.is_empty());
    assert_eq!(app.take_calibration(), None);
    app.key(quantus_ui::key::Key::Ac);
    assert_eq!(app.screen, Screen::Calc);
}

#[test]
fn a_calibration_is_stored_as_bytes() {
    let calibration = quantus_ui::touch::Calibration([0.1, -0.2, 3.0, 4.5, 0.0, -6.0]);
    assert_eq!(quantus_ui::touch::Calibration::from_bytes(&calibration.to_bytes()), Some(calibration));
    assert_eq!(quantus_ui::touch::Calibration::from_bytes(&[0; 7]), None);
}
