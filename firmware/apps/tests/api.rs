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
        local k
        function init() soft({"Neu", "", "Stufe"}) end
        function key(pressed) k = pressed; if k == "soft3" then soft({}) end end
        function draw() text(0, 0, height() .. " " .. (k or ""), "text") end
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
        local p = ""
        function init() input("grid", {cols = 3, rows = 3, digits = true}) end
        function pick(col, row) p = p .. col .. row .. " " end
        function key(k) p = p .. k .. " " end
        function draw() text(0, 0, p, "text") end
    "#);
    press(&mut app, &[Key::Right, Key::Ok, Key::Digit(9), Key::Digit(0)]);
    // 9 is the top right, as on the number block; columns and rows count from 1.
    assert!(shows(&app, "21 31 0 "));
}

#[test]
fn the_list_template_gives_the_index() {
    let mut app = open(r#"
        local e = 0
        function init() input("list", {items = {"Stahl", "Alu"}}) end
        function entered(i) e = i end
        function draw() text(0, 200, e, "text") end
    "#);
    assert!(shows(&app, "Alu"));
    press(&mut app, &[Key::Down, Key::Ok]);
    // The second line, counted from 1.
    assert!(shows(&app, "2"));
}

#[test]
fn cursor_sets_and_reads_the_grids_cursor() {
    let mut app = open(r#"
        function init() input("grid", {cols = 4, rows = 4}); cursor(3, 2) end
        function draw() local c = cursor(); text(0, 0, c.col .. "," .. c.row, "text") end
    "#);
    assert!(shows(&app, "3,2"));
    press(&mut app, &[Key::Down]);
    // The hui moved it; the app draws on the next callback.
    press(&mut app, &[Key::Digit(5)]);
    assert!(shows(&app, "3,3"));
}

#[test]
fn confirm_waits_for_the_answer() {
    let mut app = open(r#"
        local a = "?"
        function key(k) if confirm("Neu?") then a = "ja" else a = "nein" end end
        function draw() text(0, 0, a, "text") end
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
        function draw()
            local g = grid(3, 3)
            local c = g:cell(2, 3)
            local s = side()
            local hit = g:at(c.x + 1, c.y + 1)
            text(0, 0, string.format("%d %d %d %d|%d %d %d %d|%d %d|%s", c.x, c.y, c.w, c.h, s.x, s.y, s.w, s.h, hit.col, hit.row, tostring(g:at(-5, -5))), "text")
        end
    "#);
    let grid = Grid::new(480, 294, 3, 3);
    let (cell, side) = (grid.cell(1, 2), grid.side());
    let expected = format!(
        "{} {} {} {}|{} {} {} {}|2 3|nil",
        cell.top_left.x, cell.top_left.y, cell.size.width, cell.size.height, side.top_left.x, side.top_left.y, side.size.width, side.size.height
    );
    assert!(shows(&app, &expected));
}

#[test]
fn random_stays_in_its_range() {
    let mut app = open(r#"
        local ok = true
        function key(k)
            for i = 1, 200 do
                local r = math.random(6)
                if r < 1 or r > 6 or math.type(r) ~= "integer" then ok = false end
                local f = math.random()
                if f < 0 or f >= 1 then ok = false end
                local m = math.random(-2, 2)
                if m < -2 or m > 2 then ok = false end
            end
        end
        function draw() text(0, 0, tostring(ok) .. " " .. math.random(1), "text") end
    "#);
    press(&mut app, &[Key::Digit(1), Key::Digit(2)]);
    assert!(shows(&app, "true 1"));
}

#[test]
fn the_timer_ticks() {
    let mut app = open(r#"
        local n, at = 0, nil
        function init() timer(50) end
        function tick(ms) n = n + 1; at = ms; if n == 2 then timer(0) end end
        function draw() text(0, 0, n .. " " .. tostring(at), "text") end
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
        function key(k) exit() end
        function draw() text(0, 0, lang() .. " " .. math.type(now()) , "text") end
    "#);
    assert!(shows(&app, "de integer") || shows(&app, "de nil"));
    press(&mut app, &[Key::Digit(1)]);
    assert_eq!(app.screen, Screen::Calc);
}

#[test]
fn a_wrong_input_is_an_error() {
    let app = open(r#"function init() input("wheel") end function draw() end"#);
    assert!(texts(&app).iter().any(|text| text.contains("wheel")));
}

#[test]
fn the_text_template_and_ask() {
    let mut app = open(r#"
        local e = ""
        function init() input("text", {label = "Name", max = 4, value = "Al"}) end
        function entered(t) e = t end
        function key(k) if k == "up" then e = ask("Wer?", {value = "X"}) or "" end end
        function draw() text(0, 0, height() .. ":" .. e, "text") end
    "#);
    assert!(shows(&app, "250:"));
    press(&mut app, &[Key::Letter('i'), Key::Letter('c'), Key::Letter('e'), Key::Exe]);
    assert!(shows(&app, "250:Alic"));
    press(&mut app, &[Key::Up, Key::Letter('Y'), Key::Ok]);
    assert!(shows(&app, "250:XY"));
    press(&mut app, &[Key::Up, Key::Ac]);
    assert!(shows(&app, "250:"));
}

#[test]
fn the_line_template_gives_latex() {
    let mut app = open(r#"
        local e = ""
        function init() input("line", {label = "x ="}) end
        function entered(v) e = v end
        function draw() text(0, 0, e, "text") end
    "#);
    press(&mut app, &[Key::Digit(2), Key::Power, Key::Digit(5), Key::Exe]);
    assert!(shows(&app, "2^{5}"));
}

#[test]
fn the_cells_template_gives_col_row_and_value() {
    let mut app = open(r#"
        local e = ""
        function init() input("cells", {cols = 2, rows = 2, digits_only = true}) end
        function entered(c) e = e .. c.col .. c.row .. "=" .. c.value .. " " end
        function draw() text(0, 0, e, "text") end
    "#);
    press(&mut app, &[Key::Digit(4), Key::Down, Key::Digit(9)]);
    assert!(shows(&app, "11=4 22=9 "));
}

#[test]
fn taps_and_swipes_reach_the_script() {
    use embedded_graphics::prelude::Point;
    use quantus_ui::touch::Touch;
    let mut app = open(r#"
        local t = ""
        function tap(x, y)
            local c = grid(3, 3):at(x, y)
            if c then t = t .. c.col .. c.row .. " " else t = t .. "(" .. x .. "," .. y .. ") " end
        end
        function key(k) t = t .. k .. " " end
        function draw() text(0, 0, t, "text") end
    "#);
    let tap = |app: &mut App<Stub>, point: Point| {
        app.touch(Touch::Press(point));
        app.touch(Touch::Release);
    };
    // The middle of cell (1, 2) of the board, on the screen below the bar.
    let grid = Grid::new(480, 294, 3, 3);
    let cell = grid.cell(1, 2);
    tap(&mut app, cell.center() + Point::new(0, 26));
    tap(&mut app, Point::new(470, 300));
    // A swipe to the left, and one upwards.
    let swipe = |app: &mut App<Stub>, from: Point, to: Point| {
        app.touch(Touch::Press(from));
        app.touch(Touch::Drag(to));
        app.touch(Touch::Release);
    };
    swipe(&mut app, Point::new(300, 150), Point::new(200, 160));
    swipe(&mut app, Point::new(300, 250), Point::new(290, 120));
    assert!(shows(&app, "23 (470,274) left up "));
}
