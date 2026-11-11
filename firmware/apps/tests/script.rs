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
    let mut script = start("local n\nfunction init() n = 1 end\nfunction key(k) n = n + 1; if n == 3 then error(\"n \" .. n .. \" \" .. k, 0) end end\nfunction draw() end");
    assert!(matches!(script.send(Callback::Key("up".into()), Context::default()), Yield::Done(output) if output.drawn));
    // The state carries over: the second key finds n at 2 and raises an error with it.
    match script.send(Callback::Key("down".into()), Context::default()) {
        Yield::Failed(error) => assert!(error.message.contains("n 3 down"), "{error:?}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_host_call_yields_and_resumes() {
    let mut script = start("local yes\nfunction key(k) yes = confirm(\"Neu? \" .. k) end\nfunction draw() if yes == true then error(\"yes\", 0) end end");
    assert_eq!(script.send(Callback::Key("ok".into()), Context::default()), Yield::Asks(Request::Confirm("Neu? ok".into())));
    // The answer resumes it; draw sees it.
    match script.answer(Value::Bool(true)) {
        Yield::Failed(error) => assert_eq!(error.message, "yes"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_script_error_has_its_line() {
    let error = Script::start("function draw()\n  local x = = 1\nend").err().expect("a syntax error");
    assert_eq!(error.line, Some(2));
    let mut script = start("function draw() end\nfunction key(k)\n  local a = {1}\n  local b = a[5].x\nend");
    match script.send(Callback::Key("up".into()), Context::default()) {
        Yield::Failed(error) => assert_eq!(error.line, Some(4), "{error:?}"),
        other => panic!("{other:?}"),
    }
    // Without draw: allowed, for a script that is only a form.
    assert!(!Script::start("function init() end").expect("it compiles").draws);
    assert!(Script::start("local function x() end\n\nfunction draw()\nend").expect("it compiles").draws);
}

#[test]
fn optional_callbacks_and_the_top_level() {
    // No init, no key: only draw. The top level runs once, with the first callback.
    let mut script = Script::start("local x = 1\nfunction draw() end").expect("it compiles");
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
    let script = start("function draw() end");
    assert!(ends(script.close(), Duration::from_secs(2)));
    // Waiting for an answer that never comes.
    let mut script = start("function key(k) confirm(\"?\") end\nfunction draw() end");
    assert!(matches!(script.send(Callback::Key("ok".into()), Context::default()), Yield::Asks(_)));
    assert!(ends(script.close(), Duration::from_secs(2)));
}
