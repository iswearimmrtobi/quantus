//! The handheld's firmware on a classic ESP32 with the 3.5" ILI9486 display, until the ESP32-P4
//! arrives: the UI of `quantus-ui` with a stub instead of the engine. See the README.

mod display;

use std::time::Instant;

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::PinDriver;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::spi::config::{Config, DriverConfig};
use esp_idf_svc::hal::spi::{Dma, SpiDeviceDriver};
use esp_idf_svc::hal::units::FromValueType;
use esp_idf_svc::sys;
use quantus_ui::app::App;
use quantus_ui::backend::Stub;
use quantus_ui::font::Scale;
use quantus_ui::render::{Frame, Renderer, WIDTH};

use display::Display;

/// The SPI clock. The shift registers manage more; raise it once the picture is stable.
const SPI_MHZ: u32 = 16;

fn main() {
    sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    let start = Instant::now();

    // Pins: VSPI on a WROOM; see the README for the wiring.
    let peripherals = Peripherals::take().expect("peripherals");
    let pins = peripherals.pins;
    let bus = DriverConfig::new().dma(Dma::Auto(4096));
    let config = Config::new().baudrate(SPI_MHZ.MHz().into());
    let spi = SpiDeviceDriver::new_single(
        peripherals.spi3,
        pins.gpio18,       // SCLK
        pins.gpio23,       // MOSI
        Some(pins.gpio19), // MISO, for the touch controller later
        Some(pins.gpio5),  // LCD_CS
        &bus,
        &config,
    )
    .expect("spi");
    let dc = PinDriver::output(pins.gpio21).expect("LCD_RS");
    let reset = PinDriver::output(pins.gpio22).expect("RST");
    // The touch controller shares the bus and must stay deselected.
    let mut touch_cs = PinDriver::output(pins.gpio4).expect("TP_CS");
    touch_cs.set_high().expect("TP_CS");

    let mut display = Display::new(spi, dc, reset).expect("display");
    println!("display ready after {} ms", start.elapsed().as_millis());

    test_pattern(&mut display);
    FreeRtos::delay_ms(1500);

    let app = App::new(Stub);
    let mut renderer = Renderer::new();
    let drawn = Instant::now();
    let bands = renderer.render(&Frame::new(&app), &mut display).expect("render");
    println!("first frame: {bands} bands in {} ms, {} ms after boot", drawn.elapsed().as_millis(), start.elapsed().as_millis());
    loop {
        FreeRtos::delay_ms(1000);
    }
}

/// Red, green, blue and white bars with their names: colours and orientation at a glance.
fn test_pattern(display: &mut Display) {
    let bars = [(Rgb565::RED, "red"), (Rgb565::GREEN, "green"), (Rgb565::BLUE, "blue"), (Rgb565::WHITE, "white")];
    let width = WIDTH / bars.len() as u32;
    for (index, (color, name)) in bars.into_iter().enumerate() {
        let x = (index as u32 * width) as i32;
        let bar = Rectangle::new(Point::new(x, 0), Size::new(width, 280));
        let _ = bar.into_styled(PrimitiveStyle::with_fill(color)).draw(display);
        let label = Rectangle::new(Point::new(x, 280), Size::new(width, 40));
        let _ = label.into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK)).draw(display);
        Scale::Small.draw(name, Point::new(x + 10, 306), Rgb565::WHITE, display);
    }
    // The top left corner, to see where (0, 0) is.
    let corner = Rectangle::new(Point::zero(), Size::new(20, 20));
    let _ = corner.into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK)).draw(display);
}
