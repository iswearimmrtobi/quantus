//! The sheets worked out by the core: LGS, Wertetabelle, Statistik, Verteilungen, and kept in
//! the session.

use quantus_engine::Engine;
use quantus_ui::app::Settings;
use quantus_ui::backend::{Backend, Field, Line, Reply, Request, SessionCommand};
use quantus_ui::settings::{DistributionKind, StatisticsType};

fn cells(rows: &[&[&str]]) -> Vec<Vec<String>> {
    rows.iter().map(|row| row.iter().map(|cell| cell.to_string()).collect()).collect()
}

fn lines(reply: Reply) -> Vec<Line> {
    match reply {
        Reply::Lines(lines) => lines,
        table => panic!("lines expected: {table:?}"),
    }
}

fn math(lines: &[Line]) -> Vec<&str> {
    lines.iter().filter_map(|line| if let Line::Math(latex) = line { Some(latex.as_str()) } else { None }).collect()
}

#[test]
fn an_lgs_gives_its_solution_and_the_matrices_of_its_steps() {
    let mut engine = Engine::default();
    let request = Request::Linear { rows: cells(&[&["2", "1", "5"], &["1", "-1", "1"]]) };
    let lines = lines(engine.solve(&Settings::default(), &request).unwrap());
    assert_eq!(&math(&lines)[..2], ["x_{1} = 2", "x_{2} = 1"]);
    let matrix = lines.iter().find_map(|line| if let Line::Matrix(matrix) = line { Some(matrix) } else { None });
    let matrix = matrix.expect("a step shows the matrix");
    assert_eq!(matrix.rows.len(), 2);
    assert_eq!(matrix.split, 2);
    assert!(matrix.rows.iter().flatten().all(|cell| !cell.contains("textcolor")), "{matrix:?}");
}

#[test]
fn an_lgs_without_a_solution_says_so() {
    let mut engine = Engine::default();
    let request = Request::Linear { rows: cells(&[&["1", "1", "1"], &["2", "2", "3"]]) };
    let lines = lines(engine.solve(&Settings::default(), &request).unwrap());
    assert!(math(&lines)[0].contains("solution") || math(&lines)[0].contains("Lösung"), "{lines:?}");
}

#[test]
fn a_table_of_values_with_g_beside_f() {
    let mut engine = Engine::default();
    let request = Request::Table { f: "x^2".into(), g: Some("2x".into()), from: "0".into(), to: "3".into(), step: "1".into() };
    let Reply::Table { header, rows } = engine.solve(&Settings::default(), &request).unwrap() else { panic!("a table") };
    assert_eq!(header, ["x", "f(x)", "g(x)"]);
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[3], ["3", "9", "6"]);
}

#[test]
fn the_statistics_of_a_list() {
    let mut engine = Engine::default();
    let x = ["1", "2", "3", "4", "5"].map(String::from).to_vec();
    let request = Request::Statistics { kind: StatisticsType::OneVariable, x, y: None, freq: None };
    let lines = lines(engine.solve(&Settings::default(), &request).unwrap());
    let math = math(&lines);
    assert!(math.contains(&"n = 5"), "{math:?}");
    assert!(math.contains(&"\\bar{x} = 3"), "{math:?}");
}

#[test]
fn a_regression_comes_after_the_values() {
    let mut engine = Engine::default();
    let x = ["1", "2", "3"].map(String::from).to_vec();
    let y = Some(["3", "5", "7"].map(String::from).to_vec());
    let request = Request::Statistics { kind: StatisticsType::Linear, x, y, freq: None };
    let lines = lines(engine.solve(&Settings::default(), &request).unwrap());
    let heading = lines.iter().position(|line| *line == Line::Note("Regression".into())).expect("a heading");
    assert_eq!(math(&lines[heading..])[..2], ["a = 1", "b = 2"]);
}

#[test]
fn a_binomial_probability_one_k_and_several() {
    let mut engine = Engine::default();
    let settings = Settings { decimal: true, ..Settings::default() };
    let fields = vec![(Field::N, "4".to_string()), (Field::P, "0{,}5".to_string())];
    let kind = DistributionKind::BinomialDensity;
    let one = Request::Distribution { kind, fields: fields.clone(), k: vec!["2".into()] };
    let lines = lines(engine.solve(&settings, &one).unwrap());
    assert_eq!(math(&lines), ["P(X=k) = 0{,}375"]);

    let several = Request::Distribution { kind, fields, k: vec!["0".into(), "1".into()] };
    let Reply::Table { header, rows } = engine.solve(&settings, &several).unwrap() else { panic!("a table") };
    assert_eq!(header, ["k", "P(X=k)"]);
    assert_eq!(rows, [["0", "0{,}0625"], ["1", "0{,}25"]]);
}

#[test]
fn an_error_comes_back_as_text() {
    let mut engine = Engine::default();
    let request = Request::Table { f: "x".into(), g: None, from: "".into(), to: "".into(), step: "".into() };
    assert!(engine.solve(&Settings::default(), &request).is_err());
}

#[test]
fn the_session_keeps_the_sheets() {
    let mut engine = Engine::default();
    let settings = Settings::default();
    let linear = Request::Linear { rows: cells(&[&["2", "1", "5"], &["1", "-1", "1"]]) };
    let statistics = Request::Statistics {
        kind: StatisticsType::Linear,
        x: vec!["1".into(), "2".into()],
        y: Some(vec!["2".into(), "4".into()]),
        freq: None,
    };
    let distribution = Request::Distribution {
        kind: DistributionKind::PoissonCumulative,
        fields: vec![(Field::Mu, "3".into())],
        k: vec!["1".into()],
    };
    for request in [&linear, &statistics, &distribution] {
        engine.solve(&settings, request).unwrap();
    }
    let opened = engine.session(SessionCommand::Current, &settings).unwrap();
    assert_eq!(opened.inputs, [linear.clone(), statistics, distribution]);

    // A new session starts without them.
    let fresh = engine.session(SessionCommand::New, &settings).unwrap();
    assert!(fresh.inputs.is_empty());
}
