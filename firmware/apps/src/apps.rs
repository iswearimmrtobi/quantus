//! The apps there are: built into the firmware, and on the SD card (apps.md §2).

use std::path::PathBuf;

use quantus_ui::applet::{AppInfo, Applet};
use quantus_ui::text::Lang;

use crate::header;
use crate::ScriptApplet;

/// The apps compiled into the firmware: their ids and their files.
pub const BUILT_IN: &[(&str, &str)] = &[
    ("tictactoe", include_str!("../apps/tictactoe.rhai")),
    ("zaehler", include_str!("../apps/zaehler.rhai")),
];

/// The apps, built in and in the card's folder `/apps`.
#[derive(Clone, Debug, Default)]
pub struct Apps {
    /// The card's `/apps` folder, if there is a card.
    pub card: Option<PathBuf>,
}

impl Apps {
    pub fn new(card: Option<PathBuf>) -> Apps {
        Apps { card }
    }

    /// What the list shows of them, in `lang`.
    pub fn list(&self, lang: Lang) -> Vec<AppInfo> {
        let mut apps: Vec<AppInfo> = BUILT_IN.iter().filter_map(|(id, source)| header::info(id, source, true, lang)).collect();
        apps.sort_by_key(|app| app.name.to_lowercase());
        apps
    }

    /// The source of the app `id`.
    pub fn source(&self, id: &str) -> Option<String> {
        BUILT_IN.iter().find(|(built_in, _)| *built_in == id).map(|(_, source)| source.to_string())
    }

    /// The app `id`, ready to start.
    pub fn open(&self, id: &str) -> Result<Box<dyn Applet>, String> {
        let source = self.source(id).ok_or_else(|| format!("no app {id}"))?;
        Ok(Box::new(ScriptApplet::new(&source)))
    }
}

/// A small generator for the simulator and the tests, which a seed fixes; the board takes
/// `esp_random` instead.
#[derive(Clone, Copy, Debug)]
pub struct Seeded(u32);

impl Seeded {
    pub fn new(seed: u32) -> Seeded {
        Seeded(seed.max(1))
    }

    /// The next number (xorshift).
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
}
