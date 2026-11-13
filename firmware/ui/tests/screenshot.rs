use std::io::Cursor;

use embedded_graphics::image::Image;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use quantus_ui::app::App;
use quantus_ui::backend::Stub;
use quantus_ui::key::Key;
use quantus_ui::render::{bmp, Frame, Renderer, HEIGHT, WIDTH};
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
