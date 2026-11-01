//! The screen: a top bar over one calculation, the mode menu or the setup page, in the
//! desktop's dark palette with its purple accents (design/handheld-menus.md §7.6). A [`Frame`] is the screen
//! laid out once; the [`Renderer`] draws it band by band into a small buffer and sends only the
//! bands that changed, so no framebuffer is needed.

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{
    Circle, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, RoundedRectangle, StrokeAlignment, Triangle,
};

use crate::app::{App, Mode, Screen, MENU_COLUMNS, TILES_PER_PAGE};
use crate::font::Scale;
use crate::layout::{self, RowLayout};

pub const WIDTH: u32 = 480;
pub const HEIGHT: u32 = 320;
/// The height of a band; a band buffer takes WIDTH × BAND × 2 bytes.
pub const BAND: u32 = 32;
const BANDS: usize = HEIGHT.div_ceil(BAND) as usize;

/// The height of the top bar.
const BAR: i32 = 26;
const MARGIN: i32 = 10;

/// The desktop app's dark palette (`desktop/src/styles.css`): nothing bright.
pub mod colors {
    use embedded_graphics::pixelcolor::Rgb565;

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
    /// The desktop's `--cyan`, a lavender: the caret, the menu highlight, the chosen tile.
    pub const CYAN: Rgb565 = rgb(0x91, 0x86, 0xe8);
    /// The desktop's `--amber`, the accent.
    pub const AMBER: Rgb565 = rgb(0xd8, 0xa9, 0xff);
    /// Their dim forms, for fills: the top bar, the tiles, the selected line.
    pub const CYAN_DIM: Rgb565 = rgb(0x42, 0x3a, 0x63);
    pub const AMBER_DIM: Rgb565 = rgb(0x4e, 0x3c, 0x63);

}

/// Something to draw, at absolute screen coordinates.
pub enum Shape {
    Fill(Rectangle, Rgb565),
    /// A rounded box, filled and with an optional outline drawn inside it.
    Round(RoundedRectangle, Rgb565, Option<Rgb565>),
    Triangle(Triangle, Rgb565),
    Circle(Circle, Rgb565),
    /// Text with its baseline at the point.
    Text(Point, String, Scale, Rgb565),
    /// A laid out row with its origin at the point.
    Math(Point, RowLayout, Rgb565),
}

impl Shape {
    fn bounds(&self) -> Rectangle {
        match self {
            Shape::Fill(area, _) => *area,
            Shape::Round(round, _, _) => round.rectangle,
            Shape::Triangle(triangle, _) => triangle.bounding_box(),
            Shape::Circle(circle, _) => circle.bounding_box(),
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
            Shape::Round(round, fill, outline) => {
                let mut style = PrimitiveStyleBuilder::new().fill_color(*fill);
                if let Some(outline) = outline {
                    style = style
                        .stroke_color(*outline)
                        .stroke_width(2)
                        .stroke_alignment(StrokeAlignment::Inside);
                }
                let _ = round.into_styled(style.build()).draw(target);
            }
            Shape::Triangle(triangle, color) => {
                let _ = triangle.into_styled(PrimitiveStyle::with_fill(*color)).draw(target);
            }
            Shape::Circle(circle, color) => {
                let _ = circle.into_styled(PrimitiveStyle::with_fill(*color)).draw(target);
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
        match app.screen {
            Screen::Calc => calculation(&mut shapes, app),
            Screen::Menu { selected } => menu(&mut shapes, selected),
            Screen::Setup { selected } => setup(&mut shapes, app, selected),
        }
        bar(&mut shapes, app);
        Frame(shapes)
    }
}

fn full_width(top: i32, height: u32) -> Rectangle {
    Rectangle::new(Point::new(0, top), Size::new(WIDTH, height))
}

fn rounded(top_left: Point, size: Size, radius: u32) -> RoundedRectangle {
    RoundedRectangle::with_equal_corners(Rectangle::new(top_left, size), Size::new(radius, radius))
}

/// The top bar: the mode on the left; on the right SHIFT and ALPHA while
/// they are on, the digits, exact or decimal and the battery.
fn bar<B>(shapes: &mut Vec<Shape>, app: &App<B>) {
    let setup = matches!(app.screen, Screen::Setup { .. });
    shapes.push(Shape::Fill(full_width(0, BAR as u32), colors::CYAN_DIM));
    let baseline = BAR - 7;
    let title = if setup { "Setup" } else { app.mode.name() };
    shapes.push(Shape::Text(Point::new(MARGIN, baseline), title.to_string(), Scale::Small, colors::TEXT));

    // From the right edge inwards.
    let mut right = WIDTH as i32 - MARGIN;
    right -= battery(shapes, app.battery, Point::new(right, BAR / 2)) + 12;
    let form = if app.settings.decimal { "Dec" } else { "Exact" };
    let digits = format!("Sig {}", app.settings.digits);
    for text in [form.to_string(), digits] {
        let width = Scale::Small.width(&text);
        shapes.push(Shape::Text(Point::new(right - width, baseline), text, Scale::Small, colors::TEXT));
        right -= width + 12;
    }
    for (on, label, color) in [(app.alpha, "ALPHA", colors::ALPHA), (app.shift, "SHIFT", colors::SHIFT)] {
        if !on {
            continue;
        }
        let width = Scale::Small.width(label);
        let tag = rounded(Point::new(right - width - 8, 4), Size::new(width as u32 + 8, BAR as u32 - 8), 3);
        shapes.push(Shape::Round(tag, color, None));
        let at = Point::new(right - width - 4, baseline);
        shapes.push(Shape::Text(at, label.to_string(), Scale::Small, colors::BACKGROUND));
        right -= width + 8 + 8;
    }
}

/// A battery icon ending at `right`, centred on `center.y`, filled by the charge; returns its width.
fn battery(shapes: &mut Vec<Shape>, charge: Option<u8>, right_center: Point) -> i32 {
    let (width, height) = (22, 11);
    let top_left = Point::new(right_center.x - width, right_center.y - height / 2);
    let nub = Rectangle::new(Point::new(right_center.x - 2, right_center.y - 2), Size::new(2, 5));
    shapes.push(Shape::Fill(nub, colors::TEXT));
    let body = Rectangle::new(top_left, Size::new(width as u32 - 2, height as u32));
    shapes.push(Shape::Fill(body, colors::TEXT));
    shapes.push(Shape::Fill(body.offset(-1), colors::BACKGROUND));
    if let Some(charge) = charge {
        let inner = body.offset(-2);
        let filled = inner.size.width * u32::from(charge.min(100)) / 100;
        let color = if charge < 15 { colors::ERROR } else { colors::TEXT };
        shapes.push(Shape::Fill(Rectangle::new(inner.top_left, Size::new(filled, inner.size.height)), color));
    }
    width
}

/// One calculation, as on a Casio: the input from the top left, its result at the bottom right.
fn calculation<B>(shapes: &mut Vec<Shape>, app: &App<B>) {
    let width = WIDTH as i32;
    let height = HEIGHT as i32;

    let input = layout::layout(&app.editor.row, Scale::Normal);
    let mut origin = Point::new(MARGIN, BAR + 12 + input.ascent);
    let caret = layout::caret(&input, origin, app.editor.path(), app.editor.pos());
    // Scroll a long input so the caret stays on the screen.
    let overflow = caret.top_left.x - (width - MARGIN);
    if overflow > 0 {
        origin.x -= overflow;
    }
    let caret = caret.translate(Point::new(origin.x - MARGIN, 0));
    if !app.editor.is_empty() {
        shapes.push(Shape::Math(origin, input, colors::TEXT));
    }
    if app.caret_on && app.shown.is_none() {
        shapes.push(Shape::Fill(caret, colors::CYAN));
    }

    if let Some(entry) = app.shown_entry() {
        let bottom = height - MARGIN;
        match &entry.result {
            Ok(result) => {
                let result = layout::layout(result, Scale::Normal);
                let x = (width - MARGIN - result.width).max(MARGIN);
                shapes.push(Shape::Math(Point::new(x, bottom - result.descent), result, colors::RESULT));
            }
            Err(message) => {
                let metrics = Scale::Normal.metrics();
                let x = (width - MARGIN - Scale::Normal.width(message)).max(MARGIN);
                let at = Point::new(x, bottom - metrics.descent);
                shapes.push(Shape::Text(at, message.to_string(), Scale::Normal, colors::ERROR));
            }
        }
    }

    // How many calculations ↑ can bring back.
    let earlier = app.history.len() - app.shown.map_or(0, |shown| shown + 1);
    if earlier > 0 {
        let baseline = height - MARGIN - 2;
        let tip = Point::new(MARGIN + 5, baseline - 10);
        let arrow = Triangle::new(tip, Point::new(MARGIN, baseline), Point::new(MARGIN + 10, baseline));
        shapes.push(Shape::Triangle(arrow, colors::FAINT));
        let count = earlier.to_string();
        shapes.push(Shape::Text(Point::new(MARGIN + 15, baseline), count, Scale::Small, colors::FAINT));
    }
}

/// The mode menu: pages of tiles, numbered over all pages.
fn menu(shapes: &mut Vec<Shape>, selected: usize) {
    let gap = 8;
    let rows = (TILES_PER_PAGE / MENU_COLUMNS) as i32;
    let columns = MENU_COLUMNS as i32;
    let tile_width = (WIDTH as i32 - 2 * MARGIN - (columns - 1) * gap) / columns;
    let tile_height = 116;
    let top = BAR + 10;
    let page = selected / TILES_PER_PAGE;
    let first = page * TILES_PER_PAGE;
    let modes = &Mode::ALL[first..(first + TILES_PER_PAGE).min(Mode::ALL.len())];
    let small = Scale::Small.metrics();
    for (offset, mode) in modes.iter().enumerate() {
        let index = first + offset;
        let column = (offset % MENU_COLUMNS) as i32;
        let row = (offset / MENU_COLUMNS) as i32;
        let at = Point::new(MARGIN + column * (tile_width + gap), top + row * (tile_height + gap));
        let size = Size::new(tile_width as u32, tile_height as u32);
        let chosen = index == selected;
        let fill = if chosen { colors::AMBER_DIM } else { colors::CYAN_DIM };
        shapes.push(Shape::Round(rounded(at, size, 6), fill, chosen.then_some(colors::CYAN)));
        let number = (index + 1).to_string();
        shapes.push(Shape::Text(at + Point::new(12, 10 + small.ascent), number, Scale::Small, colors::TEXT));
        // The name at the bottom, a word per line when it is too wide.
        let room = tile_width - 24;
        let name = mode.name();
        let lines: Vec<&str> =
            if Scale::Normal.width(name) <= room { vec![name] } else { name.split(' ').collect() };
        let scale = Scale::Normal;
        let metrics = scale.metrics();
        let line_height = metrics.ascent + metrics.descent - 2;
        let mut baseline = at.y + tile_height - 12 - metrics.descent;
        for line in lines.iter().rev() {
            shapes.push(Shape::Text(Point::new(at.x + 12, baseline), line.to_string(), scale, colors::TEXT));
            baseline -= line_height;
        }
    }

    let pages = Mode::ALL.len().div_ceil(TILES_PER_PAGE);
    if pages > 1 {
        let y = top + rows * (tile_height + gap) + 10;
        let dot = 8;
        let start = (WIDTH as i32 - (pages as i32 * (dot + 8) - 8)) / 2;
        for index in 0..pages {
            let color = if index == page { colors::TEXT } else { colors::FAINT };
            let circle = Circle::new(Point::new(start + index as i32 * (dot + 8), y), dot as u32);
            shapes.push(Shape::Circle(circle, color));
        }
    }
}

/// The setup page: a line per setting, ←/→ change the selected one.
fn setup<B>(shapes: &mut Vec<Shape>, app: &App<B>, selected: usize) {
    let line = 44;
    let gap = 8;
    let metrics = Scale::Normal.metrics();
    let rows = [
        ("Significant digits", app.settings.digits.to_string()),
        ("Results", if app.settings.decimal { "Decimal" } else { "Exact" }.to_string()),
    ];
    for (index, (label, value)) in rows.into_iter().enumerate() {
        let y = BAR + 12 + index as i32 * (line + gap);
        let size = Size::new(WIDTH - 2 * MARGIN as u32, line as u32);
        let chosen = index == selected;
        let fill = if chosen { colors::CYAN_DIM } else { colors::SURFACE };
        shapes.push(Shape::Round(rounded(Point::new(MARGIN, y), size, 6), fill, None));
        let baseline = y + (line + metrics.digit) / 2;
        let number = (index + 1).to_string();
        shapes.push(Shape::Text(Point::new(MARGIN + 12, baseline), number, Scale::Normal, colors::MUTED));
        shapes.push(Shape::Text(Point::new(MARGIN + 40, baseline), label.to_string(), Scale::Normal, colors::TEXT));

        // The value at the right, between arrows on the selected line.
        let width = Scale::Normal.width(&value);
        let right = WIDTH as i32 - MARGIN - 14;
        let middle = y + line / 2;
        let color = if chosen { colors::TEXT } else { colors::MUTED };
        if chosen {
            let (left_tip, right_tip) = (right - width - 22, right + 2);
            shapes.push(Shape::Triangle(
                Triangle::new(
                    Point::new(left_tip, middle),
                    Point::new(left_tip + 7, middle - 6),
                    Point::new(left_tip + 7, middle + 6),
                ),
                colors::MUTED,
            ));
            shapes.push(Shape::Triangle(
                Triangle::new(
                    Point::new(right_tip + 7, middle),
                    Point::new(right_tip, middle - 6),
                    Point::new(right_tip, middle + 6),
                ),
                colors::MUTED,
            ));
        }
        let x = right - width - if chosen { 6 } else { 0 };
        shapes.push(Shape::Text(Point::new(x, baseline), value, Scale::Normal, color));
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
