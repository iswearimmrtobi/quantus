//! What computes the results: `quantus-core` (the `quantus-engine` crate), or a stub.

use crate::app::{Mode, Settings};
use crate::clock;
use crate::wallpaper::Wallpaper;

pub trait Backend {
    /// The result of `latex` in `mode`, written as `settings` say, as LaTeX, or an error message.
    fn evaluate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> Result<String, String>;

    /// Starts a calculation. A backend that answers at once returns the answer; one that
    /// calculates elsewhere (on a thread of its own) returns `None` and hands it to
    /// [`poll`](Backend::poll) later. One calculation at a time: the app waits for the answer or
    /// drops it before it starts the next.
    fn start(&mut self, mode: Mode, settings: &Settings, latex: &str) -> Option<Result<String, String>> {
        Some(self.evaluate(mode, settings, latex))
    }

    /// The answer to the last [`start`](Backend::start) that returned `None`, once it is there.
    fn poll(&mut self) -> Option<Result<String, String>> {
        None
    }

    /// [`start`](Backend::start) for an input that is already in the history, in the other form
    /// (S⇔D): the session does not record it a second time.
    fn recalculate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> Option<Result<String, String>> {
        self.start(mode, settings, latex)
    }

    /// Forgets the running calculation: its answer never comes from [`poll`](Backend::poll).
    fn drop_pending(&mut self) {}

    /// The sessions there are, in the order the session list shows them.
    fn sessions(&mut self) -> Vec<SessionInfo> {
        Vec::new()
    }

    /// Does `command` to the sessions and returns the one open afterwards, or why it failed. A
    /// backend without sessions (the stub) has none.
    fn session(&mut self, _command: SessionCommand, _settings: &Settings) -> Result<Opened, String> {
        Err("no sessions".to_string())
    }

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

    /// The settings kept from the last run, if any.
    fn load_settings(&mut self) -> Option<Settings> {
        None
    }

    /// The settings in force: given once at the start and again after every change, so the
    /// board can keep them and switch what they control (the radios, the clock).
    fn use_settings(&mut self, _settings: &Settings) {}

    /// Sets the clock to `utc_ms` (milliseconds since 1970, UTC), typed by hand. A board that
    /// can set its system clock does that instead of moving [`clock::now`].
    fn set_time(&mut self, utc_ms: i64) {
        clock::set(utc_ms);
    }
}

/// A session as the menus show it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionInfo {
    pub id: String,
    pub name: String,
}

/// What the Sitzungen menu asks of the backend. Rename, Duplicate, Delete and ClearHistory act on
/// the open session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionCommand {
    /// Nothing: just the open session, at start.
    Current,
    New,
    Open(String),
    Rename(String),
    /// A copy, which is opened.
    Duplicate,
    /// Deletes the open session and opens the next one, or a new one after the last.
    Delete,
    ClearHistory,
}

/// The open session and its history, oldest first: each input and its result as LaTeX, the
/// result written as the settings say.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Opened {
    pub info: SessionInfo,
    pub history: Vec<(String, String)>,
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
