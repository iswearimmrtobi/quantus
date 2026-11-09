//! The units the handheld's soft keys offer for a Physik input, by the input before the cursor
//! (firmware/docs/unit-keys.md §2).

use quantus_core::api::{unit_suggestions, UnitSuggestionRequest};

/// The labels of the first row, then of the second.
fn offered(before: &str, recent: &[&str], result: Option<&str>) -> (Vec<String>, Vec<String>) {
    let request = UnitSuggestionRequest {
        before: before.to_string(),
        recent: recent.iter().map(|r| r.to_string()).collect(),
        result: result.map(str::to_string),
    };
    let suggestions = unit_suggestions(&request);
    let labels = |list: Vec<quantus_core::api::UnitSuggestion>| list.into_iter().map(|s| s.label).collect();
    (labels(suggestions.best), labels(suggestions.more))
}

fn first(before: &str, count: usize) -> Vec<String> {
    offered(before, &[], None).0.into_iter().take(count).collect()
}

#[test]
fn a_sum_offers_the_units_of_the_left_side() {
    assert_eq!(first(r"3kW+", 3), ["W", "kW", "MW"]);
    assert_eq!(first(r"3\,\mathrm{kW}-", 3), ["W", "kW", "MW"]);
    assert_eq!(first(r"2\cdot 5N+", 2), ["N", "kN"]);
    assert_eq!(first(r"\frac{12\,\mathrm{m}}{3\,\mathrm{s}}+", 2), ["m/s", "km/h"]);
    assert_eq!(first(r"x=4\,\mathrm{V}+", 2), ["V", "kV"]);
}

#[test]
fn a_product_offers_what_makes_a_named_quantity() {
    assert_eq!(first(r"3kW\cdot ", 3), ["h", "s", "min"]);
    assert_eq!(first(r"5N\cdot ", 2), ["m", "s"]);
    assert_eq!(first(r"12\,\mathrm{m}\div ", 1), ["s"]);
    assert_eq!(first(r"1+(230V\cdot ", 2), ["A", "s"]);
}

#[test]
fn a_product_has_the_relatives_in_the_second_row() {
    let (best, more) = offered(r"5N\cdot ", &[], None);
    assert_eq!(best, ["m", "s"]);
    assert!(more.starts_with(&["km".to_string(), "cm".to_string()]), "{more:?}");
    assert!(more.contains(&"min".to_string()));
}

#[test]
fn a_letter_before_the_equals_sign_names_its_quantity() {
    assert_eq!(first("P=", 1), ["W"]);
    assert_eq!(first("U=", 1), ["V"]);
    assert_eq!(first("F=", 1), ["N"]);
    assert_eq!(first(r"\rho =", 1), ["kg/m³"]);
    assert_eq!(first("F_{G}=", 1), ["N"]);
}

#[test]
fn the_units_used_last_come_first() {
    assert_eq!(offered("3kW+", &["kW"], None).0[..2], ["kW", "W"]);
    assert_eq!(offered("", &["kN", "mA"], None).0[..3], ["kN", "mA", "m"]);
}

#[test]
fn without_anything_to_go_by_the_common_units() {
    assert_eq!(first("", 4), ["m", "s", "kg", "N"]);
    assert_eq!(first("3", 4), ["m", "s", "kg", "N"]);
    assert_eq!(first("3+", 4), ["m", "s", "kg", "N"]);
}

#[test]
fn a_result_offers_its_conversion_targets() {
    let (best, _) = offered("", &[], Some(r"1{,}5\,\mathrm{kJ}"));
    assert_eq!(best[..4], ["J", "kJ", "MJ", "eV"]);
    assert!(best.contains(&"kWh".to_string()));
    assert!(offered("", &[], Some("42")).0.is_empty());
}

#[test]
fn every_suggestion_is_offered_once() {
    let (best, more) = offered(r"3kW\cdot ", &["h"], None);
    let all: Vec<&String> = best.iter().chain(&more).collect();
    let mut unique = all.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(all.len(), unique.len(), "{all:?}");
}
