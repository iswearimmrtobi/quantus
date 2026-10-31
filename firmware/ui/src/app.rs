//! The calculator's state and what each key does to it.

use crate::backend::Backend;
use crate::editor::{Editor, Row};
use crate::key::Key;
use crate::latex;

/// The app's modes that work as text, in the order of the mode menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Algebra,
    Physics,
    Imaginary,
    Analysis,
    Linear,
}

impl Mode {
    pub const ALL: [Mode; 5] = [Mode::Algebra, Mode::Physics, Mode::Imaginary, Mode::Analysis, Mode::Linear];

    pub fn name(self) -> &'static str {
        match self {
            Mode::Algebra => "Algebra",
            Mode::Physics => "Physics",
            Mode::Imaginary => "Complex",
            Mode::Analysis => "Analysis",
            Mode::Linear => "Linear systems",
        }
    }

    fn index(self) -> usize {
        Mode::ALL.iter().position(|&mode| mode == self).unwrap_or(0)
    }
}

/// A calculation in the history.
pub struct Entry {
    pub input: Row,
    pub result: Result<Row, String>,
}

/// How long the caret stays on and off.
const BLINK_MS: u64 = 500;

/// Older entries are dropped.
const HISTORY: usize = 50;

pub struct App<B> {
    backend: B,
    pub mode: Mode,
    pub shift: bool,
    pub editor: Editor,
    /// Oldest first.
    pub history: Vec<Entry>,
    /// The entry whose input ↑ brought back, counted from the newest.
    recalled: Option<usize>,
    /// The selected line while the mode menu is open.
    pub menu: Option<usize>,
    pub caret_on: bool,
    last_blink: u64,
}

impl<B: Backend> App<B> {
    pub fn new(backend: B) -> Self {
        App {
            backend,
            mode: Mode::Physics,
            shift: false,
            editor: Editor::new(),
            history: Vec::new(),
            recalled: None,
            menu: None,
            caret_on: true,
            last_blink: 0,
        }
    }

    /// Blinks the caret; returns whether the screen changed.
    pub fn tick(&mut self, now_ms: u64) -> bool {
        if now_ms.saturating_sub(self.last_blink) < BLINK_MS {
            return false;
        }
        self.last_blink = now_ms;
        self.caret_on = !self.caret_on;
        true
    }

    pub fn key(&mut self, key: Key) {
        self.caret_on = true;
        if let Some(selected) = self.menu {
            self.menu_key(selected, key);
            return;
        }
        let shift = std::mem::take(&mut self.shift);
        let editor = &mut self.editor;
        match key {
            Key::Digit(digit) => editor.char(char::from(b'0' + digit.min(9))),
            Key::Letter(c) => editor.char(c),
            Key::Comma => editor.char(','),
            Key::Plus => editor.char('+'),
            Key::Minus => editor.char('-'),
            Key::Times => editor.char('×'),
            Key::Divide => editor.char('÷'),
            Key::Equals => editor.char('='),
            Key::OpenParen => editor.char('('),
            Key::CloseParen => editor.char(')'),
            Key::Pi => editor.char('π'),
            Key::E => editor.char('e'),
            Key::Fraction => editor.fraction(),
            Key::Power => editor.power(),
            Key::Square => editor.square(),
            Key::Root if shift => editor.root(),
            Key::Root => editor.sqrt(),
            Key::Subscript => editor.subscript(),
            Key::Sin => editor.function("sin"),
            Key::Cos => editor.function("cos"),
            Key::Tan => editor.function("tan"),
            Key::Ln => editor.function("ln"),
            Key::Log => editor.function("log"),
            Key::Ans => {
                if let Some(Entry { result: Ok(result), .. }) = self.history.last() {
                    editor.paste(result);
                }
            }
            Key::Left => {
                editor.left();
            }
            Key::Right => {
                editor.right();
            }
            Key::Up => {
                if !editor.up() {
                    self.recall(self.recalled.map_or(0, |index| index + 1));
                }
            }
            Key::Down => {
                if !editor.down() {
                    match self.recalled {
                        Some(0) | None => {
                            self.recalled = None;
                            self.editor.clear();
                        }
                        Some(index) => self.recall(index - 1),
                    }
                }
            }
            Key::Del => editor.delete(),
            Key::Ac => {
                editor.clear();
                self.recalled = None;
            }
            Key::Ok | Key::Exe => self.evaluate(),
            Key::Shift => self.shift = !shift,
            Key::Mode => self.menu = Some(self.mode.index()),
        }
    }

    fn menu_key(&mut self, selected: usize, key: Key) {
        let count = Mode::ALL.len();
        match key {
            Key::Digit(digit @ 1..=5) => self.choose(usize::from(digit) - 1),
            Key::Up => self.menu = Some((selected + count - 1) % count),
            Key::Down => self.menu = Some((selected + 1) % count),
            Key::Ok | Key::Exe => self.choose(selected),
            Key::Ac | Key::Mode | Key::Del => self.menu = None,
            _ => {}
        }
    }

    fn choose(&mut self, index: usize) {
        self.mode = Mode::ALL[index];
        self.menu = None;
    }

    /// Puts the input of the entry `index` places back from the newest into the editor.
    fn recall(&mut self, index: usize) {
        let Some(entry) = self.history.iter().rev().nth(index) else { return };
        self.editor.set(entry.input.clone());
        self.recalled = Some(index);
    }

    fn evaluate(&mut self) {
        if self.editor.is_empty() {
            return;
        }
        let result = self.backend.evaluate(self.mode, &self.editor.to_latex());
        let input = std::mem::take(&mut self.editor.row);
        self.history.push(Entry { input, result: result.map(|latex| latex::parse(&latex)) });
        if self.history.len() > HISTORY {
            self.history.remove(0);
        }
        self.editor.clear();
        self.recalled = None;
    }
}
