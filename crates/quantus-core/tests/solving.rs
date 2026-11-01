use std::collections::HashMap;

use quantus_core::eval::LatexCalculator;
use quantus_core::settings::{Mode, Settings};

fn solve(latex: &str, exact_roots: bool, mode: Mode) -> String {
    let settings = Settings { mode, exact_roots, ..Settings::default() };
    LatexCalculator::new(10)
        .with_settings(settings)
        .evaluate(latex, &HashMap::new())
        .map(|c| c.result)
        .unwrap_or_else(|e| format!("error: {e}"))
}

#[test]
fn cubics_and_quartics_are_exact() {
    assert_eq!(solve("x^3 - 7x + 6 = 0", true, Mode::Algebra), r"x = -3 \lor x = 1 \lor x = 2");
    assert_eq!(solve("2x^3 - x^2 - 4x + 2 = 0", true, Mode::Algebra), r"x = -\sqrt{2} \lor x = 0{,}5 \lor x = \sqrt{2}");
    assert_eq!(solve("x^3 = 2", false, Mode::Algebra), r"x = 1{,}25992105");
    assert_eq!(solve("x^4 - 5x^2 + 4 = 0", true, Mode::Algebra), r"x = -2 \lor x = -1 \lor x = 1 \lor x = 2");
    assert_eq!(solve("x^4 = 4", true, Mode::Algebra), r"x = -\sqrt{2} \lor x = \sqrt{2}");
}

#[test]
fn three_real_irrational_roots_come_as_decimals() {
    assert_eq!(solve("x^3 - 3x + 1 = 0", true, Mode::Algebra), r"x = -1{,}879385242 \lor x = 0{,}3472963553 \lor x = 1{,}532088886");
}

#[test]
fn complex_roots_of_a_cubic() {
    assert_eq!(solve("x^3 = 1", true, Mode::Imaginary), r"x = -\frac{1}{2} - \frac{\sqrt{3}}{2}i \lor x = -\frac{1}{2} + \frac{\sqrt{3}}{2}i \lor x = 1");
}

#[test]
fn polynomials_have_no_search_interval() {
    assert_eq!(solve("x^5 - 1000x^4 = 1", false, Mode::Algebra), "x = 1000");
    assert_eq!(solve("x^3 = 1000000", true, Mode::Algebra), "x = 100");
    assert_eq!(solve("x^3 - 300x^2 = 0", true, Mode::Algebra), r"x = 0 \lor x = 300");
    assert!(solve("x^4 + 1 = 0", false, Mode::Algebra).contains("no real solution"));
}

#[test]
fn the_golden_cubic_cases_still_hold() {
    let settings = Settings { mode: Mode::Physics, ..Settings::default() };
    let calculator = LatexCalculator::new(50).with_settings(settings);
    assert_eq!(calculator.evaluate("x^3 = 8", &HashMap::new()).unwrap().result, "x = 2");
    assert!(calculator.evaluate("x^3 - 2 = 0", &HashMap::new()).unwrap().result.starts_with("x = 1{,}25992104989487316476"));
}

mod calc_and_solve {
    use quantus_core::api::{self, Assignment, CalcRequest, CalculationRequest, SolveRequest};
    use quantus_core::session::SessionStore;
    use quantus_core::settings::{Mode, Settings, StepDetail};

    fn store(mode: Mode) -> SessionStore {
        let mut store = SessionStore::new();
        api::set_settings(&mut store, Settings { mode, step_detail: StepDetail::Standard, exact_roots: true, ..Settings::default() }).unwrap();
        store
    }

    fn assign(key: &str, latex: &str) -> Assignment {
        Assignment { key: key.into(), latex: latex.into() }
    }

    #[test]
    fn calc_asks_for_the_free_letters_and_substitutes_them() {
        let mut store = store(Mode::Algebra);
        let free = api::free_variables(&mut store, &CalculationRequest { latex: "a x^2 + b".into(), ..Default::default() }).unwrap();
        let keys: Vec<_> = free.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(keys, ["a", "x", "b"]);
        let request = CalcRequest {
            latex: "a x^2 + b".into(),
            values: vec![assign("a", "2"), assign("x", "3"), assign("b", "1")],
            ..Default::default()
        };
        let response = api::calc(&mut store, &request).unwrap();
        assert_eq!(response.result, "19");
        assert!(response.steps.iter().any(|s| s.kind == "substituted"), "{:?}", response.steps);
        // The values stay in the session.
        let free = api::free_variables(&mut store, &CalculationRequest { latex: "a x + c".into(), ..Default::default() }).unwrap();
        assert_eq!(free.iter().map(|f| f.key.as_str()).collect::<Vec<_>>(), ["c"]);
    }

    #[test]
    fn solve_for_a_named_letter_with_the_others_given() {
        let mut store = store(Mode::Physics);
        let request = SolveRequest {
            latex: r"F_1 = m_1 a_1".into(),
            variable: "a_1".into(),
            values: vec![assign("F_1", r"10\,\mathrm{N}"), assign("m_1", r"2\,\mathrm{kg}")],
            ..Default::default()
        };
        let response = api::solve_for(&mut store, &request).unwrap();
        assert_eq!(response.result, r"a_1 = 5\,\frac{\mathrm{N}}{\mathrm{kg}}");
        assert!(response.steps.iter().any(|s| s.latex.starts_with("L - R = 0")), "{:?}", response.steps);
    }

    #[test]
    fn solve_needs_the_other_letters_and_takes_a_start_value() {
        let mut store = store(Mode::Algebra);
        let missing = SolveRequest { latex: "a x = 6".into(), variable: "x".into(), ..Default::default() };
        assert!(api::solve_for(&mut store, &missing).unwrap_err().error.contains("Give a value to a"));
        // An expression is solved as = 0; cos x = x has no exact method, so Newton runs from 1.
        let newton = SolveRequest { latex: r"\cos x - x".into(), variable: "x".into(), start: Some("1".into()), ..Default::default() };
        assert!(api::solve_for(&mut store, &newton).unwrap().result.starts_with("x = 0{,}73908513321516064165"));
    }
}
