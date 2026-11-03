//! A sheet on the screen: its cells in rows of [`ROW`] pixels under the column heads (or beside
//! the labels of a form), the cell at the cursor in full size along the bottom; and the answer
//! EXE brings, laid over it.

use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Rectangle, Triangle};

use super::{colors, fit, panel, rounded, Shape, BAR, HEIGHT, MARGIN, WIDTH};
use crate::app::{Answer, App};
use crate::backend::{Line, Matrix, Reply};
use crate::editor::Row;
use crate::font::Scale;
use crate::latex;
use crate::layout::{self, RowLayout};
use crate::settings::UnknownNames;
use crate::sheet::{self, Column, Kind, Sheet, LABEL_WIDTH, ROWS_SHOWN};
use crate::text::{self, t, Lang};

/// The height of a row of cells: a small fraction fits.
const ROW: i32 = 40;
/// Where the column heads stand, and the first row of cells under them (a form has no heads).
const HEADS: i32 = BAR + 20;
const GRID_TOP: i32 = BAR + 26;
const FORM_TOP: i32 = BAR + 8;
/// The line between the cells and the cell at the cursor shown in full.
const EDIT_TOP: i32 = GRID_TOP + ROWS_SHOWN as i32 * ROW + 6;

/// The letters LGS names its unknowns with when *Unbekannte* says letters, as the engine does.
const LETTERS: [&str; 24] =
    ["x", "y", "z", "a", "b", "c", "d", "f", "g", "h", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s", "t", "u", "v", "w"];

/// The top of the first row of cells.
fn top(kind: Kind) -> i32 {
    if kind.form() {
        FORM_TOP
    } else {
        GRID_TOP
    }
}

/// The left of the first column of cells, and their width.
fn columns(kind: Kind, sheet: &Sheet) -> (i32, i32) {
    let (width, _) = sheet::columns_shown(sheet.columns(), kind.form());
    (MARGIN + if kind.form() { LABEL_WIDTH } else { 0 }, width)
}

/// The area of the cell `row`, `column` of the sheet, if it is on the screen.
pub fn cell_rect(kind: Kind, sheet: &Sheet, row: usize, column: usize) -> Option<Rectangle> {
    let (_, shown) = sheet::columns_shown(sheet.columns(), kind.form());
    let (row, column) = (row.checked_sub(sheet.top)?, column.checked_sub(sheet.left)?);
    if row >= ROWS_SHOWN || column >= shown {
        return None;
    }
    let (left, width) = columns(kind, sheet);
    let at = Point::new(left + column as i32 * width, top(kind) + row as i32 * ROW);
    Some(Rectangle::new(at, Size::new(width as u32 - 4, ROW as u32 - 4)))
}

/// The cell under `point`, counted over the whole sheet.
pub fn cell_at(kind: Kind, sheet: &Sheet, point: Point) -> Option<(usize, usize)> {
    let rows = sheet.top..(sheet.top + ROWS_SHOWN).min(sheet.rows());
    let columns = sheet.left..sheet.columns();
    rows.flat_map(|row| columns.clone().map(move |column| (row, column)))
        .find(|&(row, column)| cell_rect(kind, sheet, row, column).is_some_and(|area| grown(area).contains(point)))
}

/// A cell with the gaps around it, for a finger.
fn grown(area: Rectangle) -> Rectangle {
    Rectangle::new(area.top_left - Point::new(2, 2), area.size + Size::new(4, 4))
}

/// The heads of the columns, as math.
fn heads<B>(app: &App<B>, kind: Kind, sheet: &Sheet) -> Vec<Row> {
    let lang = app.settings.language;
    match kind {
        Kind::Linear => {
            let unknowns = sheet.columns() - 1;
            let mut heads: Vec<Row> = (0..unknowns)
                .map(|index| match app.settings.unknown_names {
                    UnknownNames::Letters if unknowns <= LETTERS.len() => latex::parse(LETTERS[index]),
                    _ => latex::parse(&format!("x_{{{}}}", index + 1)),
                })
                .collect();
            heads.push(latex::parse("="));
            heads
        }
        Kind::Statistics => app
            .sheets
            .statistics_columns()
            .iter()
            .map(|column| match column {
                Column::X => latex::parse("x"),
                Column::Y => latex::parse("y"),
                Column::Freq => latex::parse(&format!("\\mathrm{{{}}}", FREQ.get(lang))),
            })
            .collect(),
        Kind::Table | Kind::Distribution => Vec::new(),
    }
}

const FREQ: text::Text = t("Häuf.", "Freq");

/// The labels of a form's rows.
fn labels<B>(app: &App<B>, kind: Kind) -> Vec<String> {
    let lang = app.settings.language;
    match kind {
        Kind::Table => [
            text::same("f(x)"),
            text::same("g(x)"),
            t("Von", "From"),
            t("Bis", "To"),
            t("Schritt", "Step"),
        ]
        .iter()
        .map(|label| label.get(lang).to_string())
        .collect(),
        Kind::Distribution => app.sheets.distribution_fields().iter().map(|field| field.label().get(lang).to_string()).collect(),
        Kind::Linear | Kind::Statistics => Vec::new(),
    }
}

/// What the cell at the cursor is called, along the bottom.
fn cursor_name<B>(app: &App<B>, kind: Kind, sheet: &Sheet) -> String {
    let lang = app.settings.language;
    let row = sheet.cursor.0;
    match kind {
        Kind::Linear => format!("{} {}", t("Gl.", "Eq.").get(lang), row + 1),
        Kind::Statistics => format!("{} {}", t("Zeile", "Row").get(lang), row + 1),
        Kind::Table | Kind::Distribution => labels(app, kind).get(row).cloned().unwrap_or_default(),
    }
}

pub(super) fn sheet<B>(shapes: &mut Vec<Shape>, app: &App<B>, kind: Kind, glass: bool) {
    if let Some(answer) = &app.answer {
        return answer_page(shapes, answer, app.settings.language);
    }
    let sheet = app.sheets.get(kind);

    // The heads, with a line before the right-hand side of an LGS.
    for (column, head) in heads(app, kind, sheet).iter().enumerate().skip(sheet.left) {
        let Some(area) = cell_rect(kind, sheet, sheet.top, column) else { continue };
        let head = layout::layout(head, Scale::Small);
        let x = area.top_left.x + (area.size.width as i32 - head.width) / 2;
        shapes.push(Shape::Math(Point::new(x, HEADS - head.descent), head, colors::MUTED));
    }
    if kind == Kind::Linear {
        if let Some(area) = cell_rect(kind, sheet, sheet.top, sheet.columns() - 1) {
            let rows = (sheet.rows() - sheet.top).min(ROWS_SHOWN) as u32;
            let line = Rectangle::new(area.top_left - Point::new(3, 0), Size::new(1, rows * ROW as u32 - 4));
            shapes.push(Shape::Fill(line, colors::HAIRLINE));
        }
    }

    // The labels of a form.
    for (row, label) in labels(app, kind).iter().enumerate().skip(sheet.top).take(ROWS_SHOWN) {
        let y = top(kind) + (row - sheet.top) as i32 * ROW;
        let baseline = y + (ROW - 4 + Scale::Normal.metrics().digit) / 2;
        let label = fit(label, LABEL_WIDTH - 8, Scale::Normal);
        shapes.push(Shape::Text(Point::new(MARGIN, baseline), label, Scale::Normal, colors::MUTED));
    }

    // The cells; the one at the cursor outlined, in the accent while it is edited.
    for row in sheet.top..sheet.rows() {
        for column in sheet.left..sheet.columns() {
            let Some(area) = cell_rect(kind, sheet, row, column) else { continue };
            let here = sheet.cursor == (row, column);
            let outline = match (here, &sheet.editing) {
                (true, Some(_)) => Some(colors::AMBER),
                (true, None) => Some(colors::CYAN),
                _ => None,
            };
            let fill = if here { colors::CYAN_DIM } else { colors::SURFACE };
            shapes.push(panel(rounded(area.top_left, area.size, 6), fill, outline, glass));
            let content = match (here, &sheet.editing) {
                (true, Some(editor)) => &editor.row,
                _ => sheet.cell(row, column),
            };
            if !content.is_empty() {
                cell_content(shapes, content, area);
            }
        }
    }
    // How many rows a list has past the ones shown.
    let below = sheet.rows().saturating_sub(sheet.top + ROWS_SHOWN);
    if sheet.top > 0 {
        arrow(shapes, Point::new(WIDTH as i32 - MARGIN - 6, top(kind) - 4), true);
    }
    if below > 0 {
        arrow(shapes, Point::new(WIDTH as i32 - MARGIN - 6, EDIT_TOP - 4), false);
    }

    // The cell at the cursor in full along the bottom, with the caret while it is edited.
    shapes.push(Shape::Fill(Rectangle::new(Point::new(MARGIN, EDIT_TOP), Size::new(WIDTH - 2 * MARGIN as u32, 1)), colors::HAIRLINE));
    let name = cursor_name(app, kind, sheet);
    let normal = Scale::Normal.metrics();
    shapes.push(Shape::Text(Point::new(MARGIN, EDIT_TOP + 6 + normal.ascent), name, Scale::Normal, colors::MUTED));
    let (row, column) = sheet.cursor;
    let (content, editor) = match &sheet.editing {
        Some(editor) => (&editor.row, Some(editor)),
        None => (sheet.cell(row, column), None),
    };
    let laid = layout::layout(content, Scale::Normal);
    let bottom = HEIGHT as i32 - 8;
    let mut origin = Point::new((WIDTH as i32 - MARGIN - laid.width).max(MARGIN + 70), bottom - laid.descent);
    if let Some(editor) = editor {
        let caret = layout::caret(&laid, origin, editor.path(), editor.pos());
        // A long cell scrolls so the caret stays on the screen.
        let overflow = caret.top_left.x - (WIDTH as i32 - MARGIN);
        if overflow > 0 {
            origin.x -= overflow;
        }
        if app.caret_on {
            let caret = layout::caret(&laid, origin, editor.path(), editor.pos());
            shapes.push(Shape::Fill(caret, colors::CYAN));
        }
    }
    if !content.is_empty() {
        shapes.push(Shape::Math(origin, laid, colors::TEXT));
    }
}

/// What a cell holds, small, at its right; its beginning and three dots when it is too wide
/// (the whole of it shows along the bottom).
fn cell_content(shapes: &mut Vec<Shape>, content: &Row, area: Rectangle) {
    let room = area.size.width as i32 - 10;
    let middle = area.top_left.y + area.size.height as i32 / 2;
    let mut laid = layout::layout(content, Scale::Small);
    let cut = laid.width > room;
    if cut {
        // The longest beginning that leaves room for the dots.
        let room = room - Scale::Small.width("...");
        laid = (0..content.0.len())
            .rev()
            .map(|nodes| layout::layout(&Row(content.0[..nodes].to_vec()), Scale::Small))
            .find(|laid| laid.width <= room)
            .unwrap_or(laid);
    }
    let baseline = middle + (laid.ascent - laid.descent) / 2;
    let right = area.top_left.x + area.size.width as i32 - 5;
    let x = if cut { area.top_left.x + 5 } else { right - laid.width };
    if cut {
        let at = Point::new(x + laid.width, baseline);
        shapes.push(Shape::Text(at, "...".to_string(), Scale::Small, colors::MUTED));
    }
    shapes.push(Shape::Math(Point::new(x, baseline), laid, colors::TEXT));
}

/// A small triangle: there is more above or below.
fn arrow(shapes: &mut Vec<Shape>, tip: Point, up: bool) {
    let (dy, base) = if up { (0, 7) } else { (0, -7) };
    let tip = tip + Point::new(0, dy);
    let triangle = Triangle::new(tip, tip + Point::new(-5, base), tip + Point::new(5, base));
    shapes.push(Shape::Triangle(triangle, colors::FAINT));
}

/// The answer, from its first line or row on: lines of math and the matrices of the steps, or a
/// table; an error in its colour, broken into lines.
fn answer_page(shapes: &mut Vec<Shape>, answer: &Answer, lang: Lang) {
    let bottom = HEIGHT as i32 - 6;
    let x = MARGIN - answer.shift;
    let mut y = BAR + 8;
    let reply = match &answer.reply {
        Ok(reply) => reply,
        Err(message) => {
            let metrics = Scale::Normal.metrics();
            for line in wrap(message, WIDTH as i32 - 2 * MARGIN, Scale::Normal) {
                y += metrics.ascent;
                shapes.push(Shape::Text(Point::new(MARGIN, y), line, Scale::Normal, colors::ERROR));
                y += metrics.descent + 4;
            }
            return;
        }
    };
    if reply.is_empty() {
        let at = Point::new(MARGIN, y + Scale::Normal.metrics().ascent);
        shapes.push(Shape::Text(at, text::NO_ENTRIES.get(lang).to_string(), Scale::Normal, colors::MUTED));
        return;
    }
    let mut more = false;
    match reply {
        Reply::Lines(lines) => {
            for line in lines.iter().skip(answer.scroll) {
                let height = match line {
                    Line::Math(latex) => {
                        let laid = layout::layout(&latex::parse(latex), Scale::Normal);
                        let height = laid.height() + 6;
                        if y + height > bottom {
                            more = true;
                            break;
                        }
                        shapes.push(Shape::Math(Point::new(x, y + laid.ascent + 3), laid, colors::RESULT));
                        height
                    }
                    Line::Note(note) => {
                        let metrics = Scale::Small.metrics();
                        let height = metrics.ascent + metrics.descent + 6;
                        if y + height > bottom {
                            more = true;
                            break;
                        }
                        shapes.push(Shape::Text(Point::new(x, y + 3 + metrics.ascent), note.clone(), Scale::Small, colors::MUTED));
                        height
                    }
                    Line::Matrix(matrix) => {
                        let laid = MatrixLayout::new(matrix);
                        if y + laid.height + 8 > bottom && y > BAR + 8 {
                            more = true;
                            break;
                        }
                        let height = laid.height + 8;
                        laid.draw(shapes, Point::new(x, y + 4));
                        height
                    }
                };
                y += height;
            }
        }
        Reply::Table { header, rows } => {
            let columns = header.len().max(1);
            let width = ((WIDTH as i32 - 2 * MARGIN) / columns as i32).max(120);
            let small = Scale::Small.metrics();
            let line = 30;
            let cell = |shapes: &mut Vec<Shape>, column: usize, latex: &str, y: i32, color| {
                let left = x + column as i32 * width;
                if latex.is_empty() {
                    shapes.push(Shape::Text(Point::new(left + width - 20, y), "-".to_string(), Scale::Small, colors::FAINT));
                    return;
                }
                let laid = layout::layout(&latex::parse(latex), Scale::Small);
                let at = Point::new(left + (width - 10 - laid.width).max(0), y);
                shapes.push(Shape::Math(at, laid, color));
            };
            let heads = y + small.ascent + 2;
            for (column, head) in header.iter().enumerate() {
                cell(shapes, column, head, heads, colors::MUTED);
            }
            y += line;
            shapes.push(Shape::Fill(Rectangle::new(Point::new(MARGIN, y - 6), Size::new(WIDTH - 2 * MARGIN as u32, 1)), colors::HAIRLINE));
            for row in rows.iter().skip(answer.scroll) {
                if y + line > bottom {
                    more = true;
                    break;
                }
                for (column, value) in row.iter().enumerate() {
                    cell(shapes, column, value, y + small.ascent + 4, if column == 0 { colors::TEXT } else { colors::RESULT });
                }
                y += line;
            }
        }
    }
    if answer.scroll > 0 {
        arrow(shapes, Point::new(WIDTH as i32 - MARGIN - 6, BAR + 4), true);
    }
    if more {
        arrow(shapes, Point::new(WIDTH as i32 - MARGIN - 6, HEIGHT as i32 - 4), false);
    }
}

/// `text` broken into lines of at most `room` pixels, at spaces.
fn wrap(text: &str, room: i32, scale: Scale) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(line) if scale.width(&format!("{line} {word}")) <= room => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    lines
}

/// A step's matrix laid out: the names of the rows, the cells in columns right-aligned between
/// brackets with the bar before the right-hand side, and what was done to each row.
struct MatrixLayout {
    names: Vec<Option<RowLayout>>,
    cells: Vec<Vec<RowLayout>>,
    notes: Vec<Option<RowLayout>>,
    /// Per row: its baseline from the top.
    baselines: Vec<i32>,
    widths: Vec<i32>,
    split: usize,
    names_width: i32,
    height: i32,
}

/// The gap between columns.
const GAP: i32 = 12;

impl MatrixLayout {
    fn new(matrix: &Matrix) -> Self {
        let small = |latex: &str| layout::layout(&latex::parse(latex), Scale::Small);
        let cells: Vec<Vec<RowLayout>> = matrix.rows.iter().map(|row| row.iter().map(|cell| small(cell)).collect()).collect();
        // An empty name or note is left out, not drawn as a box to type into.
        let optional = |latex: &String| (!latex.is_empty()).then(|| small(latex));
        let names: Vec<Option<RowLayout>> = matrix.names.iter().map(optional).collect();
        let notes: Vec<Option<RowLayout>> = matrix.notes.iter().map(optional).collect();
        let columns = cells.iter().map(Vec::len).max().unwrap_or(0);
        let widths: Vec<i32> =
            (0..columns).map(|column| cells.iter().filter_map(|row| row.get(column)).map(|cell| cell.width).max().unwrap_or(0)).collect();
        let mut baselines = Vec::new();
        let mut y = 0;
        for (index, row) in cells.iter().enumerate() {
            let others = names.get(index).into_iter().chain(notes.get(index)).flatten();
            let ascent = row.iter().chain(others.clone()).map(|cell| cell.ascent).max().unwrap_or(0);
            let descent = row.iter().chain(others).map(|cell| cell.descent).max().unwrap_or(0);
            baselines.push(y + ascent + 2);
            y += ascent + descent + 4;
        }
        let names_width = names.iter().flatten().map(|name| name.width).max().map_or(0, |width| width + 8);
        MatrixLayout { names, cells, notes, baselines, widths, split: matrix.split, names_width, height: y }
    }

    fn draw(self, shapes: &mut Vec<Shape>, at: Point) {
        let MatrixLayout { names, cells, notes, baselines, widths, split, names_width, height } = self;
        let open = at.x + names_width;
        let bracket = |x: i32, toward: i32| {
            vec![Point::new(x + toward, at.y), Point::new(x, at.y + 3), Point::new(x, at.y + height - 3), Point::new(x + toward, at.y + height)]
        };
        shapes.push(Shape::Path(bracket(open, 4), colors::TEXT, 1));
        let mut column_x = Vec::new();
        let mut x = open + 8;
        for (column, width) in widths.iter().enumerate() {
            if column == split && column > 0 {
                let bar = Rectangle::new(Point::new(x - GAP / 2 - 1, at.y + 2), Size::new(1, (height - 4).max(1) as u32));
                shapes.push(Shape::Fill(bar, colors::MUTED));
            }
            column_x.push(x);
            x += width + GAP;
        }
        let close = x - GAP + 6;
        shapes.push(Shape::Path(bracket(close, -4), colors::TEXT, 1));
        for (index, row) in cells.into_iter().enumerate() {
            let baseline = at.y + baselines[index];
            for (column, cell) in row.into_iter().enumerate() {
                let right = column_x[column] + widths[column];
                shapes.push(Shape::Math(Point::new(right - cell.width, baseline), cell, colors::TEXT));
            }
        }
        for (index, name) in names.into_iter().enumerate() {
            let Some(name) = name else { continue };
            if let Some(&baseline) = baselines.get(index) {
                shapes.push(Shape::Math(Point::new(at.x, at.y + baseline), name, colors::MUTED));
            }
        }
        for (index, note) in notes.into_iter().enumerate() {
            let Some(note) = note else { continue };
            if let Some(&baseline) = baselines.get(index) {
                shapes.push(Shape::Math(Point::new(close + 12, at.y + baseline), note, colors::MUTED));
            }
        }
    }
}
