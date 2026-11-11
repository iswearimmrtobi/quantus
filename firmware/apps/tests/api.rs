//! The script API through the hui, one small script per function (apps.md §4.3–4.5).

use quantus_apps::ScriptApplet;
use quantus_ui::app::{App, Screen};
use quantus_ui::applet::{AppInfo, Grid};
use quantus_ui::backend::Stub;
use quantus_ui::key::Key;
use quantus_ui::render::{Frame, Shape};

fn open(source: &str) -> App<Stub> {
    let mut app = App::new(Stub);
    let info = AppInfo { id: "t".into(), name: "T".into(), api: 1, usable: true, ..AppInfo::default() };
    app.open_applet(Box::new(ScriptApplet::new(source)), info);
    app
}

fn texts(app: &App<Stub>) -> Vec<String> {
    Frame::new(app).0.iter().filter_map(|shape| match shape {
        Shape::Text(_, text, ..) => Some(text.clone()),
        _ => None,
    }).collect()
}

/// Whether the frame shows `text`.
fn shows(app: &App<Stub>, text: &str) -> bool {
    let shown = texts(app);
    let found = shown.iter().any(|shown| shown == text);
    if !found {
        eprintln!("{text:?} not in {shown:?}");
    }
    found
}

fn press(app: &mut App<Stub>, keys: &[Key]) {
    keys.iter().for_each(|&key| app.key(key));
}

#[test]
fn soft_turns_the_keys_on_and_the_area_follows() {
    let mut app = open(r#"
        fn init() { soft(["Neu", "", "Stufe"]); }
        fn key(k) { this.k = k; if k == "soft3" { soft([]); } }
        fn draw() { text(0, 0, `${height()} ${this.k}`, "text"); }
    "#);
    assert!(shows(&app, "Neu") && shows(&app, "Stufe"));
    assert!(shows(&app, "250 "));
    press(&mut app, &[Key::Soft(0)]);
    assert!(shows(&app, "250 soft1"));
    // The blank one is dead; the third turns them off and the app is drawn for the larger area.
    press(&mut app, &[Key::Soft(1), Key::Soft(2)]);
    assert!(shows(&app, "294 soft3"));
}

#[test]
fn the_grid_template_picks() {
    let mut app = open(r#"
        fn init() { input("grid", #{ cols: 3, rows: 3, digits: true }); this.p = ""; }
        fn pick(col, row) { this.p += `${col}${row} `; }
        fn key(k) { this.p += k + " "; }
        fn draw() { text(0, 0, this.p, "text"); }
    "#);
    press(&mut app, &[Key::Right, Key::Ok, Key::Digit(9), Key::Digit(0)]);
    // 9 is the top right, as on the number block.
    assert!(shows(&app, "10 20 0 "));
}

#[test]
fn the_list_template_gives_the_index() {
    let mut app = open(r#"
        fn init() { input("list", #{ items: ["Stahl", "Alu"] }); this.e = -1; }
        fn entered(i) { this.e = i; }
        fn draw() { text(0, 200, `${this.e}`, "text"); }
    "#);
    assert!(shows(&app, "Alu"));
    press(&mut app, &[Key::Down, Key::Ok]);
    assert!(shows(&app, "1"));
}

#[test]
fn cursor_sets_and_reads_the_grids_cursor() {
    let mut app = open(r#"
        fn init() { input("grid", #{ cols: 4, rows: 4 }); cursor(2, 1); }
        fn draw() { let c = cursor(); text(0, 0, `${c.col},${c.row}`, "text"); }
    "#);
    assert!(shows(&app, "2,1"));
    press(&mut app, &[Key::Down]);
    // The hui moved it; the app draws on the next callback.
    press(&mut app, &[Key::Digit(5)]);
    assert!(shows(&app, "2,2"));
}

#[test]
fn confirm_waits_for_the_answer() {
    let mut app = open(r#"
        fn init() { this.a = "?"; }
        fn key(k) { this.a = if confirm("Neu?") { "ja" } else { "nein" }; }
        fn draw() { text(0, 0, this.a, "text"); }
    "#);
    press(&mut app, &[Key::Digit(1)]);
    assert!(shows(&app, "Neu?"));
    press(&mut app, &[Key::Digit(1)]);
    assert!(shows(&app, "ja"));
    press(&mut app, &[Key::Digit(1), Key::Ac]);
    assert!(shows(&app, "nein"));
    assert_eq!(app.screen, Screen::App);
}

#[test]
fn grid_and_side_agree_with_the_hui() {
    let app = open(r#"
        fn draw() {
            let g = grid(3, 3);
            let c = g.cell(1, 2);
            let s = side();
            let hit = g.at(c.x + 1, c.y + 1);
            text(0, 0, `${c.x} ${c.y} ${c.w} ${c.h}|${s.x} ${s.y} ${s.w} ${s.h}|${hit.col} ${hit.row}|${g.at(-5, -5)}`, "text");
        }
    "#);
    let grid = Grid::new(480, 294, 3, 3);
    let (cell, side) = (grid.cell(1, 2), grid.side());
    let expected = format!(
        "{} {} {} {}|{} {} {} {}|1 2|",
        cell.top_left.x, cell.top_left.y, cell.size.width, cell.size.height, side.top_left.x, side.top_left.y, side.size.width, side.size.height
    );
    assert!(shows(&app, &expected));
}

#[test]
fn random_stays_in_its_range() {
    let mut app = open(r#"
        fn init() { this.ok = true; }
        fn key(k) { for i in 0..200 { let r = random(6); if r < 0 || r > 5 { this.ok = false; } } }
        fn draw() { text(0, 0, `${this.ok} ${random(1)}`, "text"); }
    "#);
    press(&mut app, &[Key::Digit(1), Key::Digit(2)]);
    assert!(shows(&app, "true 0"));
}

#[test]
fn the_timer_ticks() {
    let mut app = open(r#"
        fn init() { this.n = 0; timer(50); }
        fn tick(ms) { this.n += 1; this.ms = ms; if this.n == 2 { timer(0); } }
        fn draw() { text(0, 0, `${this.n} ${this.ms}`, "text"); }
    "#);
    // Not faster than 100 ms; stopped by timer(0).
    for now in [0, 50, 100, 150, 200, 300, 400] {
        app.tick(now);
    }
    assert!(shows(&app, "2 200"));
}

#[test]
fn lang_now_and_exit() {
    let mut app = open(r#"
        fn key(k) { exit(); }
        fn draw() { text(0, 0, `${lang()} ${type_of(now())}`, "text"); }
    "#);
    assert!(shows(&app, "de ()") || shows(&app, "de i64"));
    press(&mut app, &[Key::Digit(1)]);
    assert_eq!(app.screen, Screen::Calc);
}

#[test]
fn a_wrong_input_is_an_error() {
    let app = open(r#"fn init() { input("wheel"); } fn draw() {}"#);
    assert!(texts(&app).iter().any(|text| text.contains("wheel")));
}

#[test]
fn the_text_template_and_ask() {
    let mut app = open(r#"
        fn init() { input("text", #{ label: "Name", max: 4, value: "Al" }); this.e = ""; }
        fn entered(t) { this.e = t; }
        fn key(k) { if k == "up" { let a = ask("Wer?", #{ value: "X" }); this.e = `${a}`; } }
        fn draw() { text(0, 0, `${height()}:${this.e}`, "text"); }
    "#);
    assert!(shows(&app, "250:"));
    press(&mut app, &[Key::Letter('i'), Key::Letter('c'), Key::Letter('e'), Key::Exe]);
    assert!(shows(&app, "250:Alic"));
    press(&mut app, &[Key::Up, Key::Letter('Y'), Key::Ok]);
    assert!(shows(&app, "250:XY"));
    press(&mut app, &[Key::Up, Key::Ac]);
    assert!(shows(&app, "250:"));
}
