//! The input as a tree, edited key by key like the natural display of a Casio, and written out
//! as the LaTeX the engine reads.

/// A thin space, the gap between a number and its unit (`\,`).
pub const THIN_SPACE: char = '\u{2009}';

/// Greek letters and their LaTeX commands, for the editor's output and for reading results.
pub const GREEK: &[(char, &str)] = &[
    ('α', "alpha"), ('β', "beta"), ('γ', "gamma"), ('δ', "delta"), ('ε', "epsilon"),
    ('ε', "varepsilon"), ('ζ', "zeta"), ('η', "eta"), ('θ', "theta"), ('ϑ', "vartheta"),
    ('ι', "iota"), ('κ', "kappa"), ('λ', "lambda"), ('μ', "mu"), ('ν', "nu"), ('ξ', "xi"),
    ('π', "pi"), ('ρ', "rho"), ('σ', "sigma"), ('τ', "tau"), ('υ', "upsilon"), ('φ', "phi"),
    ('φ', "varphi"), ('χ', "chi"), ('ψ', "psi"), ('ω', "omega"), ('Γ', "Gamma"),
    ('Δ', "Delta"), ('Θ', "Theta"), ('Λ', "Lambda"), ('Ξ', "Xi"), ('Π', "Pi"),
    ('Σ', "Sigma"), ('Υ', "Upsilon"), ('Φ', "Phi"), ('Ψ', "Psi"), ('Ω', "Omega"),
];

/// Characters written as a LaTeX command other than a Greek letter.
const SYMBOLS: &[(char, &str)] = &[
    ('×', "cdot"), ('÷', "div"), ('±', "pm"), ('∞', "infty"), ('→', "to"), ('≈', "approx"),
    ('≤', "le"), ('≥', "ge"), ('≠', "neq"), ('ℏ', "hbar"), ('∧', "land"), ('∫', "int"),
    ('∑', "sum"), ('∏', "prod"), ('∨', "lor"), ('∈', "in"), ('∉', "notin"), ('∪', "cup"), ('∖', "setminus"),
    ('⇒', "Rightarrow"), ('↗', "nearrow"), ('↘', "searrow"), ('∅', "emptyset"), ('⇔', "Leftrightarrow"),
    ('⌣', "smile"), ('⌢', "frown"), ('{', "{"),
    ('}', "}"),
];

use crate::key::Key;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Row(pub Vec<Node>);

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    /// A digit, letter, operator or bracket: `×` and `÷` for the operators, `,` the decimal
    /// comma, [`THIN_SPACE`] the gap before a unit.
    Char(char),
    /// A function name, written upright: `sin` for `\sin`.
    Func(String),
    /// Upright text, a unit: `km` for `\mathrm{km}`.
    Upright(String),
    Frac(Row, Row),
    /// An exponent on what comes before it.
    Pow(Row),
    Sub(Row),
    Sqrt(Row),
    /// The index, then the radicand.
    Root(Row, Row),
    /// A mark over what it holds: x̄, ŷ. Only results have it; no key types it.
    Accent(Accent, Row),
}

/// The marks of [`Node::Accent`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Accent {
    /// `\bar`: a mean.
    Bar,
    /// `\hat`: an estimate.
    Hat,
}

impl Node {
    /// How many rows the node holds; the cursor can enter each.
    pub fn slot_count(&self) -> usize {
        match self {
            Node::Char(_) | Node::Func(_) | Node::Upright(_) => 0,
            Node::Pow(_) | Node::Sub(_) | Node::Sqrt(_) | Node::Accent(..) => 1,
            Node::Frac(..) | Node::Root(..) => 2,
        }
    }

    pub fn slot(&self, index: usize) -> &Row {
        match (self, index) {
            (Node::Pow(row) | Node::Sub(row) | Node::Sqrt(row) | Node::Accent(_, row), 0) => row,
            (Node::Frac(first, _) | Node::Root(first, _), 0) => first,
            (Node::Frac(_, second) | Node::Root(_, second), 1) => second,
            _ => panic!("{self:?} has no slot {index}"),
        }
    }

    pub fn slot_mut(&mut self, index: usize) -> &mut Row {
        match (self, index) {
            (Node::Pow(row) | Node::Sub(row) | Node::Sqrt(row) | Node::Accent(_, row), 0) => row,
            (Node::Frac(first, _) | Node::Root(first, _), 0) => first,
            (Node::Frac(_, second) | Node::Root(_, second), 1) => second,
            (node, _) => panic!("{node:?} has no slot {index}"),
        }
    }

    fn into_slots(self) -> Vec<Row> {
        match self {
            Node::Char(_) | Node::Func(_) | Node::Upright(_) => Vec::new(),
            Node::Pow(row) | Node::Sub(row) | Node::Sqrt(row) | Node::Accent(_, row) => vec![row],
            Node::Frac(first, second) | Node::Root(first, second) => vec![first, second],
        }
    }

    /// Whether a fraction typed after it takes it into the numerator: a number, a variable or
    /// what is attached to one.
    fn is_operand(&self) -> bool {
        match self {
            Node::Char(c) => c.is_alphanumeric() || *c == ',',
            Node::Pow(_) | Node::Sub(_) | Node::Upright(_) | Node::Accent(..) => true,
            _ => false,
        }
    }
}

impl Row {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn to_latex(&self) -> String {
        let mut latex = Latex::default();
        latex.row(self);
        latex.out
    }
}

/// The input and a cursor in it.
#[derive(Clone, Debug, Default)]
pub struct Editor {
    pub row: Row,
    /// From the outer row inwards: the index of a node and which of its rows the cursor is in.
    path: Vec<(usize, usize)>,
    /// The cursor's position in the innermost row: before the node with this index.
    pos: usize,
}

impl Editor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn path(&self) -> &[(usize, usize)] {
        &self.path
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn is_empty(&self) -> bool {
        self.row.is_empty()
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Replaces the input, with the cursor at its end.
    pub fn set(&mut self, row: Row) {
        self.pos = row.0.len();
        self.path.clear();
        self.row = row;
    }

    /// Inserts a copy of `row` at the cursor.
    pub fn paste(&mut self, row: &Row) {
        row.0.iter().for_each(|node| self.insert(node.clone()));
    }

    pub fn to_latex(&self) -> String {
        self.row.to_latex()
    }

    fn current(&mut self) -> &mut Row {
        let mut row = &mut self.row;
        for &(index, slot) in &self.path {
            row = row.0[index].slot_mut(slot);
        }
        row
    }

    fn insert(&mut self, node: Node) {
        let pos = self.pos;
        self.current().0.insert(pos, node);
        self.pos += 1;
    }

    /// Inserts a node with rows and puts the cursor at the start of row `slot`.
    fn insert_template(&mut self, node: Node, slot: usize) {
        self.insert(node);
        self.path.push((self.pos - 1, slot));
        self.pos = 0;
    }

    pub fn char(&mut self, c: char) {
        self.insert(Node::Char(c));
    }

    /// `sin(`: the name and the opening bracket.
    pub fn function(&mut self, name: &str) {
        self.insert(Node::Func(name.to_string()));
        self.char('(');
    }

    /// A fraction. A number or variable right before the cursor becomes its numerator and the
    /// cursor goes to the denominator; otherwise both are empty and the cursor in the numerator.
    pub fn fraction(&mut self) {
        let pos = self.pos;
        let row = self.current();
        let start = row.0[..pos].iter().rposition(|node| !node.is_operand()).map_or(0, |i| i + 1);
        let numerator: Vec<Node> = row.0.drain(start..pos).collect();
        self.pos = start;
        let slot = usize::from(!numerator.is_empty());
        self.insert_template(Node::Frac(Row(numerator), Row::default()), slot);
    }

    pub fn power(&mut self) {
        self.insert_template(Node::Pow(Row::default()), 0);
    }

    pub fn square(&mut self) {
        self.insert(Node::Pow(Row(vec![Node::Char('2')])));
    }

    pub fn subscript(&mut self) {
        self.insert_template(Node::Sub(Row::default()), 0);
    }

    pub fn sqrt(&mut self) {
        self.insert_template(Node::Sqrt(Row::default()), 0);
    }

    /// The n-th root, with the cursor in the index.
    pub fn root(&mut self) {
        self.insert_template(Node::Root(Row::default(), Row::default()), 0);
    }

    /// Deletes what is left of the cursor. Right after a fraction, root or exponent the cursor
    /// goes into it instead; at the start of one of its rows the template goes and its content
    /// stays.
    pub fn delete(&mut self) {
        let pos = self.pos;
        if pos > 0 {
            let row = self.current();
            let slots = row.0[pos - 1].slot_count();
            if slots > 0 {
                let len = row.0[pos - 1].slot(slots - 1).0.len();
                self.path.push((pos - 1, slots - 1));
                self.pos = len;
            } else {
                row.0.remove(pos - 1);
                self.pos -= 1;
            }
        } else if let Some((index, slot)) = self.path.pop() {
            let row = self.current();
            let slots = row.0.remove(index).into_slots();
            let before = slots[..slot].iter().map(|row| row.0.len()).sum::<usize>();
            row.0.splice(index..index, slots.into_iter().flat_map(|row| row.0));
            self.pos = index + before;
        }
    }

    /// Returns false at the end of the input.
    pub fn right(&mut self) -> bool {
        let pos = self.pos;
        let row = self.current();
        if pos < row.0.len() {
            if row.0[pos].slot_count() > 0 {
                self.path.push((pos, 0));
                self.pos = 0;
            } else {
                self.pos += 1;
            }
            return true;
        }
        let Some(&(index, slot)) = self.path.last() else { return false };
        self.path.pop();
        if slot + 1 < self.current().0[index].slot_count() {
            self.path.push((index, slot + 1));
            self.pos = 0;
        } else {
            self.pos = index + 1;
        }
        true
    }

    /// Returns false at the start of the input.
    pub fn left(&mut self) -> bool {
        let pos = self.pos;
        let row = self.current();
        if pos > 0 {
            let slots = row.0[pos - 1].slot_count();
            if slots > 0 {
                let len = row.0[pos - 1].slot(slots - 1).0.len();
                self.path.push((pos - 1, slots - 1));
                self.pos = len;
            } else {
                self.pos -= 1;
            }
            return true;
        }
        let Some(&(index, slot)) = self.path.last() else { return false };
        self.path.pop();
        if slot > 0 {
            let len = self.current().0[index].slot(slot - 1).0.len();
            self.path.push((index, slot - 1));
            self.pos = len;
        } else {
            self.pos = index;
        }
        true
    }

    /// From a denominator to its numerator, or from a radicand to its index. Returns false if
    /// the cursor is in neither.
    pub fn up(&mut self) -> bool {
        self.switch_slot(1, 0)
    }

    /// From a numerator to its denominator, or from an index to its radicand.
    pub fn down(&mut self) -> bool {
        self.switch_slot(0, 1)
    }

    /// Moves to row `to` of the innermost fraction or root whose row `from` holds the cursor.
    fn switch_slot(&mut self, from: usize, to: usize) -> bool {
        let mut target = None;
        let mut row = &self.row;
        for (depth, &(index, slot)) in self.path.iter().enumerate() {
            let node = &row.0[index];
            if slot == from && node.slot_count() == 2 {
                target = Some((depth, node.slot(to).0.len()));
            }
            row = node.slot(slot);
        }
        let Some((depth, len)) = target else { return false };
        self.path.truncate(depth + 1);
        self.path[depth].1 = to;
        self.pos = len;
        true
    }
}

impl Editor {
    /// Types what an input key stands for: a character, a template or a function. Returns
    /// false for the other keys (the cursor, DEL, AC, EXE, menus), which the caller handles.
    pub fn key(&mut self, key: Key, shift: bool) -> bool {
        match key {
            Key::Digit(digit) => self.char(char::from(b'0' + digit.min(9))),
            Key::Letter(c) => self.char(c),
            Key::Comma => self.char(','),
            Key::Semicolon => self.char(';'),
            Key::Plus => self.char('+'),
            Key::Minus => self.char('-'),
            Key::Times => self.char('×'),
            Key::Divide => self.char('÷'),
            Key::Equals => self.char('='),
            Key::OpenParen => self.char('('),
            Key::CloseParen => self.char(')'),
            Key::Pi => self.char('π'),
            Key::E => self.char('e'),
            Key::Fraction => self.fraction(),
            Key::Power => self.power(),
            Key::Square => self.square(),
            Key::Root if shift => self.root(),
            Key::Root => self.sqrt(),
            Key::Subscript => self.subscript(),
            Key::Sin => self.function("sin"),
            Key::Cos => self.function("cos"),
            Key::Tan => self.function("tan"),
            Key::Ln => self.function("ln"),
            Key::Log => self.function("log"),
            _ => return false,
        }
        true
    }
}

/// Writes rows as LaTeX, with a space after a command made of letters where the next text
/// would otherwise run into it (`\pi x`).
#[derive(Default)]
struct Latex {
    out: String,
    after_command: bool,
}

impl Latex {
    fn text(&mut self, text: &str) {
        if self.after_command && text.starts_with(|c: char| c.is_ascii_alphabetic()) {
            self.out.push(' ');
        }
        self.out.push_str(text);
        self.after_command = false;
    }

    fn command(&mut self, name: &str) {
        self.text("\\");
        self.out.push_str(name);
        self.after_command = name.starts_with(|c: char| c.is_ascii_alphabetic());
    }

    fn group(&mut self, row: &Row) {
        self.text("{");
        self.row(row);
        self.text("}");
    }

    fn char(&mut self, c: char) {
        match c {
            ',' => self.text("{,}"),
            THIN_SPACE => self.command(","),
            'ℝ' => self.text(r"\mathbb{R}"),
            'ℤ' => self.text(r"\mathbb{Z}"),
            _ => match GREEK.iter().chain(SYMBOLS).find(|(symbol, _)| *symbol == c) {
                Some((_, name)) => self.command(name),
                None => self.text(c.encode_utf8(&mut [0; 4])),
            },
        }
    }

    fn row(&mut self, row: &Row) {
        for node in &row.0 {
            match node {
                Node::Char(c) => self.char(*c),
                Node::Func(name) => self.command(name),
                Node::Upright(text) => {
                    self.command("mathrm");
                    self.text("{");
                    text.chars().for_each(|c| self.char(c));
                    self.text("}");
                }
                Node::Frac(numerator, denominator) => {
                    self.command("frac");
                    self.group(numerator);
                    self.group(denominator);
                }
                Node::Pow(exponent) => {
                    self.text("^");
                    self.group(exponent);
                }
                Node::Sub(index) => {
                    self.text("_");
                    self.group(index);
                }
                Node::Sqrt(radicand) => {
                    self.command("sqrt");
                    self.group(radicand);
                }
                Node::Accent(accent, row) => {
                    self.command(match accent {
                        Accent::Bar => "bar",
                        Accent::Hat => "hat",
                    });
                    self.group(row);
                }
                Node::Root(index, radicand) => {
                    self.command("sqrt");
                    self.text("[");
                    self.row(index);
                    self.text("]");
                    self.group(radicand);
                }
            }
        }
    }
}
