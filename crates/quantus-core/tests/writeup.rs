//! The Gegeben / Gesucht / Lösung writeup of a session.

use quantus_core::api::{self, CalculationRequest};
use quantus_core::session::SessionStore;
use quantus_core::settings::Language;
use quantus_core::writeup::{self, LineKind, Writeup};

fn writeup_of(inputs: &[&str], language: Language) -> Writeup {
    let mut store = SessionStore::new();
    store.settings.language = language;
    for input in inputs {
        let request = CalculationRequest { latex: input.to_string(), ..Default::default() };
        api::evaluate(&mut store, &request).unwrap_or_else(|e| panic!("{input}: {e}"));
    }
    let settings = store.settings.clone();
    writeup::build(store.session(None), &settings)
}

fn lines(writeup: &Writeup, block: usize) -> Vec<(LineKind, Option<String>, String)> {
    writeup.blocks[block].lines.iter().map(|l| (l.kind, l.text.clone(), l.latex.clone())).collect()
}

#[test]
fn ohms_law_with_the_values_typed_in() {
    let w = writeup_of(&["R = \\frac{10V}{0{,}5A}"], Language::De);
    assert_eq!(w.given, ["U = 10\\,\\mathrm{V}", "I = 0{,}5\\,\\mathrm{A}"]);
    assert_eq!(w.wanted, ["R"]);
    let lines = lines(&w, 0);
    assert_eq!(lines[0], (LineKind::Law, Some("Nach dem Ohm'schen Gesetz gilt:".into()), "U = R \\cdot I".into()));
    assert_eq!(lines[1], (LineKind::Rearranged, Some("Umgestellt nach R:".into()), "R = \\frac{U}{I}".into()));
    assert_eq!(lines[2].2, "R = \\frac{10\\,\\mathrm{V}}{0{,}5\\,\\mathrm{A}}");
    assert_eq!(lines[3].0, LineKind::Result);
    assert!(lines[3].2.starts_with("R = 20"), "{}", lines[3].2);
    assert_eq!(lines[4].0, LineKind::UnitCheck);
    assert!(lines[4].2.starts_with("[R] = \\frac{1\\,\\mathrm{V}}{1\\,\\mathrm{A}} = 1\\,"), "{}", lines[4].2);
}

#[test]
fn ohms_law_with_variables() {
    let w = writeup_of(&["U_1 = 10V", "I_1 = 0{,}5A", "R_1 = \\frac{U_1}{I_1}"], Language::De);
    assert_eq!(w.given, ["U_{1} = 10\\,\\mathrm{V}", "I_{1} = 0{,}5\\,\\mathrm{A}"]);
    assert_eq!(w.wanted, ["R_{1}"]);
    let lines = lines(&w, 0);
    assert_eq!(lines[1].2, "R_{1} = \\frac{U_{1}}{I_{1}}");
    assert_eq!(lines[2].2, "R_{1} = \\frac{10\\,\\mathrm{V}}{0{,}5\\,\\mathrm{A}}");
}

#[test]
fn a_result_used_later_is_an_intermediate_and_a_given_value_is_listed_once() {
    let w = writeup_of(&["U_1 = 10V", "I_1 = 0{,}5A", "R_1 = \\frac{U_1}{I_1}", "P_1 = \\frac{U_1^2}{R_1}"], Language::De);
    assert_eq!(w.given.len(), 2);
    assert_eq!(w.wanted, ["P_{1}"]);
    assert!(w.blocks[0].intermediate);
    assert_eq!(lines(&w, 1)[0].2, "P = \\frac{U^{2}}{R}");
    assert_eq!(lines(&w, 1)[2].2, "P_{1} = \\frac{\\left(10\\,\\mathrm{V}\\right)^{2}}{20\\,\\mathrm{\\Omega }}");
}

#[test]
fn two_unknowns_are_both_wanted() {
    let w = writeup_of(&["R = \\frac{10V}{0{,}5A}", "P = 10V \\cdot 0{,}5A"], Language::De);
    assert_eq!(w.given, ["U = 10\\,\\mathrm{V}", "I = 0{,}5\\,\\mathrm{A}"]);
    assert_eq!(w.wanted, ["R", "P"]);
}

#[test]
fn ans_refers_to_the_last_result() {
    let w = writeup_of(&["\\frac{10V}{0{,}5A}", "\\frac{(10V)^2}{\\mathrm{Ans}}"], Language::De);
    assert_eq!(w.wanted, ["P"]);
    assert_eq!(lines(&w, 1)[0].2, "P = \\frac{U^{2}}{R}");
    assert_eq!(lines(&w, 1)[1].0, LineKind::Substituted);
}

#[test]
fn the_unit_check_is_in_si_units_whatever_the_prefixes() {
    let w = writeup_of(&["R = \\frac{12kV}{3mA}"], Language::De);
    let check = lines(&w, 0).last().unwrap().2.clone();
    assert!(check.starts_with("[R] = \\frac{1\\,\\mathrm{V}}{1\\,\\mathrm{A}}"), "{check}");
}

#[test]
fn an_unknown_formula_falls_back_to_it_holds() {
    let w = writeup_of(&["x_1 = 3m \\cdot 4m \\cdot 5m \\cdot 6m"], Language::De);
    assert_eq!(lines(&w, 0)[0].1.as_deref(), Some("Es gilt:"));
}

#[test]
fn the_sentences_are_english_too() {
    let w = writeup_of(&["R = \\frac{10V}{0{,}5A}"], Language::En);
    let lines = lines(&w, 0);
    assert_eq!(lines[0].1.as_deref(), Some("By Ohm's law:"));
    assert_eq!(lines[1].1.as_deref(), Some("Solved for R:"));
}

#[test]
fn a_variable_from_the_panel_is_given_once_it_is_used() {
    let mut store = SessionStore::new();
    store.settings.language = Language::De;
    let define = CalculationRequest { latex: "U_1 = 10V".into(), ..Default::default() };
    api::define_variable(&mut store, &define).unwrap();
    let define = CalculationRequest { latex: "t_1 = 3s".into(), ..Default::default() };
    api::define_variable(&mut store, &define).unwrap();
    let request = CalculationRequest { latex: "R = \\frac{U_1}{0{,}5A}".into(), ..Default::default() };
    api::evaluate(&mut store, &request).unwrap();
    let settings = store.settings.clone();
    let w = writeup::build(store.session(None), &settings);
    assert_eq!(w.given, ["U_{1} = 10\\,\\mathrm{V}", "I = 0{,}5\\,\\mathrm{A}"]);
}

#[test]
fn a_sum_is_worked_out_first() {
    let w = writeup_of(&["I = \\frac{230V}{10\\Omega + 20\\Omega}"], Language::De);
    assert_eq!(w.given, ["U = 230\\,\\mathrm{V}", "R_{1} = 10\\,\\mathrm{\\Omega }", "R_{2} = 20\\,\\mathrm{\\Omega }"]);
    assert_eq!(w.wanted, ["I"]);
    assert_eq!(lines(&w, 0)[0].2, "R_{ges} = R_{1} + R_{2}");
    assert!(lines(&w, 0)[2].2.starts_with("R_{ges} = 30"));
    assert_eq!(lines(&w, 1)[1].2, "I = \\frac{U}{R_{ges}}");
}

#[test]
fn a_part_that_fits_a_law_of_its_own_is_split_off() {
    let w = writeup_of(&["p = \\frac{2kg \\cdot 9{,}81\\frac{m}{s^2}}{0{,}5m^2}"], Language::De);
    assert_eq!(w.blocks.len(), 2);
    assert_eq!(lines(&w, 0)[0].1.as_deref(), Some("Für die Gewichtskraft gilt:"));
    assert_eq!(lines(&w, 1)[0].1.as_deref(), Some("Für den Druck gilt:"));
    assert_eq!(lines(&w, 1)[1].2, "p = \\frac{F_{G}}{A}");
    assert_eq!(w.wanted, ["p"]);
}
