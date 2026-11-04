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

mod sessions {
    use std::path::PathBuf;

    use quantus_engine::{Engine, CURRENT};
    use quantus_ui::app::{Mode, Settings};
    use quantus_ui::backend::{Backend, SessionCommand};

    /// An empty folder of its own for each test.
    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("quantus-engine-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn files(dir: &PathBuf) -> usize {
        std::fs::read_dir(dir).unwrap().filter(|f| f.as_ref().unwrap().path().extension().is_some_and(|e| e == "json")).count()
    }

    #[test]
    fn sessions_are_kept_in_files_and_the_open_one_comes_back() {
        let dir = folder("files");
        let settings = Settings::default();
        let first = {
            let mut engine = Engine::with_clock(Some(dir.clone()), || None);
            assert_eq!(engine.session(SessionCommand::Current, &settings).unwrap().info.name, "Sitzung 1");
            engine.evaluate(Mode::Physics, &settings, "m_1 = 2kg").unwrap();
            let first = engine.current().to_string();
            let second = engine.session(SessionCommand::New, &settings).unwrap();
            assert_eq!(second.info.name, "Sitzung 2");
            assert!(second.history.is_empty());
            engine.session(SessionCommand::Rename("Labor".into()), &settings).unwrap();
            engine.session(SessionCommand::Open(first.clone()), &settings).unwrap();
            first
        };
        assert_eq!(files(&dir), 2);
        assert_eq!(std::fs::read_to_string(dir.join(CURRENT)).unwrap(), first);

        let mut engine = Engine::with_clock(Some(dir.clone()), || None);
        let opened = engine.session(SessionCommand::Current, &settings).unwrap();
        assert_eq!(opened.info.id, first);
        assert_eq!(opened.history.len(), 1);
        assert_eq!(opened.history[0].0, "m_1 = 2kg");
        // The variable was worked out again.
        assert_eq!(engine.evaluate(Mode::Physics, &settings, r"m_1 \cdot 3"), Ok(r"6\,\mathrm{kg}".to_string()));
        let names: Vec<String> = engine.sessions().into_iter().map(|s| s.name).collect();
        assert!(names.contains(&"Labor".to_string()), "{names:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn deleting_the_open_session_opens_another_or_a_new_one() {
        let dir = folder("delete");
        let settings = Settings::default();
        let mut engine = Engine::with_clock(Some(dir.clone()), || None);
        let first = engine.current().to_string();
        let copy = engine.session(SessionCommand::Duplicate, &settings).unwrap();
        assert_eq!(copy.info.name, "Sitzung 1 (Kopie)");
        assert_eq!(files(&dir), 2);
        let next = engine.session(SessionCommand::Delete, &settings).unwrap();
        assert_eq!(next.info.id, first);
        assert_eq!(files(&dir), 1);
        engine.evaluate(Mode::Algebra, &settings, "1+1").unwrap();
        let fresh = engine.session(SessionCommand::Delete, &settings).unwrap();
        assert!(fresh.history.is_empty());
        assert_eq!(fresh.info.name, "Sitzung 1");
        assert_eq!(files(&dir), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn s_d_does_not_record_and_clearing_empties_the_history() {
        let mut engine = Engine::default();
        let settings = Settings::default();
        engine.evaluate(Mode::Algebra, &settings, r"\frac{1}{3}").unwrap();
        let decimal = Settings { decimal: true, ..settings };
        assert!(engine.recalculate(Mode::Algebra, &decimal, r"\frac{1}{3}").unwrap().unwrap().starts_with(r"\approx"));
        assert_eq!(engine.session(SessionCommand::Current, &settings).unwrap().history.len(), 1);
        assert!(engine.session(SessionCommand::ClearHistory, &settings).unwrap().history.is_empty());
    }

    /// 2026-11-02 13:30 UTC.
    fn monday() -> Option<i64> {
        Some(1_793_629_800_000 - 3_600_000)
    }

    #[test]
    fn with_the_clock_set_a_new_session_is_named_after_the_local_time() {
        let mut engine = Engine::with_clock(None, monday);
        let settings = Settings::default();
        assert_eq!(engine.session(SessionCommand::Current, &settings).unwrap().info.name, "Sitzung 2026-11-02 14:30");
        let london = Settings { zone: quantus_ui::settings::Zone::London, ..settings };
        assert_eq!(engine.session(SessionCommand::New, &london).unwrap().info.name, "Sitzung 2026-11-02 13:30");
        let english = Settings { language: quantus_ui::text::Lang::En, ..settings };
        assert_eq!(engine.session(SessionCommand::New, &english).unwrap().info.name, "Session 2026-11-02 14:30");
    }
}

/// What OPTN inserts, as the editor writes it back, is what the engine reads.
mod optn {
    use quantus_engine::Engine;
    use quantus_ui::app::{Mode, Settings};
    use quantus_ui::backend::Backend;
    use quantus_ui::latex;

    /// `latex` as the hui's editor holds and writes it, worked out in `mode`.
    pub fn typed(mode: Mode, latex: &str) -> Result<String, String> {
        let edited = latex::parse(latex).to_latex();
        Engine::default().evaluate(mode, &Settings { decimal: true, ..Settings::default() }, &edited)
    }

    #[test]
    fn degrees_minutes_and_seconds() {
        let exact = |latex: &str| Engine::default().evaluate(Mode::Physics, &Settings::default(), &latex::parse(latex).to_latex());
        assert_eq!(exact(r"2^\circ 20' 30''"), Ok(r"\frac{281}{120}".to_string()));
        assert!(exact(r"50\,^\circ\mathrm{F} \blacktriangleright ^\circ\mathrm{C}").unwrap().starts_with("10"));
    }

    #[test]
    fn ncr() {
        assert_eq!(typed(Mode::Physics, r"\binom{5}{2}"), Ok("10".to_string()));
    }

    #[test]
    fn permutations_and_the_divisors() {
        assert_eq!(typed(Mode::Physics, r"\operatorname{nPr}(5;2)"), Ok("20".to_string()));
        assert_eq!(typed(Mode::Physics, r"\operatorname{ggT}(12;18)"), Ok("6".to_string()));
        assert_eq!(typed(Mode::Physics, r"\operatorname{kgV}(4;6)"), Ok("12".to_string()));
    }

    #[test]
    fn the_whole_parts_rounding_and_the_remainder() {
        assert_eq!(typed(Mode::Physics, r"\operatorname{Int}(-2{,}5)"), Ok("-2".to_string()));
        assert_eq!(typed(Mode::Physics, r"\operatorname{Intg}(-2{,}5)"), Ok("-3".to_string()));
        assert!(typed(Mode::Physics, r"\operatorname{Rnd}(\frac{1}{3})").unwrap().contains("0{,}3333333333"));
        assert!(typed(Mode::Physics, r"\operatorname{RndFix}(\pi;3)").unwrap().contains("3{,}142"));
        assert_eq!(typed(Mode::Physics, r"\operatorname{divR}(7;2)"), Ok(r"Q = 3,\; R = 1".to_string()));
    }

    #[test]
    fn the_product() {
        assert_eq!(typed(Mode::Physics, r"\prod_{k=1}^{5}k"), Ok("120".to_string()));
    }

    #[test]
    fn the_area_functions_and_coth() {
        assert!(typed(Mode::Physics, r"\operatorname{arsinh}(1)").unwrap().contains("0{,}8813"));
        assert!(typed(Mode::Physics, r"\operatorname{arcosh}(2)").unwrap().contains("1{,}3169"));
        assert!(typed(Mode::Physics, r"\operatorname{artanh}(0{,}5)").unwrap().contains("0{,}5493"));
        assert!(typed(Mode::Physics, r"\coth(1)").unwrap().contains("1{,}3130"));
    }
}

/// CONST and CONV list what the engine reads after the hui's editor has held it.
mod catalogues {
    use quantus_engine::Engine;
    use quantus_ui::app::{Mode, Settings};
    use quantus_ui::backend::{Backend, Catalogue};
    use quantus_ui::latex;
    use quantus_ui::text::Lang;

    #[test]
    fn every_constant_is_known_in_physik() {
        let mut engine = Engine::default();
        let groups = engine.catalogue(Catalogue::Constants, Lang::De);
        assert_eq!(groups.len(), 6);
        assert_eq!(groups.iter().map(|group| group.items.len()).sum::<usize>(), 47);
        for item in groups.iter().flat_map(|group| &group.items) {
            let typed = latex::parse(&item.latex).to_latex();
            let result = engine.evaluate(Mode::Physics, &Settings::default(), &typed);
            assert!(result.is_ok(), "{} ({typed}): {result:?}", item.name);
        }
    }

    #[test]
    fn every_unit_is_a_target_of_the_arrow() {
        let mut engine = Engine::default();
        let settings = Settings { extra_unit_letters: true, ..Settings::default() };
        let groups = engine.catalogue(Catalogue::Units, Lang::En);
        assert_eq!(groups[0].name, "Length");
        for item in groups.iter().flat_map(|group| &group.items) {
            let typed = latex::parse(&format!(r"1\,{0} \blacktriangleright {0}", item.latex)).to_latex();
            let result = engine.evaluate(Mode::Physics, &settings, &typed);
            assert!(result.is_ok(), "{} ({typed}): {result:?}", item.name);
        }
    }
}

/// CALC: the letters of an input, then their values.
mod calc {
    use quantus_engine::Engine;
    use quantus_ui::app::{Mode, Settings};
    use quantus_ui::backend::{Backend, Reply, Request, Variable};

    #[test]
    fn the_free_letters_are_asked_for_and_their_values_used() {
        let mut engine = Engine::default();
        let settings = Settings::default();
        let latex = r"\frac{1}{2}m\cdot v^{2}".to_string();
        let asked = engine.solve(&settings, &Request::Variables { mode: Mode::Algebra, latex: latex.clone() });
        let variable = |key: &str| Variable { key: key.into(), symbol: key.into() };
        assert_eq!(asked, Ok(Reply::Variables(vec![variable("m"), variable("v")])));
        let values = vec![("m".to_string(), "2".to_string()), ("v".to_string(), "3".to_string())];
        let result = engine.solve(&settings, &Request::Calc { mode: Mode::Algebra, latex, values });
        assert_eq!(result, Ok(Reply::Result("9".to_string())));
        // The values are the session's now: m has one, v too.
        let asked = engine.solve(&settings, &Request::Variables { mode: Mode::Algebra, latex: "m+v+w".into() });
        assert_eq!(asked, Ok(Reply::Variables(vec![variable("w")])));
    }

    #[test]
    fn an_input_the_engine_cannot_read_says_why() {
        let mut engine = Engine::default();
        let asked = engine.solve(&Settings::default(), &Request::Variables { mode: Mode::Algebra, latex: "2+".into() });
        assert!(asked.is_err());
    }
}
