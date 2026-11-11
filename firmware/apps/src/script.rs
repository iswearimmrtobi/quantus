//! A script on a thread of its own (apps-roadmap.md F1). The hui's loop cannot wait inside a
//! callback for a key, but `confirm`, `ask`, `form` and `calc` give their answer as the call's
//! value. So the script runs on its own thread, and the main side waits for it to *yield*: the
//! callback is done ([`Yield::Done`]), or the script stands in a host call that needs the hui
//! ([`Yield::Asks`]) until [`Script::answer`] resumes it. The thread also gives the interpreter
//! the stack it needs, and a script that hangs can be left: the main side stops waiting after
//! [`WAIT`], and the operation budget ends the thread.
//!
//! What crosses the channel is plain data ([`Value`], [`Output`]): Lua's own values are not
//! `Send`, and the Lua state lives and dies on the script's thread.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::OnceLock;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use mlua::{Function, IntoLuaMulti, Lua, MultiValue, Table};

use crate::api::{self, Shared};

/// How long the main side waits for a callback before the app counts as not responding.
pub const WAIT: Duration = Duration::from_secs(3);

/// The stack of the script's thread. Lua calls Lua without it, but a call through a host
/// function (a script's `submit`, its `__index` and the like) nests in C, up to Lua's 200 levels:
/// on the board that took about 250 KB (apps.md §5). The stack must hold it, since running over it
/// halts the board instead of giving an error. It comes from PSRAM. A debug build on a PC needs
/// more.
#[cfg(target_os = "espidf")]
pub const STACK: usize = 1024 * 1024;
#[cfg(not(target_os = "espidf"))]
pub const STACK: usize = 4 * 1024 * 1024;

/// What a callback cost, for a log on the board ([`set_probe`]).
#[derive(Clone, Copy, Debug)]
pub enum Probe<'a> {
    /// On the script's thread: a callback and its `draw` ran, in so long.
    Callback(&'a str, Duration),
    /// On the main side: the round trip of a callback or an answer, until the script yielded.
    Yielded(Duration),
}

static PROBE: OnceLock<fn(Probe)> = OnceLock::new();

/// Has every callback reported to `probe`; the board logs the time, its stack and heap.
pub fn set_probe(probe: fn(Probe)) {
    let _ = PROBE.set(probe);
}

static SPAWN: OnceLock<fn(&mut dyn FnMut())> = OnceLock::new();
static STARTED: OnceLock<fn()> = OnceLock::new();

/// Has the script's thread started inside `spawn`, which calls its argument once. The board
/// sets the stack's memory there: from PSRAM, since internal RAM cannot spare it.
pub fn set_spawn(spawn: fn(&mut dyn FnMut())) {
    let _ = SPAWN.set(spawn);
}

/// Has `started` run first on each script's thread. The board notes the thread there, so that
/// what the script allocates comes from PSRAM too.
pub fn set_started(started: fn()) {
    let _ = STARTED.set(started);
}

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
    /// A value of the script's, as plain data: a table whose keys are 1 to n is an array, any
    /// other a map with its keys sorted; a function is the string `"function"`. Tables nested
    /// deeper than [`DEPTH`], a table inside itself among them, are an error.
    pub fn from_lua(value: &mlua::Value) -> Result<Value, String> {
        Value::from_lua_at(value, 0)
    }

    fn from_lua_at(value: &mlua::Value, depth: usize) -> Result<Value, String> {
        Ok(match value {
            mlua::Value::Nil => Value::Unit,
            mlua::Value::Boolean(b) => Value::Bool(*b),
            mlua::Value::Integer(i) => Value::Int(*i),
            mlua::Value::Number(n) => Value::Float(*n),
            mlua::Value::String(s) => Value::Str(s.to_string_lossy()),
            mlua::Value::Table(table) => {
                if depth >= DEPTH {
                    return Err(format!("tables nested deeper than {DEPTH}"));
                }
                Value::from_table(table, depth + 1)?
            }
            other => Value::Str(other.type_name().to_string()),
        })
    }

    fn from_table(table: &Table, depth: usize) -> Result<Value, String> {
        let mut entries = Vec::new();
        for pair in table.pairs::<mlua::Value, mlua::Value>() {
            let (key, value) = pair.map_err(|error| error.to_string())?;
            entries.push((key, Value::from_lua_at(&value, depth)?));
        }
        let length = table.raw_len();
        let sequence = entries.len() == length && entries.iter().all(|(key, _)| matches!(key, mlua::Value::Integer(i) if *i >= 1 && *i as usize <= length));
        if sequence {
            let mut items = vec![Value::Unit; length];
            for (key, value) in entries {
                if let mlua::Value::Integer(i) = key {
                    items[i as usize - 1] = value;
                }
            }
            return Ok(Value::Array(items));
        }
        let mut map: Vec<(String, Value)> = entries
            .into_iter()
            .map(|(key, value)| {
                let key = match key {
                    mlua::Value::String(s) => s.to_string_lossy(),
                    mlua::Value::Integer(i) => i.to_string(),
                    mlua::Value::Number(n) => n.to_string(),
                    other => other.type_name().to_string(),
                };
                (key, value)
            })
            .collect();
        map.sort_by(|(a, _), (b, _)| a.cmp(b));
        Ok(Value::Map(map))
    }

    /// The value for the script.
    pub fn to_lua(&self, lua: &Lua) -> mlua::Result<mlua::Value> {
        Ok(match self {
            Value::Unit => mlua::Value::Nil,
            Value::Bool(b) => mlua::Value::Boolean(*b),
            Value::Int(i) => mlua::Value::Integer(*i),
            Value::Float(f) => mlua::Value::Number(*f),
            Value::Str(s) => mlua::Value::String(lua.create_string(s)?),
            Value::Array(items) => {
                let table = lua.create_table_with_capacity(items.len(), 0)?;
                for (i, item) in items.iter().enumerate() {
                    table.raw_set(i + 1, item.to_lua(lua)?)?;
                }
                mlua::Value::Table(table)
            }
            Value::Map(entries) => {
                let table = lua.create_table_with_capacity(0, entries.len())?;
                for (key, value) in entries {
                    table.raw_set(key.as_str(), value.to_lua(lua)?)?;
                }
                mlua::Value::Table(table)
            }
        })
    }

    /// A map's value at `key`.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Map(entries) => entries.iter().find(|(name, _)| name == key).map(|(_, value)| value),
            _ => None,
        }
    }
}

/// How deep tables may nest in a value that leaves the script.
pub const DEPTH: usize = 32;

/// The name the script's chunk has in Lua's messages, `app:12: …`.
pub(crate) const CHUNK: &str = "app";

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

    /// The error of a Lua call: the innermost message, without Lua's `app:12:` in front, and
    /// the line from it or, for an error of a host function, from where the script called it.
    pub(crate) fn from_lua(error: &mlua::Error) -> Error {
        let mut error = error;
        let mut traceback = None;
        loop {
            match error {
                mlua::Error::CallbackError { traceback: trace, cause } => {
                    traceback.get_or_insert(trace.clone());
                    error = cause;
                }
                mlua::Error::WithContext { cause, .. } => error = cause,
                _ => break,
            }
        }
        let text = match error {
            mlua::Error::RuntimeError(message) => message.clone(),
            mlua::Error::SyntaxError { message, .. } => message.clone(),
            mlua::Error::MemoryError(_) => "not enough memory: the app used more than it may".to_string(),
            error => error.to_string(),
        };
        let (line, message) = match located(&text) {
            Some((line, rest)) => (Some(line), rest.to_string()),
            None => (traceback.as_deref().and_then(located_anywhere), text.clone()),
        };
        Error { message: message.lines().next().unwrap_or_default().trim().to_string(), line, hung: false }
    }
}

/// `app:12: message` as the line and the message.
fn located(text: &str) -> Option<(usize, &str)> {
    let rest = text.strip_prefix(CHUNK)?.strip_prefix(':')?;
    let (line, message) = rest.split_once(':')?;
    Some((line.parse().ok()?, message.trim_start()))
}

/// The first `app:12:` in a traceback.
fn located_anywhere(text: &str) -> Option<usize> {
    let marker = format!("{CHUNK}:");
    text.match_indices(&marker).find_map(|(at, _)| {
        let digits: String = text[at + marker.len()..].chars().take_while(char::is_ascii_digit).collect();
        digits.parse().ok()
    })
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
    fn function(&self, lua: &Lua) -> mlua::Result<Option<(&'static str, MultiValue)>> {
        Ok(Some(match self {
            Callback::Init => ("init", MultiValue::new()),
            Callback::Key(key) => ("key", key.as_str().into_lua_multi(lua)?),
            Callback::Tick(ms) => ("tick", (*ms).into_lua_multi(lua)?),
            Callback::Tap(x, y) => ("tap", (*x, *y).into_lua_multi(lua)?),
            Callback::Entered(value) => ("entered", value.to_lua(lua)?.into_lua_multi(lua)?),
            Callback::Pick(col, row) => ("pick", (*col, *row).into_lua_multi(lua)?),
            Callback::Leave => ("leave", MultiValue::new()),
            Callback::Draw => return Ok(None),
        }))
    }

    /// The name of its function, for the board's log.
    fn name(&self) -> &'static str {
        match self {
            Callback::Init => "init",
            Callback::Key(_) => "key",
            Callback::Tick(_) => "tick",
            Callback::Tap(..) => "tap",
            Callback::Entered(_) => "entered",
            Callback::Pick(..) => "pick",
            Callback::Leave => "leave",
            Callback::Draw => "draw",
        }
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
    /// A form to show, its description as plain data. The answer is `#{submit: values}` when
    /// the app's `submit` is to work it out, which yields [`Request::Submitted`], or the results
    /// when the form closes.
    Form(Value),
    /// What `submit` gave: the results, or an error for a field or the form.
    Submitted(Value),
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
    /// Compiled, and whether the script has a `draw` of its own.
    Compiled(Result<bool, Error>),
    Yield(Yield),
}

/// The main side of a running script.
pub struct Script {
    to: Option<Sender<Command>>,
    from: Receiver<Reply>,
    thread: Option<JoinHandle<()>>,
    /// The script stopped answering or failed: it gets nothing more.
    dead: bool,
    /// Whether it has `draw`: a script that is only a `form(...)` call has none.
    pub draws: bool,
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
        let mut job = Some(move || run(&source, commands, replies));
        let mut spawned = None;
        let mut spawn = || {
            let job = job.take().expect("spawned once");
            spawned = Some(std::thread::Builder::new().name("app".into()).stack_size(STACK).spawn(job));
        };
        match SPAWN.get() {
            Some(wrap) => wrap(&mut spawn),
            None => spawn(),
        }
        let thread = match spawned {
            Some(Ok(thread)) => thread,
            Some(Err(error)) => return Err(Error::new(format!("no thread for the app: {error}"))),
            None => return Err(Error::new("no thread for the app")),
        };
        let mut script = Script { to: Some(to), from, thread: Some(thread), dead: false, draws: true };
        match script.from.recv_timeout(WAIT) {
            Ok(Reply::Compiled(Ok(draws))) => {
                script.draws = draws;
                Ok(script)
            }
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
        let started = Instant::now();
        let reply = match self.from.recv_timeout(WAIT) {
            Ok(Reply::Yield(reply)) => reply,
            Ok(Reply::Compiled(_)) => Yield::Failed(Error::new("the script answered out of turn")),
            Err(RecvTimeoutError::Timeout) => Yield::Failed(Error::hung()),
            Err(RecvTimeoutError::Disconnected) => Yield::Failed(Error::new("the app has stopped")),
        };
        if let Some(probe) = PROBE.get() {
            probe(Probe::Yielded(started.elapsed()));
        }
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

/// Whether `source` defines `draw`: known before its top level runs, which may show a form and
/// wait. A script that draws has `function draw()`, as the apps' page says.
fn defines_draw(source: &str) -> bool {
    source.lines().any(|line| {
        let line = line.trim_start();
        let line = line.strip_prefix("local ").unwrap_or(line).trim_start();
        line.strip_prefix("function").is_some_and(|rest| rest.trim_start().strip_prefix("draw").is_some_and(|rest| rest.trim_start().starts_with('(')))
            || line.strip_prefix("draw").is_some_and(|rest| rest.trim_start().strip_prefix('=').is_some_and(|rest| rest.trim_start().starts_with("function")))
    })
}

/// The script's thread: compiles, then runs callbacks until the channel closes.
fn run(source: &str, commands: Receiver<Command>, replies: Sender<Reply>) {
    if let Some(started) = STARTED.get() {
        started();
    }
    let commands = Rc::new(commands);
    let shared = Rc::new(RefCell::new(Shared::default()));
    let operations = Rc::new(Cell::new(0));
    let started = Instant::now();
    let compiled = api::lua(shared.clone(), commands.clone(), replies.clone(), operations.clone())
        .and_then(|lua| lua.load(source).set_name(format!("={CHUNK}")).into_function().map(|chunk| (lua, chunk)));
    if let Some(probe) = PROBE.get() {
        probe(Probe::Callback("compile", started.elapsed()));
    }
    let (lua, chunk) = match compiled {
        Ok(compiled) => compiled,
        Err(error) => {
            let _ = replies.send(Reply::Compiled(Err(Error::from_lua(&error))));
            return;
        }
    };
    if replies.send(Reply::Compiled(Ok(defines_draw(source)))).is_err() {
        return;
    }
    while let Ok(command) = commands.recv() {
        let Command::Call(callback, context) = command else {
            let _ = replies.send(Reply::Yield(Yield::Failed(Error::new("an answer nobody asked for"))));
            continue;
        };
        shared.borrow_mut().begin(context);
        operations.set(0);
        let started = Instant::now();
        let ran = callback_and_draw(&lua, &chunk, &callback);
        if let Some(probe) = PROBE.get() {
            probe(Probe::Callback(callback.name(), started.elapsed()));
        }
        let reply = match ran {
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
fn callback_and_draw(lua: &Lua, chunk: &Function, callback: &Callback) -> Result<bool, Error> {
    if *callback == Callback::Init {
        chunk.call::<()>(()).map_err(|error| Error::from_lua(&error))?;
    }
    if let Some((name, args)) = callback.function(lua).map_err(|error| Error::from_lua(&error))? {
        if let Some(function) = global(lua, name) {
            function.call::<()>(args).map_err(|error| Error::from_lua(&error))?;
        }
    }
    if *callback == Callback::Leave {
        return Ok(false);
    }
    let Some(draw) = global(lua, "draw") else { return Ok(false) };
    draw.call::<()>(()).map_err(|error| Error::from_lua(&error))?;
    Ok(true)
}

/// The script's function `name`, if it has one.
fn global(lua: &Lua, name: &str) -> Option<Function> {
    match lua.globals().raw_get::<mlua::Value>(name) {
        Ok(mlua::Value::Function(function)) => Some(function),
        _ => None,
    }
}
