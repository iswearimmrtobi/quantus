//! The modifier layers of the handheld (design/handheld-menus.md §7.4): SHIFT, ALPHA and UNIT,
//! how they latch and lock, and what each key stands for on each layer. The simulator, a USB
//! keyboard and the serial console all go through this, so they type the same.

use crate::key::Key;

/// The legend row of the keys that is in force.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Layer {
    #[default]
    Base,
    Shift,
    Alpha,
    Unit,
}

/// The waiting modifiers: one for the next key, and a lock of ALPHA or UNIT below it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifier {
    /// For the next key. UNIT stays after a prefix or an operator (see [`Modifier::settle`]).
    pub pending: Option<Layer>,
    /// ALPHA or UNIT locked with SHIFT; a one-shot modifier on top returns to it.
    pub lock: Option<Layer>,
    /// After `xʸ` in UNIT: the next digit is an exponent, not a prefix.
    pub exponent: bool,
}

impl Modifier {
    /// The layer the next key is read on.
    pub fn layer(&self) -> Layer {
        self.pending.or(self.lock).unwrap_or(Layer::Base)
    }

    /// Whether `key` is one of the three modifier keys.
    pub fn is_modifier(key: Key) -> bool {
        matches!(key, Key::Shift | Key::Alpha | Key::Unit)
    }

    /// A modifier key goes down. Another modifier replaces the pending one, the same one cancels
    /// it, and ALPHA or UNIT after SHIFT locks (or, on the lock, unlocks).
    pub fn press(&mut self, key: Key) {
        let layer = match key {
            Key::Shift => Layer::Shift,
            Key::Alpha => Layer::Alpha,
            Key::Unit => Layer::Unit,
            _ => return,
        };
        self.exponent = false;
        match self.pending {
            Some(Layer::Shift) if layer != Layer::Shift => {
                self.pending = None;
                self.lock = if self.lock == Some(layer) { None } else { Some(layer) };
            }
            Some(pending) if pending == layer => self.pending = None,
            None if self.lock == Some(layer) => self.lock = None,
            _ => self.pending = Some(layer),
        }
    }

    /// A key was typed on `layer`: SHIFT and ALPHA are used up, UNIT stays while prefixes and
    /// units, prefixes and operators are typed and ends with any other key.
    pub fn settle(&mut self, key: Key, layer: Layer) {
        let exponent = std::mem::take(&mut self.exponent);
        if layer != Layer::Unit {
            self.pending = None;
            return;
        }
        if key == Key::Power {
            self.exponent = true;
        }
        let stays = match key {
            Key::Digit(_) => !exponent,
            key => passes_through(key) || unit(key).is_some(),
        };
        if !stays {
            self.pending = None;
        }
    }
}

/// The keys that type as they are in UNIT and keep it latched.
fn passes_through(key: Key) -> bool {
    matches!(
        key,
        Key::Times | Key::Divide | Key::Square | Key::Power | Key::OpenParen | Key::CloseParen | Key::Equals
    )
}

/// What a key does on a layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolved {
    /// Handle this key as if it had been pressed on the base layer.
    Key(Key),
    /// Insert this LaTeX at the cursor (what an OPTN entry does).
    Insert(&'static str),
    /// A legend the handheld does not carry out yet.
    Nothing,
}

/// What `key` stands for on `layer`. A key with no legend there does what it does on the base
/// layer. `exponent` is set right after `xʸ` in UNIT; `base_n` makes ALPHA type hex digits.
pub fn resolve(key: Key, layer: Layer, exponent: bool, base_n: bool) -> Resolved {
    let legend = match layer {
        Layer::Base => None,
        Layer::Shift => shift(key),
        Layer::Alpha => alpha(key, base_n),
        Layer::Unit if exponent && matches!(key, Key::Digit(_)) => None,
        Layer::Unit => unit(key),
    };
    legend.unwrap_or(match base(key) {
        Some(latex) => Resolved::Insert(latex),
        None => Resolved::Key(key),
    })
}

/// What SHIFT y inserts; typed after a function, the engine takes the derivative of it.
pub const DERIVATIVE: &str = r"\frac{d}{dx}";

/// Keys of the big unit that only insert something.
fn base(key: Key) -> Option<&'static str> {
    Some(match key {
        Key::Cube => "^{3}",
        Key::TenPow => "10^{}",
        Key::Sum => r"\sum_{}^{}",
        Key::Integral => r"\int_{}^{}",
        Key::Inverse => "^{-1}",
        Key::Imag => "i",
        _ => return None,
    })
}

fn shift(key: Key) -> Option<Resolved> {
    use Resolved::{Insert, Key as K, Nothing};
    Some(match key {
        Key::Square => Insert("^{3}"),
        Key::Power => Insert(r"\log_{}()"),
        Key::Log => Insert("10^{}"),
        Key::Ln => Insert("e^{}"),
        Key::Neg => Insert("∠"),
        Key::Inverse => Insert("!"),
        Key::Sin => Insert(r"\arcsin()"),
        Key::Cos => Insert(r"\arccos()"),
        Key::Tan => Insert(r"\arctan()"),
        Key::Pi => Insert("°"),
        Key::OpenParen => Insert("||"),
        Key::Semicolon => Insert(":"),
        Key::Letter('x') => Insert(r"\int_{}^{}"),
        Key::Letter('y') => Insert(DERIVATIVE),
        Key::E => Insert(r"\sum_{}^{}"),
        Key::Imag => Insert(r"\prod_{}^{}"),
        Key::Times => Insert(r"\operatorname{nPr}(;)"),
        Key::Divide => Insert(r"\binom{}{}"),
        Key::Plus => Insert(r"\operatorname{Pol}(;)"),
        Key::Minus => Insert(r"\operatorname{Rec}(;)"),
        Key::Comma => Insert(r"\operatorname{Ran}()"),
        Key::Ans => K(Key::PreAns),
        Key::Del => K(Key::Undo),
        Key::Ac => K(Key::Redo),
        Key::Ok => K(Key::Ins),
        Key::Fraction => K(Key::Mixed),
        Key::Left => K(Key::SelectLeft),
        Key::Right => K(Key::SelectRight),
        Key::Up => K(Key::Copy),
        Key::Down => K(Key::Paste),
        Key::Exe => K(Key::Approx),
        Key::Optn => K(Key::Catalog),
        Key::Eng => K(Key::EngBack),
        // FACT and recurring decimals wait for the engine.
        Key::SwapForm => Nothing,
        _ => return None,
    })
}

fn alpha(key: Key, base_n: bool) -> Option<Resolved> {
    use Resolved::{Insert, Key as K};
    let _ = base_n; // A–F are hex digits in Basis-N and variables elsewhere: the same letters.
    Some(match key {
        Key::Digit(7) => K(Key::Letter('A')),
        Key::Digit(8) => K(Key::Letter('B')),
        Key::Digit(9) => K(Key::Letter('C')),
        Key::Digit(4) => K(Key::Letter('D')),
        Key::Digit(5) => K(Key::Letter('E')),
        Key::Digit(6) => K(Key::Letter('F')),
        Key::Digit(1) => K(Key::Letter('M')),
        Key::OpenParen => Insert(r"\%"),
        Key::Comma => Insert(r"\operatorname{RanInt}(;)"),
        _ => return None,
    })
}

/// The unit symbols on the function block, in reading order, and the prefixes on the digits.
fn unit(key: Key) -> Option<Resolved> {
    use Resolved::Insert;
    Some(Insert(match key {
        Key::Digit(7) => "G",
        Key::Digit(8) => "M",
        Key::Digit(9) => "k",
        Key::Digit(4) => "m",
        Key::Digit(5) => r"\mu ",
        Key::Digit(6) => "n",
        Key::Digit(1) => "p",
        Key::Digit(2) => "c",
        Key::Digit(3) => "d",
        Key::Digit(0) => "T",
        _ => UNITS.iter().find(|(k, _)| *k == key)?.1,
    }))
}

/// The units of the function block's 18 unit keys, row by row (firmware/docs/unit-keys.md).
pub const UNITS: [(Key, &str); 18] = [
    (Key::Fraction, "m"),
    (Key::Root, "s"),
    (Key::Log, "g"),
    (Key::Ln, "A"),
    (Key::Neg, "K"),
    (Key::Inverse, "mol"),
    (Key::Sin, "N"),
    (Key::Cos, "J"),
    (Key::Tan, "W"),
    (Key::Pi, "Pa"),
    (Key::Const, "Hz"),
    (Key::Eng, "C"),
    (Key::Letter('x'), "V"),
    (Key::Letter('y'), "Ω"),
    (Key::E, "F"),
    (Key::Imag, "T"),
    (Key::Conv, "H"),
    (Key::SwapForm, "eV"),
];
