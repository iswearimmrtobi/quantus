//! Apps inside the hui's frame (firmware/docs/apps.md): the keys an app gets, its canvas, and an
//! applet written in Rust driven through the app.

use std::cell::RefCell;
use std::rc::Rc;

use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

use quantus_ui::app::{App, Mode, Screen};
use quantus_ui::applet::{AppInfo, AppKey, AppKind, Applet, Canvas, Host, Palette};
use quantus_ui::backend::Stub;
use quantus_ui::key::Key;
use quantus_ui::render::{colors, Shape};

/// What the test applet saw.
#[derive(Default)]
struct Log {
    keys: Vec<AppKey>,
    left: usize,
    restarts: usize,
    taps: Vec<Point>,
}

/// An applet that writes down what it gets and draws a square where it is told.
struct Probe {
    log: Rc<RefCell<Log>>,
    soft: Vec<String>,
}

impl Applet for Probe {
    fn key(&mut self, key: AppKey, _host: &mut dyn Host) {
        self.log.borrow_mut().keys.push(key);
    }

    fn draw(&self, canvas: &mut Canvas) {
        canvas.clear(colors::surface());
        canvas.rect(0, -40, 20, 60, colors::accent());
    }

    fn soft_keys(&self) -> Vec<String> {
        self.soft.clone()
    }

    fn tap(&mut self, point: Point, _host: &mut dyn Host) {
        self.log.borrow_mut().taps.push(point);
    }

    fn restart(&mut self, _host: &mut dyn Host) {
        self.log.borrow_mut().restarts += 1;
    }

    fn leave(&mut self, _host: &mut dyn Host) -> Option<String> {
        self.log.borrow_mut().left += 1;
        None
    }
}

fn info() -> AppInfo {
    AppInfo { id: "probe".into(), name: "Probe".into(), kind: AppKind::Game, api: 1, built_in: true, usable: true, ..AppInfo::default() }
}

/// The app with the probe opened from the calculation, and what the probe sees.
fn opened(soft: &[&str]) -> (App<Stub>, Rc<RefCell<Log>>) {
    let mut app = App::new(Stub);
    let log = Rc::new(RefCell::new(Log::default()));
    let probe = Probe { log: log.clone(), soft: soft.iter().map(|s| s.to_string()).collect() };
    app.open_applet(Box::new(probe), info());
    (app, log)
}

fn press(app: &mut App<Stub>, keys: &[Key]) {
    keys.iter().for_each(|&key| app.key(key));
}

#[test]
fn keys_arrive_in_order() {
    let (mut app, log) = opened(&["A", "B", "C", "D"]);
    assert_eq!(app.screen, Screen::App);
    press(&mut app, &[Key::Up, Key::Digit(5), Key::Exe, Key::Soft(3)]);
    assert_eq!(log.borrow().keys, [AppKey::Up, AppKey::Digit(5), AppKey::Exe, AppKey::Soft(3)]);
}

#[test]
fn modifiers_reach_nothing() {
    let (mut app, log) = opened(&[]);
    press(&mut app, &[Key::Shift, Key::Digit(1), Key::Alpha, Key::Digit(2), Key::Sin]);
    // SHIFT and ALPHA wait for nothing: the digits come as they are.
    assert_eq!(log.borrow().keys, [AppKey::Digit(1), AppKey::Digit(2)]);
    assert_eq!(app.modifier.pending, None);
}

#[test]
fn ac_goes_back_to_where_the_app_was_opened() {
    let (mut app, log) = opened(&[]);
    app.key(Key::Ac);
    assert_eq!(app.screen, Screen::Calc);
    assert!(app.running.is_none());
    assert_eq!(log.borrow().left, 1);

    // From the mode menu, back to it.
    let mut app = App::new(Stub);
    app.key(Key::Mode);
    let log = Rc::new(RefCell::new(Log::default()));
    app.open_applet(Box::new(Probe { log: log.clone(), soft: Vec::new() }), info());
    app.key(Key::Ac);
    assert!(matches!(app.screen, Screen::Modes { .. }));
}

#[test]
fn mode_leaves_to_the_mode_menu() {
    let (mut app, log) = opened(&[]);
    app.mode = Mode::Algebra;
    app.key(Key::Mode);
    assert_eq!(app.screen, Screen::Modes { selected: 1 });
    assert_eq!(log.borrow().left, 1);
    assert!(log.borrow().keys.is_empty());
}

#[test]
fn keys_map_as_the_api_names_them() {
    let named = [
        (Key::Up, "up"),
        (Key::Down, "down"),
        (Key::Left, "left"),
        (Key::Right, "right"),
        (Key::Ok, "ok"),
        (Key::Exe, "exe"),
        (Key::Del, "del"),
        (Key::Digit(0), "0"),
        (Key::Digit(7), "7"),
        (Key::Plus, "+"),
        (Key::Minus, "-"),
        (Key::Times, "*"),
        (Key::Divide, "/"),
        (Key::Fraction, "/"),
        (Key::Equals, "="),
        (Key::Comma, "."),
        (Key::Char('.'), "."),
        (Key::Soft(0), "soft1"),
        (Key::Soft(11), "soft12"),
    ];
    for (key, name) in named {
        assert_eq!(AppKey::from_key(key).map(AppKey::name).as_deref(), Some(name), "{key:?}");
    }
    let none = [Key::Ac, Key::Mode, Key::Optn, Key::Shift, Key::Alpha, Key::Unit, Key::Symb, Key::Sin, Key::Letter('a'), Key::Calc];
    for key in none {
        assert_eq!(AppKey::from_key(key), None, "{key:?}");
    }
}

/// Where the shapes of the same calls land, on a canvas at `area`.
fn drawn(area: Rectangle) -> Vec<Rectangle> {
    let mut canvas = Canvas::new(area);
    canvas.rect(10, 20, 30, 40, colors::text());
    canvas.circle(0, 0, 10, colors::text());
    canvas.text(5, 5, "Hallo", colors::text());
    canvas
        .shapes()
        .iter()
        .map(|shape| match shape {
            Shape::Fill(rect, _) => *rect,
            Shape::Circle(circle, _) => circle.bounding_box(),
            Shape::Text(at, ..) => Rectangle::new(*at, Size::zero()),
            other => panic!("unexpected {other:?}"),
        })
        .collect()
}

#[test]
fn the_canvas_moves_by_its_area() {
    let big = Rectangle::new(Point::new(0, 26), Size::new(480, 250));
    let small = Rectangle::new(Point::new(0, 26), Size::new(320, 214));
    for area in [big, small] {
        let shapes = drawn(area);
        assert_eq!(shapes[0], Rectangle::new(Point::new(10, 46), Size::new(30, 40)));
        assert_eq!(shapes[1], Rectangle::new(Point::new(0, 26), Size::new(10, 10)));
        // Text's top at y: its baseline lower by the font's ascent.
        assert!(shapes[2].top_left.y > 31 && shapes[2].top_left.x == 5);
    }
    let canvas = Canvas::new(small);
    assert_eq!((canvas.width(), canvas.height()), (320, 214));
}

#[test]
fn a_canvas_keeps_its_limit() {
    let mut canvas = Canvas::new(Rectangle::new(Point::zero(), Size::new(480, 294)));
    for _ in 0..quantus_ui::applet::MAX_SHAPES + 10 {
        canvas.rect(0, 0, 1, 1, colors::text());
    }
    assert_eq!(canvas.shapes().len(), quantus_ui::applet::MAX_SHAPES);
    assert!(canvas.dropped());
    // A negative size is no size.
    let mut canvas = Canvas::new(Rectangle::new(Point::zero(), Size::new(10, 10)));
    canvas.rect(0, 0, -5, 3, colors::text());
    assert!(matches!(canvas.shapes()[0], Shape::Fill(rect, _) if rect.size == Size::new(0, 3)));
}

#[test]
fn colours_by_name() {
    assert_eq!(Palette::get("background"), Some(colors::background()));
    let tiles = colors::tiles();
    assert_eq!(Palette::get("tile1"), Some(tiles[0].1));
    assert_eq!(Palette::get(&format!("tile{}", tiles.len())), Some(tiles[tiles.len() - 1].1));
    assert_eq!(Palette::get("tile0"), None);
    assert_eq!(Palette::get(&format!("tile{}", tiles.len() + 1)), None);
    assert_eq!(Palette::get("pink"), None);
}

#[test]
fn the_applet_draws_below_the_bar() {
    let (app, _) = opened(&[]);
    let frame = quantus_ui::render::Frame::new(&app);
    let area = quantus_ui::render::app_area(&app);
    assert_eq!(area, Rectangle::new(Point::new(0, 26), Size::new(480, 294)));
    // The clear fills the area, the square is moved by its origin.
    let clear = frame.0.iter().position(|shape| matches!(shape, Shape::Fill(rect, color) if *rect == area && *color == colors::surface()));
    let square = frame.0.iter().position(|shape| matches!(shape, Shape::Fill(rect, color)
        if *color == colors::accent() && *rect == Rectangle::new(Point::new(0, -14), Size::new(20, 60))));
    // The bar's fill comes after both, so what reaches above the area is covered.
    let bar = frame.0.iter().position(|shape| matches!(shape, Shape::Fill(rect, _) if rect.top_left == Point::zero() && rect.size == Size::new(480, 26)));
    let (clear, square, bar) = (clear.expect("cleared"), square.expect("square"), bar.expect("bar"));
    assert!(clear < square && square < bar);
    // The app's name is the title.
    assert!(frame.0.iter().any(|shape| matches!(shape, Shape::Text(_, text, ..) if text == "Probe")));
}

/// An applet that wants to be drawn on every tick from 1 s on.
struct Clock;

impl Applet for Clock {
    fn key(&mut self, _key: AppKey, _host: &mut dyn Host) {}

    fn tick(&mut self, now_ms: u64, _host: &mut dyn Host) -> bool {
        now_ms >= 1000
    }

    fn draw(&self, _canvas: &mut Canvas) {}
}

#[test]
fn a_ticking_applet_redraws() {
    let mut app = App::new(Stub);
    app.open_applet(Box::new(Clock), info());
    app.tick(0);
    // Between blinks of the caret, only the applet asks for a redraw.
    assert!(!app.tick(100));
    app.tick(600);
    assert!(!app.tick(900));
    assert!(app.tick(1000));
}

#[test]
fn soft_keys_make_the_area_lower() {
    let (app, _) = opened(&["Neu", "2 Spieler"]);
    assert_eq!(quantus_ui::render::app_area(&app).size, Size::new(480, 250));
    let (mut app, log) = opened(&[]);
    assert_eq!(quantus_ui::render::app_area(&app).size, Size::new(480, 294));
    // Without strips the soft keys are dead.
    app.key(Key::Soft(0));
    assert!(log.borrow().keys.is_empty());
    // Only blank labels: no strips either.
    let (app, _) = opened(&["", " "]);
    assert!(!quantus_ui::soft::shows(&app));
}

#[test]
fn labels_are_plain_text() {
    let (app, _) = opened(&["2 Spieler", "x_1^2"]);
    let frame = quantus_ui::render::Frame::new(&app);
    let texts: Vec<&str> = frame.0.iter().filter_map(|shape| match shape {
        Shape::Text(_, text, ..) => Some(text.as_str()),
        _ => None,
    }).collect();
    assert!(texts.contains(&"2 Spieler") && texts.contains(&"x_1^2"), "{texts:?}");
}

#[test]
fn soft_keys_page_and_count_over_all_pages() {
    let labels: Vec<String> = (1..=13).map(|n| format!("K{n}")).collect();
    let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
    let (mut app, log) = opened(&labels);
    let bar = quantus_ui::soft::bar(&app).expect("strips");
    assert_eq!((bar.pages, bar.slots.len()), (2, 12));
    assert_eq!(bar.slot(11), Some(&quantus_ui::soft::Slot::Page));
    // The last key turns the page; then the first key is the twelfth label.
    press(&mut app, &[Key::Soft(0), Key::Soft(11), Key::Soft(0), Key::Soft(1)]);
    assert_eq!(log.borrow().keys, [AppKey::Soft(0), AppKey::Soft(11), AppKey::Soft(12)]);
    // The small unit: rows of five.
    let (mut app, _) = opened(&["A", "B", "C", "D", "E", "F"]);
    app.soft_columns = 5;
    let bar = quantus_ui::soft::bar(&app).expect("strips");
    assert_eq!(bar.columns, 5);
    assert_eq!(bar.slot(5), Some(&quantus_ui::soft::Slot::App(5, "F".into())));
}

#[test]
fn a_blank_label_is_dead() {
    let (mut app, log) = opened(&["Neu", "", "Stufe"]);
    press(&mut app, &[Key::Soft(1), Key::Soft(2)]);
    assert_eq!(log.borrow().keys, [AppKey::Soft(2)]);
}

/// The texts of the frame, in order.
fn texts(app: &App<Stub>) -> Vec<String> {
    let frame = quantus_ui::render::Frame::new(app);
    frame.0.iter().filter_map(|shape| match shape {
        Shape::Text(_, text, ..) => Some(text.clone()),
        _ => None,
    }).collect()
}

#[test]
fn optn_opens_the_apps_menu_over_it() {
    let (mut app, log) = opened(&["Neu"]);
    app.key(Key::Optn);
    let shown = texts(&app);
    for entry in ["Info", "Neu starten", "Daten löschen", "Beenden"] {
        assert!(shown.iter().any(|text| text == entry), "{entry} in {shown:?}");
    }
    // The menu takes the keys, the app gets none; its soft keys are gone meanwhile.
    assert!(!quantus_ui::soft::shows(&app));
    press(&mut app, &[Key::Down, Key::Up, Key::Soft(0)]);
    assert!(log.borrow().keys.is_empty());
    // AC closes the menu, back to the app.
    app.key(Key::Ac);
    assert_eq!(app.screen, Screen::App);
    assert!(app.running.as_ref().is_some_and(|running| running.menu.is_none()));
    app.key(Key::Up);
    assert_eq!(log.borrow().keys, [AppKey::Up]);
}

#[test]
fn info_shows_the_credits() {
    let (mut app, _) = opened(&[]);
    press(&mut app, &[Key::Optn, Key::Digit(1)]);
    let shown = texts(&app);
    assert!(shown.iter().any(|text| text == "Autor unbekannt"), "{shown:?}");
    assert!(shown.iter().any(|text| text == "API 1 · Eingebaut"), "{shown:?}");
    // Any key goes back to the menu.
    app.key(Key::Ok);
    assert!(texts(&app).iter().any(|text| text == "Beenden"));

    let mut app = App::new(Stub);
    let mut credits = info();
    credits.authors = vec!["Ada".into(), "Grace".into()];
    credits.version = Some("1.2".into());
    credits.built_in = false;
    app.open_applet(Box::new(Clock), credits);
    press(&mut app, &[Key::Optn, Key::Digit(1)]);
    let shown = texts(&app);
    for line in ["Autoren: Ada, Grace", "Version: 1.2", "API 1 · SD-Karte, probe.rhai"] {
        assert!(shown.iter().any(|text| text == line), "{line} in {shown:?}");
    }
}

#[test]
fn restart_and_quit() {
    let (mut app, log) = opened(&[]);
    press(&mut app, &[Key::Optn, Key::Digit(2)]);
    assert_eq!(log.borrow().restarts, 1);
    assert_eq!(app.screen, Screen::App);
    // No data yet: the entry says so.
    press(&mut app, &[Key::Optn, Key::Digit(3)]);
    assert!(texts(&app).iter().any(|text| text == "Keine Daten"));
    press(&mut app, &[Key::Ok, Key::Digit(4)]);
    assert_eq!(app.screen, Screen::Calc);
    assert!(app.running.is_none());
    assert_eq!(log.borrow().left, 1);
}

fn tap(app: &mut App<Stub>, point: Point) {
    use quantus_ui::touch::Touch;
    app.touch(Touch::Press(point));
    app.touch(Touch::Release);
    let _ = quantus_ui::render::Frame::new(app);
}

#[test]
fn taps_reach_the_app_in_its_coordinates() {
    let (mut app, log) = opened(&["Neu", "Stufe"]);
    tap(&mut app, Point::new(100, 126));
    assert_eq!(log.borrow().taps, [Point::new(100, 100)]);
    // A label is its soft key.
    let label = quantus_ui::render::slot_rect(1, 6).center();
    tap(&mut app, label);
    assert_eq!(log.borrow().keys, [AppKey::Soft(1)]);
    // The left of the top bar leaves.
    tap(&mut app, Point::new(20, 12));
    assert_eq!(app.screen, Screen::Calc);
    assert_eq!(log.borrow().left, 1);
}

#[test]
fn taps_on_the_apps_menu() {
    let (mut app, log) = opened(&[]);
    app.key(Key::Optn);
    // The fourth line, Beenden.
    let line = quantus_ui::render::line_rect(3, 470).center();
    tap(&mut app, line);
    assert_eq!(app.screen, Screen::Calc);
    assert!(log.borrow().taps.is_empty());
}

#[test]
fn page_three_has_the_apps_and_the_setup() {
    use quantus_ui::render::{dot_rect, Frame};
    let mut app = App::new(Stub);
    app.key(Key::Mode);
    // Down from tile 10 to tile 13, on the third page.
    app.screen = Screen::Modes { selected: 9 };
    app.key(Key::Down);
    assert_eq!(app.screen, Screen::Modes { selected: 12 });
    assert_eq!(Mode::ALL[12], Mode::Apps);
    assert_eq!(Mode::ALL[13], Mode::Setup);
    let frame = Frame::new(&app);
    let names: Vec<&str> = frame.0.iter().filter_map(|shape| match shape {
        Shape::Text(_, text, ..) => Some(text.as_str()),
        _ => None,
    }).collect();
    assert!(names.contains(&"Apps") && names.contains(&"Setup"), "{names:?}");
    // The third dot is drawn.
    let third = dot_rect(2);
    assert!(frame.0.iter().any(|shape| matches!(shape, Shape::Round(round, ..) if round.rectangle == third)
        || matches!(shape, Shape::Circle(circle, _) if circle.bounding_box() == third)
        || matches!(shape, Shape::Fill(rect, _) if *rect == third)));
}

#[test]
fn the_setup_tile_opens_setup_and_keeps_the_mode() {
    let mut app = App::new(Stub);
    app.mode = Mode::Algebra;
    app.settings.exam = true;
    app.key(Key::Mode);
    app.screen = Screen::Modes { selected: 13 };
    app.key(Key::Ok);
    let Screen::Menu(menu) = &app.screen else { panic!("no menu") };
    assert_eq!(menu.level().page, &quantus_ui::tree::SETUP);
    assert_eq!(app.mode, Mode::Algebra);
}

/// A backend with a game built in and a tool from the card, the tool asking for a newer API.
struct Shelf {
    log: Rc<RefCell<Log>>,
}

impl quantus_ui::backend::Backend for Shelf {
    fn evaluate(&mut self, mode: Mode, settings: &quantus_ui::app::Settings, latex: &str) -> Result<String, String> {
        Stub.evaluate(mode, settings, latex)
    }

    fn apps(&mut self, _lang: quantus_ui::text::Lang) -> Vec<AppInfo> {
        let tool = AppInfo {
            id: "teiler".into(),
            name: "Teiler".into(),
            kind: AppKind::Tool,
            authors: vec!["Ada".into()],
            api: 9,
            built_in: false,
            usable: false,
            ..AppInfo::default()
        };
        let broken = AppInfo { id: "kaputt".into(), name: "Kaputt".into(), kind: AppKind::Game, ..info() };
        vec![tool, info(), broken]
    }

    fn open_app(&mut self, id: &str) -> Result<Box<dyn Applet>, String> {
        match id {
            "probe" => Ok(Box::new(Probe { log: self.log.clone(), soft: Vec::new() })),
            _ => Err("Zeile 3: unbekannte Funktion".into()),
        }
    }
}


fn shelf() -> (App<Shelf>, Rc<RefCell<Log>>) {
    let log = Rc::new(RefCell::new(Log::default()));
    let mut app = App::new(Shelf { log: log.clone() });
    app.key(Key::Mode);
    app.screen = Screen::Modes { selected: 12 };
    app.key(Key::Ok);
    (app, log)
}

fn shelf_texts(app: &App<Shelf>) -> Vec<String> {
    let frame = quantus_ui::render::Frame::new(app);
    frame.0.iter().filter_map(|shape| match shape {
        Shape::Text(_, text, ..) => Some(text.clone()),
        _ => None,
    }).collect()
}

#[test]
fn the_list_groups_the_apps() {
    use quantus_ui::menu::{app_lines, AppLine, Edit};
    let (app, _) = shelf();
    let Screen::Menu(menu) = &app.screen else { panic!("no menu") };
    let Some(Edit::Apps { apps, selected, .. }) = &menu.edit else { panic!("no list") };
    // Games first, then tools; a heading over each; no hint, as one app is from the card.
    let names: Vec<&str> = apps.iter().map(|app| app.name.as_str()).collect();
    assert_eq!(names, ["Probe", "Kaputt", "Teiler"]);
    assert_eq!(
        app_lines(apps),
        [AppLine::Heading(AppKind::Game), AppLine::App(0), AppLine::App(1), AppLine::Heading(AppKind::Tool), AppLine::App(2)]
    );
    assert_eq!(*selected, 1);
    let shown = shelf_texts(&app);
    for text in ["Spiele", "Werkzeuge", "Probe", "Teiler", "Ada"] {
        assert!(shown.iter().any(|shown| shown == text), "{text} in {shown:?}");
    }
    // Only built-in apps: the hint at the end.
    let lines = app_lines(&[info()]);
    assert_eq!(lines.last(), Some(&AppLine::Hint));
}

#[test]
fn ok_opens_and_ac_comes_back_to_the_list() {
    let (mut app, log) = shelf();
    app.key(Key::Ok);
    assert_eq!(app.screen, Screen::App);
    app.key(Key::Digit(3));
    assert_eq!(log.borrow().keys, [AppKey::Digit(3)]);
    app.key(Key::Ac);
    assert!(matches!(&app.screen, Screen::Menu(menu) if matches!(menu.edit, Some(quantus_ui::menu::Edit::Apps { .. }))));
    // ↓ goes over the heading; a digit picks the line it stands on.
    press_on(&mut app, &[Key::Down, Key::Down]);
    let Screen::Menu(menu) = &app.screen else { panic!() };
    assert!(matches!(menu.edit, Some(quantus_ui::menu::Edit::Apps { selected: 4, .. })));
    press_on(&mut app, &[Key::Digit(2)]);
    assert_eq!(app.screen, Screen::App);
}

#[test]
fn a_newer_api_is_grey_and_says_so() {
    let (mut app, log) = shelf();
    // Teiler, the fifth line.
    app.key(Key::Digit(5));
    assert!(shelf_texts(&app).iter().any(|text| text.contains("neuere Firmware")));
    assert!(matches!(app.screen, Screen::Menu(_)));
    // A key goes back to the list.
    app.key(Key::Ok);
    assert!(shelf_texts(&app).iter().any(|text| text == "Teiler"));
    assert!(log.borrow().keys.is_empty());
}

fn press_on(app: &mut App<Shelf>, keys: &[Key]) {
    keys.iter().for_each(|&key| app.key(key));
}
