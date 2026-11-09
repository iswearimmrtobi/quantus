//! The labels of the soft keys: two strips at the bottom of the screen, the upper one for the
//! row of keys nearer the screen, each label above its key (firmware/docs/hardware.md §4).

use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

use super::{colors, Shape, HEIGHT, WIDTH};
use crate::app::App;
use crate::font::Scale;
use crate::layout;
use crate::soft::{self, Bar, Slot};
use crate::touch::Target;

/// The height of one strip.
const STRIP: i32 = 22;
/// The height of both: what the screen above gives up while they show.
pub const SOFT: i32 = 2 * STRIP;

/// The second row's tint, a little darker than the first's, as on the keycaps.
const LOWER: embedded_graphics::pixelcolor::Rgb565 = colors::rgb(0x22, 0x20, 0x2d);

/// Soft key `index`'s label on the screen, of a bar with `columns` keys to a row.
pub fn slot_rect(index: usize, columns: usize) -> Rectangle {
    let (row, column) = ((index / columns) as i32, (index % columns) as i32);
    let columns = columns as i32;
    let left = column * WIDTH as i32 / columns;
    let right = (column + 1) * WIDTH as i32 / columns;
    let top = HEIGHT as i32 - SOFT + row * STRIP;
    Rectangle::new(Point::new(left, top), Size::new((right - left) as u32, STRIP as u32))
}

/// The soft key under `point`, if the strips show and it is on them.
pub fn soft_at<B>(app: &App<B>, point: Point) -> Option<Target> {
    let bar = soft::bar(app)?;
    let index = (0..2 * bar.columns).find(|&index| slot_rect(index, bar.columns).contains(point))?;
    Some(Target::Soft(index))
}

pub(super) fn strips<B>(shapes: &mut Vec<Shape>, app: &App<B>, bar: &Bar) {
    let top = HEIGHT as i32 - SOFT;
    shapes.push(Shape::Fill(Rectangle::new(Point::new(0, top), Size::new(WIDTH, STRIP as u32)), colors::SURFACE));
    shapes.push(Shape::Fill(Rectangle::new(Point::new(0, top + STRIP), Size::new(WIDTH, STRIP as u32)), LOWER));
    let lang = app.settings.language;
    for index in 0..2 * bar.columns {
        let area = slot_rect(index, bar.columns);
        let slot = bar.slot(index);
        let lit = match slot {
            Some(Slot::Base(_, value)) => soft::is_set(&app.settings, *value),
            _ => false,
        };
        if app.pressed() == Some(Target::Soft(index)) {
            shapes.push(Shape::Fill(area, colors::CYAN_DIM));
        } else if lit {
            shapes.push(Shape::Fill(area, colors::AMBER_DIM));
        }
        // A hairline between the keys of a row.
        if index % bar.columns != 0 {
            shapes.push(Shape::Fill(Rectangle::new(area.top_left + Point::new(0, 4), Size::new(1, STRIP as u32 - 8)), colors::HAIRLINE));
        }
        let label = bar.label(index, lang);
        if label.is_empty() {
            continue;
        }
        let color = match slot {
            _ if bar.is_pending(index) => colors::FAINT,
            _ if lit => colors::AMBER,
            Some(Slot::Page) => colors::MUTED,
            Some(Slot::Unit(_) | Slot::Convert(_)) => colors::RESULT,
            _ => colors::TEXT,
        };
        // A symbol of SYMB in the normal size: the small font's Greek letters are tiny.
        let scale = if matches!(slot, Some(Slot::Symbol(..))) { Scale::Normal } else { Scale::Small };
        let row = layout::layout(&crate::latex::parse(&label), scale);
        let x = area.top_left.x + ((area.size.width as i32 - row.width) / 2).max(1);
        let baseline = area.top_left.y + (STRIP + scale.metrics().digit) / 2;
        shapes.push(Shape::Math(Point::new(x, baseline), row, color));
    }
    shapes.push(Shape::Fill(Rectangle::new(Point::new(0, top), Size::new(WIDTH, 1)), colors::HAIRLINE));
}
