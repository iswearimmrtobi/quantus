//! Drawing a [`Frame`] band by band and sending only the bands that changed.

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyleBuilder, Rectangle, RoundedRectangle, StrokeAlignment};

use super::{colors, Frame, Shape, BAND, BANDS, HEIGHT, WIDTH};
use crate::wallpaper::blend;

/// Draws frames band by band and sends the bands that changed since the last frame. A band
/// whose shapes and backdrop are the same as last time is not even drawn.
pub struct Renderer {
    band: Vec<Rgb565>,
    /// A hash of each band as last sent.
    sent: [Option<u64>; BANDS],
    /// A hash of what each band was last drawn from: the backdrop and its shapes.
    drawn: [Option<u64>; BANDS],
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer {
    pub fn new() -> Self {
        Renderer { band: vec![colors::BACKGROUND; (WIDTH * BAND) as usize], sent: [None; BANDS], drawn: [None; BANDS] }
    }

    /// Draws the whole screen again on the next frame.
    pub fn invalidate(&mut self) {
        self.sent = [None; BANDS];
        self.drawn = [None; BANDS];
    }

    /// Returns how many bands were sent.
    pub fn render<D: DrawTarget<Color = Rgb565>>(
        &mut self,
        frame: &Frame,
        display: &mut D,
    ) -> Result<usize, D::Error> {
        let mut sent = 0;
        let backdrop = fingerprint(&frame.1);
        let shapes: Vec<(Rectangle, u64)> = frame.0.iter().map(|shape| (shape.bounds(), fingerprint(shape))).collect();
        for (index, last) in self.sent.iter_mut().enumerate() {
            let top = index as u32 * BAND;
            let area = Rectangle::new(
                Point::new(0, top as i32),
                Size::new(WIDTH, BAND.min(HEIGHT - top)),
            );
            let inputs = shapes
                .iter()
                .filter(|(bounds, _)| !bounds.intersection(&area).is_zero_sized())
                .fold(backdrop, |hash, &(_, shape)| mix(hash, shape));
            if self.drawn[index] == Some(inputs) {
                continue;
            }
            self.drawn[index] = Some(inputs);
            let pixels = &mut self.band[..(area.size.width * area.size.height) as usize];
            match &frame.1 {
                Some(backdrop) => backdrop.copy_to((top * WIDTH) as usize, pixels),
                None => pixels.fill(colors::BACKGROUND),
            }
            let mut band = Band { pixels, area };
            for shape in &frame.0 {
                if shape.bounds().intersection(&area).is_zero_sized() {
                    continue;
                }
                match shape {
                    Shape::Glass(round, fill, alpha, outline) => {
                        band.glass(round, *fill, *alpha);
                        if let Some(outline) = outline {
                            let style = PrimitiveStyleBuilder::new()
                                .stroke_color(*outline)
                                .stroke_width(2)
                                .stroke_alignment(StrokeAlignment::Inside)
                                .build();
                            let _ = round.into_styled(style).draw(&mut band);
                        }
                    }
                    shape => shape.draw(&mut band),
                }
            }
            let hash = hash(band.pixels);
            if *last != Some(hash) {
                display.fill_contiguous(&area, band.pixels.iter().copied())?;
                *last = Some(hash);
                sent += 1;
            }
        }
        Ok(sent)
    }
}

/// FNV-1a over the `Debug` text of `value`.
fn fingerprint(value: &impl core::fmt::Debug) -> u64 {
    struct Fnv(u64);
    impl core::fmt::Write for Fnv {
        fn write_str(&mut self, text: &str) -> core::fmt::Result {
            self.0 = text.bytes().fold(self.0, |hash, byte| (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3));
            Ok(())
        }
    }
    let mut fnv = Fnv(0xcbf2_9ce4_8422_2325);
    let _ = core::fmt::write(&mut fnv, format_args!("{value:?}"));
    fnv.0
}

fn mix(hash: u64, value: u64) -> u64 {
    (hash ^ value).wrapping_mul(0x0100_0000_01b3).rotate_left(29)
}

/// FNV-1a over the pixels.
fn hash(pixels: &[Rgb565]) -> u64 {
    pixels.iter().fold(0xcbf2_9ce4_8422_2325, |hash, pixel| {
        (hash ^ u64::from(pixel.into_storage())).wrapping_mul(0x0100_0000_01b3)
    })
}

/// One band of the screen as a draw target; what falls outside is dropped.
struct Band<'a> {
    pixels: &'a mut [Rgb565],
    area: Rectangle,
}

impl Band<'_> {
    /// Lays `fill` over the pixels inside `round`: whole rows at a time, with the costly test
    /// for the rounding only in the corners.
    fn glass(&mut self, round: &RoundedRectangle, fill: Rgb565, alpha: u16) {
        let area = round.rectangle.intersection(&self.area);
        let Some(bottom_right) = area.bottom_right() else { return };
        let outer = round.rectangle;
        let corner = round.corners.top_left;
        let (corner_width, corner_height) = (corner.width as i32, corner.height as i32);
        let inner_left = outer.top_left.x + corner_width;
        let inner_right = outer.top_left.x + outer.size.width as i32 - corner_width;
        let inner_top = outer.top_left.y + corner_height;
        let inner_bottom = outer.top_left.y + outer.size.height as i32 - corner_height;
        let width = self.area.size.width as i32;
        let top = self.area.top_left.y;
        for y in area.top_left.y..=bottom_right.y {
            let start = ((y - top) * width) as usize;
            let row = &mut self.pixels[start..start + width as usize];
            let rounded = y < inner_top || y >= inner_bottom;
            for x in area.top_left.x..=bottom_right.x {
                if rounded && (x < inner_left || x >= inner_right) && !round.contains(Point::new(x, y)) {
                    continue;
                }
                let pixel = &mut row[x as usize];
                *pixel = blend(*pixel, fill, alpha);
            }
        }
    }
}

impl Dimensions for Band<'_> {
    fn bounding_box(&self) -> Rectangle {
        self.area
    }
}

impl DrawTarget for Band<'_> {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Rgb565>>,
    {
        let width = self.area.size.width as i32;
        let top = self.area.top_left.y;
        for Pixel(point, color) in pixels {
            if self.area.contains(point) {
                self.pixels[((point.y - top) * width + point.x) as usize] = color;
            }
        }
        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Rgb565) -> Result<(), Self::Error> {
        let area = area.intersection(&self.area);
        let width = self.area.size.width as i32;
        let top = self.area.top_left.y;
        for y in area.rows() {
            let start = ((y - top) * width + area.top_left.x) as usize;
            self.pixels[start..start + area.size.width as usize].fill(color);
        }
        Ok(())
    }
}
