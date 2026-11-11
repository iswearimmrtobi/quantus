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

use quantus_apps::ScriptApplet;
use quantus_ui::app::App;
use quantus_ui::applet::AppInfo;
use quantus_ui::key::Key;
use quantus_ui::render::{Frame, Shape};

fn texts(app: &App<Engine>) -> Vec<String> {
    Frame::new(app).0.iter().filter_map(|shape| match shape {
        Shape::Text(_, text, ..) => Some(text.clone()),
        _ => None,
    }).collect()
}

#[test]
fn a_script_adds_two_lengths_and_the_history_stays() {
    let mut app = App::new(Engine::default());
    app.key(Key::Digit(7));
    app.key(Key::Exe);
    let history = app.history.len();
    let source = r#"
        local t = ""
        function key(k)
            local a = calc("3\\,\\mathrm{m}+20\\,\\mathrm{cm}")
            local b = calc("1+")
            local c = calc(a.value .. "\\cdot 2")
            t = tostring(a.ok) .. " " .. a.latex .. "|" .. tostring(b.ok) .. " " .. tostring(b.error ~= nil) .. "|" .. c.latex
        end
        function draw() text(0, 0, t, "text") end
    "#;
    let info = AppInfo { id: "t".into(), name: "T".into(), api: 1, usable: true, ..AppInfo::default() };
    app.open_applet(Box::new(ScriptApplet::new(source)), info);
    app.key(Key::Digit(1));
    let shown = texts(&app);
    let line = shown.iter().find(|text| text.starts_with("true")).unwrap_or_else(|| panic!("{shown:?}"));
    let (sum, rest) = line.split_once('|').unwrap();
    assert!(sum.contains(r"\mathrm{m}") && (sum.contains("16") || sum.contains("3{,}2")), "{line}");
    assert!(rest.starts_with("false true|"), "{line}");
    assert!(rest.contains(r"\mathrm{m}"), "{line}");
    app.key(Key::Ac);
    assert_eq!(app.history.len(), history);
}

#[test]
fn var_reads_the_sessions_variables_and_insert_hands_back() {
    use quantus_ui::backend::VariableCommand;
    let settings = Settings::default();
    let store = VariableCommand::Store { mode: Mode::Physics, key: "a".into(), latex: "5".into() };
    // Through the app's backend, as STO does.
    let mut engine = Engine::default();
    engine.variables(store.clone(), &settings).expect("stored");
    let mut app = App::new(engine);
    let source = r#"
        local t
        function init() t = (variable("a") or "") .. " " .. (variable("nothing") or "") end
        function key(k) insert("\\frac{1}{2}") end
        function draw() text(0, 0, t, "text") end
    "#;
    let info = AppInfo { id: "t".into(), name: "T".into(), api: 1, usable: true, ..AppInfo::default() };
    app.open_applet(Box::new(ScriptApplet::new(source)), info);
    assert!(texts(&app).contains(&"5 ".to_string()), "{:?}", texts(&app));
    app.key(Key::Digit(1));
    // Left, back in the calculation, the value at its cursor.
    assert_eq!(app.screen, quantus_ui::app::Screen::Calc);
    assert_eq!(app.editor.to_latex(), r"\frac{1}{2}");
}

#[test]
fn the_line_with_calc_gives_the_result() {
    let mut app = App::new(Engine::default());
    let source = r#"
        local t = ""
        function init() input("line", {label = "x", calc = true}) end
        function entered(r) t = tostring(r.ok) .. " " .. r.latex end
        function draw() text(0, 0, t, "text") end
    "#;
    let info = AppInfo { id: "t".into(), name: "T".into(), api: 1, usable: true, ..AppInfo::default() };
    app.open_applet(Box::new(ScriptApplet::new(source)), info);
    for key in [Key::Digit(2), Key::Plus, Key::Digit(3), Key::Exe] {
        app.key(key);
    }
    assert!(texts(&app).contains(&"true 5".to_string()), "{:?}", texts(&app));
}

mod forms {
    use quantus_engine::Engine;
    use quantus_ui::app::{Mode, Settings};
    use quantus_ui::applet::{Applet, Host};
    use quantus_ui::backend::Backend;
    use quantus_ui::form::{Field, Form, FormApplet, Outcome};
    use quantus_ui::key::Key;
    use quantus_ui::text::Lang;

    /// What a form asks of the device, answered by the engine.
    struct Calculator(Engine);

    impl Host for Calculator {
        fn random(&mut self) -> u32 {
            4
        }

        fn lang(&self) -> Lang {
            Lang::De
        }

        fn calc(&mut self, input: &str) -> Option<Result<String, String>> {
            self.0.app_calc(Mode::Physics, &Settings::default(), input)
        }
    }

    fn field(id: &str, unit: &str, value: Option<&str>) -> Field {
        Field { id: id.into(), label: id.into(), unit: Some(unit.into()), value: value.map(str::to_string), ..Field::default() }
    }

    /// The voltage divider of apps.md §4.7.
    fn divider() -> FormApplet {
        FormApplet::new(Form {
            title: "Spannungsteiler".into(),
            fields: vec![field("U", "V", None), field("R1", "Ω", None), field("R2", "Ω", Some("10 kΩ"))],
            results: vec![
                Outcome { label: "U₂".into(), formula: "U*R2/(R1+R2)".into(), unit: Some("V".into()) },
                Outcome { label: "I".into(), formula: "U/(R1+R2)".into(), unit: Some("mA".into()) },
            ],
            submit: false,
        })
    }

    /// Types `keys` into the field under the cursor, `unit` after them, and goes on with EXE.
    fn fill(form: &mut FormApplet, host: &mut Calculator, keys: &[Key], unit: Option<&str>) {
        for &key in keys {
            form.typed(key, false, host);
        }
        if let Some(unit) = unit {
            form.insert(unit, host);
        }
        form.typed(Key::Exe, false, host);
    }

    #[test]
    fn the_voltage_divider_works_out() {
        let mut host = Calculator(Engine::default());
        let mut form = divider();
        fill(&mut form, &mut host, &[Key::Digit(1), Key::Digit(2)], None);
        fill(&mut form, &mut host, &[Key::Digit(5)], Some(r"\mathrm{k\Omega}"));
        // R₂ keeps its 10 kΩ; EXE on it works the form out.
        form.typed(Key::Exe, false, &mut host);
        let results = form.results.clone().expect("results");
        assert_eq!(results[0], ("U₂".to_string(), Ok(r"8\,\mathrm{V}".to_string())), "{results:?}");
        assert_eq!(results[1], ("I".to_string(), Ok(r"\frac{4}{5}\,\mathrm{mA}".to_string())), "{results:?}");
        // The cursor on the first result: OK hands it to the calculation.
        assert_eq!(form.result, Some(0));
        form.typed(Key::Ok, false, &mut host);
        assert_eq!(form.leave(&mut host).as_deref(), Some(r"8\,\mathrm{V}"));
    }

    #[test]
    fn a_number_alone_takes_the_fields_unit_and_a_wrong_quantity_is_the_fields_error() {
        let mut host = Calculator(Engine::default());
        let mut form = divider();
        // 12 into U, 5000 alone into R1: Ω; R₂ as given.
        fill(&mut form, &mut host, &[Key::Digit(1), Key::Digit(2)], None);
        fill(&mut form, &mut host, &[Key::Digit(5), Key::Digit(0), Key::Digit(0), Key::Digit(0)], None);
        form.typed(Key::Exe, false, &mut host);
        assert_eq!(form.results.as_ref().unwrap()[0].1, Ok(r"8\,\mathrm{V}".to_string()));

        // A length into the voltage.
        let mut form = divider();
        fill(&mut form, &mut host, &[Key::Digit(3)], Some(r"\mathrm{m}"));
        fill(&mut form, &mut host, &[Key::Digit(5)], Some(r"\mathrm{k\Omega}"));
        form.typed(Key::Exe, false, &mut host);
        assert!(form.results.is_none());
        let error = form.errors[0].clone().expect("an error on U");
        assert!(error.contains("V"), "{error}");
        assert!(form.errors[1].is_none() && form.errors[2].is_none());

        // An empty field says so.
        let mut form = divider();
        form.typed(Key::Exe, false, &mut host);
        form.typed(Key::Exe, false, &mut host);
        form.typed(Key::Exe, false, &mut host);
        assert_eq!(form.errors[0].as_deref(), Some("Hier fehlt ein Wert."));
    }

    #[test]
    fn whole_names_only_are_replaced() {
        use quantus_ui::form::substitute;
        let terms = [("R1".to_string(), "5".to_string()), ("R12".to_string(), "7".to_string())];
        assert_eq!(substitute(r"R1+R12+\sqrt{R1}+R1x", &terms), r"(5)+(7)+\sqrt{(5)}+R1x");
    }
}

mod form_scripts {
    use quantus_apps::ScriptApplet;
    use quantus_engine::Engine;
    use quantus_ui::app::{App, Screen};
    use quantus_ui::applet::AppInfo;
    use quantus_ui::key::Key;
    use quantus_ui::render::{Frame, Shape};

    const DIVIDER: &str = r#"
-- name: Spannungsteiler
-- kind: tool

form({
    title = "Spannungsteiler",
    fields = {
        {id = "U",  label = "Eingangsspannung", unit = "V"},
        {id = "R1", label = "R₁", unit = "Ω"},
        {id = "R2", label = "R₂", unit = "Ω", value = "10 kΩ"},
    },
    results = {
        {label = "U₂", formula = "U*R2/(R1+R2)", unit = "V"},
        {label = "I",  formula = "U/(R1+R2)",    unit = "mA"},
    },
})
"#;

    fn open(app: &mut App<Engine>, source: &str) {
        let info = AppInfo { id: "teiler".into(), name: "Teiler".into(), api: 1, usable: true, ..AppInfo::default() };
        app.open_applet(Box::new(ScriptApplet::new(source)), info);
    }

    fn texts(app: &App<Engine>) -> Vec<String> {
        Frame::new(app).0.iter().filter_map(|shape| match shape {
            Shape::Text(_, text, ..) => Some(text.clone()),
            _ => None,
        }).collect()
    }

    fn maths(app: &App<Engine>) -> usize {
        Frame::new(app).0.iter().filter(|shape| matches!(shape, Shape::Math(..))).count()
    }

    fn press(app: &mut App<Engine>, keys: &[Key]) {
        keys.iter().for_each(|&key| app.key(key));
    }

    #[test]
    fn a_script_that_is_only_a_form_works_out_and_keeps_its_inputs() {
        let mut app = App::new(Engine::default());
        open(&mut app, DIVIDER);
        let shown = texts(&app);
        assert!(shown.contains(&"Eingangsspannung".to_string()) && shown.contains(&"Spannungsteiler".to_string()), "{shown:?}");
        press(&mut app, &[Key::Digit(1), Key::Digit(2), Key::Exe, Key::Digit(5), Key::Digit(0), Key::Digit(0), Key::Digit(0), Key::Exe, Key::Exe]);
        let shown = texts(&app);
        assert!(shown.contains(&"U₂ =".to_string()) && shown.contains(&"I =".to_string()), "{shown:?}");
        // Left, opened again: the inputs stand in the fields.
        app.key(Key::Ac);
        let before = maths(&app);
        open(&mut app, DIVIDER);
        assert!(maths(&app) >= 3, "{before} {}", maths(&app));
        // OK on the first result hands it to the calculation.
        press(&mut app, &[Key::Exe, Key::Exe, Key::Exe, Key::Ok]);
        assert_eq!(app.screen, Screen::Calc);
        assert_eq!(app.editor.to_latex(), r"8\,\mathrm{V}");
    }

    const SUBMIT: &str = r#"
local got = "-"
function key(k)
    local r = form({
        fields = {{id = "a", unit = "m"}, {id = "b"}},
        submit = function(v)
            if v.b == "0" then return {field = "b", error = "nicht null"} end
            local s = calc(v.a .. "\\cdot " .. v.b)
            return {{label = "a·b", latex = s.latex}}
        end,
    })
    got = tostring(r["a·b"])
end
function draw() text(0, 0, got, "text") end
"#;

    #[test]
    fn submit_works_the_form_out_and_fertig_gives_the_results() {
        let mut app = App::new(Engine::default());
        open(&mut app, SUBMIT);
        app.key(Key::Digit(1));
        // The form, with Fertig on the soft keys.
        assert!(texts(&app).contains(&"Fertig".to_string()));
        // EXE keeps the value typed, EXE on the last field works it out.
        press(&mut app, &[Key::Digit(3), Key::Exe, Key::Digit(0), Key::Exe, Key::Exe]);
        assert!(texts(&app).contains(&"nicht null".to_string()), "{:?}", texts(&app));
        press(&mut app, &[Key::Del, Key::Digit(2), Key::Exe, Key::Exe]);
        assert!(texts(&app).contains(&"a·b =".to_string()), "{:?}", texts(&app));
        // Fertig: the results to the script, which draws them. `submit` got the inputs as typed.
        app.key(Key::Soft(0));
        assert!(texts(&app).contains(&"6".to_string()), "{:?}", texts(&app));
    }
}
