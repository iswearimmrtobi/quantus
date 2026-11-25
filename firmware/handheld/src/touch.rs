//! The display's FT6336U capacitive touch controller, on I²C (address 0x38). It is polled: one
//! read of its status and first-finger registers says whether the panel is touched and where, so
//! TP_INT is not needed. It reports the panel's own coordinates (320×480, portrait); the UI turns
//! them into screen points with the calibration it made on its calibration screen, which is kept
//! in flash (`store.rs`), so the panel's orientation does not matter.
//!
//! A poll that fails (a loose wire) says nothing, neither pressed nor lifted; after [`SILENT`] of
//! them in a row the finger counts as lifted, so a touch is not held down for ever.

use embedded_graphics::prelude::Point;
use esp_idf_svc::hal::delay::{FreeRtos, BLOCK};
use esp_idf_svc::hal::gpio::{Output, PinDriver};
use esp_idf_svc::hal::i2c::I2cDriver;
use esp_idf_svc::sys::{self, EspError};
use quantus_ui::touch::Touch;

/// The controller's 7-bit address.
const ADDRESS: u8 = 0x38;

/// Registers: from `STATUS` on, five bytes: the number of fingers (low nibble), then the first
/// finger's X high byte (event in the top two bits, X in the low nibble), X low, Y high, Y low.
const STATUS: u8 = 0x02;
const CHIP_ID: u8 = 0xA3;

/// The FT6336U's chip ID; the FT6236 reports 0x36.
const FT6336U: u8 = 0x64;

/// Polls in a row that say lifted before the finger counts as lifted.
const LIFTED: u8 = 2;

/// Polls in a row that fail before the finger counts as lifted anyway.
const SILENT: u8 = 10;

/// Polls in a row that fail before the bus counts as stuck and is freed ([`TouchPanel::recover`]).
const STUCK: u8 = 20;

/// The bus's pins and port, as `open_display!` sets them up.
const SDA: i32 = 7;
const SCL: i32 = 8;
const PORT: sys::i2c_port_t = 0;

/// What one poll says.
enum Reading {
    Pressed(Point),
    Lifted,
    /// The bus failed.
    Unknown,
}

pub struct TouchPanel<'d> {
    i2c: I2cDriver<'d>,
    /// Kept so TP_RST stays driven high: dropping a `PinDriver` floats the pin.
    reset: PinDriver<'d, Output>,
    down: bool,
    lifted: u8,
    silent: u8,
}

impl<'d> TouchPanel<'d> {
    /// Resets the controller through `reset` and reads its chip ID, which is logged. An
    /// unexpected one, or none at all, is only a warning (with the addresses that do answer on
    /// the bus), so the display can still be tried; such a panel reports no touches.
    pub fn new(i2c: I2cDriver<'d>, mut reset: PinDriver<'d, Output>) -> Result<Self, EspError> {
        reset.set_low()?;
        FreeRtos::delay_ms(10);
        reset.set_high()?;
        // The controller needs about 300 ms after reset before it answers.
        FreeRtos::delay_ms(300);
        let mut panel = TouchPanel { i2c, reset, down: false, lifted: 0, silent: 0 };
        let mut id = [0u8];
        match panel.i2c.write_read(ADDRESS, &[CHIP_ID], &mut id, BLOCK) {
            Ok(()) if id[0] == FT6336U => log::info!("touch: FT6336U ready"),
            Ok(()) => log::warn!("touch: chip ID 0x{:02X}, expected 0x{FT6336U:02X} (FT6336U)", id[0]),
            Err(error) => {
                // A broken bus makes every probe take about a second, so only the likely ones:
                // the codec, the FT6336U, and the addresses of other common touch chips.
                let found: Vec<String> = [0x14u8, 0x15, 0x18, 0x38, 0x5D]
                    .into_iter()
                    .filter(|&address| panel.i2c.write(address, &[], BLOCK).is_ok())
                    .map(|address| format!("0x{address:02X}"))
                    .collect();
                log::warn!("touch: no answer from 0x{ADDRESS:02X} ({error}); on the bus: {found:?}");
            }
        }
        Ok(panel)
    }

    /// The I²C bus the controller is on, for the other devices on it.
    pub fn bus(&mut self) -> &mut I2cDriver<'d> {
        &mut self.i2c
    }

    /// What the finger did since the last poll, if anything, as the panel's coordinates.
    pub fn poll(&mut self) -> Option<Touch> {
        match self.reading() {
            Reading::Pressed(raw) => {
                (self.lifted, self.silent) = (0, 0);
                let was_down = std::mem::replace(&mut self.down, true);
                Some(if was_down { Touch::Drag(raw) } else { Touch::Press(raw) })
            }
            Reading::Lifted => {
                self.silent = 0;
                self.lifted = self.lifted.saturating_add(1);
                self.lift(LIFTED <= self.lifted)
            }
            Reading::Unknown => {
                self.silent = self.silent.saturating_add(1);
                if self.silent % STUCK == 0 {
                    self.recover();
                }
                self.lift(SILENT <= self.silent)
            }
        }
    }

    /// Frees a bus that a device holds: a device that lost count of the clock in the middle of
    /// a transfer keeps SDA low, and the driver can do nothing until it lets go. The FT6336U is
    /// reset, then SCL clocked until SDA comes up, then a STOP sent, and the pins handed back to
    /// the driver. The lines' levels are logged on the way, to tell which device held it.
    pub fn recover(&mut self) {
        // SAFETY: plain GPIO and I²C driver calls on the bus's own pins and port, from the only
        // thread that uses the bus.
        unsafe {
            let level = |pin| sys::gpio_get_level(pin);
            let (sda, scl) = (level(SDA), level(SCL));
            let _ = self.reset.set_low();
            FreeRtos::delay_ms(10);
            let _ = self.reset.set_high();
            let after_reset = level(SDA);
            sys::gpio_reset_pin(SCL);
            sys::gpio_reset_pin(SDA);
            sys::gpio_set_direction(SCL, sys::gpio_mode_t_GPIO_MODE_INPUT_OUTPUT_OD);
            sys::gpio_set_direction(SDA, sys::gpio_mode_t_GPIO_MODE_INPUT_OUTPUT_OD);
            sys::gpio_set_pull_mode(SCL, sys::gpio_pull_mode_t_GPIO_PULLUP_ONLY);
            sys::gpio_set_pull_mode(SDA, sys::gpio_pull_mode_t_GPIO_PULLUP_ONLY);
            sys::gpio_set_level(SDA, 1);
            sys::gpio_set_level(SCL, 1);
            sys::esp_rom_delay_us(10);
            let mut clocks = 0;
            while level(SDA) == 0 && clocks < 16 {
                sys::gpio_set_level(SCL, 0);
                sys::esp_rom_delay_us(10);
                sys::gpio_set_level(SCL, 1);
                sys::esp_rom_delay_us(10);
                clocks += 1;
            }
            // STOP: SDA rises while SCL is high.
            sys::gpio_set_level(SCL, 0);
            sys::esp_rom_delay_us(10);
            sys::gpio_set_level(SDA, 0);
            sys::esp_rom_delay_us(10);
            sys::gpio_set_level(SCL, 1);
            sys::esp_rom_delay_us(10);
            sys::gpio_set_level(SDA, 1);
            sys::esp_rom_delay_us(10);
            let freed = (level(SDA), level(SCL));
            sys::i2c_set_pin(PORT, SDA, SCL, true, true, sys::i2c_mode_t_I2C_MODE_MASTER);
            sys::i2c_reset_tx_fifo(PORT);
            sys::i2c_reset_rx_fifo(PORT);
            log::warn!(
                "touch: bus stuck (SDA {sda}, SCL {scl}); after the FT6336U's reset SDA {after_reset}; \
                 {clocks} clocks; now SDA {}, SCL {}",
                freed.0,
                freed.1
            );
        }
        // The controller answers about 300 ms after its reset.
        FreeRtos::delay_ms(300);
    }

    /// The finger is up if `gone`.
    fn lift(&mut self, gone: bool) -> Option<Touch> {
        if !gone {
            return None;
        }
        std::mem::take(&mut self.down).then_some(Touch::Release)
    }

    /// Prints the raw registers, touched or not: built with `QUANTUS_TOUCH_DEBUG=1`, to find out
    /// why touches do not arrive.
    pub fn debug(&mut self) {
        match self.registers() {
            Ok(r) => println!("touch debug: registers {r:02X?}"),
            Err(error) => println!("touch debug: {error}"),
        }
    }

    fn reading(&mut self) -> Reading {
        match self.registers() {
            Ok(r) if r[0] & 0x0F == 0 => Reading::Lifted,
            Ok(r) => {
                let x = u16::from(r[1] & 0x0F) << 8 | u16::from(r[2]);
                let y = u16::from(r[3] & 0x0F) << 8 | u16::from(r[4]);
                Reading::Pressed(Point::new(x.into(), y.into()))
            }
            Err(error) => {
                log::warn!("touch: {error}");
                Reading::Unknown
            }
        }
    }

    fn registers(&mut self) -> Result<[u8; 5], EspError> {
        let mut registers = [0u8; 5];
        self.i2c.write_read(ADDRESS, &[STATUS], &mut registers, BLOCK)?;
        Ok(registers)
    }
}
