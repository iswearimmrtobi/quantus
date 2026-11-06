//! Testing only: finds out why the touch controller does not answer. Flash it and read the
//! serial monitor. It prints
//!
//! 1. the levels of SDA (GPIO7) and SCL (GPIO8) with nothing driving them, first with the pull-ups
//!    off, then with the chip's internal ones on: an idle bus reads high, and low with the
//!    pull-ups off means something holds the line down (a short, an unpowered chip);
//! 2. how often the audio codec (0x18) and the touch controller (0x38) answer out of 20 probes,
//!    with TP_RST (GPIO21) low and then high. The codec should answer every time, the touch
//!    controller with TP_RST high.

use std::time::Duration;

use esp_idf_svc::hal::delay::{FreeRtos, TickType};
use esp_idf_svc::hal::gpio::{PinDriver, Pull};
use esp_idf_svc::hal::i2c::{config::Config, I2cDriver};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::units::FromValueType;
use esp_idf_svc::sys;

fn main() {
    sys::link_patches();
    let mut peripherals = Peripherals::take().expect("peripherals");

    for (name, pull) in [("pull-ups off", Pull::Floating), ("internal pull-ups on", Pull::Up)] {
        // SAFETY: both drivers are dropped at the end of the round, before the pins are used again.
        let (sda, scl) = unsafe { (peripherals.pins.gpio7.reborrow(), peripherals.pins.gpio8.reborrow()) };
        let sda = PinDriver::input(sda, pull).expect("SDA");
        let scl = PinDriver::input(scl, pull).expect("SCL");
        FreeRtos::delay_ms(50);
        println!("i2cscan: {name}: SDA {}, SCL {}", level(sda.is_high()), level(scl.is_high()));
    }

    let mut reset = PinDriver::output(peripherals.pins.gpio21).expect("TP_RST");
    let mut i2c = I2cDriver::new(
        peripherals.i2c0,
        peripherals.pins.gpio7,
        peripherals.pins.gpio8,
        &Config::new().baudrate(100.kHz().into()),
    )
    .expect("i2c");
    let timeout = TickType::from(Duration::from_millis(20)).ticks();
    loop {
        for (name, high) in [("TP_RST low", false), ("TP_RST high", true)] {
            if high {
                reset.set_high().expect("TP_RST");
            } else {
                reset.set_low().expect("TP_RST");
            }
            FreeRtos::delay_ms(400);
            // Only the two addresses that matter, many times: a scan over every address fills
            // the bus with failed probes, which disturbs the result.
            let hits = |i2c: &mut I2cDriver, address: u8| (0..TRIES).filter(|_| i2c.write(address, &[], timeout).is_ok()).count();
            let (codec, touch) = (hits(&mut i2c, 0x18), hits(&mut i2c, 0x38));
            println!("i2cscan: {name}: codec 0x18 answered {codec}/{TRIES}, touch 0x38 {touch}/{TRIES}");
        }
        FreeRtos::delay_ms(1000);
    }
}

const TRIES: usize = 20;

fn level(high: bool) -> &'static str {
    if high {
        "high"
    } else {
        "LOW"
    }
}
