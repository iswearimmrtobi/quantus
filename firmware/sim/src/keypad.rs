//! The handheld's keypad, drawn beside the screen and clickable (a press types the key).
//!
//! The layout is that of `design/keypad-sizes.html` (`DEVICES`, all sizes in millimetres) and of
//! firmware/docs/hardware.md §4: the soft keys, the control row, the function keys around the
//! arrows and the number block. A key whose label has no [`Key`] yet (UNIT, SYMB, the soft keys,
//! ON, x⁻¹, ...) is drawn dimmed and does nothing.

use std::time::{Duration, Instant};

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, RoundedRectangle, Triangle};
use u8g2_fonts::types::{FontColor, HorizontalAlignment, VerticalPosition};
use u8g2_fonts::{fonts, FontRenderer};
use quantus_ui::key::Key;

/// Pixels per millimetre. The screen is 480 px on 73.6 mm of the big unit (6.5 px/mm), so the keys
/// are drawn smaller than life to fit a monitor.
const PX_PER_MM: f32 = 5.0;

/// How long a key typed on the PC keyboard stays lit.
const FLASH: Duration = Duration::from_millis(150);

/// The keys that repeat while held, as on the PC keyboard: after the delay, at the rate.
pub const REPEAT_DELAY: Duration = Duration::from_millis(400);
pub const REPEAT_RATE: Duration = Duration::from_millis(80);

/// Which of the two units to draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    Small,
    Big,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Style {
    Plain,
    Digit,
    Op,
    Shift,
    Alpha,
    Unit,
    Exe,
    Del,
    On,
    Soft,
}

struct KeyDef {
    label: &'static str,
    rect: Rectangle,
    round: bool,
    style: Style,
    key: Option<Key>,
}

/// A block of keys on a grid; `.` in a row is an empty cell, `!` marks a key of the big unit only.
struct Block {
    rows: &'static [&'static str],
    kw: f32,
    kh: f32,
    px: f32,
    py: f32,
    round: bool,
    /// The columns' centres and the block's width, if the grid is not even.
    centres: Option<(&'static [f32], f32)>,
    /// Labels drawn with another size.
    other: Option<(&'static str, f32, f32)>,
    /// Space after the block, if not the unit's gap.
    gap_after: Option<f32>,
}

struct Device {
    width: f32,
    top: f32,
    bottom: f32,
    gap: f32,
    /// Columns, rows, key size and pitch of the soft keys.
    soft: (usize, usize, f32, f32, f32, f32),
    blocks: &'static [Block],
}

const SMALL: Device = Device {
    width: 77.0,
    top: 7.0,
    bottom: 7.0,
    gap: 3.0,
    soft: (5, 2, 10.3, 5.0, 11.5, 7.0),
    blocks: &[
        Block {
            rows: &["SHIFT ALPHA UNIT OPTN MODE ON"],
            kw: 6.0,
            kh: 6.0,
            px: 0.0,
            py: 8.0,
            round: true,
            centres: Some((&[4.65, 15.95, 27.25, 38.55, 53.8, 62.8], 65.8)),
            other: None,
            gap_after: Some(0.0),
        },
        Block {
            rows: &[
                "SYMB a/b √ . ▲ .",
                "VARS x² xʸ ◄ OK ►",
                "CALC log ln . ▼ .",
                "(−) x⁻¹ sin cos tan π",
                "( ) = ; CONST ENG",
                "x y e i CONV S⇔D",
            ],
            kw: 9.3,
            kh: 5.0,
            px: 11.3,
            py: 7.0,
            round: false,
            centres: None,
            other: Some(("▲ ▼ ◄ ► OK", 7.0, 5.5)),
            gap_after: Some(1.0),
        },
        NUMBERS_SMALL,
    ],
};

const BIG: Device = Device {
    width: 97.0,
    top: 8.0,
    bottom: 8.0,
    gap: 3.0,
    soft: (6, 2, 10.4, 5.5, 12.3, 7.5),
    blocks: &[
        Block {
            rows: &["SHIFT ALPHA UNIT OPTN . MODE ON"],
            kw: 6.0,
            kh: 6.0,
            px: 0.0,
            py: 8.0,
            round: true,
            centres: Some((&[5.3, 17.9, 30.5, 43.1, 55.7, 68.3, 80.9], 86.2)),
            other: None,
            gap_after: Some(0.0),
        },
        Block {
            rows: &[
                "SYMB a/b √ x² . ▲ .",
                "VARS xʸ log ln ◄ OK ►",
                "CALC !UNDO !▶unit !x³ . ▼ .",
                "(−) x⁻¹ sin cos tan π !10ˣ",
                "( ) = ; CONST ENG !Σ",
                "x y e i CONV S⇔D !∫",
            ],
            kw: 10.6,
            kh: 5.0,
            px: 12.6,
            py: 7.0,
            round: false,
            centres: None,
            other: Some(("▲ ▼ ◄ ► OK", 7.5, 5.5)),
            gap_after: Some(1.0),
        },
        NUMBERS_BIG,
    ],
};

const NUMBER_ROWS: &[&str] = &["7 8 9 DEL AC", "4 5 6 × ÷", "1 2 3 + −", "0 , ×10ˣ Ans EXE"];

const NUMBERS_SMALL: Block = Block {
    rows: NUMBER_ROWS,
    kw: 11.4,
    kh: 8.5,
    px: 13.6,
    py: 10.5,
    round: false,
    centres: None,
    other: None,
    gap_after: None,
};

const NUMBERS_BIG: Block = Block { kw: 15.6, kh: 9.0, px: 17.6, py: 11.0, ..NUMBERS_SMALL };

/// The key a label stands for, if the UI knows it.
fn key_of(label: &str) -> Option<Key> {
    Some(match label {
        "SHIFT" => Key::Shift,
        "ALPHA" => Key::Alpha,
        "OPTN" => Key::Optn,
        "MODE" => Key::Mode,
        "OK" => Key::Ok,
        "▲" => Key::Up,
        "▼" => Key::Down,
        "◄" => Key::Left,
        "►" => Key::Right,
        "DEL" => Key::Del,
        "AC" => Key::Ac,
        "EXE" => Key::Exe,
        "CONST" => Key::Const,
        "CONV" => Key::Conv,
        "CALC" => Key::Calc,
        "VARS" => Key::Vars,
        "S⇔D" => Key::SwapForm,
        "a/b" => Key::Fraction,
        "√" => Key::Root,
        "x²" => Key::Square,
        "xʸ" => Key::Power,
        "log" => Key::Log,
        "ln" => Key::Ln,
        "sin" => Key::Sin,
        "cos" => Key::Cos,
        "tan" => Key::Tan,
        "π" => Key::Pi,
        "e" => Key::E,
        "Ans" => Key::Ans,
        "(" => Key::OpenParen,
        ")" => Key::CloseParen,
        "=" => Key::Equals,
        ";" => Key::Semicolon,
        "," => Key::Comma,
        "+" => Key::Plus,
        "−" => Key::Minus,
        "×" => Key::Times,
        "÷" => Key::Divide,
        "x" => Key::Letter('x'),
        "y" => Key::Letter('y'),
        digit if digit.len() == 1 && digit.as_bytes()[0].is_ascii_digit() => Key::Digit(digit.as_bytes()[0] - b'0'),
        _ => return None,
    })
}

/// What a key does after SHIFT, where the UI knows an action (the label drawn in SHIFT's colour).
fn shift_legend(label: &str) -> Option<&'static str> {
    Some(match label {
        "MODE" => "SETUP",
        "CALC" => "SOLVE",
        "VARS" => "STO",
        "√" => "n√",
        _ => return None,
    })
}

/// The variable a key types after ALPHA (hardware.md: A–F, M, x, y).
fn alpha_legend(label: &str) -> Option<&'static str> {
    Some(match label {
        "(−)" => "A",
        "x⁻¹" => "B",
        "sin" => "C",
        "cos" => "D",
        "tan" => "E",
        "π" => "F",
        "Ans" => "M",
        "(" => "x",
        ")" => "y",
        _ => return None,
    })
}

/// Which modifier is waiting for a key.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Layer {
    Base,
    Shift,
    Alpha,
}

const SHIFT_COLOUR: Rgb565 = Rgb565::new(0xe8 >> 3, 0xc5 >> 2, 0x3a >> 3);
const ALPHA_COLOUR: Rgb565 = Rgb565::new(0xd9 >> 3, 0x4a >> 2, 0x4a >> 3);

fn style_of(label: &str) -> Style {
    match label {
        "SHIFT" => Style::Shift,
        "ALPHA" => Style::Alpha,
        "UNIT" => Style::Unit,
        "EXE" => Style::Exe,
        "ON" => Style::On,
        "DEL" | "AC" => Style::Del,
        "×" | "÷" | "+" | "−" => Style::Op,
        l if l.len() == 1 && l.as_bytes()[0].is_ascii_digit() || l == "," => Style::Digit,
        _ => Style::Plain,
    }
}

/// What is drawn on a key, where the font lacks the label's characters.
fn shown(label: &'static str) -> &'static str {
    match label {
        "xʸ" => "x^y",
        "x⁻¹" => "x^-1",
        "−" => "-",
        "n√" => "n-sqrt",
        "√" => "sqrt",
        "∫" => "int",
        "(−)" => "(-)",
        "×10ˣ" => "×10^x",
        "10ˣ" => "10^x",
        "S⇔D" => "S<>D",
        "▶unit" => ">unit",
        label => label,
    }
}

static LABEL: FontRenderer = FontRenderer::new::<fonts::u8g2_font_6x10_tf>();
static ROUND: FontRenderer = FontRenderer::new::<fonts::u8g2_font_5x8_tf>();
static SYMBOLS: FontRenderer = FontRenderer::new::<fonts::u8g2_font_6x12_t_symbols>();
static SYMBOLS_BIG: FontRenderer = FontRenderer::new::<fonts::u8g2_font_7x13_t_symbols>();
static GREEK: FontRenderer = FontRenderer::new::<fonts::u8g2_font_cu12_t_greek>();

fn rgb(r: u8, g: u8, b: u8) -> Rgb565 {
    Rgb565::new(r >> 3, g >> 2, b >> 3)
}

const BODY: Rgb565 = Rgb565::new(3, 6, 4);

pub struct Keypad {
    keys: Vec<KeyDef>,
    /// Where the panel starts in the window, and its size.
    panel: Rectangle,
    /// The key under the mouse button.
    pressed: Option<usize>,
    /// A key typed on the PC keyboard, lit for a moment.
    flash: Option<(usize, Instant)>,
    /// The key being held and when it types again.
    held: Option<(Key, Instant)>,
}

impl Keypad {
    /// The keypad of `unit` with its top left corner at `origin`.
    pub fn new(unit: Unit, origin: Point) -> Keypad {
        let device = match unit {
            Unit::Small => &SMALL,
            Unit::Big => &BIG,
        };
        let px = |mm: f32| (mm * PX_PER_MM).round() as i32;
        let mut keys = Vec::new();
        let mut y = device.top;

        let (cols, rows, kw, kh, pitch_x, pitch_y) = device.soft;
        let x0 = (device.width - ((cols - 1) as f32 * pitch_x + kw)) / 2.0;
        for row in 0..rows {
            for col in 0..cols {
                let label: &'static str = Box::leak(format!("F{}", row * cols + col + 1).into_boxed_str());
                let rect = Rectangle::new(
                    Point::new(px(x0 + col as f32 * pitch_x), px(y + row as f32 * pitch_y)),
                    Size::new(px(kw) as u32, px(kh) as u32),
                );
                keys.push(KeyDef { label, rect, round: false, style: Style::Soft, key: None });
            }
        }
        y += rows as f32 * pitch_y + device.gap + 1.0;

        for (index, block) in device.blocks.iter().enumerate() {
            let grid: Vec<Vec<&'static str>> = block.rows.iter().map(|row| row.split_whitespace().collect()).collect();
            let columns = grid[0].len();
            let width = block.centres.map_or((columns - 1) as f32 * block.px + block.kw, |(_, width)| width);
            let left = (device.width - width) / 2.0;
            for (r, row) in grid.iter().enumerate() {
                for (c, cell) in row.iter().enumerate() {
                    if *cell == "." {
                        continue;
                    }
                    let label = cell.strip_prefix('!').unwrap_or(cell);
                    let (kw, kh) = match block.other {
                        Some((labels, w, h)) if labels.split(' ').any(|l| l == label) => (w, h),
                        _ => (block.kw, block.kh),
                    };
                    let centre = block.centres.map_or(c as f32 * block.px + block.kw / 2.0, |(centres, _)| centres[c]);
                    let rect = Rectangle::new(
                        Point::new(px(left + centre - kw / 2.0), px(y + r as f32 * block.py + (block.kh - kh) / 2.0)),
                        Size::new(px(kw) as u32, px(kh) as u32),
                    );
                    keys.push(KeyDef { label, rect, round: block.round, style: style_of(label), key: key_of(label) });
                }
            }
            let last = index == device.blocks.len() - 1;
            y += rows_len(block) + if last { 0.0 } else { block.gap_after.unwrap_or(device.gap) };
        }
        let panel = Rectangle::new(origin, Size::new(px(device.width) as u32, px(y + device.bottom) as u32));
        for key in &mut keys {
            key.rect.top_left += origin;
        }
        Keypad { keys, panel, pressed: None, flash: None, held: None }
    }

    /// The size the window needs for a screen of `screen` at its top left corner.
    pub fn window(&self, screen: Size) -> Size {
        Size::new(self.panel.top_left.x as u32 + self.panel.size.width, screen.height.max(self.panel.size.height))
    }

    fn hit(&self, point: Point) -> Option<usize> {
        self.keys.iter().position(|key| key.rect.contains(point))
    }

    /// Whether the point is on the keypad panel.
    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.panel.top_left.x
    }

    /// The mouse button goes down at `point`: the key under it, to be typed now.
    pub fn press(&mut self, point: Point, now: Instant, alpha: bool) -> Option<Key> {
        self.pressed = self.hit(point).filter(|&i| self.keys[i].key.is_some());
        let key = self.pressed.and_then(|i| {
            let def = &self.keys[i];
            // After ALPHA a key with a variable types it.
            match alpha_legend(def.label) {
                Some(letter) if alpha => letter.chars().next().map(|c| Key::Letter(c.to_ascii_lowercase())),
                _ => def.key,
            }
        });
        self.held = key.filter(|key| matches!(key, Key::Left | Key::Right | Key::Up | Key::Down | Key::Del)).map(|key| (key, now + REPEAT_DELAY));
        key
    }

    /// The button goes up. Whether the picture changed.
    pub fn release(&mut self) -> bool {
        self.held = None;
        self.pressed.take().is_some()
    }

    /// The mouse moves with the button down: leaving the key lets go of it. Whether the picture
    /// changed.
    pub fn drag(&mut self, point: Point) -> bool {
        match self.pressed {
            Some(i) if !self.keys[i].rect.contains(point) => {
                self.pressed = None;
                self.held = None;
                true
            }
            _ => false,
        }
    }

    /// A key of a held arrow or DEL, once the delay has passed; and whether the lit key went out.
    pub fn tick(&mut self, now: Instant) -> (Option<Key>, bool) {
        let mut dim = false;
        if self.flash.is_some_and(|(_, since)| now.duration_since(since) >= FLASH) {
            self.flash = None;
            dim = true;
        }
        let repeat = match &mut self.held {
            Some((key, next)) if now >= *next => {
                *next = now + REPEAT_RATE;
                Some(*key)
            }
            _ => None,
        };
        (repeat, dim)
    }

    /// A key typed on the PC keyboard lights its key on the pad for a moment. Whether it did.
    pub fn flash(&mut self, key: Key, now: Instant) -> bool {
        match self.keys.iter().position(|def| def.key == Some(key)) {
            Some(i) => {
                self.flash = Some((i, now));
                true
            }
            None => false,
        }
    }

    /// Draws the whole panel; SHIFT and ALPHA are outlined while they are waiting for a key.
    pub fn draw<D: DrawTarget<Color = Rgb565>>(&self, target: &mut D, shift: bool, alpha: bool) {
        // SHIFT wins if both wait; the app uses both flags on the next key.
        let layer = if shift { Layer::Shift } else if alpha { Layer::Alpha } else { Layer::Base };
        let _ = self.panel.into_styled(PrimitiveStyle::with_fill(BODY)).draw(target);
        for (i, key) in self.keys.iter().enumerate() {
            let lit = self.pressed == Some(i) || self.flash.is_some_and(|(flash, _)| flash == i);
            let latched = (key.style == Style::Shift && shift) || (key.style == Style::Alpha && alpha);
            draw_key(target, key, lit, latched, layer);
        }
    }
}

/// The height of a block's rows, in millimetres.
fn rows_len(block: &Block) -> f32 {
    block.rows.len() as f32 * block.py
}

fn draw_key<D: DrawTarget<Color = Rgb565>>(target: &mut D, key: &KeyDef, lit: bool, latched: bool, layer: Layer) {
    let (fill, text) = match key.style {
        Style::Plain => (rgb(0x44, 0x49, 0x55), rgb(0xee, 0xee, 0xee)),
        Style::Digit => (rgb(0x62, 0x67, 0x74), rgb(0xff, 0xff, 0xff)),
        Style::Op => (rgb(0x50, 0x56, 0x68), rgb(0xff, 0xff, 0xff)),
        Style::Shift => (rgb(0xe8, 0xc5, 0x3a), rgb(0x20, 0x20, 0x20)),
        Style::Alpha => (rgb(0xd9, 0x4a, 0x4a), rgb(0xff, 0xff, 0xff)),
        Style::Unit => (rgb(0x91, 0x86, 0xe8), rgb(0x20, 0x20, 0x20)),
        Style::Exe => (rgb(0x3a, 0x8f, 0x5a), rgb(0xff, 0xff, 0xff)),
        Style::Del => (rgb(0x9a, 0x4a, 0x4a), rgb(0xff, 0xff, 0xff)),
        Style::On => (rgb(0x20, 0x20, 0x24), rgb(0xff, 0xff, 0xff)),
        Style::Soft => (rgb(0x2c, 0x30, 0x38), rgb(0x90, 0x94, 0x9e)),
    };
    // A key that does nothing yet is dimmed.
    let dim = key.key.is_none() && key.style != Style::Soft && key.style != Style::Unit && key.style != Style::On;
    let (fill, text) = if dim { (rgb(0x32, 0x36, 0x40), rgb(0x80, 0x84, 0x8e)) } else { (fill, text) };
    let fill = if lit { rgb(fill.r() << 3 | 0x40, fill.g() << 2 | 0x40, fill.b() << 3 | 0x40) } else { fill };
    let (stroke, width) = if latched { (rgb(0xff, 0xff, 0xff), 2) } else { (rgb(0x10, 0x11, 0x14), 1) };
    let style = PrimitiveStyleBuilder::new().fill_color(fill).stroke_color(stroke).stroke_width(width).build();
    let rect = key.rect;
    if key.round {
        let diameter = rect.size.width.min(rect.size.height);
        let _ = Circle::with_center(rect.center(), diameter).into_styled(style).draw(target);
    } else {
        let _ = RoundedRectangle::with_equal_corners(rect, Size::new(4, 4)).into_styled(style).draw(target);
    }
    let centre = rect.center();
    let mut arrow = |points: [Point; 3]| {
        let _ = Triangle::new(points[0], points[1], points[2]).into_styled(PrimitiveStyle::with_fill(text)).draw(target);
    };
    match key.label {
        "▲" => arrow([centre + Point::new(0, -5), centre + Point::new(-6, 4), centre + Point::new(6, 4)]),
        "▼" => arrow([centre + Point::new(0, 5), centre + Point::new(-6, -4), centre + Point::new(6, -4)]),
        "◄" => arrow([centre + Point::new(-6, 0), centre + Point::new(4, -6), centre + Point::new(4, 6)]),
        "►" => arrow([centre + Point::new(6, 0), centre + Point::new(-4, -6), centre + Point::new(-4, 6)]),
        label => {
            let (label, text) = match layer {
                Layer::Shift if shift_legend(label).is_some() => (shift_legend(label).unwrap_or(label), SHIFT_COLOUR),
                Layer::Alpha if alpha_legend(label).is_some() => (alpha_legend(label).unwrap_or(label), ALPHA_COLOUR),
                _ => (label, text),
            };
            let label = shown(label);
            let fonts: [&FontRenderer; 4] = if key.round { [&ROUND, &SYMBOLS, &SYMBOLS_BIG, &GREEK] } else { [&LABEL, &SYMBOLS, &SYMBOLS_BIG, &GREEK] };
            // Characters are taken from the first font that has them, the label as a whole from
            // the first font that has all.
            let font = fonts.into_iter().find(|font| label.chars().all(|c| font.get_rendered_dimensions(c, Point::zero(), VerticalPosition::Baseline).is_ok()));
            let font = font.unwrap_or(fonts[0]);
            let _ = font.render_aligned(label, centre, VerticalPosition::Center, HorizontalAlignment::Center, FontColor::Transparent(text), target);
        }
    }
}
