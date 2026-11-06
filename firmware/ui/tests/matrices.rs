//! Matrizen: a matrix defined under a name in a grid, and changed again from VARS.

use quantus_ui::app::{App, Mode, Screen, Settings};
use quantus_ui::backend::{Backend, Stored, Stub, VariableCommand};
use quantus_ui::key::Key;
use quantus_ui::sheet::Kind;

/// The stub's answers, and one matrix among the variables.
struct OneMatrix;

impl Backend for OneMatrix {
    fn evaluate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> Result<String, String> {
        Stub.evaluate(mode, settings, latex)
    }

    fn variables(&mut self, _command: VariableCommand, _settings: &Settings) -> Result<Vec<Stored>, String> {
        let value = r"\begin{pmatrix} 1 & 2 & 3 \\ 4 & 5 & 6 \end{pmatrix}";
        Ok(vec![Stored { key: "M_1".into(), symbol: "M_{1}".into(), value: value.into() }])
    }
}

fn press<B: Backend>(app: &mut App<B>, keys: &[Key]) {
    keys.iter().for_each(|&key| app.key(key));
}

#[test]
fn a_matrix_is_named_filled_in_and_defined() {
    let mut app = App::new(Stub);
    app.mode = Mode::Matrix;
    // OPTN → Definieren, the name A.
    press(&mut app, &[Key::Optn, Key::Exe, Key::Letter('A'), Key::Ok]);
    assert_eq!(app.screen, Screen::Calc);
    assert_eq!(app.sheet_kind(), Some(Kind::Matrix));
    assert_eq!((app.sheets.matrix.rows(), app.sheets.matrix.columns()), (2, 2));
    // 1, 2 along the first row, 3 down; the last cell left empty is 0.
    press(&mut app, &[Key::Digit(1), Key::Exe, Key::Digit(2), Key::Exe, Key::Digit(3), Key::Exe, Key::Exe, Key::Exe]);
    assert_eq!(app.sheet_kind(), None, "the grid is closed");
    assert_eq!(app.editor.to_latex(), r"A=\begin{pmatrix}1&2\\3&0\end{pmatrix}");
    assert_eq!(app.history.len(), 1, "defined as an input");
}

#[test]
fn the_size_comes_from_optn_and_ac_leaves_the_grid() {
    let mut app = App::new(Stub);
    app.mode = Mode::Matrix;
    // Zeilen → 3, Spalten → 1.
    press(&mut app, &[Key::Optn, Key::Digit(3), Key::Digit(3), Key::Exe, Key::Digit(4), Key::Digit(1), Key::Exe, Key::Ac]);
    assert_eq!((app.settings.matrix_rows, app.settings.matrix_columns), (3, 1));
    press(&mut app, &[Key::Optn, Key::Exe, Key::Letter('B'), Key::Ok]);
    assert_eq!((app.sheets.matrix.rows(), app.sheets.matrix.columns()), (3, 1));
    press(&mut app, &[Key::Ac]);
    assert_eq!(app.sheet_kind(), None);
    assert!(app.history.is_empty());
}

#[test]
fn vars_changes_a_matrix_in_the_grid() {
    let mut app = App::new(OneMatrix);
    // VARS → M_1 → Wert ändern.
    press(&mut app, &[Key::Vars, Key::Exe, Key::Down, Key::Exe]);
    assert_eq!(app.mode, Mode::Matrix);
    assert_eq!(app.sheet_kind(), Some(Kind::Matrix));
    assert_eq!((app.sheets.matrix.rows(), app.sheets.matrix.columns()), (2, 3));
    assert_eq!(app.sheets.matrix.latex(1, 2), "6");
    assert_eq!(app.sheets.definition(), r"M_{1}=\begin{pmatrix}1&2&3\\4&5&6\end{pmatrix}");
}
