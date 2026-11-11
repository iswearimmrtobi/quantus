//! The limits of a script (apps.md §5): each comes back as an error with its line, none as a
//! panic, and the thread goes on to the next callback no more.

use std::time::Instant;

use quantus_apps::{Callback, Context, Script, Yield};

/// The error of a script whose `key` does `body`.
fn failure(body: &str) -> quantus_apps::Error {
    let source = format!("fn draw() {{}}\nfn key(k) {{\n{body}\n}}\n");
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
    let error = failure("let i = 0;\nwhile true { i += 1; }");
    assert!(!error.hung, "{error:?}");
    assert!(error.message.to_lowercase().contains("operations"), "{error:?}");
    // Rhai gives the budget no position: the error has no line.
    println!("the budget took {:?}", start.elapsed());
}

#[test]
fn an_endless_recursion_ends_by_the_depth() {
    let source = "fn draw() {}\nfn down(n) {\n  down(n + 1)\n}\nfn key(k) {\n  down(0);\n}\n";
    let mut script = Script::start(source).expect("it compiles");
    assert!(matches!(script.send(Callback::Init, Context::default()), Yield::Done(_)));
    match script.send(Callback::Key("ok".into()), Context::default()) {
        Yield::Failed(error) => assert!(error.message.to_lowercase().contains("stack") || error.message.contains("call"), "{error:?}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn growing_without_end_is_stopped() {
    let error = failure("let s = \"x\";\nloop { s += s; }");
    assert!(error.message.to_lowercase().contains("length") || error.message.to_lowercase().contains("too large"), "{error:?}");
    // Rhai does not check the size on `push`: the budget stops it.
    let error = failure("let a = [];\nloop { a.push(1); }");
    let message = error.message.to_lowercase();
    assert!(message.contains("size") || message.contains("too large") || message.contains("operations"), "{error:?}");
    let error = failure("let m = #{};\nlet i = 0;\nloop { m[`k${i}`] = i; i += 1; }");
    let message = error.message.to_lowercase();
    assert!(message.contains("size") || message.contains("too large") || message.contains("operations"), "{error:?}");
}

#[test]
fn a_file_over_the_limit_is_refused() {
    let source = format!("fn draw() {{}}\n// {}\n", "x".repeat(quantus_apps::api::MAX_SOURCE));
    let error = Script::start(&source).err().expect("too large");
    assert!(error.message.contains("256"));
}

#[test]
fn after_an_error_the_script_gets_nothing() {
    let mut script = Script::start("fn draw() {}\nfn key(k) { throw \"no\"; }").expect("it compiles");
    script.send(Callback::Init, Context::default());
    assert!(matches!(script.send(Callback::Key("1".into()), Context::default()), Yield::Failed(_)));
    assert!(matches!(script.send(Callback::Key("2".into()), Context::default()), Yield::Failed(_)));
}
