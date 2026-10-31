//! The handheld's firmware on a classic ESP32 with the 3.5" ILI9486 display, until the ESP32-P4
//! arrives: the UI of `quantus-ui` with a stub instead of the engine. See the README.

mod display;

use std::io::Read;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

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
use quantus_ui::app::{App, Mode};
use quantus_ui::backend::{Backend, Stub};
use quantus_ui::font::Scale;
use quantus_ui::key::Key;
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

    let mut app = App::new(Logged(Stub));
    let mut renderer = Renderer::new();
    let drawn = Instant::now();
    let bands = renderer.render(&Frame::new(&app), &mut display).expect("render");
    println!("first frame: {bands} bands in {} ms, {} ms after boot", drawn.elapsed().as_millis(), start.elapsed().as_millis());
    println!("Type keys here; the README lists them.");

    let (keys, received) = mpsc::channel();
    std::thread::Builder::new().stack_size(8 * 1024).spawn(move || read_keys(keys)).expect("key thread");
    loop {
        let changed = match received.recv_timeout(Duration::from_millis(50)) {
            Ok(key) => {
                app.key(key);
                true
            }
            Err(RecvTimeoutError::Timeout) => app.tick(start.elapsed().as_millis() as u64),
            Err(RecvTimeoutError::Disconnected) => return,
        };
        if changed {
            let drawn = Instant::now();
            let bands = renderer.render(&Frame::new(&app), &mut display).expect("render");
            if bands > 1 {
                log::info!("{bands} bands in {} ms", drawn.elapsed().as_millis());
            }
        }
    }
}

/// The stub, printing what it is asked over serial.
struct Logged<B>(B);

impl<B: Backend> Backend for Logged<B> {
    fn evaluate(&mut self, mode: Mode, latex: &str) -> Result<String, String> {
        let result = self.0.evaluate(mode, latex);
        println!("{}: {latex}  =>  {result:?}", mode.name());
        result
    }
}

/// Turns the bytes typed into the serial monitor into keys: characters by `Key::from_char`,
/// the arrows' escape sequences, and Delete (`ESC [ 3 ~`) as AC.
fn read_keys(keys: Sender<Key>) {
    let mut stdin = std::io::stdin();
    let mut pending: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        if !matches!(stdin.read(&mut byte), Ok(1)) {
            FreeRtos::delay_ms(10);
            continue;
        }
        pending.push(byte[0]);
        let key = match pending.as_slice() {
            [0x1b] | [0x1b, b'['] | [0x1b, b'[', b'3'] => continue,
            [0x1b, b'[', b'A'] => Some(Key::Up),
            [0x1b, b'[', b'B'] => Some(Key::Down),
            [0x1b, b'[', b'C'] => Some(Key::Right),
            [0x1b, b'[', b'D'] => Some(Key::Left),
            [0x1b, b'[', b'3', b'~'] => Some(Key::Ac),
            [0x1b, ..] => None,
            bytes => match std::str::from_utf8(bytes) {
                Ok(text) => text.chars().next().and_then(Key::from_char),
                // The start of a character of several bytes.
                Err(error) if error.error_len().is_none() => continue,
                Err(_) => None,
            },
        };
        pending.clear();
        if let Some(key) = key {
            if keys.send(key).is_err() {
                return;
            }
        }
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
