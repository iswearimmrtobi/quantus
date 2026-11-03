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

fn sketch(reply: Reply) -> quantus_ui::backend::Analysis {
    match reply {
        Reply::Analysis(analysis) => analysis,
        other => panic!("a sketch expected: {other:?}"),
    }
}

#[test]
fn a_curve_sketch_has_a_page_per_section_and_its_graph() {
    use quantus_ui::backend::{MarkKind, Status};
    let mut engine = Engine::default();
    let sketch = sketch(engine.solve(&Settings::default(), &Request::Analysis { f: "x^3-3x".into() }).unwrap());
    assert_eq!(sketch.function, "f(x) = x^{3} - 3x");
    let titles: Vec<&str> = sketch.pages.iter().map(|page| page.title.as_str()).collect();
    assert_eq!(titles[..3], ["Ableitungen", "Definitionsbereich", "Symmetrie"]);
    assert!(sketch.pages.iter().all(|page| page.status == Status::Ok && !page.lines.is_empty()));
    // The derivatives' result, a line each.
    let derivatives = math(&sketch.pages[0].lines);
    assert!(derivatives.ends_with(&["f'(x) = 3x^{2} - 3", "f''(x) = 6x", "f'''(x) = 6"]), "{derivatives:?}");
    // The monotony has its sign table.
    let monotony = sketch.pages.iter().find(|page| page.title == "Monotonie").unwrap();
    assert!(monotony.lines.iter().any(|line| matches!(line, Line::Grid(rows) if rows.len() == 3)));
    // H(−1 | 2) and T(1 | −2) are marked on the curve.
    let graph = &sketch.graph;
    let max = graph.points.iter().find(|mark| mark.kind == MarkKind::Max).unwrap();
    let min = graph.points.iter().find(|mark| mark.kind == MarkKind::Min).unwrap();
    assert_eq!((max.x, max.y, min.x, min.y), (-1.0, 2.0, 1.0, -2.0));
    assert!(graph.window.x_min < -1.0 && graph.window.x_max > 1.0);
    assert_eq!(graph.curve.len(), 1);
}

#[test]
fn a_periodic_sketch_marks_every_extremum_in_the_window() {
    use quantus_ui::backend::MarkKind;
    let mut engine = Engine::default();
    let sketch = sketch(engine.solve(&Settings::default(), &Request::Analysis { f: r"\sin(x)".into() }).unwrap());
    let maxima = sketch.graph.points.iter().filter(|mark| mark.kind == MarkKind::Max).count();
    assert!(maxima >= 2, "{maxima}");
}

#[test]
fn a_sketch_of_two_variables_says_why_not() {
    let mut engine = Engine::default();
    assert!(engine.solve(&Settings::default(), &Request::Analysis { f: "x+y".into() }).is_err());
}

fn task(reply: Reply) -> quantus_ui::backend::Task {
    match reply {
        Reply::Task(task) => task,
        other => panic!("a task expected: {other:?}"),
    }
}

#[test]
fn a_tangent_gives_its_steps_and_both_lines() {
    use quantus_ui::backend::Straight;
    let mut engine = Engine::default();
    let request = Request::Tangent { f: "x^3-3x".into(), x0: "2".into() };
    let task = task(engine.solve(&Settings::default(), &request).unwrap());
    let math = math(&task.lines);
    assert!(math.ends_with(&["t(x) = 9x - 16", r"n(x) = -\frac{1}{9}x + \frac{20}{9}"]), "{math:?}");
    assert_eq!(task.overlay.tangent, Some(Straight::Sloped { slope: 9.0, intercept: -16.0 }));
    assert!(task.overlay.normal.is_some());
    let point = task.overlay.point.unwrap();
    assert_eq!((point.x, point.y), (2.0, 2.0));
}

#[test]
fn an_area_between_the_outer_zeros() {
    let mut engine = Engine::default();
    let request = Request::Area { f: "x^3-3x".into(), from: "".into(), to: "".into() };
    let task = task(engine.solve(&Settings::default(), &request).unwrap());
    assert!(math(&task.lines).contains(&r"A = \frac{9}{2}"));
    let regions = &task.overlay.regions;
    assert_eq!(regions.len(), 2);
    assert!(regions[0].positive && !regions[1].positive);
    assert!(!regions[0].curve.is_empty());
}

#[test]
fn a_task_off_the_curve_says_why() {
    let mut engine = Engine::default();
    let tangent = Request::Tangent { f: r"\sqrt{x}".into(), x0: "-1".into() };
    assert!(engine.solve(&Settings::default(), &tangent).is_err());
    let area = Request::Area { f: "x^2+1".into(), from: "".into(), to: "".into() };
    assert!(engine.solve(&Settings::default(), &area).is_err());
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
