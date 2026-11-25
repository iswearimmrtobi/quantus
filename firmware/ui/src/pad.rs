//! The Duo's input screen (firmware/docs/duo.md §3): a 480×320 touch screen in place of the
//! keypad. It shows the mode's soft keys as buttons, a row of SHIFT, ALPHA, UNIT, OPTN and MODE,
//! the function keys in two pages and the number block. A tap sends the [`Key`] that a physical
//! key would, so the calculator works as on the handheld.

use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

use crate::app::App;
use crate::key::Key;
use crate::legend::key_of;
use crate::soft;
use crate::touch::{Calibration, Touch};

pub const WIDTH: u32 = 480;
pub const HEIGHT: u32 = 320;

/// The height of a soft key; they take two rows at the top.
const SOFT_ROW: i32 = 34;
/// The row of SHIFT, ALPHA, UNIT, OPTN, MODE and the page key.
const CONTROL_TOP: i32 = 2 * SOFT_ROW + 2;
const CONTROL_ROW: i32 = 34;
/// The function keys on the left and the number block on the right, four rows each.
const GRID_TOP: i32 = CONTROL_TOP + CONTROL_ROW + 2;
const GRID_ROWS: i32 = 4;
const GRID_SPLIT: i32 = 264;
/// The room between two keys.
const GAP: i32 = 4;
/// The room a label may take on its key.
pub const LABEL_ROOM: i32 = 3;

pub const CONTROL: [&str; 5] = ["SHIFT", "ALPHA", "UNIT", "OPTN", "MODE"];

/// The function keys of the big keypad (design/keypad-sizes.html), the ones used most on the
/// first page; four to a row.
const FUNCTIONS: [[&str; 16]; 2] = [
    ["a/b", "√", "x²", "xʸ", "sin", "cos", "tan", "log", "ln", "π", "(−)", "x⁻¹", "(", ")", "x", "e"],
    ["SYMB", "VARS", "CALC", "CONST", "UNDO", "x³", "10ˣ", "Σ", "=", ";", "ENG", "S⇔D", "y", "i", "CONV", "∫"],
];

const NUMBERS: [[&str; 5]; 4] =
    [["7", "8", "9", "DEL", "AC"], ["4", "5", "6", "×", "÷"], ["1", "2", "3", "+", "−"], ["0", ",", "×10ˣ", "Ans", "EXE"]];

/// What a key of the pad does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Key(Key),
    /// Soft key `n` of the mode's bar.
    Soft(u8),
    /// The other page of function keys.
    Page,
    /// Nothing yet: a key the UI does not carry out.
    None,
}

/// How a key looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Soft,
    Function,
    Digit,
    Operator,
    Shift,
    Alpha,
    Unit,
    Exe,
    Delete,
}

/// A key as the pad shows it now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PadKey {
    pub rect: Rectangle,
    /// The printed label ([`crate::legend`]), or a soft key's LaTeX.
    pub label: String,
    pub action: Action,
    pub style: Style,
}

/// The pad's own state: the page of function keys, the key under the finger, and its touch
/// calibration (the main screen keeps its own in the app).
#[derive(Clone, Debug, Default)]
pub struct Pad {
    pub page: usize,
    pub pressed: Option<usize>,
    pub calibration: Calibration,
}

impl Pad {
    pub fn new() -> Pad {
        Pad::default()
    }

    /// The keys as they are now: the soft keys follow the app's bar, the function keys the page.
    pub fn keys<B>(&self, app: &App<B>) -> Vec<PadKey> {
        let mut keys = Vec::new();
        if let Some(bar) = soft::bar(app) {
            let columns = bar.columns as i32;
            for index in 0..2 * bar.columns {
                let (row, column) = ((index / bar.columns) as i32, (index % bar.columns) as i32);
                let rect = cell(column, row * SOFT_ROW, WIDTH as i32 / columns, SOFT_ROW, columns, 0);
                let label = bar.label(index, app.settings.language);
                let action = if label.is_empty() { Action::None } else { Action::Soft(index as u8) };
                keys.push(PadKey { rect, label, action, style: Style::Soft });
            }
        }
        let width = WIDTH as i32 / (CONTROL.len() as i32 + 1);
        for (column, label) in CONTROL.iter().enumerate() {
            let rect = cell(column as i32, CONTROL_TOP, width, CONTROL_ROW, 0, 0);
            keys.push(printed(label, rect));
        }
        let rect = cell(CONTROL.len() as i32, CONTROL_TOP, width, CONTROL_ROW, 0, 0);
        let label = format!("{}/{}", self.page + 1, FUNCTIONS.len());
        keys.push(PadKey { rect, label, action: Action::Page, style: Style::Function });
        let row_height = (HEIGHT as i32 - GRID_TOP) / GRID_ROWS;
        for (index, label) in FUNCTIONS[self.page % FUNCTIONS.len()].iter().enumerate() {
            let (row, column) = (index as i32 / 4, index as i32 % 4);
            let rect = cell(column, GRID_TOP + row * row_height, GRID_SPLIT / 4, row_height, 0, 0);
            keys.push(printed(label, rect));
        }
        let width = (WIDTH as i32 - GRID_SPLIT) / 5;
        for (row, labels) in NUMBERS.iter().enumerate() {
            for (column, label) in labels.iter().enumerate() {
                let rect = cell(column as i32, GRID_TOP + row as i32 * row_height, width, row_height, 0, GRID_SPLIT);
                keys.push(printed(label, rect));
            }
        }
        keys
    }

    /// The key under `point`, in the pad's pixels.
    pub fn hit<B>(&self, app: &App<B>, point: Point) -> Option<usize> {
        self.keys(app).iter().position(|key| key.rect.offset(GAP / 2).contains(point))
    }

    /// A touch on the pad, in the panel's raw coordinates: a key acts when the finger leaves it.
    /// Returns whether the pad or the app changed.
    pub fn touch<B: crate::backend::Backend>(&mut self, app: &mut App<B>, touch: Touch) -> bool {
        match touch {
            Touch::Press(raw) => {
                self.pressed = self.hit(app, self.calibration.map(raw));
                self.pressed.is_some()
            }
            Touch::Drag(raw) => {
                // Sliding off a key lets it go without acting.
                let under = self.hit(app, self.calibration.map(raw));
                let changed = self.pressed.is_some() && under != self.pressed;
                if changed {
                    self.pressed = None;
                }
                changed
            }
            Touch::Release => {
                let Some(index) = self.pressed.take() else { return false };
                let action = self.keys(app).get(index).map(|key| key.action);
                self.act(app, action.unwrap_or(Action::None));
                true
            }
        }
    }

    /// Carries out a key's action.
    pub fn act<B: crate::backend::Backend>(&mut self, app: &mut App<B>, action: Action) {
        match action {
            Action::Key(key) => app.key(key),
            Action::Soft(index) => app.key(Key::Soft(index)),
            Action::Page => self.page = (self.page + 1) % FUNCTIONS.len(),
            Action::None => {}
        }
    }
}

/// A key of the keypad by its printed label.
fn printed(label: &str, rect: Rectangle) -> PadKey {
    let style = match label {
        "SHIFT" => Style::Shift,
        "ALPHA" => Style::Alpha,
        "UNIT" => Style::Unit,
        "EXE" => Style::Exe,
        "DEL" | "AC" => Style::Delete,
        "×" | "÷" | "+" | "−" => Style::Operator,
        "," | "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => Style::Digit,
        _ => Style::Function,
    };
    let action = key_of(label).map_or(Action::None, Action::Key);
    PadKey { rect, label: label.to_string(), action, style }
}

/// Cell `column` of a row at `top`, cells `width` × `height` from `left`, less the gap around
/// it. With `columns` > 0 the last cell reaches the right edge.
fn cell(column: i32, top: i32, width: i32, height: i32, columns: i32, left: i32) -> Rectangle {
    let x = left + column * width;
    let right = if columns > 0 && column == columns - 1 { WIDTH as i32 } else { x + width };
    Rectangle::new(
        Point::new(x + GAP / 2, top + GAP / 2),
        Size::new((right - x - GAP) as u32, (height - GAP) as u32),
    )
}
