//! Tic tac toe (apps.md §6.1): a game through the keys against the seeded computer, the hard
//! level against every game the other side can play, the layout at both sizes.

mod common;

use common::inside;

use std::collections::HashMap;

use quantus_apps::{Apps, Callback, Context, Script, Seeded, Yield};
use quantus_ui::app::{App, Mode, Settings};
use quantus_ui::applet::Palette;
use quantus_ui::backend::{Backend, Stub};
use quantus_ui::key::Key;
use quantus_ui::render::{Frame, Shape};

const SOURCE: &str = include_str!("../apps/tictactoe.lua");

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

/// Plays every game the other side can play against *schwer* from `board`, `turn` moving next;
/// the computer's answers are kept by position. Counts the games and the ones lost.
fn every_game(play: &mut dyn FnMut(&[&'static str; 9]) -> usize, board: &mut [&'static str; 9], turn: &'static str, games: &mut usize, lost: &mut Vec<String>) {
    if let Some(mark) = winner(board) {
        *games += 1;
        if mark == "X" {
            lost.push(format!("{board:?}"));
        }
        return;
    }
    if !board.contains(&"") {
        *games += 1;
        return;
    }
    if turn == "O" {
        let cell = play(board);
        assert!(board[cell].is_empty(), "{board:?} {cell}");
        board[cell] = "O";
        every_game(play, board, "X", games, lost);
        board[cell] = "";
    } else {
        for cell in 0..9 {
            if board[cell].is_empty() {
                board[cell] = "X";
                every_game(play, board, "O", games, lost);
                board[cell] = "";
            }
        }
    }
}

#[test]
fn schwer_loses_no_game_and_answers_quickly() {
    // The script's functions without the host; the instructions counted as the budget does.
    let lua = mlua::Lua::new();
    let instructions = std::rc::Rc::new(std::cell::Cell::new(0u64));
    let counted = instructions.clone();
    lua.set_hook(mlua::HookTriggers::new().every_nth_instruction(100), move |_, _| {
        counted.set(counted.get() + 100);
        Ok(mlua::VmState::Continue)
    })
    .unwrap();
    lua.load(SOURCE).exec().expect("it runs");
    let best: mlua::Function = lua.globals().get("best").unwrap();
    let mut answers: HashMap<String, usize> = HashMap::new();
    let mut most = 0;
    let mut play = |board: &[&'static str; 9]| {
        *answers.entry(board.join(",")).or_insert_with(|| {
            instructions.set(0);
            let cells: Vec<&str> = board.to_vec();
            let cell: usize = best.call((cells, "O")).expect("a move");
            most = most.max(instructions.get());
            // Lua counts from 1.
            cell - 1
        })
    };
    let (mut games, mut lost) = (0, Vec::new());
    for first in ["X", "O"] {
        every_game(&mut play, &mut [""; 9], first, &mut games, &mut lost);
    }
    assert!(lost.is_empty(), "lost {} of {games}: {:?}", lost.len(), &lost[..lost.len().min(5)]);
    // Every game the other side can play, both beginning.
    assert!(games > 500, "{games}");
    // A move is a small part of the budget of a million instructions.
    assert!(most < 10_000, "{most} instructions");
    eprintln!("{games} games, at most {most} instructions a move");
}

#[test]
fn board_and_side_fit_both_sizes() {
    for (width, height) in [(480, 250), (320, 214)] {
        let mut script = Script::start(SOURCE).expect("it compiles");
        let context = Context { width, height, lang: "de".into(), seed: 5, ..Context::default() };
        let Yield::Done(output) = script.send(Callback::Init, context.clone()) else { panic!("init") };
        inside(&output.draws, width, height).unwrap();
        // A round played to its end, the line drawn.
        for (col, row) in [(1, 1), (2, 1), (3, 1), (1, 2), (2, 2), (3, 2), (1, 3), (2, 3), (3, 3)] {
            let Yield::Done(output) = script.send(Callback::Pick(col, row), context.clone()) else { panic!("pick") };
            inside(&output.draws, width, height).unwrap();
        }
    }
}

#[test]
fn mode_level_and_score_are_kept() {
    let mut app = open(7);
    app.key(Key::Soft(2));
    // A round to its end, the score counted.
    for digit in (1..=9).cycle().take(40) {
        if texts(&app).iter().any(|text| text.contains("gewinnt") || text == "Unentschieden") {
            break;
        }
        app.key(Key::Digit(digit));
    }
    let score = texts(&app).into_iter().find(|text| text.contains("Remis")).expect("the score");
    assert_ne!(score, "X 0   O 0   Remis 0");
    app.key(Key::Ac);
    // Opened again in the same App, which keeps the data while the stub keeps none.
    let apps = Apps::new(None);
    let info = apps.list(quantus_ui::text::Lang::De).into_iter().find(|app| app.id == "tictactoe").unwrap();
    app.open_applet(apps.open("tictactoe").unwrap(), info);
    let labels = quantus_ui::soft::bar(&app).map(|bar| bar.label(2, quantus_ui::text::Lang::De));
    assert_eq!(labels.as_deref(), Some("schwer"));
    assert!(texts(&app).contains(&score));
}
