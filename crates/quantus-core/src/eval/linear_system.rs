//! The linear systems mode: a system typed as equations or given as the matrix `[A | b]`.
//!
//! Both go through the same elimination as a system typed with `;` in Algebra; a matrix is read
//! cell by cell into values, never turned into equations first. The mode uses no session
//! variables, so every letter is an unknown (`k_1` too), and `i` is one unless `linear_complex`
//! makes it the imaginary unit. The solution is written exactly.

use serde::{Deserialize, Serialize};

use super::calculator::LatexCalculator;
use super::evaluator::{Evaluator, Variables};
use super::system::{solve_matrix, SystemSolution};
use super::system_steps::unknown_latex;
use crate::latex::{parser, Node};
use crate::num::Value;
use crate::settings::{Mode, Settings, UnknownNames};
use crate::text::{msg, Scope};
use crate::{QuantusError, Result};

/// The letters of `UnknownNames::Letters`, in the order of the unknowns; `e` and `i` are left
/// out, being Euler's number and (with the toggle) the imaginary unit.
const LETTERS: [&str; 24] =
    ["x", "y", "z", "a", "b", "c", "d", "f", "g", "h", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s", "t", "u", "v", "w"];

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LinearSystemRequest {
    /// `2x + y = 5; x - y = 1`, or a `cases` block with one equation per line.
    Equations(String),
    /// The rows of `[A | b]` with LaTeX entries; an empty entry is 0. Without `unknowns` the
    /// columns are named by the setting `unknown_names`.
    Matrix {
        #[serde(default)]
        unknowns: Option<Vec<String>>,
        rows: Vec<Vec<String>>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LinearSystemResponse {
    /// The unknowns in LaTeX, in the order of the matrix columns: `x_{1}`.
    pub unknowns: Vec<String>,
    /// `[A | b]` as LaTeX entries, one row per equation.
    pub matrix: Vec<Vec<String>>,
    /// `x_{1} = 2 \land x_{2} = 1`, exact.
    pub result: String,
}

pub fn linear_system(settings: &Settings, request: &LinearSystemRequest) -> Result<LinearSystemResponse> {
    let _language = Scope::new(settings.language);
    // The mode decides how `i` is read, whichever mode the call comes from; results are exact.
    let settings = Settings { mode: Mode::Linear, exact_roots: true, ..settings.clone() };
    let variables = Variables::new();
    let evaluator = Evaluator::new(&variables).with_precision(settings.precision).with_mode(settings.evaluation_mode());
    let system = match request {
        LinearSystemRequest::Equations(latex) => {
            let expression = parser::parse_with(latex, &settings.parse_options())?.expression;
            let system = match expression {
                Node::System { .. } => expression,
                equation @ Node::Equation { .. } => Node::System { position: equation.position(), equations: vec![equation] },
                _ => {
                    return Err(QuantusError::other(msg!(
                        "Type the system as equations, one per line.",
                        "Das System als Gleichungen eingeben, eine pro Zeile."
                    )))
                }
            };
            evaluator.solve_system(&system, settings.elimination)?
        }
        LinearSystemRequest::Matrix { unknowns, rows } => {
            let n = width(rows)? - 1;
            let keys = match unknowns {
                Some(names) => given_names(names, n, &settings)?,
                None => default_names(settings.unknown_names, n),
            };
            solve_matrix(keys, read_matrix(rows, &settings, &evaluator)?, settings.elimination)?
        }
    };
    let calculator = LatexCalculator::new(settings.precision).with_settings(settings);
    respond(&calculator, &system)
}

fn respond(calculator: &LatexCalculator, system: &SystemSolution) -> Result<LinearSystemResponse> {
    let names: Vec<String> = system.unknowns.iter().map(|key| unknown_latex(key)).collect();
    let matrix = system
        .matrix
        .iter()
        .map(|row| row.iter().map(|value| calculator.entry(value)).collect::<Result<Vec<_>>>())
        .collect::<Result<_>>()?;
    let (result, ..) = calculator.system_result(system, &names)?;
    Ok(LinearSystemResponse { unknowns: names, matrix, result })
}

/// The number of entries per row, the same in every row and at least two: a coefficient and the
/// right-hand side.
fn width(rows: &[Vec<String>]) -> Result<usize> {
    let Some(first) = rows.first() else {
        return Err(QuantusError::other(msg!("The matrix has no rows.", "Die Matrix hat keine Zeilen.")));
    };
    if first.len() < 2 {
        return Err(QuantusError::other(msg!(
            "A row needs at least one coefficient and the right-hand side.",
            "Eine Zeile braucht mindestens einen Koeffizienten und die rechte Seite."
        )));
    }
    if let Some((r, row)) = rows.iter().enumerate().find(|(_, row)| row.len() != first.len()) {
        return Err(QuantusError::other(msg!(
            "Row {} has {} entries, but row 1 has {}.",
            "Zeile {} hat {} Einträge, Zeile 1 aber {}.",
            r + 1,
            row.len(),
            first.len()
        )));
    }
    Ok(first.len())
}

/// `x_1 … x_n`, or `x, y, z, a, …, w` and then `x_1, x_2, …`.
fn default_names(names: UnknownNames, n: usize) -> Vec<String> {
    match names {
        UnknownNames::Indexed => (1..=n).map(|k| format!("x_{k}")).collect(),
        UnknownNames::Letters => {
            let indexed = (1..).map(|k| format!("x_{k}"));
            LETTERS.iter().map(|letter| letter.to_string()).chain(indexed).take(n).collect()
        }
    }
}

/// The keys of names given with the matrix (`x_{1}` is `x_1`): one per column, each a variable,
/// none twice.
fn given_names(names: &[String], n: usize, settings: &Settings) -> Result<Vec<String>> {
    if names.len() != n {
        return Err(QuantusError::other(msg!(
            "There are {} names for {} unknowns.",
            "Es gibt {} Namen für {} Unbekannte.",
            names.len(),
            n
        )));
    }
    let mut keys = Vec::with_capacity(n);
    for name in names {
        let key = match parser::parse_with(name, &settings.parse_options()).map(|f| f.expression) {
            Ok(Node::Variable { name, index, .. }) => Node::variable_key(&name, &index),
            _ => {
                return Err(QuantusError::other(msg!(
                    "{} is not a name for an unknown; use a letter such as x or x_1.",
                    "{} ist kein Name für eine Unbekannte; einen Buchstaben wie x oder x_1 verwenden.",
                    name
                )))
            }
        };
        if keys.contains(&key) {
            return Err(QuantusError::other(msg!("The unknown {} is named twice.", "Die Unbekannte {} ist doppelt benannt.", name)));
        }
        keys.push(key);
    }
    Ok(keys)
}

/// Each entry evaluated to a number. An empty entry is 0; a letter or a unit is an error, since
/// the mode has no stored values and its coefficients are numbers.
fn read_matrix(rows: &[Vec<String>], settings: &Settings, evaluator: &Evaluator) -> Result<Vec<Vec<Value>>> {
    let mut matrix = Vec::with_capacity(rows.len());
    for (r, row) in rows.iter().enumerate() {
        let mut values = Vec::with_capacity(row.len());
        for (c, entry) in row.iter().enumerate() {
            let entry = entry.trim();
            if entry.is_empty() {
                values.push(Value::zero());
                continue;
            }
            let at = |reason: String| {
                QuantusError::other(msg!("Row {}, entry {}: {}", "Zeile {}, Eintrag {}: {}", r + 1, c + 1, reason))
            };
            let formula = parser::parse_with(entry, &settings.parse_options()).map_err(|e| at(e.to_string()))?;
            let expression = formula.expression;
            if formula.label.is_some() || matches!(expression, Node::Equation { .. } | Node::System { .. }) {
                return Err(at(msg!("an entry is a number, not an equation.", "ein Eintrag ist eine Zahl, keine Gleichung.")));
            }
            match letter(&expression) {
                Some(name) if name == "i" => {
                    return Err(at(msg!(
                        "i is not a number here; turn on the imaginary unit for complex coefficients.",
                        "i ist hier keine Zahl; für komplexe Koeffizienten die imaginäre Einheit einschalten."
                    )))
                }
                Some(name) => {
                    return Err(at(msg!("an entry is a number, but {} is a letter.", "ein Eintrag ist eine Zahl, {} aber ein Buchstabe.", name)))
                }
                None => {}
            }
            let quantity = evaluator.evaluate(&expression).map_err(|e| at(e.to_string()))?;
            if !quantity.is_dimensionless() {
                return Err(at(msg!(
                    "the coefficients of a linear system have no units.",
                    "die Koeffizienten eines linearen Gleichungssystems haben keine Einheiten."
                )));
            }
            values.push(quantity.value);
        }
        matrix.push(values);
    }
    Ok(matrix)
}

/// The first variable in an entry, as typed: `k_1`.
fn letter(node: &Node) -> Option<String> {
    match node {
        Node::Variable { name, index, .. } => Some(Node::variable_key(name, index)),
        other => other.children().into_iter().find_map(letter),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_run_from_x_to_w_and_then_get_an_index() {
        assert_eq!(default_names(UnknownNames::Indexed, 3), ["x_1", "x_2", "x_3"]);
        assert_eq!(default_names(UnknownNames::Letters, 4), ["x", "y", "z", "a"]);
        let many = default_names(UnknownNames::Letters, 26);
        assert_eq!(many[23..], ["w", "x_1", "x_2"]);
    }

    fn rows(rows: &[&[&str]]) -> Vec<Vec<String>> {
        rows.iter().map(|row| row.iter().map(|entry| entry.to_string()).collect()).collect()
    }

    fn matrix(settings: &Settings, entries: &[&[&str]]) -> Result<LinearSystemResponse> {
        linear_system(settings, &LinearSystemRequest::Matrix { unknowns: None, rows: rows(entries) })
    }

    fn equations(settings: &Settings, latex: &str) -> Result<LinearSystemResponse> {
        linear_system(settings, &LinearSystemRequest::Equations(latex.into()))
    }

    #[test]
    fn equations_and_a_matrix_give_the_same_system() {
        let settings = Settings::default();
        let typed = equations(&settings, "x_1 + x_2 = 1; x_1 - x_2 = 0").unwrap();
        let given = matrix(&settings, &[&["1", "1", "1"], &["1", "-1", ""]]).unwrap();
        assert_eq!(typed, given);
        assert_eq!(given.unknowns, ["x_{1}", "x_{2}"]);
        assert_eq!(given.matrix, [["1", "1", "1"], ["1", "-1", "0"]]);
        // Exact, not 0,5.
        assert_eq!(given.result, "x_{1} = \\frac{1}{2} \\land x_{2} = \\frac{1}{2}");
        // A single equation is a system too.
        assert!(equations(&settings, "x + y = 3").unwrap().result.contains("\\lambda \\in \\mathbb{R}"));
    }

    #[test]
    fn the_names_of_the_columns() {
        let letters = Settings { unknown_names: UnknownNames::Letters, ..Settings::default() };
        assert_eq!(matrix(&letters, &[&["1", "0", "2", "1"]]).unwrap().unknowns, ["x", "y", "z"]);
        let named = |names: &[&str]| {
            let unknowns = Some(names.iter().map(|name| name.to_string()).collect());
            linear_system(&Settings::default(), &LinearSystemRequest::Matrix { unknowns, rows: rows(&[&["2", "1", "5"], &["1", "-1", "1"]]) })
        };
        assert_eq!(named(&["a", "b_{2}"]).unwrap().result, "a = 2 \\land b_{2} = 1");
        assert!(named(&["a"]).unwrap_err().to_string().contains("1 names for 2 unknowns"));
        assert!(named(&["a", "a"]).unwrap_err().to_string().contains("named twice"));
        assert!(named(&["a", "2"]).unwrap_err().to_string().contains("not a name"));
    }

    #[test]
    fn a_matrix_holds_numbers_only() {
        let settings = Settings::default();
        let error = |entries: &[&[&str]]| matrix(&settings, entries).unwrap_err().to_string();
        assert_eq!(error(&[&["2", "k_1", "5"]]), "Row 1, entry 2: an entry is a number, but k_1 is a letter.");
        assert!(error(&[&["2", "1", "5"], &["1", "2\\,\\mathrm{m}", "1"]]).starts_with("Row 2, entry 2: the coefficients"));
        assert!(error(&[&["2", "1", "5"], &["1", "1"]]).contains("Row 2 has 2 entries, but row 1 has 3"));
        assert!(error(&[&["5"]]).contains("at least one coefficient"));
        assert!(error(&[&["x = 1", "5"]]).contains("not an equation"));
        assert!(error(&[&["i", "5"]]).contains("turn on the imaginary unit"));
        assert_eq!(matrix(&settings, &[&["\\frac{1}{2}", "\\sqrt{4}", "\\pi"]]).unwrap().matrix, [["\\frac{1}{2}", "2", "\\pi"]]);
    }

    #[test]
    fn session_values_are_not_used_and_i_follows_the_toggle() {
        // Whatever the calculator holds, k_1 and i are unknowns here.
        let real = equations(&Settings::default(), "k_1 + i = 2; k_1 - i = 0").unwrap();
        assert_eq!((real.unknowns, real.result.as_str()), (vec!["i".to_string(), "k_{1}".to_string()], "i = 1 \\land k_{1} = 1"));
        let complex = Settings { linear_complex: true, ..Settings::default() };
        assert_eq!(equations(&complex, "x + iy = 1; x - y = i").unwrap().result, "x = 0 \\land y = -i");
        let solved = matrix(&complex, &[&["1+i", "1", "2"], &["1", "-i", ""]]).unwrap();
        assert_eq!((solved.matrix[0][0].as_str(), solved.result.as_str()), ("1 + i", "x_{1} = 2 \\land x_{2} = -2i"));
        let free = equations(&complex, "x + iy = 1").unwrap().result;
        assert!(free.ends_with("\\in \\mathbb{C}"), "{free}");
    }

    #[test]
    fn more_equations_than_unknowns_and_fewer() {
        let settings = Settings::default();
        assert_eq!(matrix(&settings, &[&["1", "1", "5"], &["1", "-1", "1"], &["1", "", "3"]]).unwrap().result, "x_{1} = 3 \\land x_{2} = 2");
        assert_eq!(matrix(&settings, &[&["1", "1", "5"], &["1", "-1", "1"], &["1", "", "4"]]).unwrap().result, "\\text{no solution}");
        let under = matrix(&settings, &[&["1", "1", "1", "5"], &["1", "-1", "2", "1"]]).unwrap().result;
        assert_eq!(under, "x_{1} = 3 - \\frac{3}{2} \\cdot \\lambda \\land x_{2} = 2 + \\frac{1}{2} \\cdot \\lambda \\land x_{3} = \\lambda \\land \\lambda \\in \\mathbb{R}");
    }
}

