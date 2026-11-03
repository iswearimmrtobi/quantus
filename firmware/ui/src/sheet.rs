//! Sheets: grids of cells, each a math input, moved through and edited as on the Casio's
//! editors. LGS shows the matrix `[A | b]`, Wertetabelle and Verteilungen a form of labelled
//! fields, Statistik the data in columns (design/handheld-menus.md §7.3). EXE works the sheet out
//! through [`Backend::solve`](crate::backend::Backend::solve).
//!
//! Keys: the arrows move from cell to cell; an input key starts the cell afresh, OK edits it as
//! it is; while a cell is edited the keys go to its editor, and EXE, OK, ↑/↓ (and ←/→ at its
//! ends) keep it and move on, AC drops the edit. DEL empties a cell, EXE on a cell not being
//! edited works the sheet out.

use crate::backend::{Field, Request};
use crate::editor::{Editor, Row};
use crate::key::Key;
use crate::latex;
use crate::settings::{DistributionKind, Settings, StatisticsType};

/// Which way EXE moves after a cell is typed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
    /// Along the row, then on to the next one: the coefficients of an equation.
    Across,
    /// Down the column: a list, a form.
    Down,
}

/// What a key on a sheet asks the app for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Stay,
    /// Work the sheet out.
    Calculate,
}

/// The rows a sheet shows at a time.
pub const ROWS_SHOWN: usize = 5;
/// The width the cells share, in pixels, and that of the labels of a form.
pub const GRID_WIDTH: i32 = 460;
pub const LABEL_WIDTH: i32 = 110;
/// The narrowest a cell gets before the sheet scrolls sideways.
const NARROWEST: i32 = 88;

/// The width of a cell and how many columns show, for a sheet of `columns`, with labels or not.
pub fn columns_shown(columns: usize, labels: bool) -> (i32, usize) {
    let room = GRID_WIDTH - if labels { LABEL_WIDTH } else { 0 };
    let shown = columns.clamp(1, (room / NARROWEST) as usize);
    (room / shown as i32, shown)
}

#[derive(Clone, Debug)]
pub struct Sheet {
    /// By row, then by column; never empty.
    cells: Vec<Vec<Row>>,
    /// Row and column.
    pub cursor: (usize, usize),
    /// The cell being edited, at the cursor.
    pub editing: Option<Editor>,
    pub flow: Flow,
    /// Whether an empty row always waits at the end, to type the next value into: a list.
    pub grows: bool,
    /// The first row and column on the screen.
    pub top: usize,
    pub left: usize,
}

impl Sheet {
    pub fn new(rows: usize, columns: usize, flow: Flow) -> Self {
        let mut sheet =
            Sheet { cells: Vec::new(), cursor: (0, 0), editing: None, flow, grows: false, top: 0, left: 0 };
        sheet.resize(rows, columns);
        sheet
    }

    /// A list: an empty row always at the end.
    pub fn list(columns: usize) -> Self {
        Sheet { grows: true, ..Sheet::new(1, columns, Flow::Down) }
    }

    pub fn rows(&self) -> usize {
        self.cells.len()
    }

    pub fn columns(&self) -> usize {
        self.cells[0].len()
    }

    pub fn cell(&self, row: usize, column: usize) -> &Row {
        &self.cells[row][column]
    }

    /// Sets a cell, adding rows to reach it.
    pub fn set(&mut self, row: usize, column: usize, value: Row) {
        if row >= self.rows() {
            let columns = self.columns();
            self.cells.resize_with(row + 1, || vec![Row::default(); columns]);
        }
        if let Some(cell) = self.cells[row].get_mut(column) {
            *cell = value;
        }
        self.tidy();
    }

    pub fn latex(&self, row: usize, column: usize) -> String {
        self.cells[row][column].to_latex()
    }

    /// The cells of `column`, top to bottom.
    pub fn column(&self, column: usize) -> Vec<Row> {
        self.cells.iter().map(|row| row[column].clone()).collect()
    }

    pub fn row_is_empty(&self, row: usize) -> bool {
        self.cells[row].iter().all(Row::is_empty)
    }

    /// The rows with something in them; a list's empty rows are skipped.
    pub fn filled_rows(&self) -> Vec<usize> {
        (0..self.rows()).filter(|&row| !self.grows || !self.row_is_empty(row)).collect()
    }

    /// Makes it `rows` × `columns` (at least 1 × 1), keeping what is in the cells that stay.
    pub fn resize(&mut self, rows: usize, columns: usize) {
        let (rows, columns) = (rows.max(1), columns.max(1));
        if self.rows() == rows && self.cells.first().is_some_and(|row| row.len() == columns) {
            return;
        }
        self.commit();
        self.cells.resize_with(rows, Vec::new);
        self.cells.iter_mut().for_each(|row| row.resize_with(columns, Row::default));
        self.cursor = (self.cursor.0.min(rows - 1), self.cursor.1.min(columns - 1));
        self.tidy();
    }

    /// Every cell emptied, the cursor back at the top left.
    pub fn clear(&mut self) {
        let rows = if self.grows { 1 } else { self.rows() };
        self.cells = vec![vec![Row::default(); self.columns()]; rows];
        (self.cursor, self.editing, self.top, self.left) = ((0, 0), None, 0, 0);
    }

    /// An empty row above the cursor's (a list only).
    pub fn insert_row(&mut self) {
        if !self.grows {
            return;
        }
        self.commit();
        let columns = self.columns();
        self.cells.insert(self.cursor.0, vec![Row::default(); columns]);
    }

    /// The cursor's row taken out (a list only).
    pub fn delete_row(&mut self) {
        if !self.grows {
            return;
        }
        self.editing = None;
        if self.rows() > 1 {
            self.cells.remove(self.cursor.0);
        }
        self.tidy();
    }

    /// A list ends in exactly one empty row; the cursor stays on the sheet.
    fn tidy(&mut self) {
        if self.grows {
            while self.rows() > 1 && self.row_is_empty(self.rows() - 1) && self.row_is_empty(self.rows() - 2) {
                self.cells.pop();
            }
            if !self.row_is_empty(self.rows() - 1) {
                let columns = self.columns();
                self.cells.push(vec![Row::default(); columns]);
            }
        }
        self.cursor = (self.cursor.0.min(self.rows() - 1), self.cursor.1.min(self.columns() - 1));
    }

    /// Keeps the cell being edited.
    pub fn commit(&mut self) {
        if let Some(editor) = self.editing.take() {
            let (row, column) = self.cursor;
            self.cells[row][column] = editor.row;
            self.tidy();
        }
    }

    /// Puts the cursor on a cell, keeping the one being edited.
    pub fn select(&mut self, row: usize, column: usize) {
        self.commit();
        self.cursor = (row.min(self.rows() - 1), column.min(self.columns() - 1));
    }

    /// Starts editing the cell at the cursor as it is.
    pub fn edit(&mut self) {
        if self.editing.is_none() {
            let mut editor = Editor::new();
            editor.set(self.cell(self.cursor.0, self.cursor.1).clone());
            self.editing = Some(editor);
        }
    }

    /// Moves the cursor by rows and columns, staying on the sheet.
    fn step(&mut self, rows: isize, columns: isize) {
        let move_by = |at: usize, by: isize, len: usize| at.saturating_add_signed(by).min(len - 1);
        self.cursor = (move_by(self.cursor.0, rows, self.rows()), move_by(self.cursor.1, columns, self.columns()));
    }

    /// The next cell after one is typed: along the row or down the column, wrapping.
    fn advance(&mut self) {
        let (row, column) = self.cursor;
        let (rows, columns) = (self.rows(), self.columns());
        self.cursor = match self.flow {
            Flow::Across if column + 1 < columns => (row, column + 1),
            Flow::Across if row + 1 < rows => (row + 1, 0),
            Flow::Down if row + 1 < rows => (row + 1, column),
            Flow::Down if !self.grows && column + 1 < columns => (0, column + 1),
            _ => (row, column),
        };
    }

    pub fn key(&mut self, key: Key, shift: bool) -> Outcome {
        if let Some(editor) = self.editing.as_mut() {
            // Where the cursor goes once the cell is kept, if it leaves the cell.
            let leave = match key {
                Key::Ac => {
                    self.editing = None;
                    return Outcome::Stay;
                }
                Key::Exe | Key::Ok => None,
                Key::Up => (!editor.up()).then_some((-1, 0)),
                Key::Down => (!editor.down()).then_some((1, 0)),
                Key::Left => (!editor.left()).then_some((0, -1)),
                Key::Right => (!editor.right()).then_some((0, 1)),
                Key::Del => {
                    editor.delete();
                    return Outcome::Stay;
                }
                _ => {
                    editor.key(key, shift);
                    return Outcome::Stay;
                }
            };
            match (key, leave) {
                (Key::Exe | Key::Ok, _) => {
                    self.commit();
                    self.advance();
                }
                (_, Some((rows, columns))) => {
                    self.commit();
                    self.step(rows, columns);
                }
                _ => {}
            }
            return Outcome::Stay;
        }
        match key {
            Key::Up => self.step(-1, 0),
            Key::Down => self.step(1, 0),
            Key::Left => self.step(0, -1),
            Key::Right => self.step(0, 1),
            Key::Del => {
                let (row, column) = self.cursor;
                self.cells[row][column] = Row::default();
                self.tidy();
            }
            Key::Ok => self.edit(),
            Key::Exe => return Outcome::Calculate,
            _ => {
                let mut editor = Editor::new();
                if editor.key(key, shift) {
                    self.editing = Some(editor);
                }
            }
        }
        Outcome::Stay
    }

    /// Scrolls so the cursor is among the `rows` and `columns` on the screen.
    pub fn follow(&mut self, rows: usize, columns: usize) {
        let keep = |first: &mut usize, at: usize, shown: usize| {
            if at < *first {
                *first = at;
            } else if at >= *first + shown {
                *first = at + 1 - shown;
            }
        };
        keep(&mut self.top, self.cursor.0, rows.max(1));
        keep(&mut self.left, self.cursor.1, columns.max(1));
        self.top = self.top.min(self.rows().saturating_sub(rows.max(1)));
        self.left = self.left.min(self.columns().saturating_sub(columns.max(1)));
    }
}

/// The modes that show a sheet instead of the input line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Linear,
    Table,
    Statistics,
    Distribution,
}

impl Kind {
    /// Whether the sheet is a form: one column with a label beside each row.
    pub fn form(self) -> bool {
        matches!(self, Kind::Table | Kind::Distribution)
    }
}

/// The fields of the Wertetabelle form, top to bottom.
pub const TABLE_FIELDS: usize = 5;

/// What a list in Statistik holds in its columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Column {
    X,
    Y,
    Freq,
}

/// The columns of the Statistik list for `settings`.
pub fn statistics_columns(settings: &Settings) -> Vec<Column> {
    let mut columns = vec![Column::X];
    if settings.statistics_type != StatisticsType::OneVariable {
        columns.push(Column::Y);
    }
    if settings.statistics_freq {
        columns.push(Column::Freq);
    }
    columns
}

/// The sheets of the four modes, kept while another mode is shown.
#[derive(Clone, Debug)]
pub struct Sheets {
    pub linear: Sheet,
    pub table: Sheet,
    pub statistics: Sheet,
    pub distribution: Sheet,
    /// The Statistik columns as laid out now, and the cells of x, y and Freq kept while one
    /// of them is not shown.
    columns: Vec<Column>,
    kept: [Vec<Row>; 3],
    /// What Statistik works out with pairs.
    regression: StatisticsType,
    /// The distribution the form shows, and every field's cell, kept while another
    /// distribution is shown.
    kind: DistributionKind,
    fields: [Row; 9],
}

impl Sheets {
    pub fn new(settings: &Settings) -> Self {
        let columns = statistics_columns(settings);
        let mut sheets = Sheets {
            linear: Sheet::new(1, 1, Flow::Across),
            table: Sheet::new(TABLE_FIELDS, 1, Flow::Down),
            statistics: Sheet::list(columns.len()),
            distribution: Sheet::new(Field::of(settings.distribution).len(), 1, Flow::Down),
            columns,
            kept: Default::default(),
            regression: settings.statistics_type,
            kind: settings.distribution,
            fields: Default::default(),
        };
        sheets.fit(settings);
        sheets
    }

    pub fn get(&self, kind: Kind) -> &Sheet {
        match kind {
            Kind::Linear => &self.linear,
            Kind::Table => &self.table,
            Kind::Statistics => &self.statistics,
            Kind::Distribution => &self.distribution,
        }
    }

    pub fn get_mut(&mut self, kind: Kind) -> &mut Sheet {
        match kind {
            Kind::Linear => &mut self.linear,
            Kind::Table => &mut self.table,
            Kind::Statistics => &mut self.statistics,
            Kind::Distribution => &mut self.distribution,
        }
    }

    /// The Statistik columns as laid out now.
    pub fn statistics_columns(&self) -> &[Column] {
        &self.columns
    }

    /// The fields the distribution form shows now.
    pub fn distribution_fields(&self) -> &'static [Field] {
        Field::of(self.kind)
    }

    /// Lays the sheets out as the settings say: the size of the LGS, the Statistik columns,
    /// the fields of the distribution. What is typed stays.
    pub fn fit(&mut self, settings: &Settings) {
        let rows = usize::from(settings.system_rows.clamp(1, 10));
        let columns = usize::from(settings.system_columns.clamp(1, 10)) + 1;
        self.linear.resize(rows, columns);

        self.regression = settings.statistics_type;
        let wanted = statistics_columns(settings);
        if wanted != self.columns {
            let sheet = &mut self.statistics;
            sheet.commit();
            for (index, &column) in self.columns.iter().enumerate() {
                self.kept[column as usize] = sheet.column(index);
            }
            let at = self.columns.get(sheet.cursor.1).copied();
            let mut fresh = Sheet::list(wanted.len());
            for (index, &column) in wanted.iter().enumerate() {
                for (row, cell) in self.kept[column as usize].iter().enumerate() {
                    fresh.set(row, index, cell.clone());
                }
            }
            let column = at.and_then(|at| wanted.iter().position(|&c| c == at)).unwrap_or(0);
            fresh.select(sheet.cursor.0, column);
            *sheet = fresh;
            self.columns = wanted;
        }

        if settings.distribution != self.kind {
            let sheet = &mut self.distribution;
            sheet.commit();
            for (row, &field) in Field::of(self.kind).iter().enumerate() {
                self.fields[field as usize] = sheet.cell(row, 0).clone();
            }
            let fields = Field::of(settings.distribution);
            let mut fresh = Sheet::new(fields.len(), 1, Flow::Down);
            for (row, &field) in fields.iter().enumerate() {
                fresh.set(row, 0, self.fields[field as usize].clone());
            }
            *sheet = fresh;
            self.kind = settings.distribution;
        }
    }

    /// The request for the sheet of `kind`, from its cells.
    pub fn request(&self, kind: Kind) -> Request {
        let sheet = self.get(kind);
        let cell = |row: usize| sheet.latex(row, 0);
        match kind {
            Kind::Linear => Request::Linear {
                rows: (0..sheet.rows()).map(|row| (0..sheet.columns()).map(|column| sheet.latex(row, column)).collect()).collect(),
            },
            Kind::Table => {
                let g = cell(1);
                Request::Table { f: cell(0), g: (!g.is_empty()).then_some(g), from: cell(2), to: cell(3), step: cell(4) }
            }
            Kind::Statistics => {
                let rows = sheet.filled_rows();
                let column = |which: Column| {
                    let index = self.columns.iter().position(|&c| c == which)?;
                    Some(rows.iter().map(|&row| sheet.latex(row, index)).collect::<Vec<_>>())
                };
                let kind = match self.columns.contains(&Column::Y) {
                    true => self.regression,
                    false => StatisticsType::OneVariable,
                };
                Request::Statistics { kind, x: column(Column::X).unwrap_or_default(), y: column(Column::Y), freq: column(Column::Freq) }
            }
            Kind::Distribution => {
                let mut fields = Vec::new();
                let mut k = Vec::new();
                for (row, &field) in Field::of(self.kind).iter().enumerate() {
                    let latex = cell(row);
                    match field {
                        Field::K => k = latex.split(';').map(str::trim).filter(|v| !v.is_empty()).map(String::from).collect(),
                        _ if latex.is_empty() => {}
                        _ => fields.push((field, latex)),
                    }
                }
                Request::Distribution { kind: self.kind, fields, k }
            }
        }
    }

    /// Fills a sheet with what a session kept. The app has set the settings to the request's
    /// layout first (the size of the LGS, the type of the data, the distribution).
    pub fn fill(&mut self, request: &Request, settings: &Settings) {
        self.fit(settings);
        let parse = |latex: &str| latex::parse(latex);
        match request {
            Request::Linear { rows } => {
                let sheet = &mut self.linear;
                sheet.clear();
                for (r, row) in rows.iter().enumerate().take(sheet.rows()) {
                    for (c, cell) in row.iter().enumerate().take(sheet.columns()) {
                        sheet.set(r, c, parse(cell));
                    }
                }
            }
            Request::Table { f, g, from, to, step } => {
                let sheet = &mut self.table;
                let g = g.clone().unwrap_or_default();
                for (row, latex) in [f, &g, from, to, step].into_iter().enumerate() {
                    sheet.set(row, 0, parse(latex));
                }
            }
            Request::Statistics { x, y, freq, .. } => {
                let mut fresh = Sheet::list(self.columns.len());
                for (index, &column) in self.columns.iter().enumerate() {
                    let cells = match column {
                        Column::X => Some(x),
                        Column::Y => y.as_ref(),
                        Column::Freq => freq.as_ref(),
                    };
                    for (row, latex) in cells.into_iter().flatten().enumerate() {
                        fresh.set(row, index, parse(latex));
                    }
                }
                self.statistics = fresh;
            }
            Request::Distribution { fields, k, .. } => {
                let sheet = &mut self.distribution;
                sheet.clear();
                for (row, &field) in Field::of(self.kind).iter().enumerate() {
                    let latex = match field {
                        Field::K => k.join(";"),
                        _ => fields.iter().find(|(f, _)| *f == field).map(|(_, latex)| latex.clone()).unwrap_or_default(),
                    };
                    sheet.set(row, 0, parse(&latex));
                }
            }
        }
    }
}
