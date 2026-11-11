//! Sudoku (apps.md §6.3): the shuffled puzzles stay Sudokus, conflicts, givens that cannot be
//! changed, a grid solved with the last digit, both sizes.

mod common;

use common::inside;
use quantus_apps::json;
use quantus_apps::{Callback, Context, Script, Value, Yield};

const SOURCE: &str = include_str!("../apps/sudoku.lua");

/// A solved grid.
const SOLVED: &str = "157394268926518437348762519469135782531287946872649351793856124614923875285471693";

/// The script's functions, without the host: the top level only defines them.
fn lua() -> mlua::Lua {
    let lua = mlua::Lua::new();
    lua.load(SOURCE).exec().expect("it runs");
    lua
}

#[test]
fn a_rearranged_grid_stays_solved() {
    let lua = lua();
    let cases = [
        "{1,2,3,4,5,6,7,8,9}, {1,2,3,4,5,6,7,8,9}, {1,2,3,4,5,6,7,8,9}, false",
        "{9,8,7,6,5,4,3,2,1}, {7,8,9,1,2,3,4,5,6}, {3,1,2,9,7,8,5,6,4}, true",
        "{2,1,4,3,6,5,8,7,9}, {5,4,6,8,9,7,2,3,1}, {1,3,2,4,6,5,7,9,8}, false",
    ];
    for case in cases {
        let code = format!("local g = transform(parse('{SOLVED}'), {case}) return solved(g), table.concat(g)");
        let (ok, grid): (bool, String) = lua.load(&code).eval().unwrap();
        assert!(ok, "{case}: {grid}");
    }
    // The identity leaves it as it was.
    let code = format!("return table.concat(transform(parse('{SOLVED}'), {}))", cases[0]);
    assert_eq!(lua.load(&code).eval::<String>().unwrap(), SOLVED);
}

#[test]
fn conflicts_are_found_in_rows_columns_and_boxes() {
    let lua = lua();
    // Two 5s in the first row, cells 1 and 9.
    let row: Vec<i64> = lua.load("local g = {} for i = 1, 81 do g[i] = 0 end g[1] = 5 g[9] = 5 local b = conflicts(g) local out = {} for i = 1, 81 do if b[i] then out[#out + 1] = i end end return out").eval().unwrap();
    assert_eq!(row, [1, 9]);
    // Two 3s in the first box, cells 1 and 21 (row 3, column 3).
    let boxed: Vec<i64> = lua.load("local g = {} for i = 1, 81 do g[i] = 0 end g[1] = 3 g[21] = 3 g[81] = 3 local b = conflicts(g) local out = {} for i = 1, 81 do if b[i] then out[#out + 1] = i end end return out").eval().unwrap();
    assert_eq!(boxed, [1, 21]);
    // A solved grid has none, and is solved; with one cell empty it is not.
    let (none, full, gap): (bool, bool, bool) = lua.load(format!("local g = parse('{SOLVED}') local a = next(conflicts(g)) == nil local b = solved(g) g[40] = 0 return a, b, solved(g)")).eval().unwrap();
    assert!(none && full && !gap);
}

/// The grid as numbers.
fn digits(s: &str) -> Vec<i64> {
    s.bytes().map(|b| i64::from(b - b'0')).collect()
}

/// Saved data with `puzzle`, `entries` and the time.
fn saved(puzzle: &[i64], entries: &[i64], time: i64) -> Value {
    let list = |v: &[i64]| Value::Array(v.iter().map(|&x| Value::Int(x)).collect());
    Value::Map(vec![
        ("level".into(), Value::Int(1)),
        ("puzzle".into(), list(puzzle)),
        ("entries".into(), list(entries)),
        ("time".into(), Value::Int(time)),
        ("best".into(), list(&[0, 0, 0])),
    ])
}

fn context(data: Option<Value>, cursor: (usize, usize)) -> Context {
    Context { width: 480, height: 250, lang: "de".into(), seed: 5, data, cursor, ..Context::default() }
}

/// A list of the saved data, by its key.
fn list_of(json: &str, key: &str) -> Vec<i64> {
    let Some(Value::Map(entries)) = json::read(json) else { panic!("a map") };
    let Some((_, Value::Array(cells))) = entries.iter().find(|(k, _)| k == key) else { panic!("{key}") };
    cells.iter().map(|cell| if let Value::Int(v) = cell { *v } else { panic!() }).collect()
}

#[test]
fn new_games_are_valid_puzzles() {
    let lua = lua();
    for seed in 0..12 {
        let mut script = Script::start(SOURCE).unwrap();
        let Yield::Done(output) = script.send(Callback::Init, Context { seed, ..context(None, (0, 0)) }) else { panic!() };
        assert_eq!(output.timer, Some(1000));
        // A new game is saved when the app closes.
        let Yield::Done(output) = script.send(Callback::Leave, context(None, (0, 0))) else { panic!() };
        let puzzle = list_of(output.save.as_deref().expect("saved"), "puzzle");
        assert_eq!(puzzle.len(), 81);
        let givens = puzzle.iter().filter(|&&v| v != 0).count();
        assert!((17..=45).contains(&givens), "{givens} givens");
        let code = format!("return next(conflicts({{{}}})) == nil", puzzle.iter().map(i64::to_string).collect::<Vec<_>>().join(","));
        assert!(lua.load(&code).eval::<bool>().unwrap(), "seed {seed}: {puzzle:?}");
    }
}

#[test]
fn givens_stay_and_digits_go_where_the_cursor_is() {
    let solution = digits(SOLVED);
    // The first row given, the rest empty.
    let puzzle: Vec<i64> = (0..81).map(|i| if i < 9 { solution[i] } else { 0 }).collect();
    let entries = vec![0; 81];
    let data = Some(saved(&puzzle, &entries, 0));
    let mut script = Script::start(SOURCE).unwrap();
    script.send(Callback::Init, context(data.clone(), (0, 0)));
    // On a given: nothing changes, nothing is saved.
    let Yield::Done(output) = script.send(Callback::Key("4".into()), context(data.clone(), (3, 0))) else { panic!() };
    assert_eq!(output.save, None);
    // On row 2, column 4: the 4 is written there.
    let Yield::Done(output) = script.send(Callback::Key("4".into()), context(data.clone(), (3, 1))) else { panic!() };
    let after = list_of(output.save.as_deref().unwrap(), "entries");
    assert_eq!(after[12], 4);
    assert_eq!(after.iter().filter(|&&v| v != 0).count(), 1);
    // 0 clears it, Zurück brings it back.
    let Yield::Done(output) = script.send(Callback::Key("0".into()), context(data.clone(), (3, 1))) else { panic!() };
    assert_eq!(list_of(output.save.as_deref().unwrap(), "entries")[12], 0);
    let Yield::Done(output) = script.send(Callback::Key("soft3".into()), context(data.clone(), (3, 1))) else { panic!() };
    assert_eq!(list_of(output.save.as_deref().unwrap(), "entries")[12], 4);
    // OK counts the cell up.
    let Yield::Done(output) = script.send(Callback::Pick(4, 2), context(data, (3, 1))) else { panic!() };
    assert_eq!(list_of(output.save.as_deref().unwrap(), "entries")[12], 5);
}

#[test]
fn the_last_digit_solves_it_and_sets_the_best_time() {
    let solution = digits(SOLVED);
    let puzzle: Vec<i64> = (0..81).map(|i| if i % 2 == 0 { solution[i] } else { 0 }).collect();
    // Every gap filled but the last one, cell 80 (column 8, row 9).
    let entries: Vec<i64> = (0..81).map(|i| if i % 2 == 1 && i != 79 { solution[i] } else { 0 }).collect();
    let data = Some(saved(&puzzle, &entries, 95));
    let mut script = Script::start(SOURCE).unwrap();
    let Yield::Done(output) = script.send(Callback::Init, context(data.clone(), (0, 0))) else { panic!() };
    assert_eq!(output.timer, Some(1000));
    let digit = solution[79].to_string();
    let Yield::Done(output) = script.send(Callback::Key(digit), context(data, (7, 8))) else { panic!() };
    assert_eq!(output.timer, Some(0));
    assert_eq!(list_of(output.save.as_deref().unwrap(), "best"), [95, 0, 0]);
    assert!(output.draws.iter().any(|draw| matches!(draw, quantus_apps::api::Draw::Text(_, _, text, _, _) if text == "Gelöst!")));
}

#[test]
fn both_sizes_hold_board_and_side() {
    let solution = digits(SOLVED);
    let puzzle: Vec<i64> = (0..81).map(|i| if i % 3 == 0 { solution[i] } else { 0 }).collect();
    let entries: Vec<i64> = (0..81).map(|i| if i % 3 == 1 { solution[i] } else { 0 }).collect();
    for lang in ["de", "en"] {
        for (width, height) in [(480, 250), (480, 294), (320, 214), (320, 170)] {
            let context = Context { width, height, lang: lang.into(), seed: 3, data: Some(saved(&puzzle, &entries, 4000)), ..Context::default() };
            let mut script = Script::start(SOURCE).unwrap();
            let Yield::Done(output) = script.send(Callback::Init, context) else { panic!() };
            inside(&output.draws, width, height).unwrap();
        }
    }
}
