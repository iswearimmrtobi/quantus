//! 2048 (apps.md §6.2): slides and joins, a move that changes nothing, the game kept across
//! opening it again, the best score across Neu, both sizes.

mod common;

use common::inside;
use quantus_apps::json;
use quantus_apps::{Callback, Context, Request, Script, ScriptApplet, Value, Yield};
use quantus_ui::app::App;
use quantus_ui::applet::AppInfo;
use quantus_ui::backend::Stub;
use quantus_ui::key::Key;
use quantus_ui::render::{Frame, Shape};

const SOURCE: &str = include_str!("../apps/2048.rhai");

fn engine() -> (rhai::Engine, rhai::AST) {
    let mut engine = rhai::Engine::new();
    engine.set_max_expr_depths(64, 32);
    engine.set_max_call_levels(32);
    let ast = engine.compile(SOURCE).expect("it compiles");
    (engine, ast)
}

fn ints(array: &rhai::Array) -> Vec<i64> {
    array.iter().map(|v| v.as_int().expect("a number")).collect()
}

fn array(values: &[i64]) -> rhai::Array {
    values.iter().map(|&v| rhai::Dynamic::from(v)).collect()
}

#[test]
fn a_line_slides_and_joins_once() {
    let (engine, ast) = engine();
    let cases: [([i64; 4], [i64; 4], i64); 6] = [
        ([2, 2, 2, 2], [4, 4, 0, 0], 8),
        ([2, 2, 4, 4], [4, 8, 0, 0], 12),
        ([4, 0, 4, 8], [8, 8, 0, 0], 8),
        ([0, 0, 0, 2], [2, 0, 0, 0], 0),
        ([2, 4, 8, 16], [2, 4, 8, 16], 0),
        ([8, 8, 8, 0], [16, 8, 0, 0], 16),
    ];
    for (line, expected, gained) in cases {
        let result = engine.call_fn::<rhai::Map>(&mut rhai::Scope::new(), &ast, "slide", (array(&line),)).expect("slid");
        assert_eq!(ints(&result["line"].clone().into_array().unwrap()), expected, "{line:?}");
        assert_eq!(result["gained"].as_int().unwrap(), gained, "{line:?}");
    }
    // A board moved to the right and down.
    let board = array(&[2, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2]);
    let right = engine.call_fn::<rhai::Map>(&mut rhai::Scope::new(), &ast, "moved", (board.clone(), "right".to_string())).unwrap();
    assert_eq!(ints(&right["board"].clone().into_array().unwrap()), [0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2]);
    let down = engine.call_fn::<rhai::Map>(&mut rhai::Scope::new(), &ast, "moved", (board, "down".to_string())).unwrap();
    assert_eq!(ints(&down["board"].clone().into_array().unwrap()), [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2, 0, 2]);
}

/// A saved game with `board`.
fn saved(board: &[i64], score: i64, best: i64) -> Value {
    Value::Map(vec![
        ("board".into(), Value::Array(board.iter().map(|&v| Value::Int(v)).collect())),
        ("score".into(), Value::Int(score)),
        ("best".into(), Value::Int(best)),
        ("reached".into(), Value::Bool(false)),
    ])
}

fn context(data: Option<Value>) -> Context {
    Context { width: 480, height: 250, lang: "de".into(), seed: 11, data, ..Context::default() }
}

/// The board the script saved last.
fn board_of(json: &str) -> Vec<i64> {
    let Some(Value::Map(entries)) = json::read(json) else { panic!("a map") };
    let Some((_, Value::Array(cells))) = entries.iter().find(|(key, _)| key == "board") else { panic!("a board") };
    cells.iter().map(|cell| if let Value::Int(v) = cell { *v } else { panic!() }).collect()
}

#[test]
fn a_move_that_changes_nothing_adds_no_tile() {
    // Every row packed to the left with no equal neighbours: left changes nothing.
    let board = [2, 4, 8, 16, 4, 8, 16, 32, 2, 4, 0, 0, 0, 0, 0, 0];
    let mut script = Script::start(SOURCE).unwrap();
    assert!(matches!(script.send(Callback::Init, context(Some(saved(&board, 0, 0)))), Yield::Done(_)));
    let Yield::Done(output) = script.send(Callback::Key("left".into()), context(Some(saved(&board, 0, 0)))) else { panic!() };
    assert_eq!(output.save, None);
    // Right moves: the board is saved with one tile more.
    let Yield::Done(output) = script.send(Callback::Key("right".into()), context(Some(saved(&board, 0, 0)))) else { panic!() };
    let after = board_of(output.save.as_deref().expect("saved"));
    assert_eq!(after.iter().filter(|&&v| v != 0).count(), board.iter().filter(|&&v| v != 0).count() + 1);
    // The third row slid right.
    assert_eq!(&after[10..12], &[2, 4]);
}

#[test]
fn neu_asks_and_keeps_the_best() {
    let board = [2, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let data = Some(saved(&board, 100, 100));
    let mut script = Script::start(SOURCE).unwrap();
    script.send(Callback::Init, context(data.clone()));
    assert_eq!(script.send(Callback::Key("soft1".into()), context(data)), Yield::Asks(Request::Confirm("Neues Spiel?".into())));
    let Yield::Done(output) = script.answer(Value::Bool(true)) else { panic!() };
    let Some(Value::Map(entries)) = json::read(output.save.as_deref().unwrap()) else { panic!() };
    assert!(entries.contains(&("score".into(), Value::Int(0))));
    assert!(entries.contains(&("best".into(), Value::Int(100))));
}

fn texts(app: &App<Stub>) -> Vec<String> {
    Frame::new(app).0.iter().filter_map(|shape| match shape {
        Shape::Text(_, text, ..) => Some(text.clone()),
        _ => None,
    }).collect()
}

fn open(app: &mut App<Stub>) {
    let info = AppInfo { id: "2048".into(), name: "2048".into(), api: 1, usable: true, ..AppInfo::default() };
    app.open_applet(Box::new(ScriptApplet::new(SOURCE)), info);
}

#[test]
fn a_game_is_restored_after_reopening() {
    let mut app = App::new(Stub);
    open(&mut app);
    for key in [Key::Left, Key::Up, Key::Right, Key::Down, Key::Left] {
        app.key(key);
    }
    let before: Vec<String> = texts(&app).into_iter().filter(|text| text != "Zurück").collect();
    app.key(Key::Ac);
    open(&mut app);
    // The same tiles, the same score; the move back is not kept.
    assert_eq!(texts(&app), before);
}

#[test]
fn both_sizes_hold_board_and_side() {
    for (width, height) in [(480, 250), (320, 214)] {
        let board = [2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 0, 0];
        let data = Some(saved(&board, 123456, 123456));
        let context = Context { width, height, lang: "de".into(), seed: 3, data, ..Context::default() };
        let mut script = Script::start(SOURCE).unwrap();
        let Yield::Done(output) = script.send(Callback::Init, context.clone()) else { panic!() };
        inside(&output.draws, width, height).unwrap();
        let Yield::Done(output) = script.send(Callback::Key("down".into()), context) else { panic!() };
        inside(&output.draws, width, height).unwrap();
    }
}
