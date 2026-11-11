//! The script on its own thread: callbacks, host calls that yield, errors, the thread's end.

use std::time::{Duration, Instant};

use quantus_apps::{Callback, Context, Request, Script, Value, Yield};

fn start(source: &str) -> Script {
    let mut script = Script::start(source).expect("it compiles");
    assert!(matches!(script.send(Callback::Init, Context::default()), Yield::Done(_)));
    script
}

#[test]
fn a_callback_returns() {
    let mut script = start("fn init() { this.n = 1; } fn key(k) { this.n += 1; if this.n == 3 { throw `n ${this.n} ${k}`; } } fn draw() {}");
    assert!(matches!(script.send(Callback::Key("up".into()), Context::default()), Yield::Done(output) if output.drawn));
    // The state carries over as `this`: the second key finds n at 2 and throws with it.
    match script.send(Callback::Key("down".into()), Context::default()) {
        Yield::Failed(error) => assert!(error.message.contains("n 3 down"), "{error:?}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_host_call_yields_and_resumes() {
    let mut script = start("fn key(k) { this.yes = confirm(`Neu? ${k}`); } fn draw() { if this.yes == true { throw \"yes\"; } }");
    assert_eq!(script.send(Callback::Key("ok".into()), Context::default()), Yield::Asks(Request::Confirm("Neu? ok".into())));
    // The answer resumes it; draw sees it.
    match script.answer(Value::Bool(true)) {
        Yield::Failed(error) => assert_eq!(error.message, "yes"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_script_error_has_its_line() {
    let error = Script::start("fn draw() {\n  let x = ;\n}").err().expect("a syntax error");
    assert_eq!(error.line, Some(2));
    let mut script = start("fn draw() {}\nfn key(k) {\n  let a = [1];\n  a[5]\n}");
    match script.send(Callback::Key("up".into()), Context::default()) {
        Yield::Failed(error) => assert_eq!(error.line, Some(4), "{error:?}"),
        other => panic!("{other:?}"),
    }
    // Without draw: allowed, for a script that is only a form.
    assert!(!Script::start("fn init() {}").expect("it compiles").draws);
}

#[test]
fn optional_callbacks_and_the_top_level() {
    // No init, no key: only draw. The top level runs once, with the first callback.
    let mut script = Script::start("let x = 1; fn draw() {}").expect("it compiles");
    assert!(matches!(script.send(Callback::Init, Context::default()), Yield::Done(_)));
    assert!(matches!(script.send(Callback::Key("1".into()), Context::default()), Yield::Done(_)));
    // Leave does not draw.
    assert!(matches!(script.send(Callback::Leave, Context::default()), Yield::Done(output) if !output.drawn));
}

/// Waits for `thread` to end, at most `limit`.
fn ends(thread: std::thread::JoinHandle<()>, limit: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < limit {
        if thread.is_finished() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    false
}

#[test]
fn a_dropped_scripts_thread_is_gone() {
    // Waiting between callbacks.
    let script = start("fn draw() {}");
    assert!(ends(script.close(), Duration::from_secs(2)));
    // Waiting for an answer that never comes.
    let mut script = start("fn key(k) { confirm(\"?\"); } fn draw() {}");
    assert!(matches!(script.send(Callback::Key("ok".into()), Context::default()), Yield::Asks(_)));
    assert!(ends(script.close(), Duration::from_secs(2)));
}
