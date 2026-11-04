//! Inequalities in one unknown: polynomial and rational, solved by a sign chart.

use std::collections::HashMap;

use quantus_core::eval::LatexCalculator;
use quantus_core::settings::{Mode, Settings};

fn evaluate_with(settings: Settings, latex: &str) -> Result<String, String> {
    LatexCalculator::new(50).with_settings(settings).evaluate(latex, &HashMap::new()).map(|c| c.result).map_err(|e| e.to_string())
}

fn solve(latex: &str) -> String {
    evaluate_with(Settings::default(), latex).unwrap_or_else(|e| panic!("{latex}: {e}"))
}

fn error(latex: &str) -> String {
    evaluate_with(Settings::default(), latex).expect_err(latex)
}

#[test]
fn quadratic_and_linear_inequalities() {
    assert_eq!(solve(r"x^2 - 4 < 0"), r"-2 < x < 2");
    assert_eq!(solve(r"x^2 - x - 6 \ge 0"), r"x \le -2 \lor x \ge 3");
    assert_eq!(solve(r"2x + 3 > 7"), r"x > 2");
    assert_eq!(solve(r"-3x \le 6"), r"x \ge -2");
    assert_eq!(solve(r"3x^2 - 4x < 0"), r"0 < x < \frac{4}{3}");
}

#[test]
fn rational_inequalities_leave_out_the_poles() {
    assert_eq!(solve(r"\frac{x+1}{x-2} \ge 0"), r"x \le -1 \lor x > 2");
    assert_eq!(solve(r"\frac{1}{x} < \frac{1}{2}"), r"x < 0 \lor x > 2");
    // The cancelled factor still has its pole.
    assert_eq!(solve(r"\frac{(x-1)^2}{x-1} \ge 0"), r"x > 1");
}

#[test]
fn chains_single_points_and_the_whole_line() {
    assert_eq!(solve(r"1 < 2x + 1 \le 5"), r"0 < x \le 2");
    assert_eq!(solve(r"x^2 \le 0"), r"x = 0");
    assert_eq!(solve(r"x^2 \neq 4"), r"x < -2 \lor -2 < x < 2 \lor x > 2");
    assert_eq!(solve(r"x^2 + 1 > 0"), r"x \in \mathbb{R}");
    assert!(error(r"x^2 + 1 < 0").contains("no solution"));
}

#[test]
fn what_is_not_solved() {
    assert!(error(r"|x| < 1").contains("polynomial and rational"));
    assert!(error(r"x + z < 1").contains("more than one unknown"));
    assert!(error(r"2x > 4\,\mathrm{m}").contains("units"));
    // Verify mode checks; a free letter there stays an error.
    let verify = Settings { mode: Mode::Verify, ..Settings::default() };
    assert!(evaluate_with(verify, r"x < 2").is_err());
}
