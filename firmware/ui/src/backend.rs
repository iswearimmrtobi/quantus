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
