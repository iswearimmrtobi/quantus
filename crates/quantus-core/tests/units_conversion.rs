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

#[test]
fn the_arrow_writes_a_value_in_the_unit_after_it() {
    assert_eq!(physics(r"254\,\mathrm{cm} \blacktriangleright \mathrm{in}"), r"100\,\mathrm{in}");
    assert_eq!(physics(r"1\,\mathrm{mi} ▶ \mathrm{km}"), r"1{,}609344\,\mathrm{km}");
    assert_eq!(physics(r"36\,\frac{\mathrm{km}}{\mathrm{h}} \blacktriangleright \frac{\mathrm{m}}{\mathrm{s}}"), r"10\,\frac{\mathrm{m}}{\mathrm{s}}");
    assert_eq!(physics(r"50\,^\circ\mathrm{F} \blacktriangleright ^\circ\mathrm{C}"), r"10\,^\circ\mathrm{C}");
    assert_eq!(physics(r"100\,^\circ\mathrm{C} ▶ °F"), r"212\,^\circ\mathrm{F}");
    assert_eq!(physics(r"2\,\mathrm{m} \cdot 3\,\mathrm{m} \blacktriangleright \mathrm{ft}^{2}"), r"64{,}5834625003\,\mathrm{ft}^{2}");
    assert!(physics(r"1\,\mathrm{m} \blacktriangleright \mathrm{s}").starts_with("error: Cannot convert"));
}

#[test]
fn units_convert_in_the_algebra_mode_too() {
    let settings = Settings { mode: Mode::Algebra, ..Settings::default() };
    let result = LatexCalculator::new(12).with_settings(settings).evaluate(r"3\,\mathrm{ft} \blacktriangleright \mathrm{m}", &HashMap::new()).unwrap().result;
    assert_eq!(result, r"0{,}9144\,\mathrm{m}");
}
