//! The screen: a status bar, the history and the input at the bottom. A [`Frame`] is the screen
//! laid out once; the [`Renderer`] draws it band by band into a small buffer and sends only the
//! bands that changed, so no framebuffer is needed.

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};

use crate::app::{App, Mode};
use crate::font::Scale;
use crate::layout::{self, RowLayout};

pub const WIDTH: u32 = 480;
pub const HEIGHT: u32 = 320;
/// The height of a band; a band buffer takes WIDTH × BAND × 2 bytes.
pub const BAND: u32 = 32;
const BANDS: usize = HEIGHT.div_ceil(BAND) as usize;

const STATUS_HEIGHT: i32 = 26;
const MARGIN: i32 = 10;

/// A dark, muted palette after the desktop app's: nothing bright.
pub mod colors {
    use embedded_graphics::pixelcolor::{Rgb565, RgbColor};

    pub const fn rgb(r: u8, g: u8, b: u8) -> Rgb565 {
        Rgb565::new(r >> 3, g >> 2, b >> 3)
    }

    pub const BACKGROUND: Rgb565 = rgb(0x17, 0x16, 0x1d);
    pub const SURFACE: Rgb565 = rgb(0x1e, 0x1d, 0x27);
    pub const HAIRLINE: Rgb565 = rgb(0x32, 0x2f, 0x3f);
    pub const TEXT: Rgb565 = rgb(0xed, 0xeb, 0xf3);
    pub const MUTED: Rgb565 = rgb(0x8b, 0x87, 0xa3);
    pub const FAINT: Rgb565 = rgb(0x55, 0x50, 0x6a);
    pub const RESULT: Rgb565 = rgb(0xc9, 0xc4, 0xe0);
    pub const ERROR: Rgb565 = rgb(0xc9, 0x8a, 0x96);
    pub const SHIFT: Rgb565 = rgb(0xc9, 0xa4, 0x6a);
    pub const ALPHA: Rgb565 = rgb(0x8f, 0xb3, 0xa0);
    pub const PLACEHOLDER: Rgb565 = FAINT;

    /// `color` mixed toward the background, `amount` of 255 parts.
    pub fn darken(color: Rgb565, amount: u8) -> Rgb565 {
        fn mix(from: u8, to: u8, amount: u8) -> u8 {
            ((from as u16 * (255 - amount as u16) + to as u16 * amount as u16) / 255) as u8
        }
        let bg = BACKGROUND;
        Rgb565::new(
            mix(color.r(), bg.r(), amount),
            mix(color.g(), bg.g(), amount),
            mix(color.b(), bg.b(), amount),
        )
    }
}

/// Something to draw, at absolute screen coordinates.
pub enum Shape {
    Fill(Rectangle, Rgb565),
    /// Text with its baseline at the point.
    Text(Point, String, Scale, Rgb565),
    /// A laid out row with its origin at the point.
    Math(Point, RowLayout, Rgb565),
}

impl Shape {
    fn bounds(&self) -> Rectangle {
        match self {
            Shape::Fill(area, _) => *area,
            Shape::Text(origin, text, scale, _) => {
                let metrics = scale.metrics();
                Rectangle::new(
                    *origin - Point::new(0, metrics.ascent),
                    Size::new(scale.width(text) as u32, (metrics.ascent + metrics.descent) as u32),
                )
            }
            Shape::Math(origin, row, _) => row.bounds(*origin),
        }
    }

    fn draw<D: DrawTarget<Color = Rgb565>>(&self, target: &mut D) {
        match self {
            Shape::Fill(area, color) => {
                let _ = area.into_styled(PrimitiveStyle::with_fill(*color)).draw(target);
            }
            Shape::Text(origin, text, scale, color) => scale.draw(text, *origin, *color, target),
            Shape::Math(origin, row, color) => {
                layout::draw(row, *origin, *color, colors::PLACEHOLDER, target);
            }
        }
    }
}

/// The screen laid out, back to front.
pub struct Frame(pub Vec<Shape>);

impl Frame {
    pub fn new<B>(app: &App<B>) -> Frame {
        let mut shapes = Vec::new();
        let width = WIDTH as i32;

        // The input, at the bottom.
        let input = layout::layout(&app.editor.row, Scale::Normal);
        let input_height = (input.height() + 16).max(56);
        let input_top = HEIGHT as i32 - input_height;
        let baseline = input_top + 8 + input.ascent;
        let mut origin = Point::new(MARGIN, baseline);
        let caret = layout::caret(&input, origin, app.editor.path(), app.editor.pos());
        // Scroll a long input so the caret stays on the screen.
        let overflow = caret.top_left.x - (width - MARGIN);
        if overflow > 0 {
            origin.x -= overflow;
        }
        let caret = caret.translate(Point::new(origin.x - MARGIN, 0));
        shapes.push(Shape::Fill(full_width(input_top, 1), colors::HAIRLINE));

        // The history, newest at the bottom, as far up as there is room.
        let mut bottom = input_top - 6;
        for entry in app.history.iter().rev() {
            if bottom < STATUS_HEIGHT {
                break;
            }
            let result_height = match &entry.result {
                Ok(result) => {
                    let result = layout::layout(result, Scale::Normal);
                    let height = result.height();
                    let x = (width - MARGIN - result.width).max(MARGIN);
                    shapes.push(Shape::Math(Point::new(x, bottom - result.descent), result, colors::RESULT));
                    height
                }
                Err(message) => {
                    let metrics = Scale::Normal.metrics();
                    let x = (width - MARGIN - Scale::Normal.width(message)).max(MARGIN);
                    let origin = Point::new(x, bottom - metrics.descent);
                    shapes.push(Shape::Text(origin, message.clone(), Scale::Normal, colors::ERROR));
                    metrics.ascent + metrics.descent
                }
            };
            bottom -= result_height + 4;
            let entry_input = layout::layout(&entry.input, Scale::Normal);
            let height = entry_input.height();
            shapes.push(Shape::Math(Point::new(MARGIN, bottom - entry_input.descent), entry_input, colors::TEXT));
            bottom -= height + 6;
            shapes.push(Shape::Fill(full_width(bottom, 1), colors::HAIRLINE));
            bottom -= 5;
        }

        if !app.editor.is_empty() {
            shapes.push(Shape::Math(origin, input, colors::TEXT));
        }
        if app.caret_on && app.menu.is_none() {
            shapes.push(Shape::Fill(caret, colors::TEXT));
        }

        // The status bar over the history's top.
        shapes.push(Shape::Fill(full_width(0, STATUS_HEIGHT as u32), colors::darken(app.mode.color(), 110)));
        let status_baseline = STATUS_HEIGHT - 7;
        let mode = app.mode.name().to_string();
        shapes.push(Shape::Text(Point::new(MARGIN, status_baseline), mode, Scale::Small, colors::TEXT));
        if app.shift {
            let label = "SHIFT";
            let x = width - MARGIN - Scale::Small.width(label);
            shapes.push(Shape::Text(Point::new(x, status_baseline), label.to_string(), Scale::Small, colors::SHIFT));
        }

        if let Some(selected) = app.menu {
            menu(&mut shapes, selected);
        }
        Frame(shapes)
    }
}

fn full_width(top: i32, height: u32) -> Rectangle {
    Rectangle::new(Point::new(0, top), Size::new(WIDTH, height))
}

/// The mode menu over the history.
fn menu(shapes: &mut Vec<Shape>, selected: usize) {
    let line = 34;
    let top = STATUS_HEIGHT + 12;
    let area = Rectangle::new(
        Point::new(60, top),
        Size::new(WIDTH - 120, (line * Mode::ALL.len() as i32 + 12) as u32),
    );
    shapes.push(Shape::Fill(area, colors::HAIRLINE));
    shapes.push(Shape::Fill(area.offset(-2), colors::BACKGROUND));
    let metrics = Scale::Normal.metrics();
    for (index, mode) in Mode::ALL.iter().enumerate() {
        let y = top + 6 + line * index as i32;
        if index == selected {
            let row = Rectangle::new(Point::new(62, y), Size::new(WIDTH - 124, line as u32));
            shapes.push(Shape::Fill(row, colors::HAIRLINE));
        }
        let baseline = y + (line + metrics.digit) / 2;
        let label = format!("{}   {}", index + 1, mode.name());
        shapes.push(Shape::Text(Point::new(80, baseline), label, Scale::Normal, colors::TEXT));
    }
}

/// Draws frames band by band and sends the bands that changed since the last frame.
pub struct Renderer {
    band: Vec<Rgb565>,
    /// A hash of each band as last sent.
    sent: [Option<u64>; BANDS],
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer {
    pub fn new() -> Self {
        Renderer { band: vec![colors::BACKGROUND; (WIDTH * BAND) as usize], sent: [None; BANDS] }
    }

    /// Draws the whole screen again on the next frame.
    pub fn invalidate(&mut self) {
        self.sent = [None; BANDS];
    }

    /// Returns how many bands were sent.
    pub fn render<D: DrawTarget<Color = Rgb565>>(
        &mut self,
        frame: &Frame,
        display: &mut D,
    ) -> Result<usize, D::Error> {
        let mut sent = 0;
        for (index, last) in self.sent.iter_mut().enumerate() {
            let top = index as u32 * BAND;
            let area = Rectangle::new(
                Point::new(0, top as i32),
                Size::new(WIDTH, BAND.min(HEIGHT - top)),
            );
            let pixels = &mut self.band[..(area.size.width * area.size.height) as usize];
            pixels.fill(colors::BACKGROUND);
            let mut band = Band { pixels, area };
            for shape in &frame.0 {
                if !shape.bounds().intersection(&area).is_zero_sized() {
                    shape.draw(&mut band);
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
