//! Apps (firmware/docs/apps.md): a program that is not part of the hui takes the screen and the
//! keys for a while. The hui knows it as an [`Applet`]: it hands it the keys an app gets
//! ([`AppKey`]), lets it draw into its area ([`Canvas`]) and answers what it may ask of the
//! device ([`Host`]). The top bar, the soft-key strips, AC, MODE and OPTN stay the hui's.

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, Rectangle, RoundedRectangle};

use crate::font::Scale;
use crate::key::Key;
use crate::layout;
use crate::render::{colors, Shape};
use crate::text::Lang;

/// A program that runs inside the hui's frame.
pub trait Applet {
    /// The app opens: the place to set it up with what the host can tell.
    fn start(&mut self, _host: &mut dyn Host) {}

    /// A key pressed, of those the input template does not take.
    fn key(&mut self, key: AppKey, host: &mut dyn Host);

    /// Time passes; whether the app wants to be drawn again.
    fn tick(&mut self, _now_ms: u64, _host: &mut dyn Host) -> bool {
        false
    }

    /// The app's area, described in full.
    fn draw(&self, canvas: &mut Canvas);

    /// A tap inside the area, in its coordinates.
    fn tap(&mut self, _point: Point, _host: &mut dyn Host) {}

    /// The labels of the soft keys, from the top left along the rows; none hides the strips.
    fn soft_keys(&self) -> Vec<String> {
        Vec::new()
    }

    /// How the app is typed into.
    fn input(&self) -> Input {
        Input::Keys
    }

    /// What the input template gives (apps.md §4.4): a cell picked, a line or a list entry
    /// confirmed.
    fn event(&mut self, _event: Event, _host: &mut dyn Host) {}

    /// The answer to a question the app asked through the host ([`Host::confirm`]).
    fn answer(&mut self, _response: Response, _host: &mut dyn Host) {}

    /// Starts again from the beginning, as when it opened (*Neu starten* in the app's menu,
    /// after [`Applet::leave`]; after *Daten löschen*, without it).
    fn restart(&mut self, _host: &mut dyn Host) {}

    /// The app is left: a value for the calculation's cursor, if it hands one back.
    fn leave(&mut self, _host: &mut dyn Host) -> Option<String> {
        None
    }
}

/// What an app may ask of the device. The hui answers over its backend and its own state.
pub trait Host {
    /// A random number.
    fn random(&mut self) -> u32;

    /// The hui's language.
    fn lang(&self) -> Lang;

    /// The cursor of the grid template: its column and row.
    fn cursor(&self) -> (usize, usize) {
        (0, 0)
    }

    /// Moves the grid's cursor.
    fn set_cursor(&mut self, _col: usize, _row: usize) {}

    /// Asks `question` with *Ja* and *Nein* over the app; the answer comes to
    /// [`Applet::answer`], and until then the app gets nothing.
    fn confirm(&mut self, _question: &str) {}

    /// The app's saved data, as JSON.
    fn load(&mut self) -> Option<String> {
        None
    }

    /// Keeps the app's data, as JSON.
    fn save(&mut self, _json: &str) {}

    /// The size of the app's area, with the soft-key strips shown or not.
    fn area(&self, soft_keys: bool) -> Size {
        crate::render::app_area_with(soft_keys).size
    }

    /// Leaves the app once the call that asks for it is done.
    fn exit(&mut self) {}

    /// The time as Unix milliseconds, if the clock is set.
    fn now(&self) -> Option<i64> {
        crate::clock::now()
    }
}

/// How an app is typed into (apps.md §4.4). The hui handles the template's keys and draws its
/// part; the keys it does not use go to [`Applet::key`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Input {
    /// Every key goes to the app.
    #[default]
    Keys,
    /// A cursor over the cells of [`Grid`]: the arrows move it (round the edges with `wrap`),
    /// OK or EXE pick; `digits` lets 1–9 pick a cell of a 3 × 3 grid as the number block lies.
    Grid { cols: usize, rows: usize, digits: bool, wrap: bool },
    /// A list of the hui's in the area: ↑/↓, a digit, OK.
    List { items: Vec<String> },
    /// The calculator's input line at the bottom of the area; `calc` works it out first.
    Line { label: String, calc: bool },
    /// A line of plain text at the bottom of the area, at most `max` characters, `value` in it
    /// at first.
    Text { label: String, max: usize, value: String },
    /// The grid with a value typed into the cell under the cursor; `digits_only` takes one
    /// digit and moves on.
    Cells { cols: usize, rows: usize, digits_only: bool },
}

/// A question of the hui's over an app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Question {
    /// *Ja* or *Nein*; `yes` is where the cursor is.
    Confirm { text: String, yes: bool },
}

/// The answer to a [`Question`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Response {
    /// *Ja* (true) or *Nein*; AC is *Nein*.
    Confirmed(bool),
}

/// What an input template gives the app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// OK or EXE (or a tap) on a cell of the grid.
    Pick(usize, usize),
    /// The entry of the list at this place, chosen.
    Chosen(usize),
    /// A line confirmed: plain text, or what the input line holds as LaTeX.
    Entered(String),
    /// A cell's value confirmed: its column, its row, what was typed.
    Cell(usize, usize, String),
}

/// A board of `cols` × `rows` square cells laid into an app's area as large as fits, and the
/// part beside it for the score or whose turn it is (apps.md §4.5): where the area is wide
/// enough the side panel is to the right of the board, else below it, and the board centred.
/// The grid templates and the scripts' `grid` both use it, so they agree on the cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Grid {
    pub cols: usize,
    pub rows: usize,
    /// The board's top left, in the area's coordinates.
    pub origin: Point,
    /// The side of a cell.
    pub cell: i32,
    side: Rectangle,
}

/// The space kept around the board and between it and the side panel.
const GRID_MARGIN: i32 = 8;
/// The narrowest side panel beside the board; narrower, it goes below.
const SIDE_WIDTH: i32 = 140;
/// The height of a side panel below the board.
const SIDE_HEIGHT: i32 = 56;

impl Grid {
    /// The board for an area of `width` × `height`.
    pub fn new(width: i32, height: i32, cols: usize, rows: usize) -> Grid {
        let (cols, rows) = (cols.clamp(1, 64), rows.clamp(1, 64));
        let (c, r) = (cols as i32, rows as i32);
        let m = GRID_MARGIN;
        // Beside: the board as high as the area allows.
        let cell = ((height - 2 * m) / r).min((width - 2 * m) / c).max(1);
        let board = (cell * c, cell * r);
        let room = width - board.0 - 3 * m;
        if room >= SIDE_WIDTH {
            let origin = Point::new(m, (height - board.1) / 2);
            let side = Rectangle::new(Point::new(2 * m + board.0, m), Size::new(room.max(0) as u32, (height - 2 * m).max(0) as u32));
            return Grid { cols, rows, origin, cell, side };
        }
        // Below: the board centred over the panel.
        let cell = ((height - 3 * m - SIDE_HEIGHT) / r).min((width - 2 * m) / c).max(1);
        let board = (cell * c, cell * r);
        let origin = Point::new((width - board.0) / 2, m);
        let top = origin.y + board.1 + m;
        let side = Rectangle::new(Point::new(m, top), Size::new((width - 2 * m).max(0) as u32, (height - top - m).max(0) as u32));
        Grid { cols, rows, origin, cell, side }
    }

    /// The cell at `col`, `row`, in the area's coordinates.
    pub fn cell(&self, col: usize, row: usize) -> Rectangle {
        let at = self.origin + Point::new(col as i32 * self.cell, row as i32 * self.cell);
        Rectangle::new(at, Size::new(self.cell as u32, self.cell as u32))
    }

    /// The whole board.
    pub fn board(&self) -> Rectangle {
        Rectangle::new(self.origin, Size::new((self.cell * self.cols as i32) as u32, (self.cell * self.rows as i32) as u32))
    }

    /// The cell under `point`, if it is on the board.
    pub fn at(&self, point: Point) -> Option<(usize, usize)> {
        if !self.board().contains(point) {
            return None;
        }
        let offset = point - self.origin;
        Some(((offset.x / self.cell) as usize, (offset.y / self.cell) as usize))
    }

    /// The part of the area beside or below the board.
    pub fn side(&self) -> Rectangle {
        self.side
    }
}

/// The keys an app gets: no modifiers, no AC, MODE or OPTN (apps.md §4.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppKey {
    Up,
    Down,
    Left,
    Right,
    Ok,
    Exe,
    Del,
    Digit(u8),
    Plus,
    Minus,
    Times,
    Divide,
    Equals,
    /// The decimal comma, or the point.
    Point,
    /// A soft key, counted from 0 over all pages.
    Soft(usize),
}

impl AppKey {
    /// The key an app gets for a key of the keypad, if it gets one.
    pub fn from_key(key: Key) -> Option<AppKey> {
        Some(match key {
            Key::Up => AppKey::Up,
            Key::Down => AppKey::Down,
            Key::Left => AppKey::Left,
            Key::Right => AppKey::Right,
            Key::Ok => AppKey::Ok,
            Key::Exe => AppKey::Exe,
            Key::Del => AppKey::Del,
            Key::Digit(digit) if digit <= 9 => AppKey::Digit(digit),
            Key::Plus => AppKey::Plus,
            Key::Minus => AppKey::Minus,
            Key::Times => AppKey::Times,
            Key::Divide | Key::Fraction => AppKey::Divide,
            Key::Equals => AppKey::Equals,
            Key::Comma | Key::Char('.') => AppKey::Point,
            Key::Soft(index) => AppKey::Soft(usize::from(index)),
            _ => return None,
        })
    }

    /// The key's name, as a script gets it.
    pub fn name(self) -> String {
        match self {
            AppKey::Up => "up".into(),
            AppKey::Down => "down".into(),
            AppKey::Left => "left".into(),
            AppKey::Right => "right".into(),
            AppKey::Ok => "ok".into(),
            AppKey::Exe => "exe".into(),
            AppKey::Del => "del".into(),
            AppKey::Digit(digit) => digit.to_string(),
            AppKey::Plus => "+".into(),
            AppKey::Minus => "-".into(),
            AppKey::Times => "*".into(),
            AppKey::Divide => "/".into(),
            AppKey::Equals => "=".into(),
            AppKey::Point => ".".into(),
            AppKey::Soft(index) => format!("soft{}", index + 1),
        }
    }
}

/// A game or a tool: the list groups by it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AppKind {
    Game,
    #[default]
    Tool,
}

/// An app as the list shows it, and its credits (apps.md §2).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AppInfo {
    /// The file's name without its ending.
    pub id: String,
    pub name: String,
    pub kind: AppKind,
    pub authors: Vec<String>,
    pub version: Option<String>,
    pub about: Option<String>,
    pub license: Option<String>,
    /// The version of the script API it was written for.
    pub api: u32,
    /// Compiled into the firmware, or from the card.
    pub built_in: bool,
    /// Whether this firmware can run it: its API is not newer.
    pub usable: bool,
}

/// The most shapes one frame of an app may have; more are dropped.
pub const MAX_SHAPES: usize = 4000;

/// The app's area, which it draws into with its own coordinates (from the area's top left);
/// each call adds a [`Shape`] of the hui's at screen coordinates.
pub struct Canvas {
    area: Rectangle,
    shapes: Vec<Shape>,
    dropped: bool,
}

impl Canvas {
    pub fn new(area: Rectangle) -> Self {
        Canvas { area, shapes: Vec::new(), dropped: false }
    }

    pub fn area(&self) -> Rectangle {
        self.area
    }

    pub fn width(&self) -> i32 {
        self.area.size.width as i32
    }

    pub fn height(&self) -> i32 {
        self.area.size.height as i32
    }

    /// The shapes drawn, at screen coordinates.
    pub fn shapes(&self) -> &[Shape] {
        &self.shapes
    }

    pub fn into_shapes(self) -> Vec<Shape> {
        self.shapes
    }

    /// Whether shapes were dropped over [`MAX_SHAPES`].
    pub fn dropped(&self) -> bool {
        self.dropped
    }

    fn at(&self, x: i32, y: i32) -> Point {
        self.area.top_left + Point::new(x, y)
    }

    fn push(&mut self, shape: Shape) {
        if self.shapes.len() >= MAX_SHAPES {
            self.dropped = true;
        } else {
            self.shapes.push(shape);
        }
    }

    fn rectangle(&self, x: i32, y: i32, w: i32, h: i32) -> Rectangle {
        Rectangle::new(self.at(x, y), Size::new(w.max(0) as u32, h.max(0) as u32))
    }

    /// Fills the whole area.
    pub fn clear(&mut self, color: Rgb565) {
        self.push(Shape::Fill(self.area, color));
    }

    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: Rgb565) {
        self.push(Shape::Fill(self.rectangle(x, y, w, h), color));
    }

    /// A rectangle with corners of radius `r`.
    pub fn round(&mut self, x: i32, y: i32, w: i32, h: i32, r: i32, color: Rgb565) {
        let r = r.max(0) as u32;
        let shape = RoundedRectangle::with_equal_corners(self.rectangle(x, y, w, h), Size::new(r, r));
        self.push(Shape::Round(shape, color, None));
    }

    /// A rectangle's outline, two pixels wide, inside it.
    pub fn frame(&mut self, x: i32, y: i32, w: i32, h: i32, r: i32, color: Rgb565) {
        let r = r.max(0) as u32;
        let shape = RoundedRectangle::with_equal_corners(self.rectangle(x, y, w, h), Size::new(r, r));
        self.push(Shape::Outline(shape, color));
    }

    /// A filled circle of diameter `d`, its bounding box's top left at (x, y).
    pub fn circle(&mut self, x: i32, y: i32, d: i32, color: Rgb565) {
        self.push(Shape::Circle(Circle::new(self.at(x, y), d.max(0) as u32), color));
    }

    pub fn line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, color: Rgb565, width: i32) {
        let points = vec![self.at(x1, y1), self.at(x2, y2)];
        self.push(Shape::Path(points, color, width.clamp(1, 32) as u32));
    }

    /// Text in the normal font, the top left of its line at (x, y).
    pub fn text(&mut self, x: i32, y: i32, text: &str, color: Rgb565) {
        self.text_in(Scale::Normal, x, y, text, color);
    }

    /// Text in the small font.
    pub fn text_small(&mut self, x: i32, y: i32, text: &str, color: Rgb565) {
        self.text_in(Scale::Small, x, y, text, color);
    }

    fn text_in(&mut self, scale: Scale, x: i32, y: i32, text: &str, color: Rgb565) {
        let baseline = self.at(x, y + scale.metrics().ascent);
        self.push(Shape::Text(baseline, text.to_string(), scale, color));
    }

    /// A formula, set as results are, the top left of its box at (x, y).
    pub fn math(&mut self, x: i32, y: i32, latex: &str, color: Rgb565) {
        let row = layout::layout(&crate::latex::parse(latex), Scale::Normal);
        let origin = self.at(x, y + row.ascent);
        self.push(Shape::Math(origin, row, color));
    }
}

/// How wide `text` is in the normal font.
pub fn text_width(text: &str) -> i32 {
    Scale::Normal.width(text)
}

pub fn text_small_width(text: &str) -> i32 {
    Scale::Small.width(text)
}

/// The height of a line in the normal font, for centring.
pub fn text_height() -> i32 {
    let metrics = Scale::Normal.metrics();
    metrics.ascent + metrics.descent
}

pub fn text_small_height() -> i32 {
    let metrics = Scale::Small.metrics();
    metrics.ascent + metrics.descent
}

/// The colours an app may name, so it follows the theme (apps.md §4.3).
pub struct Palette;

impl Palette {
    /// The colour of `name`: `"background"`, `"surface"`, `"text"`, `"muted"`, `"result"`,
    /// `"error"`, `"cyan"`, `"amber"`, `"tile1"` up to the number of tiles.
    pub fn get(name: &str) -> Option<Rgb565> {
        Some(match name {
            "background" => colors::background(),
            "surface" => colors::surface(),
            "hairline" => colors::hairline(),
            "text" => colors::text(),
            "muted" => colors::muted(),
            "faint" => colors::faint(),
            "result" => colors::result(),
            "error" => colors::error(),
            "cyan" => colors::primary(),
            "amber" => colors::accent(),
            _ => {
                let index: usize = name.strip_prefix("tile")?.parse().ok()?;
                colors::tiles().get(index.checked_sub(1)?)?.1
            }
        })
    }

    pub fn rgb(r: u8, g: u8, b: u8) -> Rgb565 {
        colors::rgb(r, g, b)
    }
}
