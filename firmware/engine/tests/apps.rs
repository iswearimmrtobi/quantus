//! Apps against the real engine (apps.md §4.6): `calc` leaves the history alone.

use quantus_engine::Engine;
use quantus_ui::app::{Mode, Settings};
use quantus_ui::backend::{Backend, SessionCommand};

/// How many calculations the open session has.
fn history(engine: &mut Engine) -> usize {
    engine.session(SessionCommand::Current, &Settings::default()).map(|opened| opened.history.len()).unwrap_or(0)
}

#[test]
fn app_calc_works_out_without_the_history() {
    let mut engine = Engine::default();
    let settings = Settings::default();
    engine.evaluate(Mode::Physics, &settings, "1+1").unwrap();
    let before = history(&mut engine);
    assert_eq!(engine.app_calc(Mode::Algebra, &settings, r"\frac{1}{2}+\frac{1}{3}"), Some(Ok(r"\frac{5}{6}".to_string())));
    assert!(matches!(engine.app_calc(Mode::Algebra, &settings, "1+"), Some(Err(_))));
    assert_eq!(history(&mut engine), before);
}
