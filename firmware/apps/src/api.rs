//! The script API (apps.md §4): the functions a script can call, registered on its engine, and
//! the limits that keep a script from harming the handheld (§5).

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender};

use rhai::{Engine, EvalAltResult};

use crate::script::{Command, Context, Output, Reply, Request, Value, Yield};

/// The largest script file.
pub const MAX_SOURCE: usize = 256 * 1024;

/// What a callback may do before it counts as hung. Provisional, set on a PC; the board's
/// number comes from the spike (apps-roadmap.md 0.4), aiming at about 200 ms there.
pub const BUDGET: u64 = 500_000;

/// How deep calls may nest.
pub const CALL_LEVELS: usize = 32;

/// The longest string, the largest array and map a script may make.
pub const MAX_STRING: usize = 16 * 1024;
pub const MAX_ARRAY: usize = 10_000;
pub const MAX_MAP: usize = 1_000;

/// What the script set and drew in the callback that runs, and what it may know.
#[derive(Default)]
pub struct Shared {
    pub context: Context,
    output: Output,
}

impl Shared {
    /// A callback starts with `context`.
    pub fn begin(&mut self, context: Context) {
        self.context = context;
        self.output = Output::default();
    }

    /// What the callback left.
    pub fn take(&mut self) -> Output {
        std::mem::take(&mut self.output)
    }
}

/// A drawing command, replayed into the hui's canvas (apps.md §4.3).
#[derive(Clone, Debug, PartialEq)]
pub enum Draw {}

type Fallible<T> = Result<T, Box<EvalAltResult>>;

/// The engine of one script: its limits and the API, over the state it shares with the host
/// functions and the channels a host call that needs the hui waits on.
pub(crate) fn engine(_shared: Rc<RefCell<Shared>>, commands: Rc<Receiver<Command>>, replies: Sender<Reply>) -> Engine {
    let mut engine = Engine::new();
    engine.set_max_operations(BUDGET);
    engine.set_max_call_levels(CALL_LEVELS);
    engine.set_max_expr_depths(64, 32);
    engine.set_max_string_size(MAX_STRING);
    engine.set_max_array_size(MAX_ARRAY);
    engine.set_max_map_size(MAX_MAP);
    engine.on_print(|_| {});
    engine.on_debug(|_, _, _| {});

    // A question with Ja and Nein, answered by the hui: the script waits for it.
    engine.register_fn("confirm", move |question: &str| -> Fallible<bool> {
        match ask(&commands, &replies, Request::Confirm(question.to_string()))? {
            Value::Bool(yes) => Ok(yes),
            _ => Ok(false),
        }
    });
    engine
}

/// Hands `request` to the main side and waits for its answer.
fn ask(commands: &Receiver<Command>, replies: &Sender<Reply>, request: Request) -> Fallible<Value> {
    if replies.send(Reply::Yield(Yield::Asks(request))).is_err() {
        return Err("the app was left".into());
    }
    match commands.recv() {
        Ok(Command::Answer(value)) => Ok(value),
        Ok(Command::Call(..)) => Err("a callback before the answer".into()),
        Err(_) => Err("the app was left".into()),
    }
}
