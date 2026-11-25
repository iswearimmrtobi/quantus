//! The Duo's input screen drawn: each key a rounded button with its label, or on a layer its
//! legend in the layer's colour; the soft keys' labels as the strips of the main screen show
//! them.

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Rectangle, RoundedRectangle};

use super::{colors, fit, Frame, Shape};
use crate::app::App;
use crate::font::Scale;
use crate::layer::Layer;
use crate::layout;
use crate::legend::legend;
use crate::pad::{Action, PadKey, Style, CONTROL, LABEL_ROOM};

/// The app's pad as it is now, to draw with a [`super::Renderer`] on the pad's own 480×320
/// panel ([`super::Renderer::render_on`] with [`crate::profile::Profile::HANDHELD`]): its keys,
/// or its calibration screen. Nothing without a pad.
pub fn frame<B>(app: &App<B>) -> Frame {
    let mut shapes = Vec::new();
    let Some(pad) = &app.pad else { return Frame(shapes, None) };
    if let Some(state) = &pad.calibrating {
        let lang = app.settings.language;
        let size = Size::new(crate::pad::WIDTH, crate::pad::HEIGHT);
        super::screens::calibration_on(&mut shapes, state, size, Some(crate::text::CALIBRATE_PAD.get(lang)), lang);
        return Frame(shapes, None);
    }
    let layer = app.modifier.layer();
    for (index, key) in pad.keys(app).iter().enumerate() {
        button(&mut shapes, key, pad.pressed == Some(index), app, layer);
    }
    Frame(shapes, None)
}

fn button<B>(shapes: &mut Vec<Shape>, key: &PadKey, pressed: bool, app: &App<B>, layer: Layer) {
    let round = RoundedRectangle::with_equal_corners(key.rect, Size::new(6, 6));
    let (fill, text) = match key.style {
        Style::Soft => (colors::lower(), colors::text()),
        Style::Function => (colors::surface(), colors::text()),
        Style::Digit | Style::Operator => (colors::hairline(), colors::text()),
        Style::Shift => (colors::surface(), colors::shift()),
        Style::Alpha => (colors::surface(), colors::alpha()),
        Style::Unit => (colors::surface(), colors::unit()),
        Style::Exe => (colors::primary_dim(), colors::text()),
        Style::Delete => (colors::error_dim(), colors::text()),
    };
    let fill = if pressed { colors::primary() } else { fill };
    // SHIFT, ALPHA and UNIT outlined in their colour while they are pending or locked.
    let own = match key.style {
        Style::Shift => Some(Layer::Shift),
        Style::Alpha => Some(Layer::Alpha),
        Style::Unit => Some(Layer::Unit),
        _ => None,
    };
    let modifier = &app.modifier;
    let outline = own.filter(|own| modifier.pending == Some(*own) || modifier.lock == Some(*own)).map(|_| text);
    shapes.push(Shape::Round(round, fill, outline));
    if key.label.is_empty() {
        return;
    }
    if key.style == Style::Soft {
        let color = if key.action == Action::None { colors::faint() } else { colors::text() };
        return centred_math(shapes, key.rect, &key.label, Scale::Small, color);
    }
    // On a layer, the key's legend there; a key with none is faint.
    let (label, color) = match legend(&key.label, layer) {
        Some((legend, works)) => {
            let color = match layer {
                Layer::Shift => colors::shift(),
                Layer::Alpha => colors::alpha(),
                _ => colors::unit(),
            };
            (legend.to_string(), if works { color } else { colors::faint() })
        }
        None if layer != Layer::Base && own.is_none() => (key.label.clone(), colors::faint()),
        None if key.action == Action::None => (key.label.clone(), colors::faint()),
        None => (key.label.clone(), text),
    };
    let scale = match key.style {
        Style::Digit | Style::Operator => Scale::Large,
        // The row of SHIFT, ALPHA, ...: words, all in one size.
        Style::Shift | Style::Alpha | Style::Unit => Scale::Small,
        _ if CONTROL.contains(&key.label.as_str()) => Scale::Small,
        _ => Scale::Normal,
    };
    match math_of(&label) {
        Some(latex) => centred_math(shapes, key.rect, latex, scale, color),
        None => {
            let room = key.rect.size.width as i32 - 2 * LABEL_ROOM;
            // One character keeps its size: the operators' glyphs are wide only by their margins.
            let scale = match label.chars().count() {
                1 => scale,
                _ => [scale, Scale::Small, Scale::Tiny].into_iter().find(|scale| scale.width(&label) <= room).unwrap_or(Scale::Tiny),
            };
            let label = if label.chars().count() == 1 { label } else { fit(&label, room, scale) };
            let x = key.rect.top_left.x + (key.rect.size.width as i32 - scale.width(&label)) / 2;
            let y = key.rect.top_left.y + (key.rect.size.height as i32 + scale.metrics().digit) / 2;
            shapes.push(Shape::Text(Point::new(x, y), label, scale, color));
        }
    }
}

/// LaTeX laid out in the middle of `rect`, in the small size if it is too wide.
fn centred_math(shapes: &mut Vec<Shape>, rect: Rectangle, latex: &str, scale: Scale, color: Rgb565) {
    let row = crate::latex::parse(latex);
    let mut laid = layout::layout(&row, scale);
    let room = rect.size.width as i32 - 2 * LABEL_ROOM;
    for smaller in [Scale::Small, Scale::Tiny] {
        if laid.width > room && scale != smaller {
            laid = layout::layout(&row, smaller);
        }
    }
    let x = rect.top_left.x + ((rect.size.width as i32 - laid.width) / 2).max(2);
    let y = rect.top_left.y + (rect.size.height as i32 + laid.ascent - laid.descent) / 2;
    shapes.push(Shape::Math(Point::new(x, y), laid, color));
}

/// The labels that read better as maths than as the font's characters.
fn math_of(label: &str) -> Option<&'static str> {
    Some(match label {
        "a/b" => r"\frac{a}{b}",
        "a b/c" => r"a\frac{b}{c}",
        "√" => r"\sqrt{x}",
        "n√" => r"\sqrt[n]{x}",
        "x²" => "x^{2}",
        "x³" => "x^{3}",
        "xʸ" => "x^{y}",
        "x⁻¹" => "x^{-1}",
        "10ˣ" => "10^{x}",
        "×10ˣ" => r"\times10^{x}",
        "eˣ" => "e^{x}",
        "logₐb" => r"\log_{a}b",
        "sin⁻¹" => r"\sin^{-1}",
        "cos⁻¹" => r"\cos^{-1}",
        "tan⁻¹" => r"\tan^{-1}",
        "d/dx" => r"\frac{d}{dx}",
        "∫" => r"\int",
        "Σ" => r"\Sigma",
        "Π" => r"\Pi",
        "π" => r"\pi",
        "S⇔D" => r"S\Leftrightarrow D",
        "|x|" => r"\left|x\right|",
        _ => return None,
    })
}
