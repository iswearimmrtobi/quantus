//! What computes the results. On the P4 this will be `quantus-core`; until then a stub.

use crate::app::{Mode, Settings};
use crate::wallpaper::Wallpaper;

pub trait Backend {
    /// The result of `latex` in `mode`, written as `settings` say, as LaTeX, or an error message.
    fn evaluate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> Result<String, String>;

    /// The pictures in `/wallpapers/` on the SD card: their names without `.bmp`, sorted.
    fn wallpapers(&mut self) -> Vec<String> {
        Vec::new()
    }

    /// The file of the picture `name`.
    fn wallpaper(&mut self, _name: &str) -> Option<Vec<u8>> {
        None
    }

    /// The picture `name`, ready to draw: its file decoded into RAM, unless the board keeps
    /// it in some other way (in flash, [`Wallpaper::from_raw`]).
    fn picture(&mut self, name: &str) -> Option<Wallpaper> {
        Wallpaper::from_bmp(&self.wallpaper(name)?)
    }

    /// How full the SD card is; `None` without a card.
    fn storage(&mut self) -> Option<Storage> {
        None
    }
}

/// The size of a card and the room left on it, in bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Storage {
    pub total: u64,
    pub free: u64,
}

impl Storage {
    pub fn used(self) -> u64 {
        self.total.saturating_sub(self.free)
    }
}

/// `bytes` in decimal units, as cards are sold: `12,3 GB` in German, `12.3 GB` in English; one
/// decimal below 100 of a unit.
pub fn bytes(bytes: u64, lang: crate::text::Lang) -> String {
    const UNITS: [&str; 5] = ["kB", "MB", "GB", "TB", "PB"];
    if bytes < 1000 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64 / 1000.0;
    let mut unit = 0;
    // Rounded first, so 999 950 is 1,0 MB and not 1000,0 kB.
    while unit + 1 < UNITS.len() && (value * 10.0).round() / 10.0 >= 1000.0 {
        value /= 1000.0;
        unit += 1;
    }
    let text = if value < 99.95 { format!("{value:.1}") } else { format!("{value:.0}") };
    let text = if lang == crate::text::Lang::De { text.replace('.', ",") } else { text };
    format!("{text} {}", UNITS[unit])
}

/// Answers with the input itself, so the way back from LaTeX shows on the screen. An input with
/// a `?` gives an error; a decimal result is marked with `≈`.
pub struct Stub;

impl Backend for Stub {
    fn evaluate(&mut self, _mode: Mode, settings: &Settings, latex: &str) -> Result<String, String> {
        if latex.contains('?') {
            Err("Syntax error".to_string())
        } else if settings.decimal {
            Ok(format!(r"\approx {latex}"))
        } else {
            Ok(latex.to_string())
        }
    }
}
