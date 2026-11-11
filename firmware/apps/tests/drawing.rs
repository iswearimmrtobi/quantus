//! A script inside the hui: the Counter of apps.md §4.1, its keys and its shapes, and the error
//! page.

use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use quantus_apps::ScriptApplet;
use quantus_ui::app::{App, Screen};
use quantus_ui::applet::{AppInfo, Palette};
use quantus_ui::backend::Stub;
use quantus_ui::key::Key;
use quantus_ui::render::{Frame, Shape};

const COUNTER: &str = r#"// name: Counter
// kind: tool
// api: 1

fn init() { this.n = 0; }

fn key(k) {
    if k == "up"   { this.n += 1; }
    if k == "down" { this.n -= 1; }
}

fn draw() {
    clear("background");
    text(20, 40, `Count: ${this.n}`, "text");
    rect(width() - 30, height() - 30, 20, 20, rgb(255, 0, 0));
}
"#;

fn open(source: &str) -> App<Stub> {
    let mut app = App::new(Stub);
    let info = AppInfo { id: "counter".into(), name: "Counter".into(), api: 1, usable: true, ..AppInfo::default() };
    app.open_applet(Box::new(ScriptApplet::new(source)), info);
    app
}

fn texts(app: &App<Stub>) -> Vec<String> {
    Frame::new(app).0.iter().filter_map(|shape| match shape {
        Shape::Text(_, text, ..) => Some(text.clone()),
        _ => None,
    }).collect()
}

#[test]
fn the_counter_counts_and_draws() {
    let mut app = open(COUNTER);
    assert!(texts(&app).contains(&"Count: 0".to_string()), "{:?}", texts(&app));
    for key in [Key::Up, Key::Up, Key::Down, Key::Up] {
        app.key(key);
    }
    assert!(texts(&app).contains(&"Count: 2".to_string()));
    let frame = Frame::new(&app);
    // The background fills the area, the square sits at its bottom right in the colour asked.
    assert!(frame.0.iter().any(|shape| matches!(shape, Shape::Fill(rect, color)
        if *rect == Rectangle::new(Point::new(0, 26), Size::new(480, 294)) && Some(*color) == Palette::get("background"))));
    assert!(frame.0.iter().any(|shape| matches!(shape, Shape::Fill(rect, color)
        if *rect == Rectangle::new(Point::new(450, 290), Size::new(20, 20)) && *color == Palette::rgb(255, 0, 0))));
}

#[test]
fn an_error_turns_into_a_page_and_ok_leaves() {
    let mut app = open("fn draw() { clear(\"background\"); }\nfn key(k) {\n  rect(0, 0, 1, 1, \"pink\");\n}\n");
    app.key(Key::Digit(1));
    let shown = texts(&app);
    assert!(shown.iter().any(|text| text == "Fehler in der App"), "{shown:?}");
    assert!(shown.iter().any(|text| text.contains("pink")), "{shown:?}");
    assert!(shown.iter().any(|text| text == "Zeile 3"), "{shown:?}");
    app.key(Key::Ok);
    assert_eq!(app.screen, Screen::Calc);
}

#[test]
fn a_syntax_error_shows_at_once() {
    let app = open("fn draw( {");
    assert!(texts(&app).iter().any(|text| text == "Fehler in der App"));
}

#[test]
fn an_endless_loop_shows_the_error_page() {
    // The budget ends the loop well before the main side stops waiting.
    let mut app = open("fn draw() {}\nfn key(k) { while true {} }");
    app.key(Key::Digit(1));
    assert!(texts(&app).iter().any(|text| text == "Fehler in der App"));
}

#[test]
fn too_many_shapes_are_an_error() {
    let mut app = open("fn draw() { if this.many == true { for i in 0..5000 { rect(0, 0, 1, 1, \"text\"); } } }\nfn key(k) { this.many = true; }");
    app.key(Key::Digit(1));
    let shown = texts(&app);
    assert!(shown.iter().any(|text| text.contains("4000")), "{shown:?}");
}
