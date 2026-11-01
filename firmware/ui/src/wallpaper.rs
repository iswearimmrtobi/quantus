//! A picture from the SD card behind the screens (design/handheld-menus.md §7.6). The shell
//! reads the file; this decodes it once into the screen's pixels, at full strength for the mode
//! tiles and dimmed for the menus and the calculation, where text stands on it.

use std::sync::Arc;

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use tinybmp::Bmp;

use crate::render::{colors, HEIGHT, WIDTH};

/// How much of the picture is left in its dimmed form, out of 256.
const DIM: u16 = 80;

/// A screen-sized picture, row by row from the top left; cheap to clone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wallpaper {
    pub full: Arc<[Rgb565]>,
    pub dim: Arc<[Rgb565]>,
}

impl Wallpaper {
    /// The picture from `pixels`, which must fill the screen.
    pub fn new(pixels: Vec<Rgb565>) -> Option<Wallpaper> {
        if pixels.len() != (WIDTH * HEIGHT) as usize {
            return None;
        }
        let dim = pixels.iter().map(|&pixel| blend(colors::BACKGROUND, pixel, DIM)).collect();
        Some(Wallpaper { full: pixels.into(), dim })
    }

    /// A BMP file of the screen's size, 480 × 320, in any of the formats BMP has.
    pub fn from_bmp(bytes: &[u8]) -> Option<Wallpaper> {
        let bmp = Bmp::<Rgb565>::from_slice(bytes).ok()?;
        if bmp.size() != Size::new(WIDTH, HEIGHT) {
            return None;
        }
        let mut pixels = vec![colors::BACKGROUND; (WIDTH * HEIGHT) as usize];
        for Pixel(point, color) in bmp.pixels() {
            if let Some(pixel) = pixels.get_mut((point.y * WIDTH as i32 + point.x) as usize) {
                *pixel = color;
            }
        }
        Wallpaper::new(pixels)
    }
}

/// `over` laid on `under`, `alpha` of 256 of it showing.
pub fn blend(under: Rgb565, over: Rgb565, alpha: u16) -> Rgb565 {
    let mix = |a: u8, b: u8| ((u16::from(a) * (256 - alpha) + u16::from(b) * alpha) >> 8) as u8;
    Rgb565::new(mix(under.r(), over.r()), mix(under.g(), over.g()), mix(under.b(), over.b()))
}
