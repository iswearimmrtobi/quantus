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

    /// Where the data of the app `id` are kept, if there is a card and `id` is a name a file
    /// can have: letters, digits, `-` and `_`.
    fn data_file(&self, id: &str) -> Option<PathBuf> {
        let valid = !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        if !valid {
            return None;
        }
        Some(self.card.as_ref()?.join("data").join(format!("{id}.json")))
    }

    /// The saved data of the app `id`, from `/apps/data/<id>.json`.
    pub fn data(&self, id: &str) -> Option<String> {
        let text = std::fs::read_to_string(self.data_file(id)?).ok()?;
        (text.len() <= crate::json::MAX_DATA).then_some(text)
    }

    /// Keeps the data of the app `id`: written to a file of its own first, then put in place,
    /// so switching off while writing leaves the old data or the new.
    pub fn store_data(&self, id: &str, json: &str) -> std::io::Result<()> {
        let file = self.data_file(id).ok_or_else(|| std::io::Error::other("no card or a bad id"))?;
        if let Some(folder) = file.parent() {
            std::fs::create_dir_all(folder)?;
        }
        let new = file.with_extension("new");
        std::fs::write(&new, json)?;
        // FAT does not rename over a file.
        if file.exists() {
            std::fs::remove_file(&file)?;
        }
        std::fs::rename(&new, &file)
    }

    /// Removes the data of the app `id`; whether there were any.
    pub fn delete_data(&self, id: &str) -> bool {
        self.data_file(id).is_some_and(|file| std::fs::remove_file(file).is_ok())
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
