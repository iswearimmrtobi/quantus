use quantus_core::api::{self, CalculationRequest};
use quantus_core::session::SessionStore;
use quantus_core::settings::{Mode, NumberBase, Settings};

#[test]
fn the_mode_evaluates_in_the_set_base_with_the_others_as_forms() {
    let mut store = SessionStore::new();
    let settings = Settings { mode: Mode::BaseN, number_base: NumberBase::Hex, ..Settings::default() };
    api::set_settings(&mut store, settings).unwrap();
    let request = CalculationRequest { latex: r"\mathrm{1F} \cdot 2 \operatorname{and} FF".into(), ..Default::default() };
    let response = api::evaluate(&mut store, &request).unwrap();
    assert_eq!(response.result, r"\mathrm{3E}");
    assert_eq!(response.value, "62");
    let forms: Vec<_> = response.forms.iter().map(|f| (serde_json::to_value(f.kind).unwrap(), f.latex.clone())).collect();
    assert_eq!(forms[0], (serde_json::json!("dec"), "62".to_string()));
    assert_eq!(forms[1], (serde_json::json!("bin"), "111110".to_string()));
}

#[test]
fn the_setting_names_the_mode_base_n() {
    let settings: Settings = serde_json::from_value(serde_json::json!({ "mode": "baseN", "numberBase": "bin" })).unwrap();
    assert_eq!((settings.mode, settings.number_base), (Mode::BaseN, NumberBase::Bin));
}

#[test]
fn word_size_and_signedness_are_settings() {
    use quantus_core::settings::WordSize;
    // An old settings file without the keys loads as 32 bit, signed.
    let old: Settings = serde_json::from_value(serde_json::json!({ "mode": "baseN" })).unwrap();
    assert_eq!((old.word_size, old.signed), (WordSize::W32, true));
    let settings = Settings { word_size: WordSize::W8, signed: false, ..Settings::default() };
    let json = serde_json::to_value(&settings).unwrap();
    assert_eq!((&json["wordSize"], &json["signed"]), (&serde_json::json!(8), &serde_json::json!(false)));
    assert_eq!(serde_json::from_value::<Settings>(json).unwrap(), settings);
    assert!(serde_json::from_value::<Settings>(serde_json::json!({ "wordSize": 12 })).is_err());
}

#[test]
fn a_wrapped_result_warns_and_the_value_is_read_by_the_word() {
    use quantus_core::settings::WordSize;
    let evaluate = |latex: &str, word_size, signed| {
        let mut store = SessionStore::new();
        let settings = Settings { mode: Mode::BaseN, word_size, signed, ..Settings::default() };
        api::set_settings(&mut store, settings).unwrap();
        api::evaluate(&mut store, &CalculationRequest { latex: latex.into(), ..Default::default() }).unwrap()
    };
    let response = evaluate("200+100", WordSize::W8, false);
    assert_eq!((response.result.as_str(), response.value.as_str()), ("44", "44"));
    assert_eq!(response.warning.as_deref(), Some("Overflow: the result was wrapped to 8 bits."));
    let response = evaluate("0-1", WordSize::W64, false);
    assert_eq!(response.value, "18446744073709551615");
    assert!(response.warning.is_some());
    let response = evaluate("100+27", WordSize::W8, true);
    assert_eq!((response.result.as_str(), response.warning), ("127", None));
}
