use quantus_engine::Engine;
use quantus_ui::app::{Mode, Settings};
use quantus_ui::backend::Backend;

#[test]
fn the_engine_answers_through_the_backend() {
    let mut engine = Engine::default();
    let settings = Settings::default();
    assert_eq!(engine.evaluate(Mode::Algebra, &settings, "2+3"), Ok("5".to_string()));
    assert_eq!(engine.start(Mode::Algebra, &settings, "2^{10}"), Some(Ok("1024".to_string())));
}

#[test]
fn a_decimal_result_is_marked() {
    let mut engine = Engine::default();
    let settings = Settings { decimal: true, ..Settings::default() };
    let result = engine.evaluate(Mode::Algebra, &settings, r"\frac{1}{3}").expect("a result");
    assert!(result.starts_with(r"\approx 0{,}333"), "{result}");
}

#[test]
fn modes_without_the_engine_say_so() {
    let mut engine = Engine::default();
    assert!(engine.evaluate(Mode::Matrix, &Settings::default(), "1").is_err());
}

#[test]
fn an_exact_rational_is_a_fraction_and_s_d_makes_it_a_decimal() {
    let mut engine = Engine::default();
    let exact = Settings::default();
    let decimal = Settings { decimal: true, ..exact };
    let input = r"\frac{1}{2}+\frac{1}{3}";
    assert_eq!(engine.evaluate(Mode::Algebra, &exact, input), Ok(r"\frac{5}{6}".to_string()));
    let result = engine.evaluate(Mode::Algebra, &decimal, input).expect("a result");
    assert!(result.starts_with(r"\approx 0{,}8333"), "{result}");
    // Integers stay as they are.
    assert_eq!(engine.evaluate(Mode::Algebra, &exact, r"\frac{6}{3}"), Ok("2".to_string()));
}

#[test]
fn the_fraction_is_mixed_if_the_setting_says_so() {
    let mut engine = Engine::default();
    let settings = Settings { fraction: quantus_ui::settings::FractionForm::Mixed, ..Settings::default() };
    assert_eq!(engine.evaluate(Mode::Algebra, &settings, r"\frac{7}{3}"), Ok(r"2\frac{1}{3}".to_string()));
}

#[test]
fn a_fraction_keeps_its_unit() {
    let mut engine = Engine::default();
    let result = engine.evaluate(Mode::Physics, &Settings::default(), r"\frac{1}{3}\,\mathrm{m}").expect("a result");
    assert!(result.starts_with(r"\frac{1}{3}\,"), "{result}");
}

#[test]
fn a_long_fraction_stays_a_decimal() {
    let mut engine = Engine::default();
    let result = engine.evaluate(Mode::Algebra, &Settings::default(), r"\frac{1}{7}+\frac{1}{123456}").expect("a result");
    assert!(result.starts_with("0{,}14"), "{result}");
}

#[test]
fn a_measured_value_gives_a_fraction_too_as_on_a_casio() {
    let mut engine = Engine::default();
    let latex = r"\frac{1}{2} \cdot 68{,}7\,\mathrm{g} \cdot \left(16\,\frac{\mathrm{km}}{\mathrm{h}}\right)^{2}";
    let result = engine.evaluate(Mode::Physics, &Settings::default(), latex);
    assert_eq!(result, Ok(r"\frac{458}{675}\,\mathrm{J}".to_string()));
}
