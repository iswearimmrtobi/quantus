//! STO keeps a result under a name; VARS lists the session's variables.

use quantus_ui::app::{App, Mode, Screen, Settings};
use quantus_ui::backend::{Backend, Reply, Request, Stored, Stub, VariableCommand};
use quantus_ui::key::Key;
use quantus_ui::menu::{Edit, TextFor};
use quantus_ui::text;

/// The stub's answers, and variables kept in memory: a value is kept as its LaTeX.
#[derive(Default)]
struct Memory {
    variables: Vec<Stored>,
    commands: Vec<VariableCommand>,
}

impl Backend for Memory {
    fn evaluate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> quantus_ui::backend::Answer {
        Stub.evaluate(mode, settings, latex)
    }

    fn solve(&mut self, settings: &Settings, request: &Request) -> Result<Reply, String> {
        Stub.solve(settings, request)
    }

    fn variables(&mut self, command: VariableCommand, _settings: &Settings) -> Result<Vec<Stored>, String> {
        self.commands.push(command.clone());
        match command {
            VariableCommand::List => {}
            VariableCommand::Store { latex, .. } if latex.contains('?') => return Err("Syntax error".to_string()),
            VariableCommand::Store { key, latex, .. } => {
                self.variables.retain(|variable| variable.key != key);
                self.variables.push(Stored { symbol: key.clone(), key, value: latex });
            }
            VariableCommand::Delete(key) => self.variables.retain(|variable| variable.key != key),
        }
        Ok(self.variables.clone())
    }
}

fn press(app: &mut App<Memory>, keys: &[Key]) {
    keys.iter().for_each(|&key| app.key(key));
}

fn edit(app: &App<Memory>) -> &Edit {
    let Screen::Menu(menu) = &app.screen else { panic!("no menu open") };
    menu.edit.as_ref().expect("a submenu is open")
}

fn typed(app: &mut App<Memory>, text: &str) {
    text.chars().for_each(|c| app.key(Key::from_char(c).expect("a key")));
}

#[test]
fn sto_keeps_the_last_result_under_the_name_typed() {
    let mut app = App::new(Memory::default());
    press(&mut app, &[Key::Digit(7), Key::Exe, Key::Shift, Key::Vars]);
    assert!(matches!(edit(&app), Edit::Text { purpose: TextFor::Variable(latex), .. } if latex == r"\mathrm{Ans}"));
    typed(&mut app, "r_1");
    press(&mut app, &[Key::Ok]);
    assert!(matches!(edit(&app), Edit::Message { message, .. } if *message == text::STORED));
    let stored = &app.backend().variables;
    assert_eq!((stored[0].key.as_str(), stored[0].value.as_str()), ("r_1", r"\mathrm{Ans}"));
}

#[test]
fn sto_takes_an_older_result_as_it_is_shown() {
    let mut app = App::new(Memory::default());
    press(&mut app, &[Key::Digit(1), Key::Exe, Key::Digit(2), Key::Exe, Key::Digit(3), Key::Exe]);
    // Back to the first: ↑ twice.
    press(&mut app, &[Key::Up, Key::Up, Key::Shift, Key::Vars]);
    assert!(matches!(edit(&app), Edit::Text { purpose: TextFor::Variable(latex), .. } if latex == "1"));
}

#[test]
fn sto_refuses_a_name_that_is_none() {
    let mut app = App::new(Memory::default());
    press(&mut app, &[Key::Digit(7), Key::Exe, Key::Shift, Key::Vars]);
    typed(&mut app, "1a");
    press(&mut app, &[Key::Ok]);
    assert!(matches!(edit(&app), Edit::Text { error: true, .. }));
    assert!(app.backend().commands.is_empty());
}

#[test]
fn sto_before_any_result_says_so() {
    let mut app = App::new(Memory::default());
    press(&mut app, &[Key::Shift, Key::Vars]);
    assert!(matches!(edit(&app), Edit::Message { message, .. } if *message == text::NO_RESULT));
}

#[test]
fn names_of_variables() {
    use quantus_ui::menu::variable_name;
    for name in ["A", "r_1", "v_0", "Uges", "x_ab2"] {
        assert!(variable_name(name), "{name}");
    }
    for name in ["", "1", "_1", "r_", "a b", "r__1", "ä"] {
        assert!(!variable_name(name), "{name}");
    }
}

#[test]
fn a_key_after_stored_goes_back_to_the_calculation() {
    let mut app = App::new(Memory::default());
    press(&mut app, &[Key::Digit(7), Key::Exe, Key::Shift, Key::Vars]);
    typed(&mut app, "A");
    press(&mut app, &[Key::Ok, Key::Ok]);
    assert_eq!(app.screen, Screen::Calc);
}

/// Two variables stored, A = 5 and B = 6, and VARS open on them.
fn with_two() -> App<Memory> {
    let mut memory = Memory::default();
    for (key, value) in [("A", "5"), ("B", "6")] {
        memory.variables.push(Stored { key: key.into(), symbol: key.into(), value: value.into() });
    }
    let mut app = App::new(memory);
    app.key(Key::Vars);
    assert!(matches!(edit(&app), Edit::Variables { variables, .. } if variables.len() == 2));
    app
}

#[test]
fn vars_inserts_a_variable() {
    let mut app = with_two();
    // B → Einfügen.
    press(&mut app, &[Key::Down, Key::Exe, Key::Exe]);
    assert_eq!(app.screen, Screen::Calc);
    assert_eq!(app.editor.to_latex(), "B");
}

#[test]
fn vars_changes_a_value_in_the_form_of_calc() {
    let mut app = with_two();
    // A → Wert ändern: the form holds 5; 7 instead, EXE twice.
    press(&mut app, &[Key::Exe, Key::Down, Key::Exe]);
    assert_eq!(app.sheet_kind(), Some(quantus_ui::sheet::Kind::Formula));
    assert_eq!(app.sheets.formula.latex(0, 0), "5");
    press(&mut app, &[Key::Digit(7), Key::Exe, Key::Exe]);
    let entry = app.shown_entry().expect("the new value is shown");
    assert_eq!(entry.result.as_ref().map(|row| row.to_latex()), Ok("A;A=7".to_string()));
}

#[test]
fn vars_deletes_a_variable_and_lists_the_rest() {
    let mut app = with_two();
    press(&mut app, &[Key::Exe, Key::Down, Key::Down, Key::Exe]);
    assert!(matches!(edit(&app), Edit::Variables { variables, .. } if variables.len() == 1 && variables[0].key == "B"));
    press(&mut app, &[Key::Exe, Key::Up, Key::Exe]);
    assert!(matches!(edit(&app), Edit::Message { message, .. } if *message == text::NO_VARIABLES));
    // ← from the actions goes back to the list, ← from the list closes VARS.
    let mut app = with_two();
    press(&mut app, &[Key::Exe, Key::Left]);
    assert!(matches!(edit(&app), Edit::Variables { .. }));
    press(&mut app, &[Key::Left]);
    assert_eq!(app.screen, Screen::Calc);
}

#[test]
fn vars_shows_a_matrix_by_its_size() {
    use quantus_ui::render::{Frame, Shape};
    let mut memory = Memory::default();
    let value = r"\begin{pmatrix} 1 & 2 & 3 \\ 4 & 5 & 6 \end{pmatrix}";
    memory.variables.push(Stored { key: "A".into(), symbol: "A".into(), value: value.into() });
    let mut app = App::new(memory);
    app.key(Key::Vars);
    let shown = Frame::new(&app).0.iter().any(|shape| matches!(shape, Shape::Text(_, text, _, _) if text == "Matrix 2 × 3"));
    assert!(shown);
}
