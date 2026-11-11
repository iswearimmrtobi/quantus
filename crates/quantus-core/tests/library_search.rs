//! The quick search over constants, conversion units, functions and prefixes.

use quantus_core::api::{library_index, search_library, HitKind, LibraryHit, LibrarySearchRequest};
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

#[test]
fn roots_are_found_by_wurzel_and_sqrt() {
    let hits: Vec<String> = search("wurz", Language::De).into_iter().map(|hit| hit.latex).collect();
    assert!(hits.contains(&r"\sqrt{}".to_string()), "{hits:?}");
    assert!(hits.contains(&r"\sqrt[]{}".to_string()), "{hits:?}");
    for (query, language) in [("sqrt", Language::En), ("square root", Language::En), ("quadratwurzel", Language::De)] {
        let hit = first(query, language);
        assert_eq!((hit.kind, hit.latex.as_str()), (HitKind::Function, r"\sqrt{}"), "{query}");
    }
}

#[test]
fn functions_are_found_by_symbol_and_either_name() {
    assert_eq!(first("ggt", Language::De).latex, r"\operatorname{ggT}(;)");
    assert_eq!(first("gcd", Language::De).latex, r"\operatorname{ggT}(;)");
    assert_eq!(first("greatest common", Language::En).latex, r"\operatorname{ggT}(;)");
    assert_eq!(first("sin", Language::En).latex, r"\sin()");
    assert_eq!(first("ableitung", Language::De).latex, r"\frac{d}{dx}");
    assert_eq!(first("Sinus", Language::De).detail, "Trigonometrie");
}

#[test]
fn prefixes_are_found_by_name() {
    let kilo = first("kilo", Language::En);
    assert_eq!((kilo.kind, kilo.latex.as_str(), kilo.detail.as_str()), (HitKind::Prefix, "k", "10^{3}"));
    assert_eq!(first("mikro", Language::De).latex, r"\mu ");
    assert_eq!(first("micro", Language::De).latex, r"\mu ");
}

#[test]
fn the_index_holds_every_kind_with_its_names() {
    let index = library_index(Language::De);
    for kind in [HitKind::Constant, HitKind::Unit, HitKind::Function, HitKind::Prefix] {
        assert!(index.iter().any(|entry| entry.hit.kind == kind), "{kind:?}");
    }
    let root = index.iter().find(|entry| entry.hit.latex == r"\sqrt{}").unwrap();
    assert_eq!(root.names, ["Quadratwurzel", "Square root", "Wurzel", "root"]);
    assert_eq!(root.symbols, ["sqrt", "√"]);
}
