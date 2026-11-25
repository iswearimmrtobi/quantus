//! The Duo's main screen touch: a Goodix controller (the 10.1" bench panel's GT9271; the 5"
//! panel's GT911 speaks the same), polled over I²C as `touch.rs` polls the FT6336U. Its reset
//! and interrupt lines are not wired, so it answers at whichever of its two addresses it chose
//! at power-up. It reports the panel's own, portrait coordinates.

use embedded_graphics::prelude::Point;
use esp_idf_svc::hal::delay::BLOCK;
use esp_idf_svc::hal::i2c::I2cDriver;
use quantus_ui::touch::Touch;

/// The two addresses a Goodix controller takes.
const ADDRESSES: [u8; 2] = [0x5D, 0x14];
/// Registers, 16 bits big-endian: the product ID (4 ASCII bytes), and the status: bit 7 set when
/// a report is ready, the low nibble the number of fingers, followed by the first finger's
/// track ID, X and Y (little-endian).
const PRODUCT_ID: u16 = 0x8140;
const STATUS: u16 = 0x814E;

pub struct Goodix {
    address: u8,
    down: bool,
}

impl Goodix {
    /// The controller on `i2c`, if one answers.
    pub fn probe(i2c: &mut I2cDriver) -> Option<Goodix> {
        for address in ADDRESSES {
            let mut id = [0u8; 4];
            if i2c.write_read(address, &PRODUCT_ID.to_be_bytes(), &mut id, BLOCK).is_ok() {
                log::info!("main touch: Goodix {} at 0x{address:02X}", String::from_utf8_lossy(&id));
                return Some(Goodix { address, down: false });
            }
        }
        log::warn!("main touch: no Goodix controller at 0x5D or 0x14");
        None
    }

    /// What the finger did since the last poll, if anything, on the panel's coordinates.
    pub fn poll(&mut self, i2c: &mut I2cDriver) -> Option<Touch> {
        let mut report = [0u8; 6];
        i2c.write_read(self.address, &STATUS.to_be_bytes(), &mut report, BLOCK).ok()?;
        if report[0] & 0x80 == 0 {
            // No new report yet: the finger is where it was.
            return None;
        }
        let mut clear = [0u8; 3];
        clear[..2].copy_from_slice(&STATUS.to_be_bytes());
        let _ = i2c.write(self.address, &clear, BLOCK);
        if report[0] & 0x0F == 0 {
            return std::mem::take(&mut self.down).then_some(Touch::Release);
        }
        let x = u16::from_le_bytes([report[2], report[3]]);
        let y = u16::from_le_bytes([report[4], report[5]]);
        let point = Point::new(x.into(), y.into());
        Some(if std::mem::replace(&mut self.down, true) { Touch::Drag(point) } else { Touch::Press(point) })
    }
}
