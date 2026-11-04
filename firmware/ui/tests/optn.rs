//! OPTN's entries insert what the engine reads, at the cursor.

use quantus_ui::app::{App, Mode, Screen};
use quantus_ui::backend::Stub;
use quantus_ui::key::Key;
use quantus_ui::text::Lang;

/// Opens OPTN and goes down the entries labelled `path` (German), choosing the last.
fn optn(app: &mut App<Stub>, path: &[&str]) {
    app.key(Key::Optn);
    for label in path {
        let Screen::Menu(menu) = &app.screen else { panic!("OPTN closed before {label:?}") };
        let page = menu.level().page;
        let index = page
            .entries
            .iter()
            .position(|entry| entry.label.get(Lang::De) == *label)
            .unwrap_or_else(|| panic!("no {label:?} on {:?}", page.title.get(Lang::De)));
        let selected = menu.level().selected;
        let steps = (index + page.entries.len() - selected) % page.entries.len();
        (0..steps).for_each(|_| app.key(Key::Down));
        app.key(Key::Exe);
    }
}

fn app(mode: Mode) -> App<Stub> {
    let mut app = App::new(Stub);
    app.mode = mode;
    app
}

/// What `path` inserts into an empty input in Physik.
fn inserted(path: &[&str]) -> String {
    let mut app = app(Mode::Physics);
    optn(&mut app, path);
    assert_eq!(app.screen, Screen::Calc, "{path:?} closes OPTN");
    app.editor.to_latex()
}

#[test]
fn a_function_of_several_arguments_starts_at_the_first() {
    let mut app = app(Mode::Physics);
    optn(&mut app, &["Zahl", "max"]);
    app.key(Key::Digit(2));
    assert_eq!(app.editor.to_latex(), r"\max(2;)");
}

#[test]
fn an_angle_in_degrees_minutes_and_seconds() {
    let mut app = app(Mode::Physics);
    app.key(Key::Digit(2));
    optn(&mut app, &["Winkel", "° Grad"]);
    app.key(Key::Digit(3));
    optn(&mut app, &["Winkel", "' Minuten"]);
    app.key(Key::Digit(4));
    optn(&mut app, &["Winkel", "'' Sekunden"]);
    assert_eq!(app.editor.to_latex(), "2°3'4''");
}

#[test]
fn ncr_starts_at_n() {
    let mut app = app(Mode::Physics);
    optn(&mut app, &["Zahl", "nCr"]);
    app.key(Key::Digit(5));
    app.key(Key::Down);
    app.key(Key::Digit(2));
    assert_eq!(app.editor.to_latex(), r"\binom{5}{2}");
}

#[test]
fn permutations_and_the_divisors_take_two_numbers() {
    assert_eq!(inserted(&["Zahl", "nPr"]), r"\mathrm{nPr}(;)");
    assert_eq!(inserted(&["Zahl", "ggT"]), r"\mathrm{ggT}(;)");
    assert_eq!(inserted(&["Zahl", "kgV"]), r"\mathrm{kgV}(;)");
}

#[test]
fn the_whole_parts_rounding_and_the_remainder() {
    assert_eq!(inserted(&["Zahl", "Int"]), r"\mathrm{Int}()");
    assert_eq!(inserted(&["Zahl", "Intg"]), r"\mathrm{Intg}()");
    assert_eq!(inserted(&["Zahl", "Rnd"]), r"\mathrm{Rnd}()");
    assert_eq!(inserted(&["Zahl", "RndFix"]), r"\mathrm{RndFix}(;)");
    assert_eq!(inserted(&["Zahl", "÷R Rest"]), r"\mathrm{divR}(;)");
}

#[test]
fn the_product_has_its_bounds_to_fill() {
    assert_eq!(inserted(&["Analysis", "∏ Produkt"]), r"\prod_{}^{}");
}

#[test]
fn the_area_functions_and_coth_go_into_their_brackets() {
    let mut app = app(Mode::Physics);
    optn(&mut app, &["Hyperbol. Fkt.", "arsinh"]);
    app.key(Key::Digit(1));
    assert_eq!(app.editor.to_latex(), r"\mathrm{arsinh}(1)");
    assert_eq!(inserted(&["Hyperbol. Fkt.", "arcosh"]), r"\mathrm{arcosh}()");
    assert_eq!(inserted(&["Hyperbol. Fkt.", "artanh"]), r"\mathrm{artanh}()");
    assert_eq!(inserted(&["Hyperbol. Fkt.", "coth"]), r"\coth()");
}

#[test]
fn const_lists_the_constants_by_category_in_physik() {
    let mut app = app(Mode::Physics);
    app.key(Key::Digit(2));
    app.key(Key::Const);
    // The first category, its first constant.
    app.key(Key::Exe);
    app.key(Key::Exe);
    assert_eq!(app.screen, Screen::Calc);
    assert_eq!(app.editor.to_latex(), "2c_{0}");
    // ← from the categories closes CONST.
    app.key(Key::Const);
    app.key(Key::Left);
    assert_eq!(app.screen, Screen::Calc);
    // OPTN has them too.
    optn(&mut app, &["Konstanten"]);
    app.key(Key::Exe);
    app.key(Key::Down);
    app.key(Key::Exe);
    assert_eq!(app.editor.to_latex(), "2c_{0}h_{P}");
}

#[test]
fn const_says_the_constants_are_physiks() {
    let mut app = app(Mode::Algebra);
    app.key(Key::Const);
    let Screen::Menu(menu) = &app.screen else { panic!("CONST is open") };
    assert!(matches!(menu.edit, Some(quantus_ui::menu::Edit::Message { message, .. }) if message == quantus_ui::text::PHYSICS_ONLY));
}
