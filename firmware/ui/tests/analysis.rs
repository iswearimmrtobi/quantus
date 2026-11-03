//! Kurvendiskussion: EXE sketches the function on the input line, its pages turn with ←/→, and
//! OPTN opens its tasks.

use std::cell::RefCell;
use std::rc::Rc;

use quantus_ui::app::{App, Mode, Settings};
use quantus_ui::backend::{Backend, Opened, Reply, Request, SessionCommand, Stub};
use quantus_ui::key::Key;

/// Keeps what it was asked and answers as the stub does, now or later.
#[derive(Clone, Default)]
struct Asked(Rc<RefCell<State>>);

#[derive(Default)]
struct State {
    asked: Vec<Request>,
    later: bool,
    ready: Option<Result<Reply, String>>,
    /// What the session opened at the start holds.
    inputs: Vec<Request>,
}

impl Backend for Asked {
    fn evaluate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> Result<String, String> {
        Stub.evaluate(mode, settings, latex)
    }

    fn start_solve(&mut self, settings: &Settings, request: &Request) -> Option<Result<Reply, String>> {
        let reply = Stub.solve(settings, request);
        let mut state = self.0.borrow_mut();
        state.asked.push(request.clone());
        if state.later {
            state.ready = Some(reply);
            return None;
        }
        Some(reply)
    }

    fn poll_solve(&mut self) -> Option<Result<Reply, String>> {
        self.0.borrow_mut().ready.take()
    }

    fn drop_pending(&mut self) {
        self.0.borrow_mut().ready = None;
    }

    fn session(&mut self, _command: SessionCommand, _settings: &Settings) -> Result<Opened, String> {
        Ok(Opened { inputs: self.0.borrow().inputs.clone(), ..Opened::default() })
    }
}

fn app() -> (App<Asked>, Asked) {
    let backend = Asked::default();
    let mut app = App::new(backend.clone());
    app.mode = Mode::Analysis;
    (app, backend)
}

fn type_text(app: &mut App<Asked>, text: &str) {
    for c in text.chars() {
        app.key(Key::from_char(c).expect("a key"));
    }
}

#[test]
fn exe_sketches_the_function_and_the_pages_turn() {
    let (mut app, backend) = app();
    type_text(&mut app, "x^2");
    app.key(Key::Exe);
    assert_eq!(backend.0.borrow().asked, [Request::Analysis { f: "x^{2}".into() }]);
    let answer = app.answer.as_ref().expect("the sketch is shown");
    // The graph, then the stub's two sections.
    assert_eq!((answer.page, answer.pages()), (0, 3));
    app.key(Key::Right);
    app.key(Key::Right);
    app.key(Key::Right);
    assert_eq!(app.answer.as_ref().unwrap().page, 2);
    app.key(Key::Left);
    assert_eq!(app.answer.as_ref().unwrap().page, 1);
    // AC goes back to the input, as it was.
    app.key(Key::Ac);
    assert!(app.answer.is_none());
    assert_eq!(app.editor.to_latex(), "x^{2}");
    // Nothing went into the history.
    assert!(app.history.is_empty());
}

#[test]
fn a_key_to_type_leaves_the_sketch_and_types() {
    let (mut app, _) = app();
    type_text(&mut app, "x");
    app.key(Key::Exe);
    app.key(Key::Plus);
    assert!(app.answer.is_none());
    assert_eq!(app.editor.to_latex(), "x+");
}

#[test]
fn the_spinner_turns_while_the_sketch_is_worked_out_and_ac_drops_it() {
    let (mut app, backend) = app();
    backend.0.borrow_mut().later = true;
    type_text(&mut app, "x");
    app.key(Key::Exe);
    assert!(app.busy && app.answer.is_none());
    app.key(Key::Ac);
    assert!(!app.busy);
    app.tick(1000);
    assert!(app.answer.is_none());

    app.key(Key::Exe);
    app.tick(2000);
    assert!(app.answer.is_some() && !app.busy);
}

#[test]
fn another_mode_leaves_the_sketch() {
    let (mut app, _) = app();
    type_text(&mut app, "x");
    app.key(Key::Exe);
    app.key(Key::Mode);
    app.key(Key::Digit(2));
    assert_eq!(app.mode, Mode::Algebra);
    assert!(app.answer.is_none());
}

/// Every text drawn on the screen.
fn texts(app: &App<Asked>) -> Vec<String> {
    let frame = quantus_ui::render::Frame::new(app);
    let debug = format!("{:?}", frame.0);
    debug.split("Text(").skip(1).filter_map(|part| part.split('"').nth(1).map(String::from)).collect()
}

#[test]
fn a_section_page_shows_its_title_and_where_it_stands() {
    let (mut app, _) = app();
    type_text(&mut app, "x");
    app.key(Key::Exe);
    assert!(texts(&app).iter().any(|text| text == "1/3"));
    app.key(Key::Right);
    let shown = texts(&app);
    assert!(shown.iter().any(|text| text == "f(x)") && shown.iter().any(|text| text == "2/3"), "{shown:?}");
    // The stub's second section failed: it says so, and why.
    app.key(Key::Right);
    let shown = texts(&app);
    assert!(shown.iter().any(|text| text == "fehlgeschlagen") && shown.iter().any(|text| text == "Stub"), "{shown:?}");
}

#[test]
fn the_first_page_draws_the_graph_with_its_points() {
    let (mut app, _) = app();
    type_text(&mut app, "x^2");
    app.key(Key::Exe);
    let frame = quantus_ui::render::Frame::new(&app);
    let shapes = format!("{:?}", frame.0);
    // The stub's parabola, and its vertex named T.
    assert!(shapes.matches("Path(").count() >= 1);
    assert!(shapes.contains("Circle("));
    assert!(texts(&app).iter().any(|text| text == "1/3"));
    // Its points stay on the screen.
    for shape in &frame.0 {
        // The ‹ in the top bar aside.
        if let quantus_ui::render::Shape::Path(points, ..) = shape {
            if points.iter().all(|p| p.y < 26) {
                continue;
            }
            assert!(points.iter().all(|p| (0..480).contains(&p.x) && (26..320).contains(&p.y)), "{points:?}");
        }
    }
}

fn tap(app: &mut App<Asked>, point: embedded_graphics::prelude::Point) {
    use quantus_ui::touch::Touch;
    app.touch(Touch::Press(point));
    app.touch(Touch::Release);
}

#[test]
fn a_finger_turns_the_pages_and_the_bar_goes_back() {
    use embedded_graphics::prelude::Point;
    use quantus_ui::touch::Touch;
    let (mut app, _) = app();
    type_text(&mut app, "x");
    app.key(Key::Exe);
    // A swipe to the left: the next page.
    app.touch(Touch::Press(Point::new(300, 200)));
    for x in [260, 220, 180] {
        app.touch(Touch::Drag(Point::new(x, 205)));
    }
    app.touch(Touch::Release);
    assert_eq!(app.answer.as_ref().unwrap().page, 1);
    // A tap at the right edge, then the left one.
    tap(&mut app, Point::new(470, 200));
    assert_eq!(app.answer.as_ref().unwrap().page, 2);
    tap(&mut app, Point::new(470, 200));
    assert_eq!(app.answer.as_ref().unwrap().page, 2, "there is no page after the last");
    tap(&mut app, Point::new(10, 200));
    assert_eq!(app.answer.as_ref().unwrap().page, 1);
    // ‹ in the top bar: back to the input.
    tap(&mut app, Point::new(20, 12));
    assert!(app.answer.is_none());
    assert_eq!(app.editor.to_latex(), "x");
}

#[test]
fn optn_opens_the_tangent_form_on_the_function_typed() {
    let (mut app, backend) = app();
    type_text(&mut app, "x^2");
    app.key(Key::Optn);
    app.key(Key::Digit(2));
    assert_eq!(app.sheet_kind(), Some(quantus_ui::sheet::Kind::Tangent));
    assert!(matches!(app.screen, quantus_ui::app::Screen::Calc));
    // x₀ = 3, kept with EXE; EXE on the last field works it out.
    app.key(Key::Digit(3));
    app.key(Key::Exe);
    app.key(Key::Exe);
    assert_eq!(backend.0.borrow().asked.last(), Some(&Request::Tangent { f: "x^{2}".into(), x0: "3".into() }));
    assert!(matches!(app.answer.as_ref().unwrap().reply, Ok(Reply::Task(_))));
    // AC: back to the form, then to the input.
    app.key(Key::Ac);
    assert!(app.answer.is_none() && app.sheet_kind().is_some());
    app.key(Key::Ac);
    assert_eq!(app.sheet_kind(), None);
    assert_eq!(app.editor.to_latex(), "x^{2}");
}

#[test]
fn the_area_and_the_table_forms_send_their_fields() {
    let (mut app, backend) = app();
    type_text(&mut app, "x");
    app.key(Key::Optn);
    app.key(Key::Digit(3));
    // Both bounds empty: EXE, EXE goes to b, then works it out.
    app.key(Key::Exe);
    app.key(Key::Exe);
    assert_eq!(backend.0.borrow().asked.last(), Some(&Request::Area { f: "x".into(), from: "".into(), to: "".into() }));
    app.key(Key::Ac);
    app.key(Key::Ac);
    app.key(Key::Optn);
    app.key(Key::Digit(4));
    type_text(&mut app, "0\n2\n1\n");
    app.key(Key::Exe);
    let table = Request::Table { f: "x".into(), g: None, from: "0".into(), to: "2".into(), step: "1".into() };
    assert_eq!(backend.0.borrow().asked.last(), Some(&table));
}

#[test]
fn abschnitte_lists_the_pages_of_the_last_sketch() {
    use quantus_ui::menu::Edit;
    let (mut app, _) = app();
    // Before a sketch there is nothing to list.
    app.key(Key::Optn);
    app.key(Key::Digit(1));
    let quantus_ui::app::Screen::Menu(menu) = &app.screen else { panic!("OPTN is open") };
    assert!(matches!(menu.edit, Some(Edit::Message { .. })));
    app.key(Key::Ac);

    type_text(&mut app, "x");
    app.key(Key::Exe);
    app.key(Key::Ac);
    app.key(Key::Optn);
    app.key(Key::Digit(1));
    let quantus_ui::app::Screen::Menu(menu) = &app.screen else { panic!("OPTN is open") };
    let Some(Edit::Sections { titles, .. }) = &menu.edit else { panic!("the sections") };
    assert_eq!(titles, &["Graph", "f(x)", "?"]);
    // The third: the stub's second section.
    app.key(Key::Digit(3));
    assert!(matches!(app.screen, quantus_ui::app::Screen::Calc));
    assert_eq!(app.answer.as_ref().unwrap().page, 2);
}

#[test]
fn a_task_on_the_function_sketched_has_its_graph_on_a_second_page() {
    let (mut app, _) = app();
    type_text(&mut app, "x^2");
    // Without a sketch: the steps only.
    app.key(Key::Optn);
    app.key(Key::Digit(2));
    type_text(&mut app, "1\n\n");
    assert_eq!(app.answer.as_ref().unwrap().pages(), 1);
    app.key(Key::Ac);
    app.key(Key::Ac);

    // After the sketch: its graph with the tangent on it, a page further.
    app.key(Key::Exe);
    app.key(Key::Ac);
    app.key(Key::Optn);
    app.key(Key::Digit(2));
    app.key(Key::Exe);
    let answer = app.answer.as_ref().unwrap();
    assert_eq!(answer.pages(), 2);
    let Ok(Reply::Task(task)) = &answer.reply else { panic!("a task") };
    let graph = task.graph.as_ref().unwrap();
    assert!(graph.overlay.tangent.is_some() && !graph.points.is_empty());
    app.key(Key::Right);
    assert_eq!(app.answer.as_ref().unwrap().page, 1);
    // A sketch of another function does not lend its graph.
    app.key(Key::Ac);
    app.key(Key::Ac);
    app.key(Key::Del);
    type_text(&mut app, "3");
    app.key(Key::Optn);
    app.key(Key::Digit(2));
    app.key(Key::Exe);
    assert_eq!(app.answer.as_ref().unwrap().pages(), 1);
}

#[test]
fn the_function_a_session_kept_comes_back_into_the_input() {
    let backend = Asked::default();
    backend.0.borrow_mut().inputs = vec![Request::Analysis { f: "x^{2}-1".into() }];
    let mut app = App::new(backend);
    // Not into another mode's input.
    assert!(app.editor.is_empty());
    app.key(Key::Mode);
    app.key(Key::Digit(4));
    assert_eq!(app.mode, Mode::Analysis);
    assert_eq!(app.editor.to_latex(), "x^{2}-1");
}

#[test]
fn a_section_opens_on_its_result_and_down_shows_the_working() {
    let (mut app, _) = app();
    type_text(&mut app, "x^2");
    app.key(Key::Exe);
    app.key(Key::Right);
    // The stub's section: a note as its working, f as its result.
    let answer = app.answer.as_ref().unwrap();
    assert_eq!((answer.scroll, answer.working(), answer.lines()), (0, Some((1, 1)), 3));
    let shown = texts(&app);
    assert!(shown.iter().any(|text| text == "Rechenweg") && !shown.iter().any(|text| text == "f"), "{shown:?}");
    // DOWN: the working from its start, without the hint.
    app.key(Key::Down);
    let shown = texts(&app);
    assert!(shown.iter().any(|text| text == "f") && !shown.iter().any(|text| text == "Rechenweg"), "{shown:?}");
    // Further down there is no more; UP goes back to the result alone.
    app.key(Key::Down);
    app.key(Key::Down);
    assert_eq!(app.answer.as_ref().unwrap().scroll, 2);
    app.key(Key::Up);
    app.key(Key::Up);
    assert!(texts(&app).iter().any(|text| text == "Rechenweg"));
    // Turning the page comes back to the result view.
    app.key(Key::Down);
    app.key(Key::Right);
    app.key(Key::Left);
    assert_eq!(app.answer.as_ref().unwrap().scroll, 0);
}

fn window(app: &App<Asked>) -> quantus_ui::backend::Window {
    app.answer.as_ref().unwrap().graph().expect("a graph page").window
}

#[test]
fn ok_moves_the_graph_with_the_arrows_and_zooms_it_with_plus_and_minus() {
    use quantus_ui::backend::Window;
    let (mut app, backend) = app();
    type_text(&mut app, "x^2");
    app.key(Key::Exe);
    let home = window(&app);
    assert_eq!(home, Window { x_min: -3.0, x_max: 3.0, y_min: -1.0, y_max: 9.0 });
    // Before OK the arrows turn the pages.
    app.key(Key::Ok);
    assert!(app.answer.as_ref().unwrap().moving);
    app.key(Key::Right);
    assert_eq!(app.answer.as_ref().unwrap().page, 0, "moving, → does not turn the page");
    assert_eq!(window(&app), Window { x_min: -1.5, x_max: 4.5, ..home });
    // The curve is sampled for the new window.
    let Some(Request::Sample { f, window: asked }) = backend.0.borrow().asked.last().cloned() else { panic!("a sample") };
    assert_eq!((f.as_str(), asked), ("f(x) = x^{2}", window(&app)));
    let graph = app.answer.as_ref().unwrap().graph().unwrap();
    assert_eq!(graph.curve[0].first().map(|point| point.0), Some(-1.5));
    // + halves the window around its middle, − doubles it, 0 goes home.
    app.key(Key::Plus);
    assert_eq!(window(&app), Window { x_min: 0.0, x_max: 3.0, y_min: 1.5, y_max: 6.5 });
    app.key(Key::Minus);
    app.key(Key::Minus);
    assert_eq!(window(&app), Window { x_min: -4.5, x_max: 7.5, y_min: -6.0, y_max: 14.0 });
    app.key(Key::Digit(0));
    assert_eq!(window(&app), home);
    app.key(Key::Up);
    // AC stops moving and keeps the sketch; → turns the page again, and the window stays.
    app.key(Key::Ac);
    let answer = app.answer.as_ref().unwrap();
    assert!(!answer.moving);
    app.key(Key::Right);
    app.key(Key::Left);
    assert_eq!(window(&app), Window { y_min: 1.5, y_max: 11.5, ..home });
}

#[test]
fn moving_while_the_curve_is_sampled_asks_once_more_at_the_end() {
    let (mut app, backend) = app();
    type_text(&mut app, "x");
    app.key(Key::Exe);
    backend.0.borrow_mut().later = true;
    app.key(Key::Ok);
    app.key(Key::Right);
    app.key(Key::Right);
    app.key(Key::Right);
    // One sample asked; the graph moved on meanwhile.
    let samples = |backend: &Asked| backend.0.borrow().asked.iter().filter(|request| matches!(request, Request::Sample { .. })).count();
    assert_eq!(samples(&backend), 1);
    app.tick(100);
    // The first came back: the last window is asked for.
    assert_eq!(samples(&backend), 2);
    let Some(Request::Sample { window: asked, .. }) = backend.0.borrow().asked.last().cloned() else { panic!("a sample") };
    assert_eq!(asked, window(&app));
    app.tick(200);
    assert_eq!(samples(&backend), 2);
    assert!(!app.busy);
}
