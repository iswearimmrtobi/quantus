//! A picture from the SD card behind the screens (design/handheld-menus.md §7.6). The shell
//! reads the file; this decodes it once into the screen's pixels, at full strength for the mode
//! tiles and dimmed for the menus and the calculation, where text stands on it. A board without
//! the RAM for that keeps both forms in flash instead, as raw pixels ([`Wallpaper::from_raw`]).

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use embedded_graphics::pixelcolor::raw::RawU16;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::image::ImageDrawable;
use embedded_graphics::prelude::*;
use tinybmp::Bmp;

use crate::render::{colors, HEIGHT, WIDTH};

/// How much of the picture is left in its dimmed form, out of 256.
const DIM: u16 = 80;

/// The pixels of a screen-sized picture, row by row from the top left; cheap to clone.
#[derive(Clone, PartialEq, Eq)]
pub enum Pixels {
    /// Decoded into RAM, with a number no other picture gets.
    Ram(Arc<[Rgb565]>, u32),
    /// Raw RGB565, two bytes a pixel, little-endian: in flash, read while drawing.
    Raw(&'static [u8]),
}

/// Names the picture without its pixels: what the renderer compares from frame to frame.
impl core::fmt::Debug for Pixels {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Pixels::Ram(_, number) => write!(f, "Ram({number})"),
            Pixels::Raw(bytes) => write!(f, "Raw({:p})", bytes.as_ptr()),
        }
    }
}

/// Numbers the pictures decoded into RAM.
static DECODED: AtomicU32 = AtomicU32::new(0);

fn ram(pixels: Vec<Rgb565>) -> Pixels {
    Pixels::Ram(pixels.into(), DECODED.fetch_add(1, Ordering::Relaxed))
}

impl Pixels {
    pub fn get(&self, index: usize) -> Rgb565 {
        match self {
            Pixels::Ram(pixels, _) => pixels[index],
            Pixels::Raw(bytes) => raw(bytes[2 * index], bytes[2 * index + 1]),
        }
    }

    /// Copies the pixels from `start` on into `out`.
    pub fn copy_to(&self, start: usize, out: &mut [Rgb565]) {
        match self {
            Pixels::Ram(pixels, _) => out.copy_from_slice(&pixels[start..start + out.len()]),
            Pixels::Raw(bytes) => {
                let bytes = &bytes[2 * start..2 * (start + out.len())];
                for (pixel, pair) in out.iter_mut().zip(bytes.chunks_exact(2)) {
                    *pixel = raw(pair[0], pair[1]);
                }
            }
        }
    }

    /// As raw RGB565, the form [`Pixels::Raw`] reads.
    pub fn to_raw(&self) -> Vec<u8> {
        (0..(WIDTH * HEIGHT) as usize).flat_map(|index| self.get(index).into_storage().to_le_bytes()).collect()
    }
}

fn raw(low: u8, high: u8) -> Rgb565 {
    Rgb565::from(RawU16::new(u16::from_le_bytes([low, high])))
}

/// A screen-sized picture, at full strength and dimmed; cheap to clone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wallpaper {
    pub full: Pixels,
    pub dim: Pixels,
}

impl Wallpaper {
    /// The picture from `pixels`, which must fill the screen.
    pub fn new(pixels: Vec<Rgb565>) -> Option<Wallpaper> {
        if pixels.len() != (WIDTH * HEIGHT) as usize {
            return None;
        }
        let dim: Vec<Rgb565> = pixels.iter().map(|&pixel| blend(colors::BACKGROUND, pixel, DIM)).collect();
        Some(Wallpaper { full: ram(pixels), dim: ram(dim) })
    }

    /// A picture kept in flash: both forms as raw RGB565 ([`Pixels::to_raw`]).
    pub fn from_raw(full: &'static [u8], dim: &'static [u8]) -> Option<Wallpaper> {
        let size = 2 * (WIDTH * HEIGHT) as usize;
        (full.len() == size && dim.len() == size).then_some(Wallpaper { full: Pixels::Raw(full), dim: Pixels::Raw(dim) })
    }

    /// A BMP file of the screen's size, 480 × 320, in any of the formats BMP has.
    pub fn from_bmp(bytes: &[u8]) -> Option<Wallpaper> {
        let bmp = Bmp::<Rgb565>::from_slice(bytes).ok()?;
        if bmp.size() != Size::new(WIDTH, HEIGHT) {
            return None;
        }
        // Drawn rather than read with `Bmp::pixels`, which in tinybmp 0.7 turns a bottom-up
        // file (the usual kind) upside down.
        let mut screen = Screen(vec![colors::BACKGROUND; (WIDTH * HEIGHT) as usize]);
        bmp.draw(&mut screen).ok()?;
        Wallpaper::new(screen.0)
    }
}

/// The screen's pixels as a draw target.
struct Screen(Vec<Rgb565>);

impl OriginDimensions for Screen {
    fn size(&self) -> Size {
        Size::new(WIDTH, HEIGHT)
    }
}

impl DrawTarget for Screen {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I: IntoIterator<Item = Pixel<Rgb565>>>(&mut self, pixels: I) -> Result<(), Self::Error> {
        for Pixel(point, color) in pixels {
            if (0..WIDTH as i32).contains(&point.x) && (0..HEIGHT as i32).contains(&point.y) {
                self.0[(point.y * WIDTH as i32 + point.x) as usize] = color;
            }
        }
        Ok(())
    }
}

/// `over` laid on `under`, `alpha` of 256 of it showing.
pub fn blend(under: Rgb565, over: Rgb565, alpha: u16) -> Rgb565 {
    let mix = |a: u8, b: u8| ((u16::from(a) * (256 - alpha) + u16::from(b) * alpha) >> 8) as u8;
    Rgb565::new(mix(under.r(), over.r()), mix(under.g(), over.g()), mix(under.b(), over.b()))
}
