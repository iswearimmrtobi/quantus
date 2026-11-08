//! The 3.5" capacitive touch display (Waveshare 3.5inch Capacitive Touch LCD): an ST7796S on
//! 4-wire SPI, 320×480 native, turned to landscape by `MADCTL`. Commands and parameters go out as
//! bytes, pixels as RGB565 words; the display cannot be read back. The touch controller (FT6336U)
//! is on I²C, see `touch.rs`.

use std::rc::Rc;

use embedded_graphics::pixelcolor::raw::ToBytes;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::{Output, PinDriver};
use esp_idf_svc::hal::ledc::LedcDriver;
use esp_idf_svc::hal::spi::{SpiDeviceDriver, SpiDriver};
use esp_idf_svc::sys::{self, esp, EspError};
use quantus_ui::render::{HEIGHT, WIDTH};

/// The SPI clock, or `QUANTUS_SPI_MHZ` at build time to try another. The P4 divides 80 MHz and
/// takes the fastest whole divisor not above this: 40 gives 40 MHz (80/2). The ST7796S takes more
/// than its datasheet says, but jumper wires do not: if the picture shows noise or shifted
/// lines, try 27 (26.67 MHz, 80/3) or less.
pub const SPI_MHZ: u32 = quantus_mirror::number(option_env!("QUANTUS_SPI_MHZ"), 40);

/// The display on the SPI bus.
pub type Bus<'d> = SpiDeviceDriver<'d, Rc<SpiDriver<'d>>>;

/// The display on SPI3 and the touch controller on I²C0, wired to the ESP32-P4 dev kit's header
/// as in `firmware/docs/dev-kit.md`, from the fields of a `Peripherals`, which stays usable for
/// the other pins: `(display, touch)`.
/// Keep the touch panel even when it is not read: dropping it gives up TP_RST and the I²C bus.
/// Panics if a driver cannot be set up.
#[macro_export]
macro_rules! open_display {
    ($peripherals:ident) => {{
        use esp_idf_svc::hal::gpio::{AnyIOPin, PinDriver};
        use esp_idf_svc::hal::i2c::{config::Config as I2cConfig, I2cDriver};
        use esp_idf_svc::hal::spi::config::{Config, DriverConfig};
        use esp_idf_svc::hal::spi::{Dma, SpiDeviceDriver, SpiDriver};
        use esp_idf_svc::hal::units::FromValueType;
        let bus = DriverConfig::new().dma(Dma::Auto(4096));
        let driver = std::rc::Rc::new(
            SpiDriver::new(
                $peripherals.spi3,
                $peripherals.pins.gpio0, // SCLK
                $peripherals.pins.gpio3, // MOSI
                None::<AnyIOPin>,        // MISO: the display is never read
                &bus,
            )
            .expect("spi"),
        );
        // Two transfers in flight: one sent by DMA while the next is filled.
        let config = Config::new().baudrate($crate::display::SPI_MHZ.MHz().into()).queue_size(2);
        let spi = SpiDeviceDriver::new(driver, Some($peripherals.pins.gpio36), &config).expect("LCD_CS");
        let dc = PinDriver::output($peripherals.pins.gpio4).expect("LCD_DC");
        let reset = PinDriver::output($peripherals.pins.gpio1).expect("LCD_RST");
        // LCD_BL dimmed by PWM, above hearing.
        let timer = esp_idf_svc::hal::ledc::LedcTimerDriver::new(
            $peripherals.ledc.timer0,
            &esp_idf_svc::hal::ledc::config::TimerConfig::new()
                .frequency(20.kHz().into())
                .resolution(esp_idf_svc::hal::ledc::Resolution::Bits10),
        )
        .expect("LEDC timer");
        let backlight = esp_idf_svc::hal::ledc::LedcDriver::new($peripherals.ledc.channel0, timer, $peripherals.pins.gpio5)
            .expect("LCD_BL");
        // The onboard I²C bus (GPIO7/8) also carries the audio codec, which this firmware leaves
        // alone, and the DSI and CSI connectors.
        let i2c = I2cDriver::new(
            $peripherals.i2c0,
            $peripherals.pins.gpio7, // TP_SDA
            $peripherals.pins.gpio8, // TP_SCL
            &I2cConfig::new().baudrate(400.kHz().into()),
        )
        .expect("i2c");
        let touch_reset = PinDriver::output($peripherals.pins.gpio21).expect("TP_RST");
        (
            $crate::display::Display::new(spi, dc, reset, backlight).expect("display"),
            $crate::touch::TouchPanel::new(i2c, touch_reset).expect("touch"),
        )
    }};
}

/// Memory access control: rows and columns swapped for landscape, BGR order. 0xE8 turns the
/// picture by 180°; 0x68 and 0xA8 mirror it.
const MADCTL: u8 = 0x28;

/// The init sequence of the ST7796S (as in TFT_eSPI's driver for it): a command, its parameters,
/// and a pause in ms after it. The panel is IPS and needs inversion on.
const INIT: &[(u8, &[u8], u32)] = &[
    (0x11, &[], 120),   // out of sleep
    (0xF0, &[0xC3], 0), // command set control: enable extended commands
    (0xF0, &[0x96], 0),
    (0x36, &[MADCTL], 0),
    (0x3A, &[0x55], 0),                // 16 bits per pixel
    (0xB4, &[0x01], 0),                // 1-dot inversion
    (0xB6, &[0x80, 0x02, 0x3B], 0),    // display function control
    (0xE8, &[0x40, 0x8A, 0x00, 0x00, 0x29, 0x19, 0xA5, 0x33], 0), // output adjust
    (0xC1, &[0x06], 0),                // power control 2
    (0xC2, &[0xA7], 0),                // power control 3
    (0xC5, &[0x18], 120),              // VCOM
    (0xE0, &[0xF0, 0x09, 0x0B, 0x06, 0x04, 0x15, 0x2F, 0x54, 0x42, 0x3C, 0x17, 0x14, 0x18, 0x1B], 0),
    (0xE1, &[0xE0, 0x09, 0x0B, 0x06, 0x04, 0x03, 0x2B, 0x43, 0x42, 0x3B, 0x16, 0x14, 0x17, 0x1B], 0),
    (0xF0, &[0x3C], 0), // command set control: disable extended commands
    (0xF0, &[0x69], 120),
    (0x21, &[], 0), // inversion on
    (0x29, &[], 0), // display on
];

/// Bytes per SPI transfer of pixels; even, so no pixel is split.
const CHUNK: usize = 4096;

pub struct Display<'d> {
    spi: Bus<'d>,
    /// Low for a command, high for data.
    dc: PinDriver<'d, Output>,
    /// Kept so the pin stays driven high: dropping a `PinDriver` resets the GPIO to a floating
    /// input, and the panel then resets itself at random and turns white.
    _reset: PinDriver<'d, Output>,
    /// LCD_BL as PWM: the duty cycle is the brightness. Kept for the same reason.
    backlight: LedcDriver<'d>,
    /// Two buffers of pixel bytes: DMA sends one while the other is filled.
    buffers: [Vec<u8>; 2],
    /// The transfers of the buffers, boxed so they stay put while queued.
    transfers: Box<[sys::spi_transaction_t; 2]>,
}

impl<'d> Display<'d> {
    /// Resets the display and sends the init sequence.
    pub fn new(
        spi: Bus<'d>,
        dc: PinDriver<'d, Output>,
        mut reset: PinDriver<'d, Output>,
        backlight: LedcDriver<'d>,
    ) -> Result<Self, EspError> {
        reset.set_low()?;
        FreeRtos::delay_ms(10);
        reset.set_high()?;
        FreeRtos::delay_ms(120);
        let mut display = Display {
            spi,
            dc,
            _reset: reset,
            backlight,
            buffers: [Vec::with_capacity(CHUNK), Vec::with_capacity(CHUNK)],
            transfers: Box::new([sys::spi_transaction_t::default(); 2]),
        };
        for &(command, parameters, pause) in INIT {
            display.command(command, parameters)?;
            if pause > 0 {
                FreeRtos::delay_ms(pause);
            }
        }
        display.set_brightness(10)?;
        Ok(display)
    }

    /// Sets the backlight to `level`, 1 (dim) to 10 (full). The duty cycle follows the square of
    /// the level, since the eye is roughly logarithmic: level 1 is 1 %, level 5 is 25 %.
    pub fn set_brightness(&mut self, level: u8) -> Result<(), EspError> {
        let level = u32::from(level.clamp(1, 10));
        let max = self.backlight.get_max_duty();
        self.backlight.set_duty((max * level * level / 100).max(1))
    }

    fn command(&mut self, command: u8, parameters: &[u8]) -> Result<(), EspError> {
        self.dc.set_low()?;
        self.spi.write(&[command])?;
        if !parameters.is_empty() {
            self.dc.set_high()?;
            self.spi.write(parameters)?;
        }
        Ok(())
    }

    /// Selects the area the next pixels fill, row by row.
    fn window(&mut self, area: &Rectangle) -> Result<(), EspError> {
        let Some(bottom_right) = area.bottom_right() else { return Ok(()) };
        let [x0, x1] = [area.top_left.x as u16, bottom_right.x as u16].map(u16::to_be_bytes);
        let [y0, y1] = [area.top_left.y as u16, bottom_right.y as u16].map(u16::to_be_bytes);
        self.command(0x2A, &[x0[0], x0[1], x1[0], x1[1]])?;
        self.command(0x2B, &[y0[0], y0[1], y1[0], y1[1]])?;
        self.command(0x2C, &[])?;
        self.dc.set_high()
    }

    /// The SPI clock the driver chose, in kHz: the P4 divides 80 MHz, so it can differ from
    /// [`SPI_MHZ`].
    pub fn actual_khz(&self) -> i32 {
        let mut khz = 0;
        // SAFETY: the handle is the driver's open device.
        unsafe { sys::spi_device_get_actual_freq(self.spi.device(), &mut khz) };
        khz
    }

    /// Queues buffer `index` to be sent by DMA.
    fn queue(&mut self, index: usize) -> Result<(), EspError> {
        let buffer = &self.buffers[index];
        let transfer = &mut self.transfers[index];
        *transfer = sys::spi_transaction_t::default();
        transfer.length = buffer.len() * 8;
        transfer.__bindgen_anon_1.tx_buffer = buffer.as_ptr().cast();
        // SAFETY: the buffer and the transfer stay untouched until `wait` collects it.
        esp!(unsafe { sys::spi_device_queue_trans(self.spi.device(), transfer, esp_idf_svc::hal::delay::BLOCK) })
    }

    /// Waits until the oldest queued transfer is sent.
    fn wait(&mut self) -> Result<(), EspError> {
        let mut done: *mut sys::spi_transaction_t = core::ptr::null_mut();
        // SAFETY: as in `queue`.
        esp!(unsafe { sys::spi_device_get_trans_result(self.spi.device(), &mut done, esp_idf_svc::hal::delay::BLOCK) })
    }
}

impl OriginDimensions for Display<'_> {
    fn size(&self) -> Size {
        Size::new(WIDTH, HEIGHT)
    }
}

impl DrawTarget for Display<'_> {
    type Color = Rgb565;
    type Error = EspError;

    /// One window per pixel: slow, only for what does not come as an area.
    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Rgb565>>,
    {
        for Pixel(point, color) in pixels {
            self.fill_contiguous(&Rectangle::new(point, Size::new(1, 1)), [color])?;
        }
        Ok(())
    }

    /// The renderer sends whole bands this way.
    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Rgb565>,
    {
        let area = area.intersection(&self.bounding_box());
        if area.is_zero_sized() {
            return Ok(());
        }
        self.window(&area)?;
        let count = (area.size.width * area.size.height) as usize;
        let mut colors = colors.into_iter().take(count);
        // Fill one buffer while DMA sends the other.
        let mut current = 0;
        let mut queued = 0;
        loop {
            // The buffer about to be refilled must be sent first; the other one goes on.
            if queued == 2 {
                self.wait()?;
                queued -= 1;
            }
            let buffer = &mut self.buffers[current];
            buffer.clear();
            for color in colors.by_ref().take(CHUNK / 2) {
                buffer.extend_from_slice(&color.to_be_bytes());
            }
            if buffer.is_empty() {
                break;
            }
            self.queue(current)?;
            queued += 1;
            current ^= 1;
        }
        for _ in 0..queued {
            self.wait()?;
        }
        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Rgb565) -> Result<(), Self::Error> {
        let count = (area.size.width * area.size.height) as usize;
        self.fill_contiguous(area, core::iter::repeat_n(color, count))
    }
}
