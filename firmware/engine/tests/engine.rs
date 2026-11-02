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
