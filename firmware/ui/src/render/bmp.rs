//! The screen as a 24-bit BMP file (a screenshot): a fresh [`Renderer`] draws every band of a
//! [`Frame`] into a [`Writer`], which puts each band's rows where they belong in the file. The
//! file is bottom-up, as BMP files usually are, so it opens everywhere and works as a wallpaper;
//! no frame buffer is needed, only a seekable file.

use std::io::{self, Seek, SeekFrom, Write};

use embedded_graphics::pixelcolor::{Rgb565, Rgb888};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

use super::{Frame, Renderer, HEIGHT, WIDTH};
use crate::profile;

/// The bytes of the two headers.
pub const HEADER: u32 = 54;

/// The size of a screenshot file of the handheld's screen.
pub const SIZE: u32 = HEADER + WIDTH * 3 * HEIGHT;

/// The file and info headers of a 24-bit, bottom-up BMP of `width` × `height`.
pub fn header(width: u32, height: u32) -> [u8; HEADER as usize] {
    let row = (width * 3).div_ceil(4) * 4;
    let pixels = row * height;
    let mut bytes = [0; HEADER as usize];
    bytes[..2].copy_from_slice(b"BM");
    // File size, reserved, offset of the pixels, size of the info header, width, height.
    for (i, value) in [HEADER + pixels, 0, HEADER, 40, width, height].into_iter().enumerate() {
        bytes[2 + 4 * i..6 + 4 * i].copy_from_slice(&value.to_le_bytes());
    }
    // One plane, 24 bits a pixel.
    bytes[26..28].copy_from_slice(&1u16.to_le_bytes());
    bytes[28..30].copy_from_slice(&24u16.to_le_bytes());
    // No compression, the size of the pixels, 72 dpi both ways, no palette.
    for (i, value) in [0, pixels, 2835, 2835, 0, 0].into_iter().enumerate() {
        bytes[30 + 4 * i..34 + 4 * i].copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

/// Writes `frame` to `out` as a BMP file of the whole screen.
pub fn write<W: Write + Seek>(frame: &Frame, out: W) -> io::Result<()> {
    let mut writer = Writer::new(out)?;
    Renderer::new().render(frame, &mut writer)?;
    writer.out.flush()
}

/// The screen as a BMP file under construction: what is drawn goes to its place in `out`. A
/// band of the whole width (all the renderer sends) is one write.
pub struct Writer<W> {
    out: W,
    bytes: Vec<u8>,
    /// The panel's size, from the profile in force.
    width: u32,
    height: u32,
    /// The bytes of one row: three a pixel, padded to four.
    row: u32,
}

impl<W: Write + Seek> Writer<W> {
    /// Writes the header to `out`, at its start.
    pub fn new(out: W) -> io::Result<Self> {
        Self::sized(out, profile::get().physical())
    }

    /// [`Writer::new`] for a panel of `size` rather than the profile's: the Duo's input screen.
    pub fn sized(mut out: W, size: Size) -> io::Result<Self> {
        let Size { width, height } = size;
        out.seek(SeekFrom::Start(0))?;
        out.write_all(&header(width, height))?;
        Ok(Writer { out, bytes: Vec::new(), width, height, row: (width * 3).div_ceil(4) * 4 })
    }

    /// The file.
    pub fn into_inner(self) -> W {
        self.out
    }

    /// Where the pixel `(x, y)` of the screen lies in the file.
    fn offset(&self, x: u32, y: u32) -> u64 {
        u64::from(HEADER + (self.height - 1 - y) * self.row + x * 3)
    }
}

/// Blue, green, red: the order of a 24-bit BMP.
fn bgr(color: Rgb565) -> [u8; 3] {
    let color = Rgb888::from(color);
    [color.b(), color.g(), color.r()]
}

impl<W> OriginDimensions for Writer<W> {
    fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }
}

impl<W: Write + Seek> DrawTarget for Writer<W> {
    type Color = Rgb565;
    type Error = io::Error;

    fn draw_iter<I: IntoIterator<Item = Pixel<Rgb565>>>(&mut self, pixels: I) -> io::Result<()> {
        for Pixel(point, color) in pixels {
            let (Ok(x), Ok(y)) = (u32::try_from(point.x), u32::try_from(point.y)) else { continue };
            if x < self.width && y < self.height {
                self.out.seek(SeekFrom::Start(self.offset(x, y)))?;
                self.out.write_all(&bgr(color))?;
            }
        }
        Ok(())
    }

    fn fill_contiguous<I: IntoIterator<Item = Rgb565>>(&mut self, area: &Rectangle, colors: I) -> io::Result<()> {
        let screen = self.bounding_box();
        let whole = area.top_left.x == 0 && area.size.width == self.width && screen.intersection(area) == *area;
        if !whole {
            return self.draw_iter(area.points().zip(colors).map(|(point, color)| Pixel(point, color)));
        }
        // The rows of the band, bottom one first, as they lie in the file.
        let rows = area.size.height as usize;
        self.bytes.clear();
        let (width, row_bytes) = (self.width as usize, self.row as usize);
        self.bytes.resize(rows * row_bytes, 0);
        for (i, color) in colors.into_iter().take(rows * width).enumerate() {
            let (row, x) = (i / width, i % width);
            let at = (rows - 1 - row) * row_bytes + 3 * x;
            self.bytes[at..at + 3].copy_from_slice(&bgr(color));
        }
        let bottom = area.top_left.y as u32 + area.size.height - 1;
        self.out.seek(SeekFrom::Start(self.offset(0, bottom)))?;
        self.out.write_all(&self.bytes)
    }
}
