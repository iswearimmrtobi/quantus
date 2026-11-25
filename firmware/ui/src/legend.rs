//! The keys of the keypad by their printed labels (design/keypad-sizes.html): which [`Key`] a
//! label stands for, and what a key does on the SHIFT, ALPHA and UNIT layers
//! (design/handheld-menus.md §7.4.1). The simulator's drawn keypad and the Duo's input screen
//! (firmware/docs/duo.md §3) label their keys with them.

use crate::key::Key;
use crate::layer::Layer;

/// The key a label stands for, if the UI knows it.
pub fn key_of(label: &str) -> Option<Key> {
    Some(match label {
        "SHIFT" => Key::Shift,
        "ALPHA" => Key::Alpha,
        "SYMB" => Key::Symb,
        "UNIT" => Key::Unit,
        "(−)" => Key::Neg,
        "x⁻¹" => Key::Inverse,
        "ENG" => Key::Eng,
        "i" => Key::Imag,
        "UNDO" => Key::Undo,
        "x³" => Key::Cube,
        "10ˣ" => Key::TenPow,
        "Σ" => Key::Sum,
        "∫" => Key::Integral,
        "OPTN" => Key::Optn,
        "MODE" => Key::Mode,
        "OK" => Key::Ok,
        "▲" => Key::Up,
        "▼" => Key::Down,
        "◄" => Key::Left,
        "►" => Key::Right,
        "DEL" => Key::Del,
        "AC" => Key::Ac,
        "EXE" => Key::Exe,
        "CONST" => Key::Const,
        "CONV" => Key::Conv,
        "CALC" => Key::Calc,
        "VARS" => Key::Vars,
        "S⇔D" => Key::SwapForm,
        "a/b" => Key::Fraction,
        "√" => Key::Root,
        "x²" => Key::Square,
        "xʸ" => Key::Power,
        "log" => Key::Log,
        "ln" => Key::Ln,
        "sin" => Key::Sin,
        "cos" => Key::Cos,
        "tan" => Key::Tan,
        "π" => Key::Pi,
        "e" => Key::E,
        "Ans" => Key::Ans,
        "(" => Key::OpenParen,
        ")" => Key::CloseParen,
        "=" => Key::Equals,
        ";" => Key::Semicolon,
        "," => Key::Comma,
        "+" => Key::Plus,
        "−" => Key::Minus,
        "×" => Key::Times,
        "÷" => Key::Divide,
        "x" => Key::Letter('x'),
        "y" => Key::Letter('y'),
        digit if digit.len() == 1 && digit.as_bytes()[0].is_ascii_digit() => Key::Digit(digit.as_bytes()[0] - b'0'),
        _ => return None,
    })
}

/// What a key does after SHIFT (design/handheld-menus.md §7.4.1), and whether the handheld UI
/// carries it out yet; one that it does not is drawn dim.
fn shift_legend(label: &str) -> Option<(&'static str, bool)> {
    Some(match label {
        "a/b" => ("a b/c", true),
        "√" => ("n√", true),
        "x²" => ("x³", true),
        "xʸ" => ("logₐb", true),
        "log" => ("10ˣ", true),
        "ln" => ("eˣ", true),
        "(−)" => ("∠", true),
        "x⁻¹" => ("x!", true),
        "sin" => ("sin⁻¹", true),
        "cos" => ("cos⁻¹", true),
        "tan" => ("tan⁻¹", true),
        "π" => ("°′″", true),
        "(" => ("|x|", true),
        ";" => (":", true),
        "ENG" => ("←ENG", true),
        "x" => ("∫", true),
        "y" => ("d/dx", true),
        "e" => ("Σ", true),
        "i" => ("Π", true),
        "S⇔D" => ("FACT", false),
        "VARS" => ("STO", true),
        "CALC" => ("SOLVE", true),
        "×" => ("nPr", true),
        "÷" => ("nCr", true),
        "+" => ("Pol(", true),
        "−" => ("Rec(", true),
        "," => ("Ran#", true),
        "×10ˣ" => ("0.ẋ", false),
        "Ans" => ("PreAns", true),
        "DEL" => ("UNDO", true),
        "AC" => ("REDO", true),
        "◄" => ("sel<", true),
        "►" => ("sel>", true),
        "▲" => ("copy", true),
        "▼" => ("paste", true),
        "OK" => ("INS", true),
        "EXE" => ("≈", true),
        "OPTN" => ("CATALOG", true),
        "MODE" => ("SETUP", true),
        "7" => ("CAPTURE", true),
        _ => return None,
    })
}

/// What a key types after ALPHA: the variables A–F and M on the digits, and two commands.
fn alpha_legend(label: &str) -> Option<(&'static str, bool)> {
    Some(match label {
        "7" => ("A", true),
        "8" => ("B", true),
        "9" => ("C", true),
        "4" => ("D", true),
        "5" => ("E", true),
        "6" => ("F", true),
        "1" => ("M", true),
        "(" => ("%", true),
        "," => ("RanInt#", true),
        _ => return None,
    })
}

/// What a key types after UNIT: the prefixes on the digits and the units on the function block.
fn unit_legend(label: &str) -> Option<(&'static str, bool)> {
    let prefix = match label {
        "7" => "G",
        "8" => "M",
        "9" => "k",
        "4" => "m",
        "5" => "µ",
        "6" => "n",
        "1" => "p",
        "2" => "c",
        "3" => "d",
        "0" => "T",
        _ => "",
    };
    if !prefix.is_empty() {
        return Some((prefix, true));
    }
    let key = key_of(label)?;
    crate::layer::UNITS.iter().find(|(unit, _)| *unit == key).map(|&(_, symbol)| (symbol, true))
}

/// The legend of a key on a layer, and whether it works.
pub fn legend(label: &str, layer: Layer) -> Option<(&'static str, bool)> {
    match layer {
        Layer::Base => None,
        Layer::Shift => shift_legend(label),
        Layer::Alpha => alpha_legend(label),
        Layer::Unit => unit_legend(label),
    }
}
