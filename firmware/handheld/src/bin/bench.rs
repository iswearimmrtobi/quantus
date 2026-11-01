//! Testing only: how fast the display takes full screens at the SPI clock it was built with
//! (`QUANTUS_SPI_MHZ`), and whether the picture stays clean. Flash it, then watch the display
//! and the serial monitor:
//!
//! 1. A test picture for three seconds: smooth gradients, single-pixel lines and colour bars.
//!    Wrong colours, shifted lines or noise mean the clock is too fast for the display.
//! 2. Thirty full screens of photo-like pixels, band by band as the renderer sends them.
//! 3. Thirty full redraws of the hui's mode screen, laid out and rendered on the board.
//!
//! Each round prints the milliseconds per screen and the frames per second, then starts again.

#[path = "../display.rs"]
#[macro_use]
mod display;

use std::time::Instant;

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::sys;
use quantus_ui::app::App;
use quantus_ui::backend::Stub;
use quantus_ui::key::Key;
use quantus_ui::render::{Frame, Renderer, BAND, HEIGHT, WIDTH};

const FRAMES: u32 = 30;

fn main() {
    sys::link_patches();
    let peripherals = Peripherals::take().expect("peripherals");
    let mut display = open_display!(peripherals);
    println!("bench: SPI at {} MHz, {}×{} pixels", display::SPI_MHZ, WIDTH, HEIGHT);

    let mut app = App::new(Stub);
    app.key(Key::Mode);
    let mut renderer = Renderer::new();

    loop {
        draw_bands(&mut display, test_picture);
        println!("bench: test picture, look for wrong colours, shifted lines or noise");
        FreeRtos::delay_ms(3000);

        let start = Instant::now();
        for frame in 0..FRAMES {
            draw_bands(&mut display, |x, y| photo(x, y, frame));
        }
        report("photo-like full screens", start);

        let start = Instant::now();
        for _ in 0..FRAMES {
            renderer.invalidate();
            renderer.render(&Frame::new(&app), &mut display).expect("render");
        }
        report("hui mode screen, laid out and drawn", start);
    }
}

fn report(what: &str, start: Instant) {
    let per_frame = start.elapsed().as_secs_f32() * 1000.0 / FRAMES as f32;
    println!("bench: {what}: {per_frame:.1} ms per screen, {:.1} fps", 1000.0 / per_frame);
}

/// The whole screen in bands of the renderer's height, each sent as one area.
fn draw_bands(display: &mut display::Display, color: impl Fn(u32, u32) -> Rgb565) {
    let mut band = Vec::with_capacity((WIDTH * BAND) as usize);
    for top in (0..HEIGHT).step_by(BAND as usize) {
        let height = BAND.min(HEIGHT - top);
        band.clear();
        band.extend((top..top + height).flat_map(|y| (0..WIDTH).map(move |x| (x, y))).map(|(x, y)| color(x, y)));
        let area = Rectangle::new(Point::new(0, top as i32), Size::new(WIDTH, height));
        display.fill_contiguous(&area, band.iter().copied()).expect("draw");
    }
}

/// Changes everywhere and from frame to frame, as a picture does: nothing for the display's
/// link to skip.
fn photo(x: u32, y: u32, frame: u32) -> Rgb565 {
    let t = frame * 7;
    Rgb565::new(((x + t) / 15 % 32) as u8, ((y * 2 + x + t) / 8 % 64) as u8, ((x ^ y) % 32) as u8)
}

/// Gradients in the top half; in the bottom half single-pixel stripes, a checkerboard and
/// colour bars, where a link that drops or shifts bits shows first.
fn test_picture(x: u32, y: u32) -> Rgb565 {
    let half = HEIGHT / 2;
    if y < half {
        let third = half / 3;
        let level = x * 32 / WIDTH;
        return match y / third {
            0 => Rgb565::new(level as u8, 0, 0),
            1 => Rgb565::new(0, (level * 2) as u8, 0),
            _ => Rgb565::new(0, 0, level as u8),
        };
    }
    let column = x * 6 / WIDTH;
    let on = match column {
        0 => x % 2 == 0,
        1 => y % 2 == 0,
        2 => (x + y) % 2 == 0,
        _ => true,
    };
    if !on {
        return Rgb565::BLACK;
    }
    match column {
        3 => Rgb565::YELLOW,
        4 => Rgb565::CYAN,
        5 => Rgb565::MAGENTA,
        _ => Rgb565::WHITE,
    }
}
