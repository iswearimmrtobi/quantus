use std::collections::HashMap;

use quantus_core::eval::LatexCalculator;
use quantus_core::settings::{Mode, NumberFormat, Settings};

fn evaluate(mode: Mode, number_format: NumberFormat, latex: &str) -> String {
    let settings = Settings { mode, number_format, ..Settings::default() };
    LatexCalculator::new(20)
        .with_settings(settings)
        .evaluate(latex, &HashMap::new())
        .map(|c| c.result)
        .unwrap_or_else(|e| format!("error: {e}"))
}

#[test]
fn the_default_keeps_todays_output() {
    assert_eq!(evaluate(Mode::Physics, NumberFormat::Auto, r"\frac{1}{8}"), "0{,}125");
    assert_eq!(evaluate(Mode::Physics, NumberFormat::Auto, r"0{,}001"), "0{,}001");
}

#[test]
fn fix_and_sci_shape_the_shown_digits() {
    assert_eq!(evaluate(Mode::Physics, NumberFormat::Fix { digits: 3 }, r"\frac{2}{3}"), "0{,}667");
    assert_eq!(evaluate(Mode::Physics, NumberFormat::Fix { digits: 2 }, r"2\,\mathrm{m} \cdot 3"), r"6{,}00\,\mathrm{m}");
    assert_eq!(evaluate(Mode::Physics, NumberFormat::Sci { digits: 4 }, r"\frac{1}{7}"), r"1{,}429 \cdot 10^{-1}");
    assert_eq!(evaluate(Mode::Physics, NumberFormat::Norm1, r"0{,}001"), r"1 \cdot 10^{-3}");
}

#[test]
fn complex_parts_share_the_format() {
    assert_eq!(evaluate(Mode::Imaginary, NumberFormat::Fix { digits: 1 }, r"\frac{1}{3}+\frac{2}{3}i"), "0{,}3 + 0{,}7i");
}

#[test]
fn the_value_keeps_its_full_precision() {
    let settings = Settings { number_format: NumberFormat::Fix { digits: 2 }, ..Settings::default() };
    let calculation = LatexCalculator::new(20).with_settings(settings).evaluate(r"\frac{2}{3}", &HashMap::new()).unwrap();
    assert_eq!(calculation.value.to_plain_string(), "0.66666666666666666667");
}

#[test]
fn decimal_prefixes_apply_to_plain_numbers_only() {
    let with = |latex: &str| {
        let settings = Settings { decimal_prefixes: true, ..Settings::default() };
        LatexCalculator::new(20).with_settings(settings).evaluate(latex, &HashMap::new()).unwrap().result
    };
    assert_eq!(with(r"4{,}7 \cdot 1000"), r"4{,}7\,\mathrm{k}");
    assert_eq!(with(r"\frac{1}{4}"), r"250\,\mathrm{m}");
    assert_eq!(with(r"4700\,\mathrm{m}"), r"4700\,\mathrm{m}");
}
