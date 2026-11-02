//! The top bar: the title, the busy spinner and the indicators.

use embedded_graphics::geometry::AngleUnit;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Arc, Circle, Rectangle};

use super::{colors, full_width, goes_back, rounded, Shape, BAR, MARGIN, WIDTH};
use crate::app::{App, Screen};
use crate::font::Scale;
use crate::settings::Angle;
use crate::text;

/// The top bar: the mode (or the menu's page) on the left, followed by a spinner while busy; on the right SHIFT and ALPHA while
/// they are on, the digits, exact or decimal, the angle unit, the WLAN and Bluetooth icons when
/// on, the exam mode and the battery.
pub(super) fn bar<B>(shapes: &mut Vec<Shape>, app: &App<B>) {
    shapes.push(Shape::Fill(full_width(0, BAR as u32), colors::CYAN_DIM));
    let baseline = BAR - 7;
    let title = match &app.screen {
        Screen::Menu(menu) => menu.title().get(app.settings.language),
        Screen::Calibrate(..) => text::CALIBRATE.get(app.settings.language),
        _ => &app.mode.name(app.settings.language),
    };
    let mut left = MARGIN;
    if goes_back(&app.screen) {
        // ‹ before the title: a tap on the left of the bar goes back.
        let (x, middle) = (MARGIN + 1, BAR / 2);
        let points = vec![Point::new(x + 6, middle - 6), Point::new(x, middle), Point::new(x + 6, middle + 6)];
        shapes.push(Shape::Path(points, colors::TEXT, 2));
        left += 16;
    }
    shapes.push(Shape::Text(Point::new(left, baseline), title.to_string(), Scale::Small, colors::TEXT));
    if app.busy {
        // Eight dots on a ring; the bright one moves on a step each tick, with a trail behind it.
        let center = Point::new(left + Scale::Small.width(title) + 18, BAR / 2);
        for dot in 0..8u8 {
            let angle = f32::from(dot) * core::f32::consts::FRAC_PI_4;
            let at = center + Point::new((7.0 * angle.cos()).round() as i32, (7.0 * angle.sin()).round() as i32);
            let color = match (app.spin + 8 - dot) % 8 {
                0 => colors::TEXT,
                1 => colors::MUTED,
                _ => colors::FAINT,
            };
            shapes.push(Shape::Circle(Circle::with_center(at, 4), color));
        }
    }

    // From the right edge inwards.
    let mut right = WIDTH as i32 - MARGIN;
    right -= battery(shapes, app.battery, app.charging, Point::new(right, BAR / 2)) + 12;
    let settings = &app.settings;
    let lang = settings.language;
    // The exam mode as a tag in the accent, the radios as icons; the angle unit, the digits and
    // the result form as text.
    if settings.exam {
        right -= tag(shapes, right, text::EXAM.get(lang), colors::AMBER) + 8;
    }
    if settings.wlan {
        right -= wlan(shapes, right, app.wlan_connected) + 10;
    }
    if settings.bluetooth {
        right -= bluetooth(shapes, right) + 10;
    }
    let angle = match settings.angle {
        Angle::Degrees => "D",
        Angle::Radians => "R",
    };
    let form = if settings.decimal { text::DECIMAL_SHORT } else { text::EXACT_SHORT };
    let digits = format!("Sig {}", settings.digits);
    for text in [angle, form.get(lang)].map(str::to_string).into_iter().chain([digits]) {
        let width = Scale::Small.width(&text);
        shapes.push(Shape::Text(Point::new(right - width, baseline), text, Scale::Small, colors::TEXT));
        right -= width + 12;
    }
    for (on, label, color) in [(app.alpha, "ALPHA", colors::ALPHA), (app.shift, "SHIFT", colors::SHIFT)] {
        if on {
            right -= tag(shapes, right, label, color) + 8;
        }
    }
}

/// A label on a coloured tag in the top bar, ending at `right`; returns its width.
fn tag(shapes: &mut Vec<Shape>, right: i32, label: &str, color: Rgb565) -> i32 {
    let width = Scale::Small.width(label);
    let tag = rounded(Point::new(right - width - 8, 4), Size::new(width as u32 + 8, BAR as u32 - 8), 3);
    shapes.push(Shape::Round(tag, color, None));
    let at = Point::new(right - width - 4, BAR - 7);
    shapes.push(Shape::Text(at, label.to_string(), Scale::Small, colors::BACKGROUND));
    width + 8
}

/// The WLAN icon, three arcs over a dot, ending at `right`, dim while not `connected`; returns
/// its width.
fn wlan(shapes: &mut Vec<Shape>, right: i32, connected: bool) -> i32 {
    let width = 18;
    let base = Point::new(right - width / 2, BAR - 6);
    let color = if connected { colors::TEXT } else { colors::FAINT };
    for diameter in [9, 17, 25] {
        let arc = Arc::with_center(base, diameter, (-135.0).deg(), 90.0.deg());
        shapes.push(Shape::Arc(arc, color, 2));
    }
    shapes.push(Shape::Circle(Circle::with_center(base, 4), color));
    width
}

/// The Bluetooth rune ending at `right`, centred in the bar; returns its width.
fn bluetooth(shapes: &mut Vec<Shape>, right: i32) -> i32 {
    let (width, top, bottom) = (9, 6, BAR - 6);
    let (x, quarter) = (right - width / 2 - 1, (bottom - top) / 4);
    let points = [
        (x - 4, top + quarter),
        (x + 4, bottom - quarter),
        (x, bottom),
        (x, top),
        (x + 4, top + quarter),
        (x - 4, bottom - quarter),
    ];
    shapes.push(Shape::Path(points.iter().map(|&(x, y)| Point::new(x, y)).collect(), colors::TEXT, 2));
    width
}

/// A battery icon ending at `right`, centred on `center.y`, filled by the charge, in green with a
/// bolt inside while charging; returns its width.
fn battery(shapes: &mut Vec<Shape>, charge: Option<u8>, charging: bool, right_center: Point) -> i32 {
    let (width, height) = (26, 14);
    let top_left = Point::new(right_center.x - width, right_center.y - height / 2);
    let nub = Rectangle::new(Point::new(right_center.x - 2, right_center.y - 3), Size::new(2, 6));
    shapes.push(Shape::Fill(nub, colors::TEXT));
    let body = Rectangle::new(top_left, Size::new(width as u32 - 2, height as u32));
    shapes.push(Shape::Fill(body, colors::TEXT));
    shapes.push(Shape::Fill(body.offset(-1), colors::BACKGROUND));
    if let Some(charge) = charge {
        let inner = body.offset(-2);
        let filled = inner.size.width * u32::from(charge.min(100)) / 100;
        let color = match (charging, charge < 15) {
            (true, _) => colors::ALPHA,
            (false, true) => colors::ERROR,
            (false, false) => colors::TEXT,
        };
        shapes.push(Shape::Fill(Rectangle::new(inner.top_left, Size::new(filled, inner.size.height)), color));
    }
    if charging {
        // The bolt in the middle, as pixels: per row, where it starts and how wide it is. A dark rim
        // around it lets it show on the fill and on the empty part alike.
        const BOLT: [(i32, u32); 8] = [(3, 2), (2, 2), (1, 2), (0, 6), (3, 2), (2, 2), (1, 2), (1, 1)];
        let origin = Point::new(right_center.x - width / 2 - 4, right_center.y - 4);
        let rim = [(-1, 0), (1, 0), (0, -1), (0, 1)];
        for ((dx, dy), color) in rim.iter().map(|&o| (o, colors::BACKGROUND)).chain([((0, 0), colors::TEXT)]) {
            for (row, &(start, len)) in BOLT.iter().enumerate() {
                let at = origin + Point::new(start + dx, row as i32 + dy);
                shapes.push(Shape::Fill(Rectangle::new(at, Size::new(len, 1)), color));
            }
        }
    }
    width
}
