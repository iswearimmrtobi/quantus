//! The analysis mode end to end: the function in, each section's result and the marked points
//! out, for the cases of `docs/analysis-mode.md` (§17).

use quantus_core::analysis::{analyze, Asymptote, Discussion};
use quantus_core::settings::{Language, QuadraticFormula, Settings};

fn sketch(latex: &str) -> Discussion {
    analyze(latex, None, &Settings::default(), None).unwrap()
}

fn result(d: &Discussion, id: &str) -> String {
    d.sections.iter().find(|s| s.id == id).unwrap().result.clone().unwrap_or_default()
}

fn status(d: &Discussion, id: &str) -> &'static str {
    d.sections.iter().find(|s| s.id == id).unwrap().status.name()
}

fn points(d: &Discussion, kind: &str) -> Vec<String> {
    d.points.iter().filter(|p| p.kind.name() == kind).map(|p| p.latex.clone()).collect()
}

#[test]
fn a_cubic() {
    let d = sketch("x^3-3x");
    assert_eq!(d.function, "f(x) = x^{3} - 3x");
    assert_eq!(result(&d, "derivatives"), "f'(x) = 3x^{2} - 3, \\quad f''(x) = 6x, \\quad f'''(x) = 6");
    assert_eq!(result(&d, "domain"), "D = \\mathbb{R}");
    assert_eq!(result(&d, "symmetry"), "\\text{Symmetric about the origin}");
    assert_eq!(
        points(&d, "zero"),
        [
            "Z_{1}\\left(-\\sqrt{3} \\mid 0\\right) \\approx Z_{1}\\left(-1{,}73205 \\mid 0\\right)",
            "Z_{2}\\left(0 \\mid 0\\right)",
            "Z_{3}\\left(\\sqrt{3} \\mid 0\\right) \\approx Z_{3}\\left(1{,}73205 \\mid 0\\right)"
        ]
    );
    assert_eq!(points(&d, "max"), ["Max\\left(-1 \\mid 2\\right)"]);
    assert_eq!(points(&d, "min"), ["Min\\left(1 \\mid -2\\right)"]);
    assert_eq!(points(&d, "inflection"), ["I\\left(0 \\mid 0\\right)"]);
    assert_eq!(result(&d, "monotonicity"), "\\text{strictly increasing on}: (-\\infty, -1], [1, \\infty), \\quad \\text{strictly decreasing on}: [-1, 1]");
    assert_eq!(result(&d, "antiderivative"), "F(x) = \\frac{1}{4}x^{4} - \\frac{3}{2}x^{2} + C");
    assert!(d.asymptotes.is_empty());
}

#[test]
fn a_quartic_touches_the_axis() {
    let d = sketch("x^4-2x^2");
    assert_eq!(result(&d, "symmetry"), "\\text{Symmetric about the y-axis}");
    assert!(result(&d, "zeros").contains("\\left(0 \\mid 0\\right) \\ \\text{(double root: touches the axis)}"));
    assert_eq!(points(&d, "min"), ["Min_{1}\\left(-1 \\mid -1\\right)", "Min_{2}\\left(1 \\mid -1\\right)"]);
    assert_eq!(points(&d, "max"), ["Max\\left(0 \\mid 0\\right)"]);
    assert_eq!(
        points(&d, "inflection")[1],
        "I_{2}\\left(\\frac{\\sqrt{3}}{3} \\mid -\\frac{5}{9}\\right) \\approx I_{2}\\left(0{,}57735 \\mid -0{,}555556\\right)"
    );
}

#[test]
fn a_cubic_with_guessed_roots() {
    let d = sketch("x^3-6x^2+11x-6");
    let zeros = d.sections.iter().find(|s| s.id == "zeros").unwrap();
    assert!(zeros.steps.iter().any(|s| s.rule.as_deref() == Some("RATIONAL_ROOT_GUESS")));
    assert!(zeros.steps.iter().any(|s| s.latex == "(x^{3} - 6x^{2} + 11x - 6) : (x - 1) = x^{2} - 5x + 6"));
    assert_eq!(d.points.iter().filter(|p| p.kind.name() == "zero").map(|p| p.x).collect::<Vec<_>>(), [1.0, 2.0, 3.0]);
}

#[test]
fn x_times_e_to_the_x() {
    let d = sketch("x e^x");
    assert_eq!(result(&d, "derivatives"), "f'(x) = (x + 1)e^{x}, \\quad f''(x) = (x + 2)e^{x}, \\quad f'''(x) = (x + 3)e^{x}");
    assert_eq!(points(&d, "min"), ["Min\\left(-1 \\mid -\\frac{1}{e}\\right) \\approx Min\\left(-1 \\mid -0{,}367879\\right)"]);
    assert_eq!(points(&d, "inflection"), ["I\\left(-2 \\mid -\\frac{2}{e^{2}}\\right) \\approx I\\left(-2 \\mid -0{,}270671\\right)"]);
    assert_eq!(result(&d, "behaviour"), "\\text{horizontal asymptote} y = 0 \\ \\text{(for x → −∞)}");
    assert_eq!(result(&d, "antiderivative"), "F(x) = (x - 1)e^{x} + C");
}

#[test]
fn a_rational_function_with_a_pole_and_an_oblique_asymptote() {
    let d = sketch("\\frac{x^2-1}{x-2}");
    assert_eq!(result(&d, "domain"), "D = \\mathbb{R} \\setminus \\{2\\}");
    assert_eq!(points(&d, "yIntercept"), ["S_y\\left(0 \\mid \\frac{1}{2}\\right)"]);
    assert_eq!(points(&d, "max"), ["Max\\left(2 - \\sqrt{3} \\mid 4 - 2\\sqrt{3}\\right) \\approx Max\\left(0{,}267949 \\mid 0{,}535898\\right)"]);
    assert_eq!(points(&d, "min"), ["Min\\left(2 + \\sqrt{3} \\mid 4 + 2\\sqrt{3}\\right) \\approx Min\\left(3{,}73205 \\mid 7{,}4641\\right)"]);
    assert_eq!(result(&d, "behaviour"), "\\text{vertical asymptote} x = 2, \\quad \\text{oblique asymptote} y = x + 2");
    let behaviour = d.sections.iter().find(|s| s.id == "behaviour").unwrap();
    assert!(behaviour.steps.iter().any(|s| s.explanation.as_deref() == Some("Pole with a sign change")));
    assert!(matches!(d.asymptotes[1], Asymptote::Oblique { slope, intercept, .. } if slope == 1.0 && intercept == 2.0));
    assert_eq!(result(&d, "antiderivative"), "F(x) = \\frac{1}{2}x^{2} + 2x + 3\\ln\\left|x - 2\\right| + C");
}

#[test]
fn a_removable_gap_is_a_hole() {
    let d = sketch("\\frac{x^2-1}{x-1}");
    assert_eq!(d.function, "f(x) = \\frac{x^{2} - 1}{x - 1}");
    assert_eq!(points(&d, "hole"), ["G\\left(1 \\mid 2\\right)"]);
    assert!(d.asymptotes.is_empty());
}

#[test]
fn the_logarithm_over_x() {
    let d = sketch("\\frac{\\ln x}{x}");
    assert_eq!(result(&d, "domain"), "D = (0, \\infty)");
    assert_eq!(points(&d, "max"), ["Max\\left(e \\mid \\frac{1}{e}\\right) \\approx Max\\left(2{,}71828 \\mid 0{,}367879\\right)"]);
    assert_eq!(
        points(&d, "inflection"),
        ["I\\left(e^{\\frac{3}{2}} \\mid \\frac{3}{2e^{\\frac{3}{2}}}\\right) \\approx I\\left(4{,}48169 \\mid 0{,}334695\\right)"]
    );
    assert_eq!(result(&d, "behaviour"), "\\text{vertical asymptote} x = 0, \\quad \\text{horizontal asymptote} y = 0 \\ \\text{(for x → ∞)}");
    assert_eq!(result(&d, "antiderivative"), "F(x) = \\frac{1}{2}\\ln^{2}(x) + C");
    let antiderivative = d.sections.iter().find(|s| s.id == "antiderivative").unwrap();
    assert!(antiderivative.steps.iter().any(|s| s.rule.as_deref() == Some("INTEGRAL_SUBSTITUTION")));
}

#[test]
fn sine_has_periodic_points() {
    let d = sketch("\\sin x");
    assert_eq!(d.period.as_deref(), Some("2\\pi"));
    assert!(result(&d, "zeros").ends_with("x = k\\pi, \\ k \\in \\mathbb{Z}"));
    assert_eq!(
        result(&d, "extrema"),
        "Max\\left(\\frac{\\pi}{2} + 2k\\pi \\mid 1\\right), \\ k \\in \\mathbb{Z}, \\quad Min\\left(\\frac{3\\pi}{2} + 2k\\pi \\mid -1\\right), \\ k \\in \\mathbb{Z}"
    );
    assert_eq!(points(&d, "max").len(), 2);
    assert_eq!(result(&d, "antiderivative"), "F(x) = -\\cos(x) + C");
    assert!(d.sections.iter().find(|s| s.id == "behaviour").unwrap().steps[0].latex.contains("oscillates"));
}

#[test]
fn x_plus_sine_has_saddle_points() {
    let d = sketch("x + \\sin x");
    assert_eq!(points(&d, "saddle")[2], "S_{3}\\left(\\pi \\mid \\pi\\right) \\approx S_{3}\\left(3{,}14159 \\mid 3{,}14159\\right)");
    assert_eq!(result(&d, "monotonicity"), "\\text{strictly increasing on}: (-\\infty, \\infty)");
}

#[test]
fn tangent_has_periodic_poles() {
    let d = sketch("\\tan x");
    assert_eq!(result(&d, "domain"), "D = \\mathbb{R} \\setminus \\{\\frac{\\pi}{2} + k\\pi \\mid k \\in \\mathbb{Z}\\}");
    assert_eq!(result(&d, "behaviour"), "\\text{vertical asymptotes} x = \\frac{\\pi}{2} + k\\pi, \\ k \\in \\mathbb{Z}");
    assert_eq!(d.asymptotes.len(), 2);
}

#[test]
fn x_cubed_has_a_saddle_point_and_no_extremum() {
    let d = sketch("x^3");
    assert_eq!(result(&d, "extrema"), "\\text{No extrema}");
    let extrema = d.sections.iter().find(|s| s.id == "extrema").unwrap();
    assert!(extrema.steps.iter().any(|s| s.rule.as_deref() == Some("SIGN_CHANGE_TEST")));
    assert_eq!(points(&d, "saddle"), ["S\\left(0 \\mid 0\\right)"]);
}

#[test]
fn kinks_and_cusps_are_extrema_where_f_prime_is_undefined() {
    assert_eq!(points(&sketch("|x|"), "min"), ["Min\\left(0 \\mid 0\\right)"]);
    assert_eq!(points(&sketch("\\sqrt[3]{x^2}"), "min"), ["Min\\left(0 \\mid 0\\right)"]);
}

#[test]
fn a_closed_end_of_the_domain_is_a_boundary_point() {
    let d = sketch("\\sqrt{x}");
    assert_eq!(result(&d, "domain"), "D = [0, \\infty)");
    assert_eq!(points(&d, "boundaryMin"), ["B\\left(0 \\mid 0\\right)"]);
}

#[test]
fn arctan_has_two_horizontal_asymptotes() {
    let d = sketch("\\arctan x");
    assert_eq!(
        result(&d, "behaviour"),
        "\\text{horizontal asymptote} y = -\\frac{\\pi}{2} \\ \\text{(for x → −∞)}, \\quad \\text{horizontal asymptote} y = \\frac{\\pi}{2} \\ \\text{(for x → ∞)}"
    );
}

#[test]
fn the_witch_of_agnesi() {
    let d = sketch("\\frac{1}{x^2+1}");
    assert_eq!(points(&d, "max"), ["Max\\left(0 \\mid 1\\right)"]);
    assert_eq!(points(&d, "inflection")[0], "I_{1}\\left(-\\frac{\\sqrt{3}}{3} \\mid \\frac{3}{4}\\right) \\approx I_{1}\\left(-0{,}57735 \\mid 0{,}75\\right)");
    assert_eq!(result(&d, "behaviour"), "\\text{horizontal asymptote} y = 0");
    assert_eq!(result(&d, "antiderivative"), "F(x) = \\arctan(x) + C");
}

#[test]
fn exact_and_missing_antiderivatives() {
    let zero = sketch("e^{x}-2");
    assert_eq!(points(&zero, "zero"), ["Z\\left(\\ln(2) \\mid 0\\right) \\approx Z\\left(0{,}693147 \\mid 0\\right)"]);
    let gauss = sketch("e^{x^2}");
    assert_eq!(status(&gauss, "antiderivative"), "partial");
    assert_eq!(result(&gauss, "antiderivative"), "\\text{No elementary antiderivative found}");
}

#[test]
fn the_pq_formula_on_request() {
    let settings = Settings { quadratic_formula: QuadraticFormula::Pq, ..Settings::default() };
    let d = analyze("x^2-2x-3", None, &settings, None).unwrap();
    let zeros = d.sections.iter().find(|s| s.id == "zeros").unwrap();
    assert!(zeros.steps.iter().any(|s| s.rule.as_deref() == Some("QUADRATIC_PQ") && s.explanation.as_deref() == Some("pq formula with p = -2, q = -3")));
    assert_eq!(d.points.iter().filter(|p| p.kind.name() == "zero").map(|p| p.x).collect::<Vec<_>>(), [-1.0, 3.0]);
}

#[test]
fn german_names_and_notation() {
    let settings = Settings { language: Language::De, ..Settings::default() };
    let d = analyze("f(x) = \\frac{\\ln x}{x}", None, &settings, None).unwrap();
    assert_eq!(d.sections[1].title, "Definitionsbereich");
    assert_eq!(result(&d, "domain"), "D = ]0; \\infty[");
    assert_eq!(points(&d, "max"), ["H\\left(e \\mid \\frac{1}{e}\\right) \\approx H\\left(2{,}71828 \\mid 0{,}367879\\right)"]);
    assert_eq!(points(&d, "zero"), ["N\\left(1 \\mid 0\\right)"]);
    assert_eq!(result(&d, "monotonicity"), "\\text{streng monoton steigend auf}: ]0; e], \\quad \\text{streng monoton fallend auf}: [e; \\infty[");
}

#[test]
fn inputs_are_read_with_or_without_a_name() {
    for input in ["f(x) = x^2", "f\\left(x\\right)=x^2", "y = x^2", "x^2"] {
        assert_eq!(sketch(input).function, "f(x) = x^{2}", "{input}");
    }
    let error = analyze("f(x) = x^2 +", None, &Settings::default(), None).unwrap_err();
    assert_eq!(error.position(), analyze("x^2 +", None, &Settings::default(), None).unwrap_err().position().map(|p| p + 7));
    assert!(analyze("x^2 + a", None, &Settings::default(), None).unwrap_err().to_string().contains("function families"));
    assert!(analyze("\\ln(-x^2 - 1)", None, &Settings::default(), None).unwrap_err().to_string().contains("defined nowhere"));
}

#[test]
fn every_sketch_answers_quickly() {
    for input in ["x^3-3x", "\\frac{x^2-1}{x-2}", "\\frac{\\ln x}{x}", "\\tan x", "x^2 e^{-x}", "x \\ln x", "\\frac{x^3}{x^2-4}"] {
        let start = std::time::Instant::now();
        let d = sketch(input);
        assert!(start.elapsed().as_millis() < 1000, "{input}");
        assert!(d.sections.iter().all(|s| s.status.name() != "failed"), "{input}");
    }
}

#[test]
fn constant_functions_say_so() {
    let d = sketch("3");
    for id in ["extrema", "inflection", "monotonicity", "curvature"] {
        assert_eq!(result(&d, id), "\\text{f is constant}", "{id}");
        assert_eq!(status(&d, id), "ok", "{id}");
    }
}

#[test]
fn boundary_points_share_their_letter() {
    let d = sketch("x\\sqrt{1-x^2}");
    assert_eq!(points(&d, "boundaryMax"), ["B_{1}\\left(-1 \\mid 0\\right)"]);
    assert_eq!(points(&d, "boundaryMin"), ["B_{2}\\left(1 \\mid 0\\right)"]);
}

#[test]
fn notes_on_zeros_and_the_y_axis() {
    assert!(result(&sketch("x^3"), "zeros").contains("\\text{(triple root: crosses the axis)}"));
    assert!(result(&sketch("x^4"), "zeros").contains("\\text{(4-fold root: touches the axis)}"));
    assert_eq!(result(&sketch("\\ln x"), "yIntercept"), "\\text{0 ∉ D: no y-intercept}");
    let extrema = sketch("x^3");
    let extrema = extrema.sections.iter().find(|s| s.id == "extrema").unwrap();
    assert!(extrema.steps.iter().any(|s| s.explanation.as_deref().is_some_and(|e| e.ends_with("(saddle candidate)"))));
}

#[test]
fn periodic_families_merge_and_move() {
    let sine = sketch("\\sin x");
    assert_eq!(result(&sine, "inflection"), "I\\left(k\\pi \\mid 0\\right), \\ k \\in \\mathbb{Z}");
    assert_eq!(result(&sine, "symmetry"), "\\text{Symmetric about the origin}");
    let behaviour = sine.sections.iter().find(|s| s.id == "behaviour").unwrap();
    assert!(behaviour.steps.iter().take(2).all(|s| s.latex.contains("oscillates")));
    assert!(sine.points.iter().filter(|p| p.kind.name() != "yIntercept").all(|p| p.period.is_some()));
    let shifted = sketch("x + \\sin x");
    assert_eq!(result(&shifted, "inflection").split(", \\quad ").last().unwrap(), "S\\left(\\pi + 2k\\pi \\mid \\pi + 2k\\pi\\right), \\ k \\in \\mathbb{Z}");
    assert!(shifted.points.iter().all(|p| p.period.is_none()));
    let tangent = sketch("\\tan x");
    assert!(tangent.asymptotes.iter().all(|a| matches!(a, Asymptote::Vertical { period: Some(p), .. } if (p - std::f64::consts::PI).abs() < 1e-12)));
}

/// The rows of §17 that the tests above leave out.
#[test]
fn the_remaining_rows_of_the_specification() {
    let cubic = sketch("x^3-3x");
    assert_eq!(result(&cubic, "curvature"), "\\text{left-curved (convex) on}: [0, \\infty), \\quad \\text{right-curved (concave) on}: (-\\infty, 0]");

    let quartic = sketch("x^4-2x^2");
    let zeros: Vec<f64> = quartic.points.iter().filter(|p| p.kind.name() == "zero").map(|p| p.x).collect();
    assert_eq!(zeros.len(), 3);
    assert!((zeros[0] + 2f64.sqrt()).abs() < 1e-12 && (zeros[2] - 2f64.sqrt()).abs() < 1e-12);
    assert!(points(&quartic, "zero")[0].starts_with("Z_{1}\\left(-\\sqrt{2} \\mid 0\\right)"));
    let zero_steps = &quartic.sections.iter().find(|s| s.id == "zeros").unwrap().steps;
    assert!(zero_steps.iter().any(|s| s.rule.as_deref() == Some("FACTOR_OUT")));
    assert_eq!(
        points(&quartic, "inflection")[0],
        "I_{1}\\left(-\\frac{\\sqrt{3}}{3} \\mid -\\frac{5}{9}\\right) \\approx I_{1}\\left(-0{,}57735 \\mid -0{,}555556\\right)"
    );

    let exp = sketch("x e^x");
    let behaviour = exp.sections.iter().find(|s| s.id == "behaviour").unwrap();
    assert!(behaviour.steps.iter().any(|s| s.latex == "\\lim_{x \\to \\infty} f(x) = \\infty"));
    let antiderivative = exp.sections.iter().find(|s| s.id == "antiderivative").unwrap();
    assert!(antiderivative.steps.iter().any(|s| s.rule.as_deref() == Some("INTEGRATION_BY_PARTS")));

    let log = sketch("\\frac{\\ln x}{x}");
    let behaviour = log.sections.iter().find(|s| s.id == "behaviour").unwrap();
    assert!(behaviour.steps.iter().any(|s| s.latex == "\\lim_{x \\to 0^{+}} f(x) = -\\infty"));
    assert!(behaviour.steps.iter().any(|s| s.rule.as_deref() == Some("LIMIT_DOMINANCE")));
    assert_eq!(points(&log, "zero"), ["Z\\left(1 \\mid 0\\right)"]);

    assert!(result(&sketch("x^3"), "zeros").starts_with("Z\\left(0 \\mid 0\\right) \\ \\text{(triple root"));
    assert_eq!(points(&sketch("x^{2/3}"), "min"), ["Min\\left(0 \\mid 0\\right)"]);
    assert_eq!(
        points(&sketch("\\frac{1}{x^2+1}"), "inflection")[1],
        "I_{2}\\left(\\frac{\\sqrt{3}}{3} \\mid \\frac{3}{4}\\right) \\approx I_{2}\\left(0{,}57735 \\mid 0{,}75\\right)"
    );
    // The minima of \sin x inside the window [-2π, 2π]: -π/2 and 3π/2.
    let minima: Vec<f64> = sketch("\\sin x").points.iter().filter(|p| p.kind.name() == "min").map(|p| p.x).collect();
    assert_eq!(minima.len(), 2);
    assert!((minima[0] + std::f64::consts::FRAC_PI_2).abs() < 1e-12);
}

/// Every antiderivative the tests expect is differentiated back to f.
#[test]
fn every_antiderivative_differentiates_back() {
    use quantus_core::analysis::derive::derivative;
    use quantus_core::analysis::domain::{domain, DomainContext};
    use quantus_core::analysis::input::parse_function;
    use quantus_core::analysis::integrate::{antiderivative, IntegrateContext};
    use quantus_core::analysis::simplify::{is_zero, is_zero_numeric, simplify};
    use quantus_core::analysis::Expr;

    let settings = Settings::default();
    for input in ["x^3-3x", "x e^x", "\\frac{x^2-1}{x-2}", "\\frac{\\ln x}{x}", "\\sin x", "\\frac{1}{x^2+1}", "\\arctan x", "x^4-2x^2", "\\sqrt{x}", "e^{x}-2"] {
        let (f, var) = parse_function(input, None, &settings).unwrap();
        let (d, _) = domain(&f, &DomainContext { settings: &settings, var: &var, digits: 6 }).unwrap();
        let found = antiderivative(&simplify(&f), &IntegrateContext { settings: &settings, var: &var, domain: &d }).unwrap_or_else(|| panic!("{input}"));
        let back = simplify(&Expr::Sum(vec![derivative(&found.result), Expr::neg(f.clone())]));
        assert!(is_zero(&back) || is_zero_numeric(&back, &d.sample_points(9)), "{input}");
    }
}

fn steps(d: &Discussion, id: &str) -> Vec<String> {
    d.sections.iter().find(|s| s.id == id).unwrap().steps.iter().map(|s| s.latex.clone()).collect()
}

#[test]
fn every_solution_has_its_step() {
    // x = 0 from the factor x is a step of its own, not only a result.
    let extrema = steps(&sketch("x^3-3x^2+2"), "extrema");
    assert!(extrema.contains(&"x = 0 \\lor 3x - 6 = 0".to_string()), "{extrema:?}");
    // The equation is not repeated as a "Factor out" step when it is already shown factored.
    let extrema = steps(&sketch("x e^{2x}"), "extrema");
    assert_eq!(extrema.iter().filter(|s| s.as_str() == "(2x + 1)e^{2x} = 0").count(), 0, "{extrema:?}");
    // No `0 + k \cdot 2\pi`.
    let zeros = steps(&sketch("\\sin(x^2)"), "zeros");
    assert!(zeros.iter().any(|s| s.starts_with("x^{2} = k \\cdot 2\\pi")), "{zeros:?}");
}

#[test]
fn steps_without_a_change_are_left_out() {
    let d = sketch("\\frac{x^2+1}{x-1}");
    let derivatives = steps(&d, "derivatives");
    assert!(derivatives.iter().any(|s| s.ends_with("{(x - 1)^{4}}")), "{derivatives:?}");
    assert!(!derivatives.iter().any(|s| s.contains("((x - 1)^{2})^{2}")), "{derivatives:?}");
    let antiderivative = steps(&d, "antiderivative");
    assert!(!antiderivative.contains(&"\\frac{2}{x - 1} = \\frac{2}{x - 1}".to_string()), "{antiderivative:?}");
}

#[test]
#[ignore]
fn show_steps() {
    let input = std::env::var("INPUT").unwrap_or_default();
    let section = std::env::var("SECTION").unwrap_or_else(|_| "derivatives".into());
    let d = sketch(&input);
    for step in &d.sections.iter().find(|s| s.id == section).unwrap().steps {
        println!("{:?}\t{}\t[{}]", step.level, step.latex, step.explanation.clone().unwrap_or_default());
    }
}

/// The working of a section as shown at `detail`: `(latex, explanation)`.
fn working(d: &Discussion, id: &str, detail: quantus_core::settings::StepDetail) -> Vec<(String, String)> {
    let steps = &d.sections.iter().find(|s| s.id == id).unwrap().steps;
    quantus_core::eval::calculator::shown_steps(steps, detail)
        .into_iter()
        .map(|s| (s.latex, s.explanation.unwrap_or_default()))
        .collect()
}

#[test]
fn simplifying_a_derivative_shows_the_algebra_from_standard_on() {
    use quantus_core::settings::StepDetail;
    let d = sketch(r"\frac{x^2+1}{x-1}");
    let brief = working(&d, "derivatives", StepDetail::Brief);
    let standard = working(&d, "derivatives", StepDetail::Standard);
    assert!(standard.len() > brief.len());
    assert!(standard.contains(&(
        r"f'(x) = \frac{2x^{2} - 2x - x^{2} - 1}{(x - 1)^{2}}".to_string(),
        "A minus sign in front of a bracket flips every sign inside: -(x^{2} + 1) = -x^{2} - 1".to_string()
    )));
    // Each run of lines ends in the Simplify line of Brief.
    let simplify = standard.iter().position(|(_, text)| text == "Simplify").unwrap();
    assert_eq!(standard[simplify].0, r"f'(x) = \frac{x^{2} - 2x - 1}{(x - 1)^{2}}");
    let detailed = working(&d, "derivatives", StepDetail::Detailed);
    assert!(detailed.iter().any(|(latex, _)| latex == r"f''(x) = \frac{4 \cdot \cancel{(x - 1)}}{(x - 1)^{3} \cdot \cancel{(x - 1)}}"));
}

#[test]
fn the_sum_rule_writes_each_term_before_the_power_rule() {
    use quantus_core::settings::StepDetail;
    let d = sketch("x^3-3x^2+2");
    let detailed = working(&d, "derivatives", StepDetail::Detailed);
    assert_eq!(detailed[0], ("f'(x) = (x^{3})' - 3(x^{2})' + (2)'".to_string(), "Term by term".to_string()));
    assert_eq!(detailed[1].0, r"f'(x) = 3x^{3 - 1} - 3 \cdot 2x^{2 - 1} + 0");
    assert_eq!(working(&d, "derivatives", StepDetail::Standard).len(), detailed.len() - 2);
    assert_eq!(working(&d, "derivatives", StepDetail::Brief)[0].0, "f'(x) = 3x^{2} - 6x + 0");
}

#[test]
fn the_product_rule_shows_the_chain_rule_of_its_parts_and_puts_them_in() {
    use quantus_core::settings::StepDetail;
    let d = sketch("x e^{2x}");
    let detailed = working(&d, "derivatives", StepDetail::Detailed);
    assert_eq!(detailed[0], (r"v' = e^{2x} \cdot 2".to_string(), "v' by the chain rule: outer e^{u}, inner u = 2x, u' = 2".to_string()));
    assert_eq!(detailed[2], (r"f'(x) = 1 \cdot e^{2x} + x \cdot 2e^{2x}".to_string(), "Put in".to_string()));
    assert!(!working(&d, "derivatives", StepDetail::Standard).iter().any(|(_, text)| text == "Put in"));

    let d = sketch(r"\sin(x^2)");
    let standard = working(&d, "derivatives", StepDetail::Standard);
    assert!(standard.iter().any(|(latex, text)| latex == r"f''(x) = 2 \cdot \left(x\cos(x^{2})\right)'" && text == "Constant factor rule: 2 stays in front"));
}

#[test]
fn the_chain_rule_writes_a_power_and_multiplies_outer_by_inner() {
    use quantus_core::settings::StepDetail;
    let d = sketch(r"\frac{x^2+1}{x-1}");
    let standard = working(&d, "derivatives", StepDetail::Standard);
    let at = standard.iter().position(|(_, text)| text == "Write as a power").unwrap();
    assert_eq!(standard[at].0, "f''(x) = 4(x - 1)^{-3}");
    assert_eq!(standard[at + 1].1, "Inner function");
    assert_eq!(standard[at + 2], (r"f'''(x) = 4 \cdot \left(-3\right)(x - 1)^{-4} \cdot 1".to_string(), "Outer derivative times inner derivative".to_string()));
}

#[test]
fn the_quadratic_formula_computes_the_discriminant_and_divides() {
    use quantus_core::settings::StepDetail;
    let d = sketch("x^3-3x^2+2");
    let detailed = working(&d, "zeros", StepDetail::Detailed);
    let at = detailed.iter().position(|(_, text)| text == "Discriminant").unwrap();
    assert_eq!(detailed[at].0, r"b^{2} - 4ac = \left(-2\right)^{2} - 4 \cdot 1 \cdot \left(-2\right) = 4 + 8 = 12");
    assert_eq!(detailed[at + 2], (r"\sqrt{12} = \sqrt{4 \cdot 3} = 2\sqrt{3}".to_string(), "Take the square out of the root".to_string()));
    assert_eq!(detailed[at + 3], (r"x_{1,2} = 1 \pm \sqrt{3}".to_string(), "Divide by 2".to_string()));
    assert!(!working(&d, "zeros", StepDetail::Standard).iter().any(|(_, text)| text == "Take the square out of the root"));

    let pq = Settings { quadratic_formula: QuadraticFormula::Pq, ..Settings::default() };
    let d = analyze("x^3-3x^2+2", None, &pq, None).unwrap();
    let standard = working(&d, "zeros", StepDetail::Standard);
    assert!(standard.contains(&(r"\left(\frac{p}{2}\right)^{2} - q = \left(-1\right)^{2} - \left(-2\right) = 1 + 2 = 3".to_string(), "Radicand".to_string())));
}

#[test]
fn a_linear_equation_isolates_the_term_first() {
    use quantus_core::settings::StepDetail;
    let d = sketch("x^3-3x^2+2");
    let detailed = working(&d, "extrema", StepDetail::Detailed);
    let at = detailed.iter().position(|(latex, _)| latex == "3x = 6").unwrap();
    assert_eq!(detailed[at].1, "Add 6");
    assert_eq!(detailed[at + 1].0, "x = 2");
    assert!(!working(&d, "extrema", StepDetail::Standard).iter().any(|(latex, _)| latex == "3x = 6"));
}

#[test]
fn an_even_power_of_x_bounds_k_and_takes_the_root() {
    use quantus_core::settings::StepDetail;
    let d = sketch(r"\sin(x^2)");
    let standard = working(&d, "zeros", StepDetail::Standard);
    assert!(standard.contains(&(r"x = \pm \sqrt{k \cdot 2\pi}, \quad k \in \mathbb{N}_{0}".to_string(), "x^{2} ≥ 0, so k ≥ 0; take the root".to_string())));
    assert_eq!(working(&d, "zeros", StepDetail::Brief).len(), standard.len() - 2);
}

#[test]
#[ignore]
fn show_tangent() {
    use quantus_core::settings::StepDetail;
    let r = quantus_core::analysis::tasks::tangent(&std::env::var("INPUT").unwrap(), None, &std::env::var("X0").unwrap_or("1".into()), &Settings::default(), None).unwrap();
    for detail in [StepDetail::Brief, StepDetail::Standard, StepDetail::Maximal] {
        println!("--- {detail:?}");
        for s in quantus_core::eval::calculator::shown_steps(&r.steps, detail) {
            println!("{}\t[{}]", s.latex, s.explanation.unwrap_or_default());
        }
    }
}

fn task_working(steps: &[quantus_core::eval::Step], detail: quantus_core::settings::StepDetail) -> Vec<(String, String)> {
    quantus_core::eval::calculator::shown_steps(steps, detail).into_iter().map(|s| (s.latex, s.explanation.unwrap_or_default())).collect()
}

#[test]
fn the_tangent_puts_x0_in_and_multiplies_out() {
    use quantus_core::settings::StepDetail;
    let r = quantus_core::analysis::tasks::tangent("x^2-3x", None, "1", &Settings::default(), None).unwrap();
    let brief = task_working(&r.steps, StepDetail::Brief);
    let standard = task_working(&r.steps, StepDetail::Standard);
    assert_eq!(brief[0].0, "f(1) = -2");
    assert_eq!(standard[0].0, r"f(1) = 1^{2} - 3 \cdot 1 = -2");
    assert_eq!(standard[1].0, r"f'(x) = 2x - 3, \quad m = f'(1) = 2 \cdot 1 - 3 = -1");
    assert!(standard.contains(&("t(x) = -x + 1 - 2".to_string(), "A minus sign in front of a bracket flips every sign inside: -(x - 1) = -x + 1".to_string())));
    assert!(standard.contains(&(r"-\frac{1}{m} = -\frac{1}{-1} = 1".to_string(), "Slope of the normal".to_string())));
    assert_eq!(standard.len(), brief.len() + 2);
}

#[test]
fn an_area_puts_the_bounds_into_the_antiderivative() {
    use quantus_core::settings::StepDetail;
    let r = quantus_core::analysis::tasks::area("x^2-1", None, Some("0"), Some("2"), &Settings::default(), None).unwrap();
    let standard = task_working(&r.steps, StepDetail::Standard);
    let at = standard.iter().position(|(_, text)| text == "Upper minus lower bound").unwrap();
    assert!(standard[at].0.starts_with("A_{1} = F(1) - F(0) = "));
    assert!(standard[at + 1].0.starts_with("A_{1} = "));
    assert_eq!(standard.len(), task_working(&r.steps, StepDetail::Brief).len() + 2);
}

#[test]
fn limits_show_the_highest_power_and_the_signs_at_a_pole() {
    use quantus_core::settings::StepDetail;
    let d = sketch("x^3-3x^2+2");
    let standard = working(&d, "behaviour", StepDetail::Standard);
    assert_eq!(standard[0], (r"f(x) = x^{3}\left(1 - \frac{3}{x} + \frac{2}{x^{3}}\right)".to_string(), "Factor out the highest power".to_string()));
    assert_eq!(standard[1].0, r"\lim_{x \to -\infty} x^{3} \cdot 1 = -\infty");

    let d = sketch(r"\frac{x^2+1}{x-1}");
    let standard = working(&d, "behaviour", StepDetail::Standard);
    assert!(standard.contains(&("1^{2} + 1 = 2 > 0".to_string(), "The numerator at 1".to_string())));
    assert!(standard.contains(&(r"x \to 1^{-}: x - 1 \to 0^{-}, \quad x \to 1^{+}: x - 1 \to 0^{+}".to_string(), "Sign of the denominator".to_string())));

    let d = sketch(r"\frac{2x^2+1}{x^2-4}");
    let standard = working(&d, "behaviour", StepDetail::Standard);
    assert_eq!(standard[0].0, r"f(x) = \frac{2 + \frac{1}{x^{2}}}{1 - \frac{4}{x^{2}}}");
    assert!(working(&d, "behaviour", StepDetail::Brief).len() < standard.len());
}

#[test]
fn antiderivatives_show_the_rule_put_in() {
    use quantus_core::settings::StepDetail;
    let detailed = working(&sketch("x^3-3x^2+2"), "antiderivative", StepDetail::Detailed);
    assert_eq!(detailed[0].0, r"F(x) = \frac{x^{4}}{4} - 3 \cdot \frac{x^{3}}{3} + 2x + C");

    let standard = working(&sketch("x e^{2x}"), "antiderivative", StepDetail::Standard);
    assert_eq!(standard[1].0, r"F(x) = \frac{1}{2}xe^{2x} - \int \frac{1}{2}e^{2x} \, dx");
    assert_eq!(standard[2].0, r"F(x) = \frac{1}{2}xe^{2x} - \frac{1}{4}e^{2x} + C");

    let d = sketch(r"\frac{1}{x^2-1}");
    let detailed = working(&d, "antiderivative", StepDetail::Detailed);
    assert_eq!(detailed[0], (r"\frac{1}{x^{2} - 1} = \frac{A}{x - 1} + \frac{B}{x + 1}".to_string(), "Ansatz".to_string()));
    assert_eq!(detailed[1].0, r"1 = A(x + 1) + B(x - 1) \Rightarrow A = \frac{1}{2}, \ B = -\frac{1}{2}");

    let standard = working(&sketch("x e^{x^2}"), "antiderivative", StepDetail::Standard);
    assert_eq!(standard[1].0, r"\int xe^{x^{2}} \, dx = \int \frac{1}{2}e^{u} \, du");
}

#[test]
fn polynomial_division_multiplies_back_before_subtracting() {
    use quantus_core::settings::StepDetail;
    let d = sketch("x^3-3x^2+2");
    let detailed = working(&d, "zeros", StepDetail::Detailed);
    let at = detailed.iter().position(|(_, text)| text == "Multiply back").unwrap();
    assert_eq!(detailed[at].0, r"x^{2} \cdot (x - 1) = x^{3} - x^{2}");
    assert_eq!(detailed[at + 1].1, "Partial quotient x^{2}");
    assert!(!working(&d, "zeros", StepDetail::Standard).iter().any(|(_, text)| text == "Multiply back"));
}

#[test]
fn the_second_derivative_test_and_the_y_values_put_x_in() {
    use quantus_core::settings::StepDetail;
    let d = sketch("x^3-3x^2+2");
    let brief = working(&d, "extrema", StepDetail::Brief);
    let standard = working(&d, "extrema", StepDetail::Standard);
    assert!(brief.iter().any(|(latex, _)| latex == "f''(0) = -6 < 0"));
    assert!(!standard.iter().any(|(latex, _)| latex == "f''(0) = -6 < 0"));
    assert!(standard.iter().any(|(latex, _)| latex == r"f''(0) = 6 \cdot 0 - 6 = -6 < 0"));
    assert!(standard.contains(&(r"f(2) = 2^{3} - 3 \cdot 2^{2} + 2 = -2".to_string(), "The y-value".to_string())));
    assert!(working(&d, "inflection", StepDetail::Standard).iter().any(|(latex, _)| latex == r"f(1) = 1^{3} - 3 \cdot 1^{2} + 2 = 0"));
}

#[test]
fn symmetry_and_the_y_intercept_put_the_values_in() {
    use quantus_core::settings::StepDetail;
    let d = sketch("x^3-3x^2+2");
    let standard = working(&d, "symmetry", StepDetail::Standard);
    assert_eq!(standard[0].0, r"f(-x) = (-x)^{3} - 3 \cdot (-x)^{2} + 2");
    assert_eq!(standard[2].0, r"f(x) = x^{3} - 3x^{2} + 2, \quad -f(x) = -x^{3} + 3x^{2} - 2");
    assert_eq!(working(&d, "symmetry", StepDetail::Brief).len(), 2);
    assert_eq!(working(&d, "yIntercept", StepDetail::Standard)[0].0, r"f(0) = 0^{3} - 3 \cdot 0^{2} + 2 = 2");
    assert_eq!(working(&d, "yIntercept", StepDetail::Brief)[0].0, "f(0) = 2");
}

#[test]
fn a_sign_condition_of_the_domain_solves_for_the_zeros_first() {
    use quantus_core::settings::StepDetail;
    let d = sketch(r"\sqrt{x^2-4}");
    let standard = working(&d, "domain", StepDetail::Standard);
    assert_eq!(standard[0], ("x^{2} - 4 = 0".to_string(), "Where the sign can change".to_string()));
    assert!(standard.contains(&(r"x_{1,2} = \pm 2".to_string(), "Take the root".to_string())));
    assert_eq!(working(&d, "domain", StepDetail::Brief).len(), 2);
}
