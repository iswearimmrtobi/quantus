//! The limits of a script (apps.md §5): each comes back as an error with its line, none as a
//! panic, and the thread goes on to the next callback no more.

use std::time::Instant;

use quantus_apps::{Callback, Context, Script, Yield};

/// The error of a script whose `key` does `body`.
fn failure(body: &str) -> quantus_apps::Error {
    let source = format!("function draw() end\nfunction key(k)\n{body}\nend\n");
    let mut script = Script::start(&source).expect("it compiles");
    assert!(matches!(script.send(Callback::Init, Context::default()), Yield::Done(_)));
    match script.send(Callback::Key("ok".into()), Context::default()) {
        Yield::Failed(error) => error,
        other => panic!("no error: {other:?}"),
    }
}

#[test]
fn an_endless_loop_ends_by_the_budget() {
    let start = Instant::now();
    let error = failure("local i = 0\nwhile true do i = i + 1 end");
    assert!(!error.hung, "{error:?}");
    assert!(error.message.contains("too many operations"), "{error:?}");
    assert_eq!(error.line, Some(4), "{error:?}");
    println!("the budget took {:?}", start.elapsed());
}

#[test]
fn the_budget_cannot_be_caught() {
    // `pcall` and coroutines could catch the budget's error and go on for ever: there are none.
    let error = failure("pcall(function() end)");
    assert!(error.message.contains("pcall"), "{error:?}");
    let error = failure("coroutine.wrap(function() end)()");
    assert!(error.message.contains("coroutine"), "{error:?}");
}

#[test]
fn an_endless_recursion_ends_by_the_memory() {
    let source = "function draw() end\nfunction down(n)\n  return down(n + 1) + 1\nend\nfunction key(k)\n  down(0)\nend\n";
    let mut script = Script::start(source).expect("it compiles");
    assert!(matches!(script.send(Callback::Init, Context::default()), Yield::Done(_)));
    match script.send(Callback::Key("ok".into()), Context::default()) {
        Yield::Failed(error) => {
            let message = error.message.to_lowercase();
            assert!(message.contains("memory") || message.contains("stack overflow") || message.contains("too many operations"), "{error:?}");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_recursion_through_the_host_ends_by_the_c_stack() {
    // A table's `__index` calls back into Lua from C, as a host function would: Lua stops at its
    // limit of nested C calls.
    let error = failure("local t = setmetatable({}, {__index = function(t, k) return t[k + 1] end})\nlocal x = t[1]");
    let message = error.message.to_lowercase();
    assert!(message.contains("stack overflow") || message.contains("c stack"), "{error:?}");
}

#[test]
fn growing_without_end_is_stopped() {
    let error = failure("local s = \"x\"\nwhile true do s = s .. s end");
    assert!(error.message.contains("memory"), "{error:?}");
    let error = failure("local a = {}\nwhile true do a[#a + 1] = {1, 2, 3} end");
    assert!(error.message.contains("memory") || error.message.contains("too many operations"), "{error:?}");
    let error = failure("local s = string.rep(\"x\", 100000000)");
    assert!(error.message.contains("memory"), "{error:?}");
}

#[test]
fn no_code_from_text_or_files() {
    for call in ["dofile(\"x\")", "loadfile(\"x\")", "require(\"x\")", "io.open(\"x\")", "os.exit()", "debug.getinfo(1)"] {
        let error = failure(call);
        assert!(error.message.contains("nil"), "{call}: {error:?}");
    }
}

#[test]
fn load_gives_the_saved_data_and_runs_nothing() {
    let error = failure("if load(\"return 1\") ~= nil then error(\"ran\") end\nerror(\"nothing ran\")");
    assert_eq!(error.message, "nothing ran");
}

#[test]
fn a_table_inside_itself_is_not_saved() {
    let error = failure("local t = {}\nt.t = t\nsave(t)");
    assert!(error.message.contains("nested"), "{error:?}");
}

#[test]
fn a_file_over_the_limit_is_refused() {
    let source = format!("function draw() end\n-- {}\n", "x".repeat(quantus_apps::api::MAX_SOURCE));
    let error = Script::start(&source).err().expect("too large");
    assert!(error.message.contains("256"));
}

#[test]
fn after_an_error_the_script_gets_nothing() {
    let mut script = Script::start("function draw() end\nfunction key(k) error(\"no\") end").expect("it compiles");
    script.send(Callback::Init, Context::default());
    assert!(matches!(script.send(Callback::Key("1".into()), Context::default()), Yield::Failed(_)));
    assert!(matches!(script.send(Callback::Key("2".into()), Context::default()), Yield::Failed(_)));
}
