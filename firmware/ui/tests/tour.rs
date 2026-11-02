//! The UI tour (`quantus_ui::tour`) without its pauses, with a wallpaper shown everywhere:
//! every screen on the way is drawn, and it ends where it should.

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use quantus_ui::app::{App, Mode, Screen, Settings};
use quantus_ui::backend::{Backend, Stub};
use quantus_ui::render::{Frame, Renderer, HEIGHT, WIDTH};
use quantus_ui::settings::WallpaperOn;
use quantus_ui::text::Lang;
use quantus_ui::tour::tour;

/// The stub, and an SD card with two pictures.
struct Card;

impl Backend for Card {
    fn evaluate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> Result<String, String> {
        Stub.evaluate(mode, settings, latex)
    }

    fn wallpapers(&mut self) -> Vec<String> {
        vec!["Abend".to_string(), "Morgen".to_string()]
    }

    fn wallpaper(&mut self, _name: &str) -> Option<Vec<u8>> {
        Some(bmp())
    }
}

/// A grey 480×320 24-bit BMP.
fn bmp() -> Vec<u8> {
    let (width, height) = (WIDTH, HEIGHT);
    let row = width * 3;
    let mut bytes = b"BM".to_vec();
    for value in [54 + row * height, 0, 54, 40, width, height] {
        bytes.extend(value.to_le_bytes());
    }
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(24u16.to_le_bytes());
    for value in [0, row * height, 2835, 2835, 0, 0] {
        bytes.extend(value.to_le_bytes());
    }
    bytes.resize(bytes.len() + (row * height) as usize, 0x80);
    bytes
}

/// Takes what the renderer sends and drops it.
struct Nowhere;

impl OriginDimensions for Nowhere {
    fn size(&self) -> Size {
        Size::new(WIDTH, HEIGHT)
    }
}

impl DrawTarget for Nowhere {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I: IntoIterator<Item = Pixel<Rgb565>>>(&mut self, _: I) -> Result<(), Self::Error> {
        Ok(())
    }
}

fn title(app: &App<Card>) -> String {
    match &app.screen {
        Screen::Menu(menu) => menu.title().get(Lang::De).to_string(),
        Screen::Modes { selected } => format!("MODE {}", selected + 1),
        Screen::Calc => format!("{} {}", app.mode.name(Lang::De), app.editor.to_latex()),
        Screen::Calibrate(..) => "calibration".to_string(),
    }
}

#[test]
fn the_tour_shows_the_wallpaper_everywhere_and_ends_on_the_tiles() {
    let mut app = App::new(Card);
    let mut renderer = Renderer::new();
    let mut shown_everywhere = false;
    for (index, step) in tour().iter().enumerate() {
        if let Some(note) = step.note {
            println!("-- {note}");
        }
        app.key(step.key);
        let frame = Frame::new(&app);
        renderer.render(&frame, &mut Nowhere).unwrap();
        println!("{index:3} {:?} -> {}", step.key, title(&app));
        if app.settings.wallpaper_on == WallpaperOn::Everywhere && app.preview.is_none() {
            shown_everywhere = true;
            assert!(frame.1.is_some(), "no wallpaper behind {}", title(&app));
        }
    }
    assert!(shown_everywhere);
    assert_eq!(app.wallpaper_name.as_deref(), Some("Abend"));
    assert_eq!(app.screen, Screen::Modes { selected: 0 });
    assert_eq!(app.mode, Mode::Physics);
    let inputs: Vec<String> = app.history.iter().map(|entry| entry.input.to_latex()).collect();
    assert_eq!(inputs.len(), 5, "{inputs:?}");
    assert!(app.history.iter().all(|entry| entry.result.is_ok()));
    // It only looks at the other settings and leaves them as they were.
    assert_eq!(app.settings, Settings { wallpaper_on: WallpaperOn::Everywhere, ..Settings::default() });
}

#[test]
fn the_tour_takes_minutes_like_a_person() {
    let total: u32 = tour().iter().map(|step| step.pause_ms).sum();
    println!("the tour takes {} s", total / 1000);
    assert!((60_000..300_000).contains(&total), "{total} ms");
}
