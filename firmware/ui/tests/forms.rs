//! A form drawn and typed into by the hui (firmware/docs/apps.md §4.7), built in Rust.

use quantus_ui::app::App;
use quantus_ui::applet::AppInfo;
use quantus_ui::backend::Stub;
use quantus_ui::form::{value_latex, Field, Form, FormApplet, Outcome};
use quantus_ui::key::Key;
use quantus_ui::render::{Frame, Shape};

fn field(id: &str, unit: Option<&str>) -> Field {
    Field { id: id.into(), label: format!("Feld {id}"), unit: unit.map(str::to_string), ..Field::default() }
}

fn form() -> Form {
    let mut fields: Vec<Field> = (1..=6).map(|n| field(&format!("a{n}"), Some("V"))).collect();
    fields[1].value = Some("10 kΩ".into());
    fields[2].hint = Some("Nur positive Werte".into());
    fields.push(Field { choices: vec!["Stahl".into(), "Alu".into(), "Kupfer".into()], ..field("m", None) });
    Form {
        title: "Test".into(),
        fields,
        results: vec![Outcome { label: "S".into(), formula: "a1+a2".into(), unit: None }],
        submit: false,
    }
}

fn open() -> App<Stub> {
    let mut app = App::new(Stub);
    let info = AppInfo { id: "f".into(), name: "Form".into(), api: 1, usable: true, ..AppInfo::default() };
    app.open_applet(Box::new(FormApplet::new(form())), info);
    app
}

fn texts(app: &App<Stub>) -> Vec<String> {
    Frame::new(app).0.iter().filter_map(|shape| match shape {
        Shape::Text(_, text, ..) => Some(text.clone()),
        _ => None,
    }).collect()
}

fn press(app: &mut App<Stub>, keys: &[Key]) {
    keys.iter().for_each(|&key| app.key(key));
}

#[test]
fn values_from_the_description() {
    assert_eq!(value_latex("10 kΩ"), r"10\,\mathrm{kΩ}");
    assert_eq!(value_latex("2,5"), "2,5");
    assert_eq!(value_latex(r"3\,\mathrm{m}"), r"3\,\mathrm{m}");
    let applet = FormApplet::new(form());
    let values = applet.inputs();
    // As the editor writes it back.
    assert_eq!(values[1], ("a2".into(), r"10\,\mathrm{k\Omega}".into()));
    assert_eq!(values[6], ("m".into(), "Stahl".into()));
}

#[test]
fn fields_are_typed_into_and_scroll() {
    let mut app = open();
    let shown = texts(&app);
    assert!(shown.contains(&"Feld a1".to_string()) && shown.contains(&"Nur positive Werte".to_string()), "{shown:?}");
    assert!(shown.iter().filter(|text| *text == "V").count() >= 3);
    // The last field is below the area until the cursor goes there.
    assert!(!shown.contains(&"Feld m".to_string()));
    press(&mut app, &[Key::Digit(1), Key::Digit(2), Key::Exe]);
    for _ in 0..5 {
        app.key(Key::Down);
    }
    let shown = texts(&app);
    assert!(shown.contains(&"Feld m".to_string()) && shown.contains(&"Stahl".to_string()), "{shown:?}");
    assert!(!shown.contains(&"Feld a1".to_string()));
}

#[test]
fn a_choice_is_picked_from_its_list() {
    // Driven directly, to read the inputs back.
    let mut applet = FormApplet::new(form());
    struct Quiet;
    impl quantus_ui::applet::Host for Quiet {
        fn random(&mut self) -> u32 { 4 }
        fn lang(&self) -> quantus_ui::text::Lang { quantus_ui::text::Lang::De }
    }
    use quantus_ui::applet::Applet;
    let mut host = Quiet;
    for _ in 0..6 {
        applet.typed(Key::Down, false, &mut host);
    }
    applet.typed(Key::Ok, false, &mut host);
    assert_eq!(applet.choosing, Some(0));
    applet.typed(Key::Digit(3), false, &mut host);
    assert_eq!(applet.inputs()[6].1, "Kupfer");
    applet.typed(Key::Right, false, &mut host);
    assert_eq!(applet.inputs()[6].1, "Stahl");
    // A field typed into, with a unit from the layer, and one cleared with DEL.
    applet.typed(Key::Up, false, &mut host);
    applet.typed(Key::Digit(5), false, &mut host);
    applet.insert(r"\mathrm{mV}", &mut host);
    applet.typed(Key::Exe, false, &mut host);
    assert_eq!(applet.inputs()[5].1, r"5\mathrm{mV}");
    for _ in 0..5 {
        applet.typed(Key::Up, false, &mut host);
    }
    applet.typed(Key::Del, false, &mut host);
    assert_eq!(applet.inputs()[1].1, "");
}
