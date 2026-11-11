//! The script API (apps.md §4): the functions a script can call, registered on its engine, and
//! the limits that keep a script from harming the handheld (§5).

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender};

use embedded_graphics::pixelcolor::Rgb565;
use quantus_ui::applet::{self, Canvas, Grid, Input, Palette, MAX_SHAPES};
use rhai::{Array, Dynamic, Engine, EvalAltResult, Map, INT};

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
    /// The random numbers' state, seeded for each callback.
    rng: u32,
    /// The last board `grid` laid out, for `side`.
    grid: Option<Grid>,
}

impl Shared {
    /// A number from 0 to `n` − 1 (xorshift).
    fn random(&mut self, n: INT) -> INT {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        if n <= 0 { 0 } else { INT::from(x) % n }
    }

    fn draw(&mut self, draw: Draw) -> Fallible<()> {
        if self.output.draws.len() >= MAX_SHAPES {
            return Err(format!("more than {MAX_SHAPES} shapes in one frame").into());
        }
        self.output.draws.push(draw);
        Ok(())
    }
}

impl Shared {
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

/// A whole number from a script's number, whole or not.
pub(crate) fn number(value: &Dynamic) -> Fallible<i32> {
    if let Ok(i) = value.as_int() {
        return Ok(i.clamp(i32::MIN as i64, i32::MAX as i64) as i32);
    }
    if let Ok(f) = value.as_float() {
        if f.is_finite() {
            return Ok(f.round().clamp(i32::MIN as f64, i32::MAX as f64) as i32);
        }
    }
    Err(format!("a number was expected, not {}", value.type_name()).into())
}

/// A colour by its name in the palette, or as `rgb` gives it: `#rrggbb`.
pub(crate) fn color(value: &Dynamic) -> Fallible<Rgb565> {
    let name = value.clone().into_string().map_err(|_| format!("a colour was expected, not {}", value.type_name()))?;
    if let Some(color) = Palette::get(&name) {
        return Ok(color);
    }
    let hex = name.strip_prefix('#').filter(|hex| hex.len() == 6).and_then(|hex| u32::from_str_radix(hex, 16).ok());
    match hex {
        Some(rgb) => Ok(Palette::rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)),
        None => Err(format!("no colour called {name:?}").into()),
    }
}

type Fallible<T> = Result<T, Box<EvalAltResult>>;

/// The engine of one script: its limits and the API, over the state it shares with the host
/// functions and the channels a host call that needs the hui waits on.
pub(crate) fn engine(shared: Rc<RefCell<Shared>>, commands: Rc<Receiver<Command>>, replies: Sender<Reply>) -> Engine {
    let mut engine = Engine::new();
    engine.set_max_operations(BUDGET);
    engine.set_max_call_levels(CALL_LEVELS);
    engine.set_max_expr_depths(64, 32);
    engine.set_max_string_size(MAX_STRING);
    engine.set_max_array_size(MAX_ARRAY);
    engine.set_max_map_size(MAX_MAP);
    engine.on_print(|_| {});
    engine.on_debug(|_, _, _| {});

    drawing(&mut engine, &shared);
    templates(&mut engine, &shared);
    helpers(&mut engine, &shared);

    asking(&mut engine, &commands, &replies);

    // The calculator: the script waits for the engine.
    let (c, r) = (commands.clone(), replies.clone());
    engine.register_fn("calc", move |input: Dynamic| -> Fallible<Dynamic> {
        Ok(ask(&c, &r, Request::Calc(input.to_string()))?.to_dynamic())
    });

    // A question with Ja and Nein, answered by the hui: the script waits for it.
    engine.register_fn("confirm", move |question: &str| -> Fallible<bool> {
        match ask(&commands, &replies, Request::Confirm(question.to_string()))? {
            Value::Bool(yes) => Ok(yes),
            _ => Ok(false),
        }
    });
    engine
}

/// The screen and drawing (apps.md §4.3).
fn drawing(engine: &mut Engine, shared: &Rc<RefCell<Shared>>) {
    let s = shared.clone();
    engine.register_fn("width", move || -> i64 { s.borrow().context.width.into() });
    let s = shared.clone();
    engine.register_fn("height", move || -> i64 { s.borrow().context.height.into() });
    let s = shared.clone();
    engine.register_fn("clear", move |c: Dynamic| -> Fallible<()> { s.borrow_mut().draw(Draw::Clear(color(&c)?)) });
    // Rhai's own `clear` of a string would take `clear("background")`: this one, with the exact
    // type, comes first.
    let s = shared.clone();
    engine.register_fn("clear", move |c: rhai::ImmutableString| -> Fallible<()> {
        s.borrow_mut().draw(Draw::Clear(color(&c.into())?))
    });
    let s = shared.clone();
    engine.register_fn("rect", move |x: Dynamic, y: Dynamic, w: Dynamic, h: Dynamic, c: Dynamic| -> Fallible<()> {
        s.borrow_mut().draw(Draw::Rect(number(&x)?, number(&y)?, number(&w)?, number(&h)?, color(&c)?))
    });
    let s = shared.clone();
    engine.register_fn("round", move |x: Dynamic, y: Dynamic, w: Dynamic, h: Dynamic, r: Dynamic, c: Dynamic| -> Fallible<()> {
        s.borrow_mut().draw(Draw::Round(number(&x)?, number(&y)?, number(&w)?, number(&h)?, number(&r)?, color(&c)?))
    });
    let s = shared.clone();
    engine.register_fn("frame", move |x: Dynamic, y: Dynamic, w: Dynamic, h: Dynamic, r: Dynamic, c: Dynamic| -> Fallible<()> {
        s.borrow_mut().draw(Draw::Frame(number(&x)?, number(&y)?, number(&w)?, number(&h)?, number(&r)?, color(&c)?))
    });
    let s = shared.clone();
    engine.register_fn("circle", move |x: Dynamic, y: Dynamic, d: Dynamic, c: Dynamic| -> Fallible<()> {
        s.borrow_mut().draw(Draw::Circle(number(&x)?, number(&y)?, number(&d)?, color(&c)?))
    });
    let s = shared.clone();
    engine.register_fn("line", move |x1: Dynamic, y1: Dynamic, x2: Dynamic, y2: Dynamic, c: Dynamic, w: Dynamic| -> Fallible<()> {
        s.borrow_mut().draw(Draw::Line(number(&x1)?, number(&y1)?, number(&x2)?, number(&y2)?, color(&c)?, number(&w)?))
    });
    for (name, small) in [("text", false), ("text_small", true)] {
        let s = shared.clone();
        engine.register_fn(name, move |x: Dynamic, y: Dynamic, text: Dynamic, c: Dynamic| -> Fallible<()> {
            s.borrow_mut().draw(Draw::Text(number(&x)?, number(&y)?, text.to_string(), color(&c)?, small))
        });
    }
    let s = shared.clone();
    engine.register_fn("math", move |x: Dynamic, y: Dynamic, latex: &str, c: Dynamic| -> Fallible<()> {
        s.borrow_mut().draw(Draw::Math(number(&x)?, number(&y)?, latex.to_string(), color(&c)?))
    });
    engine.register_fn("text_width", |text: Dynamic| -> i64 { applet::text_width(&text.to_string()).into() });
    engine.register_fn("text_small_width", |text: Dynamic| -> i64 { applet::text_small_width(&text.to_string()).into() });
    engine.register_fn("text_height", || -> i64 { applet::text_height().into() });
    engine.register_fn("text_small_height", || -> i64 { applet::text_small_height().into() });
    engine.register_fn("rgb", |r: i64, g: i64, b: i64| -> String {
        let channel = |v: i64| v.clamp(0, 255);
        format!("#{:02x}{:02x}{:02x}", channel(r), channel(g), channel(b))
    });
}

/// The soft keys and the input templates (apps.md §4.3, §4.4).
fn templates(engine: &mut Engine, shared: &Rc<RefCell<Shared>>) {
    let s = shared.clone();
    engine.register_fn("soft", move |labels: Array| {
        let labels = labels.iter().map(|label| if label.is_unit() { String::new() } else { label.to_string() }).collect();
        s.borrow_mut().output.soft = Some(labels);
    });
    let s = shared.clone();
    engine.register_fn("input", move |kind: &str| -> Fallible<()> {
        s.borrow_mut().output.input = Some(input(kind, &Map::new())?);
        Ok(())
    });
    let s = shared.clone();
    engine.register_fn("input", move |kind: &str, options: Map| -> Fallible<()> {
        s.borrow_mut().output.input = Some(input(kind, &options)?);
        Ok(())
    });
    let s = shared.clone();
    engine.register_fn("cursor", move || -> Map {
        let (col, row) = s.borrow().context.cursor;
        cell_map(&[("col", col as INT), ("row", row as INT)])
    });
    let s = shared.clone();
    engine.register_fn("cursor", move |col: INT, row: INT| {
        let cursor = (col.max(0) as usize, row.max(0) as usize);
        let mut shared = s.borrow_mut();
        shared.context.cursor = cursor;
        shared.output.cursor = Some(cursor);
    });
}

/// The template `kind` with its `options`.
fn input(kind: &str, options: &Map) -> Fallible<Input> {
    let int = |name: &str, default: usize| options.get(name).and_then(|v| v.as_int().ok()).map_or(default, |v| v.clamp(1, 64) as usize);
    let flag = |name: &str| options.get(name).and_then(|v| v.as_bool().ok()).unwrap_or(false);
    let text = |name: &str| options.get(name).map(|v| v.to_string()).unwrap_or_default();
    Ok(match kind {
        "keys" => Input::Keys,
        "grid" => Input::Grid { cols: int("cols", 3), rows: int("rows", 3), digits: flag("digits"), wrap: flag("wrap") },
        "cells" => Input::Cells { cols: int("cols", 3), rows: int("rows", 3), digits_only: flag("digits_only") },
        "list" => {
            let items = options.get("items").and_then(|items| items.clone().into_array().ok()).unwrap_or_default();
            Input::List { items: items.iter().map(|item| item.to_string()).collect() }
        }
        "line" => Input::Line { label: text("label"), calc: flag("calc") },
        "text" => Input::Text {
            label: text("label"),
            max: options.get("max").and_then(|v| v.as_int().ok()).map_or(32, |v| v.clamp(1, 256) as usize),
            value: text("value"),
        },
        kind => return Err(format!("no input called {kind:?}").into()),
    })
}

fn cell_map(entries: &[(&str, INT)]) -> Map {
    entries.iter().map(|(key, value)| ((*key).into(), Dynamic::from(*value))).collect()
}

fn rect_map(rect: embedded_graphics::primitives::Rectangle) -> Map {
    cell_map(&[
        ("x", rect.top_left.x.into()),
        ("y", rect.top_left.y.into()),
        ("w", rect.size.width.into()),
        ("h", rect.size.height.into()),
    ])
}

/// The board a map from `grid` stands for.
fn grid_of(map: &Map) -> Fallible<Grid> {
    let get = |name: &str| map.get(name).and_then(|v| v.as_int().ok()).ok_or_else(|| format!("not a grid: no {name}"));
    let (width, height, cols, rows) = (get("area_w")?, get("area_h")?, get("cols")?, get("rows")?);
    Ok(Grid::new(width as i32, height as i32, cols as usize, rows as usize))
}

/// The games' helpers (apps.md §4.5).
fn helpers(engine: &mut Engine, shared: &Rc<RefCell<Shared>>) {
    let s = shared.clone();
    engine.register_fn("grid", move |cols: INT, rows: INT| -> Map {
        let mut shared = s.borrow_mut();
        let (width, height) = (shared.context.width, shared.context.height);
        let grid = Grid::new(width, height, cols.clamp(1, 64) as usize, rows.clamp(1, 64) as usize);
        shared.grid = Some(grid);
        let board = grid.board();
        let mut map = rect_map(board);
        map.extend(cell_map(&[
            ("cols", grid.cols as INT),
            ("rows", grid.rows as INT),
            ("cell", grid.cell.into()),
            ("area_w", width.into()),
            ("area_h", height.into()),
        ]));
        map
    });
    engine.register_fn("cell", |grid: &mut Map, col: INT, row: INT| -> Fallible<Map> {
        Ok(rect_map(grid_of(grid)?.cell(col.max(0) as usize, row.max(0) as usize)))
    });
    engine.register_fn("at", |grid: &mut Map, x: INT, y: INT| -> Fallible<Dynamic> {
        Ok(match grid_of(grid)?.at(embedded_graphics::prelude::Point::new(x as i32, y as i32)) {
            Some((col, row)) => cell_map(&[("col", col as INT), ("row", row as INT)]).into(),
            None => Dynamic::UNIT,
        })
    });
    let s = shared.clone();
    engine.register_fn("side", move || -> Map {
        let shared = s.borrow();
        let (width, height) = (shared.context.width, shared.context.height);
        let side = shared.grid.map_or_else(|| Grid::new(width, height, 1, 1).side(), |grid| grid.side());
        rect_map(side)
    });
    let s = shared.clone();
    engine.register_fn("random", move |n: INT| -> INT { s.borrow_mut().random(n) });
    let s = shared.clone();
    engine.register_fn("timer", move |ms: INT| {
        s.borrow_mut().output.timer = Some(ms.max(0) as u64);
    });
    let s = shared.clone();
    engine.register_fn("lang", move || -> String { s.borrow().context.lang.clone() });
    let s = shared.clone();
    engine.register_fn("now", move || -> Dynamic { s.borrow().context.now.map_or(Dynamic::UNIT, Dynamic::from) });
    let s = shared.clone();
    engine.register_fn("save", move |value: Dynamic| -> Fallible<()> {
        let value = Value::from_dynamic(&value);
        let json = crate::json::write(&value);
        if json.len() > crate::json::MAX_DATA {
            return Err(format!("the data to save is over {} KB", crate::json::MAX_DATA / 1024).into());
        }
        let mut shared = s.borrow_mut();
        shared.context.data = Some(value);
        shared.output.save = Some(json);
        Ok(())
    });
    let s = shared.clone();
    engine.register_fn("load", move || -> Dynamic { s.borrow().context.data.as_ref().map_or(Dynamic::UNIT, Value::to_dynamic) });
    let s = shared.clone();
    engine.register_fn("exit", move || {
        s.borrow_mut().output.exit = true;
    });
}

/// `ask(label)` and `ask(label, options)`: a line of text over the app.
fn asking(engine: &mut Engine, commands: &Rc<Receiver<Command>>, replies: &Sender<Reply>) {
    let (c, r) = (commands.clone(), replies.clone());
    engine.register_fn("ask", move |label: &str| -> Fallible<Dynamic> {
        let request = Request::Ask { label: label.to_string(), max: 32, value: String::new() };
        Ok(ask(&c, &r, request)?.to_dynamic())
    });
    let (c, r) = (commands.clone(), replies.clone());
    engine.register_fn("ask", move |label: &str, options: Map| -> Fallible<Dynamic> {
        let max = options.get("max").and_then(|v| v.as_int().ok()).map_or(32, |v| v.clamp(1, 256) as usize);
        let value = options.get("value").map(|v| v.to_string()).unwrap_or_default();
        Ok(ask(&c, &r, Request::Ask { label: label.to_string(), max, value })?.to_dynamic())
    });
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
