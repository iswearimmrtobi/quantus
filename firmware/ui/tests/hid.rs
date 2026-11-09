//! A USB keyboard's reports turned into keys (`quantus_ui::hid`).

use quantus_ui::hid::{boot_keyboard, key, BootInterface, Keyboard, Report, REPEAT_DELAY, REPEAT_RATE};
use quantus_ui::key::Key;

const SHIFT: u8 = 0x02;
const CTRL: u8 = 0x01;
const ALT_GR: u8 = 0x40;

fn report(modifiers: u8, keys: &[u8]) -> Report {
    let mut bytes = [0u8; 8];
    bytes[0] = modifiers;
    bytes[2..2 + keys.len()].copy_from_slice(keys);
    Report::parse(&bytes).expect("a report")
}

#[test]
fn letters_and_digits_follow_a_german_layout() {
    assert_eq!(key(0x04, 0), Some(Key::Letter('a')));
    assert_eq!(key(0x04, SHIFT), Some(Key::Letter('A')));
    assert_eq!(key(0x1C, 0), Some(Key::Letter('z')));
    assert_eq!(key(0x1D, 0), Some(Key::Letter('y')));
    assert_eq!(key(0x1E, 0), Some(Key::Digit(1)));
    assert_eq!(key(0x27, 0), Some(Key::Digit(0)));
    assert_eq!(key(0x33, 0), Some(Key::Letter('ö')));
    assert_eq!(key(0x2D, SHIFT), Some(Key::Letter('?')));
}

#[test]
fn shifted_characters_are_those_of_the_simulator() {
    assert_eq!(key(0x24, SHIFT), Some(Key::Fraction)); // Shift+7: /
    assert_eq!(key(0x25, SHIFT), Some(Key::OpenParen));
    assert_eq!(key(0x26, SHIFT), Some(Key::CloseParen));
    assert_eq!(key(0x27, SHIFT), Some(Key::Equals));
    assert_eq!(key(0x30, 0), Some(Key::Plus));
    assert_eq!(key(0x30, SHIFT), Some(Key::Times));
    assert_eq!(key(0x37, SHIFT), Some(Key::Divide));
    assert_eq!(key(0x36, SHIFT), Some(Key::Semicolon));
    assert_eq!(key(0x38, SHIFT), Some(Key::Subscript));
    assert_eq!(key(0x35, 0), Some(Key::Power));
    // No key stands for these: they are for text fields; the point is the comma elsewhere.
    assert_eq!(key(0x1F, SHIFT), Some(Key::Char('"')));
    assert_eq!(key(0x37, 0), Some(Key::Char('.')));
    assert_eq!(Key::Char('.').plain(), Some(Key::Comma));
    assert_eq!(Key::Char('"').plain(), None);
}

#[test]
fn named_keys_and_the_keypad() {
    assert_eq!(key(0x28, 0), Some(Key::Exe));
    assert_eq!(key(0x29, 0), Some(Key::Ac));
    assert_eq!(key(0x4C, 0), Some(Key::Ac));
    assert_eq!(key(0x2A, 0), Some(Key::Del));
    assert_eq!(key(0x2B, 0), Some(Key::Mode));
    assert_eq!(key(0x3A, 0), Some(Key::Shift));
    assert_eq!(key(0x3D, 0), Some(Key::Optn));
    assert_eq!(key(0x50, 0), Some(Key::Left));
    assert_eq!(key(0x52, 0), Some(Key::Up));
    assert_eq!(key(0x59, 0), Some(Key::Digit(1)));
    assert_eq!(key(0x62, 0), Some(Key::Digit(0)));
    assert_eq!(key(0x54, 0), Some(Key::Fraction));
    assert_eq!(key(0x63, 0), Some(Key::Comma));
    assert_eq!(key(0x45, 0), Some(Key::Symb));
}

#[test]
fn shift_and_the_function_keys_are_the_soft_keys() {
    assert_eq!(key(0x3A, 0x02), Some(Key::Soft(0)));
    assert_eq!(key(0x40, 0x20), Some(Key::Soft(6)));
    assert_eq!(key(0x45, 0x02), Some(Key::Soft(11)));
}

#[test]
fn ctrl_and_alt_gr() {
    assert_eq!(key(0x1B, CTRL), Some(Key::Ac)); // Ctrl+X
    assert_eq!(key(0x16, CTRL), Some(Key::Sin)); // Ctrl+S
    assert_eq!(key(0x06, CTRL), None); // Ctrl+C belongs to the serial monitor
    assert_eq!(key(0x14, ALT_GR), Some(Key::Char('@'))); // AltGr+Q
    assert_eq!(key(0x64, ALT_GR), Some(Key::Char('|')));
    assert_eq!(key(0x08, ALT_GR), None); // AltGr+E: €, not ASCII
}

#[test]
fn only_new_keys_are_pressed() {
    let mut keyboard = Keyboard::default();
    assert_eq!(keyboard.report(report(0, &[0x04]), 0), [Key::Letter('a')]);
    assert_eq!(keyboard.report(report(0, &[0x04, 0x05]), 10), [Key::Letter('b')]);
    assert_eq!(keyboard.report(report(0, &[0x05]), 20), []);
    assert_eq!(keyboard.report(report(0, &[]), 30), []);
    assert_eq!(keyboard.report(report(0, &[0x04]), 40), [Key::Letter('a')]);
}

#[test]
fn rollover_reports_are_dropped() {
    assert_eq!(Report::parse(&[0, 0, 1, 1, 1, 1, 1, 1]), None);
    assert_eq!(Report::parse(&[0, 0, 4]), None);
}

#[test]
fn arrows_and_del_repeat_while_held() {
    let mut keyboard = Keyboard::default();
    assert_eq!(keyboard.report(report(0, &[0x4F]), 0), [Key::Right]);
    assert_eq!(keyboard.tick(REPEAT_DELAY - 1), None);
    assert_eq!(keyboard.tick(REPEAT_DELAY), Some(Key::Right));
    assert_eq!(keyboard.tick(REPEAT_DELAY + 1), None);
    assert_eq!(keyboard.tick(REPEAT_DELAY + REPEAT_RATE), Some(Key::Right));
    keyboard.report(report(0, &[]), 600);
    assert_eq!(keyboard.tick(10_000), None);

    // Digits and EXE do not repeat.
    keyboard.report(report(0, &[0x1E]), 0);
    assert_eq!(keyboard.tick(10_000), None);
}

#[test]
fn finds_the_boot_keyboard_interface() {
    #[rustfmt::skip]
    let config = [
        9, 2, 59, 0, 2, 1, 0, 0xA0, 50,     // configuration
        9, 4, 0, 0, 1, 3, 0, 0, 0,          // interface 0: HID, no boot protocol
        9, 0x21, 0x11, 1, 0, 1, 0x22, 50, 0, // HID
        7, 5, 0x82, 3, 16, 0, 10,           // endpoint 2 IN
        9, 4, 1, 0, 1, 3, 1, 1, 0,          // interface 1: boot keyboard
        9, 0x21, 0x11, 1, 0, 1, 0x22, 63, 0, // HID
        7, 5, 0x81, 3, 8, 0, 10,            // endpoint 1 IN, 8 bytes
    ];
    assert_eq!(
        boot_keyboard(&config),
        Some(BootInterface { number: 1, alternate: 0, endpoint: 0x81, max_packet: 8 })
    );
    assert_eq!(boot_keyboard(&config[..50]), None);
}
