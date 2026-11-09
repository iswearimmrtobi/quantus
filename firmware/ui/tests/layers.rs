//! SHIFT, ALPHA and UNIT: latch, lock and the legends (design/handheld-menus.md §7.4.1).

use quantus_ui::app::{App, Mode};
use quantus_ui::backend::Stub;
use quantus_ui::key::Key;
use quantus_ui::layer::Layer;

fn press(app: &mut App<Stub>, keys: &[Key]) {
    keys.iter().for_each(|&key| app.key(key));
}

fn typed(keys: &[Key]) -> String {
    let mut app = App::new(Stub);
    app.mode = Mode::Physics;
    press(&mut app, keys);
    app.editor.to_latex()
}

#[test]
fn a_modifier_serves_one_key() {
    assert_eq!(typed(&[Key::Shift, Key::Square, Key::Square]), "^{3}^{2}");
    assert_eq!(typed(&[Key::Alpha, Key::Digit(7), Key::Digit(7)]), "A7");
}

#[test]
fn the_same_modifier_cancels_and_another_replaces() {
    assert_eq!(typed(&[Key::Alpha, Key::Alpha, Key::Digit(7)]), "7");
    assert_eq!(typed(&[Key::Shift, Key::Alpha, Key::Digit(7)]), "A", "SHIFT ALPHA locks");
}

#[test]
fn shift_then_alpha_locks_and_shift_then_alpha_again_unlocks() {
    assert_eq!(typed(&[Key::Shift, Key::Alpha, Key::Digit(7), Key::Digit(8)]), "AB");
    assert_eq!(typed(&[Key::Shift, Key::Alpha, Key::Digit(7), Key::Shift, Key::Alpha, Key::Digit(8)]), "A8");
    assert_eq!(typed(&[Key::Shift, Key::Alpha, Key::Alpha, Key::Digit(8)]), "8");
}

#[test]
fn shift_is_one_shot_on_top_of_a_lock() {
    let mut app = App::new(Stub);
    app.mode = Mode::Physics;
    press(&mut app, &[Key::Shift, Key::Unit, Key::Shift, Key::Square]);
    assert_eq!(app.modifier.layer(), Layer::Unit);
    assert_eq!(app.editor.to_latex(), "^{3}");
}

#[test]
fn unit_types_units_and_prefixes() {
    assert_eq!(typed(&[Key::Unit, Key::Digit(9), Key::Fraction]), "km");
    assert_eq!(typed(&[Key::Unit, Key::Fraction, Key::Plus, Key::Digit(1)]), "m+1", "a plain key ends it");
}

#[test]
fn operators_keep_unit_latched_and_an_exponent_ends_it() {
    // UNIT m ÷ s x²
    assert_eq!(typed(&[Key::Unit, Key::Fraction, Key::Divide, Key::Root, Key::Square]), "m\\div s^{2}");
    // UNIT m xʸ 2 then 5: the 2 is an exponent, the latch ends after it.
    assert_eq!(typed(&[Key::Unit, Key::Fraction, Key::Power, Key::Digit(2), Key::Right, Key::Digit(5)]), "m^{2}5");
}

#[test]
fn a_locked_unit_stays_until_unlocked() {
    assert_eq!(typed(&[Key::Shift, Key::Unit, Key::Digit(9), Key::Fraction, Key::Digit(4), Key::Root]), "kmms");
    assert_eq!(typed(&[Key::Shift, Key::Unit, Key::Unit, Key::Digit(4)]), "4");
}

#[test]
fn alpha_has_its_digits() {
    assert_eq!(typed(&[Key::Alpha, Key::Digit(4), Key::Alpha, Key::Digit(1)]), "DM");
    assert_eq!(typed(&[Key::Alpha, Key::Digit(2)]), "2", "no legend: the base key");
}

#[test]
fn shift_ok_overtypes() {
    assert_eq!(typed(&[Key::Digit(1), Key::Digit(2), Key::Left, Key::Left, Key::Shift, Key::Ok, Key::Digit(9), Key::Digit(8)]), "98");
    assert_eq!(typed(&[Key::Digit(1), Key::Digit(2), Key::Left, Key::Digit(9)]), "192");
}

#[test]
fn a_mixed_number_takes_the_number_before_it() {
    assert_eq!(typed(&[Key::Digit(2), Key::Shift, Key::Fraction, Key::Digit(1), Key::Down, Key::Digit(3)]), r"(2+\frac{1}{3})");
}

#[test]
fn undo_and_redo_step_through_the_changes() {
    let keys = [Key::Digit(1), Key::Digit(2), Key::Left, Key::Plus];
    assert_eq!(typed(&keys), "1+2");
    let mut with = keys.to_vec();
    with.extend([Key::Shift, Key::Del]);
    assert_eq!(typed(&with), "12", "undo");
    with.extend([Key::Shift, Key::Del]);
    assert_eq!(typed(&with), "1");
    with.extend([Key::Shift, Key::Ac]);
    assert_eq!(typed(&with), "12", "redo");
    with.extend([Key::Shift, Key::Ac, Key::Shift, Key::Ac]);
    assert_eq!(typed(&with), "1+2", "no more to redo");
}

#[test]
fn select_copy_and_paste() {
    // 123, select the last two, copy, go to the end, paste: 12323.
    let keys = [Key::Digit(1), Key::Digit(2), Key::Digit(3), Key::Shift, Key::Left, Key::Shift, Key::Left, Key::Shift, Key::Up];
    let mut with = keys.to_vec();
    with.extend([Key::Right, Key::Right, Key::Shift, Key::Down]);
    assert_eq!(typed(&with), "12323");
    // Typing over a selection replaces it.
    let mut over = keys[..7].to_vec();
    over.push(Key::Digit(9));
    assert_eq!(typed(&over), "19");
    // DEL deletes it.
    let mut del = keys[..7].to_vec();
    del.push(Key::Del);
    assert_eq!(typed(&del), "1");
}

#[test]
fn shift_exe_is_a_decimal() {
    let mut app = App::new(Stub);
    app.mode = Mode::Physics;
    press(&mut app, &[Key::Digit(1), Key::Shift, Key::Exe]);
    assert!(app.shown_entry().is_some_and(|entry| entry.decimal));
}

#[test]
fn catalog_lists_the_optn_functions_by_name_and_inserts() {
    use quantus_ui::app::Screen;
    let mut app = App::new(Stub);
    app.mode = Mode::Algebra;
    press(&mut app, &[Key::Shift, Key::Optn]);
    assert!(matches!(app.screen, Screen::Menu(_)));
    // The list is alphabetical.
    let items = quantus_ui::tree::functions(Mode::Algebra, quantus_ui::text::Lang::En);
    assert!(items.windows(2).all(|pair| pair[0].name.to_lowercase() <= pair[1].name.to_lowercase()));
    assert!(items.iter().any(|item| item.name == "gcd" || item.name.to_lowercase().contains("gcd")));
    // OK inserts the first one and leaves the menu.
    press(&mut app, &[Key::Ok]);
    assert!(matches!(app.screen, Screen::Calc));
    assert!(!app.editor.is_empty());
}
