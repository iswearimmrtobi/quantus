//! Systems of linear equations: the golden cases of `systems.json`, the result and every step
//! with both elimination methods and at each detail level; and the linear systems mode through
//! `api::linear_system`, typed and as a matrix.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use quantus_core::api::{self, LinearSystemRequest, LinearSystemResponse};
use quantus_core::eval::LatexCalculator;
use quantus_core::settings::{Elimination, Settings, UnknownNames};
use serde_json::{json, Value as Json};

mod common;

#[test]
fn system_cases_match_their_steps() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/systems.json");
    let golden: Json = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    common::check_all(golden["cases"].as_array().unwrap(), |case| case["name"].as_str().unwrap().to_string(), check_case);
}

fn check_case(case: &Json) {
    let name = case["name"].as_str().unwrap();
    let settings: Settings = serde_json::from_value(json!({
        "mode": "algebra",
        "elimination": case["elimination"],
        "stepDetail": case["stepDetail"],
    }))
    .unwrap();
    let calculation = LatexCalculator::default()
        .with_settings(settings)
        .evaluate(case["latex"].as_str().unwrap(), &HashMap::new())
        .unwrap_or_else(|e| panic!("{name}: {e}"));
    assert_eq!(calculation.result, case["expect"]["result"].as_str().unwrap(), "{name}: result");
    let steps: Vec<Json> =
        calculation.steps.iter().map(|s| json!([s.latex, s.explanation.clone().unwrap_or_default()])).collect();
    let expected = case["expect"]["steps"].as_array().unwrap();
    for (i, (step, want)) in steps.iter().zip(expected).enumerate() {
        assert_eq!(step, want, "{name}: step {i}");
    }
    assert_eq!(steps.len(), expected.len(), "{name}: number of steps");
}

fn typed(settings: &Settings, latex: &str) -> LinearSystemResponse {
    api::linear_system(settings, &LinearSystemRequest::Equations(latex.into())).unwrap_or_else(|e| panic!("{latex}: {e}"))
}

fn given(settings: &Settings, rows: &[&[&str]]) -> LinearSystemResponse {
    let rows = rows.iter().map(|row| row.iter().map(|entry| entry.to_string()).collect()).collect();
    api::linear_system(settings, &LinearSystemRequest::Matrix { unknowns: None, rows }).unwrap()
}

fn with(elimination: Elimination, linear_complex: bool) -> Settings {
    Settings { elimination, linear_complex, ..Settings::default() }
}

#[test]
fn both_methods_give_the_same_solution_typed_and_as_a_matrix() {
    let systems: [(&str, &[&[&str]], &str); 4] = [
        ("2x_1 + x_2 = 5; x_1 - x_2 = 1", &[&["2", "1", "5"], &["1", "-1", "1"]], "x_{1} = 2 \\land x_{2} = 1"),
        (
            "x_1 + x_2 + x_3 = 6; x_1 - x_2 = 0; 2x_3 = 6",
            &[&["1", "1", "1", "6"], &["1", "-1", "", ""], &["", "", "2", "6"]],
            "x_{1} = \\frac{3}{2} \\land x_{2} = \\frac{3}{2} \\land x_{3} = 3",
        ),
        ("x_1 + x_2 = 1; 2x_1 + 2x_2 = 3", &[&["1", "1", "1"], &["2", "2", "3"]], "\\text{no solution}"),
        // More equations than unknowns, one of them redundant.
        ("x_1 + x_2 = 5; x_1 - x_2 = 1; 2x_1 = 6", &[&["1", "1", "5"], &["1", "-1", "1"], &["2", "", "6"]], "x_{1} = 3 \\land x_{2} = 2"),
    ];
    for elimination in [Elimination::Gauss, Elimination::GaussJordan] {
        let settings = with(elimination, false);
        for (latex, rows, result) in systems {
            let (typed, given) = (typed(&settings, latex), given(&settings, rows));
            assert_eq!((typed.result.as_str(), given.result.as_str()), (result, result), "{latex}, {elimination:?}");
            assert_eq!(typed.matrix, given.matrix, "{latex}");
        }
    }
}

#[test]
fn fewer_equations_than_unknowns_give_parameters() {
    let gauss = typed(&with(Elimination::Gauss, false), "x + y + z = 6; x - y = 0");
    assert_eq!(gauss.result, "x = 3 - \\frac{1}{2} \\cdot \\lambda \\land y = 3 - \\frac{1}{2} \\cdot \\lambda \\land z = \\lambda \\land \\lambda \\in \\mathbb{R}");
    // Gauss–Jordan leaves the free unknown as it is.
    let jordan = typed(&with(Elimination::GaussJordan, false), "x + y + z = 6; x - y = 0");
    assert_eq!(jordan.result, "x = 3 - \\frac{1}{2} \\cdot z \\land y = 3 - \\frac{1}{2} \\cdot z \\land z \\in \\mathbb{R}");
}

#[test]
fn i_is_an_unknown_unless_the_toggle_makes_it_the_imaginary_unit() {
    for elimination in [Elimination::Gauss, Elimination::GaussJordan] {
        let real = typed(&with(elimination, false), "x + i = 3; x - i = 1");
        assert_eq!((real.unknowns, real.result), (vec!["x".to_string(), "i".to_string()], "x = 2 \\land i = 1".to_string()));
        let complex = with(elimination, true);
        assert_eq!(typed(&complex, "x + iy = 1; x - y = i").result, "x = 0 \\land y = -i");
        assert_eq!(given(&complex, &[&["1", "i", "1"], &["1", "-1", "i"]]).result, "x_{1} = 0 \\land x_{2} = -i");
    }
}

#[test]
fn equations_to_matrix_to_equations_round_trips() {
    let settings = Settings { unknown_names: UnknownNames::Letters, ..Settings::default() };
    let first = typed(&settings, "3y - x = \\frac{1}{2}; z + 2x = \\sqrt{2}; y = \\pi");
    assert_eq!(first.equations, ["-x + 3y = \\frac{1}{2}", "2x + z = \\sqrt{2}", "y = \\pi"]);
    // The matrix as the matrix tab would send it, named by the setting (x, y, z), then back.
    let rows: Vec<Vec<&str>> = first.matrix.iter().map(|row| row.iter().map(String::as_str).collect()).collect();
    let rows: Vec<&[&str]> = rows.iter().map(Vec::as_slice).collect();
    let second = given(&settings, &rows);
    assert_eq!((&second.unknowns, &second.equations), (&first.unknowns, &first.equations));
    let third = typed(&settings, &format!("\\begin{{cases}} {} \\end{{cases}}", second.equations.join(" \\\\ ")));
    assert_eq!((third.matrix, third.result), (first.matrix, first.result));
}

#[test]
fn the_mode_refuses_what_is_not_a_linear_system() {
    let settings = Settings::default();
    let error = |request: LinearSystemRequest| api::linear_system(&settings, &request).unwrap_err().error;
    assert!(error(LinearSystemRequest::Equations("x^2 + y = 1; x = 2".into())).contains("not linear"));
    assert!(error(LinearSystemRequest::Equations("2 + 3".into())).contains("as equations"));
    let rows = vec![vec!["1".to_string(), "2\\,\\mathrm{m}".to_string(), "3".to_string()]];
    assert!(error(LinearSystemRequest::Matrix { unknowns: None, rows }).contains("no units"));
    let german = Settings { language: quantus_core::settings::Language::De, ..Settings::default() };
    let rows = vec![vec!["1".to_string(), "k".to_string()]];
    let message = api::linear_system(&german, &LinearSystemRequest::Matrix { unknowns: None, rows }).unwrap_err().error;
    assert_eq!(message, "Zeile 1, Eintrag 2: ein Eintrag ist eine Zahl, k aber ein Buchstabe.");
}
