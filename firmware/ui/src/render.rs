//! The screen: a top bar over one calculation, the mode menu or a menu list (SETUP, OPTN), in the
//! desktop's dark palette with its purple accents (design/handheld-menus.md §7.6). A [`Frame`] is the screen
//! laid out once; the [`Renderer`] draws it band by band into a small buffer and sends only the
//! bands that changed, so no framebuffer is needed. A wallpaper, if one is set, lies under it
//! all, and the tiles and menu lines let it shine through.

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::geometry::AngleUnit;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{
    Arc, Circle, Polyline, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, RoundedRectangle, StrokeAlignment,
    Triangle,
};

use crate::app::{App, Mode, Screen, MENU_COLUMNS, TILES_PER_PAGE};
use crate::backend::Storage;
use crate::menu::{self, Edit, Kind, Menu, ROWS};
use crate::settings::{Angle, WallpaperOn};
use crate::wallpaper::{blend, Pixels};
use crate::text;
use crate::font::Scale;
use crate::layout::{self, RowLayout};
use crate::touch::{self, Button, Calibrating, Target};

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
            Screen::Calc => calculation(&mut shapes, app, glass),
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

/// The top bar: the mode (or the menu's page) on the left, followed by a spinner while busy; on the right SHIFT and ALPHA while
/// they are on, the digits, exact or decimal, the angle unit, the WLAN and Bluetooth icons when
/// on, the exam mode and the battery.
fn bar<B>(shapes: &mut Vec<Shape>, app: &App<B>) {
    shapes.push(Shape::Fill(full_width(0, BAR as u32), colors::CYAN_DIM));
    let baseline = BAR - 7;
    let title = match &app.screen {
        Screen::Menu(menu) => menu.title().get(app.settings.language),
        Screen::Calibrate(..) => text::CALIBRATE.get(app.settings.language),
        _ => &app.mode.name(app.settings.language),
    };
    let mut left = MARGIN;
    if goes_back(&app.screen) {
        // ‹ before the title: a tap on the left of the bar goes back.
        let (x, middle) = (MARGIN + 1, BAR / 2);
        let points = vec![Point::new(x + 6, middle - 6), Point::new(x, middle), Point::new(x + 6, middle + 6)];
        shapes.push(Shape::Path(points, colors::TEXT, 2));
        left += 16;
    }
    shapes.push(Shape::Text(Point::new(left, baseline), title.to_string(), Scale::Small, colors::TEXT));
    if app.busy {
        // Eight dots on a ring; the bright one moves on a step each tick, with a trail behind it.
        let center = Point::new(left + Scale::Small.width(title) + 18, BAR / 2);
        for dot in 0..8u8 {
            let angle = f32::from(dot) * core::f32::consts::FRAC_PI_4;
            let at = center + Point::new((7.0 * angle.cos()).round() as i32, (7.0 * angle.sin()).round() as i32);
            let color = match (app.spin + 8 - dot) % 8 {
                0 => colors::TEXT,
                1 => colors::MUTED,
                _ => colors::FAINT,
            };
            shapes.push(Shape::Circle(Circle::with_center(at, 4), color));
        }
    }

    // From the right edge inwards.
    let mut right = WIDTH as i32 - MARGIN;
    right -= battery(shapes, app.battery, app.charging, Point::new(right, BAR / 2)) + 12;
    let settings = &app.settings;
    let lang = settings.language;
    // The exam mode as a tag in the accent, the radios as icons; the angle unit, the digits and
    // the result form as text.
    if settings.exam {
        right -= tag(shapes, right, text::EXAM.get(lang), colors::AMBER) + 8;
    }
    if settings.wlan {
        right -= wlan(shapes, right) + 10;
    }
    if settings.bluetooth {
        right -= bluetooth(shapes, right) + 10;
    }
    let angle = match settings.angle {
        Angle::Degrees => "D",
        Angle::Radians => "R",
    };
    let form = if settings.decimal { text::DECIMAL_SHORT } else { text::EXACT_SHORT };
    let digits = format!("Sig {}", settings.digits);
    for text in [angle, form.get(lang)].map(str::to_string).into_iter().chain([digits]) {
        let width = Scale::Small.width(&text);
        shapes.push(Shape::Text(Point::new(right - width, baseline), text, Scale::Small, colors::TEXT));
        right -= width + 12;
    }
    for (on, label, color) in [(app.alpha, "ALPHA", colors::ALPHA), (app.shift, "SHIFT", colors::SHIFT)] {
        if on {
            right -= tag(shapes, right, label, color) + 8;
        }
    }
}

/// A label on a coloured tag in the top bar, ending at `right`; returns its width.
fn tag(shapes: &mut Vec<Shape>, right: i32, label: &str, color: Rgb565) -> i32 {
    let width = Scale::Small.width(label);
    let tag = rounded(Point::new(right - width - 8, 4), Size::new(width as u32 + 8, BAR as u32 - 8), 3);
    shapes.push(Shape::Round(tag, color, None));
    let at = Point::new(right - width - 4, BAR - 7);
    shapes.push(Shape::Text(at, label.to_string(), Scale::Small, colors::BACKGROUND));
    width + 8
}

/// The WLAN icon, three arcs over a dot, ending at `right`; returns its width.
fn wlan(shapes: &mut Vec<Shape>, right: i32) -> i32 {
    let width = 18;
    let base = Point::new(right - width / 2, BAR - 6);
    for diameter in [9, 17, 25] {
        let arc = Arc::with_center(base, diameter, (-135.0).deg(), 90.0.deg());
        shapes.push(Shape::Arc(arc, colors::TEXT, 2));
    }
    shapes.push(Shape::Circle(Circle::with_center(base, 4), colors::TEXT));
    width
}

/// The Bluetooth rune ending at `right`, centred in the bar; returns its width.
fn bluetooth(shapes: &mut Vec<Shape>, right: i32) -> i32 {
    let (width, top, bottom) = (9, 6, BAR - 6);
    let (x, quarter) = (right - width / 2 - 1, (bottom - top) / 4);
    let points = [
        (x - 4, top + quarter),
        (x + 4, bottom - quarter),
        (x, bottom),
        (x, top),
        (x + 4, top + quarter),
        (x - 4, bottom - quarter),
    ];
    shapes.push(Shape::Path(points.iter().map(|&(x, y)| Point::new(x, y)).collect(), colors::TEXT, 2));
    width
}

/// A battery icon ending at `right`, centred on `center.y`, filled by the charge, in green with a
/// bolt inside while charging; returns its width.
fn battery(shapes: &mut Vec<Shape>, charge: Option<u8>, charging: bool, right_center: Point) -> i32 {
    let (width, height) = (26, 14);
    let top_left = Point::new(right_center.x - width, right_center.y - height / 2);
    let nub = Rectangle::new(Point::new(right_center.x - 2, right_center.y - 3), Size::new(2, 6));
    shapes.push(Shape::Fill(nub, colors::TEXT));
    let body = Rectangle::new(top_left, Size::new(width as u32 - 2, height as u32));
    shapes.push(Shape::Fill(body, colors::TEXT));
    shapes.push(Shape::Fill(body.offset(-1), colors::BACKGROUND));
    if let Some(charge) = charge {
        let inner = body.offset(-2);
        let filled = inner.size.width * u32::from(charge.min(100)) / 100;
        let color = match (charging, charge < 15) {
            (true, _) => colors::ALPHA,
            (false, true) => colors::ERROR,
            (false, false) => colors::TEXT,
        };
        shapes.push(Shape::Fill(Rectangle::new(inner.top_left, Size::new(filled, inner.size.height)), color));
    }
    if charging {
        // The bolt in the middle, as pixels: per row, where it starts and how wide it is. A dark rim
        // around it lets it show on the fill and on the empty part alike.
        const BOLT: [(i32, u32); 8] = [(3, 2), (2, 2), (1, 2), (0, 6), (3, 2), (2, 2), (1, 2), (1, 1)];
        let origin = Point::new(right_center.x - width / 2 - 4, right_center.y - 4);
        let rim = [(-1, 0), (1, 0), (0, -1), (0, 1)];
        for ((dx, dy), color) in rim.iter().map(|&o| (o, colors::BACKGROUND)).chain([((0, 0), colors::TEXT)]) {
            for (row, &(start, len)) in BOLT.iter().enumerate() {
                let at = origin + Point::new(start + dx, row as i32 + dy);
                shapes.push(Shape::Fill(Rectangle::new(at, Size::new(len, 1)), color));
            }
        }
    }
    width
}

/// One calculation, as on a Casio: the input from the top left, its result at the bottom right,
/// over the test buttons.
fn calculation<B>(shapes: &mut Vec<Shape>, app: &App<B>, glass: bool) {
    let width = WIDTH as i32;

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

    for (index, (button_, label)) in CALC_BUTTONS.iter().enumerate() {
        let lit = app.pressed() == Some(Target::Button(*button_));
        button(shapes, calc_button(index), label, lit, glass);
    }
    let bottom = calc_button(0).top_left.y - 8;
    if let Some(entry) = app.shown_entry() {
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
        let baseline = bottom - 2;
        let tip = Point::new(MARGIN + 5, baseline - 10);
        let arrow = Triangle::new(tip, Point::new(MARGIN, baseline), Point::new(MARGIN + 10, baseline));
        shapes.push(Shape::Triangle(arrow, colors::FAINT));
        let count = earlier.to_string();
        shapes.push(Shape::Text(Point::new(MARGIN + 15, baseline), count, Scale::Small, colors::FAINT));
    }
}

/// The mode menu: pages of tiles, numbered over all pages. While a finger drags them, the page
/// moves with it and its neighbour comes in.
fn modes<B>(shapes: &mut Vec<Shape>, app: &App<B>, selected: usize, glass: bool) {
    let lang = app.settings.language;
    let page = selected / TILES_PER_PAGE;
    let dx = app.drag().map_or(0, |drag| drag.x);
    let pages = if dx == 0 { page..=page } else { page.saturating_sub(1)..=(page + 1).min(PAGES - 1) };
    for shown in pages {
        let shift = Point::new((shown as i32 - page as i32) * WIDTH as i32 + dx, 0);
        tiles(shapes, shown, selected, app.pressed(), shift, lang, glass);
    }

    if PAGES > 1 {
        for index in 0..PAGES {
            let color = if index == page { colors::TEXT } else { colors::FAINT };
            let dot = dot_rect(index);
            shapes.push(Shape::Circle(Circle::new(dot.top_left, dot.size.width), color));
        }
    }
}

/// The tiles of mode page `page`, moved by `shift`.
fn tiles(
    shapes: &mut Vec<Shape>,
    page: usize,
    selected: usize,
    pressed: Option<Target>,
    shift: Point,
    lang: text::Lang,
    glass: bool,
) {
    let first = page * TILES_PER_PAGE;
    let modes = &Mode::ALL[first..(first + TILES_PER_PAGE).min(Mode::ALL.len())];
    let small = Scale::Small.metrics();
    for (offset, mode) in modes.iter().enumerate() {
        let index = first + offset;
        let Rectangle { top_left: at, size } = tile_rect(offset).translate(shift);
        let (tile_width, tile_height) = (size.width as i32, size.height as i32);
        let chosen = index == selected;
        let (tile, lit) = colors::TILES[index % colors::TILES.len()];
        let fill = if chosen || pressed == Some(Target::Tile(index)) { lit } else { tile };
        shapes.push(panel(rounded(at, size, 6), fill, chosen.then_some(colors::TEXT), glass));
        let number = (index + 1).to_string();
        shapes.push(Shape::Text(at + Point::new(12, 10 + small.ascent), number, Scale::Small, colors::TEXT));
        // The name at the bottom, a word per line when it is too wide, and small when a word is.
        let room = tile_width - 18;
        let name = mode.name(lang);
        let lines: Vec<String> = if Scale::Normal.width(&name) <= room {
            vec![name]
        } else {
            // At the spaces, and at the soft hyphens with a hyphen.
            let hyphenated = mode.text().get(lang).replace('\u{ad}', "-\u{ad}");
            hyphenated.split([' ', '\u{ad}']).map(str::to_string).collect()
        };
        let fits = lines.iter().all(|line| Scale::Normal.width(line) <= room);
        let scale = if fits { Scale::Normal } else { Scale::Small };
        let metrics = scale.metrics();
        let line_height = metrics.ascent + metrics.descent - 2;
        let mut baseline = at.y + tile_height - 12 - metrics.descent;
        for line in lines.iter().rev() {
            shapes.push(Shape::Text(Point::new(at.x + 12, baseline), line.to_string(), scale, colors::TEXT));
            baseline -= line_height;
        }
    }
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

/// A line of a menu list.
struct Line {
    label: String,
    value: Option<String>,
    /// What stands at the right end.
    mark: Mark,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mark {
    None,
    /// Opens a further page.
    Arrow,
    /// The value set now, in a list of choices.
    Dot,
}

const LINE: i32 = 50;
const LINE_GAP: i32 = 6;

/// SETUP or OPTN: the page on the screen, or the submenu open over it.
fn menu_screen<B>(shapes: &mut Vec<Shape>, app: &App<B>, menu: &Menu, glass: bool) {
    let lang = app.settings.language;
    let list = List {
        scroll: app.drag().map_or(0, |drag| drag.y),
        pressed: match app.pressed() {
            Some(Target::Line(offset)) => Some(offset),
            _ => None,
        },
        glass,
    };
    match &menu.edit {
        None => {
            let level = menu.level();
            let lines = level.page.entries.iter().map(|entry| Line {
                label: entry.label.get(lang).to_string(),
                value: match entry.kind {
                    Kind::Wallpapers => Some(app.wallpaper_name.clone().unwrap_or_else(|| text::NONE.get(lang).to_string())),
                    _ => menu::value(entry, &app.settings, lang),
                },
                mark: if matches!(entry.kind, Kind::Page(_)) { Mark::Arrow } else { Mark::None },
            });
            lines_page(shapes, lines.collect(), level.selected, &list);
        }
        Some(Edit::Choice { entry, selected }) => {
            let Kind::Choice(setting, opts) = &entry.kind else { return };
            let value = app.settings.get(*setting);
            let lines = opts.iter().map(|opt| Line {
                label: opt.text.get(lang).to_string(),
                value: None,
                mark: if opt.value == value { Mark::Dot } else { Mark::None },
            });
            lines_page(shapes, lines.collect(), *selected, &list);
        }
        Some(Edit::Confirm { .. }) => {
            let yes = Line { label: text::YES.get(lang).to_string(), value: None, mark: Mark::None };
            lines_page(shapes, vec![yes], 0, &list);
        }
        Some(Edit::Number { setting, min, max, typed, error, .. }) => {
            let top = BAR + 30;
            let height = 60;
            let size = Size::new(WIDTH - 2 * MARGIN as u32, height as u32);
            shapes.push(panel(rounded(Point::new(MARGIN, top), size, 6), colors::SURFACE, Some(colors::CYAN), glass));
            let metrics = Scale::Normal.metrics();
            let baseline = top + (height + metrics.digit) / 2;
            let x = MARGIN + 16;
            if typed.is_empty() {
                // The value set now, until a digit is typed.
                let now = app.settings.get(*setting).to_string();
                shapes.push(Shape::Text(Point::new(x + 6, baseline), now, Scale::Normal, colors::FAINT));
            } else {
                shapes.push(Shape::Text(Point::new(x, baseline), typed.clone(), Scale::Normal, colors::TEXT));
            }
            if app.caret_on {
                let caret_x = if typed.is_empty() { x } else { x + Scale::Normal.width(typed) + 2 };
                let caret = Rectangle::new(Point::new(caret_x, baseline - metrics.ascent + 4), Size::new(2, metrics.ascent as u32));
                shapes.push(Shape::Fill(caret, colors::CYAN));
            }
            let hint = format!("{}: {min} {} {max}", text::RANGE.get(lang), text::TO.get(lang));
            let color = if *error { colors::ERROR } else { colors::MUTED };
            let at = Point::new(MARGIN + 4, top + height + 28);
            shapes.push(Shape::Text(at, hint, Scale::Small, color));
            for (index, (target, label)) in NUMBER_BUTTONS.iter().enumerate() {
                button(shapes, number_button(index), label, app.pressed() == Some(*target), glass);
            }
        }
        Some(Edit::Pictures { names, selected, .. }) => {
            let set = app.wallpaper_name.as_ref().and_then(|name| names.iter().position(|n| n == name)).map_or(0, |i| i + 1);
            let lines = [text::NONE.get(lang).to_string()].into_iter().chain(names.iter().cloned()).enumerate();
            let lines = lines.map(|(index, label)| Line {
                label,
                value: None,
                mark: if index == set { Mark::Dot } else { Mark::None },
            });
            let x = WIDTH as i32 - MARGIN - THUMBNAIL.width as i32;
            lines_in(shapes, lines.collect(), *selected, &list, list_edge(menu));
            // The picture under the cursor, or why there is none.
            let at = Point::new(x, BAR + (HEIGHT as i32 - BAR - THUMBNAIL.height as i32) / 2);
            let frame = Rectangle::new(at - Point::new(2, 2), THUMBNAIL + Size::new(4, 4));
            shapes.push(Shape::Round(RoundedRectangle::with_equal_corners(frame, Size::new(4, 4)), colors::HAIRLINE, None));
            match app.wallpaper() {
                Some(wallpaper) => shapes.push(Shape::Thumbnail(at, wallpaper.full.clone())),
                None => {
                    shapes.push(Shape::Fill(Rectangle::new(at, THUMBNAIL), colors::BACKGROUND));
                    let message = if *selected == 0 { text::NONE } else { text::BAD_PICTURE }.get(lang);
                    let message = fit(message, THUMBNAIL.width as i32 - 16, Scale::Small);
                    let middle = at + Point::new((THUMBNAIL.width as i32 - Scale::Small.width(&message)) / 2, THUMBNAIL.height as i32 / 2);
                    shapes.push(Shape::Text(middle, message, Scale::Small, colors::MUTED));
                }
            }
        }
        Some(Edit::Storage { storage: Some(storage), .. }) => storage_screen(shapes, *storage, lang, glass),
        Some(Edit::Storage { storage: None, .. }) => centred(shapes, text::NO_CARD.get(lang)),
        Some(Edit::Message { message, .. }) => centred(shapes, message.get(lang)),
    }
}

/// A muted line in the middle of the screen under the top bar.
fn centred(shapes: &mut Vec<Shape>, text: &str) {
    let x = (WIDTH as i32 - Scale::Normal.width(text)) / 2;
    let y = BAR + (HEIGHT as i32 - BAR) / 2;
    shapes.push(Shape::Text(Point::new(x.max(MARGIN), y), text.to_string(), Scale::Normal, colors::MUTED));
}

/// SETUP → Gerät → Speicherplatz: the card's name and how full it is in percent, a bar of the
/// space used, and the space used, free and in all.
fn storage_screen(shapes: &mut Vec<Shape>, storage: Storage, lang: text::Lang, glass: bool) {
    let top = BAR + 14;
    let size = Size::new(WIDTH - 2 * MARGIN as u32, HEIGHT - top as u32 - 14);
    shapes.push(panel(rounded(Point::new(MARGIN, top), size, 8), colors::SURFACE, Some(colors::HAIRLINE), glass));
    let (left, right) = (MARGIN + 20, WIDTH as i32 - MARGIN - 20);
    let right_aligned = |text: &str, scale: Scale| right - scale.width(text);

    let heading = top + 38;
    shapes.push(Shape::Text(Point::new(left, heading), text::SD_CARD.get(lang).to_string(), Scale::Normal, colors::TEXT));
    // A thousandth, so a nearly full card does not show 100 %.
    let permille = match storage.total {
        0 => 0,
        total => (u128::from(storage.used()) * 1000 / u128::from(total)) as u32,
    };
    let percent = format!("{} %", permille / 10);
    let color = if permille >= 900 { colors::ERROR } else { colors::RESULT };
    shapes.push(Shape::Text(Point::new(right_aligned(&percent, Scale::Normal), heading), percent, Scale::Normal, color));

    let (bar_top, bar_height, width) = (heading + 18, 22, right - left);
    shapes.push(Shape::Round(rounded(Point::new(left, bar_top), Size::new(width as u32, bar_height), 6), colors::BACKGROUND, Some(colors::HAIRLINE)));
    // At least a sliver while anything is used, inside the outline.
    let filled = ((width - 4) as u32 * permille / 1000).max(u32::from(storage.used() > 0) * 6);
    if filled > 0 {
        let fill = if permille >= 900 { colors::ERROR } else { colors::CYAN };
        shapes.push(Shape::Round(rounded(Point::new(left + 2, bar_top + 2), Size::new(filled, bar_height - 4), 4), fill, None));
    }

    let rows = [(text::USED, storage.used()), (text::FREE, storage.free), (text::TOTAL, storage.total)];
    for (index, (label, value)) in rows.into_iter().enumerate() {
        let baseline = bar_top + bar_height as i32 + 44 + index as i32 * 40;
        shapes.push(Shape::Text(Point::new(left, baseline), label.get(lang).to_string(), Scale::Small, colors::MUTED));
        let value = crate::backend::bytes(value, lang);
        shapes.push(Shape::Text(Point::new(right_aligned(&value, Scale::Normal), baseline), value, Scale::Normal, colors::TEXT));
    }
}

/// How a list is shown: moved by a dragging finger, the line under it lit.
struct List {
    scroll: i32,
    /// The line the finger is on, counted on the page.
    pressed: Option<usize>,
    glass: bool,
}

/// A list of lines, [`menu::ROWS`] to a page, numbered on each page, with a scroll bar when
/// there are more.
fn lines_page(shapes: &mut Vec<Shape>, lines: Vec<Line>, selected: usize, list: &List) {
    lines_in(shapes, lines, selected, list, WIDTH as i32 - MARGIN);
}

/// [`lines_page`] with the lines ending at `edge`; a label too long for them is cut short. The
/// lines lie one under the other, the page of `selected` at the top, so a drag shows the pages
/// before and after it moving in.
fn lines_in(shapes: &mut Vec<Shape>, lines: Vec<Line>, selected: usize, list: &List, edge: i32) {
    let first = selected / ROWS * ROWS;
    let normal = Scale::Normal.metrics();
    let small = Scale::Small.metrics();
    let scrolls = lines.len() > ROWS;
    let right = edge - if scrolls { 8 } else { 0 };
    for (index, line) in lines.iter().enumerate() {
        let place = index as i32 - first as i32;
        let area = line_rect(0, right).translate(Point::new(0, place * (LINE + LINE_GAP) + list.scroll));
        let y = area.top_left.y;
        if y + LINE <= BAR || y >= HEIGHT as i32 {
            continue;
        }
        let chosen = index == selected || (list.scroll == 0 && list.pressed.is_some_and(|offset| first + offset == index));
        let fill = if chosen { colors::CYAN_DIM } else { colors::SURFACE };
        shapes.push(panel(RoundedRectangle::with_equal_corners(area, Size::new(6, 6)), fill, None, list.glass));
        let baseline = y + (LINE + normal.digit) / 2;
        let number = (index % ROWS + 1).to_string();
        shapes.push(Shape::Text(Point::new(MARGIN + 12, baseline), number, Scale::Small, colors::MUTED));
        let label = fit(&line.label, right - MARGIN - 34 - 14, Scale::Normal);
        shapes.push(Shape::Text(Point::new(MARGIN + 34, baseline), label, Scale::Normal, colors::TEXT));

        let middle = y + LINE / 2;
        let mut end = right - 14;
        match line.mark {
            Mark::Arrow => {
                let arrow = Triangle::new(
                    Point::new(end, middle),
                    Point::new(end - 7, middle - 6),
                    Point::new(end - 7, middle + 6),
                );
                shapes.push(Shape::Triangle(arrow, colors::MUTED));
                end -= 16;
            }
            Mark::Dot => {
                let dot = 10;
                shapes.push(Shape::Circle(Circle::new(Point::new(end - dot, middle - dot / 2), dot as u32), colors::CYAN));
                end -= dot + 8;
            }
            Mark::None => {}
        }
        if let Some(value) = &line.value {
            let width = Scale::Small.width(value);
            let color = if chosen { colors::TEXT } else { colors::MUTED };
            let at = Point::new(end - width, y + (LINE + small.digit) / 2);
            shapes.push(Shape::Text(at, value.clone(), Scale::Small, color));
        }
    }
    if scrolls {
        let top = BAR + 8;
        let height = ROWS as i32 * (LINE + LINE_GAP) - LINE_GAP;
        let x = edge - 3;
        shapes.push(Shape::Fill(Rectangle::new(Point::new(x, top), Size::new(3, height as u32)), colors::HAIRLINE));
        let pages = lines.len().div_ceil(ROWS) as i32;
        let thumb = height / pages;
        let page = (first / ROWS) as i32;
        let at = Point::new(x, top + page * thumb);
        shapes.push(Shape::Fill(Rectangle::new(at, Size::new(3, thumb as u32)), colors::CYAN));
    }
}

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
        Some(Edit::Number { .. } | Edit::Message { .. } | Edit::Storage { .. }) => None,
    }
}

/// The test buttons of the calculation screen, until the keypad exists.
pub const CALC_BUTTONS: [(Button, &str); 3] = [(Button::Menu, "MENU"), (Button::Setup, "SETUP"), (Button::Optn, "OPTN")];

/// The buttons under a number being entered: it is stepped, there is no keyboard on the screen.
pub const NUMBER_BUTTONS: [(Target, &str); 3] = [(Target::Step(-1), "−"), (Target::Step(1), "+"), (Target::Ok, "OK")];

/// The calculation screen's button `index`, in a row along the bottom.
pub fn calc_button(index: usize) -> Rectangle {
    let height = 32;
    button_rect(HEIGHT as i32 - MARGIN - height, height as u32, CALC_BUTTONS.len(), index)
}

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

/// A button with its label in the middle, lit while a finger is on it.
fn button(shapes: &mut Vec<Shape>, area: Rectangle, label: &str, lit: bool, glass: bool) {
    let round = RoundedRectangle::with_equal_corners(area, Size::new(6, 6));
    let fill = if lit { colors::CYAN_DIM } else { colors::SURFACE };
    shapes.push(panel(round, fill, Some(colors::HAIRLINE), glass));
    let digit = Scale::Small.metrics().digit;
    let x = (area.size.width as i32 - Scale::Small.width(label)) / 2;
    let at = area.top_left + Point::new(x, (area.size.height as i32 + digit) / 2);
    shapes.push(Shape::Text(at, label.to_string(), Scale::Small, colors::TEXT));
}

/// What a tap at `point` hits on the screen as `app` shows it.
pub fn target<B>(app: &App<B>, point: Point) -> Option<Target> {
    if goes_back(&app.screen) && back_area().contains(point) {
        return Some(Target::Back);
    }
    match &app.screen {
        Screen::Calibrate(..) => None,
        Screen::Calc => {
            let index = (0..CALC_BUTTONS.len()).find(|&index| calc_button(index).contains(point))?;
            Some(Target::Button(CALC_BUTTONS[index].0))
        }
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

/// The touch calibration: a cross at the target in turn, filled while it is pressed, and what
/// to do in the middle of the screen.
fn calibration(shapes: &mut Vec<Shape>, state: &Calibrating, lang: text::Lang) {
    let step = state.readings.len().min(touch::TARGETS.len() - 1);
    let at = touch::TARGETS[step];
    let pressed = state.sum.2 > 0;
    for (from, to) in [(Point::new(-16, 0), Point::new(16, 0)), (Point::new(0, -16), Point::new(0, 16))] {
        shapes.push(Shape::Path(vec![at + from, at + to], colors::TEXT, 2));
    }
    let ring = if pressed { colors::CYAN } else { colors::AMBER };
    shapes.push(Shape::Circle(Circle::with_center(at, 10), ring));
    let centred = |shapes: &mut Vec<Shape>, y: i32, text: &str, scale: Scale, color: Rgb565| {
        let x = (WIDTH as i32 - scale.width(text)) / 2;
        shapes.push(Shape::Text(Point::new(x.max(MARGIN), y), text.to_string(), scale, color));
    };
    centred(shapes, 115, text::CALIBRATE_HINT.get(lang), Scale::Normal, colors::TEXT);
    let count = format!("{} / {}", step + 1, touch::TARGETS.len());
    centred(shapes, 220, &count, Scale::Small, colors::MUTED);
    if state.retry {
        centred(shapes, 245, text::CALIBRATE_RETRY.get(lang), Scale::Small, colors::ERROR);
    }
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
