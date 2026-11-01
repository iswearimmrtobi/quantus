//! The calculator's state and what each key does to it.

use crate::backend::Backend;
use crate::editor::{Editor, Row};
use crate::key::Key;
use crate::latex;
use crate::tree;
use crate::menu::{Action, Menu, Outcome};
pub use crate::settings::Settings;

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

/// Tiles per page of the mode menu, in [`MENU_COLUMNS`] columns.
pub const TILES_PER_PAGE: usize = 6;
pub const MENU_COLUMNS: usize = 3;

/// What fills the screen below the top bar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Screen {
    Calc,
    /// The mode menu, with the selected tile counted over all pages.
    Modes { selected: usize },
    /// SETUP or OPTN.
    Menu(Menu),
}

/// A calculation in the history.
pub struct Entry {
    pub input: Row,
    pub result: Result<Row, String>,
    /// Whether the result is written as a decimal; S⇔D flips it.
    pub decimal: bool,
}

/// How long the caret stays on and off.
const BLINK_MS: u64 = 500;

/// Older entries are dropped.
const HISTORY: usize = 50;

pub struct App<B> {
    backend: B,
    pub mode: Mode,
    pub settings: Settings,
    pub screen: Screen,
    pub shift: bool,
    pub alpha: bool,
    pub editor: Editor,
    /// Oldest first.
    pub history: Vec<Entry>,
    /// The entry on the screen, counted from the newest: the editor holds its input and its
    /// result is shown. `None` while an input is being typed.
    pub shown: Option<usize>,
    /// The charge in percent, if the board can tell.
    pub battery: Option<u8>,
    pub caret_on: bool,
    last_blink: u64,
}

impl<B> App<B> {
    /// The entry whose result is on the screen.
    pub fn shown_entry(&self) -> Option<&Entry> {
        self.history.iter().rev().nth(self.shown?)
    }
}

impl<B: Backend> App<B> {
    pub fn new(backend: B) -> Self {
        App {
            backend,
            mode: Mode::Physics,
            settings: Settings::default(),
            screen: Screen::Calc,
            shift: false,
            alpha: false,
            editor: Editor::new(),
            history: Vec::new(),
            shown: None,
            battery: None,
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
        let shift = std::mem::take(&mut self.shift);
        let alpha = std::mem::take(&mut self.alpha);
        match std::mem::replace(&mut self.screen, Screen::Calc) {
            Screen::Modes { selected } => self.modes_key(selected, key),
            Screen::Menu(mut menu) => match menu.key(key, &mut self.settings) {
                Outcome::Stay => self.screen = Screen::Menu(menu),
                Outcome::Close => {}
                Outcome::Insert(latex) => self.insert(latex),
                Outcome::Act(action) => {
                    self.act(action);
                    self.screen = Screen::Menu(menu);
                }
            },
            Screen::Calc => self.calc_key(key, shift, alpha),
        }
    }

    /// Does what a menu entry asks for.
    fn act(&mut self, action: Action) {
        match action {
            Action::ResetSettings => self.settings = Settings::default(),
            Action::ResetMemory => {
                self.history.clear();
                self.reset();
            }
            Action::ResetAll => {
                self.settings = Settings::default();
                self.history.clear();
                self.reset();
            }
            Action::ClearInput => self.reset(),
        }
    }

    /// Puts LaTeX from a menu into the input, as if typed: a shown result is left first.
    fn insert(&mut self, latex: &str) {
        if self.shown.is_some() {
            self.reset();
        }
        self.editor.paste(&latex::parse(latex));
    }

    fn calc_key(&mut self, key: Key, shift: bool, alpha: bool) {
        match key {
            Key::Shift => return self.shift = !shift,
            Key::Alpha => return self.alpha = !alpha,
            Key::Mode if shift => return self.screen = Screen::Menu(Menu::new(&tree::SETUP)),
            Key::Mode => return self.screen = Screen::Modes { selected: self.mode.index() },
            _ => {}
        }
        if self.shown.is_some() && !self.leave_result(key) {
            return;
        }
        let editor = &mut self.editor;
        match key {
            Key::Digit(digit) => editor.char(char::from(b'0' + digit.min(9))),
            Key::Letter(c) => editor.char(c),
            Key::Comma => editor.char(','),
            Key::Semicolon => editor.char(';'),
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
                    self.recall(0);
                }
            }
            Key::Down => {
                editor.down();
            }
            Key::Del => editor.delete(),
            Key::Ac => editor.clear(),
            Key::Ok | Key::Exe => self.evaluate(),
            Key::SwapForm | Key::Shift | Key::Alpha | Key::Mode => {}
        }
    }

    /// A key pressed while a result is shown, as on a Casio: an operator goes on with the result,
    /// ←/→/DEL edit the input again, ↑/↓ step through the history, other keys start afresh.
    /// Returns whether the key still goes to the editor.
    fn leave_result(&mut self, key: Key) -> bool {
        let Some(shown) = self.shown else { return true };
        match key {
            Key::Up => {
                self.recall(shown + 1);
                false
            }
            Key::Down => {
                match shown {
                    0 => self.reset(),
                    _ => self.recall(shown - 1),
                }
                false
            }
            Key::Ok | Key::Exe => false,
            Key::SwapForm => {
                self.swap_form(shown);
                false
            }
            Key::Left | Key::Right => {
                self.shown = None;
                false
            }
            Key::Del => {
                self.shown = None;
                true
            }
            Key::Ac => {
                self.reset();
                false
            }
            Key::Plus | Key::Minus | Key::Times | Key::Divide | Key::Power | Key::Square | Key::Fraction => {
                let result = self.shown_entry().and_then(|entry| entry.result.as_ref().ok()).cloned();
                self.reset();
                if let Some(result) = result {
                    self.editor.paste(&result);
                }
                true
            }
            _ => {
                self.reset();
                true
            }
        }
    }

    /// An empty input, no result.
    fn reset(&mut self) {
        self.editor.clear();
        self.shown = None;
    }

    fn modes_key(&mut self, selected: usize, key: Key) {
        let count = Mode::ALL.len();
        let row = selected % TILES_PER_PAGE / MENU_COLUMNS;
        let selected = match key {
            Key::Digit(digit @ 1..=9) if usize::from(digit) <= count => {
                return self.choose(usize::from(digit) - 1);
            }
            Key::Ok | Key::Exe => return self.choose(selected),
            Key::Ac | Key::Mode | Key::Del => return self.screen = Screen::Calc,
            Key::Left => selected.saturating_sub(1),
            Key::Right => (selected + 1).min(count - 1),
            Key::Up if row > 0 => selected - MENU_COLUMNS,
            Key::Down if row + 1 < TILES_PER_PAGE / MENU_COLUMNS => {
                (selected + MENU_COLUMNS).min(count - 1)
            }
            _ => selected,
        };
        self.screen = Screen::Modes { selected };
    }

    fn choose(&mut self, index: usize) {
        self.mode = Mode::ALL[index];
        self.screen = Screen::Calc;
    }

    /// Shows the entry `index` places back from the newest, its input in the editor.
    fn recall(&mut self, index: usize) {
        let Some(entry) = self.history.iter().rev().nth(index) else { return };
        self.editor.set(entry.input.clone());
        self.shown = Some(index);
    }

    /// Evaluates the shown entry again, as a decimal if it was exact and the other way round.
    fn swap_form(&mut self, shown: usize) {
        let index = self.history.len() - 1 - shown;
        let entry = &self.history[index];
        let settings = Settings { decimal: !entry.decimal, ..self.settings };
        let result = self.backend.evaluate(self.mode, &settings, &entry.input.to_latex());
        let entry = &mut self.history[index];
        entry.result = result.map(|latex| latex::parse(&latex));
        entry.decimal = settings.decimal;
    }

    fn evaluate(&mut self) {
        if self.editor.is_empty() {
            return;
        }
        let result = self.backend.evaluate(self.mode, &self.settings, &self.editor.to_latex());
        let input = self.editor.row.clone();
        self.history.push(Entry {
            input: input.clone(),
            result: result.map(|latex| latex::parse(&latex)),
            decimal: self.settings.decimal,
        });
        if self.history.len() > HISTORY {
            self.history.remove(0);
        }
        self.editor.set(input);
        self.shown = Some(0);
    }
}
