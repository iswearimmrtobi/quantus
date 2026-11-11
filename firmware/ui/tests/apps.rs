//! Apps inside the hui's frame (firmware/docs/apps.md): the keys an app gets, its canvas, and an
//! applet written in Rust driven through the app.

use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

use quantus_ui::applet::{AppKey, Canvas, Palette};
use quantus_ui::key::Key;
use quantus_ui::render::{colors, Shape};

#[test]
fn keys_map_as_the_api_names_them() {
    let named = [
        (Key::Up, "up"),
        (Key::Down, "down"),
        (Key::Left, "left"),
        (Key::Right, "right"),
        (Key::Ok, "ok"),
        (Key::Exe, "exe"),
        (Key::Del, "del"),
        (Key::Digit(0), "0"),
        (Key::Digit(7), "7"),
        (Key::Plus, "+"),
        (Key::Minus, "-"),
        (Key::Times, "*"),
        (Key::Divide, "/"),
        (Key::Fraction, "/"),
        (Key::Equals, "="),
        (Key::Comma, "."),
        (Key::Char('.'), "."),
        (Key::Soft(0), "soft1"),
        (Key::Soft(11), "soft12"),
    ];
    for (key, name) in named {
        assert_eq!(AppKey::from_key(key).map(AppKey::name).as_deref(), Some(name), "{key:?}");
    }
    let none = [Key::Ac, Key::Mode, Key::Optn, Key::Shift, Key::Alpha, Key::Unit, Key::Symb, Key::Sin, Key::Letter('a'), Key::Calc];
    for key in none {
        assert_eq!(AppKey::from_key(key), None, "{key:?}");
    }
}

/// Where the shapes of the same calls land, on a canvas at `area`.
fn drawn(area: Rectangle) -> Vec<Rectangle> {
    let mut canvas = Canvas::new(area);
    canvas.rect(10, 20, 30, 40, colors::TEXT);
    canvas.circle(0, 0, 10, colors::TEXT);
    canvas.text(5, 5, "Hallo", colors::TEXT);
    canvas
        .shapes()
        .iter()
        .map(|shape| match shape {
            Shape::Fill(rect, _) => *rect,
            Shape::Circle(circle, _) => circle.bounding_box(),
            Shape::Text(at, ..) => Rectangle::new(*at, Size::zero()),
            other => panic!("unexpected {other:?}"),
        })
        .collect()
}

#[test]
fn the_canvas_moves_by_its_area() {
    let big = Rectangle::new(Point::new(0, 26), Size::new(480, 250));
    let small = Rectangle::new(Point::new(0, 26), Size::new(320, 214));
    for area in [big, small] {
        let shapes = drawn(area);
        assert_eq!(shapes[0], Rectangle::new(Point::new(10, 46), Size::new(30, 40)));
        assert_eq!(shapes[1], Rectangle::new(Point::new(0, 26), Size::new(10, 10)));
        // Text's top at y: its baseline lower by the font's ascent.
        assert!(shapes[2].top_left.y > 31 && shapes[2].top_left.x == 5);
    }
    let canvas = Canvas::new(small);
    assert_eq!((canvas.width(), canvas.height()), (320, 214));
}

#[test]
fn a_canvas_keeps_its_limit() {
    let mut canvas = Canvas::new(Rectangle::new(Point::zero(), Size::new(480, 294)));
    for _ in 0..quantus_ui::applet::MAX_SHAPES + 10 {
        canvas.rect(0, 0, 1, 1, colors::TEXT);
    }
    assert_eq!(canvas.shapes().len(), quantus_ui::applet::MAX_SHAPES);
    assert!(canvas.dropped());
    // A negative size is no size.
    let mut canvas = Canvas::new(Rectangle::new(Point::zero(), Size::new(10, 10)));
    canvas.rect(0, 0, -5, 3, colors::TEXT);
    assert!(matches!(canvas.shapes()[0], Shape::Fill(rect, _) if rect.size == Size::new(0, 3)));
}

#[test]
fn colours_by_name() {
    assert_eq!(Palette::get("background"), Some(colors::BACKGROUND));
    assert_eq!(Palette::get("tile1"), Some(colors::TILES[0].1));
    assert_eq!(Palette::get(&format!("tile{}", colors::TILES.len())), Some(colors::TILES[colors::TILES.len() - 1].1));
    assert_eq!(Palette::get("tile0"), None);
    assert_eq!(Palette::get(&format!("tile{}", colors::TILES.len() + 1)), None);
    assert_eq!(Palette::get("pink"), None);
}
