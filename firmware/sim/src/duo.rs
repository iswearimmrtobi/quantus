//! The Duo (firmware/docs/duo.md) in the window: the 5" main screen on top at its 1280×720, the
//! 3.5" input screen under it at 480×320, and the hardware buttons beside that: the arrows with
//! OK on the left, MENU and EXE on the right. Both screens at one pixel per pixel, so the input
//! screen looks smaller than it is next to the denser main one.

use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle};
use embedded_graphics::text::{Alignment, Text};
use quantus_ui::app::App;
use quantus_ui::backend::Backend;
use quantus_ui::key::Key;
use quantus_ui::pad;
use quantus_ui::profile::Profile;
use quantus_ui::render::{self, Renderer};
use quantus_ui::touch::Touch;

/// The room between the two screens, and under the input screen.
const HINGE: i32 = 16;
/// The diameter of a hardware button.
const BUTTON: u32 = 56;

const BODY: Rgb565 = Rgb565::new(3, 6, 4);

/// What the mouse went down on.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Held {
    Main,
    Pad,
    Button(usize),
}

pub struct Duo {
    renderer: Renderer,
    buttons: Vec<(Rectangle, &'static str, Option<Key>)>,
    held: Option<Held>,
}

impl Duo {
    pub fn new() -> Duo {
        let (x, y) = (Self::pad_area().top_left.x, Self::pad_area().top_left.y);
        let size = Size::new(BUTTON, BUTTON);
        let at = |x: i32, y: i32| Rectangle::new(Point::new(x, y), size);
        let pitch = BUTTON as i32 + 8;
        // The arrows as a cross around OK, left of the input screen.
        let cross = Point::new(x / 2 - BUTTON as i32 / 2, y + pad::HEIGHT as i32 / 2 - BUTTON as i32 / 2);
        // MENU and EXE, and the power button, right of it.
        let right = x + pad::WIDTH as i32 + (x - BUTTON as i32) / 2;
        let buttons = vec![
            (at(cross.x, cross.y - pitch), "UP", Some(Key::Up)),
            (at(cross.x - pitch, cross.y), "LEFT", Some(Key::Left)),
            (at(cross.x, cross.y), "OK", Some(Key::Ok)),
            (at(cross.x + pitch, cross.y), "RIGHT", Some(Key::Right)),
            (at(cross.x, cross.y + pitch), "DOWN", Some(Key::Down)),
            (at(right, y + 24), "MENU", Some(Key::Mode)),
            (at(right, y + 24 + pitch), "EXE", Some(Key::Exe)),
            (at(right, y + pad::HEIGHT as i32 - BUTTON as i32 - 8), "ON", None),
        ];
        Duo { renderer: Renderer::new(), buttons, held: None }
    }

    /// The window: both screens one under the other, the buttons beside the lower one.
    pub fn window() -> Size {
        let main = Profile::DUO.physical();
        Size::new(main.width, main.height + HINGE as u32 + pad::HEIGHT + HINGE as u32)
    }

    pub fn main_area() -> Rectangle {
        Rectangle::new(Point::zero(), Profile::DUO.physical())
    }

    pub fn pad_area() -> Rectangle {
        let main = Profile::DUO.physical();
        let x = (main.width - pad::WIDTH) as i32 / 2;
        Rectangle::new(Point::new(x, main.height as i32 + HINGE), Size::new(pad::WIDTH, pad::HEIGHT))
    }

    /// The input screen and the buttons; the main screen is the caller's.
    pub fn draw<D: DrawTarget<Color = Rgb565>, B>(&mut self, display: &mut D, app: &App<B>) {
        let frame = render::pad::frame(app);
        let mut pad = display.cropped(&Self::pad_area());
        let _ = self.renderer.render_on(&frame, &mut pad, Profile::HANDHELD);
        let label = MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE);
        for (index, (rect, name, key)) in self.buttons.iter().enumerate() {
            let lit = self.held == Some(Held::Button(index));
            let fill = match (lit, key) {
                (true, _) => Rgb565::new(12, 26, 14),
                (false, Some(Key::Exe)) => Rgb565::new(7, 18, 11),
                (false, Some(_)) => Rgb565::new(8, 18, 10),
                (false, None) => Rgb565::new(5, 10, 6),
            };
            let style = PrimitiveStyleBuilder::new().fill_color(fill).stroke_color(Rgb565::new(2, 4, 2)).stroke_width(1).build();
            let _ = Circle::new(rect.top_left, BUTTON).into_styled(style).draw(display);
            let _ = Text::with_alignment(name, rect.center() + Point::new(0, 3), label, Alignment::Center).draw(display);
        }
    }

    /// Clears the window's body around the screens.
    pub fn clear<D: DrawTarget<Color = Rgb565>>(display: &mut D) {
        let _ = Rectangle::new(Point::zero(), Self::window()).into_styled(PrimitiveStyle::with_fill(BODY)).draw(display);
    }

    /// The mouse went down at `point` in the window. Returns whether something changed.
    pub fn press<B: Backend>(&mut self, app: &mut App<B>, point: Point) -> bool {
        if Self::main_area().contains(point) {
            self.held = Some(Held::Main);
            return app.touch(Touch::Press(Profile::DUO.logical(point)));
        }
        if Self::pad_area().contains(point) {
            self.held = Some(Held::Pad);
            return app.pad_touch(Touch::Press(point - Self::pad_area().top_left));
        }
        let Some(index) = self.buttons.iter().position(|(rect, ..)| Circle::new(rect.top_left, BUTTON).contains(point)) else {
            return false;
        };
        self.held = Some(Held::Button(index));
        if let Some(key) = self.buttons[index].2 {
            app.key(key);
        }
        true
    }

    pub fn drag<B: Backend>(&mut self, app: &mut App<B>, point: Point) -> bool {
        match self.held {
            Some(Held::Main) => app.touch(Touch::Drag(Profile::DUO.logical(point))),
            Some(Held::Pad) => app.pad_touch(Touch::Drag(point - Self::pad_area().top_left)),
            _ => false,
        }
    }

    pub fn release<B: Backend>(&mut self, app: &mut App<B>) -> bool {
        match self.held.take() {
            Some(Held::Main) => app.touch(Touch::Release),
            Some(Held::Pad) => app.pad_touch(Touch::Release),
            Some(Held::Button(_)) => true,
            None => false,
        }
    }
}
