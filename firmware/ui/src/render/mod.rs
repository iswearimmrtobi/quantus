//! The screen: a top bar over one calculation, the mode menu or a menu list (SETUP, OPTN), in the
//! desktop's dark palette with its purple accents (design/handheld-menus.md §7.6). A [`Frame`] is the screen
//! laid out once; the [`Renderer`] draws it band by band into a small buffer and sends only the
//! bands that changed, so no framebuffer is needed. A wallpaper, if one is set, lies under it
//! all, and the tiles and menu lines let it shine through.

mod bar;
mod renderer;
mod screens;

pub use renderer::Renderer;

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{
    Arc, Circle, Polyline, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, RoundedRectangle, StrokeAlignment,
    Triangle,
};

use crate::app::{App, Mode, Screen, MENU_COLUMNS, TILES_PER_PAGE};
use crate::font::Scale;
use crate::layout::{self, RowLayout};
use crate::menu::{Edit, Kind, Menu, ROWS};
use crate::settings::WallpaperOn;
use crate::touch::Target;
use crate::wallpaper::Pixels;
use bar::bar;
use screens::{calculation, calibration, menu_screen, modes};

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
    /// A result, in a soft purple.
    pub const RESULT: Rgb565 = rgb(0xbf, 0xa0, 0xf2);
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

    /// A muted colour per mode tile, in the order of `Mode::ALL`: (the tile, the chosen tile).
    pub const TILES: [(Rgb565, Rgb565); 12] = [
        (rgb(0x4a, 0x3d, 0x6b), rgb(0x63, 0x52, 0x8e)), // purple
        (rgb(0x34, 0x44, 0x6b), rgb(0x46, 0x59, 0x8c)), // blue
        (rgb(0x2f, 0x55, 0x60), rgb(0x3e, 0x6f, 0x7d)), // teal
        (rgb(0x35, 0x57, 0x4a), rgb(0x46, 0x72, 0x62)), // green
        (rgb(0x4d, 0x55, 0x36), rgb(0x66, 0x6f, 0x47)), // olive
        (rgb(0x62, 0x51, 0x3a), rgb(0x80, 0x6a, 0x4c)), // amber
        (rgb(0x64, 0x3f, 0x35), rgb(0x83, 0x53, 0x46)), // rust
        (rgb(0x5e, 0x36, 0x40), rgb(0x7b, 0x47, 0x54)), // red
        (rgb(0x5c, 0x37, 0x56), rgb(0x78, 0x48, 0x71)), // plum
        (rgb(0x3a, 0x3a, 0x6e), rgb(0x4d, 0x4d, 0x90)), // indigo
        (rgb(0x2f, 0x4f, 0x6a), rgb(0x3e, 0x67, 0x8b)), // steel
        (rgb(0x45, 0x4a, 0x5a), rgb(0x5c, 0x62, 0x76)), // slate
    ];

}

/// Something to draw, at absolute screen coordinates. Its `Debug` text says all that it draws:
/// the renderer compares it to skip bands that stay the same.
#[derive(Debug)]
pub enum Shape {
    Fill(Rectangle, Rgb565),
    /// A rounded box, filled and with an optional outline drawn inside it.
    Round(RoundedRectangle, Rgb565, Option<Rgb565>),
    /// A rounded box laid over what is under it, `alpha` of 256 of its colour showing; drawn
    /// as a [`Shape::Round`] on targets that cannot be read back.
    Glass(RoundedRectangle, Rgb565, u16, Option<Rgb565>),
    Triangle(Triangle, Rgb565),
    Circle(Circle, Rgb565),
    /// An arc, stroked with the width.
    Arc(Arc, Rgb565, u32),
    /// Lines through the points, stroked with the width.
    Path(Vec<Point>, Rgb565, u32),
    /// Text with its baseline at the point.
    Text(Point, String, Scale, Rgb565),
    /// A laid out row with its origin at the point.
    Math(Point, RowLayout, Rgb565),
    /// A screen-sized picture at half its size, its top left at the point.
    Thumbnail(Point, Pixels),
}

/// The size of a [`Shape::Thumbnail`].
const THUMBNAIL: Size = Size::new(WIDTH / 2, HEIGHT / 2);

impl Shape {
    fn bounds(&self) -> Rectangle {
        match self {
            Shape::Fill(area, _) => *area,
            Shape::Round(round, _, _) | Shape::Glass(round, ..) => round.rectangle,
            Shape::Triangle(triangle, _) => triangle.bounding_box(),
            Shape::Circle(circle, _) => circle.bounding_box(),
            Shape::Arc(arc, color, width) => arc.into_styled(PrimitiveStyle::with_stroke(*color, *width)).bounding_box(),
            Shape::Path(points, color, width) => {
                Polyline::new(points).into_styled(PrimitiveStyle::with_stroke(*color, *width)).bounding_box()
            }
            Shape::Text(origin, text, scale, _) => {
                let metrics = scale.metrics();
                Rectangle::new(
                    *origin - Point::new(0, metrics.ascent),
                    Size::new(scale.width(text) as u32, (metrics.ascent + metrics.descent) as u32),
                )
            }
            Shape::Math(origin, row, _) => row.bounds(*origin),
            Shape::Thumbnail(at, _) => Rectangle::new(*at, THUMBNAIL),
        }
    }

    fn draw<D: DrawTarget<Color = Rgb565>>(&self, target: &mut D) {
        match self {
            Shape::Fill(area, color) => {
                let _ = area.into_styled(PrimitiveStyle::with_fill(*color)).draw(target);
            }
            Shape::Round(round, fill, outline) | Shape::Glass(round, fill, _, outline) => {
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
            Shape::Arc(arc, color, width) => {
                let _ = arc.into_styled(PrimitiveStyle::with_stroke(*color, *width)).draw(target);
            }
            Shape::Path(points, color, width) => {
                let _ = Polyline::new(points).into_styled(PrimitiveStyle::with_stroke(*color, *width)).draw(target);
            }
            Shape::Text(origin, text, scale, color) => scale.draw(text, *origin, *color, target),
            Shape::Math(origin, row, color) => {
                layout::draw(row, *origin, *color, colors::PLACEHOLDER, target);
            }
            Shape::Thumbnail(at, pixels) => {
                let width = THUMBNAIL.width;
                let colors = (0..THUMBNAIL.height)
                    .flat_map(move |y| (0..width).map(move |x| pixels.get((2 * y * WIDTH + 2 * x) as usize)));
                let _ = target.fill_contiguous(&Rectangle::new(*at, THUMBNAIL), colors);
            }
        }
    }
}

/// The screen laid out, back to front, over the backdrop: the wallpaper's pixels if it shows,
/// else the plain background.
pub struct Frame(pub Vec<Shape>, pub Option<Pixels>);

impl Frame {
    pub fn new<B>(app: &App<B>) -> Frame {
        let mut shapes = Vec::new();
        let backdrop = backdrop(app);
        let glass = backdrop.is_some();
        match &app.screen {
            Screen::Calc => calculation(&mut shapes, app),
            Screen::Modes { selected } => modes(&mut shapes, app, *selected, glass),
            Screen::Menu(menu) => menu_screen(&mut shapes, app, menu, glass),
            Screen::Calibrate(state, _) => calibration(&mut shapes, state, app.settings.language),
        }
        bar(&mut shapes, app);
        Frame(shapes, backdrop)
    }
}

/// The wallpaper under the screen, as SETUP → Gerät → Hintergrundbild → Anzeigen says: at full
/// strength behind the mode tiles, dimmed where text stands on it.
fn backdrop<B>(app: &App<B>) -> Option<Pixels> {
    let wallpaper = app.wallpaper()?;
    let on = app.settings.wallpaper_on;
    match app.screen {
        Screen::Modes { .. } => Some(wallpaper.full.clone()),
        Screen::Menu(_) if on != WallpaperOn::Modes => Some(wallpaper.dim.clone()),
        Screen::Calc if on == WallpaperOn::Everywhere => Some(wallpaper.dim.clone()),
        _ => None,
    }
}

/// How much of a tile or menu line covers the wallpaper, out of 256.
const GLASS: u16 = 200;

/// A box, of glass over a wallpaper.
fn panel(shape: RoundedRectangle, fill: Rgb565, outline: Option<Rgb565>, glass: bool) -> Shape {
    if glass {
        Shape::Glass(shape, fill, GLASS, outline)
    } else {
        Shape::Round(shape, fill, outline)
    }
}

fn full_width(top: i32, height: u32) -> Rectangle {
    Rectangle::new(Point::new(0, top), Size::new(WIDTH, height))
}

fn rounded(top_left: Point, size: Size, radius: u32) -> RoundedRectangle {
    RoundedRectangle::with_equal_corners(Rectangle::new(top_left, size), Size::new(radius, radius))
}

const TILE_GAP: i32 = 8;
const TILE_HEIGHT: i32 = 116;
/// The pages of the mode menu.
const PAGES: usize = Mode::ALL.len().div_ceil(TILES_PER_PAGE);

/// The tile `offset` of a page of the mode menu.
pub fn tile_rect(offset: usize) -> Rectangle {
    let columns = MENU_COLUMNS as i32;
    let width = (WIDTH as i32 - 2 * MARGIN - (columns - 1) * TILE_GAP) / columns;
    let column = (offset % MENU_COLUMNS) as i32;
    let row = (offset / MENU_COLUMNS) as i32;
    let at = Point::new(MARGIN + column * (width + TILE_GAP), BAR + 10 + row * (TILE_HEIGHT + TILE_GAP));
    Rectangle::new(at, Size::new(width as u32, TILE_HEIGHT as u32))
}

/// The dot of page `index` under the tiles.
pub fn dot_rect(index: usize) -> Rectangle {
    let dot = 8;
    let rows = (TILES_PER_PAGE / MENU_COLUMNS) as i32;
    let y = BAR + 10 + rows * (TILE_HEIGHT + TILE_GAP) + 10;
    let start = (WIDTH as i32 - (PAGES as i32 * (dot + 8) - 8)) / 2;
    Rectangle::new(Point::new(start + index as i32 * (dot + 8), y), Size::new(dot as u32, dot as u32))
}

const LINE: i32 = 50;
const LINE_GAP: i32 = 6;

/// The line `offset` of a page of a list, ending at `right`.
pub fn line_rect(offset: usize, right: i32) -> Rectangle {
    let y = BAR + 8 + offset as i32 * (LINE + LINE_GAP);
    Rectangle::new(Point::new(MARGIN, y), Size::new((right - MARGIN) as u32, LINE as u32))
}

/// Where the lists of `menu` may reach: the edge, or the wallpaper's preview.
fn list_edge(menu: &Menu) -> i32 {
    match menu.edit {
        Some(Edit::Pictures { .. }) => WIDTH as i32 - MARGIN - THUMBNAIL.width as i32 - 10,
        _ => WIDTH as i32 - MARGIN,
    }
}

/// How many lines the list on the screen has, if a list is.
fn list_len(menu: &Menu) -> Option<usize> {
    match &menu.edit {
        None => Some(menu.level().page.entries.len()),
        Some(Edit::Choice { entry, .. }) => match &entry.kind {
            Kind::Choice(_, opts) => Some(opts.len()),
            _ => None,
        },
        Some(Edit::Confirm { .. }) => Some(1),
        Some(Edit::Pictures { names, .. }) => Some(names.len() + 1),
        Some(Edit::Sessions { sessions, .. }) => Some(sessions.len()),
        Some(Edit::Number { .. } | Edit::Message { .. } | Edit::Storage { .. } | Edit::Name { .. } | Edit::Time { .. }) => {
            None
        }
    }
}

/// The buttons under a number being entered: it is stepped, there is no keyboard on the screen.
pub const NUMBER_BUTTONS: [(Target, &str); 3] = [(Target::Step(-1), "−"), (Target::Step(1), "+"), (Target::Ok, "OK")];

/// The number entry's button `index`, under the range.
pub fn number_button(index: usize) -> Rectangle {
    button_rect(BAR + 140, 48, NUMBER_BUTTONS.len(), index)
}

/// The left of the top bar, where a tap goes back, reaching a little below it for the finger.
pub fn back_area() -> Rectangle {
    Rectangle::new(Point::zero(), Size::new(WIDTH / 2, BAR as u32 + 4))
}

/// Button `index` of `count` side by side between the margins.
fn button_rect(top: i32, height: u32, count: usize, index: usize) -> Rectangle {
    let (gap, count) = (8, count as i32);
    let width = (WIDTH as i32 - 2 * MARGIN - (count - 1) * gap) / count;
    Rectangle::new(Point::new(MARGIN + index as i32 * (width + gap), top), Size::new(width as u32, height))
}

/// What a tap at `point` hits on the screen as `app` shows it.
pub fn target<B>(app: &App<B>, point: Point) -> Option<Target> {
    if goes_back(&app.screen) && back_area().contains(point) {
        return Some(Target::Back);
    }
    match &app.screen {
        Screen::Calibrate(..) | Screen::Calc => None,
        Screen::Modes { selected } => {
            let first = selected / TILES_PER_PAGE * TILES_PER_PAGE;
            let on_page = (Mode::ALL.len() - first).min(TILES_PER_PAGE);
            if let Some(offset) = (0..on_page).find(|&offset| tile_rect(offset).contains(point)) {
                return Some(Target::Tile(first + offset));
            }
            // Half the gap to the next dot on each side, more above and below for the finger.
            let around = |dot: Rectangle| Rectangle::new(dot.top_left - Point::new(4, 12), dot.size + Size::new(8, 24));
            (0..PAGES).find(|&index| around(dot_rect(index)).contains(point)).map(Target::Page)
        }
        Screen::Menu(menu) => {
            if let Some(Edit::Message { .. } | Edit::Storage { .. }) = menu.edit {
                return Some(Target::Anywhere);
            }
            if let Some(Edit::Number { .. }) = menu.edit {
                let index = (0..NUMBER_BUTTONS.len()).find(|&index| number_button(index).contains(point))?;
                return Some(NUMBER_BUTTONS[index].0);
            }
            list_len(menu)?;
            // The whole width, and half the gap above and below: a finger is not precise.
            let around = |line: Rectangle| {
                Rectangle::new(line.top_left - Point::new(MARGIN, LINE_GAP / 2), line.size + Size::new(2 * MARGIN as u32, LINE_GAP as u32))
            };
            let edge = list_edge(menu);
            (0..ROWS).find(|&offset| around(line_rect(offset, edge)).contains(point)).map(Target::Line)
        }
    }
}

/// Whether the screen has ‹ in the top bar.
fn goes_back(screen: &Screen) -> bool {
    matches!(screen, Screen::Modes { .. } | Screen::Menu(_))
}

/// `text`, cut short with three dots to fit `room` (the fonts have no ellipsis).
fn fit(text: &str, room: i32, scale: Scale) -> String {
    if scale.width(text) <= room {
        return text.to_string();
    }
    let mut cut: String = text.to_string();
    while !cut.is_empty() && scale.width(&format!("{cut}...")) > room {
        cut.pop();
    }
    format!("{cut}...")
}
