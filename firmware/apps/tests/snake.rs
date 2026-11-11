//! Snake (apps.md §6.5): a step moves, grows and crashes, turns are queued and a reversal is
//! ignored, the timer runs from the first arrow to the crash, both sizes.

mod common;

use common::inside;
use quantus_apps::api::Draw;
use quantus_apps::json;
use quantus_apps::{Callback, Context, Script, Value, Yield};

const SOURCE: &str = include_str!("../apps/snake.lua");

/// The script's functions, without the host: the top level only defines them.
fn lua() -> mlua::Lua {
    let lua = mlua::Lua::new();
    lua.load(SOURCE).exec().expect("it runs");
    lua
}

/// One step of a snake on a board 5 × 3: what `advance` says, and the body after it.
fn step(body: &str, dir: &str, food: i64) -> (String, String) {
    let code = format!("local g = {{cols = 5, rows = 3, body = {{{body}}}, dir = '{dir}', food = {food}}} local r = advance(g) return r, table.concat(g.body, ',')");
    lua().load(&code).eval().unwrap()
}

#[test]
fn a_step_moves_grows_and_crashes() {
    // Cells 1 to 15, row by row: 6 7 8 9 10 is the middle row.
    assert_eq!(step("8, 7, 6", "right", 1), ("moved".into(), "9,8,7".into()));
    assert_eq!(step("8, 7, 6", "right", 9), ("ate".into(), "9,8,7,6".into()));
    assert_eq!(step("8, 7, 6", "up", 1), ("moved".into(), "3,8,7".into()));
    // Into the wall: it stays as it was.
    assert_eq!(step("10, 9, 8", "right", 1), ("crash".into(), "10,9,8".into()));
    assert_eq!(step("3, 8, 7", "up", 1), ("crash".into(), "3,8,7".into()));
    // Into itself: 7 8 3 2 1 with the head at 7 turning up into 2.
    assert_eq!(step("7, 8, 3, 2, 1", "up", 15), ("crash".into(), "7,8,3,2,1".into()));
    // Into the cell its tail leaves, which is no crash, unless it eats there and grows.
    assert_eq!(step("7, 8, 3, 2", "up", 15), ("moved".into(), "2,7,8,3".into()));
}

#[test]
fn turns_wait_in_a_queue_of_two() {
    let lua = lua();
    let queued = |code: &str| -> String { lua.load(format!("local q = {{}} {code} return table.concat(q, ',')")).eval().unwrap() };
    // A reversal and the heading itself are ignored.
    assert_eq!(queued("steer(q, 'left', 'right') steer(q, 'right', 'right')"), "");
    // Up then left is two turns; a third waits for none.
    assert_eq!(queued("steer(q, 'up', 'right') steer(q, 'left', 'right') steer(q, 'down', 'right')"), "up,left");
    // Against the last turn queued, not the heading: up then down is a reversal.
    assert_eq!(queued("steer(q, 'up', 'right') steer(q, 'down', 'right')"), "up");
    // The food never lands on the snake.
    for _ in 0..50 {
        let food: i64 = lua.load("return food_for({cols = 3, rows = 1, body = {1, 3}})").eval().unwrap();
        assert_eq!(food, 2);
    }
    let none: Option<i64> = lua.load("return food_for({cols = 2, rows = 1, body = {1, 2}})").eval().unwrap();
    assert_eq!(none, None);
}

fn context() -> Context {
    Context { width: 480, height: 250, lang: "de".into(), seed: 2, ..Context::default() }
}

/// Whether a text drawn holds `part`: on a narrow area the side panel is one line below the board.
fn shows(draws: &[Draw], part: &str) -> bool {
    draws.iter().any(|draw| matches!(draw, Draw::Text(_, _, text, ..) if text.contains(part)))
}

#[test]
fn the_timer_runs_from_the_first_arrow_to_the_crash() {
    let mut script = Script::start(SOURCE).unwrap();
    let Yield::Done(output) = script.send(Callback::Init, context()) else { panic!() };
    assert!(shows(&output.draws, "Pfeil: Start"));
    // A tick before the start changes nothing.
    let Yield::Done(before) = script.send(Callback::Tick(150), context()) else { panic!() };
    assert_eq!(before.draws, output.draws);
    let Yield::Done(output) = script.send(Callback::Key("right".into()), context()) else { panic!() };
    assert_eq!(output.timer, Some(150));
    // Pause and on again.
    let Yield::Done(output) = script.send(Callback::Key("soft2".into()), context()) else { panic!() };
    assert_eq!(output.timer, Some(0));
    assert_eq!(output.soft.as_deref(), Some(&["Neu".to_string(), "Weiter".to_string()][..]));
    let Yield::Done(output) = script.send(Callback::Key("soft2".into()), context()) else { panic!() };
    assert!(matches!(output.timer, Some(100..=150)));
    // The head starts in column 8 of 16, heading right: the ninth step hits the wall.
    let mut crashed = None;
    for k in 1..=9 {
        let Yield::Done(output) = script.send(Callback::Tick(k * 150), context()) else { panic!() };
        if shows(&output.draws, "Vorbei") {
            crashed = Some((k, output));
            break;
        }
    }
    let (k, output) = crashed.expect("it crashed");
    assert_eq!(k, 9);
    assert_eq!(output.timer, Some(0));
    // Any key after that sets up the next game.
    let Yield::Done(output) = script.send(Callback::Key("ok".into()), context()) else { panic!() };
    assert!(shows(&output.draws, "Pfeil: Start"));
}

#[test]
fn the_best_score_is_kept() {
    let data = json::read(r#"{"best": 17}"#);
    let mut script = Script::start(SOURCE).unwrap();
    let Yield::Done(output) = script.send(Callback::Init, Context { data, ..context() }) else { panic!() };
    assert!(shows(&output.draws, "17"));
    let Yield::Done(output) = script.send(Callback::Leave, context()) else { panic!() };
    assert_eq!(json::read(output.save.as_deref().unwrap()), Some(Value::Map(vec![("best".into(), Value::Int(17))])));
}

#[test]
fn both_sizes_hold_board_and_side() {
    for lang in ["de", "en"] {
        for (width, height) in [(480, 250), (480, 294), (320, 214), (320, 170)] {
            let context = Context { width, height, lang: lang.into(), seed: 4, ..Context::default() };
            let mut script = Script::start(SOURCE).unwrap();
            let Yield::Done(output) = script.send(Callback::Init, context.clone()) else { panic!() };
            inside(&output.draws, width, height).unwrap();
            script.send(Callback::Key("down".into()), context.clone());
            for k in 1..=8 {
                let Yield::Done(output) = script.send(Callback::Tick(k * 150), context.clone()) else { panic!() };
                inside(&output.draws, width, height).unwrap();
            }
        }
    }
}
