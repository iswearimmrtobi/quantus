//! STO keeps a result under a name; VARS lists the session's variables.

use quantus_ui::app::{App, Mode, Screen, Settings};
use quantus_ui::backend::{Backend, Stored, Stub, VariableCommand};
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
    fn evaluate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> Result<String, String> {
        Stub.evaluate(mode, settings, latex)
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
