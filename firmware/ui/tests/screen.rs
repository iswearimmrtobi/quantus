use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use quantus_ui::app::{App, Mode, Screen};
use quantus_ui::backend::Stub;
use quantus_ui::editor::Editor;
use quantus_ui::font::Scale;
use quantus_ui::key::Key;
use quantus_ui::layout::{self, Kind};
use quantus_ui::render::{colors, Frame, Renderer, BAND, HEIGHT, WIDTH};
use quantus_ui::settings::WallpaperOn;
use quantus_ui::wallpaper::Wallpaper;

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

/// The signs the fonts lack are drawn as lines inside their item, in both sizes.
#[test]
fn the_drawn_signs_stay_inside_their_place() {
    for scale in [Scale::Normal, Scale::Small] {
        for c in layout::DRAWN.chars() {
            let row = layout::layout(&quantus_ui::editor::Row(vec![quantus_ui::editor::Node::Char('x'), quantus_ui::editor::Node::Char(c)]), scale);
            let item = &row.items[1];
            let Kind::Strokes { lines, .. } = &item.kind else { panic!("{c} is drawn") };
            for point in lines.iter().flatten() {
                assert!((0..item.width).contains(&point.x), "{c} {point:?} in {}", item.width);
                assert!((-item.ascent..=item.descent).contains(&point.y), "{c} {point:?}");
            }
        }
    }
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
    // Physik is selected; ↓ goes to the second row of tiles.
    press(&mut app, &[Key::Mode, Key::Down, Key::Exe]);
    assert_eq!((app.mode, &app.screen), (Mode::Analysis, &Screen::Calc));
    press(&mut app, &[Key::Mode, Key::Left, Key::Left, Key::Ok]);
    assert_eq!(app.mode, Mode::Algebra);
    press(&mut app, &[Key::Mode, Key::Digit(5)]);
    assert_eq!(app.mode, Mode::Linear);
    // ↓ from the second row goes to the next page.
    press(&mut app, &[Key::Mode, Key::Down, Key::Ok]);
    assert_eq!(app.mode, Mode::BaseN);
}

#[test]
fn every_mode_with_an_input_line_calculates() {
    for (digit, mode) in [(7, Mode::Matrix), (8, Mode::BaseN)] {
        let mut app = App::new(Stub);
        press(&mut app, &[Key::Mode, Key::Digit(digit), Key::Digit(1), Key::Exe]);
        assert_eq!(app.mode, mode);
        assert_eq!(app.shown_entry().unwrap().result.as_ref().map(|row| row.to_latex()), Ok("1".to_string()));
    }
}

#[test]
fn the_sessions_tile_opens_its_menu() {
    let mut app = App::new(Stub);
    press(&mut app, &[Key::Mode, Key::Right, Key::Right, Key::Right, Key::Right, Key::Right]);
    press(&mut app, &[Key::Down, Key::Down, Key::Down, Key::Ok]);
    assert!(matches!(app.screen, Screen::Menu(_)));
    assert_eq!(app.mode, Mode::Physics, "the mode stays");
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

#[test]
fn the_wallpaper_shows_where_the_setting_says() {
    let red = Rgb565::RED;
    let mut app = App::new(Stub);
    app.wallpaper = Wallpaper::new(vec![red; (WIDTH * HEIGHT) as usize]);
    let wallpaper = app.wallpaper.clone().unwrap();
    // Left of the first tile, and inside it.
    let margin = Point::new(4, 100);
    let tile = Point::new(60, 100);
    let shot = |app: &App<Stub>| {
        let mut screen = Screen565::default();
        Renderer::new().render(&Frame::new(app), &mut screen).unwrap();
        screen
    };

    press(&mut app, &[Key::Mode]);
    let modes = shot(&app);
    assert_eq!(modes.at(margin), red, "full strength behind the tiles");
    assert!(![red, colors::BACKGROUND].contains(&modes.at(tile)), "the tile lets it shine through");

    press(&mut app, &[Key::Ac]);
    assert_eq!(shot(&app).at(margin), colors::BACKGROUND, "not behind the calculation");
    app.settings.wallpaper_on = WallpaperOn::Everywhere;
    assert_eq!(shot(&app).at(margin), wallpaper.dim.get(0), "dimmed behind the calculation");

    app.wallpaper = None;
    assert_eq!(shot(&app).at(margin), colors::BACKGROUND);
}

#[test]
fn a_bmp_of_the_screen_size_is_read_and_others_are_not() {
    let wallpaper = Wallpaper::from_bmp(&bmp(WIDTH, HEIGHT)).unwrap();
    // The right way up: red at the top, blue at the bottom.
    let last = (WIDTH * HEIGHT) as usize - 1;
    assert_eq!((wallpaper.full.get(0), wallpaper.full.get(last)), (Rgb565::RED, Rgb565::BLUE));
    // The same picture from raw pixels, as a board keeps it in flash.
    let full: &'static [u8] = wallpaper.full.to_raw().leak();
    let dim: &'static [u8] = wallpaper.dim.to_raw().leak();
    let flash = Wallpaper::from_raw(full, dim).unwrap();
    assert_eq!((flash.full.get(0), flash.full.get(last), flash.dim.get(0)), (Rgb565::RED, Rgb565::BLUE, wallpaper.dim.get(0)));
    let mut row = [Rgb565::BLACK; 3];
    flash.full.copy_to(last - 2, &mut row);
    assert_eq!(row, [Rgb565::BLUE; 3]);
    assert!(Wallpaper::from_bmp(&bmp(100, 100)).is_none());
    assert!(Wallpaper::from_bmp(b"not a picture").is_none());
}

/// A 24-bit BMP, bottom-up as usual: red above, blue below.
fn bmp(width: u32, height: u32) -> Vec<u8> {
    let row = (width * 3).div_ceil(4) * 4;
    let size = 54 + row * height;
    let mut bytes = b"BM".to_vec();
    for value in [size, 0, 54, 40, width, height] {
        bytes.extend(value.to_le_bytes());
    }
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(24u16.to_le_bytes());
    for value in [0, row * height, 2835, 2835, 0, 0] {
        bytes.extend(value.to_le_bytes());
    }
    // From the bottom row up; the bytes are blue, green, red.
    for y in (0..height).rev() {
        let pixel = if y < height / 2 { [0, 0, 255] } else { [255, 0, 0] };
        let mut line: Vec<u8> = (0..width).flat_map(|_| pixel).collect();
        line.resize(row as usize, 0);
        bytes.extend(line);
    }
    bytes
}

#[test]
fn another_picture_with_the_same_shapes_is_drawn_again() {
    let mut app = App::new(Stub);
    press(&mut app, &[Key::Mode]);
    let mut renderer = Renderer::new();
    let mut display = Display::default();
    app.wallpaper = Wallpaper::new(vec![Rgb565::RED; (WIDTH * HEIGHT) as usize]);
    renderer.render(&Frame::new(&app), &mut display).unwrap();
    assert_eq!(renderer.render(&Frame::new(&app), &mut display).unwrap(), 0);
    // Same shapes, another picture: every band changes.
    app.wallpaper = Wallpaper::new(vec![Rgb565::BLUE; (WIDTH * HEIGHT) as usize]);
    assert_eq!(renderer.render(&Frame::new(&app), &mut display).unwrap(), HEIGHT.div_ceil(BAND) as usize);
}

/// Keeps the pixels the renderer sends.
struct Screen565(Vec<Rgb565>);

impl Default for Screen565 {
    fn default() -> Self {
        Screen565(vec![Rgb565::BLACK; (WIDTH * HEIGHT) as usize])
    }
}

impl Screen565 {
    fn at(&self, point: Point) -> Rgb565 {
        self.0[(point.y as u32 * WIDTH + point.x as u32) as usize]
    }
}

impl OriginDimensions for Screen565 {
    fn size(&self) -> Size {
        Size::new(WIDTH, HEIGHT)
    }
}

impl DrawTarget for Screen565 {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I: IntoIterator<Item = Pixel<Rgb565>>>(&mut self, pixels: I) -> Result<(), Self::Error> {
        for Pixel(point, color) in pixels {
            self.0[(point.y as u32 * WIDTH + point.x as u32) as usize] = color;
        }
        Ok(())
    }
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
    assert!(matches!(and.kind, Kind::Strokes { text: None, .. }), "the fonts have no ∧");
    assert!(and.width > Scale::Normal.metrics().digit / 2);
    assert_eq!(Scale::Normal.width("∧"), 0);
    assert!(Scale::Normal.width("∫") > 0);
}

#[test]
fn every_mode_name_fits_its_tile() {
    use quantus_ui::text::Lang;
    let room = (WIDTH as i32 - 2 * 10 - 2 * 8) / 3 - 18;
    for mode in Mode::ALL {
        for lang in [Lang::De, Lang::En] {
            for word in mode.text().get(lang).replace('\u{ad}', "-\u{ad}").split([' ', '\u{ad}']) {
                assert!(Scale::Small.width(word) <= room, "{word}");
            }
        }
    }
}
