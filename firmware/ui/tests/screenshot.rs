use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::{Cursor, Seek, SeekFrom, Write};
use std::rc::Rc;

use embedded_graphics::image::Image;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use quantus_ui::app::{screenshot_name, App, Mode, Screen, Settings, NOTICE_MS};
use quantus_ui::backend::{Answer, Backend, Create, File, Stub};
use quantus_ui::clock::Zone;
use quantus_ui::layer::Layer;
use quantus_ui::text;
use quantus_ui::key::Key;
use quantus_ui::render::{bmp, Frame, Renderer, Shape, HEIGHT, WIDTH};
use quantus_ui::wallpaper::Wallpaper;
use tinybmp::Bmp;

fn press<B: quantus_ui::backend::Backend>(app: &mut App<B>, keys: &[Key]) {
    keys.iter().for_each(|&key| app.key(key));
}

/// A picture of the whole screen in memory.
struct Screen565(Vec<Rgb565>);

impl Default for Screen565 {
    fn default() -> Self {
        Screen565(vec![Rgb565::BLACK; (WIDTH * HEIGHT) as usize])
    }
}

impl OriginDimensions for Screen565 {
    fn size(&self) -> Size {
        Size::new(WIDTH, HEIGHT)
    }
}

impl DrawTarget for Screen565 {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I: IntoIterator<Item = Pixel<Rgb565>>>(&mut self, pixels: I) -> Result<(), Self::Error> {
        for Pixel(point, color) in pixels {
            if let (Ok(x @ 0..WIDTH), Ok(y @ 0..HEIGHT)) = (u32::try_from(point.x), u32::try_from(point.y)) {
                self.0[(y * WIDTH + x) as usize] = color;
            }
        }
        Ok(())
    }
}

/// What a BMP file shows, read with tinybmp as a wallpaper is.
fn read(bytes: &[u8]) -> Screen565 {
    let bmp = Bmp::<Rgb565>::from_slice(bytes).expect("a BMP file");
    assert_eq!(bmp.size(), Size::new(WIDTH, HEIGHT));
    let mut screen = Screen565::default();
    Image::new(&bmp, Point::zero()).draw(&mut screen).unwrap();
    screen
}

/// Every colour of RGB565 somewhere: red across, green down, blue in steps.
fn rainbow() -> Vec<Rgb565> {
    (0..WIDTH * HEIGHT)
        .map(|i| {
            let (x, y) = (i % WIDTH, i / WIDTH);
            Rgb565::new((x % 32) as u8, (y % 64) as u8, ((x / 32 + y / 64) % 32) as u8)
        })
        .collect()
}

#[test]
fn a_screenshot_is_the_screen_pixel_for_pixel() {
    let mut app = App::new(Stub);
    app.wallpaper = Wallpaper::new(rainbow());
    press(&mut app, &[Key::Digit(1), Key::Plus, Key::Digit(2), Key::Exe]);
    let frame = Frame::new(&app);
    let mut shown = Screen565::default();
    Renderer::new().render(&frame, &mut shown).unwrap();

    let mut file = Cursor::new(Vec::new());
    bmp::write(&frame, &mut file).unwrap();
    let bytes = file.into_inner();
    assert_eq!(bytes.len(), bmp::SIZE as usize);
    assert_eq!(bytes.len(), 54 + 480 * 320 * 3);
    assert!(read(&bytes).0 == shown.0, "the file differs from the screen");
}

#[test]
fn the_rows_are_bottom_up_and_blue_green_red() {
    let mut file = Cursor::new(Vec::new());
    let mut writer = bmp::Writer::new(&mut file).unwrap();
    // A red top row and a blue bottom row, through both ways of drawing.
    let top = Rectangle::new(Point::zero(), Size::new(WIDTH, 1));
    writer.fill_contiguous(&top, std::iter::repeat(Rgb565::RED)).unwrap();
    let bottom = Rectangle::new(Point::new(0, HEIGHT as i32 - 1), Size::new(WIDTH, 1));
    bottom.into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE)).draw(&mut writer).unwrap();
    // A green pixel through a part of a row.
    writer.fill_contiguous(&Rectangle::new(Point::new(2, 1), Size::new(1, 1)), [Rgb565::GREEN]).unwrap();
    let bytes = file.into_inner();
    assert_eq!(&bytes[..2], b"BM");
    assert_eq!(&bytes[54..57], &[255, 0, 0], "the first row in the file is the bottom one, blue");
    assert_eq!(&bytes[bytes.len() - 3..], &[0, 0, 255], "the last is the top one, red");
    let row = 480 * 3;
    let green = bytes.len() - 2 * row + 2 * 3;
    assert_eq!(&bytes[green..green + 3], &[0, 255, 0]);
    // It reads back as a wallpaper.
    let wallpaper = Wallpaper::from_bmp(&bytes).expect("a wallpaper");
    assert_eq!(wallpaper.full.get(0), Rgb565::RED);
    assert_eq!(wallpaper.full.get((WIDTH * HEIGHT - 1) as usize), Rgb565::BLUE);
}

/// Whether the frame shows `text` somewhere.
fn shows<B>(app: &App<B>, text: &str) -> bool {
    Frame::new(app).0.iter().any(|shape| matches!(shape, Shape::Text(_, shown, ..) if shown == text))
}

#[test]
fn a_notice_shows_over_the_bar_for_two_seconds() {
    let mut app = App::new(Stub);
    app.tick(10_000);
    app.notify("Saved", false);
    assert!(shows(&app, "Saved"));
    // Its time starts at the next tick.
    app.tick(10_100);
    assert!(app.notice.is_some());
    app.tick(10_100 + NOTICE_MS - 1);
    assert!(shows(&app, "Saved"));
    assert!(app.tick(10_100 + NOTICE_MS));
    assert!(app.notice.is_none() && !shows(&app, "Saved"));
}

#[test]
fn a_key_takes_the_notice_away_and_does_what_it_does() {
    let mut app = App::new(Stub);
    app.notify("No SD card", true);
    press(&mut app, &[Key::Digit(4)]);
    assert!(app.notice.is_none());
    assert_eq!(app.editor.to_latex(), "4");
}

/// The files on a card in memory, by path.
type Files = Rc<RefCell<BTreeMap<String, Vec<u8>>>>;

/// The stub's answers, with a card in memory.
struct Card(Files);

impl Backend for Card {
    fn evaluate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> Answer {
        Stub.evaluate(mode, settings, latex)
    }

    fn create(&mut self, path: &str) -> Result<Box<dyn File>, Create> {
        if self.0.borrow().contains_key(path) {
            return Err(Create::Exists);
        }
        self.0.borrow_mut().insert(path.to_string(), Vec::new());
        Ok(Box::new(Written { path: path.to_string(), bytes: Cursor::new(Vec::new()), files: self.0.clone() }))
    }
}

/// A file being written; on the card when dropped.
struct Written {
    path: String,
    bytes: Cursor<Vec<u8>>,
    files: Files,
}

impl Write for Written {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.bytes.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Seek for Written {
    fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
        self.bytes.seek(to)
    }
}

impl Drop for Written {
    fn drop(&mut self) {
        self.files.borrow_mut().insert(self.path.clone(), std::mem::take(self.bytes.get_mut()));
    }
}

fn card() -> (App<Card>, Files) {
    let files = Files::default();
    (App::new(Card(files.clone())), files)
}

/// The names of the screenshots on the card.
fn names(files: &Files) -> Vec<String> {
    files.borrow().keys().cloned().collect()
}

#[test]
fn shift_7_saves_the_screen_without_the_shift_mark_and_says_where() {
    let (mut app, files) = card();
    press(&mut app, &[Key::Digit(1), Key::Plus, Key::Digit(2), Key::Exe, Key::Shift, Key::Digit(7)]);
    let names = names(&files);
    assert_eq!(names.len(), 1);
    assert!(names[0].starts_with("screenshots/") && names[0].ends_with(".bmp"), "{names:?}");
    // Nothing typed, SHIFT used up.
    assert_eq!(app.modifier.pending, None);
    assert!(app.history.len() == 1 && app.editor.to_latex() == "1+2");
    let notice = app.notice.clone().expect("a notice");
    assert!(!notice.error && notice.text.ends_with(&names[0].replace("screenshots/", "/screenshots/")), "{notice:?}");
    // The picture is the screen without the notice and without SHIFT in the bar.
    app.notice = None;
    let mut shown = Screen565::default();
    Renderer::new().render(&Frame::new(&app), &mut shown).unwrap();
    assert!(read(&files.borrow()[&names[0]]).0 == shown.0);
}

#[test]
fn a_second_screenshot_gets_a_name_of_its_own() {
    let (mut app, files) = card();
    press(&mut app, &[Key::Shift, Key::Digit(7), Key::Screenshot]);
    assert_eq!(names(&files).len(), 2);
    assert!(files.borrow().values().all(|bytes| bytes.len() == bmp::SIZE as usize));
}

#[test]
fn shift_7_works_in_a_menu_and_on_the_mode_tiles() {
    let (mut app, files) = card();
    // SETUP, then SHIFT + 7 in it.
    press(&mut app, &[Key::Shift, Key::Mode]);
    assert!(matches!(app.screen, Screen::Menu(_)));
    press(&mut app, &[Key::Shift]);
    assert_eq!(app.modifier.pending, Some(Layer::Shift), "the bar shows SHIFT");
    press(&mut app, &[Key::Digit(7)]);
    assert!(matches!(app.screen, Screen::Menu(_)));
    assert_eq!(names(&files).len(), 1);
    // The mode tiles.
    press(&mut app, &[Key::Ac, Key::Ac, Key::Ac, Key::Mode]);
    assert!(matches!(app.screen, Screen::Modes { .. }), "{:?}", app.screen);
    press(&mut app, &[Key::Shift, Key::Digit(7)]);
    assert!(matches!(app.screen, Screen::Modes { .. }));
    assert_eq!(names(&files).len(), 2);
}

#[test]
fn shift_then_another_key_in_a_menu_does_what_the_key_does() {
    let (mut shifted, _) = card();
    let (mut plain, _) = card();
    press(&mut shifted, &[Key::Shift, Key::Mode, Key::Shift, Key::Down, Key::Shift, Key::Shift, Key::Down]);
    press(&mut plain, &[Key::Shift, Key::Mode, Key::Down, Key::Down]);
    assert_eq!(shifted.screen, plain.screen);
    assert_eq!(shifted.modifier, plain.modifier);
}

#[test]
fn without_a_card_the_notice_says_so() {
    let mut app = App::new(Stub);
    press(&mut app, &[Key::Shift, Key::Digit(7)]);
    let notice = app.notice.clone().expect("a notice");
    assert!(notice.error);
    assert_eq!(notice.text, text::NO_CARD.get(app.settings.language));
}

#[test]
fn screenshots_are_named_by_the_time_or_numbered() {
    assert_eq!(screenshot_name(None, Zone::Berlin, 1), "screenshot-1.bmp");
    assert_eq!(screenshot_name(None, Zone::Berlin, 12), "screenshot-12.bmp");
    // 14:03:27 in Berlin, in winter time; then a second one in the same second.
    let noon = quantus_ui::clock::to_utc(2026, 11, 12, 14, 3, Zone::Berlin) + 27_000;
    assert_eq!(screenshot_name(Some(noon), Zone::Berlin, 1), "2026-11-12_14-03-27.bmp");
    assert_eq!(screenshot_name(Some(noon), Zone::Berlin, 2), "2026-11-12_14-03-27-2.bmp");
    assert_eq!(screenshot_name(Some(noon), Zone::Utc, 1), "2026-11-12_13-03-27.bmp");
}
