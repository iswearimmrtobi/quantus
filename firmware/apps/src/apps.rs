//! The apps there are: built into the firmware, and on the SD card (apps.md §2).

use std::path::PathBuf;

use quantus_ui::applet::{AppInfo, Applet};
use quantus_ui::text::Lang;

use crate::header;
use crate::ScriptApplet;

/// The apps compiled into the firmware: their ids and their files.
pub const BUILT_IN: &[(&str, &str)] = &[
    ("2048", include_str!("../apps/2048.lua")),
    ("tictactoe", include_str!("../apps/tictactoe.lua")),
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

    /// What the list shows of them, in `lang`, sorted by name: the built-in ones, and the
    /// card's `/apps/*.lua`, read only as far as their header. A file without a `name`, over
    /// the size limit, with another ending or a name that is no plain id is left out. One with
    /// the id of a built-in app is listed beside it, as `card:<id>`, so their data do not mix.
    pub fn list(&self, lang: Lang) -> Vec<AppInfo> {
        let mut apps: Vec<AppInfo> = BUILT_IN.iter().filter_map(|(id, source)| header::info(id, source, true, lang)).collect();
        for (id, head) in self.card_files() {
            let id = if BUILT_IN.iter().any(|(built_in, _)| *built_in == id) { format!("{CARD}{id}") } else { id };
            apps.extend(header::info(&id, &head, false, lang));
        }
        apps.sort_by_key(|app| app.name.to_lowercase());
        apps
    }

    /// The ids of the card's apps and the start of their files, enough for the header.
    fn card_files(&self) -> Vec<(String, String)> {
        let Some(folder) = &self.card else { return Vec::new() };
        let Ok(entries) = std::fs::read_dir(folder) else { return Vec::new() };
        let mut files = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(id) = path.file_stem().and_then(|stem| stem.to_str()).map(str::to_string) else { continue };
            let lua = path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("lua"));
            let small = entry.metadata().is_ok_and(|meta| meta.is_file() && meta.len() <= crate::api::MAX_SOURCE as u64);
            if !lua || !small || !plain_id(&id) {
                continue;
            }
            if let Some(head) = head(&path) {
                files.push((id, head));
            }
        }
        files
    }

    /// The source of the app `id`: built in, or the card's (`card:<id>` for one beside a
    /// built-in app of the same id).
    pub fn source(&self, id: &str) -> Option<String> {
        if let Some(card) = id.strip_prefix(CARD) {
            return self.card_source(card);
        }
        match BUILT_IN.iter().find(|(built_in, _)| *built_in == id) {
            Some((_, source)) => Some(source.to_string()),
            None => self.card_source(id),
        }
    }

    fn card_source(&self, id: &str) -> Option<String> {
        if !plain_id(id) {
            return None;
        }
        let path = self.card.as_ref()?.join(format!("{id}.lua"));
        let bytes = std::fs::read(path).ok()?;
        (bytes.len() <= crate::api::MAX_SOURCE).then(|| String::from_utf8_lossy(&bytes).into_owned())
    }

    /// Where the data of the app `id` are kept, if there is a card and `id` is a name a file
    /// can have: letters, digits, `-` and `_`.
    fn data_file(&self, id: &str) -> Option<PathBuf> {
        // A card's app beside a built-in one: `~` cannot be in an id, so the files do not mix.
        let name = match id.strip_prefix(CARD) {
            Some(card) if plain_id(card) => format!("card~{card}"),
            Some(_) => return None,
            None if plain_id(id) => id.to_string(),
            None => return None,
        };
        Some(self.card.as_ref()?.join("data").join(format!("{name}.json")))
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

/// Marks an app of the card whose id a built-in app has.
pub const CARD: &str = "card:";

/// How much of a file is read for its header.
const HEAD: u64 = 2048;

/// Whether `id` is a name a file can have everywhere: letters, digits, `-` and `_`.
fn plain_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// The first [`HEAD`] bytes of the file at `path`, as text.
fn head(path: &std::path::Path) -> Option<String> {
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path).ok()?.take(HEAD).read_to_end(&mut bytes).ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
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
