//! The soft keys: two rows of keys under the screen whose labels are drawn in two strips at its
//! bottom (firmware/docs/hardware.md §4). What they hold follows the mode and the input
//! (design/handheld-menus.md §7.4.1, firmware/docs/unit-keys.md §2):
//!
//! - while an input is typed, the mode's commands ([`tree::soft_keys`]); in Physik the units
//!   that fit the input before the cursor, asked of the backend;
//! - while a result is shown, its forms: S⇔D, ENG, ←ENG, and in Physik the units it converts to;
//! - after SYMB, pages of Greek letters and math symbols, one of them typed with one key;
//! - on a sheet, an answer page or a graph, nothing: the strips are hidden and the keys inert.
//!
//! More than fit go on pages; the last key then turns the page.

use crate::app::{App, Mode, Screen};
use crate::backend::Item;
use crate::editor::{Node, Row};
use crate::key::Key;
use crate::settings::Setting;
use crate::text::Lang;
use crate::tree::{self, Soft};

/// What one soft key does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Slot {
    /// Nothing, to keep the units of the first row together.
    Empty,
    /// A command, as OPTN carries it out.
    Command(&'static Soft),
    /// A unit typed at the cursor.
    Unit(Item),
    /// A shown result converted to a unit (`▶ kJ`).
    Convert(Item),
    /// A symbol of SYMB: its label, and the LaTeX it types.
    Symbol(&'static str, &'static str),
    /// A key of the keypad, by its label: S⇔D, ENG.
    Key(&'static str, Key),
    /// A number base of Basis-N.
    Base(&'static str, i32),
    /// The next page.
    Page,
    /// An app's key: its number over all pages, and its label, plain text.
    App(usize, String),
}

/// The soft keys on the screen: those of the page shown, row by row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bar {
    pub slots: Vec<Slot>,
    pub columns: usize,
    pub page: usize,
    pub pages: usize,
}

impl Bar {
    /// The slot under soft key `index`.
    pub fn slot(&self, index: usize) -> Option<&Slot> {
        self.slots.get(index)
    }

    /// The LaTeX the strip draws for slot `index`, as the layout reads it.
    pub fn label(&self, index: usize, lang: Lang) -> String {
        let Some(slot) = self.slots.get(index) else { return String::new() };
        match slot {
            Slot::Empty => String::new(),
            Slot::Command(soft) => soft.label.get(lang).to_string(),
            Slot::Unit(item) => unit_label(&item.name),
            Slot::Convert(item) => format!(r"\blacktriangleright {}", unit_label(&item.name)),
            Slot::Symbol(label, _) | Slot::Key(label, _) | Slot::Base(label, _) => label.to_string(),
            Slot::Page => format!(r"\blacktriangleright {}/{}", self.page + 1, self.pages),
            Slot::App(_, label) => label.clone(),
        }
    }

    /// Whether slot `index` waits for the engine or the firmware: drawn faint.
    pub fn is_pending(&self, index: usize) -> bool {
        matches!(self.slots.get(index), Some(Slot::Command(soft)) if soft.entry.kind == crate::menu::Kind::Pending)
    }
}

/// A unit's label as the fonts can draw it: they have the Greek µ, not the micro sign.
fn unit_label(name: &str) -> String {
    name.replace('µ', "μ")
}

/// The symbols of SYMB that lead in Physik, and in the other modes.
const PHYSICS_FIRST: &[(&str, &str)] =
    &[("Δ", r"\Delta "), ("ω", r"\omega "), ("ρ", r"\rho "), ("μ", r"\mu "), ("ε", r"\varepsilon "), ("Ω", r"\Omega "), ("°", "°")];
const MATH_FIRST: &[(&str, &str)] = &[("π", r"\pi "), ("∞", r"\infty "), ("±", r"\pm "), ("≤", r"\le "), ("≥", r"\ge ")];

/// The Greek letters, then the math symbols; what leads is not repeated.
const SYMBOLS: &[(&str, &str)] = &[
    ("α", r"\alpha "),
    ("β", r"\beta "),
    ("γ", r"\gamma "),
    ("δ", r"\delta "),
    ("ε", r"\varepsilon "),
    ("θ", r"\theta "),
    ("λ", r"\lambda "),
    ("μ", r"\mu "),
    ("ρ", r"\rho "),
    ("σ", r"\sigma "),
    ("τ", r"\tau "),
    ("φ", r"\varphi "),
    ("ω", r"\omega "),
    ("Δ", r"\Delta "),
    ("Ω", r"\Omega "),
    ("Σ", r"\Sigma "),
    ("Φ", r"\Phi "),
    ("∞", r"\infty "),
    ("±", r"\pm "),
    ("≤", r"\le "),
    ("≥", r"\ge "),
    ("≈", r"\approx "),
    ("°", "°"),
    ("'", "'"),
    ("''", "''"),
    (r"\angle", "∠"),
    ("π", r"\pi "),
];

/// The bases of Basis-N, by their values of [`Setting::NumberBase`].
const BASES: [(&str, i32); 4] = [("DEC", 0), ("HEX", 1), ("BIN", 2), ("OCT", 3)];

/// The result forms every mode with a result has.
const FORMS: [(&str, Key); 3] = [(r"S\Leftrightarrow D", Key::SwapForm), ("ENG", Key::Eng), ("←ENG", Key::EngBack)];

/// Whether the strips are on the screen.
pub fn shows<B>(app: &App<B>) -> bool {
    bar(app).is_some()
}

/// The soft keys as the screen is now, or `None` where they are hidden.
pub fn bar<B>(app: &App<B>) -> Option<Bar> {
    let columns = app.soft_columns.clamp(2, 6);
    if let (Screen::App, Some(running)) = (&app.screen, &app.running) {
        let labels = running.applet.soft_keys();
        let slots = labels
            .into_iter()
            .enumerate()
            .map(|(index, label)| if label.trim().is_empty() { Slot::Empty } else { Slot::App(index, label) })
            .collect::<Vec<_>>();
        if slots.iter().all(|slot| *slot == Slot::Empty) {
            return None;
        }
        return Some(paginate(slots, columns, app.soft_page));
    }
    if !matches!(app.screen, Screen::Calc) || app.answer.is_some() {
        return None;
    }
    if let Some(page) = app.symbols {
        let lead = if app.mode == Mode::Physics { PHYSICS_FIRST } else { MATH_FIRST };
        let rest = SYMBOLS.iter().filter(|symbol| !lead.contains(symbol));
        let slots = lead.iter().chain(rest).map(|&(label, latex)| Slot::Symbol(label, latex)).collect();
        return Some(paginate(slots, columns, page));
    }
    if app.sheet_kind().is_some() {
        return None;
    }
    let slots = match (app.shown.is_some(), app.mode) {
        (true, Mode::Physics) => {
            let mut slots: Vec<Slot> = forms();
            slots.resize(columns, Slot::Empty);
            slots.extend(app.units.best.iter().chain(&app.units.more).cloned().map(Slot::Convert));
            slots
        }
        (true, Mode::Imaginary) => {
            let mut slots = forms();
            slots.extend(tree::SOFT_COMPLEX[5..7].iter().map(Slot::Command));
            slots
        }
        (true, Mode::Algebra | Mode::Matrix) => forms(),
        (_, Mode::Physics) => {
            // The best guesses fill the first row, the rest follow on the second.
            let mut slots: Vec<Slot> = app.units.best.iter().take(columns).cloned().map(Slot::Unit).collect();
            let rest: Vec<Slot> = app.units.best.iter().skip(columns).chain(&app.units.more).cloned().map(Slot::Unit).collect();
            if !rest.is_empty() {
                slots.resize(columns, Slot::Empty);
            }
            slots.extend(rest);
            slots
        }
        (_, Mode::BaseN) => {
            let bases = BASES.iter().map(|&(label, value)| Slot::Base(label, value));
            bases.chain(tree::SOFT_BASE_N.iter().map(Slot::Command)).collect()
        }
        (_, mode) => tree::soft_keys(mode).iter().map(Slot::Command).collect(),
    };
    if slots.iter().all(|slot| *slot == Slot::Empty) {
        return None;
    }
    Some(paginate(slots, columns, app.soft_page))
}

fn forms() -> Vec<Slot> {
    FORMS.iter().map(|&(label, key)| Slot::Key(label, key)).collect()
}

/// The page `page` of `slots` on two rows of `columns` keys: when they do not fit, each page
/// keeps its last key for turning the page.
fn paginate(slots: Vec<Slot>, columns: usize, page: usize) -> Bar {
    let keys = 2 * columns;
    if slots.len() <= keys {
        return Bar { slots, columns, page: 0, pages: 1 };
    }
    let per_page = keys - 1;
    let pages = slots.len().div_ceil(per_page);
    let page = page % pages;
    let mut shown: Vec<Slot> = slots.into_iter().skip(page * per_page).take(per_page).collect();
    shown.resize(per_page, Slot::Empty);
    shown.push(Slot::Page);
    Bar { slots: shown, columns, page, pages }
}

/// What the unit bar of Physik goes by: the cursor's row up to its last operator, `=` or
/// bracket. While the letters of a unit are typed it stays the same, so the bar does not change
/// under the finger.
pub fn context(before: &[Node], path_len: usize) -> String {
    let cut = before
        .iter()
        .rposition(|node| matches!(node, Node::Char(c) if "+-−=×÷<>≤≥≠(;".contains(*c)))
        .map_or(0, |at| at + 1);
    format!("{path_len}:{}", Row(before[..cut].to_vec()).to_latex())
}

/// Whether `setting` is at `value`: the base set now is lit.
pub fn is_set(settings: &crate::app::Settings, value: i32) -> bool {
    settings.get(Setting::NumberBase) == value
}
