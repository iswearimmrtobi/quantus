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

const COUNTER: &str = r#"-- name: Counter
-- kind: tool
-- api: 1

local n = 0

function key(k)
    if k == "up" then n = n + 1 end
    if k == "down" then n = n - 1 end
end

function draw()
    clear("background")
    text(20, 40, "Count: " .. n, "text")
    rect(width() - 30, height() - 30, 20, 20, rgb(255, 0, 0))
end
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
    let mut app = open("function draw() clear(\"background\") end\nfunction key(k)\n  rect(0, 0, 1, 1, \"pink\")\nend\n");
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
    let app = open("function draw( end");
    assert!(texts(&app).iter().any(|text| text == "Fehler in der App"));
}

#[test]
fn an_endless_loop_shows_the_error_page() {
    // The budget ends the loop well before the main side stops waiting.
    let mut app = open("function draw() end\nfunction key(k) while true do end end");
    app.key(Key::Digit(1));
    assert!(texts(&app).iter().any(|text| text == "Fehler in der App"));
}

#[test]
fn too_many_shapes_are_an_error() {
    let mut app = open("local many\nfunction draw() if many then for i = 1, 5000 do rect(0, 0, 1, 1, \"text\") end end end\nfunction key(k) many = true end");
    app.key(Key::Digit(1));
    let shown = texts(&app);
    assert!(shown.iter().any(|text| text.contains("4000")), "{shown:?}");
}
