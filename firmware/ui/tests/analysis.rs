//! Kurvendiskussion: EXE sketches the function on the input line, its pages turn with ←/→, and
//! OPTN opens its tasks.

use std::cell::RefCell;
use std::rc::Rc;

use quantus_ui::app::{App, Mode, Settings};
use quantus_ui::backend::{Backend, Reply, Request, Stub};
use quantus_ui::key::Key;

/// Keeps what it was asked and answers as the stub does, now or later.
#[derive(Clone, Default)]
struct Asked(Rc<RefCell<State>>);

#[derive(Default)]
struct State {
    asked: Vec<Request>,
    later: bool,
    ready: Option<Result<Reply, String>>,
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
