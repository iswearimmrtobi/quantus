//! A script as the hui's [`Applet`]: keys, ticks and taps become callbacks on the script's
//! thread, what `draw` drew is replayed into the canvas, a question the script asks goes to the
//! host, and an error turns the app into a page that says what went wrong.

use embedded_graphics::prelude::*;
use quantus_ui::applet::{AppKey, Applet, Canvas, Event, Host, Input, Palette, Response};
use quantus_ui::font::Scale;
use quantus_ui::form::FormApplet;
use quantus_ui::key::Key;
use quantus_ui::text::{self, Lang};

use crate::api::Draw;
use crate::script::{Callback, Context, Error, Output, Request, Script, Value, Yield};

/// The fastest the timer ticks.
pub const MIN_TIMER: u64 = 100;

/// A running script.
pub struct ScriptApplet {
    source: String,
    script: Option<Script>,
    /// What the last `draw` drew.
    draws: Vec<Draw>,
    /// The soft keys' labels as the script set them.
    soft: Vec<String>,
    input: Input,
    /// Every how many ms `tick` is called, and when it was called last.
    timer: Option<u64>,
    last_tick: u64,
    /// When the app opened, by the hui's clock; set at the first tick.
    opened: Option<u64>,
    /// The request the script waits on.
    asking: Option<Request>,
    /// What went wrong; the app shows it and does nothing else.
    error: Option<Error>,
    lang: Lang,
    /// The saved data the script last saved or was opened with.
    data: Option<Value>,
    /// LaTeX for the calculation, handed back when the app is left.
    insert: Option<String>,
    /// A line worked out for `entered`, waiting for the engine (`calc: true`).
    entering: bool,
    /// The form the script waits on in `form(...)`.
    form: Option<FormApplet>,
    /// The script has no `draw`: it is only a form, whose inputs are its saved data.
    form_only: bool,
}

impl ScriptApplet {
    /// The script of `source`; it compiles now and starts when the hui opens it.
    pub fn new(source: &str) -> ScriptApplet {
        let mut applet = ScriptApplet {
            source: source.to_string(),
            script: None,
            draws: Vec::new(),
            soft: Vec::new(),
            input: Input::Keys,
            timer: None,
            last_tick: 0,
            opened: None,
            asking: None,
            error: None,
            lang: Lang::De,
            data: None,
            insert: None,
            entering: false,
            form: None,
            form_only: false,
        };
        match Script::start(source) {
            Ok(script) => {
                applet.form_only = !script.draws;
                applet.script = Some(script);
            }
            Err(error) => applet.error = Some(error),
        }
        applet
    }

    /// What went wrong, if the app stopped.
    pub fn error(&self) -> Option<&Error> {
        self.error.as_ref()
    }

    /// What the last `draw` drew.
    pub fn draws(&self) -> &[Draw] {
        &self.draws
    }

    fn soft_shown(&self) -> bool {
        self.soft.iter().any(|label| !label.trim().is_empty())
    }

    /// What the area depends on: the soft keys and an input line at its bottom.
    fn layout(&self) -> (bool, i32) {
        (self.soft_shown(), self.input.line_height())
    }

    fn context(&self, host: &mut dyn Host) -> Context {
        let area = host.area(self.soft_shown(), &self.input);
        Context {
            width: area.width as i32,
            height: area.height as i32,
            cursor: host.cursor(),
            lang: host.lang().code().to_string(),
            now: host.now(),
            seed: host.random(),
            data: self.data.clone(),
        }
    }

    /// Runs `callback` unless the app stopped or waits on the hui.
    fn call(&mut self, callback: Callback, host: &mut dyn Host) {
        self.lang = host.lang();
        if self.error.is_some() || self.asking.is_some() {
            return;
        }
        let context = self.context(host);
        let Some(script) = &mut self.script else { return };
        let reply = script.send(callback, context);
        self.handle(reply, host);
    }

    /// Takes in where the script got to.
    fn handle(&mut self, reply: Yield, host: &mut dyn Host) {
        match reply {
            Yield::Done(output) => {
                let layout = self.layout();
                if let Some((col, row)) = output.cursor {
                    host.set_cursor(col, row);
                }
                if output.exit {
                    host.exit();
                }
                if output.insert.is_some() {
                    self.insert = output.insert.clone();
                }
                if let Some(json) = &output.save {
                    host.save(json);
                    self.data = crate::json::read(json);
                }
                self.apply(output);
                // The strips or the line came or went: the area changed, so it is drawn again.
                if self.layout() != layout {
                    self.call(Callback::Draw, host);
                }
            }
            Yield::Asks(request) => {
                match &request {
                    Request::Confirm(question) => host.confirm(question),
                    Request::Ask { label, max, value } => host.ask(label, *max, value),
                    Request::Var(name) => {
                        let value = host.variable(name).map_or(Value::Unit, Value::Str);
                        let Some(script) = &mut self.script else { return };
                        let reply = script.answer(value);
                        return self.handle(reply, host);
                    }
                    Request::Form(description) => match crate::form::read(description) {
                        Ok(form) => {
                            let mut form = FormApplet::new(form);
                            // A tool that is only a form: its inputs as they were left.
                            if let (true, Some(Value::Map(entries))) = (self.form_only, self.data.as_ref().and_then(|data| data.get("inputs"))) {
                                let inputs: Vec<(String, String)> = entries
                                    .iter()
                                    .filter_map(|(id, value)| match value {
                                        Value::Str(latex) => Some((id.clone(), latex.clone())),
                                        _ => None,
                                    })
                                    .collect();
                                form.restore(&inputs);
                            }
                            self.form = Some(form);
                        }
                        Err(message) => return self.fail(Error::new(message)),
                    },
                    Request::Submitted(given) => {
                        self.submitted(given);
                        // The script waits in `form` again.
                        self.asking = Some(Request::Form(Value::Unit));
                        return;
                    }
                    Request::Calc(input) => {
                        if let Some(answer) = host.calc(input) {
                            // Answered at once: the script goes on.
                            let Some(script) = &mut self.script else { return };
                            let reply = script.answer(calc_answer(answer));
                            return self.handle(reply, host);
                        }
                    }
                }
                self.asking = Some(request);
            }
            Yield::Failed(error) => self.fail(error),
        }
    }

    /// What `submit` gave, into the form: a list of `#{label, latex}`, an error text for the
    /// form, or `#{field, error}` for a field.
    fn submitted(&mut self, given: &Value) {
        let Some(form) = &mut self.form else { return };
        match given {
            Value::Array(results) => {
                let text = |value: Option<&Value>| match value {
                    Some(Value::Str(s)) => s.clone(),
                    Some(Value::Int(i)) => i.to_string(),
                    Some(Value::Float(f)) => f.to_string(),
                    _ => String::new(),
                };
                let results = results
                    .iter()
                    .map(|result| (text(result.get("label")), Ok(text(result.get("latex").or(result.get("value"))))))
                    .collect();
                form.submitted(results);
            }
            Value::Map(_) => {
                let field = given.get("field").and_then(|field| match field {
                    Value::Str(id) => form.form.fields.iter().position(|f| f.id == *id),
                    _ => None,
                });
                let message = match given.get("error") {
                    Some(Value::Str(message)) => message.clone(),
                    _ => String::new(),
                };
                match field {
                    Some(index) => form.errors[index] = Some(message),
                    None => form.error = Some(message),
                }
            }
            Value::Str(message) => form.error = Some(message.clone()),
            _ => {}
        }
    }

    /// After a key on the form: its inputs kept if it was worked out, and with `submit` the
    /// script asked.
    fn after_form(&mut self, host: &mut dyn Host) {
        let Some(form) = &mut self.form else { return };
        if !std::mem::take(&mut form.asked) {
            return;
        }
        let inputs = Value::Map(form.inputs().into_iter().map(|(id, latex)| (id, Value::Str(latex))).collect());
        if self.form_only {
            let data = Value::Map(vec![("inputs".into(), inputs.clone())]);
            host.save(&crate::json::write(&data));
            self.data = Some(data);
        }
        if form.form.submit {
            let Some(script) = &mut self.script else { return };
            self.asking = None;
            let reply = script.answer(Value::Map(vec![("submit".into(), inputs)]));
            self.handle(reply, host);
        }
    }

    /// The results of the form as a map, label to LaTeX, and the form closed; the script goes on.
    fn close_form(&mut self, host: &mut dyn Host) {
        let Some(form) = self.form.take() else { return };
        let results = form
            .results
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(label, value)| value.ok().map(|latex| (label, Value::Str(latex))))
            .collect();
        self.asking = None;
        let Some(script) = &mut self.script else { return };
        let reply = script.answer(Value::Map(results));
        self.handle(reply, host);
    }

    fn apply(&mut self, output: Output) {
        if output.drawn {
            self.draws = output.draws;
        }
        if let Some(soft) = output.soft {
            self.soft = soft;
        }
        if let Some(input) = output.input {
            self.input = input;
        }
        if let Some(every) = output.timer {
            self.timer = (every > 0).then(|| every.max(MIN_TIMER));
        }
    }

    fn fail(&mut self, error: Error) {
        self.error = Some(error);
        self.soft.clear();
        self.input = Input::Keys;
        self.timer = None;
        // The thread is let go; it ends by itself.
        self.script = None;
    }

    /// The page an error turns the app into: what happened and where.
    fn error_page(&self, error: &Error, canvas: &mut Canvas) {
        let lang = self.lang;
        canvas.clear(Palette::get("background").unwrap_or_default());
        let room = canvas.width() - 24;
        let title = if error.hung { text::NOT_RESPONDING } else { text::APP_ERROR };
        let error_color = Palette::get("error").unwrap_or_default();
        let mut y = 16;
        canvas.text(12, y, title.get(lang), error_color);
        y += quantus_ui::applet::text_height() + 8;
        if !error.hung {
            for line in wrap(&error.message, room) {
                canvas.text_small(12, y, &line, Palette::get("text").unwrap_or_default());
                y += quantus_ui::applet::text_small_height() + 2;
            }
        }
        if let Some(line) = error.line {
            y += 6;
            canvas.text_small(12, y, &format!("{} {line}", text::LINE.get(lang)), Palette::get("muted").unwrap_or_default());
        }
    }
}

/// The engine's answer as `calc` gives it: `#{ok, value, latex, error}`. `latex` is the result
/// as the calculator shows it, `value` what `calc` takes again (without `≈`).
pub fn calc_answer(answer: Result<String, String>) -> Value {
    let (ok, value, latex, error) = match answer {
        Ok(latex) => {
            let value = latex.trim_start_matches(r"\approx").trim().to_string();
            (true, Value::Str(value), Value::Str(latex), Value::Unit)
        }
        Err(error) => (false, Value::Unit, Value::Unit, Value::Str(error)),
    };
    Value::Map(vec![("ok".into(), Value::Bool(ok)), ("value".into(), value), ("latex".into(), latex), ("error".into(), error)])
}

/// `text` broken into lines of at most `room` pixels in the small font.
fn wrap(text: &str, room: i32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let next = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
        if Scale::Small.width(&next) > room && !line.is_empty() {
            lines.push(std::mem::replace(&mut line, word.to_string()));
        } else {
            line = next;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines.truncate(6);
    lines
}

impl Applet for ScriptApplet {
    fn start(&mut self, host: &mut dyn Host) {
        self.data = host.load().and_then(|json| crate::json::read(&json));
        self.call(Callback::Init, host);
    }

    fn types(&self) -> bool {
        self.form.is_some() && self.error.is_none()
    }

    fn typed(&mut self, key: Key, shift: bool, host: &mut dyn Host) -> bool {
        let Some(form) = &mut self.form else { return false };
        let took = form.typed(key, shift, host);
        self.after_form(host);
        took
    }

    fn insert(&mut self, latex: &str, host: &mut dyn Host) {
        if let Some(form) = &mut self.form {
            form.insert(latex, host);
        }
    }

    fn key(&mut self, key: AppKey, host: &mut dyn Host) {
        // *Fertig* closes a form of a script that draws itself.
        if self.form.is_some() {
            if key == AppKey::Soft(0) && !self.form_only {
                self.close_form(host);
            }
            return;
        }
        if self.error.is_some() {
            // OK or EXE leave the error page, as AC does.
            if matches!(key, AppKey::Ok | AppKey::Exe) {
                host.exit();
            }
            return;
        }
        self.call(Callback::Key(key.name()), host);
    }

    fn tick(&mut self, now_ms: u64, host: &mut dyn Host) -> bool {
        let opened = *self.opened.get_or_insert(now_ms);
        let Some(every) = self.timer else { return false };
        if now_ms.saturating_sub(self.last_tick) < every.max(MIN_TIMER) {
            return false;
        }
        self.last_tick = now_ms;
        self.call(Callback::Tick((now_ms - opened) as i64), host);
        true
    }

    fn draw(&self, canvas: &mut Canvas) {
        if let Some(error) = &self.error {
            return self.error_page(error, canvas);
        }
        if let Some(form) = &self.form {
            return form.draw(canvas);
        }
        for draw in &self.draws {
            draw.replay(canvas);
        }
    }

    fn tap(&mut self, point: Point, host: &mut dyn Host) {
        self.call(Callback::Tap(point.x.into(), point.y.into()), host);
    }

    fn soft_keys(&self) -> Vec<String> {
        match (&self.form, self.form_only) {
            (Some(_), false) => vec![if self.lang == Lang::De { "Fertig" } else { "Done" }.to_string()],
            (Some(_), true) => Vec::new(),
            (None, _) => self.soft.clone(),
        }
    }

    fn input(&self) -> Input {
        if self.form.is_some() {
            return Input::Keys;
        }
        self.input.clone()
    }

    fn event(&mut self, event: Event, host: &mut dyn Host) {
        let callback = match event {
            Event::Pick(col, row) => Callback::Pick(col as i64, row as i64),
            Event::Chosen(index) => Callback::Entered(Value::Int(index as i64)),
            // The input line with `calc: true` gives what `calc` gives.
            Event::Entered(latex) if matches!(self.input, Input::Line { calc: true, .. }) && self.error.is_none() => {
                match host.calc(&latex) {
                    Some(answer) => Callback::Entered(calc_answer(answer)),
                    None => {
                        self.asking = Some(Request::Calc(latex));
                        self.entering = true;
                        return;
                    }
                }
            }
            Event::Entered(text) => Callback::Entered(Value::Str(text)),
            Event::Cell(col, row, value) => Callback::Entered(Value::Map(vec![
                ("col".into(), Value::Int(col as i64)),
                ("row".into(), Value::Int(row as i64)),
                ("value".into(), Value::Str(value)),
            ])),
        };
        self.call(callback, host);
    }

    fn answer(&mut self, response: Response, host: &mut dyn Host) {
        // The engine's answer to a step of the form.
        if let (Some(form), Response::Calc(_)) = (&mut self.form, &response) {
            if form.busy() {
                return form.answer(response, host);
            }
        }
        let Some(_) = self.asking.take() else { return };
        if std::mem::take(&mut self.entering) {
            if let Response::Calc(answer) = response {
                self.call(Callback::Entered(calc_answer(answer)), host);
            }
            return;
        }
        let value = match response {
            Response::Confirmed(yes) => Value::Bool(yes),
            Response::Text(Some(text)) => Value::Str(text),
            Response::Text(None) => Value::Unit,
            Response::Calc(answer) => calc_answer(answer),
        };
        let Some(script) = &mut self.script else { return };
        let reply = script.answer(value);
        self.handle(reply, host);
    }

    fn restart(&mut self, host: &mut dyn Host) {
        let source = std::mem::take(&mut self.source);
        *self = ScriptApplet::new(&source);
        self.start(host);
    }

    fn leave(&mut self, host: &mut dyn Host) -> Option<String> {
        if let Some(form) = &mut self.form {
            return form.leave(host).or(self.insert.take());
        }
        if self.error.is_none() && self.asking.is_none() {
            self.call(Callback::Leave, host);
        }
        self.insert.take()
    }
}
