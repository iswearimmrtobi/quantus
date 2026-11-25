//! The Duo's main screen (firmware/docs/duo.md): the layout on its logical 720×405 canvas, drawn
//! on the 1280×720 panel with the larger fonts.

use std::io::Cursor;

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use quantus_ui::app::App;
use quantus_ui::backend::Stub;
use quantus_ui::key::Key;
use quantus_ui::profile::{self, Profile};
use quantus_ui::render::{bmp, colors, Frame, Renderer};
use quantus_ui::tour::tour;

const PANEL: Size = Size::new(1280, 720);

/// The panel in memory; pixels drawn outside it are counted.
struct Panel {
    pixels: Vec<Rgb565>,
    outside: usize,
}

impl Panel {
    fn new() -> Self {
        Panel { pixels: vec![Rgb565::BLACK; (PANEL.width * PANEL.height) as usize], outside: 0 }
    }

    /// How many pixels in `area` are not the background.
    fn drawn_in(&self, area: Rectangle) -> usize {
        area.points().filter(|point| self.pixels[(point.y as u32 * PANEL.width + point.x as u32) as usize] != colors::background()).count()
    }
}

impl OriginDimensions for Panel {
    fn size(&self) -> Size {
        PANEL
    }
}

impl DrawTarget for Panel {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I: IntoIterator<Item = Pixel<Rgb565>>>(&mut self, pixels: I) -> Result<(), Self::Error> {
        for Pixel(point, color) in pixels {
            match (u32::try_from(point.x), u32::try_from(point.y)) {
                (Ok(x), Ok(y)) if x < PANEL.width && y < PANEL.height => self.pixels[(y * PANEL.width + x) as usize] = color,
                _ => self.outside += 1,
            }
        }
        Ok(())
    }
}

#[test]
fn the_profile_scales_the_canvas_to_the_panel() {
    assert_eq!(Profile::DUO.physical(), PANEL);
    assert_eq!(Profile::HANDHELD.physical(), Size::new(480, 320));
    assert_eq!(Profile::DUO_BENCH.physical(), Size::new(1280, 800));
    let rect = Profile::DUO.rect(Rectangle::new(Point::new(9, 9), Size::new(9, 18)));
    assert_eq!(rect, Rectangle::new(Point::new(16, 16), Size::new(16, 32)));
}

#[test]
fn a_calculation_fills_the_panel() {
    profile::set(Profile::DUO);
    let mut app = App::new(Stub);
    for key in [Key::Digit(1), Key::Plus, Key::Digit(2), Key::Exe] {
        app.key(key);
    }
    let mut panel = Panel::new();
    let sent = Renderer::new().render(&Frame::new(&app), &mut panel).unwrap();
    assert_eq!(sent, 23, "every band of 32 rows");
    assert_eq!(panel.outside, 0);
    // The result at the bottom right of the panel, not in the handheld's 480×320 corner.
    let right = Rectangle::new(Point::new(960, 360), Size::new(320, 360));
    assert!(panel.drawn_in(right) > 100, "nothing at the bottom right");
}

#[test]
fn the_tour_draws_every_screen_on_the_panel() {
    profile::set(Profile::DUO);
    let mut app = App::new(Stub);
    let mut renderer = Renderer::new();
    let mut panel = Panel::new();
    for step in tour() {
        app.key(step.key);
        renderer.render(&Frame::new(&app), &mut panel).unwrap();
    }
    assert_eq!(panel.outside, 0);
}

#[test]
fn a_screenshot_is_the_panel_size() {
    profile::set(Profile::DUO);
    let app = App::new(Stub);
    let mut file = Cursor::new(Vec::new());
    bmp::write(&Frame::new(&app), &mut file).unwrap();
    let bytes = file.into_inner();
    assert_eq!(&bytes[18..22], &1280u32.to_le_bytes());
    assert_eq!(&bytes[22..26], &720u32.to_le_bytes());
    assert_eq!(bytes.len() as u32, bmp::HEADER + 1280 * 3 * 720);
}

#[test]
fn tiles_and_sheets_fill_the_height() {
    use quantus_ui::render::{sheet_rows, tile_rect};
    profile::set(Profile::HANDHELD);
    assert_eq!(tile_rect(0).size.height, 116);
    assert_eq!(sheet_rows(), 5);
    profile::set(Profile::DUO);
    // Three rows of four tiles down to the page dots, near the bottom of the 405 high canvas.
    assert_eq!(quantus_ui::app::tiles_per_page(), 12);
    assert_eq!(tile_rect(3).top_left.y, tile_rect(0).top_left.y, "four to a row");
    assert!(tile_rect(11).bottom_right().unwrap().y > 350, "{:?}", tile_rect(11));
    assert_eq!(sheet_rows(), 7);
    assert_eq!(quantus_ui::sheet::grid_width(), 700);
}
