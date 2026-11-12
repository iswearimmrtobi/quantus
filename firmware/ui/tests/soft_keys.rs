//! The soft keys and their labels at the bottom of the screen: what they hold by mode and input,
//! and what a press does (design/handheld-menus.md §7.4.1, firmware/docs/unit-keys.md §2).

use std::cell::RefCell;
use std::rc::Rc;

use quantus_ui::app::{App, Mode, Screen, Settings};
use quantus_ui::backend::{Backend, Item, Stub, Units};
use quantus_ui::key::Key;
use quantus_ui::menu::Edit;
use quantus_ui::render::{slot_rect, Frame, HEIGHT};
use quantus_ui::settings::NumberBase;
use quantus_ui::soft;
use quantus_ui::text::Lang;
use quantus_ui::touch::{Target, Touch};

/// What the app asked for: the input before the cursor, the recent units, a result.
type Asked = Rc<RefCell<Vec<(String, Vec<String>, Option<String>)>>>;

/// The stub, and units that say what they were asked for.
struct Recorder {
    asked: Asked,
}

impl Backend for Recorder {
    fn evaluate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> quantus_ui::backend::Answer {
        Stub.evaluate(mode, settings, latex)
    }

    fn unit_suggestions(&mut self, before: &str, recent: &[String], result: Option<&str>) -> Units {
        self.asked.borrow_mut().push((before.to_string(), recent.to_vec(), result.map(str::to_string)));
        let item = |name: &str| Item { name: name.to_string(), latex: format!(r"\mathrm{{{name}}}") };
        match result {
            Some(_) => Units { best: vec![item("J"), item("kJ")], more: Vec::new() },
            None => Units { best: vec![item("W"), item("kW")], more: vec![item("hp")] },
        }
    }
}

fn recorder(mode: Mode) -> (App<Recorder>, Asked) {
    let asked = Asked::default();
    let mut app = App::new(Recorder { asked: asked.clone() });
    app.mode = mode;
    (app, asked)
}

fn app(mode: Mode) -> App<Stub> {
    let mut app = App::new(Stub);
    app.mode = mode;
    app
}

fn press<B: Backend>(app: &mut App<B>, keys: &[Key]) {
    keys.iter().for_each(|&key| app.key(key));
}

/// The labels of the soft keys shown, as the strips draw them.
fn labels<B>(app: &App<B>) -> Vec<String> {
    let Some(bar) = soft::bar(app) else { return Vec::new() };
    (0..bar.slots.len()).map(|index| bar.label(index, Lang::De)).collect()
}

#[test]
fn basis_n_shows_the_bases_and_the_logic_and_sets_the_base() {
    let mut app = app(Mode::BaseN);
    assert_eq!(labels(&app), ["DEC", "HEX", "BIN", "OCT", "and", "or", "xor", "xnor", "Not", "Neg"]);
    press(&mut app, &[Key::Soft(1)]);
    assert_eq!(app.settings.number_base, NumberBase::Hex);
    press(&mut app, &[Key::Digit(5), Key::Soft(5)]);
    assert_eq!(app.editor.to_latex(), r"5\mathrm{or}");
}

#[test]
fn pruefen_types_the_relations() {
    let mut app = app(Mode::Verify);
    press(&mut app, &[Key::Digit(1), Key::Soft(1), Key::Digit(2)]);
    assert_eq!(app.editor.to_latex(), r"1\neq2");
}

#[test]
fn more_keys_than_fit_go_on_pages() {
    let mut app = app(Mode::Algebra);
    app.soft_columns = 5;
    let first = labels(&app);
    assert_eq!(first.len(), 10);
    assert_eq!(first[9], r"\blacktriangleright 1/2");
    press(&mut app, &[Key::Soft(9)]);
    assert_eq!(labels(&app)[..4], ["ggT", "kgV", "mod", ""]);
    // Another key goes back to the first page.
    press(&mut app, &[Key::Digit(1)]);
    assert_eq!(labels(&app)[0], ";");
}

#[test]
fn symb_types_one_symbol_and_the_bar_comes_back() {
    let mut app = app(Mode::Physics);
    press(&mut app, &[Key::Symb]);
    assert_eq!(labels(&app)[..7], ["Δ", "ω", "ρ", "μ", "ε", "Ω", "°"]);
    press(&mut app, &[Key::Soft(0), Key::Letter('x')]);
    assert_eq!(app.editor.to_latex(), r"\Delta x");
    assert_eq!(app.symbols, None);
    // Algebra leads with π; SYMB again closes.
    let mut app = self::app(Mode::Algebra);
    press(&mut app, &[Key::Symb]);
    assert_eq!(labels(&app)[0], "π");
    press(&mut app, &[Key::Symb]);
    assert_eq!(labels(&app)[0], ";");
}

#[test]
fn the_strips_hide_on_sheets_and_answers() {
    assert!(soft::bar(&app(Mode::Linear)).is_none());
    assert!(soft::bar(&app(Mode::Statistics)).is_none());
    let mut app = app(Mode::Analysis);
    assert_eq!(labels(&app)[0], "Abschn.");
    press(&mut app, &[Key::Soft(1)]);
    assert!(soft::bar(&app).is_none(), "the tangent's form is a sheet");
    // SYMB shows on a sheet, to type into its cell.
    press(&mut app, &[Key::Symb]);
    assert!(soft::bar(&app).is_some());
}

#[test]
fn physik_asks_for_units_when_the_input_reaches_an_operator() {
    let (mut app, asked) = recorder(Mode::Physics);
    asked.borrow_mut().clear();
    press(&mut app, &[Key::Digit(3), Key::Letter('k'), Key::Letter('W')]);
    assert!(asked.borrow().is_empty(), "no new question while the first term is typed");
    press(&mut app, &[Key::Plus]);
    assert_eq!(asked.borrow().last().unwrap().0, "3kW+");
    assert_eq!(labels(&app), ["W", "kW", "", "", "", "", "hp"]);
    press(&mut app, &[Key::Digit(4), Key::Soft(1)]);
    assert_eq!(app.editor.to_latex(), r"3kW+4\mathrm{kW}");
    assert_eq!(asked.borrow().len(), 1, "typing after the operator asks nothing new");
}

#[test]
fn units_typed_come_first_after_exe() {
    let (mut app, asked) = recorder(Mode::Physics);
    press(&mut app, &[Key::Digit(3), Key::Plus, Key::Digit(4), Key::Soft(1), Key::Exe]);
    // And units typed on the UNIT layer, a prefix and its unit as one.
    press(&mut app, &[Key::Ac, Key::Digit(2), Key::Unit, Key::Digit(9), Key::Sin, Key::Exe]);
    assert_eq!(app.recent_units, ["kN", "kW"]);
    press(&mut app, &[Key::Ac, Key::Digit(1), Key::Plus]);
    assert_eq!(asked.borrow().last().unwrap().1, ["kN", "kW"]);
}

#[test]
fn a_result_has_its_forms_and_converts() {
    let (mut app, asked) = recorder(Mode::Physics);
    press(&mut app, &[Key::Digit(5), Key::Letter('J'), Key::Exe]);
    assert_eq!(asked.borrow().last().unwrap().2.as_deref(), Some("5J"));
    assert_eq!(labels(&app), [r"S\Leftrightarrow D", "ENG", "←ENG", "", "", "", r"\blacktriangleright J", r"\blacktriangleright kJ"]);
    press(&mut app, &[Key::Soft(7)]);
    let entry = app.history.last().unwrap();
    assert_eq!(entry.input.to_latex(), r"5J\blacktriangleright\mathrm{kJ}");
    // Algebra has the forms alone.
    let mut app = self::app(Mode::Algebra);
    press(&mut app, &[Key::Digit(2), Key::Exe]);
    assert_eq!(labels(&app), [r"S\Leftrightarrow D", "ENG", "←ENG"]);
}

#[test]
fn a_command_waiting_for_the_engine_says_so_and_goes_back() {
    let mut app = app(Mode::Imaginary);
    press(&mut app, &[Key::Soft(1)]);
    let Screen::Menu(menu) = &app.screen else { panic!("no message") };
    assert!(matches!(menu.edit, Some(Edit::Message { .. })));
    press(&mut app, &[Key::Ok]);
    assert!(matches!(app.screen, Screen::Calc));
}

#[test]
fn a_tap_on_a_label_presses_its_key() {
    let mut app = app(Mode::Verify);
    let at = slot_rect(4, 6).center();
    assert!(at.y > HEIGHT as i32 - 44);
    app.touch(Touch::Press(at));
    assert_eq!(app.pressed(), Some(Target::Soft(4)));
    let _ = Frame::new(&app);
    app.touch(Touch::Release);
    assert_eq!(app.editor.to_latex(), r"\le");
}
