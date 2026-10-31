//! The keys of the handheld (design/calculator-hardware.md §4). The simulator and the serial
//! console only map their input to these.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Digit(u8),
    /// A letter typed with ALPHA, or straight from a PC keyboard.
    Letter(char),
    /// The decimal comma.
    Comma,
    Plus,
    Minus,
    Times,
    Divide,
    /// `=`, for equations; EXE evaluates.
    Equals,
    OpenParen,
    CloseParen,
    Fraction,
    Power,
    Square,
    /// The square root; after SHIFT the n-th root.
    Root,
    Subscript,
    Pi,
    E,
    Sin,
    Cos,
    Tan,
    Ln,
    Log,
    Ans,
    Left,
    Right,
    Up,
    Down,
    Ok,
    Del,
    Ac,
    Exe,
    Shift,
    Mode,
}
