//! CALC: a form over the input asks for its letters, EXE works it out into the history.

use quantus_ui::app::{App, Mode, Screen};
use quantus_ui::backend::Stub;
use quantus_ui::key::Key;
use quantus_ui::sheet::Kind;

fn press(app: &mut App<Stub>, keys: &[Key]) {
    keys.iter().for_each(|&key| app.key(key));
}

fn algebra() -> App<Stub> {
    let mut app = App::new(Stub);
    app.mode = Mode::Algebra;
    app
}

fn result(app: &App<Stub>) -> Result<String, String> {
    app.shown_entry().expect("a result is shown").result.clone().map(|row| row.to_latex())
}

#[test]
fn calc_asks_for_each_letter_and_works_the_input_out() {
    let mut app = algebra();
    press(&mut app, &[Key::Digit(2), Key::Letter('x'), Key::Plus, Key::Letter('y'), Key::Calc]);
    assert_eq!(app.sheet_kind(), Some(Kind::Formula));
    let asked = app.sheets.asked.as_ref().expect("the form is open");
    assert_eq!(asked.variables.iter().map(|v| v.key.as_str()).collect::<Vec<_>>(), ["x", "y"]);
    // x = 3, y = 1; EXE on the last field works it out.
    press(&mut app, &[Key::Digit(3), Key::Exe, Key::Digit(1), Key::Exe, Key::Exe]);
    assert_eq!(app.sheet_kind(), None, "the form is closed");
    assert_eq!(result(&app), Ok("2x+y;x=3;y=1".to_string()));
    assert_eq!(app.editor.to_latex(), "2x+y", "the input stays");
    assert_eq!(app.history.len(), 1);
}

#[test]
fn calc_without_letters_works_the_input_out_at_once() {
    let mut app = algebra();
    press(&mut app, &[Key::Digit(2), Key::Plus, Key::Digit(3), Key::Calc]);
    assert_eq!(app.sheet_kind(), None);
    assert_eq!(result(&app), Ok("2+3".to_string()));
}

#[test]
fn ac_leaves_the_form_and_keeps_the_input() {
    let mut app = algebra();
    press(&mut app, &[Key::Letter('a'), Key::Calc, Key::Ac]);
    assert_eq!(app.sheet_kind(), None);
    assert_eq!(app.editor.to_latex(), "a");
    assert!(app.history.is_empty());
}

#[test]
fn optn_has_calc_under_werkzeuge() {
    let mut app = algebra();
    press(&mut app, &[Key::Letter('a'), Key::Optn]);
    // Werkzeuge is the last group of Algebra's OPTN, CALC its first entry.
    press(&mut app, &[Key::Up, Key::Exe, Key::Exe]);
    assert_eq!(app.screen, Screen::Calc);
    assert_eq!(app.sheet_kind(), Some(Kind::Formula));
}

#[test]
fn solve_is_for_the_letter_left_empty() {
    let mut app = algebra();
    // F = m a, SHIFT + CALC: F = 10, m = 2, a left empty.
    press(&mut app, &[Key::Letter('F'), Key::Equals, Key::Letter('m'), Key::Letter('a'), Key::Shift, Key::Calc]);
    assert!(app.sheets.asked.as_ref().is_some_and(|asked| asked.solve));
    press(&mut app, &[Key::Digit(1), Key::Digit(0), Key::Exe, Key::Digit(2), Key::Exe, Key::Exe]);
    assert_eq!(result(&app), Ok("a=?".to_string()));
}

#[test]
fn solve_with_every_letter_given_starts_from_the_one_at_the_cursor() {
    let mut app = algebra();
    press(&mut app, &[Key::Letter('x'), Key::Power, Key::Digit(2), Key::Right, Key::Equals, Key::Digit(2), Key::Shift, Key::Calc]);
    press(&mut app, &[Key::Digit(1), Key::Exe, Key::Exe]);
    assert_eq!(result(&app), Ok("x=1".to_string()));
}

#[test]
fn solve_without_letters_says_so() {
    let mut app = algebra();
    press(&mut app, &[Key::Digit(2), Key::Shift, Key::Calc]);
    assert_eq!(app.sheet_kind(), None);
    assert_eq!(result(&app), Err(quantus_ui::text::NO_UNKNOWN.de.to_string()));
}
