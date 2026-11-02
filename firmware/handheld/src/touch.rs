//! The display's XPT2046 resistive touch controller, on the display's SPI bus with its own chip
//! select. It is polled, and a press is told by its pressure, so TP_IRQ is not needed. It reports
//! the panel's raw readings; the hui turns them into screen points with the calibration it made
//! on its calibration screen, which is kept in flash (NVS) here.
//!
//! On jumper wires a reading is now and then garbage (all zeros, or more than 12 bits): such a
//! poll says nothing, neither pressed nor lifted, so it neither starts nor ends a touch.

use embedded_graphics::prelude::Point;
use esp_idf_svc::nvs::{EspDefaultNvs, EspDefaultNvsPartition};
use esp_idf_svc::sys::EspError;
use quantus_ui::touch::{Calibration, Touch};

use crate::display::Bus;

/// The conversions, 12 bits, differential, powered down between them.
const READ_X: u8 = 0xD0;
const READ_Y: u8 = 0x90;
const READ_Z1: u8 = 0xB0;
const READ_Z2: u8 = 0xC0;

/// Z1 at least this, and Z2 below [`Z2_MAX`]: pressed. Untouched, Z1 reads about 0 and Z2 about
/// 4080; a finger gives about 500 and 2500.
const Z1_MIN: u16 = 100;
const Z2_MAX: u16 = 4000;

/// Readings per position; the median of the sound ones is taken, if there are enough.
const SAMPLES: usize = 7;
const ENOUGH: usize = 4;

/// How far the middle half of the sound samples of an axis may lie apart; more means the finger
/// slid or the readings are unsteady.
const SPREAD: u16 = 150;

/// Polls in a row that say lifted before the finger counts as lifted.
const LIFTED: u8 = 2;

/// Polls in a row that say nothing before the finger counts as lifted anyway: a wire that came
/// loose must not hold a touch down for ever.
const SILENT: u8 = 10;

/// What one poll says.
enum Reading {
    Pressed(Point),
    Lifted,
    /// Garbage on the wire.
    Unknown,
}

pub struct TouchPanel<'d> {
    spi: Bus<'d>,
    down: bool,
    /// A first pressed poll, waiting for a second one to confirm it.
    pending: bool,
    lifted: u8,
    silent: u8,
}

impl<'d> TouchPanel<'d> {
    pub fn new(spi: Bus<'d>) -> Self {
        TouchPanel { spi, down: false, pending: false, lifted: 0, silent: 0 }
    }

    /// What the finger did since the last poll, if anything, as raw readings. A touch starts
    /// with the second pressed poll in a row, so a single stray reading does not tap.
    pub fn poll(&mut self) -> Option<Touch> {
        match self.reading() {
            Reading::Pressed(raw) => {
                (self.lifted, self.silent) = (0, 0);
                if self.down {
                    return Some(Touch::Drag(raw));
                }
                if !std::mem::replace(&mut self.pending, true) {
                    return None;
                }
                self.down = true;
                Some(Touch::Press(raw))
            }
            Reading::Lifted => {
                self.silent = 0;
                self.lifted = self.lifted.saturating_add(1);
                self.lift(LIFTED <= self.lifted)
            }
            Reading::Unknown => {
                self.silent = self.silent.saturating_add(1);
                self.lift(SILENT <= self.silent)
            }
        }
    }

    /// The finger is up if `gone`.
    fn lift(&mut self, gone: bool) -> Option<Touch> {
        if !gone {
            return None;
        }
        self.pending = false;
        std::mem::take(&mut self.down).then_some(Touch::Release)
    }

    /// Prints the raw readings, pressed or not: built with `QUANTUS_TOUCH_DEBUG=1`, to find out
    /// why touches do not arrive.
    pub fn debug(&mut self) {
        let mut read = |command| self.read(command).map_or(-1, i32::from);
        let (z1, z2, x, y) = (read(READ_Z1), read(READ_Z2), read(READ_X), read(READ_Y));
        println!("touch debug: z1 {z1}, z2 {z2}, x {x}, y {y}");
    }

    fn reading(&mut self) -> Reading {
        match self.raw() {
            Ok(reading) => reading,
            Err(error) => {
                log::warn!("touch: {error}");
                Reading::Unknown
            }
        }
    }

    /// The raw X and Y readings, if the panel is pressed firmly enough and they are sound.
    fn raw(&mut self) -> Result<Reading, EspError> {
        match self.pressed()? {
            Some(true) => {}
            Some(false) => return Ok(Reading::Lifted),
            None => return Ok(Reading::Unknown),
        }
        let (mut xs, mut ys) = (Vec::with_capacity(SAMPLES), Vec::with_capacity(SAMPLES));
        for _ in 0..SAMPLES {
            xs.push(self.read(READ_X)?);
            ys.push(self.read(READ_Y)?);
        }
        let (Some(x), Some(y)) = (median(xs), median(ys)) else { return Ok(Reading::Unknown) };
        // Lifted while reading: the samples are not to be trusted.
        Ok(match self.pressed()? {
            Some(true) => Reading::Pressed(Point::new(x.into(), y.into())),
            Some(false) => Reading::Lifted,
            None => Reading::Unknown,
        })
    }

    /// Whether the pressure says the panel is pressed; `None` for garbage.
    fn pressed(&mut self) -> Result<Option<bool>, EspError> {
        let (z1, z2) = (self.read(READ_Z1)?, self.read(READ_Z2)?);
        if (z1 == 0 && z2 == 0) || z1 > 4095 || z2 > 4095 {
            return Ok(None);
        }
        Ok(Some(z1 >= Z1_MIN && z2 > z1 && z2 < Z2_MAX))
    }

    /// One conversion: the command, then 12 bits in the next two bytes; more than 12 bits is
    /// garbage on the wire.
    fn read(&mut self, command: u8) -> Result<u16, EspError> {
        let mut read = [0u8; 3];
        self.spi.transfer(&mut read, &[command, 0, 0])?;
        Ok(((u16::from(read[1]) << 8) | u16::from(read[2])) >> 3)
    }
}

/// The median of the sound samples (neither 0 nor more than 12 bits), if there are enough and
/// they agree.
fn median(mut samples: Vec<u16>) -> Option<u16> {
    samples.retain(|&sample| sample > 0 && sample <= 4095);
    if samples.len() < ENOUGH {
        return None;
    }
    samples.sort_unstable();
    let quarter = samples.len() / 4;
    (samples[samples.len() - 1 - quarter] - samples[quarter] <= SPREAD).then(|| samples[samples.len() / 2])
}

/// Where the calibration and the wallpaper's name are kept: a namespace in the NVS partition.
pub struct Store(EspDefaultNvs);

const NAMESPACE: &str = "quantus";
const KEY: &str = "touch_cal";

impl Store {
    pub fn open() -> Result<Store, EspError> {
        Ok(Store(EspDefaultNvs::new(EspDefaultNvsPartition::take()?, NAMESPACE, true)?))
    }

    pub fn load(&self) -> Option<Calibration> {
        let mut buffer = [0u8; 24];
        let bytes = self.0.get_blob(KEY, &mut buffer).ok()??;
        Calibration::from_bytes(bytes)
    }

    pub fn save(&self, calibration: &Calibration) -> Result<(), EspError> {
        self.0.set_blob(KEY, &calibration.to_bytes())
    }

    /// The name of the wallpaper set, if any.
    pub fn load_wallpaper(&self) -> Option<String> {
        let mut buffer = [0u8; 256];
        self.0.get_str(WALLPAPER, &mut buffer).ok()?.map(str::to_string)
    }

    /// Keeps the wallpaper `name`, or forgets it for `None`.
    pub fn save_wallpaper(&self, name: Option<&str>) -> Result<(), EspError> {
        match name {
            Some(name) => self.0.set_str(WALLPAPER, name),
            None => self.0.remove(WALLPAPER).map(|_| ()),
        }
    }
}

const WALLPAPER: &str = "wallpaper";
