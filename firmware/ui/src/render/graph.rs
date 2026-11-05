//! The graph of a curve sketch: a grid with 1-2-5 steps, the axes with their numbers, the
//! curve, the asymptotes dashed and the marked points with their names.

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, Rectangle};

use super::{colors, Shape};
use crate::backend::{Graph, Mark, MarkKind, Straight, Window};
use crate::font::Scale;
use crate::latex;
use crate::layout;
use crate::text::Lang;

/// Where on the screen the window of the plane lies.
#[derive(Clone, Copy, Debug)]
pub struct Plot {
    pub area: Rectangle,
    pub window: Window,
}

impl Plot {
    fn left(&self) -> f32 {
        self.area.top_left.x as f32
    }

    fn top(&self) -> f32 {
        self.area.top_left.y as f32
    }

    fn right(&self) -> f32 {
        self.left() + self.area.size.width as f32 - 1.0
    }

    fn bottom(&self) -> f32 {
        self.top() + self.area.size.height as f32 - 1.0
    }

    /// The screen position of the point (x, y) of the plane, not rounded.
    fn screen(&self, x: f32, y: f32) -> (f32, f32) {
        let w = self.window;
        let sx = self.left() + (x - w.x_min) / (w.x_max - w.x_min) * (self.right() - self.left());
        let sy = self.bottom() - (y - w.y_min) / (w.y_max - w.y_min) * (self.bottom() - self.top());
        (sx, sy)
    }

    fn point(&self, x: f32, y: f32) -> Option<Point> {
        let (sx, sy) = self.screen(x, y);
        let inside = sx >= self.left() && sx <= self.right() && sy >= self.top() && sy <= self.bottom();
        inside.then(|| Point::new(sx.round() as i32, sy.round() as i32))
    }

    /// The part of the line from `a` to `b` (on the screen) inside the area (Liang–Barsky).
    fn clip(&self, a: (f32, f32), b: (f32, f32)) -> Option<((f32, f32), (f32, f32))> {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let (mut low, mut high) = (0.0f32, 1.0f32);
        for (p, q) in [(-dx, a.0 - self.left()), (dx, self.right() - a.0), (-dy, a.1 - self.top()), (dy, self.bottom() - a.1)] {
            if p == 0.0 {
                if q < 0.0 {
                    return None;
                }
            } else {
                let t = q / p;
                if p < 0.0 {
                    low = low.max(t);
                } else {
                    high = high.min(t);
                }
            }
        }
        (low <= high).then_some(((a.0 + low * dx, a.1 + low * dy), (a.0 + high * dx, a.1 + high * dy)))
    }

    /// A polyline of the plane, clipped to the area, as the pieces that stay inside.
    fn polylines(&self, points: &[(f32, f32)]) -> Vec<Vec<Point>> {
        let round = |(x, y): (f32, f32)| Point::new(x.round() as i32, y.round() as i32);
        let mut out: Vec<Vec<Point>> = Vec::new();
        let mut current: Vec<Point> = Vec::new();
        for pair in points.windows(2) {
            let (a, b) = (self.screen(pair[0].0, pair[0].1), self.screen(pair[1].0, pair[1].1));
            if !(a.0.is_finite() && a.1.is_finite() && b.0.is_finite() && b.1.is_finite()) {
                continue;
            }
            match self.clip(a, b) {
                Some((from, to)) => {
                    let (from, to) = (round(from), round(to));
                    if current.last() != Some(&from) {
                        if current.len() > 1 {
                            out.push(std::mem::take(&mut current));
                        }
                        current = vec![from];
                    }
                    if current.last() != Some(&to) {
                        current.push(to);
                    }
                }
                None => {
                    if current.len() > 1 {
                        out.push(std::mem::take(&mut current));
                    }
                    current.clear();
                }
            }
        }
        if current.len() > 1 {
            out.push(current);
        }
        out
    }

    /// A straight line across the area, as its two ends on the screen.
    fn across(&self, line: Straight) -> Option<((f32, f32), (f32, f32))> {
        let w = self.window;
        let (a, b) = match line {
            Straight::Vertical(x) => ((x, w.y_min), (x, w.y_max)),
            Straight::Sloped { .. } => ((w.x_min, line.at(w.x_min)?), (w.x_max, line.at(w.x_max)?)),
        };
        self.clip(self.screen(a.0, a.1), self.screen(b.0, b.1))
    }
}

/// A step of 1, 2 or 5 times a power of ten that cuts `span` into about `parts`.
pub fn nice_step(span: f32, parts: f32) -> f32 {
    let rough = (span / parts).max(f32::MIN_POSITIVE);
    let power = 10f32.powf(rough.log10().floor());
    let step = match rough / power {
        r if r < 1.5 => 1.0,
        r if r < 3.5 => 2.0,
        r if r < 7.5 => 5.0,
        _ => 10.0,
    };
    step * power
}

/// A number on an axis, with as many decimals as its step needs.
fn tick_label(value: f32, step: f32, lang: Lang) -> String {
    let decimals = (-step.log10().floor()).max(0.0) as usize;
    let value = if value.abs() < step / 1000.0 { 0.0 } else { value };
    let text = format!("{value:.decimals$}");
    text.replace('.', lang.decimal())
}

/// The colour a point is marked in.
fn mark_color(kind: MarkKind) -> Rgb565 {
    match kind {
        MarkKind::Max | MarkKind::Min | MarkKind::Boundary => colors::AMBER,
        MarkKind::Zero => colors::CYAN,
        MarkKind::Inflection | MarkKind::Saddle => colors::ALPHA,
        MarkKind::YIntercept => colors::MUTED,
        MarkKind::Hole => colors::TEXT,
        MarkKind::Touch => colors::SHIFT,
    }
}

/// Calls `put` with `x`, and with every member of its periodic family between `low` and
/// `high` (a few hundred at most, however small the period).
pub fn repeat(x: f32, period: Option<f32>, low: f32, high: f32, mut put: impl FnMut(f32)) {
    let Some(period) = period.filter(|period| *period > 0.0 && period.is_finite()) else { return put(x) };
    let first = x + ((low - x) / period).ceil() * period;
    let mut at = first;
    while at <= high && at < first + 400.0 * period {
        put(at);
        at += period;
    }
}

/// A line of short dashes from `a` to `b`.
fn dashed(shapes: &mut Vec<Shape>, a: (f32, f32), b: (f32, f32), color: Rgb565) {
    const DASH: f32 = 6.0;
    const GAP: f32 = 4.0;
    let length = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
    if length < 1.0 {
        return;
    }
    let (ux, uy) = ((b.0 - a.0) / length, (b.1 - a.1) / length);
    let mut at = 0.0;
    while at < length {
        let end = (at + DASH).min(length);
        let from = Point::new((a.0 + ux * at).round() as i32, (a.1 + uy * at).round() as i32);
        let to = Point::new((a.0 + ux * end).round() as i32, (a.1 + uy * end).round() as i32);
        shapes.push(Shape::Path(vec![from, to], color, 1));
        at += DASH + GAP;
    }
}

/// Points of a line drawn as one shape: the band renderer draws a shape in every band its
/// bounds touch, so a curve across the screen is cut into short pieces.
const PIECE: usize = 12;

/// `line` in short pieces, 1 px wide; `wide` draws it again a pixel to the right and down,
/// which is far cheaper than a 2 px stroke (a curve of 600 points took 7 s on the board).
fn stroke(shapes: &mut Vec<Shape>, line: &[Point], color: Rgb565, wide: bool) {
    let offsets: &[Point] = if wide { &[Point::zero(), Point::new(1, 0), Point::new(0, 1)] } else { &[Point::zero()] };
    for piece in line.chunks(PIECE - 1).enumerate().map(|(index, _)| &line[index * (PIECE - 1)..((index + 1) * (PIECE - 1) + 1).min(line.len())]) {
        if piece.len() < 2 {
            continue;
        }
        for &offset in offsets {
            shapes.push(Shape::Path(piece.iter().map(|&point| point + offset).collect(), color, 1));
        }
    }
}

/// Upright lines every few pixels between `curve` and the x-axis.
fn hatch(shapes: &mut Vec<Shape>, plot: &Plot, curve: &[(f32, f32)], color: Rgb565) {
    const EVERY: usize = 3;
    let screen: Vec<(f32, f32)> = curve.iter().map(|&(x, y)| plot.screen(x, y)).filter(|(x, y)| x.is_finite() && y.is_finite()).collect();
    let axis = plot.screen(0.0, 0.0).1;
    let (Some(first), Some(last)) = (screen.first(), screen.last()) else { return };
    let (from, to) = (first.0.max(plot.left()).ceil() as i32, last.0.min(plot.right()).floor() as i32);
    let mut at = 0;
    for x in (from..=to).step_by(EVERY) {
        // The curve's height at x, between the samples around it.
        while at + 1 < screen.len() && screen[at + 1].0 < x as f32 {
            at += 1;
        }
        let (a, b) = (screen[at], screen[(at + 1).min(screen.len() - 1)]);
        let t = if b.0 > a.0 { (x as f32 - a.0) / (b.0 - a.0) } else { 0.0 };
        let y = a.1 + t.clamp(0.0, 1.0) * (b.1 - a.1);
        let Some((top, bottom)) = plot.clip((x as f32, y), (x as f32, axis)) else { continue };
        let (top, bottom) = (top.1.round() as i32, bottom.1.round() as i32);
        let (top, bottom) = (top.min(bottom), top.max(bottom));
        if bottom > top {
            shapes.push(Shape::Fill(Rectangle::new(Point::new(x, top), Size::new(1, (bottom - top) as u32)), color));
        }
    }
}

/// Draws `graph` into `area`.
pub fn graph(shapes: &mut Vec<Shape>, graph: &Graph, area: Rectangle, lang: Lang) {
    let window = graph.window;
    if !(window.x_max > window.x_min && window.y_max > window.y_min) {
        return;
    }
    let plot = Plot { area, window };
    let small = Scale::Small.metrics();

    // The grid, and the axes over it.
    let x_step = nice_step(window.x_max - window.x_min, 6.0);
    let y_step = nice_step(window.y_max - window.y_min, 4.0);
    let ticks = |low: f32, high: f32, step: f32| {
        let first = (low / step).ceil() as i64;
        let last = (high / step).floor() as i64;
        (first..=last).map(move |k| k as f32 * step)
    };
    let (origin_x, origin_y) = plot.screen(0.0, 0.0);
    let axis_x = origin_y.clamp(plot.top(), plot.bottom()).round() as i32;
    let axis_y = origin_x.clamp(plot.left(), plot.right()).round() as i32;
    for x in ticks(window.x_min, window.x_max, x_step) {
        let sx = plot.screen(x, 0.0).0.round() as i32;
        let line = Rectangle::new(Point::new(sx, area.top_left.y), Size::new(1, area.size.height));
        shapes.push(Shape::Fill(line, colors::HAIRLINE));
    }
    for y in ticks(window.y_min, window.y_max, y_step) {
        let sy = plot.screen(0.0, y).1.round() as i32;
        let line = Rectangle::new(Point::new(area.top_left.x, sy), Size::new(area.size.width, 1));
        shapes.push(Shape::Fill(line, colors::HAIRLINE));
    }
    shapes.push(Shape::Fill(Rectangle::new(Point::new(area.top_left.x, axis_x), Size::new(area.size.width, 1)), colors::MUTED));
    shapes.push(Shape::Fill(Rectangle::new(Point::new(axis_y, area.top_left.y), Size::new(1, area.size.height)), colors::MUTED));
    // The numbers: under the x-axis, right of the y-axis; 0 once, at the origin.
    let label_y = (axis_x + 3 + small.ascent).min(area.top_left.y + area.size.height as i32 - small.descent);
    for x in ticks(window.x_min, window.x_max, x_step).filter(|x| x.abs() > x_step / 1000.0) {
        let text = tick_label(x, x_step, lang);
        let width = Scale::Small.width(&text);
        let sx = plot.screen(x, 0.0).0.round() as i32 - width / 2;
        let sx = sx.clamp(area.top_left.x, area.top_left.x + area.size.width as i32 - width);
        shapes.push(Shape::Text(Point::new(sx, label_y), text, Scale::Small, colors::FAINT));
    }
    for y in ticks(window.y_min, window.y_max, y_step).filter(|y| y.abs() > y_step / 1000.0) {
        let text = tick_label(y, y_step, lang);
        let width = Scale::Small.width(&text);
        let sy = plot.screen(0.0, y).1.round() as i32 + small.digit / 2;
        if sy - small.digit < area.top_left.y || sy > area.top_left.y + area.size.height as i32 {
            continue;
        }
        let sx = (axis_y + 4).min(area.top_left.x + area.size.width as i32 - width);
        shapes.push(Shape::Text(Point::new(sx, sy), text, Scale::Small, colors::FAINT));
    }

    // An area as upright hatching down to the axis, in a tint per side.
    for region in &graph.overlay.regions {
        let color = if region.positive { colors::CYAN_DIM } else { colors::AMBER_DIM };
        hatch(shapes, &plot, &region.curve, color);
    }

    // The asymptotes, then the curve over them.
    for asymptote in &graph.asymptotes {
        let lines: Vec<Straight> = match asymptote.line {
            Straight::Vertical(x) => {
                let mut xs = Vec::new();
                repeat(x, asymptote.period, window.x_min, window.x_max, |x| xs.push(Straight::Vertical(x)));
                xs
            }
            line => vec![line],
        };
        for line in lines {
            if let Some((a, b)) = plot.across(line) {
                dashed(shapes, a, b, colors::MUTED);
            }
        }
    }
    for piece in &graph.curve {
        for line in plot.polylines(piece) {
            stroke(shapes, &line, colors::RESULT, true);
        }
    }

    // A task's tangent and normal over the curve.
    for (line, color) in [(graph.overlay.tangent, colors::SHIFT), (graph.overlay.normal, colors::ALPHA)] {
        let Some((a, b)) = line.and_then(|line| plot.across(line)) else { continue };
        let round = |(x, y): (f32, f32)| Point::new(x.round() as i32, y.round() as i32);
        shapes.push(Shape::Path(vec![round(a), round(b)], color, 1));
    }

    // The points with their names, a hole as a ring; a name that would cover one already
    // written is left out (N₂, W and S_y at the origin).
    let mut written: Vec<Rectangle> = Vec::new();
    // The points as the sketch lists them, then the other members of their periodic families
    // in the window, each place once (H_2 of the sine is also a member of H_1's family). Only
    // the listed ones are named.
    let mut members: Vec<(Point, &Mark, bool)> = Vec::new();
    let marks: Vec<&Mark> = graph.overlay.point.iter().chain(&graph.points).collect();
    for &mark in &marks {
        if let Some(at) = plot.point(mark.x, mark.y) {
            members.push((at, mark, true));
        }
    }
    for &mark in &marks {
        repeat(mark.x, mark.period, window.x_min, window.x_max, |x| {
            let Some(at) = plot.point(x, mark.y) else { return };
            let taken = members.iter().any(|(other, kind, _)| kind.kind == mark.kind && (other.x - at.x).abs() <= 2 && (other.y - at.y).abs() <= 2);
            if !taken {
                members.push((at, mark, false));
            }
        });
    }
    for (at, mark, listed) in members {
        let color = mark_color(mark.kind);
        shapes.push(Shape::Circle(Circle::with_center(at, 7), color));
        if mark.kind == MarkKind::Hole {
            shapes.push(Shape::Circle(Circle::with_center(at, 3), colors::BACKGROUND));
        }
        if !listed {
            continue;
        }
        let name = layout::layout(&latex::parse(&mark.name), Scale::Small);
        let x = if at.x + 6 + name.width > area.top_left.x + area.size.width as i32 { at.x - 6 - name.width } else { at.x + 6 };
        let y = if at.y - 4 - name.ascent < area.top_left.y { at.y + 6 + name.ascent } else { at.y - 4 };
        let bounds = name.bounds(Point::new(x, y));
        if written.iter().any(|other| other.intersection(&bounds).size != Size::zero()) {
            continue;
        }
        written.push(bounds);
        shapes.push(Shape::Math(Point::new(x, y), name, color));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_are_one_two_or_five() {
        assert_eq!(nice_step(10.0, 5.0), 2.0);
        assert_eq!(nice_step(5.4, 6.0), 1.0);
        assert_eq!(nice_step(18.0, 4.0), 5.0);
        assert!((nice_step(0.9, 4.0) - 0.2).abs() < 1e-6);
    }

    #[test]
    fn a_label_has_the_decimals_of_its_step() {
        assert_eq!(tick_label(2.0, 1.0, Lang::De), "2");
        assert_eq!(tick_label(-0.5, 0.5, Lang::De), "-0,5");
        assert_eq!(tick_label(0.4, 0.2, Lang::En), "0.4");
    }

    #[test]
    fn a_periodic_point_repeats_across_the_window() {
        let mut xs = Vec::new();
        repeat(0.0, Some(std::f32::consts::PI), -7.0, 7.0, |x| xs.push(x));
        assert_eq!(xs.len(), 5);
        let mut one = Vec::new();
        repeat(2.0, None, -7.0, 7.0, |x| one.push(x));
        assert_eq!(one, [2.0]);
    }

    /// The sine's maximum at π/2, listed once with its period, and its poles' family of tan.
    fn periodic() -> Graph {
        use crate::backend::Asymptote;
        let max = Mark { kind: MarkKind::Max, name: "H".into(), x: std::f32::consts::FRAC_PI_2, y: 1.0, period: Some(std::f32::consts::TAU) };
        let pole = Asymptote { line: Straight::Vertical(std::f32::consts::FRAC_PI_2), period: Some(std::f32::consts::PI) };
        Graph { points: vec![max], asymptotes: vec![pole], ..Graph::default() }
    }

    #[test]
    fn a_family_is_marked_in_any_window() {
        let area = Rectangle::new(Point::new(10, 50), Size::new(460, 260));
        let count = |window: Window| {
            let mut shapes = Vec::new();
            graph(&mut shapes, &Graph { window, ..periodic() }, area, Lang::De);
            let dots = shapes.iter().filter(|shape| matches!(shape, Shape::Circle(..))).count();
            let names = shapes.iter().filter(|shape| matches!(shape, Shape::Math(..))).count();
            (dots, names)
        };
        // Far from where it was listed: two maxima, 4π apart, without a name.
        assert_eq!(count(Window { x_min: 20.0, x_max: 30.0, y_min: -2.0, y_max: 2.0 }), (2, 0));
        assert_eq!(count(Window { x_min: -1.0, x_max: 2.0, y_min: -2.0, y_max: 2.0 }), (1, 1));
    }

    #[test]
    fn a_line_is_cut_to_the_area() {
        let plot = Plot { area: Rectangle::new(Point::new(0, 0), Size::new(101, 101)), window: Window { x_min: 0.0, x_max: 10.0, y_min: 0.0, y_max: 10.0 } };
        let lines = plot.polylines(&[(0.0, 5.0), (5.0, 5.0), (5.0, 20.0), (5.0, 30.0), (10.0, 5.0)]);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], [Point::new(0, 50), Point::new(50, 50), Point::new(50, 0)]);
        assert!(plot.across(Straight::Vertical(20.0)).is_none());
    }
}
