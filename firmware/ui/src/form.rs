//! Forms (firmware/docs/apps.md §4.7): a tool that only asks for values and shows results is a
//! description, drawn by the hui in its own look. The description is plain data with nothing in
//! it that only the hui knows, so the core could read the same one later.

/// A form: its title, its fields, and what is worked out of them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Form {
    pub title: String,
    pub fields: Vec<Field>,
    pub results: Vec<Outcome>,
    /// The results come from the app's own function instead of `results`.
    pub submit: bool,
}

/// A value asked for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Field {
    /// The name formulas use for it.
    pub id: String,
    pub label: String,
    /// The unit a bare number is taken in.
    pub unit: Option<String>,
    /// What stands in it at first, as typed.
    pub value: Option<String>,
    /// A line below it.
    pub hint: Option<String>,
    /// A list to pick from instead of an input.
    pub choices: Vec<String>,
}

/// A result: its label, the formula over the fields' ids, the unit it is shown in.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub label: String,
    pub formula: String,
    pub unit: Option<String>,
}

use std::collections::VecDeque;

use crate::applet::{AppKey, Applet, Canvas, Host, Palette, Response};
use crate::font::Scale;
use crate::key::Key;
use crate::latex;
use crate::sheet::{self, Flow, Sheet};

/// The height of a field's line, and of the line under it for a hint or an error.
const FIELD: i32 = 44;
const NOTE: i32 = 20;
/// The width of the labels, and the room kept for a unit.
const LABEL: i32 = 150;
const UNIT: i32 = 56;

/// A form on the screen: its fields as the sheets have them, one per line with its label and
/// unit, the editor in the field under the cursor; a field with choices picks from a list.
pub struct FormApplet {
    pub form: Form,
    /// The fields' inputs, one column.
    pub sheet: Sheet,
    /// The choice picked in each field with choices.
    pub chosen: Vec<usize>,
    /// The list of a field's choices while it is open, and the one under the cursor.
    pub choosing: Option<usize>,
    /// What went wrong with each field when it was worked out.
    pub errors: Vec<Option<String>>,
    /// The results, once worked out: the label, the LaTeX or the error.
    pub results: Option<Vec<(String, Result<String, String>)>>,
    /// An error of the form as a whole, under the results.
    pub error: Option<String>,
    /// The result under the cursor, while the cursor is below the fields.
    pub result: Option<usize>,
    /// What is still to be worked out, and the step the engine works on now.
    jobs: VecDeque<Job>,
    waiting: Option<Job>,
    /// Each field as it goes into the formulas: its value, with its unit if it had none.
    terms: Vec<String>,
    /// The form was asked to be worked out since this was last cleared: for the app around it,
    /// to keep the inputs and, with `submit`, to work it out itself.
    pub asked: bool,
    /// OK was pressed on a result: leaving hands it to the calculation.
    handing: bool,
}

/// A step of working a form out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Job {
    /// What a field's value is: a number alone, or one with a unit.
    Value(usize),
    /// Whether a field's value with a unit is of the field's unit's quantity.
    Unit(usize),
    /// A result's formula.
    Result(usize),
}

/// `formula` with every name of a field replaced by its term: whole names only, so `R1` stays
/// apart from `R12`, and a LaTeX command (`\sqrt`) is left alone.
pub fn substitute(formula: &str, terms: &[(String, String)]) -> String {
    let mut out = String::new();
    let mut chars = formula.chars().peekable();
    let mut command = false;
    while let Some(c) = chars.next() {
        if c.is_ascii_alphabetic() {
            let mut name = c.to_string();
            while let Some(&next) = chars.peek().filter(|next| next.is_ascii_alphanumeric() || **next == '_') {
                name.push(next);
                chars.next();
            }
            match terms.iter().find(|(id, _)| *id == name).filter(|_| !command) {
                Some((_, term)) => out.push_str(&format!("({term})")),
                None => out.push_str(&name),
            }
            command = false;
        } else {
            command = c == '\\';
            out.push(c);
        }
    }
    out
}

/// A unit written upright, as the engine reads it.
fn unit(unit: &str) -> String {
    format!(r"\mathrm{{{unit}}}")
}

/// A value as a form's description writes it (`10 kΩ`), as the calculator's LaTeX: a number
/// and a unit after a space become the number with the unit set upright.
pub fn value_latex(value: &str) -> String {
    let value = value.trim();
    match value.split_once(' ') {
        Some((number, unit)) if !unit.is_empty() && !value.contains('\\') && number.parse::<f64>().is_ok() => {
            format!(r"{number}\,\mathrm{{{}}}", unit.trim())
        }
        _ => value.to_string(),
    }
}

impl FormApplet {
    pub fn new(form: Form) -> FormApplet {
        let count = form.fields.len().max(1);
        let mut sheet = Sheet::new(count, 1, Flow::Down);
        for (index, field) in form.fields.iter().enumerate() {
            if let (Some(value), true) = (&field.value, field.choices.is_empty()) {
                sheet.set(index, 0, latex::parse(&value_latex(value)));
            }
        }
        sheet.select(0, 0);
        let chosen = form
            .fields
            .iter()
            .map(|field| field.value.as_ref().and_then(|value| field.choices.iter().position(|choice| choice == value)).unwrap_or(0))
            .collect();
        let errors = vec![None; form.fields.len()];
        let terms = vec![String::new(); form.fields.len()];
        FormApplet {
            form,
            sheet,
            chosen,
            choosing: None,
            errors,
            results: None,
            error: None,
            result: None,
            jobs: VecDeque::new(),
            waiting: None,
            terms,
            asked: false,
            handing: false,
        }
    }

    /// The field under the cursor.
    pub fn field(&self) -> usize {
        self.sheet.cursor.0
    }

    /// The fields' inputs by their ids, as LaTeX; a choice is its text.
    pub fn inputs(&self) -> Vec<(String, String)> {
        self.form
            .fields
            .iter()
            .enumerate()
            .map(|(index, field)| {
                let value = match field.choices.get(self.chosen[index]) {
                    Some(choice) => choice.clone(),
                    None => self.sheet.latex(index, 0),
                };
                (field.id.clone(), value)
            })
            .collect()
    }

    /// Sets the inputs kept from the last time, by the fields' ids.
    pub fn restore(&mut self, inputs: &[(String, String)]) {
        for (index, field) in self.form.fields.iter().enumerate() {
            let Some((_, value)) = inputs.iter().find(|(id, _)| *id == field.id) else { continue };
            match field.choices.iter().position(|choice| choice == value) {
                Some(choice) => self.chosen[index] = choice,
                None if field.choices.is_empty() => self.sheet.set(index, 0, latex::parse(value)),
                None => {}
            }
        }
        self.sheet.select(0, 0);
    }

    /// Takes a key as the sheets do; EXE on the last field asks for the form to be worked out,
    /// which [`FormApplet::calculate`] starts.
    fn sheet_key(&mut self, key: Key, shift: bool, host: &mut dyn Host) {
        let field = self.field();
        let choices = self.form.fields.get(field).map_or(0, |field| field.choices.len());
        if choices > 0 && self.sheet.editing.is_none() {
            match key {
                Key::Ok => return self.choosing = Some(self.chosen[field]),
                Key::Left | Key::Right => {
                    let step = if key == Key::Left { choices - 1 } else { 1 };
                    self.chosen[field] = (self.chosen[field] + step) % choices;
                    return self.changed();
                }
                _ => {}
            }
        }
        let editing = self.sheet.editing.is_some();
        if self.sheet.key(key, shift) == sheet::Outcome::Calculate {
            return self.calculate(host);
        }
        if editing || self.sheet.editing.is_some() || key == Key::Del {
            self.changed();
        }
        // ↓ from the last field goes to the results.
        let last = field + 1 == self.form.fields.len();
        let results = self.results.as_ref().is_some_and(|results| !results.is_empty());
        if key == Key::Down && !editing && self.sheet.cursor.0 == field && last && results {
            self.result = Some(0);
        }
    }

    /// An input changed: the results are of the inputs before.
    fn changed(&mut self) {
        let field = self.field();
        if let Some(error) = self.errors.get_mut(field) {
            *error = None;
        }
    }

    /// Works the form out: each field's value, then each result; the results come into
    /// [`FormApplet::results`], what is wrong with a field to it.
    pub fn calculate(&mut self, host: &mut dyn Host) {
        self.sheet.commit();
        self.asked = true;
        if self.form.submit {
            // The app's own function works it out (`quantus-apps`).
            return;
        }
        self.errors.iter_mut().for_each(|error| *error = None);
        (self.results, self.error, self.result) = (None, None, None);
        let missing = crate::text::VALUE_MISSING.get(host.lang()).to_string();
        for (index, field) in self.form.fields.iter().enumerate() {
            if let Some(choice) = field.choices.get(self.chosen[index]) {
                self.terms[index] = choice.clone();
            } else if self.sheet.cell(index, 0).is_empty() {
                self.errors[index] = Some(missing.clone());
            } else {
                self.jobs.push_back(Job::Value(index));
            }
        }
        self.work(host);
    }

    /// The engine's input for `job`.
    fn input(&self, job: Job) -> String {
        match job {
            Job::Value(index) => self.sheet.latex(index, 0),
            Job::Unit(index) => {
                let field = &self.form.fields[index];
                format!(r"({})\blacktriangleright {}", self.sheet.latex(index, 0), unit(field.unit.as_deref().unwrap_or_default()))
            }
            Job::Result(index) => {
                let outcome = &self.form.results[index];
                let terms: Vec<(String, String)> = self.form.fields.iter().map(|field| field.id.clone()).zip(self.terms.iter().cloned()).collect();
                let formula = substitute(&outcome.formula, &terms);
                match &outcome.unit {
                    Some(to) => format!(r"({formula})\blacktriangleright {}", unit(to)),
                    None => formula,
                }
            }
        }
    }

    /// Gives the engine the next steps, as long as it answers at once.
    fn work(&mut self, host: &mut dyn Host) {
        while self.waiting.is_none() {
            let Some(job) = self.jobs.pop_front() else {
                if self.done() {
                    continue;
                }
                return;
            };
            match host.calc(&self.input(job)) {
                Some(answer) => self.take(job, answer),
                None => self.waiting = Some(job),
            }
        }
    }

    /// What the engine made of `job`.
    fn take(&mut self, job: Job, answer: Result<String, String>) {
        match (job, answer) {
            (Job::Value(index) | Job::Unit(index), Err(error)) => self.errors[index] = Some(error),
            (Job::Value(index), Ok(value)) => {
                let typed = self.sheet.latex(index, 0);
                match &self.form.fields[index].unit {
                    // A value with a unit of its own is taken as typed, once its quantity is the
                    // field's.
                    Some(_) if value.contains(r"\mathrm{") => {
                        self.terms[index] = typed;
                        self.jobs.push_front(Job::Unit(index));
                    }
                    // A number alone is in the field's unit.
                    Some(to) => self.terms[index] = format!(r"({typed})\,{}", unit(to)),
                    None => self.terms[index] = typed,
                }
            }
            (Job::Unit(_), Ok(_)) => {}
            (Job::Result(index), answer) => {
                if let Some(results) = &mut self.results {
                    results[index].1 = answer;
                }
            }
        }
    }

    /// Whether a step waits for the engine.
    pub fn busy(&self) -> bool {
        self.waiting.is_some()
    }

    /// The results the app's `submit` gave: the cursor goes to the first.
    pub fn submitted(&mut self, results: Vec<(String, Result<String, String>)>) {
        self.errors.iter_mut().for_each(|error| *error = None);
        self.error = None;
        self.result = (!results.is_empty()).then_some(0);
        self.results = Some(results);
    }

    /// Every field is read: the results next, unless a field is wrong; or all is done. Whether
    /// there is more to work out.
    fn done(&mut self) -> bool {
        if self.results.is_some() {
            // The answer below, the cursor on its first result.
            self.result = self.results.as_ref().filter(|results| !results.is_empty()).map(|_| 0);
            return false;
        }
        if self.errors.iter().any(Option::is_some) {
            return false;
        }
        self.results = Some(self.form.results.iter().map(|outcome| (outcome.label.clone(), Err(String::new()))).collect());
        self.jobs.extend((0..self.form.results.len()).map(Job::Result));
        true
    }

    /// The lines of the form from the top: (field or result, top, height).
    fn lines(&self) -> Vec<(Line, i32, i32)> {
        let mut lines = Vec::new();
        let mut y = if self.form.title.is_empty() { 4 } else { 30 };
        for (index, field) in self.form.fields.iter().enumerate() {
            let note = field.hint.is_some() || self.errors[index].is_some();
            let height = FIELD + if note { NOTE } else { 0 };
            lines.push((Line::Field(index), y, height));
            y += height + 4;
        }
        if let Some(results) = &self.results {
            y += 6;
            for index in 0..results.len() {
                lines.push((Line::Result(index), y, FIELD));
                y += FIELD + 2;
            }
        }
        if self.error.is_some() {
            lines.push((Line::Error, y, NOTE * 2));
        }
        lines
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Line {
    Field(usize),
    Result(usize),
    Error,
}

fn color(name: &str) -> embedded_graphics::pixelcolor::Rgb565 {
    Palette::get(name).unwrap_or_default()
}

impl Applet for FormApplet {
    fn key(&mut self, _key: AppKey, _host: &mut dyn Host) {}

    fn types(&self) -> bool {
        true
    }

    fn typed(&mut self, key: Key, shift: bool, host: &mut dyn Host) -> bool {
        // The list of a field's choices.
        if let Some(selected) = self.choosing {
            let field = self.field();
            let count = self.form.fields[field].choices.len();
            match key {
                Key::Up => self.choosing = Some((selected + count - 1) % count),
                Key::Down => self.choosing = Some((selected + 1) % count),
                Key::Digit(digit @ 1..=9) if usize::from(digit) <= count => {
                    self.chosen[field] = usize::from(digit) - 1;
                    self.choosing = None;
                }
                Key::Ok | Key::Exe => {
                    self.chosen[field] = selected;
                    self.choosing = None;
                }
                Key::Left | Key::Del => self.choosing = None,
                _ => {}
            }
            return true;
        }
        // On the results: ↑/↓ go through them, OK hands one to the calculation.
        if let Some(result) = self.result {
            let count = self.results.as_ref().map_or(0, Vec::len);
            match key {
                Key::Up if result == 0 => self.result = None,
                Key::Up => self.result = Some(result - 1),
                Key::Down => self.result = Some((result + 1).min(count.saturating_sub(1))),
                Key::Ok | Key::Exe => {
                    if let Some((_, Ok(_))) = self.results.as_ref().and_then(|results| results.get(result)) {
                        self.handing = true;
                        host.exit();
                    }
                }
                _ => {}
            }
            return true;
        }
        self.sheet_key(key, shift, host);
        true
    }

    fn insert(&mut self, latex: &str, _host: &mut dyn Host) {
        if self.choosing.is_some() || self.result.is_some() {
            return;
        }
        self.sheet.edit();
        if let Some(editor) = self.sheet.editing.as_mut() {
            crate::app::paste_latex(editor, &latex::parse(latex));
        }
        self.changed();
    }

    fn draw(&self, canvas: &mut Canvas) {
        canvas.clear(color("background"));
        let (width, height) = (canvas.width(), canvas.height());
        if !self.form.title.is_empty() {
            canvas.text_small(10, 6, &self.form.title, color("muted"));
        }
        // Scrolled so the line under the cursor shows.
        let lines = self.lines();
        let current = match self.result {
            Some(result) => Line::Result(result),
            None => Line::Field(self.field()),
        };
        let bottom = lines.iter().find(|(line, ..)| *line == current).map_or(0, |(_, top, h)| top + h);
        let scroll = (bottom - height + 4).max(0);
        let normal = Scale::Normal.metrics();
        for (line, top, line_height) in lines {
            let y = top - scroll;
            if y + line_height < 0 || y > height {
                continue;
            }
            let middle = y + FIELD / 2;
            let baseline = middle + normal.digit / 2;
            match line {
                Line::Field(index) => {
                    let field = &self.form.fields[index];
                    let here = current == line && self.choosing.is_none();
                    canvas.text_small(10, middle - crate::applet::text_small_height() / 2, &field.label, color("text"));
                    let (x, w) = (LABEL, width - LABEL - UNIT - 10);
                    canvas.round(x, y + 4, w, FIELD - 8, 6, color(if here { "surface" } else { "hairline" }));
                    if here {
                        canvas.frame(x, y + 4, w, FIELD - 8, 6, color("cyan"));
                    }
                    match (field.choices.get(self.chosen[index]), here.then_some(self.sheet.editing.as_ref()).flatten()) {
                        // A choice: ←/→ or OK change it.
                        (Some(choice), _) => canvas.text(x + 10, middle - crate::applet::text_height() / 2, choice, color("text")),
                        (None, Some(editor)) => canvas.editor(x + 8, baseline, editor, true, color("text")),
                        (None, None) => canvas.row(x + 8, baseline, self.sheet.cell(index, 0), Scale::Normal, color("text")),
                    }
                    if let Some(unit) = &field.unit {
                        canvas.text_small(width - UNIT, middle - crate::applet::text_small_height() / 2, unit, color("muted"));
                    }
                    let note = self.errors[index].as_ref().map(|error| (error, "error")).or(field.hint.as_ref().map(|hint| (hint, "muted")));
                    if let Some((note, tone)) = note {
                        canvas.text_small(LABEL, y + FIELD, note, color(tone));
                    }
                }
                Line::Result(index) => {
                    let Some((label, value)) = self.results.as_ref().and_then(|results| results.get(index)) else { continue };
                    if current == line {
                        canvas.round(4, y, width - 8, FIELD, 6, color("surface"));
                        canvas.frame(4, y, width - 8, FIELD, 6, color("cyan"));
                    }
                    canvas.text_small(10, middle - crate::applet::text_small_height() / 2, &format!("{label} ="), color("text"));
                    match value {
                        Ok(value) => canvas.row(LABEL, baseline, &latex::parse(value), Scale::Normal, color("result")),
                        Err(error) => canvas.text_small(LABEL, middle - crate::applet::text_small_height() / 2, error, color("error")),
                    }
                }
                Line::Error => {
                    if let Some(error) = &self.error {
                        canvas.text_small(10, y, error, color("error"));
                    }
                }
            }
        }
        // A field's choices, over the form.
        if let Some(selected) = self.choosing {
            let choices = &self.form.fields[self.field()].choices;
            let rows = choices.len().min(6) as i32;
            let (w, h) = (width - 80, rows * 36 + 12);
            let (x, top) = (40, (height - h) / 2);
            canvas.round(x, top, w, h, 8, color("surface"));
            canvas.frame(x, top, w, h, 8, color("hairline"));
            let first = selected.saturating_sub(5);
            for (offset, choice) in choices.iter().enumerate().skip(first).take(6) {
                let y = top + 6 + (offset - first) as i32 * 36;
                if offset == selected {
                    canvas.round(x + 6, y, w - 12, 34, 6, color("hairline"));
                }
                canvas.text(x + 16, y + 4, &format!("{} {choice}", offset + 1), color("text"));
            }
        }
    }

    fn answer(&mut self, response: Response, host: &mut dyn Host) {
        let (Response::Calc(answer), Some(job)) = (response, self.waiting.take()) else { return };
        self.take(job, answer);
        self.work(host);
    }

    fn leave(&mut self, _host: &mut dyn Host) -> Option<String> {
        // The result under the cursor, handed to the calculation by OK; AC hands nothing.
        if !std::mem::take(&mut self.handing) {
            return None;
        }
        let results = self.results.as_ref()?;
        match results.get(self.result?) {
            Some((_, Ok(value))) => Some(value.clone()),
            _ => None,
        }
    }
}
