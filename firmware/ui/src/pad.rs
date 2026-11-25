//! The Duo's input screen (firmware/docs/duo.md §3, firmware/docs/duo-keys.md): a 480×320 touch
//! screen in place of the keypad, where every key is a soft key. One frame for every context:
//!
//! - a top row: SHIFT, ALPHA, UNIT, OPTN, DEL, AC;
//! - 12 mode keys on the left, the keys of the mode and the screen in the order of their rank;
//!   with more than 12, the last one turns the page (the only page key);
//! - a number block of 4 × 4 on the right: the digits and + − × ÷, or 0–F in Basis-N.
//!
//! An Analysis answer or graph shows its view keys over the whole pad, menus a large number
//! block, and a line of text a letter keyboard. The arrows, OK, EXE and MENU are hardware
//! buttons. A tap sends the [`Key`] a key of the keypad would, or carries out a soft key as its
//! strip would, so the calculator works as on the handheld. The app holds the pad
//! ([`App::pad`]) and takes its touches ([`App::pad_touch`]); it has a calibration of its own.

use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

use crate::app::{App, Mode, Screen};
use crate::key::Key;
use crate::legend::key_of;
use crate::settings::NumberBase;
use crate::sheet;
use crate::soft::{self, Slot};
use crate::touch::{self, Calibrating, Calibration, Fed, Touch};
use crate::tree;

pub const WIDTH: u32 = 480;
pub const HEIGHT: u32 = 320;

/// The top row's height, and the four rows under it.
const TOP: i32 = 44;
const ROWS: i32 = 4;
const ROW: i32 = (HEIGHT as i32 - TOP) / ROWS;
/// The mode keys: three columns left of the number block.
const CELL_COLUMNS: i32 = 3;
const CELLS: usize = (CELL_COLUMNS * ROWS) as usize;
const BLOCK_LEFT: i32 = 216;
/// The room between two keys.
const GAP: i32 = 4;
/// The room a label may take on its key.
pub const LABEL_ROOM: i32 = 3;

/// The top row.
pub const CONTROL: [&str; 6] = ["SHIFT", "ALPHA", "UNIT", "OPTN", "DEL", "AC"];

const NUMBERS: [&str; 16] = ["7", "8", "9", "÷", "4", "5", "6", "×", "1", "2", "3", "−", "0", ",", "Ans", "+"];
/// The hex digits as hex keypads lay them out: 0 at the bottom left, F at the top right.
const HEX: [char; 16] = ['C', 'D', 'E', 'F', '8', '9', 'A', 'B', '4', '5', '6', '7', '0', '1', '2', '3'];

/// What a key of the pad does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Key(Key),
    /// A soft key, carried out as its strip does.
    Slot(Slot),
    /// LaTeX typed at the cursor, for a key the keypad has only on a layer.
    Insert(&'static str),
    /// The next page of mode keys.
    Page,
    /// The letter keyboard's capitals, and its digits and signs.
    Caps,
    Signs,
    /// Nothing here.
    None,
}

/// How a key looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    /// Its label is LaTeX.
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

/// Whether a key is drawn as usual, faint (it does nothing here, or waits to be built) or lit
/// (the base set now).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Look {
    #[default]
    Plain,
    Faint,
    Lit,
}

/// A key as the pad shows it now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PadKey {
    pub rect: Rectangle,
    /// The printed label ([`crate::legend`]), or LaTeX for [`Style::Soft`].
    pub label: String,
    pub action: Action,
    pub style: Style,
    pub look: Look,
}

/// What the pad shows: its page resets when this changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Context {
    Keyboard,
    View,
    Numbers,
    App,
    Calc { mode: Mode, shown: bool, sheet: Option<sheet::Kind>, symbols: bool },
}

/// The pad's own state: the page of mode keys, the key under the finger, the letter keyboard's
/// layer, and its touch calibration (the main screen keeps its own in the app).
#[derive(Clone, Debug, Default)]
pub struct Pad {
    pub page: usize,
    pub pressed: Option<usize>,
    /// Capitals, and digits and signs, on the letter keyboard.
    pub caps: bool,
    pub signs: bool,
    pub calibration: Calibration,
    /// The calibration screen, while it shows on the pad instead of the keys.
    pub calibrating: Option<Calibrating>,
    /// A calibration just made, for the board to store.
    calibrated: Option<Calibration>,
    /// What the page was turned on.
    context: Option<Context>,
}

/// A key before it has its place.
#[derive(Clone, Debug)]
struct Item {
    label: String,
    action: Action,
    style: Style,
    look: Look,
}

impl Item {
    /// A key of the keypad by its printed label.
    fn printed(label: &str) -> Item {
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
        let look = if action == Action::None { Look::Faint } else { Look::Plain };
        Item { label: label.to_string(), action, style, look }
    }

    /// A key that types a letter: a name in Matrizen, a hex digit in Basis-N.
    fn letter(letter: char) -> Item {
        Item { label: letter.to_string(), action: Action::Key(Key::Letter(letter)), style: Style::Digit, look: Look::Plain }
    }

    fn insert(latex_label: &str, latex: &'static str) -> Item {
        Item { label: latex_label.to_string(), action: Action::Insert(latex), style: Style::Soft, look: Look::Plain }
    }

    /// A soft key, labelled as its strip labels it.
    fn slot<B>(slot: Slot, app: &App<B>) -> Item {
        let lang = app.settings.language;
        let label = soft::Bar { slots: vec![slot.clone()], columns: 1, page: 0, pages: 1 }.label(0, lang);
        let look = match &slot {
            Slot::Command(soft) if soft.entry.kind == crate::menu::Kind::Pending => Look::Faint,
            Slot::Base(_, value) if soft::is_set(&app.settings, *value) => Look::Lit,
            _ => Look::Plain,
        };
        Item { label, action: Action::Slot(slot), style: Style::Soft, look }
    }

    fn commands<B>(softs: &'static [tree::Soft], app: &App<B>) -> Vec<Item> {
        softs.iter().map(|soft| Item::slot(Slot::Command(soft), app)).collect()
    }

    fn faint(mut self) -> Item {
        self.look = Look::Faint;
        self.action = Action::None;
        self
    }
}

fn printed(labels: &[&str]) -> Vec<Item> {
    labels.iter().map(|label| Item::printed(label)).collect()
}

/// The calculating core: the A keys of every calculating mode (duo-keys.md §2).
const CORE: [&str; 6] = ["(", ")", "(−)", "a/b", "xʸ", "√"];
const FUNCTIONS: [&str; 7] = ["sin", "cos", "tan", "ln", "log", "π", "e"];

impl Pad {
    pub fn new() -> Pad {
        Pad::default()
    }

    /// The keys as they are now.
    pub fn keys<B>(&self, app: &App<B>) -> Vec<PadKey> {
        let context = context(app);
        if context == Context::Keyboard {
            return keyboard(app, self.caps, self.signs);
        }
        let mut keys = top_row(app);
        match context {
            Context::View => keys.extend(view(app)),
            Context::Numbers => keys.extend(wide_numbers()),
            _ => {
                let (cells, block) = layout(app, context);
                let page = if self.context == Some(context) { self.page } else { 0 };
                keys.extend(paged(cells, page));
                keys.extend(block);
            }
        }
        keys
    }

    /// The key under `point`, in the pad's pixels.
    pub fn hit<B>(&self, app: &App<B>, point: Point) -> Option<usize> {
        self.keys(app).iter().position(|key| key.rect.offset(GAP / 2).contains(point))
    }

    /// The targets of its calibration.
    pub fn targets() -> [Point; 5] {
        touch::targets(Size::new(WIDTH, HEIGHT))
    }

    /// Shows the calibration screen instead of the keys.
    pub fn calibrate(&mut self) {
        self.pressed = None;
        self.calibrating = Some(Calibrating::default());
    }

    /// A calibration made since the last call, to be stored.
    pub fn take_calibration(&mut self) -> Option<Calibration> {
        self.calibrated.take()
    }

    /// A touch on the pad, in the panel's raw coordinates: a key acts when the finger leaves it.
    /// Returns whether the pad or the app changed.
    pub(crate) fn touch<B: crate::backend::Backend>(&mut self, app: &mut App<B>, touch: Touch) -> bool {
        if let Some(state) = &mut self.calibrating {
            return match state.feed(touch, &Self::targets()) {
                Fed::Going(changed) => changed,
                Fed::Done(calibration) => {
                    self.calibration = calibration;
                    self.calibrated = Some(calibration);
                    self.calibrating = None;
                    true
                }
            };
        }
        // A new context starts on its first page.
        let now = context(app);
        if self.context != Some(now) {
            self.context = Some(now);
            self.page = 0;
        }
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
                let action = self.keys(app).get(index).map(|key| key.action.clone());
                self.act(app, action.unwrap_or(Action::None));
                true
            }
        }
    }

    /// Carries out a key's action.
    pub(crate) fn act<B: crate::backend::Backend>(&mut self, app: &mut App<B>, action: Action) {
        match action {
            Action::Key(key) => {
                app.key(key);
                // A capital is typed once, as on a phone.
                self.caps = false;
            }
            Action::Slot(slot) => app.pad_slot(slot),
            Action::Insert(latex) => app.pad_insert(latex),
            Action::Page => self.page += 1,
            Action::Caps => self.caps = !self.caps,
            Action::Signs => self.signs = !self.signs,
            Action::None => {}
        }
    }
}

/// What the pad shows for the app as it is.
fn context<B>(app: &App<B>) -> Context {
    if app.text_entry() {
        return Context::Keyboard;
    }
    match app.screen {
        Screen::Calc if app.answer.is_some() => Context::View,
        Screen::Calc => Context::Calc {
            mode: app.mode,
            shown: app.shown.is_some(),
            sheet: app.sheet_kind(),
            symbols: app.symbols.is_some(),
        },
        Screen::App if app.running.as_ref().is_some_and(|running| running.menu.is_none()) => Context::App,
        _ => Context::Numbers,
    }
}

/// The top row; UNIT is faint outside Physik, where units have no use.
fn top_row<B>(app: &App<B>) -> Vec<PadKey> {
    let width = WIDTH as i32 / CONTROL.len() as i32;
    let physics = matches!(app.screen, Screen::Calc) && app.mode == Mode::Physics;
    CONTROL
        .iter()
        .enumerate()
        .map(|(column, label)| {
            let item = Item::printed(label);
            let item = if *label == "UNIT" && !physics { item.faint() } else { item };
            place(item, cell(column as i32 * width, 0, width, TOP))
        })
        .collect()
}

/// The mode keys in the order of their rank, and the number block, for a calculating context.
fn layout<B>(app: &App<B>, context: Context) -> (Vec<Item>, Vec<PadKey>) {
    let numbers = || number_block(NUMBERS.iter().map(|label| Item::printed(label)).collect());
    if context == Context::App {
        // The app's soft keys first, then what a line of maths needs.
        let mut cells: Vec<Item> = soft::slots(app).unwrap_or_default().into_iter().map(|slot| Item::slot(slot, app)).collect();
        if app.running.as_ref().is_some_and(|running| running.applet.input().line()) {
            cells.extend(printed(&CORE));
            cells.extend(printed(&FUNCTIONS));
        }
        return (cells, numbers());
    }
    let Context::Calc { mode, shown, sheet, symbols } = context else { return (Vec::new(), numbers()) };
    let bar = || -> Vec<Item> { soft::slots(app).unwrap_or_default().into_iter().map(|slot| Item::slot(slot, app)).collect() };
    if symbols {
        return (bar(), numbers());
    }
    if let Some(kind) = sheet {
        return (sheet_keys(kind, mode), numbers());
    }
    if mode == Mode::BaseN {
        return base_n(app);
    }
    let mut cells = Vec::new();
    // Over a result its forms lead: S⇔D, ENG, ←ENG, Physik's conversions, Komplex' forms.
    let result = if shown { bar() } else { Vec::new() };
    let (lead, rest) = result.split_at(result.len().min(5));
    cells.extend_from_slice(lead);
    match mode {
        Mode::Physics => {
            // The units that fit the input: the best three first, the rest after the core and
            // what numbers in physics need, ×10ˣ and the constants.
            let units = if shown { Vec::new() } else { bar() };
            let (best, more) = units.split_at(units.len().min(3));
            cells.extend_from_slice(best);
            cells.extend(printed(&CORE));
            cells.extend(printed(&["×10ˣ", "CONST", "x²", "S⇔D", "ENG"]));
            cells.extend_from_slice(more);
            cells.extend(printed(&FUNCTIONS));
        }
        Mode::Algebra | Mode::Linear => {
            cells.extend(printed(&["x", "y", "=", ";", "(", ")", "a/b", "xʸ", "√", "x²", "(−)"]));
            cells.extend(printed(&FUNCTIONS));
            // Today's bar but its `;`, which leads already.
            cells.extend(Item::commands(&tree::SOFT_ALGEBRA[1..], app));
        }
        Mode::Imaginary => {
            cells.push(Item::printed("i"));
            cells.push(Item::insert(r"\angle", "∠"));
            cells.extend(printed(&CORE));
            // Today's bar but its ∠, which leads already.
            cells.extend(Item::commands(&tree::SOFT_COMPLEX[..7], app));
            cells.push(Item::printed("x²"));
            cells.extend(printed(&FUNCTIONS));
        }
        Mode::Analysis => {
            cells.push(Item::printed("x"));
            cells.extend(printed(&CORE));
            cells.extend(printed(&["e", "ln", "sin", "cos"]));
            cells.extend(Item::commands(&tree::SOFT_ANALYSIS, app));
            cells.extend(printed(&["tan", "log", "π", "x²"]));
        }
        Mode::Matrix => {
            cells.extend("ABCDEFM".chars().map(Item::letter));
            cells.extend(Item::commands(&tree::SOFT_MATRIX, app));
            cells.extend(printed(&CORE));
            cells.extend(printed(&["x²"]));
            cells.extend(printed(&FUNCTIONS));
        }
        Mode::Verify => {
            cells.extend(Item::commands(&tree::SOFT_VERIFY, app));
            cells.extend(printed(&CORE));
            cells.extend(printed(&["x", "y", "x²"]));
            cells.extend(printed(&FUNCTIONS));
        }
        _ => {
            cells.extend(printed(&CORE));
            cells.extend(printed(&["x", "x²"]));
            cells.extend(printed(&FUNCTIONS));
        }
    }
    cells.extend_from_slice(rest);
    (cells, numbers())
}

/// The keys of a sheet: numbers first, and what the form takes.
fn sheet_keys(kind: sheet::Kind, mode: Mode) -> Vec<Item> {
    let mut cells = printed(&["(−)", "a/b", "√"]);
    match kind {
        sheet::Kind::Table => cells.push(Item::printed("x")),
        sheet::Kind::Linear => cells.push(Item::printed("i")),
        sheet::Kind::Distribution => cells.push(Item::printed(";")),
        sheet::Kind::Matrix => cells.extend("ABCDEFM".chars().map(Item::letter)),
        _ => {}
    }
    if mode == Mode::Linear && kind != sheet::Kind::Linear {
        cells.push(Item::printed("i"));
    }
    cells.extend(printed(&["(", ")", "xʸ", "x²", "×10ˣ"]));
    cells.extend(printed(&FUNCTIONS));
    cells
}

/// Basis-N: the bases, the logic and + − × ÷ on the left; 0–F in the block, those that are not
/// digits of the base set faint.
fn base_n<B>(app: &App<B>) -> (Vec<Item>, Vec<PadKey>) {
    let base = |label: &'static str, value| Item::slot(Slot::Base(label, value), app);
    let logic = |index: usize| Item::slot(Slot::Command(&tree::SOFT_BASE_N[index]), app);
    let mut cells = vec![base("DEC", 0), base("HEX", 1), base("BIN", 2), base("OCT", 3)];
    // and, or, xor, Not; xnor and Neg are rarer than the operators.
    cells.extend([logic(0), logic(1), logic(2), logic(4)]);
    cells.extend(printed(&["+", "−", "×", "÷"]));
    cells.extend([logic(3), logic(5)]);
    cells.extend(printed(&["(", ")", "Ans"]));
    let digits = match app.settings.number_base {
        NumberBase::Dec => 10,
        NumberBase::Hex => 16,
        NumberBase::Bin => 2,
        NumberBase::Oct => 8,
    };
    let block = HEX
        .iter()
        .map(|&digit| {
            let value = digit.to_digit(16).expect("a hex digit");
            let item = match digit.to_digit(10) {
                Some(_) => Item::printed(&digit.to_string()),
                None => Item::letter(digit),
            };
            if value < digits { item } else { item.faint() }
        })
        .collect();
    (cells, number_block(block))
}

/// `items` on the mode keys: all of them if they fit, else page `page` of eleven and the page
/// key.
fn paged(items: Vec<Item>, page: usize) -> Vec<PadKey> {
    let (shown, page_key) = if items.len() <= CELLS {
        (items, None)
    } else {
        let per_page = CELLS - 1;
        let pages = items.len().div_ceil(per_page);
        let page = page % pages;
        let shown: Vec<Item> = items.into_iter().skip(page * per_page).take(per_page).collect();
        let label = format!(r"\blacktriangleright {}/{pages}", page + 1);
        (shown, Some(Item { label, action: Action::Page, style: Style::Soft, look: Look::Plain }))
    };
    let width = BLOCK_LEFT / CELL_COLUMNS;
    let at = |index: usize| {
        let (row, column) = (index as i32 / CELL_COLUMNS, index as i32 % CELL_COLUMNS);
        cell(column * width, TOP + row * ROW, width, ROW)
    };
    let mut keys: Vec<PadKey> = shown.into_iter().enumerate().map(|(index, item)| place(item, at(index))).collect();
    if let Some(item) = page_key {
        keys.push(place(item, at(CELLS - 1)));
    }
    keys
}

/// 16 keys as the 4 × 4 block right of the mode keys.
fn number_block(items: Vec<Item>) -> Vec<PadKey> {
    let width = (WIDTH as i32 - BLOCK_LEFT) / 4;
    items
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            let (row, column) = (index as i32 / 4, index as i32 % 4);
            place(item, cell(BLOCK_LEFT + column * width, TOP + row * ROW, width, ROW))
        })
        .collect()
}

/// The digits over the whole width, for menus and lists: 1–9 pick a line.
fn wide_numbers() -> Vec<PadKey> {
    let labels = ["7", "8", "9", "4", "5", "6", "1", "2", "3", "(−)", "0", ","];
    let width = WIDTH as i32 / 3;
    labels
        .iter()
        .enumerate()
        .map(|(index, label)| {
            let (row, column) = (index as i32 / 3, index as i32 % 3);
            place(Item::printed(label), cell(column * width, TOP + row * ROW, width, ROW))
        })
        .collect()
}

/// An Analysis answer or graph: the keys its pages take (`sketch_key`, `graph_key`) over the
/// whole pad.
fn view<B>(app: &App<B>) -> Vec<PadKey> {
    use crate::text;
    let lang = app.settings.language;
    let named = |text: text::Text, key: Key| Item { label: text.get(lang).to_string(), action: Action::Key(key), style: Style::Function, look: Look::Plain };
    let arrow = |label: &str, key: Key| Item { label: label.to_string(), action: Action::Key(key), style: Style::Function, look: Look::Plain };
    let rows = [
        vec![arrow("◄", Key::Left), arrow("▲", Key::Up), arrow("►", Key::Right)],
        vec![named(text::PAD_PAN, Key::Ok), arrow("▼", Key::Down), named(text::PAD_HOME, Key::Digit(0))],
        vec![named(text::PAD_ZOOM_IN, Key::Plus), named(text::PAD_ZOOM_OUT, Key::Minus)],
        vec![named(text::PAD_CLOSE, Key::Ac)],
    ];
    let mut keys = Vec::new();
    for (row, items) in rows.into_iter().enumerate() {
        let width = WIDTH as i32 / items.len() as i32;
        for (column, item) in items.into_iter().enumerate() {
            keys.push(place(item, cell(column as i32 * width, TOP + row as i32 * ROW, width, ROW)));
        }
    }
    keys
}

/// The letter keyboard over the whole pad, for a line of text: QWERTZ in German, QWERTY
/// otherwise; Aa for one capital, 123 for digits and signs.
fn keyboard<B>(app: &App<B>, caps: bool, signs: bool) -> Vec<PadKey> {
    let german = app.settings.language.code() == "de";
    let rows: [&str; 3] = match (signs, german) {
        (true, _) => ["1234567890", "-/:;()&@\"", "#%+=*_?!'"],
        (false, true) => ["qwertzuiop", "asdfghjkl", "yxcvbnm.-"],
        (false, false) => ["qwertyuiop", "asdfghjkl", "zxcvbnm.-"],
    };
    let typed = |c: char| {
        let c = if caps { c.to_uppercase().next().unwrap_or(c) } else { c };
        let action = Key::from_char(c).map_or(Action::None, Action::Key);
        Item { label: c.to_string(), action, style: Style::Digit, look: Look::Plain }
    };
    let control = |label: &str, action: Action| Item { label: label.to_string(), action, style: Style::Function, look: Look::Plain };
    let row_height = HEIGHT as i32 / 4;
    let width = WIDTH as i32 / 10;
    let mut keys = Vec::new();
    let mut put = |item: Item, row: i32, column: i32, span: i32| {
        keys.push(place(item, cell(column * width, row * row_height, span * width, row_height)));
    };
    for (column, c) in rows[0].chars().enumerate() {
        put(typed(c), 0, column as i32, 1);
    }
    for (column, c) in rows[1].chars().enumerate() {
        put(typed(c), 1, column as i32, 1);
    }
    put(control("DEL", Action::Key(Key::Del)), 1, 9, 1);
    let caps_look = if caps { Look::Lit } else { Look::Plain };
    put(Item { look: caps_look, ..control("Aa", Action::Caps) }, 2, 0, 1);
    for (column, c) in rows[2].chars().enumerate() {
        put(typed(c), 2, column as i32 + 1, 1);
    }
    let last: [char; 3] = if german && !signs { ['ä', 'ö', 'ü'] } else { [',', '.', '?'] };
    put(control(if signs { "abc" } else { "123" }, Action::Signs), 3, 0, 1);
    for (column, c) in last.into_iter().enumerate() {
        put(typed(c), 3, column as i32 + 1, 1);
    }
    put(control(crate::text::PAD_SPACE.get(app.settings.language), Action::Key(Key::Space)), 3, 4, 3);
    put(Item { style: Style::Delete, ..control("AC", Action::Key(Key::Ac)) }, 3, 7, 1);
    put(Item { style: Style::Exe, ..control("OK", Action::Key(Key::Ok)) }, 3, 8, 2);
    keys
}

fn place(item: Item, rect: Rectangle) -> PadKey {
    PadKey { rect, label: item.label, action: item.action, style: item.style, look: item.look }
}

/// The cell at `(x, y)` of `width` × `height`, less the gap around it.
fn cell(x: i32, y: i32, width: i32, height: i32) -> Rectangle {
    Rectangle::new(Point::new(x + GAP / 2, y + GAP / 2), Size::new((width - GAP) as u32, (height - GAP) as u32))
}
