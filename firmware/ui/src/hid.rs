//! A USB keyboard on the handheld: its boot-protocol reports turned into keys, with a German
//! (QWERTZ) layout and the simulator's key table (`firmware/sim/README.md#keys`). The USB side
//! is in `firmware/handheld/src/keyboard.rs`; this part needs no hardware and is tested on the PC.

use crate::key::Key;

/// Left and right Ctrl in the report's modifier byte.
const CTRL: u8 = 0x01 | 0x10;
/// Left and right Shift.
const SHIFT: u8 = 0x02 | 0x20;
/// The right Alt, AltGr on a German keyboard.
const ALT_GR: u8 = 0x40;

/// How long a key is held before it repeats, and how often it then repeats, in ms.
pub const REPEAT_DELAY: u64 = 500;
pub const REPEAT_RATE: u64 = 50;

/// The key for a pressed key's usage ID (HID usage table, page 7) with the report's modifiers.
pub fn key(usage: u8, modifiers: u8) -> Option<Key> {
    let named = match usage {
        0x4F => Some(Key::Right),
        0x50 => Some(Key::Left),
        0x51 => Some(Key::Down),
        0x52 => Some(Key::Up),
        // Esc, Delete
        0x29 | 0x4C => Some(Key::Ac),
        // Enter, keypad Enter
        0x28 | 0x58 => Some(Key::Exe),
        0x2A => Some(Key::Del),
        0x2B => Some(Key::Mode),
        0x3A => Some(Key::Shift),
        0x3B => Some(Key::Alpha),
        0x3C => Some(Key::SwapForm),
        0x3D => Some(Key::Optn),
        0x3E => Some(Key::Ok),
        0x3F => Some(Key::Const),
        0x40 => Some(Key::Conv),
        0x41 => Some(Key::Calc),
        0x54 => Some(Key::Fraction),
        0x55 => Some(Key::Times),
        0x56 => Some(Key::Minus),
        0x57 => Some(Key::Plus),
        0x59..=0x61 => Some(Key::Digit(usage - 0x58)),
        0x62 => Some(Key::Digit(0)),
        // Keypad comma and point
        0x63 | 0x85 => Some(Key::Comma),
        _ => None,
    };
    if named.is_some() {
        return named;
    }
    // AltGr types @, braces and the like, none of them a key: for text fields.
    if modifiers & ALT_GR != 0 {
        return alt_gr(usage).map(Key::Char);
    }
    let c = character(usage, modifiers & SHIFT != 0)?;
    let c = if modifiers & CTRL != 0 && c.is_ascii_alphabetic() {
        char::from(c.to_ascii_lowercase() as u8 - b'a' + 1)
    } else {
        c
    };
    Key::from_char(c)
}

/// The character a key types on a German keyboard.
fn character(usage: u8, shift: bool) -> Option<char> {
    const SHIFTED_DIGITS: [char; 10] = ['!', '"', '§', '$', '%', '&', '/', '(', ')', '='];
    let (plain, shifted) = match usage {
        0x04..=0x1D => {
            // Y and Z swap places.
            let c = match usage {
                0x1C => 'z',
                0x1D => 'y',
                _ => char::from(b'a' + usage - 0x04),
            };
            (c, c.to_ascii_uppercase())
        }
        0x1E..=0x26 => (char::from(b'1' + usage - 0x1E), SHIFTED_DIGITS[usize::from(usage - 0x1E)]),
        0x27 => ('0', '='),
        0x2C => (' ', ' '),
        0x2D => ('ß', '?'),
        0x2F => ('ü', 'Ü'),
        0x30 => ('+', '*'),
        // The key left of Enter: US backslash, ISO non-US #.
        0x31 | 0x32 => ('#', '\''),
        0x33 => ('ö', 'Ö'),
        0x34 => ('ä', 'Ä'),
        0x35 => ('^', '°'),
        0x36 => (',', ';'),
        0x37 => ('.', ':'),
        0x38 => ('-', '_'),
        0x64 => ('<', '>'),
        _ => return None,
    };
    Some(if shift { shifted } else { plain })
}

/// The character AltGr and a key type on a German keyboard, if it is ASCII.
fn alt_gr(usage: u8) -> Option<char> {
    Some(match usage {
        0x14 => '@',
        0x24 => '{',
        0x25 => '[',
        0x26 => ']',
        0x27 => '}',
        0x2D => '\\',
        0x30 => '~',
        0x64 => '|',
        _ => return None,
    })
}

/// The keys that repeat while held, as on a calculator: the arrows and DEL.
fn repeats(key: Key) -> bool {
    matches!(key, Key::Left | Key::Right | Key::Up | Key::Down | Key::Del)
}

/// A boot-protocol keyboard report: the modifiers and up to six pressed keys.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub modifiers: u8,
    pub keys: [u8; 6],
}

impl Report {
    /// The report in the first eight bytes, or `None` for a short one or one that only says
    /// too many keys are down (usages 1 to 3), which tells nothing about which.
    pub fn parse(bytes: &[u8]) -> Option<Report> {
        let bytes = bytes.get(..8)?;
        let mut keys = [0; 6];
        keys.copy_from_slice(&bytes[2..8]);
        if keys.iter().any(|usage| (1..=3).contains(usage)) {
            return None;
        }
        Some(Report { modifiers: bytes[0], keys })
    }

    /// The usages down in this report that were not in `previous`.
    pub fn pressed<'a>(&'a self, previous: &'a Report) -> impl Iterator<Item = u8> + 'a {
        self.keys.iter().copied().filter(move |&usage| usage != 0 && !previous.keys.contains(&usage))
    }
}

/// The keyboard's state between reports: what was down, and the key that repeats.
#[derive(Debug, Default)]
pub struct Keyboard {
    previous: Report,
    /// The held usage and when it next repeats (ms).
    held: Option<(u8, u64)>,
}

impl Keyboard {
    /// The keys a new report presses, at `now` (ms).
    pub fn report(&mut self, report: Report, now: u64) -> Vec<Key> {
        let mut keys = Vec::new();
        for usage in report.pressed(&self.previous) {
            if let Some(key) = key(usage, report.modifiers) {
                keys.push(key);
                self.held = repeats(key).then_some((usage, now + REPEAT_DELAY));
            }
        }
        if let Some((usage, _)) = self.held {
            if !report.keys.contains(&usage) {
                self.held = None;
            }
        }
        self.previous = report;
        keys
    }

    /// The held key once its time to repeat has come.
    pub fn tick(&mut self, now: u64) -> Option<Key> {
        let (usage, next) = self.held.as_mut()?;
        if now < *next {
            return None;
        }
        *next = now + REPEAT_RATE;
        key(*usage, self.previous.modifiers)
    }

    /// Forgets what was down, when the keyboard goes away.
    pub fn reset(&mut self) {
        *self = Keyboard::default();
    }
}

/// The interface of a boot keyboard and its interrupt IN endpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BootInterface {
    pub number: u8,
    pub alternate: u8,
    pub endpoint: u8,
    pub max_packet: u16,
}

/// The first boot keyboard interface in a configuration descriptor, with all that follows it.
pub fn boot_keyboard(config: &[u8]) -> Option<BootInterface> {
    const INTERFACE: u8 = 4;
    const ENDPOINT: u8 = 5;
    let mut found: Option<(u8, u8)> = None;
    let mut rest = config;
    while let [length, kind, ..] = *rest {
        let length = usize::from(length);
        if length < 2 || length > rest.len() {
            break;
        }
        let descriptor = &rest[..length];
        match kind {
            // Class HID, subclass boot, protocol keyboard.
            INTERFACE if length >= 9 => {
                found = (descriptor[5..8] == [3, 1, 1]).then_some((descriptor[2], descriptor[3]));
            }
            ENDPOINT if length >= 7 => {
                let (address, attributes) = (descriptor[2], descriptor[3]);
                if let Some((number, alternate)) = found {
                    if address & 0x80 != 0 && attributes & 0x03 == 0x03 {
                        let max_packet = u16::from_le_bytes([descriptor[4], descriptor[5]]) & 0x7FF;
                        return Some(BootInterface { number, alternate, endpoint: address, max_packet });
                    }
                }
            }
            _ => {}
        }
        rest = &rest[length..];
    }
    None
}
