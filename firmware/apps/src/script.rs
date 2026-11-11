//! A script on a thread of its own (apps-roadmap.md F1). The hui's loop cannot wait inside a
//! callback for a key, but `confirm`, `ask`, `form` and `calc` give their answer as the call's
//! value. So the script runs on its own thread, and the main side waits for it to *yield*: the
//! callback is done ([`Yield::Done`]), or the script stands in a host call that needs the hui
//! ([`Yield::Asks`]) until [`Script::answer`] resumes it. The thread also gives the interpreter
//! the stack it needs, and a script that hangs can be left: the main side stops waiting after
//! [`WAIT`], and the operation budget ends the thread.
//!
//! What crosses the channel is plain data ([`Value`], [`Output`]): Rhai's own values are not
//! `Send`, and the engine and the AST live and die on the script's thread.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::JoinHandle;
use std::time::Duration;

use rhai::{CallFnOptions, Dynamic, Engine, EvalAltResult, Map, Position, Scope, AST};

use crate::api::{self, Shared};

/// How long the main side waits for a callback before the app counts as not responding.
pub const WAIT: Duration = Duration::from_secs(3);

/// The stack of the script's thread. On the board the number waits for the spike (step 0.3);
/// a debug build on a PC needs far more per call level.
#[cfg(target_os = "espidf")]
pub const STACK: usize = 64 * 1024;
#[cfg(not(target_os = "espidf"))]
pub const STACK: usize = 4 * 1024 * 1024;

/// A value of a script, as plain data.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Unit,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Array(Vec<Value>),
    /// The keys in order.
    Map(Vec<(String, Value)>),
}

impl Value {
    pub fn from_dynamic(value: &Dynamic) -> Value {
        if value.is_unit() {
            Value::Unit
        } else if let Ok(b) = value.as_bool() {
            Value::Bool(b)
        } else if let Ok(i) = value.as_int() {
            Value::Int(i)
        } else if let Ok(f) = value.as_float() {
            Value::Float(f)
        } else if value.is_string() {
            Value::Str(value.clone().into_string().unwrap_or_default())
        } else if value.is_array() {
            let array = value.clone().into_array().unwrap_or_default();
            Value::Array(array.iter().map(Value::from_dynamic).collect())
        } else if value.is_map() {
            let map = value.clone().cast::<Map>();
            Value::Map(map.iter().map(|(key, value)| (key.to_string(), Value::from_dynamic(value))).collect())
        } else if value.is_char() {
            Value::Str(value.to_string())
        } else {
            Value::Str(value.type_name().to_string())
        }
    }

    pub fn to_dynamic(&self) -> Dynamic {
        match self {
            Value::Unit => Dynamic::UNIT,
            Value::Bool(b) => (*b).into(),
            Value::Int(i) => (*i).into(),
            Value::Float(f) => (*f).into(),
            Value::Str(s) => s.clone().into(),
            Value::Array(items) => Dynamic::from_array(items.iter().map(Value::to_dynamic).collect()),
            Value::Map(entries) => {
                let map: Map = entries.iter().map(|(key, value)| (key.as_str().into(), value.to_dynamic())).collect();
                Dynamic::from_map(map)
            }
        }
    }

    /// A map's value at `key`.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Map(entries) => entries.iter().find(|(name, _)| name == key).map(|(_, value)| value),
            _ => None,
        }
    }
}

/// What went wrong in a script: the message, and the line where the script was.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub message: String,
    pub line: Option<usize>,
    /// The callback did not come back in time.
    pub hung: bool,
}

impl Error {
    pub fn new(message: impl Into<String>) -> Error {
        Error { message: message.into(), line: None, hung: false }
    }

    fn hung() -> Error {
        Error { message: "not responding".into(), line: None, hung: true }
    }

    fn at(message: impl Into<String>, position: Position) -> Error {
        Error { message: message.into(), line: position.line(), hung: false }
    }

    pub(crate) fn from_eval(error: &EvalAltResult) -> Error {
        // The innermost error of a call, with its own line.
        let mut error = error;
        while let EvalAltResult::ErrorInFunctionCall(_, _, inner, _) = error {
            error = inner;
        }
        let message = match error {
            // `throw`: what was thrown.
            EvalAltResult::ErrorRuntime(value, _) => value.to_string(),
            error => error.to_string().split(" (line ").next().unwrap_or_default().to_string(),
        };
        Error::at(message, error.position())
    }
}

/// A callback of the script (apps.md §4.1).
#[derive(Clone, Debug, PartialEq)]
pub enum Callback {
    /// The top level of the file, then `init()`.
    Init,
    Key(String),
    Tick(i64),
    Tap(i64, i64),
    Entered(Value),
    Pick(i64, i64),
    Leave,
    /// Only `draw()`: the area changed.
    Draw,
}

impl Callback {
    /// The script's function and its arguments; `None` for the ones only drawn.
    fn function(&self) -> Option<(&'static str, Vec<Dynamic>)> {
        Some(match self {
            Callback::Init => ("init", Vec::new()),
            Callback::Key(key) => ("key", vec![key.clone().into()]),
            Callback::Tick(ms) => ("tick", vec![(*ms).into()]),
            Callback::Tap(x, y) => ("tap", vec![(*x).into(), (*y).into()]),
            Callback::Entered(value) => ("entered", vec![value.to_dynamic()]),
            Callback::Pick(col, row) => ("pick", vec![(*col).into(), (*row).into()]),
            Callback::Leave => ("leave", Vec::new()),
            Callback::Draw => return None,
        })
    }
}

/// What the script may know of the device when a callback runs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Context {
    /// The app's area.
    pub width: i32,
    pub height: i32,
    /// The grid template's cursor.
    pub cursor: (usize, usize),
    /// `"de"` or `"en"`.
    pub lang: String,
    /// Unix milliseconds, if the clock is set.
    pub now: Option<i64>,
    /// Seeds the random numbers of this callback; the host's, so a seeded simulator repeats.
    pub seed: u32,
    /// The app's saved data, as the script last saved it or the card had it.
    pub data: Option<Value>,
}

/// What a callback asks of the hui before it can go on.
#[derive(Clone, Debug, PartialEq)]
pub enum Request {
    /// A question with *Ja* and *Nein*; the answer is a [`Value::Bool`].
    Confirm(String),
    /// A line of plain text: its label, the longest, what stands in it at first; the answer is
    /// a [`Value::Str`], or [`Value::Unit`] after AC.
    Ask { label: String, max: usize, value: String },
    /// A calculation; the answer is `#{ok, value, latex, error}`.
    Calc(String),
    /// A variable of the session; the answer is its value, or `()`.
    Var(String),
}

/// What a callback that ran to its end leaves for the hui: what `draw` drew and what the script
/// set through the API meanwhile.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Output {
    pub draws: Vec<api::Draw>,
    /// Whether `draw` ran.
    pub drawn: bool,
    /// The soft keys' labels, if `soft` was called.
    pub soft: Option<Vec<String>>,
    /// The input template, if `input` was called.
    pub input: Option<quantus_ui::applet::Input>,
    /// The timer, if `timer` was called: every so many ms, 0 for none.
    pub timer: Option<u64>,
    /// Where `cursor` put the grid's cursor.
    pub cursor: Option<(usize, usize)>,
    /// `exit` was called.
    pub exit: bool,
    /// What `save` saved last, as JSON.
    pub save: Option<String>,
    /// LaTeX `insert` hands to the calculation; the app leaves.
    pub insert: Option<String>,
}

/// Where the main side is with the script.
#[derive(Debug, PartialEq)]
pub enum Yield {
    Done(Output),
    Asks(Request),
    Failed(Error),
}

/// From the main side to the script's thread.
pub(crate) enum Command {
    Call(Callback, Context),
    Answer(Value),
}

/// From the script's thread to the main side.
pub(crate) enum Reply {
    Compiled(Result<(), Error>),
    Yield(Yield),
}

/// The main side of a running script.
pub struct Script {
    to: Option<Sender<Command>>,
    from: Receiver<Reply>,
    thread: Option<JoinHandle<()>>,
    /// The script stopped answering or failed: it gets nothing more.
    dead: bool,
}

impl Script {
    /// Compiles `source` on a new thread; its top level and `init` run with the first
    /// [`Callback::Init`].
    pub fn start(source: &str) -> Result<Script, Error> {
        if source.len() > api::MAX_SOURCE {
            return Err(Error::new(format!("the script is over {} KB", api::MAX_SOURCE / 1024)));
        }
        let (to, commands) = mpsc::channel();
        let (replies, from) = mpsc::channel();
        let source = source.to_string();
        let thread = std::thread::Builder::new()
            .name("app".into())
            .stack_size(STACK)
            .spawn(move || run(&source, commands, replies))
            .map_err(|error| Error::new(format!("no thread for the app: {error}")))?;
        let script = Script { to: Some(to), from, thread: Some(thread), dead: false };
        match script.from.recv_timeout(WAIT) {
            Ok(Reply::Compiled(Ok(()))) => Ok(script),
            Ok(Reply::Compiled(Err(error))) => Err(error),
            Ok(Reply::Yield(_)) => Err(Error::new("the script answered out of turn")),
            Err(_) => Err(Error::hung()),
        }
    }

    /// Runs `callback` and then `draw`, as far as the script gets.
    pub fn send(&mut self, callback: Callback, context: Context) -> Yield {
        self.command(Command::Call(callback, context))
    }

    /// The answer to the request the script stands in.
    pub fn answer(&mut self, value: Value) -> Yield {
        self.command(Command::Answer(value))
    }

    fn command(&mut self, command: Command) -> Yield {
        if self.dead {
            return Yield::Failed(Error::new("the app has stopped"));
        }
        let sent = self.to.as_ref().is_some_and(|to| to.send(command).is_ok());
        if !sent {
            self.dead = true;
            return Yield::Failed(Error::new("the app has stopped"));
        }
        let reply = match self.from.recv_timeout(WAIT) {
            Ok(Reply::Yield(reply)) => reply,
            Ok(Reply::Compiled(_)) => Yield::Failed(Error::new("the script answered out of turn")),
            Err(RecvTimeoutError::Timeout) => Yield::Failed(Error::hung()),
            Err(RecvTimeoutError::Disconnected) => Yield::Failed(Error::new("the app has stopped")),
        };
        if matches!(reply, Yield::Failed(_)) {
            self.dead = true;
        }
        reply
    }

    /// Lets go of the script and hands back its thread, which ends once it sees the channel
    /// closed: at once if it waits, by the operation budget if it is inside a callback.
    pub fn close(mut self) -> JoinHandle<()> {
        self.to = None;
        self.thread.take().expect("a script has its thread")
    }
}

impl Drop for Script {
    fn drop(&mut self) {
        // Closing the channel ends the thread; it is not waited for.
        self.to = None;
    }
}

/// The script's thread: compiles, then runs callbacks until the channel closes.
fn run(source: &str, commands: Receiver<Command>, replies: Sender<Reply>) {
    let commands = Rc::new(commands);
    let shared = Rc::new(RefCell::new(Shared::default()));
    let engine = api::engine(shared.clone(), commands.clone(), replies.clone());
    let ast = match engine.compile(source) {
        Ok(ast) => ast,
        Err(error) => {
            let message = error.err_type().to_string();
            let _ = replies.send(Reply::Compiled(Err(Error::at(message, error.position()))));
            return;
        }
    };
    if !ast.iter_functions().any(|function| function.name == "draw" && function.params.is_empty()) {
        let _ = replies.send(Reply::Compiled(Err(Error::new("the app has no draw()"))));
        return;
    }
    if replies.send(Reply::Compiled(Ok(()))).is_err() {
        return;
    }
    let mut state = Dynamic::from_map(Map::new());
    let mut scope = Scope::new();
    while let Ok(command) = commands.recv() {
        let Command::Call(callback, context) = command else {
            let _ = replies.send(Reply::Yield(Yield::Failed(Error::new("an answer nobody asked for"))));
            continue;
        };
        shared.borrow_mut().begin(context);
        let reply = match callback_and_draw(&engine, &ast, &mut scope, &mut state, &callback) {
            Ok(drawn) => {
                let mut output = shared.borrow_mut().take();
                output.drawn = drawn;
                Yield::Done(output)
            }
            Err(error) => Yield::Failed(error),
        };
        if replies.send(Reply::Yield(reply)).is_err() {
            return;
        }
    }
}

/// Runs the callback if the script has it, and `draw` after it but after `leave`; whether
/// `draw` ran.
fn callback_and_draw(engine: &Engine, ast: &AST, scope: &mut Scope, state: &mut Dynamic, callback: &Callback) -> Result<bool, Error> {
    if *callback == Callback::Init {
        engine.run_ast_with_scope(scope, ast).map_err(|error| Error::from_eval(&error))?;
    }
    if let Some((name, args)) = callback.function() {
        if ast.iter_functions().any(|function| function.name == name && function.params.len() == args.len()) {
            call(engine, ast, scope, state, name, args)?;
        }
    }
    if *callback == Callback::Leave {
        return Ok(false);
    }
    call(engine, ast, scope, state, "draw", Vec::new())?;
    Ok(true)
}

fn call(engine: &Engine, ast: &AST, scope: &mut Scope, state: &mut Dynamic, name: &str, args: Vec<Dynamic>) -> Result<(), Error> {
    let options = CallFnOptions::new().eval_ast(false).rewind_scope(true).bind_this_ptr(state);
    engine.call_fn_with_options::<Dynamic>(options, scope, ast, name, args).map(|_| ()).map_err(|error| Error::from_eval(&error))
}
