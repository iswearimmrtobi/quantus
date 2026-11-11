//! The script API (apps.md §4): the functions a script can call, set on its Lua state, and the
//! limits that keep a script from harming the handheld (§5).

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender};

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::Point;
use embedded_graphics::primitives::Rectangle;
use mlua::{Function, HookTriggers, Lua, LuaOptions, StdLib, Table, Variadic, VmState};
use quantus_ui::applet::{self, Canvas, Grid, Input, Palette, MAX_SHAPES};

use crate::script::{Command, Context, Output, Reply, Request, Value, Yield};

/// The largest script file.
pub const MAX_SOURCE: usize = 256 * 1024;

/// The Lua instructions a callback may run before it is stopped: about 200 ms on the board,
/// which runs some 5,000 a millisecond (apps.md §5); a move of either game takes under 10,000.
pub const BUDGET: u64 = 1_000_000;

/// How often the budget is looked at, in instructions.
const EVERY: u32 = 1000;

/// The memory a script's Lua state may take.
pub const MEMORY: usize = 4 * 1024 * 1024;

/// The message of a callback stopped by the budget.
pub const TOO_LONG: &str = "too many operations: the app ran too long without an answer";

/// What the script set and drew in the callback that runs, and what it may know.
#[derive(Default)]
pub struct Shared {
    pub context: Context,
    output: Output,
    /// The random numbers' state, seeded for each callback.
    rng: u32,
    /// The last board `grid` laid out, for `side`.
    grid: Option<Grid>,
}

impl Shared {
    /// The next number of the generator (xorshift).
    fn next(&mut self) -> u32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        x
    }

    fn draw(&mut self, draw: Draw) -> mlua::Result<()> {
        if self.output.draws.len() >= MAX_SHAPES {
            return Err(mlua::Error::runtime(format!("more than {MAX_SHAPES} shapes in one frame")));
        }
        self.output.draws.push(draw);
        Ok(())
    }

    /// A callback starts with `context`.
    pub fn begin(&mut self, context: Context) {
        self.rng = context.seed | 1;
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
pub enum Draw {
    Clear(Rgb565),
    Rect(i32, i32, i32, i32, Rgb565),
    Round(i32, i32, i32, i32, i32, Rgb565),
    Frame(i32, i32, i32, i32, i32, Rgb565),
    Circle(i32, i32, i32, Rgb565),
    Line(i32, i32, i32, i32, Rgb565, i32),
    /// Text, in the small font or the normal one.
    Text(i32, i32, String, Rgb565, bool),
    Math(i32, i32, String, Rgb565),
}

impl Draw {
    /// Draws it into `canvas`.
    pub fn replay(&self, canvas: &mut Canvas) {
        match self {
            Draw::Clear(c) => canvas.clear(*c),
            &Draw::Rect(x, y, w, h, c) => canvas.rect(x, y, w, h, c),
            &Draw::Round(x, y, w, h, r, c) => canvas.round(x, y, w, h, r, c),
            &Draw::Frame(x, y, w, h, r, c) => canvas.frame(x, y, w, h, r, c),
            &Draw::Circle(x, y, d, c) => canvas.circle(x, y, d, c),
            &Draw::Line(x1, y1, x2, y2, c, w) => canvas.line(x1, y1, x2, y2, c, w),
            Draw::Text(x, y, text, c, false) => canvas.text(*x, *y, text, *c),
            Draw::Text(x, y, text, c, true) => canvas.text_small(*x, *y, text, *c),
            Draw::Math(x, y, latex, c) => canvas.math(*x, *y, latex, *c),
        }
    }
}

type V = mlua::Value;

/// A whole number from a script's number, whole or not: Lua's `/` always gives a float.
pub(crate) fn number(value: &V) -> mlua::Result<i32> {
    match value {
        V::Integer(i) => Ok((*i).clamp(i32::MIN as i64, i32::MAX as i64) as i32),
        V::Number(f) if f.is_finite() => Ok(f.round().clamp(i32::MIN as f64, i32::MAX as f64) as i32),
        other => Err(mlua::Error::runtime(format!("a number was expected, not {}", other.type_name()))),
    }
}

/// A colour by its name in the palette, or as `rgb` gives it: `#rrggbb`.
pub(crate) fn color(value: &V) -> mlua::Result<Rgb565> {
    let V::String(name) = value else {
        return Err(mlua::Error::runtime(format!("a colour was expected, not {}", value.type_name())));
    };
    let name = name.to_string_lossy();
    if let Some(color) = Palette::get(&name) {
        return Ok(color);
    }
    let hex = name.strip_prefix('#').filter(|hex| hex.len() == 6).and_then(|hex| u32::from_str_radix(hex, 16).ok());
    match hex {
        Some(rgb) => Ok(Palette::rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)),
        None => Err(mlua::Error::runtime(format!("no colour called {name:?}"))),
    }
}

/// A value as text, as `tostring` writes it; `nil` is empty.
pub(crate) fn text_of(value: &V) -> String {
    match value {
        V::Nil => String::new(),
        V::String(s) => s.to_string_lossy(),
        V::Integer(i) => i.to_string(),
        V::Number(n) if n.fract() == 0.0 && n.abs() < 1e15 => format!("{n:.1}"),
        V::Number(n) => n.to_string(),
        V::Boolean(b) => b.to_string(),
        other => other.type_name().to_string(),
    }
}

/// A table of whole numbers.
fn numbers(lua: &Lua, entries: &[(&str, i64)]) -> mlua::Result<Table> {
    let table = lua.create_table_with_capacity(0, entries.len())?;
    for (key, value) in entries {
        table.raw_set(*key, *value)?;
    }
    Ok(table)
}

fn rect_table(lua: &Lua, rect: Rectangle) -> mlua::Result<Table> {
    numbers(lua, &[
        ("x", rect.top_left.x.into()),
        ("y", rect.top_left.y.into()),
        ("w", rect.size.width.into()),
        ("h", rect.size.height.into()),
    ])
}

/// The Lua state of one script: the standard libraries that are safe, the limits and the API,
/// over the state it shares with the host functions, the channels a host call that needs the
/// hui waits on, and the instructions the callback has run.
pub(crate) fn lua(shared: Rc<RefCell<Shared>>, commands: Rc<Receiver<Command>>, replies: Sender<Reply>, operations: Rc<Cell<u64>>) -> mlua::Result<Lua> {
    // No io, os, package, debug or coroutines: a script reaches only what the host gives it.
    let lua = Lua::new_with(StdLib::STRING | StdLib::TABLE | StdLib::MATH | StdLib::UTF8, LuaOptions::default())?;
    lua.set_memory_limit(MEMORY)?;
    let globals = lua.globals();
    // Running code from text or files, and catching errors: `pcall` could catch the budget's
    // error and go on for ever, so a failing call ends the callback.
    for name in ["dofile", "loadfile", "load", "require", "collectgarbage", "pcall", "xpcall"] {
        globals.raw_set(name, V::Nil)?;
    }
    globals.raw_set("print", lua.create_function(|_, _: Variadic<V>| Ok(()))?)?;

    lua.set_hook(HookTriggers::new().every_nth_instruction(EVERY), move |_, debug| {
        let done = operations.get() + u64::from(EVERY);
        operations.set(done);
        if done > BUDGET {
            // With the line the script was on, as Lua's own errors have it.
            let message = match debug.current_line() {
                Some(line) => format!("{}:{line}: {TOO_LONG}", crate::script::CHUNK),
                None => TOO_LONG.to_string(),
            };
            return Err(mlua::Error::runtime(message));
        }
        Ok(VmState::Continue)
    })?;

    drawing(&lua, &shared)?;
    templates(&lua, &shared)?;
    helpers(&lua, &shared)?;
    asking(&lua, &shared, &commands, &replies)?;
    drop(globals);
    Ok(lua)
}

/// The screen and drawing (apps.md §4.3).
fn drawing(lua: &Lua, shared: &Rc<RefCell<Shared>>) -> mlua::Result<()> {
    let g = lua.globals();
    let s = shared.clone();
    g.raw_set("width", lua.create_function(move |_, ()| Ok(s.borrow().context.width))?)?;
    let s = shared.clone();
    g.raw_set("height", lua.create_function(move |_, ()| Ok(s.borrow().context.height))?)?;
    let s = shared.clone();
    g.raw_set("clear", lua.create_function(move |_, c: V| s.borrow_mut().draw(Draw::Clear(color(&c)?)))?)?;
    let s = shared.clone();
    g.raw_set("rect", lua.create_function(move |_, (x, y, w, h, c): (V, V, V, V, V)| {
        s.borrow_mut().draw(Draw::Rect(number(&x)?, number(&y)?, number(&w)?, number(&h)?, color(&c)?))
    })?)?;
    let s = shared.clone();
    g.raw_set("round", lua.create_function(move |_, (x, y, w, h, r, c): (V, V, V, V, V, V)| {
        s.borrow_mut().draw(Draw::Round(number(&x)?, number(&y)?, number(&w)?, number(&h)?, number(&r)?, color(&c)?))
    })?)?;
    let s = shared.clone();
    g.raw_set("frame", lua.create_function(move |_, (x, y, w, h, r, c): (V, V, V, V, V, V)| {
        s.borrow_mut().draw(Draw::Frame(number(&x)?, number(&y)?, number(&w)?, number(&h)?, number(&r)?, color(&c)?))
    })?)?;
    let s = shared.clone();
    g.raw_set("circle", lua.create_function(move |_, (x, y, d, c): (V, V, V, V)| {
        s.borrow_mut().draw(Draw::Circle(number(&x)?, number(&y)?, number(&d)?, color(&c)?))
    })?)?;
    let s = shared.clone();
    g.raw_set("line", lua.create_function(move |_, (x1, y1, x2, y2, c, w): (V, V, V, V, V, V)| {
        let w = if w.is_nil() { 1 } else { number(&w)? };
        s.borrow_mut().draw(Draw::Line(number(&x1)?, number(&y1)?, number(&x2)?, number(&y2)?, color(&c)?, w))
    })?)?;
    for (name, small) in [("text", false), ("text_small", true)] {
        let s = shared.clone();
        g.raw_set(name, lua.create_function(move |_, (x, y, text, c): (V, V, V, V)| {
            s.borrow_mut().draw(Draw::Text(number(&x)?, number(&y)?, text_of(&text), color(&c)?, small))
        })?)?;
    }
    let s = shared.clone();
    g.raw_set("formula", lua.create_function(move |_, (x, y, latex, c): (V, V, V, V)| {
        s.borrow_mut().draw(Draw::Math(number(&x)?, number(&y)?, text_of(&latex), color(&c)?))
    })?)?;
    g.raw_set("text_width", lua.create_function(|_, text: V| Ok(applet::text_width(&text_of(&text))))?)?;
    g.raw_set("text_small_width", lua.create_function(|_, text: V| Ok(applet::text_small_width(&text_of(&text))))?)?;
    g.raw_set("text_height", lua.create_function(|_, ()| Ok(applet::text_height()))?)?;
    g.raw_set("text_small_height", lua.create_function(|_, ()| Ok(applet::text_small_height()))?)?;
    g.raw_set("rgb", lua.create_function(|_, (r, g, b): (V, V, V)| {
        let channel = |v: &V| number(v).map(|v| v.clamp(0, 255));
        Ok(format!("#{:02x}{:02x}{:02x}", channel(&r)?, channel(&g)?, channel(&b)?))
    })?)?;
    Ok(())
}

/// The soft keys and the input templates (apps.md §4.3, §4.4).
fn templates(lua: &Lua, shared: &Rc<RefCell<Shared>>) -> mlua::Result<()> {
    let g = lua.globals();
    let s = shared.clone();
    g.raw_set("soft", lua.create_function(move |_, labels: Option<Table>| {
        let labels = match labels {
            Some(labels) => {
                let count = labels.raw_len();
                (1..=count).map(|i| labels.raw_get::<V>(i).map(|label| if matches!(label, V::Boolean(false)) { String::new() } else { text_of(&label) })).collect::<mlua::Result<Vec<_>>>()?
            }
            None => Vec::new(),
        };
        s.borrow_mut().output.soft = Some(labels);
        Ok(())
    })?)?;
    let s = shared.clone();
    g.raw_set("input", lua.create_function(move |_, (kind, options): (String, Option<Table>)| {
        s.borrow_mut().output.input = Some(input(&kind, options.as_ref())?);
        Ok(())
    })?)?;
    // `cursor()` gives the grid's cursor, `cursor(col, row)` sets it; both count from 1.
    let s = shared.clone();
    g.raw_set("cursor", lua.create_function(move |lua, args: Variadic<V>| {
        if args.is_empty() {
            let (col, row) = s.borrow().context.cursor;
            return Ok(V::Table(numbers(lua, &[("col", col as i64 + 1), ("row", row as i64 + 1)])?));
        }
        let col = number(args.first().unwrap_or(&V::Nil))?;
        let row = number(args.get(1).unwrap_or(&V::Nil))?;
        let cursor = ((col - 1).max(0) as usize, (row - 1).max(0) as usize);
        let mut shared = s.borrow_mut();
        shared.context.cursor = cursor;
        shared.output.cursor = Some(cursor);
        Ok(V::Nil)
    })?)?;
    Ok(())
}

/// The template `kind` with its `options`.
fn input(kind: &str, options: Option<&Table>) -> mlua::Result<Input> {
    let get = |name: &str| options.map_or(Ok(V::Nil), |options| options.raw_get::<V>(name));
    let int = |name: &str, default: usize, most: i32| -> mlua::Result<usize> {
        let value = get(name)?;
        Ok(if value.is_nil() { default } else { number(&value)?.clamp(1, most) as usize })
    };
    let flag = |name: &str| -> mlua::Result<bool> { Ok(matches!(get(name)?, V::Boolean(true))) };
    let text = |name: &str| -> mlua::Result<String> { Ok(text_of(&get(name)?)) };
    Ok(match kind {
        "keys" => Input::Keys,
        "grid" => Input::Grid { cols: int("cols", 3, 64)?, rows: int("rows", 3, 64)?, digits: flag("digits")?, wrap: flag("wrap")? },
        "cells" => Input::Cells { cols: int("cols", 3, 64)?, rows: int("rows", 3, 64)?, digits_only: flag("digits_only")? },
        "list" => {
            let items = match get("items")? {
                V::Table(items) => (1..=items.raw_len()).map(|i| items.raw_get::<V>(i).map(|item| text_of(&item))).collect::<mlua::Result<Vec<_>>>()?,
                _ => Vec::new(),
            };
            Input::List { items }
        }
        "line" => Input::Line { label: text("label")?, calc: flag("calc")? },
        "text" => Input::Text { label: text("label")?, max: int("max", 32, 256)?, value: text("value")? },
        kind => return Err(mlua::Error::runtime(format!("no input called {kind:?}"))),
    })
}

/// The board a table from `grid` stands for.
fn grid_of(table: &Table) -> mlua::Result<Grid> {
    let get = |name: &str| -> mlua::Result<i32> {
        match table.raw_get::<V>(name)? {
            V::Nil => Err(mlua::Error::runtime(format!("not a grid: no {name}"))),
            value => number(&value),
        }
    };
    let (width, height, cols, rows) = (get("area_w")?, get("area_h")?, get("cols")?, get("rows")?);
    Ok(Grid::new(width, height, cols.max(1) as usize, rows.max(1) as usize))
}

/// The games' helpers (apps.md §4.5).
fn helpers(lua: &Lua, shared: &Rc<RefCell<Shared>>) -> mlua::Result<()> {
    let g = lua.globals();
    // A grid's methods: `g:cell(col, row)` and `g:at(x, y)`, counting from 1.
    let methods = lua.create_table()?;
    methods.raw_set("cell", lua.create_function(|lua, (grid, col, row): (Table, V, V)| {
        let (col, row) = (number(&col)?, number(&row)?);
        rect_table(lua, grid_of(&grid)?.cell((col - 1).max(0) as usize, (row - 1).max(0) as usize))
    })?)?;
    methods.raw_set("at", lua.create_function(|lua, (grid, x, y): (Table, V, V)| {
        Ok(match grid_of(&grid)?.at(Point::new(number(&x)?, number(&y)?)) {
            Some((col, row)) => V::Table(numbers(lua, &[("col", col as i64 + 1), ("row", row as i64 + 1)])?),
            None => V::Nil,
        })
    })?)?;
    let meta = lua.create_table()?;
    meta.raw_set("__index", methods)?;
    let s = shared.clone();
    g.raw_set("grid", lua.create_function(move |lua, (cols, rows): (V, V)| {
        let (cols, rows) = (number(&cols)?.clamp(1, 64) as usize, number(&rows)?.clamp(1, 64) as usize);
        let mut shared = s.borrow_mut();
        let (width, height) = (shared.context.width, shared.context.height);
        let grid = Grid::new(width, height, cols, rows);
        shared.grid = Some(grid);
        let table = rect_table(lua, grid.board())?;
        for (key, value) in [("cols", cols as i64), ("rows", rows as i64), ("size", grid.cell.into()), ("area_w", width.into()), ("area_h", height.into())] {
            table.raw_set(key, value)?;
        }
        table.set_metatable(Some(meta.clone()))?;
        Ok(table)
    })?)?;
    let s = shared.clone();
    g.raw_set("side", lua.create_function(move |lua, ()| {
        let shared = s.borrow();
        let (width, height) = (shared.context.width, shared.context.height);
        let side = shared.grid.map_or_else(|| Grid::new(width, height, 1, 1).side(), |grid| grid.side());
        rect_table(lua, side)
    })?)?;
    // Lua's own `math.random`, from the host's numbers, so that a seeded simulator repeats.
    let math: Table = g.raw_get("math")?;
    let s = shared.clone();
    math.raw_set("random", lua.create_function(move |_, args: Variadic<V>| {
        let x = s.borrow_mut().next();
        let range = |low: i64, high: i64| -> mlua::Result<V> {
            if low > high {
                return Err(mlua::Error::runtime("bad argument to 'random' (interval is empty)"));
            }
            let span = (high - low) as u64 + 1;
            Ok(V::Integer(low + (u64::from(x) % span) as i64))
        };
        match args.len() {
            0 => Ok(V::Number(f64::from(x) / 4_294_967_296.0)),
            1 => range(1, number(&args[0])?.into()),
            _ => range(number(&args[0])?.into(), number(&args[1])?.into()),
        }
    })?)?;
    math.raw_set("randomseed", V::Nil)?;
    let s = shared.clone();
    g.raw_set("timer", lua.create_function(move |_, ms: V| {
        s.borrow_mut().output.timer = Some(number(&ms)?.max(0) as u64);
        Ok(())
    })?)?;
    let s = shared.clone();
    g.raw_set("lang", lua.create_function(move |_, ()| Ok(s.borrow().context.lang.clone()))?)?;
    let s = shared.clone();
    g.raw_set("now", lua.create_function(move |_, ()| Ok(s.borrow().context.now))?)?;
    let s = shared.clone();
    g.raw_set("save", lua.create_function(move |_, value: V| {
        let value = Value::from_lua(&value).map_err(mlua::Error::runtime)?;
        let json = crate::json::write(&value);
        if json.len() > crate::json::MAX_DATA {
            return Err(mlua::Error::runtime(format!("the data to save is over {} KB", crate::json::MAX_DATA / 1024)));
        }
        let mut shared = s.borrow_mut();
        shared.context.data = Some(value);
        shared.output.save = Some(json);
        Ok(())
    })?)?;
    let s = shared.clone();
    g.raw_set("load", lua.create_function(move |lua, ()| {
        let data = s.borrow().context.data.clone();
        data.map_or(Ok(V::Nil), |data| data.to_lua(lua))
    })?)?;
    let s = shared.clone();
    g.raw_set("exit", lua.create_function(move |_, ()| {
        s.borrow_mut().output.exit = true;
        Ok(())
    })?)?;
    let s = shared.clone();
    g.raw_set("insert", lua.create_function(move |_, latex: V| {
        let mut shared = s.borrow_mut();
        shared.output.insert = Some(text_of(&latex));
        shared.output.exit = true;
        Ok(())
    })?)?;
    Ok(())
}

/// What waits for the hui: questions, the calculator, forms.
fn asking(lua: &Lua, shared: &Rc<RefCell<Shared>>, commands: &Rc<Receiver<Command>>, replies: &Sender<Reply>) -> mlua::Result<()> {
    let g = lua.globals();
    let _ = shared;
    // A question with Ja and Nein.
    let (c, r) = (commands.clone(), replies.clone());
    g.raw_set("confirm", lua.create_function(move |_, question: V| {
        Ok(matches!(ask(&c, &r, Request::Confirm(text_of(&question)))?, Value::Bool(true)))
    })?)?;
    // A line of text over the app: `ask(label)` and `ask(label, {max = 10, value = "…"})`.
    let (c, r) = (commands.clone(), replies.clone());
    g.raw_set("ask", lua.create_function(move |lua, (label, options): (V, Option<Table>)| {
        let (mut max, mut value) = (32, String::new());
        if let Some(options) = options {
            let given: V = options.raw_get("max")?;
            if !given.is_nil() {
                max = number(&given)?.clamp(1, 256) as usize;
            }
            value = text_of(&options.raw_get::<V>("value")?);
        }
        ask(&c, &r, Request::Ask { label: text_of(&label), max, value })?.to_lua(lua)
    })?)?;
    // The session's variables.
    let (c, r) = (commands.clone(), replies.clone());
    g.raw_set("variable", lua.create_function(move |lua, name: V| ask(&c, &r, Request::Var(text_of(&name)))?.to_lua(lua))?)?;
    // The calculator: the script waits for the engine.
    let (c, r) = (commands.clone(), replies.clone());
    g.raw_set("calc", lua.create_function(move |lua, input: V| ask(&c, &r, Request::Calc(text_of(&input)))?.to_lua(lua))?)?;
    // A form: the script waits for it to close, and works it out with `submit` meanwhile.
    let (c, r) = (commands.clone(), replies.clone());
    g.raw_set("form", lua.create_function(move |lua, description: Table| {
        let submit: Option<Function> = match description.raw_get::<V>("submit")? {
            V::Function(submit) => Some(submit),
            _ => None,
        };
        let described = Value::from_lua(&V::Table(description)).map_err(mlua::Error::runtime)?;
        let mut answer = ask(&c, &r, Request::Form(described))?;
        loop {
            let values = match (&submit, answer.get("submit")) {
                (Some(_), Some(values)) => values.to_lua(lua)?,
                _ => return answer.to_lua(lua),
            };
            let submit = submit.as_ref().expect("a submit function");
            let given: V = submit.call(values)?;
            let given = Value::from_lua(&given).map_err(mlua::Error::runtime)?;
            answer = ask(&c, &r, Request::Submitted(given))?;
        }
    })?)?;
    Ok(())
}

/// Hands `request` to the main side and waits for its answer.
fn ask(commands: &Receiver<Command>, replies: &Sender<Reply>, request: Request) -> mlua::Result<Value> {
    if replies.send(Reply::Yield(Yield::Asks(request))).is_err() {
        return Err(mlua::Error::runtime("the app was left"));
    }
    match commands.recv() {
        Ok(Command::Answer(value)) => Ok(value),
        Ok(Command::Call(..)) => Err(mlua::Error::runtime("a callback before the answer")),
        Err(_) => Err(mlua::Error::runtime("the app was left")),
    }
}
