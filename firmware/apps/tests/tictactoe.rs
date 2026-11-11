//! Tic tac toe (apps.md §6.1): a game through the keys against the seeded computer, the hard
//! level against random moves, the layout at both sizes.

mod common;

use common::inside;

use std::collections::HashMap;

use quantus_apps::{Apps, Callback, Context, Script, Seeded, Yield};
use quantus_ui::app::{App, Mode, Settings};
use quantus_ui::applet::Palette;
use quantus_ui::backend::{Backend, Stub};
use quantus_ui::key::Key;
use quantus_ui::render::{Frame, Shape};

const SOURCE: &str = include_str!("../apps/tictactoe.rhai");

/// The stub with the apps and seeded random numbers, as the simulator with `--seed`.
struct Seeded1(Seeded);

impl Backend for Seeded1 {
    fn evaluate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> Result<String, String> {
        Stub.evaluate(mode, settings, latex)
    }

    fn random(&mut self) -> u32 {
        self.0.next_u32()
    }
}

fn open(seed: u32) -> App<Seeded1> {
    let mut app = App::new(Seeded1(Seeded::new(seed)));
    let apps = Apps::new(None);
    let info = apps.list(quantus_ui::text::Lang::De).into_iter().find(|app| app.id == "tictactoe").expect("built in");
    app.open_applet(apps.open("tictactoe").expect("it opens"), info);
    app
}

fn texts(app: &App<Seeded1>) -> Vec<String> {
    Frame::new(app).0.iter().filter_map(|shape| match shape {
        Shape::Text(_, text, ..) => Some(text.clone()),
        _ => None,
    }).collect()
}

/// How many O are on the board: the amber rings.
fn rings(app: &App<Seeded1>) -> usize {
    let amber = Palette::get("amber");
    Frame::new(app).0.iter().filter(|shape| matches!(shape, Shape::Circle(_, color) if Some(*color) == amber)).count()
}

#[test]
fn a_game_through_the_keys() {
    let mut app = open(42);
    assert!(texts(&app).iter().any(|text| text == "Am Zug: X"));
    app.key(Key::Digit(5));
    // The computer answered at once.
    assert_eq!(rings(&app), 1);
    assert!(texts(&app).iter().any(|text| text == "Am Zug: X"));
    // Every digit in turn until the round is over; a taken cell is ignored.
    for digit in (1..=9).cycle().take(40) {
        if texts(&app).iter().any(|text| text.contains("gewinnt") || text == "Unentschieden") {
            break;
        }
        app.key(Key::Digit(digit));
    }
    let shown = texts(&app);
    assert!(shown.iter().any(|text| text.contains("gewinnt") || text == "Unentschieden"), "{shown:?}");
    assert!(shown.iter().any(|text| text.starts_with("X ") && text.contains("Remis")));
    // Any key: the next round, the computer beginning.
    app.key(Key::Digit(1));
    assert_eq!(rings(&app), 1);
    assert!(texts(&app).iter().any(|text| text == "Am Zug: X"));
}

#[test]
fn the_soft_keys_switch_mode_and_level() {
    let mut app = open(1);
    let labels = |app: &App<Seeded1>| quantus_ui::soft::bar(app).map(|bar| (0..3).map(|i| bar.label(i, quantus_ui::text::Lang::De)).collect::<Vec<_>>());
    assert_eq!(labels(&app), Some(vec!["Neu".into(), "Rechner".into(), "leicht".into()]));
    app.key(Key::Soft(2));
    assert_eq!(labels(&app), Some(vec!["Neu".into(), "Rechner".into(), "schwer".into()]));
    app.key(Key::Soft(1));
    assert_eq!(labels(&app), Some(vec!["Neu".into(), "2 Spieler".into(), String::new()]));
    // Two players: the second mark is O, by hand.
    app.key(Key::Digit(5));
    assert_eq!(rings(&app), 0);
    app.key(Key::Digit(1));
    assert_eq!(rings(&app), 1);
    assert!(texts(&app).iter().any(|text| text == "Am Zug: X"));
}

/// The winner of a board of "", "X" and "O".
fn winner(board: &[&str; 9]) -> Option<&'static str> {
    const LINES: [[usize; 3]; 8] = [[0, 1, 2], [3, 4, 5], [6, 7, 8], [0, 3, 6], [1, 4, 7], [2, 5, 8], [0, 4, 8], [2, 4, 6]];
    LINES.iter().find_map(|line| match board[line[0]] {
        "" => None,
        mark if board[line[1]] == mark && board[line[2]] == mark => Some(if mark == "X" { "X" } else { "O" }),
        _ => None,
    })
}

#[test]
fn schwer_loses_no_game_against_random_moves() {
    let mut engine = rhai::Engine::new();
    engine.set_max_expr_depths(64, 32);
    engine.set_max_call_levels(32);
    let ast = engine.compile(SOURCE).expect("it compiles");
    // The same position comes up in many games: its answer is kept.
    let mut answers: HashMap<String, i64> = HashMap::new();
    let mut random = Seeded::new(2026);
    let (mut lost, mut games) = (0, 0);
    for game in 0..300 {
        let mut board = [""; 9];
        let mut turn = if game % 2 == 0 { "X" } else { "O" };
        while winner(&board).is_none() && board.contains(&"") {
            let cell = if turn == "X" {
                let free: Vec<usize> = (0..9).filter(|&i| board[i].is_empty()).collect();
                free[random.next_u32() as usize % free.len()]
            } else {
                let key = board.join(",");
                *answers.entry(key).or_insert_with(|| {
                    let array: rhai::Array = board.iter().map(|cell| rhai::Dynamic::from(cell.to_string())).collect();
                    engine.call_fn::<i64>(&mut rhai::Scope::new(), &ast, "best", (array, "O".to_string())).expect("a move")
                }) as usize
            };
            assert!(board[cell].is_empty(), "{board:?} {cell}");
            board[cell] = turn;
            turn = if turn == "X" { "O" } else { "X" };
        }
        games += 1;
        if winner(&board) == Some("X") {
            lost += 1;
            eprintln!("lost: {board:?}");
        }
    }
    assert_eq!((lost, games), (0, 300));
}

#[test]
fn board_and_side_fit_both_sizes() {
    for (width, height) in [(480, 250), (320, 214)] {
        let mut script = Script::start(SOURCE).expect("it compiles");
        let context = Context { width, height, lang: "de".into(), seed: 5, ..Context::default() };
        let Yield::Done(output) = script.send(Callback::Init, context.clone()) else { panic!("init") };
        inside(&output.draws, width, height).unwrap();
        // A round played to its end, the line drawn.
        for (col, row) in [(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1), (0, 2), (1, 2), (2, 2)] {
            let Yield::Done(output) = script.send(Callback::Pick(col, row), context.clone()) else { panic!("pick") };
            inside(&output.draws, width, height).unwrap();
        }
    }
}
