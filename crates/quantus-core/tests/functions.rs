use std::collections::HashMap;

use quantus_core::eval::LatexCalculator;
use quantus_core::settings::{Mode, Settings};

fn evaluate(mode: Mode, latex: &str) -> String {
    let settings = Settings { mode, exact_roots: true, ..Settings::default() };
    LatexCalculator::new(12)
        .with_settings(settings)
        .evaluate(latex, &HashMap::new())
        .map(|c| c.result)
        .unwrap_or_else(|e| format!("error: {e}"))
}

fn physics(latex: &str) -> String {
    evaluate(Mode::Physics, latex)
}

#[test]
fn area_functions_and_coth() {
    assert_eq!(physics(r"\operatorname{arsinh}(0)"), "0");
    assert_eq!(physics(r"\operatorname{arsinh} 1"), "0{,}88137358702");
    assert_eq!(physics(r"\sinh^{-1}(1)"), "0{,}88137358702");
    assert_eq!(physics(r"\operatorname{arcosh}(2)"), "1{,}31695789692");
    assert_eq!(physics(r"\tanh^{-1}\frac{1}{2}"), "0{,}549306144334");
    assert_eq!(physics(r"\operatorname{artanh}(0{,}5)"), "0{,}549306144334");
    assert_eq!(physics(r"\coth 1"), "1{,}3130352855");
    assert!(physics(r"\operatorname{arcosh}(0)").starts_with("error"));
    assert!(physics(r"\operatorname{artanh}(1)").starts_with("error"));
    assert!(physics(r"\coth 0").starts_with("error"));
}

#[test]
fn area_functions_differentiate() {
    // d/dx arsinh x = 1/√(x²+1), at 0 it is 1.
    assert_eq!(physics(r"\left.\frac{d}{dx}\operatorname{arsinh}(x)\right|_{x=0}"), "1");
    assert_eq!(physics(r"\left.\frac{d}{dx}\operatorname{artanh}(x)\right|_{x=0}"), "1");
}

#[test]
fn the_input_is_written_back_with_operatorname() {
    let settings = Settings { mode: Mode::Physics, ..Settings::default() };
    let calculation = LatexCalculator::new(10).with_settings(settings).evaluate(r"\cosh^{-1}(3)", &HashMap::new()).unwrap();
    assert_eq!(calculation.input, r"\operatorname{arcosh}{3}");
}

#[test]
fn products_multiply_their_terms() {
    assert_eq!(physics(r"\prod_{i=1}^{5} i"), "120");
    assert_eq!(physics(r"\prod_{k=1}^{3} \frac{k}{k+1}"), "0{,}25");
    assert_eq!(physics(r"\prod_{i=1}^{0} i"), "1");
    assert_eq!(physics(r"\prod_{i=1}^{2} 2\,\mathrm{m}"), r"4\,\mathrm{m}^{2}");
    assert_eq!(physics(r"\sum_{i=1}^{3} i + \prod_{i=1}^{3} i"), "12");
    let settings = Settings { mode: Mode::Physics, ..Settings::default() };
    let input = LatexCalculator::new(10).with_settings(settings).evaluate(r"\prod_{i=1}^{4} i", &HashMap::new()).unwrap().input;
    assert_eq!(input, r"\prod_{i=1}^{4} i");
}

#[test]
fn integer_functions() {
    assert_eq!(physics(r"\operatorname{nPr}(5; 2)"), "20");
    assert_eq!(physics(r"\operatorname{ggT}(12; 18)"), "6");
    assert_eq!(physics(r"\gcd(12; 18; 8)"), "2");
    assert_eq!(physics(r"\operatorname{kgV}(4; 6)"), "12");
    assert_eq!(physics(r"\operatorname{lcm}(4; 6; 10)"), "60");
    assert_eq!(physics(r"\operatorname{Int}(-2{,}7)"), "-2");
    assert_eq!(physics(r"\operatorname{Intg}(-2{,}7)"), "-3");
    assert_eq!(physics(r"\operatorname{RndFix}(\pi; 3)"), "3{,}142");
    // Rnd keeps the ten digits Norm shows, so the product is not 2 again.
    assert_eq!(physics(r"\operatorname{Rnd}(\frac{2}{3}) \cdot 3"), "2{,}0000000001");
    assert_eq!(physics(r"\operatorname{divR}(13; 4)"), r"Q = 3,\; R = 1");
    assert_eq!(physics(r"\operatorname{divR}(13; 4) + 1"), "4");
    assert!(physics(r"\operatorname{nPr}(2; 5)").starts_with("error"));
}

#[test]
fn rnd_follows_the_number_format() {
    let settings = Settings { number_format: quantus_core::settings::NumberFormat::Fix { digits: 2 }, ..Settings::default() };
    let result = LatexCalculator::new(20)
        .with_settings(settings)
        .evaluate(r"\operatorname{Rnd}(\frac{1}{3}) \cdot 3", &HashMap::new())
        .unwrap()
        .value
        .to_plain_string();
    assert_eq!(result, "0.99");
}

#[test]
fn the_casio_constants_work_in_physics_only() {
    assert_eq!(physics(r"Z_{0}"), r"376{,}730313668\,\mathrm{\Omega }");
    assert_eq!(physics(r"\frac{m_{p}}{m_{e}}"), "1836{,}15267344");
    assert_eq!(physics(r"\alpha_{f}"), "0{,}0072973525693");
    assert!(evaluate(Mode::Algebra, r"Z_{0} + 1").contains("Z"));
}
