//! Minesweeper (apps.md §6.4): the mines laid away from the first cell, the flood fill, flags
//! that stay shut, opening around a number, a win and a loss, every level at both sizes.

mod common;

use common::inside;
use quantus_apps::api::Draw;
use quantus_apps::json;
use quantus_apps::{Callback, Context, Script, Value, Yield};

const SOURCE: &str = include_str!("../apps/minesweeper.lua");

/// The script's functions, without the host: the top level only defines them.
fn lua() -> mlua::Lua {
    let lua = mlua::Lua::new();
    lua.load(SOURCE).exec().expect("it runs");
    lua
}

#[test]
fn the_mines_keep_clear_of_the_first_cell() {
    let lua = lua();
    for safe in [1, 5, 41, 81, 73] {
        for _ in 0..20 {
            let mines: Vec<i64> = lua.load(format!("return place(9, 9, 10, {safe})")).eval().unwrap();
            assert_eq!(mines.iter().sum::<i64>(), 10);
            let around: Vec<i64> = lua.load(format!("return neighbours({safe}, 9, 9)")).eval().unwrap();
            assert_eq!(mines[safe as usize - 1], 0);
            assert!(around.iter().all(|&j| mines[j as usize - 1] == 0), "{safe}: {mines:?}");
        }
    }
    // Too full a board for that: only the cell itself stays clear.
    let mines: Vec<i64> = lua.load("return place(3, 3, 8, 5)").eval().unwrap();
    assert_eq!(mines, [1, 1, 1, 1, 0, 1, 1, 1, 1]);
}

#[test]
fn a_cell_without_mines_near_opens_its_area() {
    let lua = lua();
    // 5 × 5, one mine at the bottom right. Opening the top left opens all the rest; a flag on
    // cell 3 stays a flag.
    let (hit, state, done): (bool, Vec<i64>, bool) = lua.load(
        "local m = {} local st = {} for i = 1, 25 do m[i] = 0 st[i] = 0 end m[25] = 1 st[3] = 2
         local cn = count(m, 5, 5) local hit = open(st, m, cn, 5, 5, 1) return hit, st, cleared(st, m)",
    ).eval().unwrap();
    assert!(!hit);
    assert!(!done, "the flagged cell is still shut");
    let expected: Vec<i64> = (1..=25).map(|i| match i { 3 => 2, 25 => 0, _ => 1 }).collect();
    assert_eq!(state, expected);
    // The numbers next to the mine.
    let counts: Vec<i64> = lua.load("local m = {} for i = 1, 9 do m[i] = 0 end m[5] = 1 return count(m, 3, 3)").eval().unwrap();
    assert_eq!(counts, [1, 1, 1, 1, 0, 1, 1, 1, 1]);
    // A mine opened is a hit.
    let hit: bool = lua.load("local m = {1, 0, 0, 0} local st = {0, 0, 0, 0} return open(st, m, count(m, 2, 2), 2, 2, 1)").eval().unwrap();
    assert!(hit);
}

/// Saved data of a game on level 1 (9 × 9): mines on `mines`, the cells of `open` open and of
/// `flags` flagged, `time` seconds played.
fn saved(mines: &[usize], open: &[usize], flags: &[usize], time: i64) -> Option<Value> {
    let list = |pick: &dyn Fn(usize) -> i64| (1..=81).map(|i| pick(i).to_string()).collect::<Vec<_>>().join(",");
    let m = list(&|i| i64::from(mines.contains(&i)));
    let st = list(&|i| if open.contains(&i) { 1 } else if flags.contains(&i) { 2 } else { 0 });
    json::read(&format!(r#"{{"level":1,"best":[0,0,0],"game":{{"mines":[{m}],"state":[{st}],"time":{time}}}}}"#))
}

fn context(data: Option<Value>) -> Context {
    Context { width: 480, height: 250, lang: "de".into(), seed: 9, data, ..Context::default() }
}

/// The saved data's entry `key`.
fn entry(json: &str, key: &str) -> Option<Value> {
    let Some(Value::Map(entries)) = json::read(json) else { panic!("a map") };
    entries.into_iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn texts(draws: &[Draw]) -> Vec<String> {
    draws.iter().filter_map(|draw| if let Draw::Text(_, _, text, ..) = draw { Some(text.clone()) } else { None }).collect()
}

#[test]
fn the_last_safe_cell_wins() {
    let mines: Vec<usize> = (1..=10).collect();
    let open: Vec<usize> = (11..=80).collect();
    let data = saved(&mines, &open, &[], 42);
    let mut script = Script::start(SOURCE).unwrap();
    let Yield::Done(output) = script.send(Callback::Init, context(data.clone())) else { panic!() };
    assert_eq!(output.timer, Some(1000));
    let Yield::Done(output) = script.send(Callback::Pick(9, 9), context(data)) else { panic!() };
    assert_eq!(output.timer, Some(0));
    let save = output.save.as_deref().unwrap();
    assert_eq!(entry(save, "game"), None, "a finished game is not kept");
    assert_eq!(entry(save, "best"), Some(Value::Array(vec![Value::Int(42), Value::Int(0), Value::Int(0)])));
    assert!(texts(&output.draws).iter().any(|text| text == "Gewonnen!"));
}

#[test]
fn a_mine_loses() {
    let data = saved(&[1, 2, 3], &[50], &[], 3);
    let mut script = Script::start(SOURCE).unwrap();
    script.send(Callback::Init, context(data.clone()));
    let Yield::Done(output) = script.send(Callback::Pick(2, 1), context(data.clone())) else { panic!() };
    assert_eq!(output.timer, Some(0));
    let save = output.save.as_deref().unwrap();
    assert_eq!(entry(save, "game"), None);
    assert_eq!(entry(save, "best"), Some(Value::Array(vec![Value::Int(0), Value::Int(0), Value::Int(0)])));
    assert!(texts(&output.draws).iter().any(|text| text == "Verloren"));
    // The next pick starts a new game, which lays no mines before its first opening.
    let Yield::Done(output) = script.send(Callback::Pick(5, 5), context(data)) else { panic!() };
    assert!(texts(&output.draws).iter().any(|text| text == "Minen 10"));
}

#[test]
fn ok_on_a_number_with_its_flags_opens_around_it() {
    // One mine at cell 1, flagged; cell 2 open and showing 1. OK on it opens the rest.
    let data = saved(&[1], &[2], &[1], 0);
    let mut script = Script::start(SOURCE).unwrap();
    script.send(Callback::Init, context(data.clone()));
    let Yield::Done(output) = script.send(Callback::Pick(2, 1), context(data.clone())) else { panic!() };
    assert!(texts(&output.draws).iter().any(|text| text == "Gewonnen!"));
    // Without the flag nothing opens, and 0 sets one on the cursor's cell.
    let data = saved(&[1], &[2], &[], 0);
    let mut script = Script::start(SOURCE).unwrap();
    script.send(Callback::Init, context(data.clone()));
    let Yield::Done(output) = script.send(Callback::Pick(2, 1), context(data.clone())) else { panic!() };
    assert!(!texts(&output.draws).iter().any(|text| text == "Gewonnen!"));
    let Yield::Done(output) = script.send(Callback::Key("0".into()), Context { cursor: (0, 0), ..context(data) }) else { panic!() };
    assert!(texts(&output.draws).iter().any(|text| text == "Minen 9"));
}

#[test]
fn every_level_at_both_sizes() {
    for level in 1..=3 {
        let (cols, rows) = [(9, 9), (16, 12), (20, 12)][level - 1];
        let cells = cols * rows;
        let mines: Vec<String> = (1..=cells).map(|i| (i64::from(i % 7 == 0)).to_string()).collect();
        // Half open, a few flags, as after a loss.
        let state: Vec<String> = (1..=cells).map(|i| if i % 7 == 0 { "0" } else if i % 2 == 0 { "1" } else if i % 5 == 0 { "2" } else { "0" }.to_string()).collect();
        let data = json::read(&format!(r#"{{"level":{level},"best":[61,3700,0],"game":{{"mines":[{}],"state":[{}],"time":3599}}}}"#, mines.join(","), state.join(",")));
        assert!(data.is_some());
        for lang in ["de", "en"] {
            for (width, height) in [(480, 250), (480, 294), (320, 214), (320, 170)] {
                let context = Context { width, height, lang: lang.into(), seed: 1, data: data.clone(), ..Context::default() };
                let mut script = Script::start(SOURCE).unwrap();
                let Yield::Done(output) = script.send(Callback::Init, context.clone()) else { panic!() };
                inside(&output.draws, width, height).unwrap();
                // A mine opened: the board as lost.
                let Yield::Done(output) = script.send(Callback::Pick(7, 1), context) else { panic!() };
                inside(&output.draws, width, height).unwrap();
            }
        }
    }
}
