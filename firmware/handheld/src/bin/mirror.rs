//! Testing only: the board as the screen of the simulator on the PC (`cargo run -- --mirror
//! <port>` in `firmware/sim`), which runs the UI and the real engine and sends each band that
//! changed over serial (see `quantus-mirror`). The normal firmware is `src/main.rs`.

#[path = "../display.rs"]
#[macro_use]
mod display;

use embedded_graphics::pixelcolor::raw::RawU16;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use esp_idf_svc::hal::delay::{FreeRtos, BLOCK};
use esp_idf_svc::hal::gpio::AnyIOPin;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::uart::{config, UartDriver};
use esp_idf_svc::hal::units::Hertz;
use esp_idf_svc::sys;
use quantus_mirror::Decoder;
use quantus_ui::render::{BAND, WIDTH};

use quantus_mirror::BAUD;

fn main() {
    sys::link_patches();
    let peripherals = Peripherals::take().expect("peripherals");
    let (mut display, _touch) = open_display!(peripherals);
    let black = Rectangle::new(Point::zero(), display.size());
    display.fill_solid(&black, Rgb565::BLACK).expect("clear");

    // The console's UART, faster: the boot log above went out at 115200.
    let config = config::Config::new().baudrate(Hertz(BAUD)).rx_fifo_size(4096);
    let uart = UartDriver::new(
        peripherals.uart0,
        peripherals.pins.gpio1,
        peripherals.pins.gpio3,
        Option::<AnyIOPin>::None,
        Option::<AnyIOPin>::None,
        &config,
    )
    .expect("uart");
    FreeRtos::delay_ms(50);
    // Tells the simulator to send the whole screen.
    uart.write(b"\nREADY\n").expect("uart");

    let mut decoder = Decoder::new((WIDTH * BAND) as usize);
    let mut buffer = [0u8; 1024];
    loop {
        let read = uart.read(&mut buffer, BLOCK).unwrap_or(0);
        for &byte in &buffer[..read] {
            if let Some(frame) = decoder.push(byte) {
                let area = Rectangle::new(
                    Point::new(frame.x.into(), frame.y.into()),
                    Size::new(frame.width.into(), frame.height.into()),
                );
                let colors = frame.pixels.into_iter().map(|raw| Rgb565::from(RawU16::new(raw)));
                let _ = display.fill_contiguous(&area, colors);
            }
        }
    }
}
