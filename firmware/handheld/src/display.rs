//! The 3.5" RPi display (Waveshare (A) layout): an ILI9486 behind shift registers that turn SPI
//! into a 16-bit bus. Every command and parameter goes out as a 16-bit word, pixels as RGB565
//! words, and every transfer must be a whole number of words. The display cannot be read back.

use embedded_graphics::pixelcolor::raw::ToBytes;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::{Output, PinDriver};
use esp_idf_svc::hal::spi::{SpiDeviceDriver, SpiDriver};
use esp_idf_svc::sys::{self, esp, EspError};
use quantus_ui::render::{HEIGHT, WIDTH};

/// The SPI clock: 16 MHz, or `QUANTUS_SPI_MHZ` at build time to try more. The ESP32's VSPI
/// pins manage 40 MHz and more; the display's shift registers set the limit.
pub const SPI_MHZ: u32 = quantus_mirror::number(option_env!("QUANTUS_SPI_MHZ"), 16);

/// The display on VSPI of a WROOM (see the README for the wiring), from the fields of a
/// `Peripherals`, which stays usable for the other pins. Panics if a driver cannot be set up.
macro_rules! open_display {
    ($peripherals:ident) => {{
        use esp_idf_svc::hal::gpio::PinDriver;
        use esp_idf_svc::hal::spi::config::{Config, DriverConfig};
        use esp_idf_svc::hal::spi::{Dma, SpiDeviceDriver};
        use esp_idf_svc::hal::units::FromValueType;
        let bus = DriverConfig::new().dma(Dma::Auto(4096));
        // Two transfers in flight: one sent by DMA while the next is filled.
        let config = Config::new().baudrate(display::SPI_MHZ.MHz().into()).queue_size(2);
        let spi = SpiDeviceDriver::new_single(
            $peripherals.spi3,
            $peripherals.pins.gpio18,       // SCLK
            $peripherals.pins.gpio23,       // MOSI
            Some($peripherals.pins.gpio19), // MISO, for the touch controller later
            Some($peripherals.pins.gpio5),  // LCD_CS
            &bus,
            &config,
        )
        .expect("spi");
        let dc = PinDriver::output($peripherals.pins.gpio21).expect("LCD_RS");
        let reset = PinDriver::output($peripherals.pins.gpio22).expect("RST");
        let touch_cs = PinDriver::output($peripherals.pins.gpio4).expect("TP_CS");
        display::Display::new(spi, dc, reset, touch_cs).expect("display")
    }};
}

/// Memory access control: rows and columns swapped for landscape, BGR order. 0xE8 turns the
/// picture by 180°.
const MADCTL: u8 = 0x28;

/// The init sequence of the Linux fbtft overlay for this display: a command, its parameters,
/// and a pause in ms after it.
const INIT: &[(u8, &[u8], u32)] = &[
    (0xB0, &[0x00], 0), // interface mode
    (0x11, &[], 255),   // out of sleep
    (0x3A, &[0x55], 0), // 16 bits per pixel
    (0x36, &[MADCTL], 0),
    (0xC2, &[0x44], 0),                   // power control 3
    (0xC5, &[0x00, 0x00, 0x00, 0x00], 0), // VCOM
    (0xE0, &[0x0F, 0x1F, 0x1C, 0x0C, 0x0F, 0x08, 0x48, 0x98, 0x37, 0x0A, 0x13, 0x04, 0x11, 0x0D, 0x00], 0),
    (0xE1, &[0x0F, 0x32, 0x2E, 0x0B, 0x0D, 0x05, 0x47, 0x75, 0x37, 0x06, 0x10, 0x03, 0x24, 0x20, 0x00], 0),
    (0xE2, &[0x0F, 0x32, 0x2E, 0x0B, 0x0D, 0x05, 0x47, 0x75, 0x37, 0x06, 0x10, 0x03, 0x24, 0x20, 0x00], 0),
    (0x29, &[], 0), // display on
];

/// Bytes per SPI transfer of pixels; even, so no word is split.
const CHUNK: usize = 4096;

pub struct Display<'d> {
    spi: SpiDeviceDriver<'d, SpiDriver<'d>>,
    /// Low for a command, high for data.
    dc: PinDriver<'d, Output>,
    /// Kept so the pin stays driven high: dropping a `PinDriver` resets the GPIO to a floating
    /// input, and the panel then resets itself at random and turns white.
    _reset: PinDriver<'d, Output>,
    /// The touch controller shares the bus and must stay deselected; kept for the same reason.
    _touch_cs: PinDriver<'d, Output>,
    /// Two buffers of pixel bytes: DMA sends one while the other is filled.
    buffers: [Vec<u8>; 2],
    /// The transfers of the buffers, boxed so they stay put while queued.
    transfers: Box<[sys::spi_transaction_t; 2]>,
}

impl<'d> Display<'d> {
    /// Resets the display and sends the init sequence.
    pub fn new(
        spi: SpiDeviceDriver<'d, SpiDriver<'d>>,
        dc: PinDriver<'d, Output>,
        mut reset: PinDriver<'d, Output>,
        mut touch_cs: PinDriver<'d, Output>,
    ) -> Result<Self, EspError> {
        touch_cs.set_high()?;
        reset.set_low()?;
        FreeRtos::delay_ms(10);
        reset.set_high()?;
        FreeRtos::delay_ms(120);
        let mut display = Display {
            spi,
            dc,
            _reset: reset,
            _touch_cs: touch_cs,
            buffers: [Vec::with_capacity(CHUNK), Vec::with_capacity(CHUNK)],
            transfers: Box::new([sys::spi_transaction_t::default(); 2]),
        };
        for &(command, parameters, pause) in INIT {
            display.command(command, parameters)?;
            if pause > 0 {
                FreeRtos::delay_ms(pause);
            }
        }
        Ok(display)
    }

    fn command(&mut self, command: u8, parameters: &[u8]) -> Result<(), EspError> {
        self.dc.set_low()?;
        self.spi.write(&[0, command])?;
        if !parameters.is_empty() {
            self.dc.set_high()?;
            let words: Vec<u8> = parameters.iter().flat_map(|&byte| [0, byte]).collect();
            self.spi.write(&words)?;
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

    /// The SPI clock the driver chose, in kHz: the ESP32 divides 80 MHz, so it can differ from
    /// [`SPI_MHZ`].
    #[allow(dead_code)] // only the benchmark asks
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
