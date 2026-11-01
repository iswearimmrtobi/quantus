//! The 2D layout of a [`Row`]: fractions stacked, exponents raised, roots drawn. Positions are in
//! pixels, relative to the left end of a row's baseline, y downwards.

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Polyline, PrimitiveStyle, Rectangle};

use crate::editor::{Node, Row};
use crate::font::Scale;

/// Characters that get a gap on both sides.
const OPERATORS: &str = "+-=×÷±→≈≤≥≠";

#[derive(Clone, Debug)]
pub struct RowLayout {
    pub scale: Scale,
    pub width: i32,
    pub ascent: i32,
    pub descent: i32,
    /// One per node of the row.
    pub items: Vec<Item>,
}

#[derive(Clone, Debug)]
pub struct Item {
    pub x: i32,
    pub width: i32,
    pub ascent: i32,
    pub descent: i32,
    pub kind: Kind,
    /// The node's rows, each with its origin relative to the item's origin.
    pub slots: Vec<(Point, RowLayout)>,
}

#[derive(Clone, Debug)]
pub enum Kind {
    /// Text, starting `dx` right of the item's origin.
    Text { text: String, dx: i32 },
    /// A fraction bar.
    Bar(Rectangle),
    /// The lines of a root sign.
    Sign([Point; 4]),
    /// Only its slot: an exponent or index.
    Script,
    /// A bracket drawn as a curve over the item's height, grown to what it holds.
    Paren { open: bool },
}

impl RowLayout {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn height(&self) -> i32 {
        self.ascent + self.descent
    }

    /// The area drawn when the row's origin is at `origin`.
    pub fn bounds(&self, origin: Point) -> Rectangle {
        Rectangle::new(
            origin - Point::new(0, self.ascent),
            Size::new(self.width.max(0) as u32, self.height().max(0) as u32),
        )
    }
}

/// Lays out `row`; `nested` is true inside a fraction, where fractions get small.
pub fn layout(row: &Row, scale: Scale) -> RowLayout {
    layout_nested(row, scale, false)
}

fn layout_nested(row: &Row, scale: Scale, nested: bool) -> RowLayout {
    let metrics = scale.metrics();
    if row.is_empty() {
        // A box to type into.
        return RowLayout {
            scale,
            width: metrics.digit * 2 / 3 + 2,
            ascent: metrics.digit,
            descent: 0,
            items: Vec::new(),
        };
    }
    let mut items: Vec<Item> = Vec::with_capacity(row.0.len());
    let mut x = 0;
    // The items of the brackets still open.
    let mut open = Vec::new();
    for (index, node) in row.0.iter().enumerate() {
        let base_ascent = items.last().map_or(metrics.digit, |item| item.ascent);
        let unary = index == 0 || matches!(row.0[index - 1], Node::Char(c) if c == '(' || OPERATORS.contains(c));
        let mut item = node_layout(node, scale, nested, base_ascent, unary);
        item.x = x;
        x += item.width;
        items.push(item);
        // A closed bracket grows before what follows it, so an exponent sits above it.
        match node {
            Node::Char('(') => open.push(index),
            Node::Char(')') => {
                let start = open.pop().unwrap_or(0);
                grow(&mut items, start, index, scale);
            }
            _ => {}
        }
    }
    // Open brackets reach to the end of the row.
    while let Some(start) = open.pop() {
        let last = items.len() - 1;
        grow(&mut items, start, last, scale);
    }
    RowLayout {
        scale,
        width: x,
        ascent: items.iter().map(|item| item.ascent).max().unwrap_or(0).max(metrics.digit),
        descent: items.iter().map(|item| item.descent).max().unwrap_or(0),
        items,
    }
}

/// Makes the brackets among `items[first]` and `items[last]` as tall as what lies between them,
/// when that is taller than the font's bracket.
fn grow(items: &mut [Item], first: usize, last: usize, scale: Scale) {
    let metrics = scale.metrics();
    // Brackets and scripts do not count: an exponent may stick out, as after a single digit.
    let counts = |item: &Item| match &item.kind {
        Kind::Text { text, .. } => text != "(" && text != ")",
        Kind::Paren { .. } | Kind::Script => false,
        _ => true,
    };
    let inner = || items[first..=last].iter().filter(|item| counts(item));
    let ascent = inner().map(|item| item.ascent).max().unwrap_or(0);
    let descent = inner().map(|item| item.descent).max().unwrap_or(0);
    if ascent <= metrics.ascent + 2 && descent <= metrics.descent + 2 {
        return;
    }
    let (ascent, descent) = (ascent.max(metrics.ascent) + 1, descent.max(metrics.descent) + 1);
    for index in [first, last] {
        let item = &mut items[index];
        let open = match &item.kind {
            Kind::Text { text, .. } if text == "(" || text == ")" => text == "(",
            Kind::Paren { open } => *open,
            _ => continue,
        };
        item.kind = Kind::Paren { open };
        item.ascent = ascent;
        item.descent = descent;
    }
}

fn text_item(text: String, dx: i32, width: i32, scale: Scale) -> Item {
    let metrics = scale.metrics();
    Item {
        x: 0,
        width,
        ascent: metrics.ascent,
        descent: metrics.descent,
        kind: Kind::Text { text, dx },
        slots: Vec::new(),
    }
}

/// `base_ascent` is how far what comes before reaches up, for placing an exponent; `unary` says
/// that an operator here is a sign and gets no gaps.
fn node_layout(node: &Node, scale: Scale, nested: bool, base_ascent: i32, unary: bool) -> Item {
    let metrics = scale.metrics();
    let small = Scale::Small.metrics();
    match node {
        Node::Char(c) => {
            let text = c.to_string();
            let gap = if OPERATORS.contains(*c) && !unary { metrics.digit / 4 } else { 0 };
            let width = scale.width(&text) + 2 * gap;
            text_item(text, gap, width, scale)
        }
        Node::Func(name) => text_item(name.clone(), 0, scale.width(name) + 2, scale),
        Node::Upright(text) => text_item(text.clone(), 0, scale.width(text), scale),
        Node::Pow(exponent) => {
            let exponent = layout_nested(exponent, Scale::Small, nested);
            let rise = base_ascent.max(metrics.digit) - small.digit / 2;
            script(exponent, -rise)
        }
        Node::Sub(index) => script(layout_nested(index, Scale::Small, nested), small.digit / 2),
        Node::Frac(numerator, denominator) => {
            let part = if nested { Scale::Small } else { scale };
            let numerator = layout_nested(numerator, part, true);
            let denominator = layout_nested(denominator, part, true);
            let pad = 2;
            let gap = 2;
            let inner = numerator.width.max(denominator.width) + 2 * pad;
            let bar_y = -metrics.axis;
            let numerator_at =
                Point::new(1 + (inner - numerator.width) / 2, bar_y - gap - numerator.descent);
            let denominator_at =
                Point::new(1 + (inner - denominator.width) / 2, bar_y + 1 + gap + denominator.ascent);
            Item {
                x: 0,
                width: inner + 2,
                ascent: numerator.ascent - numerator_at.y,
                descent: denominator_at.y + denominator.descent,
                kind: Kind::Bar(Rectangle::new(Point::new(1, bar_y), Size::new(inner as u32, 1))),
                slots: vec![(numerator_at, numerator), (denominator_at, denominator)],
            }
        }
        Node::Sqrt(radicand) => root(None, layout_nested(radicand, scale, nested), scale),
        Node::Root(index, radicand) => {
            let index = layout_nested(index, Scale::Small, nested);
            root(Some(index), layout_nested(radicand, scale, nested), scale)
        }
    }
}

fn script(row: RowLayout, dy: i32) -> Item {
    Item {
        x: 0,
        width: row.width + 1,
        ascent: row.ascent - dy,
        descent: row.descent + dy,
        kind: Kind::Script,
        slots: vec![(Point::new(0, dy), row)],
    }
}

fn root(index: Option<RowLayout>, radicand: RowLayout, scale: Scale) -> Item {
    let metrics = scale.metrics();
    let sign = metrics.digit / 2 + 2;
    let top = -(radicand.ascent + 3);
    let bottom = radicand.descent.max(1);
    // The index sits over the short stroke and may push the sign to the right.
    let lead = index.as_ref().map_or(0, |index| (index.width - sign / 2).max(0));
    let points = [
        Point::new(lead, -metrics.axis),
        Point::new(lead + sign / 3, bottom),
        Point::new(lead + sign, top),
        Point::new(lead + sign + radicand.width + 3, top),
    ];
    let mut ascent = -top + 1;
    let mut slots = vec![(Point::new(lead + sign + 2, 0), radicand)];
    if let Some(index) = index {
        let at = Point::new(0, -metrics.axis - 2 - index.descent);
        ascent = ascent.max(index.ascent - at.y);
        slots.insert(0, (at, index));
    }
    Item {
        x: 0,
        width: points[3].x + 1,
        ascent,
        descent: bottom + 1,
        kind: Kind::Sign(points),
        slots,
    }
}

/// The caret for a cursor at `pos` in the row the `path` leads to, with the layout's origin at
/// `origin`.
pub fn caret(row: &RowLayout, origin: Point, path: &[(usize, usize)], pos: usize) -> Rectangle {
    if let Some((&(index, slot), rest)) = path.split_first() {
        let item = &row.items[index];
        let (offset, inner) = &item.slots[slot];
        return caret(inner, origin + Point::new(item.x, 0) + *offset, rest, pos);
    }
    let x = row.items.get(pos).map_or(row.width, |item| item.x);
    let x = if row.is_empty() { 1 } else { x };
    let metrics = row.scale.metrics();
    Rectangle::new(
        origin + Point::new(x - 1, -metrics.ascent),
        Size::new(2, (metrics.ascent + metrics.descent) as u32),
    )
}

/// Draws the row with its origin at `origin`; empty rows as a box in `placeholder`.
pub fn draw<D: DrawTarget<Color = Rgb565>>(
    row: &RowLayout,
    origin: Point,
    color: Rgb565,
    placeholder: Rgb565,
    target: &mut D,
) {
    if row.is_empty() {
        let bounds = row.bounds(origin);
        let bounds = Rectangle::new(bounds.top_left + Point::new(1, 0), bounds.size - Size::new(2, 0));
        let _ = bounds.into_styled(PrimitiveStyle::with_stroke(placeholder, 1)).draw(target);
        return;
    }
    for item in &row.items {
        let at = origin + Point::new(item.x, 0);
        match &item.kind {
            Kind::Text { text, dx } => row.scale.draw(text, at + Point::new(*dx, 0), color, target),
            Kind::Bar(bar) => {
                let _ = bar.translate(at).into_styled(PrimitiveStyle::with_fill(color)).draw(target);
            }
            Kind::Sign(points) => {
                let line = Polyline::new(points).translate(at);
                let _ = line.into_styled(PrimitiveStyle::with_stroke(color, 1)).draw(target);
            }
            Kind::Script => {}
            Kind::Paren { open } => paren(*open, item, at, row.scale, color, target),
        }
        for (offset, slot) in &item.slots {
            draw(slot, at + *offset, color, placeholder, target);
        }
    }
}

/// A bracket over the item's whole height: a flat parabola, thickest in the middle like the font's.
fn paren<D: DrawTarget<Color = Rgb565>>(
    open: bool,
    item: &Item,
    at: Point,
    scale: Scale,
    color: Rgb565,
    target: &mut D,
) {
    const STEPS: i32 = 16;
    let top = at.y - item.ascent + 1;
    let height = item.ascent + item.descent - 2;
    let (inner, outer) = (at.x + 1, at.x + item.width - 2);
    let (inner, outer) = if open { (inner, outer) } else { (outer, inner) };
    let points: Vec<Point> = (0..=STEPS)
        .map(|step| {
            let t = 2 * step - STEPS;
            let x = inner + (outer - inner) * t * t / (STEPS * STEPS);
            Point::new(x, top + height * step / STEPS)
        })
        .collect();
    let width = if scale == Scale::Normal { 2 } else { 1 };
    let _ = Polyline::new(&points).into_styled(PrimitiveStyle::with_stroke(color, width)).draw(target);
}
