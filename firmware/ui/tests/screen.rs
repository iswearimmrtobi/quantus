use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use quantus_ui::app::{App, Mode};
use quantus_ui::backend::Stub;
use quantus_ui::editor::Editor;
use quantus_ui::font::Scale;
use quantus_ui::key::Key;
use quantus_ui::layout::{self, Kind};
use quantus_ui::render::{Frame, Renderer, BAND, HEIGHT, WIDTH};

fn press(app: &mut App<Stub>, keys: &[Key]) {
    keys.iter().for_each(|&key| app.key(key));
}

#[test]
fn a_fraction_is_taller_and_an_exponent_higher_than_a_digit() {
    let digit = layout::layout(&quantus_ui::latex::parse("2"), Scale::Normal);
    let fraction = layout::layout(&quantus_ui::latex::parse(r"\frac{1}{2}"), Scale::Normal);
    let power = layout::layout(&quantus_ui::latex::parse("2^{3}"), Scale::Normal);
    let nested = layout::layout(&quantus_ui::latex::parse(r"\frac{\frac{1}{2}}{3}"), Scale::Normal);
    assert!(fraction.ascent > digit.ascent && fraction.descent > digit.descent);
    assert!(power.ascent > digit.ascent);
    assert!(nested.height() > fraction.height());
    assert!(matches!(fraction.items[0].kind, Kind::Bar(_)));
    // Inside a fraction, fractions are small.
    assert_eq!(nested.items[0].slots[0].1.items[0].slots[0].1.scale, Scale::Small);
}

#[test]
fn the_caret_follows_the_cursor_into_a_denominator() {
    let mut editor = Editor::new();
    editor.char('1');
    editor.fraction();
    editor.char('2');
    let row = layout::layout(&editor.row, Scale::Normal);
    let caret = layout::caret(&row, Point::new(0, 100), editor.path(), editor.pos());
    assert!(caret.top_left.y > 100 - Scale::Normal.metrics().ascent, "below the bar");
    assert!(caret.top_left.x > 0);
}

#[test]
fn exe_moves_the_input_into_the_history() {
    let mut app = App::new(Stub);
    press(&mut app, &[Key::Digit(3), Key::Fraction, Key::Digit(4), Key::Right, Key::Plus, Key::Digit(1), Key::Exe]);
    assert!(app.editor.is_empty());
    let entry = &app.history[0];
    assert_eq!(entry.input.to_latex(), r"\frac{3}{4}+1");
    assert_eq!(entry.result.as_ref().unwrap().to_latex(), r"\frac{3}{4}+1");

    press(&mut app, &[Key::Letter('?'), Key::Exe]);
    assert!(app.history[1].result.is_err());

    // ↑ brings inputs back, ↓ goes forward again.
    press(&mut app, &[Key::Up, Key::Up]);
    assert_eq!(app.editor.to_latex(), r"\frac{3}{4}+1");
    press(&mut app, &[Key::Down, Key::Down]);
    assert!(app.editor.is_empty());

    press(&mut app, &[Key::Ans]);
    assert!(app.editor.is_empty(), "the last result was an error");
}

#[test]
fn the_mode_menu_switches_the_mode() {
    let mut app = App::new(Stub);
    press(&mut app, &[Key::Mode, Key::Down, Key::Down, Key::Exe]);
    assert_eq!((app.mode, app.menu), (Mode::Analysis, None));
    press(&mut app, &[Key::Mode, Key::Digit(1)]);
    assert_eq!(app.mode, Mode::Algebra);
}

#[test]
fn only_changed_bands_are_sent() {
    let mut app = App::new(Stub);
    let mut renderer = Renderer::new();
    let mut display = Screen::default();
    let bands = HEIGHT.div_ceil(BAND) as usize;
    assert_eq!(renderer.render(&Frame::new(&app), &mut display).unwrap(), bands);
    assert_eq!(renderer.render(&Frame::new(&app), &mut display).unwrap(), 0);

    // Typing changes the input at the bottom only.
    app.key(Key::Digit(7));
    let sent = renderer.render(&Frame::new(&app), &mut display).unwrap();
    assert!((1..=2).contains(&sent), "{sent} bands");

    // Each band goes out whole, one pixel per point of its area.
    let (area, pixels) = display.writes.last().unwrap();
    assert_eq!((area.size.width, *pixels), (WIDTH, (WIDTH * area.size.height) as usize));
}

/// Records what the renderer sends.
#[derive(Default)]
struct Screen {
    writes: Vec<(Rectangle, usize)>,
}

impl OriginDimensions for Screen {
    fn size(&self) -> Size {
        Size::new(WIDTH, HEIGHT)
    }
}

impl DrawTarget for Screen {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I: IntoIterator<Item = Pixel<Rgb565>>>(&mut self, _: I) -> Result<(), Self::Error> {
        Ok(())
    }

    fn fill_contiguous<I: IntoIterator<Item = Rgb565>>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error> {
        self.writes.push((*area, colors.into_iter().count()));
        Ok(())
    }
}
