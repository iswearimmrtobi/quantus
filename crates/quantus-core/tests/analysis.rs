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
