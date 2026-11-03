//! Kurvendiskussion on the screen: a page per section of the sketch, its steps and then its
//! result, under a line with the section's title and where it stands among the pages.

use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Rectangle, Triangle};

use super::sheet::wrap;
use super::{colors, fit, Shape, BAR, HEIGHT, MARGIN, WIDTH};
use crate::app::Answer;
use crate::backend::{Analysis, Graph, Line, Status, Task};
use crate::editor::{Node, Row};
use crate::font::Scale;
use crate::latex;
use crate::layout::{self, RowLayout};
use crate::text::{self, Lang};

/// Where the page's lines begin, under its title.
const TOP: i32 = BAR + 30;
const BOTTOM: i32 = HEIGHT as i32 - 6;
/// The room for a line of math.
const ROOM: i32 = WIDTH as i32 - 2 * MARGIN;

/// The page of the sketch the answer is on.
pub(super) fn sketch(shapes: &mut Vec<Shape>, answer: &Answer, analysis: &Analysis, lang: Lang) {
    let pages = answer.pages();
    match answer.page.checked_sub(1).and_then(|page| analysis.pages.get(page)) {
        Some(page) => {
            title(shapes, &page.title, page.status, answer.page, pages, lang);
            section(shapes, page.message.as_deref(), &page.lines, &page.result, answer.scroll);
        }
        None => graph_page(shapes, &analysis.function, &analysis.graph, 0, pages, lang),
    }
    sides(shapes, answer.page, pages);
}

/// The answer to a task: its steps and result, and then the graph with what it adds.
pub(super) fn task(shapes: &mut Vec<Shape>, answer: &Answer, task: &Task, lang: Lang) {
    let pages = answer.pages();
    match (&task.graph, answer.page) {
        (Some(graph), 1) => graph_page(shapes, &format!("f(x) = {}", task.function), graph, 1, pages, lang),
        _ => {
            title(shapes, &task.title, Status::Ok, 0, pages, lang);
            section(shapes, None, &task.lines, &task.result, answer.scroll);
        }
    }
    sides(shapes, answer.page, pages);
}

/// The graph, with the function in the title's place.
fn graph_page(shapes: &mut Vec<Shape>, function: &str, graph: &Graph, page: usize, pages: usize, lang: Lang) {
    title(shapes, "", Status::Ok, page, pages, lang);
    let function = layout::layout(&latex::parse(function), Scale::Small);
    let baseline = BAR + 6 + Scale::Small.metrics().ascent;
    shapes.push(Shape::Math(Point::new(MARGIN, baseline), function, colors::AMBER));
    super::graph::graph(shapes, graph, graph_area(), lang);
}

/// Triangles at the sides: there are pages before or after.
fn sides(shapes: &mut Vec<Shape>, answer_page: usize, pages: usize) {
    let middle = (BAR + HEIGHT as i32) / 2;
    if answer_page > 0 {
        let tip = Point::new(1, middle);
        shapes.push(Shape::Triangle(Triangle::new(tip, tip + Point::new(5, -6), tip + Point::new(5, 6)), colors::FAINT));
    }
    if answer_page + 1 < pages {
        let tip = Point::new(WIDTH as i32 - 2, middle);
        shapes.push(Shape::Triangle(Triangle::new(tip, tip + Point::new(-5, -6), tip + Point::new(-5, 6)), colors::FAINT));
    }
}

/// Where the graph is drawn: under the title, between the triangles that turn the page.
pub fn graph_area() -> Rectangle {
    Rectangle::new(Point::new(MARGIN, TOP - 2), Size::new(WIDTH - 2 * MARGIN as u32, (HEIGHT as i32 - TOP - 4) as u32))
}

/// The line on top: the section's title, a word for a section the engine did not finish, and
/// the page among the pages.
fn title(shapes: &mut Vec<Shape>, title: &str, status: Status, page: usize, pages: usize, lang: Lang) {
    let small = Scale::Small.metrics();
    let baseline = BAR + 6 + small.ascent;
    let count = format!("{}/{}", page + 1, pages);
    let count_width = Scale::Small.width(&count);
    shapes.push(Shape::Text(Point::new(WIDTH as i32 - MARGIN - count_width, baseline), count, Scale::Small, colors::FAINT));
    let mark = match status {
        Status::Ok => None,
        Status::Partial => Some(text::SECTION_PARTIAL.get(lang)),
        Status::Failed => Some(text::SECTION_FAILED.get(lang)),
    };
    let mark_width = mark.map_or(0, |mark| Scale::Small.width(mark) + 10);
    let room = WIDTH as i32 - 2 * MARGIN - count_width - 10 - mark_width;
    let title = fit(title, room, Scale::Small);
    let title_width = Scale::Small.width(&title);
    shapes.push(Shape::Text(Point::new(MARGIN, baseline), title, Scale::Small, colors::AMBER));
    if let Some(mark) = mark {
        shapes.push(Shape::Text(Point::new(MARGIN + title_width + 10, baseline), mark.to_string(), Scale::Small, colors::ERROR));
    }
    let rule = Rectangle::new(Point::new(MARGIN, baseline + small.descent + 4), Size::new(WIDTH - 2 * MARGIN as u32, 1));
    shapes.push(Shape::Fill(rule, colors::HAIRLINE));
}

/// What a section says: why it is not complete, its steps, and its result, from the line
/// `scroll` on.
fn section(shapes: &mut Vec<Shape>, message: Option<&str>, steps: &[Line], result: &[Line], scroll: usize) {
    let mut y = TOP;
    let mut more = false;
    if scroll == 0 {
        if let Some(message) = message {
            let small = Scale::Small.metrics();
            for line in wrap(message, ROOM, Scale::Small) {
                shapes.push(Shape::Text(Point::new(MARGIN, y + small.ascent), line, Scale::Small, colors::ERROR));
                y += small.ascent + small.descent + 2;
            }
            y += 4;
        }
    }
    let count = steps.len();
    let steps = steps.iter().map(|line| (line, colors::TEXT));
    let result = result.iter().map(|line| (line, colors::RESULT));
    for (index, (line, color)) in steps.chain(result).enumerate().skip(scroll) {
        // A line between the steps and the result.
        if index == count && index > 0 {
            if y + 6 > BOTTOM {
                more = true;
                break;
            }
            let rule = Rectangle::new(Point::new(MARGIN, y + 2), Size::new(WIDTH / 3, 1));
            shapes.push(Shape::Fill(rule, colors::HAIRLINE));
            y += 6;
        }
        let drawn = lines(line, color);
        let height: i32 = drawn.iter().map(Drawn::height).sum();
        // A line taller than the page still shows at its top.
        if y + height > BOTTOM && y > TOP {
            more = true;
            break;
        }
        for drawn in drawn {
            y += drawn.draw(shapes, y);
        }
    }
    if more {
        let tip = Point::new(WIDTH as i32 - MARGIN - 6, HEIGHT as i32 - 4);
        shapes.push(Shape::Triangle(Triangle::new(tip, tip + Point::new(-5, -7), tip + Point::new(5, -7)), colors::FAINT));
    }
    if scroll > 0 {
        let tip = Point::new(WIDTH as i32 - MARGIN - 6, TOP - 2);
        shapes.push(Shape::Triangle(Triangle::new(tip, tip + Point::new(-5, 7), tip + Point::new(5, 7)), colors::FAINT));
    }
}

/// A line ready to draw: one row of math or text, at the left or the right.
enum Drawn {
    Math { laid: RowLayout, x: i32, color: embedded_graphics::pixelcolor::Rgb565 },
    Note(String),
    Grid(Vec<Vec<RowLayout>>),
}

/// The gap between the columns of a sign table, and below each line.
const GAP: i32 = 12;
const SPACING: i32 = 6;

impl Drawn {
    fn height(&self) -> i32 {
        match self {
            Drawn::Math { laid, .. } => laid.height() + SPACING,
            Drawn::Note(_) => {
                let small = Scale::Small.metrics();
                small.ascent + small.descent + 4
            }
            Drawn::Grid(rows) => rows.iter().map(|row| row_height(row) + 2).sum::<i32>() + SPACING,
        }
    }

    /// Draws it with its top at `y`; returns its height.
    fn draw(self, shapes: &mut Vec<Shape>, y: i32) -> i32 {
        let height = self.height();
        match self {
            Drawn::Math { laid, x, color } => shapes.push(Shape::Math(Point::new(x, y + laid.ascent + 2), laid, color)),
            Drawn::Note(note) => {
                let at = Point::new(MARGIN, y + Scale::Small.metrics().ascent + 2);
                shapes.push(Shape::Text(at, note, Scale::Small, colors::MUTED));
            }
            Drawn::Grid(rows) => {
                let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
                let widths: Vec<i32> =
                    (0..columns).map(|column| rows.iter().filter_map(|row| row.get(column)).map(|cell| cell.width).max().unwrap_or(0)).collect();
                let mut top = y + 2;
                for (index, row) in rows.into_iter().enumerate() {
                    let ascent = row.iter().map(|cell| cell.ascent).max().unwrap_or(0);
                    let row_height = row_height(&row);
                    let mut x = MARGIN;
                    for (column, cell) in row.into_iter().enumerate() {
                        let color = if column == 0 || index == 0 { colors::MUTED } else { colors::TEXT };
                        // Centred in its column.
                        let at = Point::new(x + (widths[column] - cell.width) / 2, top + ascent);
                        shapes.push(Shape::Math(at, cell, color));
                        x += widths[column] + GAP;
                    }
                    top += row_height + 2;
                    if index == 0 {
                        let width = (x - GAP - MARGIN).max(0) as u32;
                        shapes.push(Shape::Fill(Rectangle::new(Point::new(MARGIN, top - 2), Size::new(width, 1)), colors::HAIRLINE));
                    }
                }
                let first = widths.first().copied().unwrap_or(0);
                let rule = Rectangle::new(Point::new(MARGIN + first + GAP / 2, y + 2), Size::new(1, (top - y - 4).max(1) as u32));
                shapes.push(Shape::Fill(rule, colors::HAIRLINE));
            }
        }
        height
    }
}

fn row_height(row: &[RowLayout]) -> i32 {
    row.iter().map(|cell| cell.ascent).max().unwrap_or(0) + row.iter().map(|cell| cell.descent).max().unwrap_or(0)
}

/// `line` as it fits the screen: a long equation broken before its `=`, `≈` or `⇒`, the
/// rest right-aligned below as on a Casio, or small when it still does not fit; a sign table
/// turned on its side when it is too wide.
fn lines(line: &Line, color: embedded_graphics::pixelcolor::Rgb565) -> Vec<Drawn> {
    match line {
        Line::Note(note) => wrap(note, ROOM, Scale::Small).into_iter().map(Drawn::Note).collect(),
        Line::Math(latex) => broken(&latex::parse(latex), color),
        Line::Grid(rows) => {
            let laid = |rows: &[Vec<String>]| -> Vec<Vec<RowLayout>> {
                rows.iter().map(|row| row.iter().map(|cell| layout::layout(&latex::parse(cell), Scale::Small)).collect()).collect()
            };
            let grid = laid(rows);
            let columns = grid.iter().map(Vec::len).max().unwrap_or(0);
            let width: i32 = (0..columns)
                .map(|column| grid.iter().filter_map(|row| row.get(column)).map(|cell| cell.width).max().unwrap_or(0) + GAP)
                .sum();
            if width <= ROOM {
                return vec![Drawn::Grid(grid)];
            }
            // The intervals down the side, each with its signs.
            let turned: Vec<Vec<String>> = (0..columns).map(|column| rows.iter().map(|row| row.get(column).cloned().unwrap_or_default()).collect()).collect();
            vec![Drawn::Grid(laid(&turned))]
        }
        Line::Matrix(_) => Vec::new(),
    }
}

/// A row of math broken to fit [`ROOM`].
fn broken(row: &Row, color: embedded_graphics::pixelcolor::Rgb565) -> Vec<Drawn> {
    let whole = layout::layout(row, Scale::Normal);
    if whole.width <= ROOM {
        return vec![Drawn::Math { laid: whole, x: MARGIN, color }];
    }
    // The pieces: each starts with its relation (but the first), or after a `:` or `,` that
    // ends a list's lead or item.
    let mut pieces: Vec<Row> = vec![Row::default()];
    let mut depth = 0;
    for (index, node) in row.0.iter().enumerate() {
        let open = !pieces.last().is_some_and(|piece| piece.0.is_empty());
        match node {
            Node::Char('(') => depth += 1,
            Node::Char(')') => depth -= 1,
            Node::Char('=' | '≈' | '⇒' | '⇔') if depth == 0 && open => pieces.push(Row::default()),
            _ => {}
        }
        pieces.last_mut().expect("never empty").0.push(node.clone());
        // Not the decimal comma, which stands between digits.
        let digit = |at: Option<&Node>| matches!(at, Some(Node::Char(c)) if c.is_ascii_digit());
        let decimal = *node == Node::Char(',') && digit(index.checked_sub(1).and_then(|at| row.0.get(at))) && digit(row.0.get(index + 1));
        if depth == 0 && matches!(node, Node::Char(':' | ',')) && !decimal {
            pieces.push(Row::default());
        }
    }
    // As many pieces on a line as fit.
    let mut out = Vec::new();
    let mut current = Row::default();
    for piece in pieces {
        let mut joined = current.clone();
        joined.0.extend(piece.0.iter().cloned());
        if current.0.is_empty() || layout::layout(&joined, Scale::Normal).width <= ROOM {
            current = joined;
        } else {
            out.push(std::mem::replace(&mut current, piece));
        }
    }
    out.push(current);
    // A line starts without the space a break left.
    for row in &mut out {
        while matches!(row.0.first(), Some(Node::Char(crate::editor::THIN_SPACE | ' '))) {
            row.0.remove(0);
        }
    }
    out.retain(|row| !row.0.is_empty());
    let count = out.len();
    out.into_iter()
        .enumerate()
        .map(|(index, row)| {
            let mut laid = layout::layout(&row, Scale::Normal);
            if laid.width > ROOM {
                laid = layout::layout(&row, Scale::Small);
            }
            // The first from the left, the rest to the right.
            let x = if index == 0 || count == 1 { MARGIN } else { (WIDTH as i32 - MARGIN - laid.width).max(MARGIN) };
            Drawn::Math { laid, x, color }
        })
        .collect()
}
