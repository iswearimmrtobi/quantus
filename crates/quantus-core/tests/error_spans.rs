//! Errors that point at the part of the input they are about (feature idea 6).

use std::collections::HashMap;

use quantus_core::api::{self, CalculationRequest};
use quantus_core::eval::LatexCalculator;
use quantus_core::session::SessionStore;
use quantus_core::settings::{Mode, Settings};

/// The text of the input the error for `latex` marks, or `None` when it marks nothing.
fn marked(mode: Mode, latex: &str) -> Option<String> {
    let error = LatexCalculator::new(50)
        .with_settings(Settings { mode, ..Settings::default() })
        .evaluate(latex, &HashMap::new())
        .expect_err(latex);
    let span = error.span()?;
    Some(latex.chars().skip(span.start).take(span.end - span.start).collect())
}

fn physics(latex: &str) -> Option<String> {
    marked(Mode::Physics, latex)
}

#[test]
fn a_parse_error_marks_the_token_it_found() {
    assert_eq!(physics("2 \\cdot mgh").as_deref(), Some("mgh"));
    assert_eq!(physics("2 + \\foo").as_deref(), Some("\\foo"));
}

#[test]
fn something_missing_at_the_end_marks_the_end() {
    assert_eq!(physics("2 +").as_deref(), Some(""));
}

#[test]
fn modes_with_their_own_language_mark_nothing() {
    assert_eq!(marked(Mode::BaseN, "1 \\div 0"), None);
}

#[test]
fn the_api_reports_the_span_and_the_bare_reason() {
    let mut store = SessionStore::new();
    let request = CalculationRequest { latex: "2 \\cdot mgh".into(), ..Default::default() };
    let error = api::evaluate(&mut store, &request).unwrap_err();
    let span = error.span.expect("a span");
    assert_eq!((span.start, span.end), (8, 11));
    assert_eq!(error.position, Some(8));
    assert!(error.error.contains("position 9"), "{}", error.error);
    assert!(!error.reason.contains("position"), "{}", error.reason);
}

#[test]
fn adding_metres_to_seconds_marks_the_sum() {
    let input = "1\\,\\mathrm{m} + 2\\,\\mathrm{s} + 3";
    assert_eq!(physics(input).as_deref(), Some("1\\,\\mathrm{m} + 2\\,\\mathrm{s}"));
}

#[test]
fn a_division_by_zero_marks_the_denominator() {
    assert_eq!(physics("2 + \\frac{1}{3-3}").as_deref(), Some("3-3"));
    assert_eq!(physics("2 + 1 / (3-3)").as_deref(), Some("(3-3)"));
}

#[test]
fn a_symbolic_division_by_zero_marks_the_denominator() {
    assert_eq!(marked(Mode::Algebra, "x + \\frac{x}{0}").as_deref(), Some("0"));
}

#[test]
fn an_undefined_variable_is_marked() {
    assert_eq!(physics("2 \\cdot v_{1} + 3").as_deref(), Some("v_{1}"));
}

#[test]
fn a_unit_where_a_number_belongs_marks_the_argument() {
    assert_eq!(physics("1 + \\sin\\left(2\\,\\mathrm{m}\\right)").as_deref(), Some("\\left(2\\,\\mathrm{m}\\right)"));
}
