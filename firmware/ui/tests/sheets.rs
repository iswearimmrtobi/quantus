//! The sheets of LGS, Wertetabelle, Statistik and Verteilungen: moving and typing in the cells,
//! the request EXE sends, the answer over the sheet, and the sheets a session brings back.

use std::cell::RefCell;
use std::rc::Rc;

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use quantus_ui::app::{App, Mode, Screen, Settings};
use quantus_ui::backend::{Backend, Field, Line, Matrix, Opened, Reply, Request, SessionCommand, Stub};
use quantus_ui::key::Key;
use quantus_ui::render::{self, Frame, Renderer, HEIGHT, WIDTH};
use quantus_ui::settings::{DistributionKind, StatisticsType, SystemInput};
use quantus_ui::touch::Target;

/// Keeps what it was asked and answers as the test sets it, now or later.
#[derive(Clone, Default)]
struct Sheets(Rc<RefCell<State>>);

#[derive(Default)]
struct State {
    asked: Vec<Request>,
    /// Answer later, through `poll_solve`.
    later: bool,
    ready: Option<Result<Reply, String>>,
    dropped: usize,
    /// What the session opened at the start holds.
    inputs: Vec<Request>,
}

impl Backend for Sheets {
    fn evaluate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> Result<String, String> {
        Stub.evaluate(mode, settings, latex)
    }

    fn solve(&mut self, settings: &Settings, request: &Request) -> Result<Reply, String> {
        self.0.borrow_mut().asked.push(request.clone());
        Stub.solve(settings, request)
    }

    fn start_solve(&mut self, settings: &Settings, request: &Request) -> Option<Result<Reply, String>> {
        let reply = self.solve(settings, request);
        let mut state = self.0.borrow_mut();
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
        let mut state = self.0.borrow_mut();
        state.dropped += 1;
        state.ready = None;
    }

    fn session(&mut self, _command: SessionCommand, _settings: &Settings) -> Result<Opened, String> {
        Ok(Opened { inputs: self.0.borrow().inputs.clone(), ..Opened::default() })
    }
}

fn app(mode: Mode) -> (App<Sheets>, Sheets) {
    let backend = Sheets::default();
    let mut app = App::new(backend.clone());
    app.mode = mode;
    (app, backend)
}

fn press(app: &mut App<Sheets>, keys: &[Key]) {
    keys.iter().for_each(|&key| app.key(key));
}

fn asked(backend: &Sheets) -> Request {
    backend.0.borrow().asked.last().cloned().expect("a request")
}

fn strings(cells: &[&str]) -> Vec<String> {
    cells.iter().map(|cell| cell.to_string()).collect()
}

#[test]
fn lgs_opens_on_a_three_by_three_matrix() {
    let (app, _) = app(Mode::Linear);
    assert_eq!(app.settings.system_input, SystemInput::Matrix);
    let sheet = &app.sheets.linear;
    assert_eq!((sheet.rows(), sheet.columns()), (3, 4), "three unknowns and the right-hand side");
    assert_eq!(app.sheet_kind(), Some(quantus_ui::sheet::Kind::Linear));
}

#[test]
fn exe_goes_along_the_row_and_sends_the_matrix() {
    let (mut app, backend) = app(Mode::Linear);
    app.settings.system_rows = 2;
    app.settings.system_columns = 2;
    press(&mut app, &[Key::Ac]);
    use Key::{Digit as D, Exe};
    press(&mut app, &[D(2), Exe, D(1), Exe, D(5), Exe, D(1), Exe, Key::Minus, D(1), Exe, D(1), Exe]);
    assert_eq!(app.sheets.linear.cursor, (1, 2), "the last cell keeps the cursor");
    assert!(app.sheets.linear.editing.is_none());
    press(&mut app, &[Exe]);
    assert_eq!(asked(&backend), Request::Linear { rows: vec![strings(&["2", "1", "5"]), strings(&["1", "-1", "1"])] });
    let answer = app.answer.as_ref().expect("the answer shows");
    assert!(matches!(&answer.reply, Ok(Reply::Lines(lines)) if lines[0] == Line::Math("2; 1; 5".into())));

    // AC goes back to the sheet, as it was.
    press(&mut app, &[Key::Ac]);
    assert!(app.answer.is_none());
    assert_eq!(app.sheets.linear.latex(1, 1), "-1");
}

#[test]
fn a_cell_is_edited_with_templates_and_left_with_the_arrows() {
    let (mut app, _) = app(Mode::Linear);
    press(&mut app, &[Key::Digit(1), Key::Fraction, Key::Digit(2)]);
    assert_eq!(app.sheets.linear.editing.as_ref().unwrap().to_latex(), r"\frac{1}{2}");
    // ↑ and ↓ inside the fraction stay in the cell; ↓ from the denominator leaves it.
    press(&mut app, &[Key::Up, Key::Down]);
    assert_eq!(app.sheets.linear.cursor, (0, 0));
    press(&mut app, &[Key::Down]);
    assert_eq!(app.sheets.linear.cursor, (1, 0));
    assert_eq!(app.sheets.linear.latex(0, 0), r"\frac{1}{2}");
    // AC drops an edit, DEL empties a cell.
    press(&mut app, &[Key::Digit(7), Key::Ac]);
    assert_eq!(app.sheets.linear.latex(1, 0), "");
    press(&mut app, &[Key::Up, Key::Del]);
    assert_eq!(app.sheets.linear.latex(0, 0), "");
}

#[test]
fn exe_goes_from_cell_to_cell_and_works_the_sheet_out_on_the_last() {
    let (mut app, backend) = app(Mode::Distributions);
    press(&mut app, &[Key::Exe]);
    assert_eq!(app.sheets.distribution.cursor, (1, 0));
    press(&mut app, &[Key::Exe]);
    assert_eq!(app.sheets.distribution.cursor, (2, 0));
    assert!(backend.0.borrow().asked.is_empty(), "nothing worked out yet");
    press(&mut app, &[Key::Exe]);
    assert!(app.answer.is_some());

    // The LGS goes along the row, then on to the next one.
    let (mut app, _) = self::app(Mode::Linear);
    press(&mut app, &[Key::Exe, Key::Exe, Key::Exe, Key::Exe]);
    assert_eq!(app.sheets.linear.cursor, (1, 0));
}

#[test]
fn ok_edits_a_cell_as_it_is_and_a_key_starts_it_afresh() {
    let (mut app, _) = app(Mode::Linear);
    press(&mut app, &[Key::Digit(4), Key::Digit(2), Key::Exe, Key::Left]);
    press(&mut app, &[Key::Ok, Key::Digit(1), Key::Exe, Key::Left]);
    assert_eq!(app.sheets.linear.latex(0, 0), "421");
    press(&mut app, &[Key::Digit(9), Key::Exe]);
    assert_eq!(app.sheets.linear.latex(0, 0), "9");
}

#[test]
fn rows_and_columns_from_optn_resize_the_sheet_and_keep_the_cells() {
    let (mut app, _) = app(Mode::Linear);
    press(&mut app, &[Key::Digit(5), Key::Exe]);
    // OPTN → Zeilen → 5, OK.
    press(&mut app, &[Key::Optn, Key::Digit(2), Key::Digit(5), Key::Ok, Key::Ac]);
    assert_eq!(app.settings.system_rows, 5);
    assert_eq!((app.sheets.linear.rows(), app.sheets.linear.columns()), (5, 4));
    assert_eq!(app.sheets.linear.latex(0, 0), "5");
}

#[test]
fn equations_as_input_bring_back_the_input_line() {
    let (mut app, backend) = app(Mode::Linear);
    app.settings.system_input = SystemInput::Equations;
    press(&mut app, &[Key::Letter('x'), Key::Equals, Key::Digit(1), Key::Exe]);
    assert!(backend.0.borrow().asked.is_empty());
    assert_eq!(app.history.len(), 1);
}

#[test]
fn a_long_lgs_scrolls_to_the_cursor() {
    let (mut app, _) = app(Mode::Linear);
    app.settings.system_rows = 10;
    app.settings.system_columns = 10;
    press(&mut app, &[Key::Ac]);
    for _ in 0..9 {
        press(&mut app, &[Key::Down]);
    }
    for _ in 0..10 {
        press(&mut app, &[Key::Right]);
    }
    let sheet = &app.sheets.linear;
    assert_eq!(sheet.cursor, (9, 10));
    assert_eq!(sheet.top, 5);
    assert!(sheet.left > 0 && sheet.left + 5 >= 10, "the last column shows: {}", sheet.left);
}

#[test]
fn the_table_form_sends_f_g_and_the_range() {
    let (mut app, backend) = app(Mode::Table);
    use Key::{Digit as D, Exe};
    // EXE passes g(x) by; on the last field it works the form out.
    press(&mut app, &[Key::Letter('x'), Key::Square, Exe, Exe, D(0), Exe, D(5), Exe, D(1), Exe, Exe]);
    assert_eq!(
        asked(&backend),
        Request::Table { f: "x^{2}".into(), g: None, from: "0".into(), to: "5".into(), step: "1".into() }
    );
    assert!(matches!(&app.answer.as_ref().unwrap().reply, Ok(Reply::Table { .. })));
}

#[test]
fn a_list_grows_as_values_are_typed_and_sends_its_columns() {
    let (mut app, backend) = app(Mode::Statistics);
    use Key::{Digit as D, Exe};
    press(&mut app, &[D(1), Exe, D(2), Exe, D(4), Exe]);
    assert_eq!(app.sheets.statistics.rows(), 4, "an empty row waits at the end");
    assert_eq!(app.sheets.statistics.cursor, (3, 0));
    press(&mut app, &[Exe]);
    assert_eq!(
        asked(&backend),
        Request::Statistics { kind: StatisticsType::OneVariable, x: strings(&["1", "2", "4"]), y: None, freq: None, at: Default::default() }
    );

    // Pairs with a frequency column: the x values stay.
    press(&mut app, &[Key::Ac]);
    app.settings.statistics_type = StatisticsType::Linear;
    app.settings.statistics_freq = true;
    press(&mut app, &[Key::Up, Key::Up, Key::Up, Key::Right, D(3), Exe, D(5), Exe, D(9), Key::Right, D(2), Exe]);
    assert_eq!(app.sheets.statistics.columns(), 3);
    press(&mut app, &[Exe]);
    assert_eq!(
        asked(&backend),
        Request::Statistics {
            kind: StatisticsType::Linear,
            x: strings(&["1", "2", "4"]),
            y: Some(strings(&["3", "5", "9"])),
            freq: Some(strings(&["", "", "2"])),
            at: Default::default(),
        }
    );

    // Back to one variable and again to pairs: the y column was kept.
    app.settings.statistics_type = StatisticsType::OneVariable;
    press(&mut app, &[Key::Ac]);
    assert_eq!(app.sheets.statistics.columns(), 2);
    app.settings.statistics_type = StatisticsType::Quadratic;
    press(&mut app, &[Key::Ac]);
    assert_eq!(app.sheets.statistics.latex(2, 1), "9");
}

#[test]
fn optn_inserts_and_deletes_rows_of_a_list() {
    let (mut app, _) = app(Mode::Statistics);
    use Key::{Digit as D, Exe};
    press(&mut app, &[D(1), Exe, D(2), Exe, Key::Up]);
    // OPTN → Zeile einfügen (the third entry).
    press(&mut app, &[Key::Optn, D(3)]);
    assert_eq!(app.screen, Screen::Calc, "the action closes OPTN");
    assert_eq!(app.sheets.statistics.latex(1, 0), "");
    assert_eq!(app.sheets.statistics.latex(2, 0), "2");
    press(&mut app, &[Key::Optn, D(4)]);
    assert_eq!(app.sheets.statistics.latex(1, 0), "2");
    assert_eq!(app.sheets.statistics.rows(), 3);
}

#[test]
fn a_distribution_form_follows_its_type_and_keeps_shared_fields() {
    let (mut app, backend) = app(Mode::Distributions);
    assert_eq!(app.sheets.distribution_fields(), [Field::X, Field::Sigma, Field::Mu]);
    use Key::{Digit as D, Exe};
    press(&mut app, &[D(1), Exe, D(2), Exe, D(3), Exe, Exe]);
    assert_eq!(
        asked(&backend),
        Request::Distribution {
            kind: DistributionKind::NormalDensity,
            fields: vec![(Field::X, "1".into()), (Field::Sigma, "2".into()), (Field::Mu, "3".into())],
            k: Vec::new(),
        }
    );
    press(&mut app, &[Key::Ac]);

    // Poisson: k and μ; μ is the one typed for the normal distribution.
    app.settings.distribution = DistributionKind::PoissonDensity;
    press(&mut app, &[Key::Up]);
    assert_eq!(app.sheets.distribution.latex(1, 0), "3");
    press(&mut app, &[Key::Up, D(0), Key::Semicolon, D(1), Key::Semicolon, D(2), Exe, Exe]);
    assert_eq!(
        asked(&backend),
        Request::Distribution { kind: DistributionKind::PoissonDensity, fields: vec![(Field::Mu, "3".into())], k: strings(&["0", "1", "2"]) }
    );
}

#[test]
fn optn_inserts_into_the_cell() {
    let (mut app, _) = app(Mode::Distributions);
    // OPTN → Griechisch → σ.
    press(&mut app, &[Key::Optn, Key::Digit(3)]);
    let greek = app.screen.clone();
    assert!(matches!(greek, Screen::Menu(_)));
    press(&mut app, &[Key::Down, Key::Down, Key::Down, Key::Down, Key::Down, Key::Down, Key::Down, Key::Down, Key::Down, Key::Ok]);
    let editing = app.sheets.distribution.editing.as_ref().expect("the cell is edited");
    assert_eq!(editing.to_latex(), r"\sigma");
}

#[test]
fn the_spinner_turns_until_the_answer_comes_and_ac_drops_it() {
    let (mut app, backend) = app(Mode::Linear);
    backend.0.borrow_mut().later = true;
    app.sheets.linear.select(2, 3);
    press(&mut app, &[Key::Exe]);
    assert!(app.busy && app.answer.is_none());
    // Keys wait meanwhile.
    press(&mut app, &[Key::Digit(3)]);
    assert!(app.sheets.linear.editing.is_none());
    assert!(app.tick(10));
    assert!(!app.busy && app.answer.is_some());

    press(&mut app, &[Key::Ac, Key::Exe, Key::Ac]);
    assert!(!app.busy && app.answer.is_none());
    assert_eq!(backend.0.borrow().dropped, 1);
    app.tick(20);
    assert!(app.answer.is_none(), "a dropped answer does not show");
}

#[test]
fn the_answer_scrolls() {
    let (mut app, _) = app(Mode::Linear);
    app.settings.system_rows = 4;
    press(&mut app, &[Key::Ac]);
    app.sheets.linear.select(3, 3);
    press(&mut app, &[Key::Exe]);
    press(&mut app, &[Key::Down, Key::Down, Key::Down, Key::Down, Key::Down]);
    assert_eq!(app.answer.as_ref().unwrap().scroll, 3, "no further than the last line");
    press(&mut app, &[Key::Up, Key::Right]);
    let answer = app.answer.as_ref().unwrap();
    assert_eq!((answer.scroll, answer.shift), (2, 80));
}

#[test]
fn a_session_brings_its_sheets_back_with_their_layout() {
    let backend = Sheets::default();
    backend.0.borrow_mut().inputs = vec![
        Request::Linear { rows: vec![strings(&["2", "1", "5"]), strings(&["1", "-1", "1"])] },
        Request::Statistics { kind: StatisticsType::Power, x: strings(&["1", "2"]), y: Some(strings(&["3", "4"])), freq: None, at: Default::default() },
        Request::Distribution { kind: DistributionKind::BinomialCumulative, fields: vec![(Field::N, "10".into())], k: strings(&["1", "2"]) },
        Request::Table { f: "x".into(), g: None, from: "1".into(), to: "2".into(), step: "1".into() },
    ];
    let app = App::new(backend.clone());
    assert_eq!((app.settings.system_rows, app.settings.system_columns), (2, 2));
    assert_eq!(app.sheets.linear.latex(1, 1), "-1");
    assert_eq!(app.settings.statistics_type, StatisticsType::Power);
    assert_eq!(app.sheets.statistics.latex(1, 1), "4");
    assert_eq!(app.settings.distribution, DistributionKind::BinomialCumulative);
    assert_eq!(app.sheets.distribution.latex(0, 0), "1;2");
    assert_eq!(app.sheets.distribution.latex(1, 0), "10");
    assert_eq!(app.sheets.table.latex(3, 0), "2");
}

#[test]
fn a_tap_selects_a_cell_and_a_second_one_edits_it() {
    let (mut app, _) = app(Mode::Linear);
    let point = |app: &App<Sheets>, row, column| {
        let center = |x: i32, y: i32| Point::new(x, y);
        (0..WIDTH as i32)
            .step_by(4)
            .flat_map(|x| (0..HEIGHT as i32).step_by(4).map(move |y| center(x, y)))
            .find(|&p| render::target(app, p) == Some(Target::Cell(row, column)))
            .expect("the cell is on the screen")
    };
    let at = point(&app, 1, 2);
    use quantus_ui::touch::Touch;
    let tap = |app: &mut App<Sheets>, at: Point| {
        app.touch(Touch::Press(at));
        app.touch(Touch::Release);
    };
    tap(&mut app, at);
    assert_eq!(app.sheets.linear.cursor, (1, 2));
    assert!(app.sheets.linear.editing.is_none());
    tap(&mut app, at);
    assert!(app.sheets.linear.editing.is_some());
}

/// The screens of the sheets drawn, without a panic; with `QUANTUS_SHOTS` set to a folder,
/// written there as PPM pictures to look at.
#[test]
fn every_sheet_and_answer_draws() {
    let matrix = Matrix {
        names: strings(&[r"\text{I}", r"\text{II}"]),
        rows: vec![strings(&["2", "1", "5"]), strings(&["0", r"\frac{3}{2}", "-1"])],
        split: 2,
        notes: strings(&["", r"2 \cdot \text{II} - \text{I}"]),
    };
    let lines = Reply::Lines(vec![
        Line::Math("x_{1} = 2".into()),
        Line::Math(r"\bar{x} = 3".into()),
        Line::Note("Rechenweg".into()),
        Line::Matrix(matrix),
    ]);
    let table = Reply::Table { header: strings(&["x", "f(x)", "g(x)"]), rows: vec![strings(&["0", "0", ""]), strings(&["1", "1", "2"])] };
    type Shot = (&'static str, Mode, Option<Result<Reply, String>>);
    let shots: [Shot; 8] = [
        ("lgs-wide", Mode::Linear, None),
        ("lgs", Mode::Linear, None),
        ("lgs-answer", Mode::Linear, Some(Ok(lines))),
        ("table", Mode::Table, None),
        ("table-answer", Mode::Table, Some(Ok(table))),
        ("statistics", Mode::Statistics, None),
        ("distribution", Mode::Distributions, None),
        ("error", Mode::Statistics, Some(Err("Eine Regression braucht Paare x, y, und das ist eine lange Meldung.".into()))),
    ];
    for (name, mode, reply) in shots {
        let (mut app, _) = app(mode);
        if name == "lgs-wide" {
            (app.settings.system_rows, app.settings.system_columns) = (10, 10);
            press(&mut app, &[Key::Ac, Key::Down, Key::Down]);
            press(&mut app, &[Key::Digit(1); 14]);
            press(&mut app, &[Key::Exe, Key::Down, Key::Down, Key::Down, Key::Down, Key::Pi]);
        }
        press(&mut app, &[Key::Digit(1), Key::Fraction, Key::Digit(2), Key::Exe, Key::Digit(3)]);
        app.answer = reply.map(quantus_ui::app::Answer::new);
        let mut screen = Picture(vec![Rgb565::BLACK; (WIDTH * HEIGHT) as usize]);
        Renderer::new().render(&Frame::new(&app), &mut screen).unwrap();
        if let Ok(dir) = std::env::var("QUANTUS_SHOTS") {
            screen.save(&std::path::Path::new(&dir).join(format!("{name}.ppm")));
        }
    }
}

struct Picture(Vec<Rgb565>);

impl Picture {
    fn save(&self, path: &std::path::Path) {
        let mut out = format!("P6\n{WIDTH} {HEIGHT}\n255\n").into_bytes();
        for pixel in &self.0 {
            out.extend([pixel.r() << 3, pixel.g() << 2, pixel.b() << 3]);
        }
        std::fs::write(path, out).unwrap();
    }
}

impl OriginDimensions for Picture {
    fn size(&self) -> Size {
        Size::new(WIDTH, HEIGHT)
    }
}

impl DrawTarget for Picture {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I: IntoIterator<Item = Pixel<Rgb565>>>(&mut self, pixels: I) -> Result<(), Self::Error> {
        for Pixel(point, color) in pixels {
            if (0..WIDTH as i32).contains(&point.x) && (0..HEIGHT as i32).contains(&point.y) {
                self.0[(point.y as u32 * WIDTH + point.x as u32) as usize] = color;
            }
        }
        Ok(())
    }
}
