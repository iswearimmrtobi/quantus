use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use quantus_ui::app::{App, Mode, Screen};
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
fn exe_shows_the_result_and_up_steps_through_the_history() {
    let mut app = App::new(Stub);
    press(&mut app, &[Key::Digit(3), Key::Fraction, Key::Digit(4), Key::Right, Key::Plus, Key::Digit(1), Key::Exe]);
    let entry = app.shown_entry().unwrap();
    assert_eq!(entry.input.to_latex(), r"\frac{3}{4}+1");
    assert_eq!(entry.result.as_ref().unwrap().to_latex(), r"\frac{3}{4}+1");
    assert_eq!(app.editor.to_latex(), r"\frac{3}{4}+1", "the input stays on the screen");

    // A digit starts afresh.
    press(&mut app, &[Key::Digit(2), Key::Letter('?'), Key::Exe]);
    assert!(app.shown_entry().unwrap().result.is_err());

    // ↑ brings the earlier calculation back with its result, ↓ goes forward to an empty input.
    press(&mut app, &[Key::Up]);
    assert_eq!((app.shown, app.editor.to_latex()), (Some(1), r"\frac{3}{4}+1".to_string()));
    press(&mut app, &[Key::Down, Key::Down]);
    assert!(app.editor.is_empty() && app.shown.is_none());

    press(&mut app, &[Key::Ans]);
    assert!(app.editor.is_empty(), "the last result was an error");
}

#[test]
fn an_operator_after_a_result_goes_on_with_it() {
    let mut app = App::new(Stub);
    press(&mut app, &[Key::Digit(7), Key::Exe, Key::Times, Key::Digit(2)]);
    assert_eq!((app.shown, app.editor.to_latex()), (None, r"7\cdot2".to_string()));

    // ← edits the input again.
    press(&mut app, &[Key::Exe, Key::Left, Key::Digit(1)]);
    assert_eq!(app.editor.to_latex(), r"7\cdot21");
}

#[test]
fn swap_form_writes_the_result_as_a_decimal_and_back() {
    let mut app = App::new(Stub);
    press(&mut app, &[Key::Digit(5), Key::Exe, Key::SwapForm]);
    let entry = app.shown_entry().unwrap();
    assert!(entry.decimal);
    assert_eq!(entry.result.as_ref().unwrap().to_latex(), r"\approx5");
    press(&mut app, &[Key::SwapForm]);
    assert!(!app.shown_entry().unwrap().decimal);
    assert!(!app.settings.decimal, "the setting stays");
}

#[test]
fn the_mode_menu_switches_the_mode() {
    let mut app = App::new(Stub);
    // Physics is selected; ↓ goes to the second row of tiles.
    press(&mut app, &[Key::Mode, Key::Down, Key::Exe]);
    assert_eq!((app.mode, app.screen), (Mode::Linear, Screen::Calc));
    press(&mut app, &[Key::Mode, Key::Left, Key::Left, Key::Ok]);
    assert_eq!(app.mode, Mode::Imaginary);
    press(&mut app, &[Key::Mode, Key::Digit(1)]);
    assert_eq!(app.mode, Mode::Algebra);
}

#[test]
fn shift_mode_opens_the_setup() {
    let mut app = App::new(Stub);
    press(&mut app, &[Key::Shift, Key::Mode]);
    assert_eq!(app.screen, Screen::Setup { selected: 0 });
    press(&mut app, &[Key::Left, Key::Left, Key::Down, Key::Right, Key::Ac]);
    assert_eq!((app.settings.digits, app.settings.decimal), (8, true));
    assert_eq!(app.screen, Screen::Calc);
}

#[test]
fn only_changed_bands_are_sent() {
    let mut app = App::new(Stub);
    let mut renderer = Renderer::new();
    let mut display = Display::default();
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
struct Display {
    writes: Vec<(Rectangle, usize)>,
}

impl OriginDimensions for Display {
    fn size(&self) -> Size {
        Size::new(WIDTH, HEIGHT)
    }
}

impl DrawTarget for Display {
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

#[test]
fn brackets_grow_around_a_fraction_but_not_around_digits() {
    let parens = |latex: &str| {
        let row = layout::layout(&quantus_ui::latex::parse(latex), Scale::Normal);
        let grown: Vec<_> = row.items.iter().filter(|item| matches!(item.kind, Kind::Paren { .. })).map(|item| item.ascent).collect();
        (row, grown)
    };
    assert!(parens("(1+2)").1.is_empty());
    assert!(parens("(2^{3})").1.is_empty(), "an exponent alone keeps them");
    let (row, grown) = parens(r"(\frac{3}{4}+1)^{2}");
    assert_eq!(grown.len(), 2);
    assert!(grown[0] > Scale::Normal.metrics().ascent);
    // The exponent sits above the grown bracket.
    let exponent = row.items.last().unwrap();
    assert!(exponent.ascent > grown[1]);
    // A bracket not yet closed reaches to the end of the input.
    assert_eq!(parens(r"2(\frac{1}{2}").1.len(), 1);
}

#[test]
fn and_is_drawn_as_lines_and_the_integral_comes_from_the_font() {
    let row = layout::layout(&quantus_ui::latex::parse(r"x = 2 \land y = 1"), Scale::Normal);
    let and = &row.items[3];
    assert!(matches!(and.kind, Kind::Sign(_)), "the fonts have no ∧");
    assert!(and.width > Scale::Normal.metrics().digit / 2);
    assert_eq!(Scale::Normal.width("∧"), 0);
    assert!(Scale::Normal.width("∫") > 0);
}
