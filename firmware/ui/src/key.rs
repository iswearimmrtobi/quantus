//! The keys of the handheld (firmware/docs/hardware.md §4). The simulator and the serial
//! console only map their input to these.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Digit(u8),
    /// A letter typed with ALPHA, or straight from a PC keyboard.
    Letter(char),
    /// The decimal comma.
    Comma,
    /// A space, in a name (a PC or USB keyboard's space bar).
    Space,
    /// Between the equations of a system, and between arguments.
    Semicolon,
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
    /// The next key types its letter.
    Alpha,
    Mode,
    /// The mode's option menu.
    Optn,
    /// S⇔D: the shown result as a decimal or exact.
    SwapForm,
}

/// Control characters for the keys that have no character of their own: Ctrl and a letter in a
/// terminal. Ctrl+C and Ctrl+R are left out, the serial monitor uses them.
pub const CONTROLS: &[(char, Key)] = &[
    ('\u{01}', Key::Ans),       // Ctrl+A
    ('\u{04}', Key::SwapForm),  // Ctrl+D
    ('\u{05}', Key::E),         // Ctrl+E
    ('\u{06}', Key::Shift),     // Ctrl+F
    ('\u{07}', Key::Log),       // Ctrl+G
    ('\u{0b}', Key::Alpha),     // Ctrl+K
    ('\u{0c}', Key::Ln),        // Ctrl+L
    ('\u{0e}', Key::Optn),      // Ctrl+N
    ('\u{0f}', Key::Cos),       // Ctrl+O
    ('\u{10}', Key::Pi),        // Ctrl+P
    ('\u{11}', Key::Square),    // Ctrl+Q
    ('\u{13}', Key::Sin),       // Ctrl+S
    ('\u{14}', Key::Tan),       // Ctrl+T
    ('\u{17}', Key::Root),      // Ctrl+W
    ('\u{18}', Key::Ac),        // Ctrl+X
];

impl Key {
    /// The key a typed character stands for, the same in the simulator and over serial.
    pub fn from_char(c: char) -> Option<Key> {
        if let Some(&(_, key)) = CONTROLS.iter().find(|(control, _)| *control == c) {
            return Some(key);
        }
        Some(match c {
            '0'..='9' => Key::Digit(c as u8 - b'0'),
            ',' | '.' => Key::Comma,
            ' ' => Key::Space,
            ';' => Key::Semicolon,
            '+' => Key::Plus,
            '-' => Key::Minus,
            '*' => Key::Times,
            ':' => Key::Divide,
            '/' => Key::Fraction,
            '^' => Key::Power,
            '_' => Key::Subscript,
            '=' => Key::Equals,
            '(' => Key::OpenParen,
            ')' => Key::CloseParen,
            '\r' | '\n' => Key::Exe,
            '\u{08}' | '\u{7f}' => Key::Del,
            '\t' => Key::Mode,
            c if c.is_alphabetic() || c == '?' => Key::Letter(c),
            _ => return None,
        })
    }
}
