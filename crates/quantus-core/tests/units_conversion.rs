use std::collections::HashMap;

use quantus_core::eval::LatexCalculator;
use quantus_core::settings::{Mode, Settings};

fn physics(latex: &str) -> String {
    let settings = Settings { mode: Mode::Physics, ..Settings::default() };
    LatexCalculator::new(12)
        .with_settings(settings)
        .evaluate(latex, &HashMap::new())
        .map(|c| c.result)
        .unwrap_or_else(|e| format!("error: {e}"))
}

#[test]
fn imperial_units_have_their_exact_definitions() {
    assert_eq!(physics(r"\frac{1\,\mathrm{mi}}{1\,\mathrm{km}}"), "1{,}609344");
    assert_eq!(physics(r"\frac{3\,\mathrm{ft}}{1\,\mathrm{yd}}"), "1");
    assert_eq!(physics(r"\frac{1\,\mathrm{lb}}{1\,\mathrm{kg}}"), "0{,}45359237");
    assert_eq!(physics(r"\frac{16\,\mathrm{oz}}{1\,\mathrm{lb}}"), "1");
    assert_eq!(physics(r"\frac{1\,\mathrm{gal}}{1\,\mathrm{l}}"), "3{,}785411784");
    assert_eq!(physics(r"\frac{1\,\mathrm{psi}}{1\,\mathrm{Pa}}"), "6894{,}75729317");
    assert_eq!(physics(r"\frac{1\,\mathrm{hp}}{1\,\mathrm{W}}"), "745{,}699871582");
    assert_eq!(physics(r"\frac{1\,\mathrm{Btu}}{1\,\mathrm{J}}"), "1055{,}05585262");
    assert_eq!(physics(r"\frac{1\,\mathrm{kn}}{1\,\mathrm{km}/\mathrm{h}}"), "1{,}852");
    assert_eq!(physics(r"\frac{1\,\mathrm{nmi}}{1\,\mathrm{m}}"), "1852");
    assert_eq!(physics(r"\frac{1\,\mathrm{acre}}{1\,\mathrm{m}^2}"), "4046{,}8564224");
}

#[test]
fn fahrenheit_is_an_offset_unit_like_celsius() {
    assert_eq!(physics(r"68\,^\circ\mathrm{F}"), r"68\,^\circ\mathrm{F}");
    assert_eq!(physics(r"68°F"), r"68\,^\circ\mathrm{F}");
    // Like Celsius, it does not multiply.
    assert!(physics(r"\frac{50\,^\circ\mathrm{F}}{1\,\mathrm{K}}").starts_with("error"));
}
