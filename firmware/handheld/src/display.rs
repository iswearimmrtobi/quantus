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
use esp_idf_svc::sys::EspError;
use quantus_ui::render::{HEIGHT, WIDTH};

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
    buffer: Vec<u8>,
}

impl<'d> Display<'d> {
    /// Resets the display and sends the init sequence.
    pub fn new(
        spi: SpiDeviceDriver<'d, SpiDriver<'d>>,
        dc: PinDriver<'d, Output>,
        mut reset: PinDriver<'d, Output>,
    ) -> Result<Self, EspError> {
        reset.set_low()?;
        FreeRtos::delay_ms(10);
        reset.set_high()?;
        FreeRtos::delay_ms(120);
        let mut display = Display { spi, dc, buffer: Vec::with_capacity(CHUNK) };
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

    fn flush(&mut self) -> Result<(), EspError> {
        if !self.buffer.is_empty() {
            self.spi.write(&self.buffer)?;
            self.buffer.clear();
        }
        Ok(())
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
        for color in colors.into_iter().take(count) {
            self.buffer.extend_from_slice(&color.to_be_bytes());
            if self.buffer.len() == CHUNK {
                self.flush()?;
            }
        }
        self.flush()
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Rgb565) -> Result<(), Self::Error> {
        let count = (area.size.width * area.size.height) as usize;
        self.fill_contiguous(area, core::iter::repeat_n(color, count))
    }
}
