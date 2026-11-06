//! The quick search over constants and conversion units.

use quantus_core::api::{search_library, HitKind, LibraryHit, LibrarySearchRequest};
use quantus_core::settings::Language;

fn search(query: &str, language: Language) -> Vec<LibraryHit> {
    search_library(language, &LibrarySearchRequest { query: query.to_string(), limit: None })
}

fn first(query: &str, language: Language) -> LibraryHit {
    search(query, language).into_iter().next().unwrap_or_else(|| panic!("nothing found for {query}"))
}

#[test]
fn a_misspelt_start_finds_stefan_boltzmann_first() {
    for query in ["steff", "stefan", "stefan boltz", "sigma_SB"] {
        assert_eq!(first(query, Language::En).latex, r"\sigma_{SB}", "{query}");
    }
}

#[test]
fn a_later_word_of_the_name_is_found() {
    let hits = search("boltz", Language::En);
    assert!(hits.iter().any(|hit| hit.latex == r"\sigma_{SB}"));
    assert!(hits.iter().all(|hit| hit.name.to_lowercase().contains("boltzmann")));
}

#[test]
fn units_are_found_by_name_in_either_language() {
    let inch = first("inch", Language::En);
    assert_eq!((inch.kind, inch.latex.as_str()), (HitKind::Unit, r"\mathrm{in}"));
    assert_eq!(first("zoll", Language::De).latex, r"\mathrm{in}");
    assert_eq!(first("inch", Language::De).latex, r"\mathrm{in}");
    assert_eq!(first("fuss", Language::De).latex, r"\mathrm{ft}");
}

#[test]
fn a_symbol_comes_before_names_that_contain_it() {
    assert_eq!(first("c", Language::En).latex, "c_{0}");
    assert_eq!(first("km/h", Language::En).latex, r"\frac{\mathrm{km}}{\mathrm{h}}");
    assert_eq!(first("mph", Language::En).latex, r"\mathrm{mph}");
}

#[test]
fn german_finds_by_german_and_english_name() {
    assert!(first("avogadro", Language::De).name.starts_with("Avogadro"));
    assert_eq!(first("lichtgeschw", Language::De).latex, "c_{0}");
}

#[test]
fn nothing_for_an_empty_or_unknown_query() {
    assert!(search("", Language::En).is_empty());
    assert!(search("   ", Language::En).is_empty());
    assert!(search("xyzzyq", Language::En).is_empty());
}

#[test]
fn a_unit_in_two_categories_comes_once() {
    let watts = search("watt", Language::En).into_iter().filter(|hit| hit.latex == r"\mathrm{W}").count();
    assert_eq!(watts, 1);
}
