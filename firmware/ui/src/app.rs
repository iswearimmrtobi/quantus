//! The calculator's state and what each key does to it.

use crate::backend::{Backend, Opened, Reply, Request, SessionCommand, SessionInfo, WlanStatus};
use crate::editor::{Editor, Node, Row};
use crate::key::Key;
use crate::latex;
use crate::text::{self, same, t, Lang, Text};
use crate::tree;
use crate::clock;
use crate::menu::{self, Action, Edit, Kind, Menu, Outcome, TextFor, TimeField};
use crate::render;
use crate::settings::SystemInput;
use crate::sheet::{self, Sheets};
use crate::touch::{self, Calibrating, Calibration, Gesture, Target, Touch, Tracker};
use crate::wallpaper::Wallpaper;
use embedded_graphics::prelude::Point;
pub use crate::settings::Settings;

/// The modes of the MODE menu (design/handheld-menus.md §7.1), in its order: two pages of six.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Physics,
    Algebra,
    Imaginary,
    Analysis,
    Linear,
    Table,
    Matrix,
    BaseN,
    Statistics,
    Distributions,
    Verify,
    /// Not a mode: the tile opens the sessions menu.
    Sessions,
}

impl Mode {
    pub const ALL: [Mode; 12] = [
        Mode::Physics,
        Mode::Algebra,
        Mode::Imaginary,
        Mode::Analysis,
        Mode::Linear,
        Mode::Table,
        Mode::Matrix,
        Mode::BaseN,
        Mode::Statistics,
        Mode::Distributions,
        Mode::Verify,
        Mode::Sessions,
    ];

    /// The name, with a soft hyphen where a tile may break it.
    pub fn text(self) -> Text {
        match self {
            Mode::Physics => t("Physik", "Physics"),
            Mode::Algebra => same("Algebra"),
            Mode::Imaginary => t("Komplex", "Complex"),
            Mode::Analysis => t("Kurven\u{ad}diskussion", "Curve sketching"),
            Mode::Linear => t("LGS", "Linear systems"),
            Mode::Table => t("Werte\u{ad}tabelle", "Table"),
            Mode::Matrix => t("Matrizen", "Matrices"),
            Mode::BaseN => same("Basis-N"),
            Mode::Statistics => t("Statistik", "Statistics"),
            Mode::Distributions => t("Vertei\u{ad}lungen", "Distri\u{ad}butions"),
            Mode::Verify => t("Prüfen", "Verify"),
            Mode::Sessions => t("Sitzungen", "Sessions"),
        }
    }

    /// The name on one line.
    pub fn name(self, lang: Lang) -> String {
        self.text().get(lang).replace('\u{ad}', "")
    }

    /// Whether the engine calculates in this mode; the others have their tile and their name
    /// in the top bar, and EXE says they are not available yet.
    pub fn engine(self) -> bool {
        matches!(self, Mode::Physics | Mode::Algebra | Mode::Imaginary | Mode::Analysis | Mode::Linear)
    }

    /// The sheet the mode shows instead of the input line, if it has one: LGS with its input
    /// set to the matrix, Wertetabelle, Statistik and Verteilungen.
    pub fn sheet(self, settings: &Settings) -> Option<sheet::Kind> {
        match self {
            Mode::Linear if settings.system_input == SystemInput::Matrix => Some(sheet::Kind::Linear),
            Mode::Table => Some(sheet::Kind::Table),
            Mode::Statistics => Some(sheet::Kind::Statistics),
            Mode::Distributions => Some(sheet::Kind::Distribution),
            _ => None,
        }
    }

    fn index(self) -> usize {
        Mode::ALL.iter().position(|&mode| mode == self).unwrap_or(0)
    }
}

/// Tiles per page of the mode menu, in [`MENU_COLUMNS`] columns.
pub const TILES_PER_PAGE: usize = 6;
pub const MENU_COLUMNS: usize = 3;

/// What fills the screen below the top bar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Screen {
    Calc,
    /// The mode menu, with the selected tile counted over all pages.
    Modes { selected: usize },
    /// SETUP or OPTN.
    Menu(Menu),
    /// The touch calibration, and the screen to go back to.
    Calibrate(Calibrating, Box<Screen>),
}

/// A calculation in the history.
pub struct Entry {
    pub input: Row,
    pub result: Result<Row, String>,
    /// Whether the result is written as a decimal; S⇔D flips it.
    pub decimal: bool,
}

/// A calculation the backend is still working on, and where its answer goes.
enum Pending {
    /// A new entry for this input, written as a decimal or not.
    New { input: Row, decimal: bool },
    /// The other form of the entry at this place in the history (oldest first).
    Swap { index: usize, decimal: bool },
    /// The answer to a sheet.
    Sheet,
}

/// A sheet worked out, shown over it until AC.
#[derive(Clone, Debug, PartialEq)]
pub struct Answer {
    pub reply: Result<Reply, String>,
    /// The first line or row on the screen.
    pub scroll: usize,
    /// How far it is moved to the left, in pixels, to see a wide matrix or table.
    pub shift: i32,
    /// The page shown, of a curve sketch: 0 the graph, then a page per section.
    pub page: usize,
}

impl Answer {
    pub fn new(reply: Result<Reply, String>) -> Self {
        Answer { reply, scroll: 0, shift: 0, page: 0 }
    }

    /// How many pages it has: the graph and the sections of a sketch, one otherwise.
    pub fn pages(&self) -> usize {
        match &self.reply {
            Ok(Reply::Analysis(analysis)) => 1 + analysis.pages.len(),
            _ => 1,
        }
    }

    /// The lines or rows of the page shown, the steps of its scrolling.
    pub fn lines(&self) -> usize {
        match &self.reply {
            Ok(Reply::Analysis(analysis)) => self.page.checked_sub(1).and_then(|page| analysis.pages.get(page)).map_or(0, |page| page.len()),
            Ok(reply) => reply.len(),
            Err(_) => 0,
        }
    }

    /// Scrolls by `lines`, staying on the page.
    pub fn scroll_by(&mut self, lines: isize) {
        self.scroll = self.scroll.saturating_add_signed(lines).min(self.lines().saturating_sub(1));
    }

    /// Turns to the page `page`, from its top; false if there is none.
    pub fn turn_to(&mut self, page: usize) -> bool {
        if page >= self.pages() || page == self.page {
            return false;
        }
        (self.page, self.scroll, self.shift) = (page, 0, 0);
        true
    }
}

/// How far ←/→ move a wide answer.
const SHIFT_STEP: i32 = 80;

/// How long the caret stays on and off.
const BLINK_MS: u64 = 500;

/// How long the busy spinner takes per step; eight steps make a turn.
const SPIN_MS: u64 = 125;

/// How often the WLAN connection is asked for.
const WLAN_MS: u64 = 500;

/// Older entries are dropped.
const HISTORY: usize = 50;

/// A drag this far, in pixels, turns the page; a shorter one springs back.
const TURN: i32 = 40;

pub struct App<B> {
    backend: B,
    pub mode: Mode,
    pub settings: Settings,
    /// The settings the backend was last given, to notice a change.
    used: Settings,
    pub screen: Screen,
    pub shift: bool,
    pub alpha: bool,
    pub editor: Editor,
    /// Oldest first.
    pub history: Vec<Entry>,
    /// The entry on the screen, counted from the newest: the editor holds its input and its
    /// result is shown. `None` while an input is being typed.
    pub shown: Option<usize>,
    /// The charge in percent, if the board can tell.
    pub battery: Option<u8>,
    /// Whether the battery is charging, set by the board while USB power is in.
    pub charging: bool,
    /// Whether a calculation is running: a spinner turns in the top bar. Set while the backend
    /// works on an answer (the engine on its own thread), and by the board for a calculation
    /// elsewhere (the remote engine over WLAN).
    pub busy: bool,
    /// The spinner's step, 0 to 7.
    pub spin: u8,
    pub caret_on: bool,
    /// The picture behind the screens, read from the SD card through the backend.
    pub wallpaper: Option<Wallpaper>,
    /// Its name in `/wallpapers/`.
    pub wallpaper_name: Option<String>,
    /// While the list of wallpapers is open: the place in it of the picture shown instead, and
    /// that picture (`None` for none, or one that could not be read).
    pub preview: Option<(usize, Option<Wallpaper>)>,
    /// The session open now, whose history is shown; `None` for a backend without sessions.
    pub session: Option<SessionInfo>,
    /// What LGS, Wertetabelle, Statistik and Verteilungen hold.
    pub sheets: Sheets,
    /// The answer to the sheet on the screen, shown over it.
    pub answer: Option<Answer>,
    /// Turns the panel's readings into screen points; set by the board from what it stored.
    pub calibration: Calibration,
    /// A calibration just made, for the board to store.
    calibrated: Option<Calibration>,
    /// The calculation the backend is working on.
    pending: Option<Pending>,
    touch: Tracker,
    /// What the finger is on, lit until it lifts or drags.
    pressed: Option<Target>,
    last_blink: u64,
    last_spin: u64,
    /// The minute the clock showed at the last tick, to redraw a page that shows it.
    minute: Option<i64>,
    /// Whether WLAN is connected: the fan in the top bar is lit, not dim.
    pub wlan_connected: bool,
    /// When the connection was asked for last; `None` asks at the next tick.
    last_wlan: Option<u64>,
}

impl<B> App<B> {
    /// The sheet the screen shows instead of the input line, if any: the mode's own
    /// ([`Mode::sheet`]).
    pub fn sheet_kind(&self) -> Option<sheet::Kind> {
        self.mode.sheet(&self.settings)
    }

    /// The picture to show behind the screen.
    pub fn wallpaper(&self) -> Option<&Wallpaper> {
        match &self.preview {
            Some((_, preview)) => preview.as_ref(),
            None => self.wallpaper.as_ref(),
        }
    }

    /// The entry whose result is on the screen.
    pub fn shown_entry(&self) -> Option<&Entry> {
        self.history.iter().rev().nth(self.shown?)
    }

    /// What the finger is on, while it is down and has not moved away.
    pub fn pressed(&self) -> Option<Target> {
        self.pressed
    }

    /// How far a finger drags the screen: across the mode tiles, along a menu list.
    pub fn drag(&self) -> Option<Point> {
        let moved = self.touch.drag()?;
        match &self.screen {
            Screen::Modes { .. } => Some(Point::new(moved.x, 0)),
            Screen::Menu(menu) if menu.list().is_some() => Some(Point::new(0, moved.y)),
            Screen::Calc if self.answer.is_some() && self.sheet_kind().is_some() => Some(Point::new(0, moved.y)),
            Screen::Calc if self.answer.is_some() => Some(moved),
            _ => None,
        }
    }

    /// A calibration made since the last call, to be stored.
    pub fn take_calibration(&mut self) -> Option<Calibration> {
        self.calibrated.take()
    }
}

impl<B: Backend> App<B> {
    /// The app with the settings kept from the last run, on the session the backend has open,
    /// with its history.
    pub fn new(mut backend: B) -> Self {
        let settings = backend.load_settings().unwrap_or_default();
        backend.use_settings(&settings);
        let mut app = App {
            backend,
            mode: Mode::Physics,
            settings,
            used: settings,
            screen: Screen::Calc,
            shift: false,
            alpha: false,
            editor: Editor::new(),
            history: Vec::new(),
            shown: None,
            battery: None,
            charging: false,
            busy: false,
            spin: 0,
            caret_on: true,
            wallpaper: None,
            wallpaper_name: None,
            preview: None,
            session: None,
            sheets: Sheets::new(&settings),
            answer: None,
            calibration: Calibration::default(),
            calibrated: None,
            pending: None,
            touch: Tracker::default(),
            pressed: None,
            last_blink: 0,
            last_spin: 0,
            minute: None,
            wlan_connected: false,
            last_wlan: None,
        };
        let settings = app.settings;
        if let Ok(opened) = app.backend.session(SessionCommand::Current, &settings) {
            app.load(opened);
        }
        app
    }

    /// Shows the session `opened`: its history, an empty input.
    fn load(&mut self, opened: Opened) {
        self.cancel();
        let skip = opened.history.len().saturating_sub(HISTORY);
        let decimal = self.settings.decimal;
        self.history = opened
            .history
            .into_iter()
            .skip(skip)
            .map(|(input, result)| Entry { input: latex::parse(&input), result: Ok(latex::parse(&result)), decimal })
            .collect();
        self.session = Some(opened.info);
        self.reset();
        // The sheets as the session left them, laid out as they were.
        self.answer = None;
        self.sheets = Sheets::new(&self.settings);
        for request in &opened.inputs {
            let settings = &mut self.settings;
            match request {
                Request::Linear { rows } => {
                    let width = rows.first().map_or(2, Vec::len);
                    settings.system_rows = rows.len().clamp(1, 10) as u8;
                    settings.system_columns = width.saturating_sub(1).clamp(1, 10) as u8;
                }
                Request::Statistics { kind, freq, .. } => {
                    settings.statistics_type = *kind;
                    settings.statistics_freq = freq.is_some();
                }
                Request::Distribution { kind, .. } => settings.distribution = *kind,
                Request::Table { .. } | Request::Analysis { .. } | Request::Tangent { .. } | Request::Area { .. } => {}
            }
            self.sheets.fill(request, &self.settings);
        }
        self.fit_sheets();
    }

    /// Lays the sheets out as the settings say, and scrolls the one on the screen to its cursor.
    fn fit_sheets(&mut self) {
        self.sheets.fit(&self.settings);
        if let Some(kind) = self.sheet_kind() {
            let sheet = self.sheets.get_mut(kind);
            let (_, columns) = sheet::columns_shown(sheet.columns(), kind.form());
            sheet.follow(sheet::ROWS_SHOWN, columns);
        }
    }

    /// Sends `command` to the backend. A session opened goes to the calculation screen, a renamed
    /// one stays in the menu; a failure says so in the menu.
    fn session_command(&mut self, mut menu: Menu, command: SessionCommand) {
        let rename = matches!(command, SessionCommand::Rename(_));
        if !rename {
            self.cancel();
        }
        let label = menu.title();
        let settings = self.settings;
        match self.backend.session(command, &settings) {
            Ok(opened) if rename => {
                self.session = Some(opened.info);
                menu.edit = None;
                self.screen = Screen::Menu(menu);
            }
            Ok(opened) => {
                self.load(opened);
                self.screen = Screen::Calc;
            }
            Err(_) => {
                menu.edit = Some(Edit::Message { label, message: text::NOT_AVAILABLE });
                self.screen = Screen::Menu(menu);
            }
        }
    }

    /// Opens the list of sessions, on the one open now.
    fn open_sessions(&mut self, menu: &mut Menu, label: Text) {
        let sessions = self.backend.sessions();
        if sessions.is_empty() {
            menu.edit = Some(Edit::Message { label, message: text::NOT_AVAILABLE });
            return;
        }
        let open = self.session.as_ref().and_then(|open| sessions.iter().position(|s| s.id == open.id));
        menu.edit = Some(Edit::Sessions { label, sessions, selected: open.unwrap_or(0) });
    }

    /// Hands changed settings to the backend, takes an answer it has finished, blinks the caret and turns the spinner while
    /// busy; returns whether the screen changed.
    pub fn tick(&mut self, now_ms: u64) -> bool {
        let mut changed = false;
        if self.settings != self.used {
            self.used = self.settings;
            self.backend.use_settings(&self.settings);
            self.last_wlan = None;
        }
        match self.pending {
            Some(Pending::Sheet) => {
                if let Some(reply) = self.backend.poll_solve() {
                    self.finish_sheet(reply);
                    changed = true;
                }
            }
            Some(_) => {
                if let Some(result) = self.backend.poll() {
                    self.finish(result);
                    changed = true;
                }
            }
            None => {}
        }
        changed |= self.wlan_tick(now_ms);
        let minute = clock::now().map(|now| now / 60_000);
        if minute != self.minute {
            self.minute = minute;
            changed |= self.shows_clock();
        }
        if self.busy && now_ms.saturating_sub(self.last_spin) >= SPIN_MS {
            self.last_spin = now_ms;
            self.spin = (self.spin + 1) % 8;
            changed = true;
        }
        if now_ms.saturating_sub(self.last_blink) >= BLINK_MS {
            self.last_blink = now_ms;
            self.caret_on = !self.caret_on;
            changed = true;
        }
        changed
    }

    pub fn key(&mut self, key: Key) {
        let typing = matches!(&self.screen, Screen::Menu(menu) if matches!(menu.edit, Some(Edit::Text { .. })));
        let Some(key) = (if typing { Some(key) } else { key.plain() }) else { return };
        self.caret_on = true;
        let shift = std::mem::take(&mut self.shift);
        let alpha = std::mem::take(&mut self.alpha);
        match std::mem::replace(&mut self.screen, Screen::Calc) {
            Screen::Modes { selected } => self.modes_key(selected, key),
            Screen::Menu(mut menu) => {
                let outcome = menu.key(key, &mut self.settings);
                self.outcome(menu, outcome);
            }
            Screen::Calc => self.calc_key(key, shift, alpha),
            // AC or ← leaves it with the calibration as it was.
            Screen::Calibrate(state, back) => match key {
                Key::Ac | Key::Left => self.screen = *back,
                _ => self.screen = Screen::Calibrate(state, back),
            },
        }
        self.fit_sheets();
    }

    /// The touch calibration screen, back to this screen afterwards.
    pub fn calibrate(&mut self) {
        let back = std::mem::replace(&mut self.screen, Screen::Calc);
        self.screen = Screen::Calibrate(Calibrating::default(), Box::new(back));
    }

    /// What the finger did, as the panel reads it; returns whether the screen may have changed.
    /// What it is on lights up as it comes down; a tap does what the key for its target does, a
    /// drag moves a list or the mode tiles with the finger and turns the page when it lifts.
    pub fn touch(&mut self, touch: Touch) -> bool {
        if let Screen::Calibrate(..) = self.screen {
            return self.calibration_touch(touch);
        }
        let touch = match touch {
            Touch::Press(raw) => Touch::Press(self.calibration.map(raw)),
            Touch::Drag(raw) => Touch::Drag(self.calibration.map(raw)),
            Touch::Release => Touch::Release,
        };
        let dragged = self.drag();
        let gesture = self.touch.feed(touch);
        match touch {
            Touch::Press(point) => {
                self.pressed = render::target(self, point);
                return self.pressed.is_some();
            }
            Touch::Drag(_) => {
                let drag = self.drag();
                let unlit = self.touch.drag().is_some() && self.pressed.take().is_some();
                return unlit || drag != dragged;
            }
            Touch::Release => self.pressed = None,
        }
        self.caret_on = true;
        match gesture {
            Some(Gesture::Tap(point)) => {
                if let Some(target) = render::target(self, point) {
                    self.tap(target);
                }
            }
            Some(Gesture::Drag(moved)) => self.turn(moved),
            None => {}
        }
        self.fit_sheets();
        true
    }

    /// A drag let go: the next page or the one before when it went far enough.
    fn turn(&mut self, moved: Point) {
        match std::mem::replace(&mut self.screen, Screen::Calc) {
            Screen::Modes { selected } => {
                self.screen = Screen::Modes { selected };
                let page = selected / TILES_PER_PAGE;
                if moved.x <= -TURN {
                    self.show_page(page + 1);
                } else if moved.x >= TURN && page > 0 {
                    self.show_page(page - 1);
                }
            }
            Screen::Menu(mut menu) => {
                let outcome = match moved.y {
                    y if y <= -TURN => menu.page(true),
                    y if y >= TURN => menu.page(false),
                    _ => Outcome::Stay,
                };
                self.outcome(menu, outcome);
            }
            Screen::Calc => {
                // Sideways across a curve sketch: the next page or the one before.
                if let Some(answer) = self.answer.as_mut().filter(|answer| answer.pages() > 1) {
                    if moved.x.abs() >= TURN && moved.x.abs() > moved.y.abs() {
                        let page = if moved.x < 0 { answer.page + 1 } else { answer.page.saturating_sub(1) };
                        answer.turn_to(page);
                        return;
                    }
                }
                if let Some(answer) = &mut self.answer {
                    let page = sheet::ROWS_SHOWN as isize;
                    let by = match moved.y {
                        y if y <= -TURN => page,
                        y if y >= TURN => -page,
                        _ => 0,
                    };
                    answer.scroll_by(by);
                }
            }
            screen => self.screen = screen,
        }
    }

    /// The calibration screen: each press is averaged over while it lasts, and when the finger
    /// lifts the next cross comes; after the last one the readings are fitted, and done again if
    /// they are too far off.
    fn calibration_touch(&mut self, touch: Touch) -> bool {
        let Screen::Calibrate(state, _) = &mut self.screen else { return false };
        match touch {
            Touch::Press(raw) | Touch::Drag(raw) => {
                let first = state.sum.2 == 0;
                state.sum = (state.sum.0 + raw.x, state.sum.1 + raw.y, state.sum.2 + 1);
                first
            }
            Touch::Release => {
                let (x, y, count) = std::mem::take(&mut state.sum);
                if count == 0 {
                    return false;
                }
                state.readings.push(Point::new(x / count, y / count));
                if state.readings.len() < touch::TARGETS.len() {
                    return true;
                }
                let pairs: Vec<(Point, Point)> = state.readings.iter().copied().zip(touch::TARGETS).collect();
                match Calibration::fit(&pairs) {
                    Some((calibration, error)) if error <= touch::CALIBRATION_ERROR => {
                        self.calibration = calibration;
                        self.calibrated = Some(calibration);
                        let Screen::Calibrate(_, back) = std::mem::replace(&mut self.screen, Screen::Calc) else {
                            return true;
                        };
                        self.screen = *back;
                    }
                    _ => *state = Calibrating { retry: true, ..Calibrating::default() },
                }
                true
            }
        }
    }

    fn tap(&mut self, target: Target) {
        match target {
            Target::Back if matches!(self.screen, Screen::Modes { .. } | Screen::Calc) => self.key(Key::Ac),
            Target::Cell(row, column) => {
                let Some(kind) = self.sheet_kind() else { return };
                let sheet = self.sheets.get_mut(kind);
                // A tap on the cell under the cursor edits it, one on another cell moves there.
                if sheet.cursor == (row, column) {
                    sheet.edit();
                } else {
                    sheet.select(row, column);
                }
            }
            Target::Back => self.key(Key::Left),
            Target::Side(next) => self.key(if next { Key::Right } else { Key::Left }),
            Target::Ok | Target::Anywhere => self.key(Key::Ok),
            Target::Tile(index) => self.choose(index),
            Target::Page(page) => self.show_page(page),
            Target::Line(_) | Target::Step(_) => {
                let Screen::Menu(mut menu) = std::mem::replace(&mut self.screen, Screen::Calc) else { return };
                let outcome = match target {
                    Target::Line(offset) => menu.tap(offset, &mut self.settings),
                    Target::Step(delta) => {
                        menu.step(delta, &self.settings);
                        Outcome::Stay
                    }
                    _ => Outcome::Stay,
                };
                self.outcome(menu, outcome);
            }
        }
    }

    /// Fills in the networks once the board has found them, and follows the connection, on its
    /// page and in the top bar, twice a second; returns whether the screen changed.
    fn wlan_tick(&mut self, now_ms: u64) -> bool {
        let mut changed = false;
        if let Screen::Menu(menu) = &mut self.screen {
            if let Some(Edit::Networks { networks: networks @ None, .. }) = &mut menu.edit {
                *networks = self.backend.networks();
                changed |= networks.is_some();
            }
        }
        if self.last_wlan.is_some_and(|last| now_ms.saturating_sub(last) < WLAN_MS) {
            return changed;
        }
        self.last_wlan = Some(now_ms);
        let status = if self.settings.wlan { self.backend.wlan_status() } else { WlanStatus::Off };
        if status.connected() != self.wlan_connected {
            self.wlan_connected = status.connected();
            changed = true;
        }
        if let Screen::Menu(menu) = &mut self.screen {
            if let Some(Edit::WlanStatus { status: shown, .. }) = &mut menu.edit {
                if *shown != status {
                    *shown = status;
                    changed = true;
                }
            }
        }
        changed
    }

    /// Whether the screen shows the time: a menu page with the date or the time on it.
    fn shows_clock(&self) -> bool {
        let Screen::Menu(menu) = &self.screen else { return false };
        menu.edit.is_none() && menu.level().page.entries.iter().any(|entry| matches!(entry.kind, Kind::Time(_)))
    }

    /// The mode menu on the first tile of `page`, if there is one.
    fn show_page(&mut self, page: usize) {
        let selected = page * TILES_PER_PAGE;
        if matches!(self.screen, Screen::Modes { .. }) && selected < Mode::ALL.len() {
            self.screen = Screen::Modes { selected };
        }
    }

    /// Does what the open menu asks for after a key or a tap, and puts it back on the screen
    /// unless it closed.
    fn outcome(&mut self, mut menu: Menu, outcome: Outcome) {
        match outcome {
            Outcome::Stay => {}
            Outcome::Close => {
                self.preview = None;
                return;
            }
            Outcome::Insert(latex) => return self.insert(latex),
            Outcome::Act(Action::Calibrate) => {
                self.screen = Screen::Menu(menu);
                return self.calibrate();
            }
            Outcome::Act(action) if action.session() => {
                let command = match action {
                    Action::NewSession => SessionCommand::New,
                    Action::DuplicateSession => SessionCommand::Duplicate,
                    _ => SessionCommand::Delete,
                };
                return self.session_command(menu, command);
            }
            // What is done to a sheet closes OPTN, to show it.
            Outcome::Act(action @ (Action::ClearInput | Action::InsertRow | Action::DeleteRow))
                if self.sheet_kind().is_some() =>
            {
                self.act(action);
                return;
            }
            Outcome::Act(action) => self.act(action),
            Outcome::Sessions(label) => self.open_sessions(&mut menu, label),
            Outcome::OpenSession(index) => {
                let Some(Edit::Sessions { sessions, .. }) = &menu.edit else { return };
                let Some(id) = sessions.get(index).map(|session| session.id.clone()) else { return };
                return self.session_command(menu, SessionCommand::Open(id));
            }
            Outcome::NameOf(label) => {
                let typed = self.session.as_ref().map(|session| session.name.clone()).unwrap_or_default();
                menu.edit = Some(Edit::Text { label, purpose: TextFor::Name, typed, error: false, plain: true });
            }
            Outcome::Rename => {
                let Some(Edit::Text { typed, .. }) = &menu.edit else { return };
                let name = typed.trim().to_string();
                return self.session_command(menu, SessionCommand::Rename(name));
            }
            Outcome::Pictures(label) => self.open_pictures(&mut menu, label),
            Outcome::Preview(index) => {
                let picture = self.picture(&menu, index);
                self.preview = Some((index, picture));
            }
            Outcome::Keep(index) => self.keep_picture(&mut menu, index),
            Outcome::Restore => self.preview = None,
            Outcome::Storage(label) => menu.edit = Some(Edit::Storage { label, storage: self.backend.storage() }),
            Outcome::SetTime(field) => self.set_time(&mut menu, field),
            Outcome::Networks(label) if !self.settings.wlan => {
                menu.edit = Some(Edit::Message { label, message: text::WLAN_OFF });
            }
            Outcome::Networks(label) => {
                self.backend.scan();
                menu.edit = Some(Edit::Networks { label, networks: None, selected: 0 });
            }
            Outcome::Network(index) => {
                let Some(Edit::Networks { label, networks: Some(networks), .. }) = &menu.edit else { return };
                let (label, network) = (*label, networks[index].clone());
                if network.saved || !network.secured {
                    self.backend.connect(&network.ssid, None);
                    menu.edit = Some(Edit::WlanStatus { label, status: self.backend.wlan_status() });
                } else {
                    let purpose = TextFor::Password(network.ssid);
                    menu.edit = Some(Edit::Text { label, purpose, typed: String::new(), error: false, plain: false });
                }
            }
            Outcome::Connect => {
                let Some(Edit::Text { label, purpose: TextFor::Password(ssid), typed, .. }) = &menu.edit else { return };
                let label = *label;
                self.backend.connect(ssid, Some(typed));
                menu.edit = Some(Edit::WlanStatus { label, status: self.backend.wlan_status() });
            }
            Outcome::SavedNetworks(label) => {
                let names = self.backend.saved_networks();
                menu.edit = Some(match names.is_empty() {
                    true => Edit::Message { label, message: text::NO_SAVED_NETWORKS },
                    false => Edit::Saved { label, names, selected: 0 },
                });
            }
            Outcome::Forget => {
                let Some(Edit::Forget { label, ssid }) = &menu.edit else { return };
                let label = *label;
                self.backend.forget(ssid);
                menu.edit = Some(Edit::Message { label, message: text::DONE });
            }
            Outcome::WlanStatus(label) => {
                menu.edit = Some(Edit::WlanStatus { label, status: self.backend.wlan_status() });
            }
        }
        self.screen = Screen::Menu(menu);
    }

    /// Sets the clock to the date or the time typed in `menu`, keeping the other from the clock
    /// as it is now (noon for a date typed before the clock was set). Typed by hand, the time
    /// comes from now on from the hand, not the network.
    fn set_time(&mut self, menu: &mut Menu, field: TimeField) {
        let Some(Edit::Time { label, typed, .. }) = &menu.edit else { return };
        let zone = self.settings.zone;
        let now = clock::now().map(|now| clock::local(now, zone));
        let utc = match field {
            TimeField::Date => {
                let Some((year, month, day)) = menu::typed_date(typed) else { return };
                let (hour, minute) = now.map_or((12, 0), |now| (now.hour, now.minute));
                clock::to_utc(year, month, day, hour, minute, zone)
            }
            TimeField::Clock => {
                let Some((hour, minute)) = menu::typed_clock(typed) else { return };
                let Some(now) = now else {
                    menu.edit = Some(Edit::Message { label: *label, message: text::DATE_FIRST });
                    return;
                };
                clock::to_utc(now.year, now.month, now.day, hour, minute, zone)
            }
        };
        self.backend.set_time(utc);
        self.settings.time_from_network = false;
        self.minute = clock::now().map(|now| now / 60_000);
        menu.edit = None;
    }

    /// Does what a menu entry asks for.
    fn act(&mut self, action: Action) {
        match action {
            Action::ResetSettings => self.settings = Settings::default(),
            Action::ResetMemory => self.clear_memory(),
            Action::ResetAll => {
                self.settings = Settings::default();
                self.clear_memory();
            }
            Action::ClearInput => match self.sheet_kind() {
                Some(kind) => {
                    self.sheets.get_mut(kind).clear();
                    self.answer = None;
                }
                None => self.reset(),
            },
            Action::InsertRow | Action::DeleteRow => {
                if let Some(kind) = self.sheet_kind() {
                    let sheet = self.sheets.get_mut(kind);
                    match action {
                        Action::InsertRow => sheet.insert_row(),
                        _ => sheet.delete_row(),
                    }
                    self.answer = None;
                }
            }
            // Done by `outcome`, which knows the menu to come back to.
            Action::Calibrate | Action::NewSession | Action::DuplicateSession | Action::DeleteSession => {}
        }
    }

    /// The history cleared, in the open session too, and the input.
    fn clear_memory(&mut self) {
        self.cancel();
        self.history.clear();
        self.reset();
        let settings = self.settings;
        if let Ok(opened) = self.backend.session(SessionCommand::ClearHistory, &settings) {
            self.session = Some(opened.info);
        }
    }

    /// Opens the list of wallpapers on the SD card, on the one set now.
    fn open_pictures(&mut self, menu: &mut Menu, label: Text) {
        let names = self.backend.wallpapers();
        if names.is_empty() {
            menu.edit = Some(Edit::Message { label, message: text::NO_PICTURES });
            return;
        }
        let selected = self.wallpaper_name.as_ref().and_then(|name| names.iter().position(|n| n == name)).map_or(0, |i| i + 1);
        menu.edit = Some(Edit::Pictures { label, names, selected });
    }

    /// The name at `index` of the open list of wallpapers; 0 is none.
    fn picture_name(menu: &Menu, index: usize) -> Option<String> {
        let Some(Edit::Pictures { names, .. }) = &menu.edit else { return None };
        names.get(index.checked_sub(1)?).cloned()
    }

    /// Reads and decodes the wallpaper at `index` of the open list.
    fn picture(&mut self, menu: &Menu, index: usize) -> Option<Wallpaper> {
        let name = Self::picture_name(menu, index)?;
        self.backend.picture(&name)
    }

    /// Sets the wallpaper at `index` of the open list, or says why it cannot.
    fn keep_picture(&mut self, menu: &mut Menu, index: usize) {
        let name = Self::picture_name(menu, index);
        let picture = match self.preview.take() {
            Some((shown, picture)) if shown == index => picture,
            _ => self.picture(menu, index),
        };
        let Some(Edit::Pictures { label, .. }) = menu.edit.take() else { return };
        if name.is_some() && picture.is_none() {
            menu.edit = Some(Edit::Message { label, message: text::BAD_PICTURE });
            return;
        }
        self.wallpaper = picture;
        self.wallpaper_name = name;
    }

    /// Puts LaTeX from a menu into the input, as if typed: a shown result is left first.
    fn insert(&mut self, latex: &str) {
        // The input is kept as it was sent until the answer comes.
        if self.pending.is_some() {
            return;
        }
        let row = latex::parse(latex);
        // Into the cell at the cursor, on a sheet.
        let editor = match self.sheet_kind() {
            Some(kind) => {
                self.answer = None;
                let sheet = self.sheets.get_mut(kind);
                sheet.edit();
                sheet.editing.as_mut().expect("the cell is being edited")
            }
            None => {
                if self.shown.is_some() {
                    self.reset();
                }
                &mut self.editor
            }
        };
        editor.paste(&row);
        // Into a pair of brackets or bars: `\sinh(|)`, `|x|`.
        if matches!(row.0.last(), Some(Node::Char(')' | '|'))) && row.0.len() > 1 {
            editor.left();
        }
    }

    fn calc_key(&mut self, key: Key, shift: bool, alpha: bool) {
        match key {
            Key::Shift => return self.shift = !shift,
            Key::Alpha => return self.alpha = !alpha,
            Key::Mode if shift => return self.screen = Screen::Menu(Menu::new(&tree::SETUP)),
            Key::Mode => return self.screen = Screen::Modes { selected: self.mode.index() },
            Key::Optn => return self.screen = Screen::Menu(Menu::new(tree::optn(self.mode))),
            _ => {}
        }
        // While the backend calculates, AC drops its answer and the other keys wait.
        if self.pending.is_some() {
            if key == Key::Ac {
                self.cancel();
            }
            return;
        }
        if let Some(kind) = self.sheet_kind() {
            return self.sheet_key(kind, key, shift);
        }
        if self.answer.is_some() && !self.sketch_key(key) {
            return;
        }
        if self.shown.is_some() && !self.leave_result(key) {
            return;
        }
        let editor = &mut self.editor;
        if editor.key(key, shift) {
            return;
        }
        match key {
            Key::Ans => {
                if let Some(Entry { result: Ok(result), .. }) = self.history.last() {
                    editor.paste(result);
                }
            }
            Key::Left => {
                editor.left();
            }
            Key::Right => {
                editor.right();
            }
            Key::Up => {
                if !editor.up() {
                    self.recall(0);
                }
            }
            Key::Down => {
                editor.down();
            }
            Key::Del => editor.delete(),
            Key::Ac => editor.clear(),
            Key::Ok | Key::Exe => self.evaluate(),
            _ => {}
        }
    }

    /// A key on a sheet, or on its answer: there ↑/↓ scroll, ←/→ move a wide one sideways,
    /// and AC, DEL, OK or EXE go back to the sheet.
    fn sheet_key(&mut self, kind: sheet::Kind, key: Key, shift: bool) {
        if let Some(answer) = &mut self.answer {
            match key {
                Key::Up => answer.scroll_by(-1),
                Key::Down => answer.scroll_by(1),
                Key::Left => answer.shift = (answer.shift - SHIFT_STEP).max(0),
                Key::Right => answer.shift += SHIFT_STEP,
                Key::Ac | Key::Del | Key::Ok | Key::Exe => self.answer = None,
                _ => {}
            }
            return;
        }
        if self.sheets.get_mut(kind).key(key, shift) == sheet::Outcome::Calculate {
            let request = self.sheets.request(kind);
            self.solve(&request);
        }
    }

    /// Asks the backend to work `request` out; the answer comes now or when `tick` gets it.
    fn solve(&mut self, request: &Request) {
        let settings = self.settings;
        self.pending = Some(Pending::Sheet);
        match self.backend.start_solve(&settings, request) {
            Some(reply) => self.finish_sheet(reply),
            None => self.busy = true,
        }
    }

    /// A key on a curve sketch: ←/→ turn its pages, ↑/↓ scroll one, AC, DEL, OK and EXE go
    /// back to the input. Another key leaves it too and goes on to the input; returns whether
    /// it does.
    fn sketch_key(&mut self, key: Key) -> bool {
        let Some(answer) = &mut self.answer else { return true };
        match key {
            Key::Left => {
                answer.turn_to(answer.page.saturating_sub(1));
            }
            Key::Right => {
                answer.turn_to(answer.page + 1);
            }
            Key::Up => answer.scroll_by(-1),
            Key::Down => answer.scroll_by(1),
            Key::Ac | Key::Del | Key::Ok | Key::Exe => self.answer = None,
            _ => {
                self.answer = None;
                return true;
            }
        }
        false
    }

    /// Shows the answer to the sheet.
    fn finish_sheet(&mut self, reply: Result<Reply, String>) {
        self.busy = false;
        if let Some(Pending::Sheet) = self.pending.take() {
            self.answer = Some(Answer::new(reply));
        }
    }

    /// A key pressed while a result is shown, as on a Casio: an operator goes on with the result,
    /// ←/→/DEL edit the input again, ↑/↓ step through the history, other keys start afresh.
    /// Returns whether the key still goes to the editor.
    fn leave_result(&mut self, key: Key) -> bool {
        let Some(shown) = self.shown else { return true };
        match key {
            Key::Up => {
                self.recall(shown + 1);
                false
            }
            Key::Down => {
                match shown {
                    0 => self.reset(),
                    _ => self.recall(shown - 1),
                }
                false
            }
            Key::Ok | Key::Exe => false,
            Key::SwapForm => {
                self.swap_form(shown);
                false
            }
            Key::Left | Key::Right => {
                self.shown = None;
                false
            }
            Key::Del => {
                self.shown = None;
                true
            }
            Key::Ac => {
                self.reset();
                false
            }
            Key::Plus | Key::Minus | Key::Times | Key::Divide | Key::Power | Key::Square | Key::Fraction => {
                let result = self.shown_entry().and_then(|entry| entry.result.as_ref().ok()).cloned();
                self.reset();
                if let Some(result) = result {
                    self.editor.paste(&result);
                }
                true
            }
            _ => {
                self.reset();
                true
            }
        }
    }

    /// An empty input, no result.
    fn reset(&mut self) {
        self.editor.clear();
        self.shown = None;
    }

    /// The mode menu: ←/→ through the tiles of all pages, ↑/↓ by a row, across pages too; a
    /// digit picks tiles 1–9, the others need the cursor.
    fn modes_key(&mut self, selected: usize, key: Key) {
        let count = Mode::ALL.len();
        let selected = match key {
            Key::Digit(digit @ 1..=9) if usize::from(digit) <= count => {
                return self.choose(usize::from(digit) - 1);
            }
            Key::Ok | Key::Exe => return self.choose(selected),
            Key::Ac | Key::Mode | Key::Del => return self.screen = Screen::Calc,
            Key::Left => selected.saturating_sub(1),
            Key::Right => (selected + 1).min(count - 1),
            Key::Up if selected >= MENU_COLUMNS => selected - MENU_COLUMNS,
            Key::Down if selected + MENU_COLUMNS < count => selected + MENU_COLUMNS,
            _ => selected,
        };
        self.screen = Screen::Modes { selected };
    }

    fn choose(&mut self, index: usize) {
        match Mode::ALL[index] {
            Mode::Sessions => self.screen = Screen::Menu(Menu::new(&tree::SESSIONS)),
            mode => {
                if mode != self.mode {
                    self.answer = None;
                }
                self.mode = mode;
                self.screen = Screen::Calc;
            }
        }
    }

    /// Shows the entry `index` places back from the newest, its input in the editor.
    fn recall(&mut self, index: usize) {
        let Some(entry) = self.history.iter().rev().nth(index) else { return };
        self.editor.set(entry.input.clone());
        self.shown = Some(index);
    }

    /// Evaluates the shown entry again, as a decimal if it was exact and the other way round.
    fn swap_form(&mut self, shown: usize) {
        let index = self.history.len() - 1 - shown;
        let entry = &self.history[index];
        let settings = Settings { decimal: !entry.decimal, ..self.settings };
        let latex = entry.input.to_latex();
        self.start(Pending::Swap { index, decimal: settings.decimal }, &settings, &latex);
    }

    fn evaluate(&mut self) {
        if self.editor.is_empty() {
            return;
        }
        // Kurvendiskussion sketches the function, a page per section.
        if self.mode == Mode::Analysis {
            let request = Request::Analysis { f: self.editor.to_latex() };
            return self.solve(&request);
        }
        let pending = Pending::New { input: self.editor.row.clone(), decimal: self.settings.decimal };
        if self.mode.engine() {
            let settings = self.settings;
            self.start(pending, &settings, &self.editor.to_latex());
        } else {
            self.pending = Some(pending);
            self.finish(Err(text::NOT_AVAILABLE.get(self.settings.language).to_string()));
        }
    }

    /// Asks the backend; the answer goes where `pending` says, now or when `tick` gets it.
    fn start(&mut self, pending: Pending, settings: &Settings, latex: &str) {
        let answer = match pending {
            Pending::New { .. } => self.backend.start(self.mode, settings, latex),
            Pending::Swap { .. } | Pending::Sheet => self.backend.recalculate(self.mode, settings, latex),
        };
        self.pending = Some(pending);
        match answer {
            Some(result) => self.finish(result),
            None => self.busy = true,
        }
    }

    /// Writes the answer to the pending calculation into the history and shows it.
    fn finish(&mut self, result: Result<String, String>) {
        self.busy = false;
        let result = result.map(|latex| latex::parse(&latex));
        match self.pending.take() {
            Some(Pending::New { input, decimal }) => {
                self.history.push(Entry { input: input.clone(), result, decimal });
                if self.history.len() > HISTORY {
                    self.history.remove(0);
                }
                self.editor.set(input);
                self.shown = Some(0);
            }
            Some(Pending::Swap { index, decimal }) => {
                if let Some(entry) = self.history.get_mut(index) {
                    entry.result = result;
                    entry.decimal = decimal;
                }
            }
            Some(Pending::Sheet) | None => {}
        }
    }

    /// Drops the pending calculation: its answer never shows, the input stays as it was.
    fn cancel(&mut self) {
        if self.pending.take().is_some() {
            self.backend.drop_pending();
            self.busy = false;
        }
    }
}
