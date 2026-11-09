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
    /// The next key types a unit or a prefix.
    Unit,
    /// (−): the sign of a number; after SHIFT the angle of a complex number.
    Neg,
    /// x⁻¹.
    Inverse,
    /// The imaginary unit.
    Imag,
    /// ENG: the result in engineering notation.
    Eng,
    /// UNDO and REDO of the big unit, and SHIFT on DEL and AC.
    Undo,
    Redo,
    /// SHIFT on OK: insert instead of overtype.
    Ins,
    /// The keys of the big unit that stand for a template of their own: x³, 10ˣ, Σ and ∫.
    Cube,
    TenPow,
    Sum,
    Integral,
    Mode,
    /// The mode's option menu.
    Optn,
    /// CONST: the physical constants by category.
    Const,
    /// CONV: a value converted to a unit, chosen by category.
    Conv,
    /// CALC: the input worked out for values of its letters, asked for in a form; after SHIFT,
    /// SOLVE: solved for the letter left empty.
    Calc,
    /// VARS: the session's variables; after SHIFT, STO: the result kept under a name.
    Vars,
    /// S⇔D: the shown result as a decimal or exact.
    SwapForm,
    /// A character from a keyboard that no key stands for (`@`, `!`, …), and the point, which
    /// the calculator takes as its comma ([`Key::plain`]): only a text field (a name, a
    /// password) takes it as it is.
    Char(char),
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
    ('\u{15}', Key::Const),     // Ctrl+U
    ('\u{19}', Key::Conv),      // Ctrl+Y
    ('\u{02}', Key::Calc),      // Ctrl+B
    ('\u{16}', Key::Vars),      // Ctrl+V
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
            ',' => Key::Comma,
            '.' => Key::Char('.'),
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
            c if !c.is_control() => Key::Char(c),
            _ => return None,
        })
    }

    /// The key outside a text field: the point is the comma, other characters do nothing.
    pub fn plain(self) -> Option<Key> {
        match self {
            Key::Char('.') => Some(Key::Comma),
            Key::Char(_) => None,
            key => Some(key),
        }
    }

    /// The character the key types into a text field, if any.
    pub fn text(self) -> Option<char> {
        Some(match self {
            Key::Letter(c) | Key::Char(c) => c,
            Key::Digit(digit) => char::from(b'0' + digit.min(9)),
            Key::Comma => ',',
            Key::Space => ' ',
            Key::Semicolon => ';',
            Key::Plus => '+',
            Key::Minus => '-',
            Key::Times => '*',
            Key::Divide => ':',
            Key::Equals => '=',
            Key::OpenParen => '(',
            Key::CloseParen => ')',
            Key::Fraction => '/',
            Key::Power => '^',
            Key::Subscript => '_',
            _ => return None,
        })
    }
}
