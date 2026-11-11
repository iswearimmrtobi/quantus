//! The calculator's state and what each key does to it.

use crate::applet::{AppInfo, AppKey, Applet, Event, Grid, Host, Input, Question, Response};
use crate::backend::{
    Backend, Catalogue, Graph, Group, Item, Opened, Reply, Request, SessionCommand, SessionInfo, Units, VariableCommand, Window,
    WlanStatus,
};
use crate::editor::{Editor, Node, Row};
use crate::key::Key;
use crate::latex;
use crate::layer::{self, Layer, Modifier, Resolved};
use crate::text::{self, same, tx, Lang, Text};
use crate::tree;
use crate::clock;
use crate::menu::{self, Action, Edit, Kind, Level, Menu, Outcome, TextFor, TimeField};
use crate::search;
use crate::render::{self, colors::{self, Palette}};
use crate::settings::{Setting, SystemInput};
use crate::soft::{self, Slot};
use crate::sheet::{self, Sheets};
use crate::touch::{self, Calibrating, Calibration, Gesture, Target, Touch, Tracker};
use crate::wallpaper::Wallpaper;
use embedded_graphics::prelude::Point;
pub use crate::settings::Settings;

/// The modes of the MODE menu (design/handheld-menus.md §7.1), in its order: two pages of six
/// and a third with the apps and the setup.
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
    /// Not a mode: the tile opens the list of apps.
    Apps,
    /// Not a mode: the tile opens SETUP, as SHIFT + MODE does.
    Setup,
}

impl Mode {
    pub const ALL: [Mode; 14] = [
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
        Mode::Apps,
        Mode::Setup,
    ];

    /// The name, with a soft hyphen where a tile may break it.
    pub fn text(self) -> Text {
        match self {
            Mode::Physics => tx!("hui.app.physics"),
            Mode::Algebra => same("Algebra"),
            Mode::Imaginary => tx!("hui.app.complex"),
            Mode::Analysis => tx!("hui.app.curve_sketching"),
            Mode::Linear => tx!("hui.app.linear_systems"),
            Mode::Table => tx!("hui.app.table"),
            Mode::Matrix => tx!("hui.app.matrices"),
            Mode::BaseN => tx!("hui.base_n"),
            Mode::Statistics => tx!("hui.app.statistics"),
            Mode::Distributions => tx!("hui.app.distri_butions"),
            Mode::Verify => tx!("hui.app.verify"),
            Mode::Sessions => tx!("hui.app.sessions"),
            Mode::Apps => tx!("hui.app.apps"),
            Mode::Setup => tx!("hui.app.setup"),
        }
    }

    /// The name on one line.
    pub fn name(self, lang: Lang) -> String {
        self.text().get(lang).replace('\u{ad}', "")
    }

    /// Whether the engine evaluates the input line in this mode; the others show a sheet
    /// instead ([`Mode::sheet`]), worked out by [`Backend::solve`].
    pub fn engine(self) -> bool {
        matches!(
            self,
            Mode::Physics
                | Mode::Algebra
                | Mode::Imaginary
                | Mode::Analysis
                | Mode::Linear
                | Mode::Matrix
                | Mode::BaseN
                | Mode::Verify
        )
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
    /// An app, which [`App::running`] holds.
    App,
}

/// The app on the screen: the applet, what the list said of it, and the screen it was opened
/// from.
pub struct Running {
    pub applet: Box<dyn Applet>,
    pub info: AppInfo,
    /// The app's menu (OPTN), over it: the applet gets no keys while it is open.
    pub menu: Option<Menu>,
    pub back: Box<Screen>,
    /// The state of the input template, the hui's.
    pub template: Template,
}

/// What the input templates keep while an app runs (apps.md §4.4).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Template {
    /// The grid's cursor: its column and row.
    pub cursor: (usize, usize),
    /// The list's selected item.
    pub list: usize,
    /// A question open over the app; it gets nothing until it is answered.
    pub question: Option<Question>,
    /// The app asked to be left.
    pub exit: bool,
}

/// What an app asks of the device, answered over the backend.
struct AppHost<'a, B> {
    backend: &'a mut B,
    lang: Lang,
    template: &'a mut Template,
}

impl<B: Backend> Host for AppHost<'_, B> {
    fn random(&mut self) -> u32 {
        self.backend.random()
    }

    fn lang(&self) -> Lang {
        self.lang
    }

    fn cursor(&self) -> (usize, usize) {
        self.template.cursor
    }

    fn set_cursor(&mut self, col: usize, row: usize) {
        self.template.cursor = (col, row);
    }

    fn exit(&mut self) {
        self.template.exit = true;
    }

    fn confirm(&mut self, question: &str) {
        self.template.question = Some(Question::Confirm { text: question.to_string(), yes: true });
    }
}

/// A calculation in the history.
pub struct Entry {
    pub input: Row,
    pub result: Result<Row, String>,
    /// Whether the result is written as a decimal; S⇔D flips it.
    pub decimal: bool,
    /// The steps ENG took the result to (0 after the first press, −1 after ←ENG), if it is
    /// written in engineering notation; S⇔D ends it.
    pub eng: Option<i32>,
}

/// A calculation the backend is still working on, and where its answer goes.
enum Pending {
    /// A new entry for this input, written as a decimal or not.
    New { input: Row, decimal: bool },
    /// The other form of the entry at this place in the history (oldest first).
    Swap { index: usize, decimal: bool },
    /// The answer to a sheet.
    Sheet,
    /// The curve of the graph on the screen, moved or zoomed.
    Curve,
    /// The letters of the input, for CALC's or SOLVE's form.
    Variables { mode: Mode, solve: bool },
    /// CALC's answer: a new entry for this input.
    Formula { input: Row },
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
    /// Whether the arrows move the graph on the screen, + and − zoom it.
    pub moving: bool,
    /// The graph's window before it was first moved, for 0.
    pub home: Option<Window>,
}

impl Answer {
    pub fn new(reply: Result<Reply, String>) -> Self {
        Answer { reply, scroll: 0, shift: 0, page: 0, moving: false, home: None }
    }

    /// How many pages it has: the graph and the sections of a sketch, one otherwise.
    pub fn pages(&self) -> usize {
        match &self.reply {
            Ok(Reply::Analysis(analysis)) => 1 + analysis.pages.len(),
            Ok(Reply::Task(task)) if task.graph.is_some() => 2,
            _ => 1,
        }
    }

    /// The steps and the result lines of the page shown, if it is a section of a sketch or the
    /// working of a task.
    pub fn working(&self) -> Option<(usize, usize)> {
        match &self.reply {
            Ok(Reply::Analysis(analysis)) => {
                let page = analysis.pages.get(self.page.checked_sub(1)?)?;
                Some((page.lines.len(), page.result.len()))
            }
            Ok(Reply::Task(task)) if self.page == 0 => Some((task.lines.len(), task.result.len())),
            _ => None,
        }
    }

    /// The lines or rows of the page shown, the steps of its scrolling. A page with working
    /// opens on its result alone (scroll 0); from 1 on it shows the working, the result again
    /// at its end.
    pub fn lines(&self) -> usize {
        match (self.working(), &self.reply) {
            (Some((0, result)), _) => result,
            (Some((steps, result)), _) => 1 + steps + result,
            (None, Ok(Reply::Analysis(_) | Reply::Task(_))) => 0,
            (None, Ok(reply)) => reply.len(),
            (None, Err(_)) => 0,
        }
    }

    /// The graph on the page shown, if the page is one: a sketch's first, a task's second.
    pub fn graph(&self) -> Option<&Graph> {
        match &self.reply {
            Ok(Reply::Analysis(analysis)) if self.page == 0 => Some(&analysis.graph),
            Ok(Reply::Task(task)) if self.page == 1 => task.graph.as_ref(),
            _ => None,
        }
    }

    fn graph_mut(&mut self) -> Option<&mut Graph> {
        match &mut self.reply {
            Ok(Reply::Analysis(analysis)) if self.page == 0 => Some(&mut analysis.graph),
            Ok(Reply::Task(task)) if self.page == 1 => task.graph.as_mut(),
            _ => None,
        }
    }

    /// The function the graph is of, as the engine reads it.
    fn function(&self) -> Option<String> {
        match &self.reply {
            Ok(Reply::Analysis(analysis)) => Some(analysis.function.clone()),
            Ok(Reply::Task(task)) => Some(task.function.clone()),
            _ => None,
        }
    }

    /// Moves the graph's window to `window`; returns whether it is a graph page.
    pub fn move_to(&mut self, window: Window) -> bool {
        let Some(graph) = self.graph_mut() else { return false };
        let old = std::mem::replace(&mut graph.window, window);
        self.home.get_or_insert(old);
        true
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
    /// The waiting SHIFT, ALPHA and UNIT, and the lock below them.
    pub modifier: Modifier,
    /// What COPY took from the input, for PASTE.
    clipboard: Row,
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
    /// The colour theme set. Its colours are in force ([`colors::set`]) unless the list of
    /// themes shows another.
    pub theme: quantus_themes::Theme,
    /// While the list of themes is open: the themes in it, read once.
    themes: Vec<quantus_themes::Entry>,
    /// The session open now, whose history is shown; `None` for a backend without sessions.
    pub session: Option<SessionInfo>,
    /// What LGS, Wertetabelle, Statistik and Verteilungen hold.
    pub sheets: Sheets,
    /// The answer to the sheet on the screen, shown over it.
    pub answer: Option<Answer>,
    /// The form of Kurvendiskussion open over the input: the tangent's, the area's, the table's.
    pub task: Option<sheet::Kind>,
    /// The last curve sketch and the input it is of, for OPTN → Abschnitte and to draw a
    /// task's lines on its graph.
    pub sketch: Option<(String, crate::backend::Analysis)>,
    /// The function of the request the backend works on.
    asked: Option<String>,
    /// Whether the graph moved while its curve was sampled: it is asked for again.
    resample_again: bool,
    /// The function of Kurvendiskussion the open session kept, for the input when the mode is
    /// chosen.
    kept_function: Option<Row>,
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
    /// How many soft keys a row has: 6 on the big unit, 5 on the small one.
    pub soft_columns: usize,
    /// The page of the soft keys shown; back to the first with any other key.
    pub soft_page: usize,
    /// The page of SYMB's symbols on the soft keys, while they are open.
    pub symbols: Option<usize>,
    /// The units Physik's soft keys offer, and what they were asked for.
    pub units: Units,
    units_for: Option<String>,
    /// The labels of the units typed lately, newest first, for the order of the unit bar.
    pub recent_units: Vec<String>,
    /// The units typed into the input since the last EXE, and a unit being typed on the UNIT
    /// layer (its prefix and symbol).
    typed_units: Vec<String>,
    unit_run: String,
    /// What CATALOG's search looks through, built when it is first used in a language and mode.
    search: Option<search::Index>,
    /// The app on the screen while [`Screen::App`] shows.
    pub running: Option<Running>,
}

/// How many recent units the unit bar remembers.
const RECENT_UNITS: usize = 12;

impl<B> App<B> {
    /// The sheet the screen shows instead of the input line, if any: the mode's own
    /// ([`Mode::sheet`]), or in Kurvendiskussion the form of a task.
    pub fn sheet_kind(&self) -> Option<sheet::Kind> {
        if self.sheets.asked.is_some() {
            return Some(sheet::Kind::Formula);
        }
        if self.sheets.defining.is_some() {
            return Some(sheet::Kind::Matrix);
        }
        if self.mode == Mode::Statistics && self.sheets.estimating {
            return Some(sheet::Kind::Estimates);
        }
        match self.mode {
            Mode::Analysis => self.task,
            mode => mode.sheet(&self.settings),
        }
    }

    /// The place of the theme set in the open list of themes.
    pub fn theme_index(&self) -> Option<usize> {
        self.themes.iter().position(|entry| entry.id == self.theme.id)
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
            // A graph moves with the finger either way.
            Screen::Calc if self.answer.as_ref().is_some_and(|answer| answer.graph().is_some()) => Some(moved),
            Screen::Calc if self.answer.is_some() && self.sheet_kind().is_some() => Some(Point::new(0, moved.y)),
            Screen::Calc if self.answer.is_some() => Some(moved),
            _ => None,
        }
    }

    /// What computes the results.
    pub fn backend(&self) -> &B {
        &self.backend
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
        // The theme kept, if the card still has it; else the default one.
        let theme = backend
            .load_theme()
            .and_then(|id| quantus_themes::find(&id, backend.themes()).ok())
            .unwrap_or_else(quantus_themes::default_theme);
        colors::set(Palette::from_theme(&theme));
        let mut app = App {
            backend,
            mode: Mode::Physics,
            settings,
            used: settings,
            screen: Screen::Calc,
            modifier: Modifier::default(),
            clipboard: Row::default(),
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
            theme,
            themes: Vec::new(),
            session: None,
            sheets: Sheets::new(&settings),
            answer: None,
            task: None,
            sketch: None,
            asked: None,
            resample_again: false,
            kept_function: None,
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
            soft_columns: 6,
            soft_page: 0,
            symbols: None,
            units: Units::default(),
            units_for: None,
            recent_units: Vec::new(),
            typed_units: Vec::new(),
            unit_run: String::new(),
            search: None,
            running: None,
        };
        let settings = app.settings;
        if let Ok(opened) = app.backend.session(SessionCommand::Current, &settings) {
            app.load(opened);
        }
        app.refresh_units();
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
            .map(|(input, result)| Entry { input: latex::parse(&input), result: Ok(latex::parse(&result)), decimal, eng: None })
            .collect();
        self.session = Some(opened.info);
        self.reset();
        self.recent_units.clear();
        self.typed_units.clear();
        // The sheets as the session left them, laid out as they were.
        self.answer = None;
        (self.task, self.sketch, self.kept_function) = (None, None, None);
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
                Request::Analysis { f } => self.kept_function = Some(latex::parse(f)),
                Request::Table { .. }
                | Request::Tangent { .. }
                | Request::Area { .. }
                | Request::Sample { .. }
                | Request::Variables { .. }
                | Request::Calc { .. }
                | Request::Solve { .. } => {}
            }
            self.sheets.fill(request, &self.settings);
        }
        self.fit_sheets();
        self.restore_function();
    }

    /// In Kurvendiskussion with nothing typed, the function the session kept goes into the input.
    fn restore_function(&mut self) {
        if self.mode == Mode::Analysis && self.editor.is_empty() && self.shown.is_none() {
            if let Some(function) = self.kept_function.take() {
                self.editor.set(function);
            }
        }
    }

    /// Lays the sheets out as the settings say, and scrolls the one on the screen to its cursor.
    fn fit_sheets(&mut self) {
        self.sheets.fit(&self.settings);
        if let Some(kind) = self.sheet_kind() {
            let sheet = self.sheets.get_mut(kind);
            let (_, columns) = sheet::columns_shown(sheet.columns(), kind.form());
            sheet.follow(kind.rows_shown(), columns);
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
            Some(Pending::Curve) => {
                if let Some(reply) = self.backend.poll_solve() {
                    self.finish_curve(reply);
                    changed = true;
                }
            }
            Some(Pending::Variables { .. } | Pending::Formula { .. }) => {
                if let Some(reply) = self.backend.poll_solve() {
                    self.finish_formula(reply);
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
        // Prüfungsmodus ends a running app.
        if self.settings.exam && self.running.is_some() {
            self.close_app();
            self.screen = Screen::Calc;
            changed = true;
        }
        if let Some((applet, mut host)) = self.applet() {
            changed |= applet.tick(now_ms, &mut host);
        }
        if self.running.as_ref().is_some_and(|running| running.template.exit) {
            self.app_exit();
            changed = true;
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
        changed |= self.refresh_units();
        changed
    }

    pub fn key(&mut self, key: Key) {
        self.press(key);
        self.app_exit();
        self.refresh_units();
    }

    fn press(&mut self, key: Key) {
        let typing = matches!(&self.screen, Screen::Menu(menu) if matches!(
            menu.edit,
            Some(Edit::Text { .. } | Edit::Catalogue { catalogue: Catalogue::Functions, .. })
        ));
        let Some(key) = (if typing { Some(key) } else { key.plain() }) else { return };
        self.caret_on = true;
        if matches!(self.screen, Screen::App) {
            return self.app_key(key);
        }
        let calc = matches!(self.screen, Screen::Calc);
        if calc && Modifier::is_modifier(key) {
            return self.modifier.press(key);
        }
        // Any other key closes SYMB's symbols and goes back to the first page of soft keys.
        if !matches!(key, Key::Soft(_) | Key::Symb) {
            self.symbols = None;
            self.soft_page = 0;
        }
        let layer = self.modifier.layer();
        let resolved = layer::resolve(key, layer, self.modifier.exponent, self.mode == Mode::BaseN);
        self.modifier.settle(key, layer);
        if calc {
            self.track_unit(layer, resolved);
        }
        let key = match resolved {
            Resolved::Key(key) => key,
            Resolved::Insert(latex) if calc => return self.insert(latex),
            Resolved::Nothing => return,
            // Menus and the other screens have no cursor to insert at.
            Resolved::Insert(_) => return,
        };
        let shift = layer == Layer::Shift;
        // UNDO and REDO step through the input as it was, also back out of a shown result.
        if calc && matches!(key, Key::Undo | Key::Redo) {
            if self.pending.is_none() && self.sheet_kind().is_none() && self.answer.is_none() {
                self.shown = None;
                if key == Key::Undo { self.editor.undo() } else { self.editor.redo() }
            }
            return;
        }
        let before = self.editor.snapshot();
        match std::mem::replace(&mut self.screen, Screen::Calc) {
            Screen::Modes { selected } => self.modes_key(selected, key),
            Screen::Menu(mut menu) => {
                let outcome = menu.key(key, &mut self.settings);
                self.outcome(menu, outcome);
            }
            Screen::Calc => self.calc_key(key, shift),
            // AC or ← leaves it with the calibration as it was.
            Screen::Calibrate(state, back) => match key {
                Key::Ac | Key::Left => self.screen = *back,
                _ => self.screen = Screen::Calibrate(state, back),
            },
            Screen::App => self.screen = Screen::App,
        }
        self.fit_sheets();
        self.editor.commit(before);
    }

    /// Opens `applet` over the screen shown now, which AC goes back to.
    pub fn open_applet(&mut self, applet: Box<dyn Applet>, info: AppInfo) {
        self.close_app();
        let back = std::mem::replace(&mut self.screen, Screen::App);
        self.running = Some(Running { applet, info, menu: None, back: Box::new(back), template: Template::default() });
        self.soft_page = 0;
        if let Some((applet, mut host)) = self.applet() {
            applet.start(&mut host);
        }
        self.app_exit();
    }

    /// Leaves the app if it asked to be left.
    fn app_exit(&mut self) {
        if self.running.as_ref().is_some_and(|running| running.template.exit) {
            let back = self.close_app();
            self.screen = back.unwrap_or(Screen::Calc);
        }
    }

    /// The applet and what it may ask of the device, borrowed apart.
    fn applet(&mut self) -> Option<(&mut dyn Applet, AppHost<'_, B>)> {
        let lang = self.settings.language;
        let Running { applet, template, .. } = self.running.as_mut()?;
        Some((applet.as_mut(), AppHost { backend: &mut self.backend, lang, template }))
    }

    /// A key on a question over the app: the arrows go between *Ja* and *Nein*, OK or EXE
    /// answer, 1 is *Ja* and 2 *Nein*, AC is *Nein*. MODE still leaves the app.
    fn question_key(&mut self, key: Key) {
        let Some(Question::Confirm { yes, .. }) = self.running.as_mut().and_then(|running| running.template.question.as_mut()) else { return };
        let answer = match key {
            Key::Left | Key::Right | Key::Up | Key::Down => {
                *yes = !*yes;
                return;
            }
            Key::Ok | Key::Exe => *yes,
            Key::Digit(1) => true,
            Key::Digit(2) | Key::Ac => false,
            Key::Mode => {
                self.close_app();
                self.screen = Screen::Modes { selected: self.mode.index() };
                return;
            }
            _ => return,
        };
        self.respond(Response::Confirmed(answer));
    }

    /// Closes the question over the app and gives it the answer.
    fn respond(&mut self, response: Response) {
        let Some((applet, mut host)) = self.applet() else { return };
        host.template.question = None;
        applet.answer(response, &mut host);
    }

    /// A key the input template takes; whether it took it.
    fn template_key(&mut self, key: AppKey) -> bool {
        let Some((applet, mut host)) = self.applet() else { return false };
        match applet.input() {
            Input::Keys | Input::Line { .. } | Input::Text { .. } | Input::Cells { .. } => false,
            Input::List { items } => {
                let count = items.len();
                if count == 0 {
                    return false;
                }
                let selected = host.template.list.min(count - 1);
                let chosen = match key {
                    AppKey::Up => {
                        host.template.list = (selected + count - 1) % count;
                        return true;
                    }
                    AppKey::Down => {
                        host.template.list = (selected + 1) % count;
                        return true;
                    }
                    AppKey::Ok | AppKey::Exe => selected,
                    AppKey::Digit(digit @ 1..=9) if usize::from(digit) <= menu::ROWS => {
                        let index = selected / menu::ROWS * menu::ROWS + usize::from(digit) - 1;
                        if index >= count {
                            return true;
                        }
                        index
                    }
                    _ => return false,
                };
                host.template.list = chosen;
                applet.event(Event::Chosen(chosen), &mut host);
                true
            }
            Input::Grid { cols, rows, digits, wrap } => {
                let (cols, rows) = (cols.max(1), rows.max(1));
                let (col, row) = host.template.cursor;
                let (col, row) = (col.min(cols - 1), row.min(rows - 1));
                let step = |at: usize, by: isize, count: usize| match (at.checked_add_signed(by), wrap) {
                    (Some(next), _) if next < count => next,
                    (_, true) if by < 0 => count - 1,
                    (_, true) => 0,
                    (_, false) => at,
                };
                let cursor = match key {
                    AppKey::Left => (step(col, -1, cols), row),
                    AppKey::Right => (step(col, 1, cols), row),
                    AppKey::Up => (col, step(row, -1, rows)),
                    AppKey::Down => (col, step(row, 1, rows)),
                    AppKey::Ok | AppKey::Exe => {
                        host.template.cursor = (col, row);
                        applet.event(Event::Pick(col, row), &mut host);
                        return true;
                    }
                    // 1–9 as the number block lies: 7 at the top left.
                    AppKey::Digit(digit @ 1..=9) if digits && cols == 3 && rows == 3 => {
                        let index = usize::from(digit) - 1;
                        let (col, row) = (index % 3, 2 - index / 3);
                        host.template.cursor = (col, row);
                        applet.event(Event::Pick(col, row), &mut host);
                        return true;
                    }
                    _ => return false,
                };
                host.template.cursor = cursor;
                true
            }
        }
    }

    /// A tap in the app's area: on a cell of the grid it picks the cell, else the applet gets it.
    fn app_tap(&mut self, point: embedded_graphics::prelude::Point) {
        let area = render::app_area(self);
        let Some((applet, mut host)) = self.applet() else { return };
        if let Input::Grid { cols, rows, .. } = applet.input() {
            let grid = Grid::new(area.size.width as i32, area.size.height as i32, cols, rows);
            if let Some((col, row)) = grid.at(point) {
                host.template.cursor = (col, row);
                return applet.event(Event::Pick(col, row), &mut host);
            }
        }
        applet.tap(point, &mut host);
    }

    /// A key on an app: AC leaves to where it was opened from, MODE to the mode menu; the others
    /// go to the applet as far as it gets them.
    fn app_key(&mut self, key: Key) {
        if let Some(menu) = self.running.as_mut().and_then(|running| running.menu.take()) {
            return self.app_menu_key(menu, key);
        }
        if self.running.as_ref().is_some_and(|running| running.template.question.is_some()) {
            return self.question_key(key);
        }
        match key {
            Key::Optn => {
                if let Some(running) = &mut self.running {
                    running.menu = Some(Menu::new(&tree::APP_MENU));
                }
            }
            Key::Ac => {
                let back = self.close_app();
                self.screen = back.unwrap_or(Screen::Calc);
            }
            Key::Mode => {
                self.close_app();
                self.screen = Screen::Modes { selected: self.mode.index() };
            }
            Key::Soft(index) => {
                let Some(bar) = soft::bar(self) else { return };
                match bar.slot(usize::from(index)) {
                    Some(Slot::Page) => self.soft_page = bar.page + 1,
                    Some(Slot::App(index, _)) => {
                        let index = *index;
                        let Some((applet, mut host)) = self.applet() else { return };
                        applet.key(AppKey::Soft(index), &mut host);
                    }
                    _ => {}
                }
            }
            key => {
                let Some(key) = AppKey::from_key(key) else { return };
                if self.template_key(key) {
                    return;
                }
                let Some((applet, mut host)) = self.applet() else { return };
                applet.key(key, &mut host);
            }
        }
    }

    /// A key in the app's menu; AC and OPTN close it, back to the app.
    fn app_menu_key(&mut self, mut menu: Menu, key: Key) {
        let outcome = menu.key(key, &mut self.settings);
        self.app_menu_outcome(menu, outcome);
    }

    /// Does what the app's menu asks for, and keeps it open unless it closed.
    fn app_menu_outcome(&mut self, mut menu: Menu, outcome: Outcome) {
        let label = menu.title();
        let Some(running) = &mut self.running else { return };
        match outcome {
            Outcome::Close => {}
            Outcome::Act(Action::AppInfo) => {
                menu.edit = Some(Edit::AppInfo { label, info: running.info.clone() });
                running.menu = Some(menu);
            }
            Outcome::Act(Action::AppRestart) => {
                if let Some((applet, mut host)) = self.applet() {
                    applet.restart(&mut host);
                }
            }
            Outcome::Act(Action::AppDeleteData) => {
                menu.edit = Some(Edit::Message { label, message: text::NO_DATA });
                running.menu = Some(menu);
            }
            Outcome::Act(Action::AppQuit) => {
                let back = self.close_app();
                self.screen = back.unwrap_or(Screen::Calc);
            }
            _ => running.menu = Some(menu),
        }
    }

    /// Leaves the app, if one runs: it is told, and the screen it was opened from is given back.
    fn close_app(&mut self) -> Option<Screen> {
        let (applet, mut host) = self.applet()?;
        applet.leave(&mut host);
        let running = self.running.take()?;
        Some(*running.back)
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
                self.app_exit();
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
                // Across a graph: it moves with the finger, and its curve is sampled anew.
                if let Some(answer) = self.answer.as_mut().filter(|answer| answer.graph().is_some()) {
                    let window = answer.graph().expect("a graph page").window;
                    answer.move_to(render::dragged(window, moved));
                    self.resample();
                    return;
                }
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
            Target::Back if matches!(self.screen, Screen::Modes { .. } | Screen::Calc | Screen::App) => self.key(Key::Ac),
            Target::App(point) => self.app_tap(point),
            Target::Answer(yes) => self.respond(Response::Confirmed(yes)),
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
            Target::Soft(index) => self.key(Key::Soft(index as u8)),
            Target::Page(page) => self.show_page(page),
            // A line of an app's list: picked as its digit picks it.
            Target::Line(offset) if self.running.as_ref().is_some_and(|running| running.menu.is_none()) => {
                let first = self.running.as_ref().map_or(0, |running| running.template.list / menu::ROWS * menu::ROWS);
                if let Some(running) = &mut self.running {
                    running.template.list = first + offset;
                }
                self.template_key(AppKey::Ok);
            }
            Target::Line(offset) if matches!(self.screen, Screen::App) => {
                let Some(mut menu) = self.running.as_mut().and_then(|running| running.menu.take()) else { return };
                let outcome = menu.tap(offset, &mut self.settings);
                self.app_menu_outcome(menu, outcome);
            }
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
                if !self.themes.is_empty() {
                    self.themes.clear();
                    colors::set(Palette::from_theme(&self.theme));
                }
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
            Outcome::Act(Action::DefineMatrix) => {
                let purpose = TextFor::Matrix;
                menu.edit = Some(Edit::Text { label: text::MATRIX_NAME, purpose, typed: String::new(), error: false, plain: true });
            }
            Outcome::Act(Action::EditMatrix) => return self.vars(),
            // The form over the data, on its first field.
            Outcome::Act(Action::Estimates) => {
                self.answer = None;
                self.sheets.estimating = true;
                self.sheets.estimates.select(0, 0);
                return;
            }
            Outcome::Define => {
                let Some(Edit::Text { typed, .. }) = &menu.edit else { return };
                let (name, settings) = (typed.clone(), self.settings);
                self.reset();
                self.answer = None;
                self.sheets.define(name, None, &settings);
                return;
            }
            Outcome::Act(action @ (Action::TangentForm | Action::AreaForm | Action::PointsForm | Action::Calc | Action::Solve)) => {
                self.act(action);
                return;
            }
            Outcome::Sections(label) => match &self.sketch {
                Some((_, sketch)) => {
                    let lang = self.settings.language;
                    let titles = std::iter::once(text::GRAPH.get(lang).to_string()).chain(sketch.pages.iter().map(|page| page.title.clone()));
                    let selected = self.answer.as_ref().filter(|answer| answer.pages() > 1).map_or(0, |answer| answer.page);
                    menu.edit = Some(Edit::Sections { label, titles: titles.collect(), selected });
                }
                None => menu.edit = Some(Edit::Message { label, message: text::SKETCH_FIRST }),
            },
            Outcome::Section(page) => {
                let Some((_, sketch)) = self.sketch.clone() else { return };
                self.task = None;
                let mut answer = Answer::new(Ok(Reply::Analysis(sketch)));
                answer.turn_to(page);
                self.answer = Some(answer);
                return;
            }
            Outcome::Act(action) => self.act(action),
            Outcome::Sessions(label) => self.open_sessions(&mut menu, label),
            Outcome::OpenApp(index) => {
                let Some(Edit::Apps { apps, view, .. }) = &mut menu.edit else { return };
                let Some(info) = apps.get(index).cloned() else { return };
                let lang = self.settings.language;
                if !info.usable {
                    *view = menu::AppsView::Note(text::NEWER_API.get(lang).to_string());
                } else {
                    match self.backend.open_app(&info.id) {
                        Ok(applet) => {
                            // AC comes back to the list.
                            self.screen = Screen::Menu(menu);
                            return self.open_applet(applet, info);
                        }
                        Err(error) => *view = menu::AppsView::Note(error),
                    }
                }
            }
            Outcome::DeleteAppData(_) => {
                let Some(Edit::Apps { view, .. }) = &mut menu.edit else { return };
                *view = menu::AppsView::Note(text::NO_DATA.get(self.settings.language).to_string());
            }
            Outcome::OpenSession(index) => {
                let Some(Edit::Sessions { sessions, .. }) = &menu.edit else { return };
                let Some(id) = sessions.get(index).map(|session| session.id.clone()) else { return };
                return self.session_command(menu, SessionCommand::Open(id));
            }
            Outcome::NameOf(label) => {
                let typed = self.session.as_ref().map(|session| session.name.clone()).unwrap_or_default();
                menu.edit = Some(Edit::Text { label, purpose: TextFor::Name, typed, error: false, plain: true });
            }
            Outcome::Variable(action) => {
                let Some(Edit::Variable { variables, index, .. }) = &menu.edit else { return };
                let Some(variable) = variables.get(*index).cloned() else { return };
                match action {
                    0 => return self.insert(&variable.symbol),
                    1 => return self.change(variable),
                    _ => match self.variables(VariableCommand::Delete(variable.key)) {
                        // The list again, on the variable after the one deleted.
                        Edit::Variables { label, variables, .. } => {
                            let selected = (*index).min(variables.len() - 1);
                            menu.edit = Some(Edit::Variables { label, variables, selected });
                        }
                        edit => menu.edit = Some(edit),
                    },
                }
            }
            Outcome::Store => {
                let Some(Edit::Text { label, purpose: TextFor::Variable(latex), typed, .. }) = &menu.edit else { return };
                let (label, key, latex) = (*label, typed.clone(), latex.clone());
                let mode = if self.mode.engine() { self.mode } else { Mode::Physics };
                let settings = self.settings;
                menu.edit = Some(match self.backend.variables(VariableCommand::Store { mode, key: key.clone(), latex }, &settings) {
                    Ok(_) => Edit::Message { label, message: text::STORED },
                    Err(note) => Edit::Note { label, note },
                });
            }
            Outcome::Rename => {
                let Some(Edit::Text { typed, .. }) = &menu.edit else { return };
                let name = typed.trim().to_string();
                return self.session_command(menu, SessionCommand::Rename(name));
            }
            Outcome::Pictures(label) => self.open_pictures(&mut menu, label),
            Outcome::Themes(label) => self.open_themes(&mut menu, label),
            Outcome::Preview(index) if matches!(menu.edit, Some(Edit::Themes { .. })) => self.preview_theme(index),
            Outcome::Preview(index) => {
                let picture = self.picture(&menu, index);
                self.preview = Some((index, picture));
            }
            Outcome::Keep(index) if matches!(menu.edit, Some(Edit::Themes { .. })) => self.keep_theme(&mut menu, index),
            Outcome::Keep(index) => self.keep_picture(&mut menu, index),
            Outcome::Restore => {
                self.preview = None;
                self.themes.clear();
                colors::set(Palette::from_theme(&self.theme));
            }
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
            Outcome::Catalogue(label, catalogue) => menu.edit = Some(self.catalogue(label, catalogue, false)),
            Outcome::Search => {
                let Some(Edit::Catalogue { query, hits, selected, .. }) = &mut menu.edit else { return };
                let (lang, mode) = (self.settings.language, self.mode);
                if !self.search.as_ref().is_some_and(|index| index.lang == lang && index.mode == mode) {
                    self.search = Some(search::Index::new(self.backend.search_index(lang), mode, lang));
                }
                *hits = match query.is_empty() {
                    true => Vec::new(),
                    false => self.search.as_ref().map_or_else(Vec::new, |index| index.search(query)),
                };
                *selected = 0;
            }
            Outcome::Item if matches!(&menu.edit, Some(Edit::Catalogue { query, .. }) if !query.is_empty()) => {
                let Some(Edit::Catalogue { hits, selected, .. }) = &menu.edit else { return };
                let Some(hit) = hits.get(*selected) else { return };
                match hit.target.clone() {
                    search::Target::Insert(latex) => return self.insert(&latex),
                    search::Target::Open(root, path) => return self.open_place(root, &path),
                }
            }
            Outcome::Item => {
                let Some(Edit::Catalogue { catalogue, groups, group: Some(group), selected, .. }) = &menu.edit else { return };
                let Some(item) = groups.get(*group).and_then(|group| group.items.get(*selected)) else { return };
                // A unit of CONV is what the value is converted to.
                let latex = match catalogue {
                    Catalogue::Constants => item.latex.clone(),
                    Catalogue::Units => format!(r"\blacktriangleright {}", item.latex),
                    Catalogue::Functions => item.latex.clone(),
                };
                return self.insert(&latex);
            }
        }
        self.screen = Screen::Menu(menu);
    }

    /// CONST or CONV, opened by its key: the catalogue's categories, which closes the menu when
    /// left. The constants are Physik's only; a value converts in every mode.
    fn open_catalogue(&mut self, catalogue: Catalogue) {
        let page = match catalogue {
            Catalogue::Constants => &tree::CONSTANTS,
            Catalogue::Units => &tree::CONVERSIONS,
            Catalogue::Functions => tree::optn(self.mode),
        };
        let label = match catalogue {
            Catalogue::Functions => text::CATALOG,
            _ => page.title,
        };
        let mut menu = Menu::new(page);
        menu.edit = Some(match catalogue {
            Catalogue::Constants if self.mode != Mode::Physics => Edit::Message { label, message: text::PHYSICS_ONLY },
            _ => self.catalogue(label, catalogue, true),
        });
        self.screen = Screen::Menu(menu);
    }

    /// The list of a catalogue the backend has, or a message if it has none; `close` for one
    /// opened by its key, which closes the menu when it is left.
    fn catalogue(&mut self, label: Text, catalogue: Catalogue, close: bool) -> Edit {
        if catalogue == Catalogue::Functions {
            let items = tree::functions(self.mode, self.settings.language);
            let groups = vec![Group { name: label.get(self.settings.language).to_string(), items }];
            return Edit::Catalogue { label, catalogue, groups, group: Some(0), selected: 0, close, query: String::new(), hits: Vec::new() };
        }
        let groups = self.backend.catalogue(catalogue, self.settings.language);
        match groups.is_empty() {
            true => Edit::Message { label, message: text::NOT_AVAILABLE },
            false => Edit::Catalogue { label, catalogue, groups, group: None, selected: 0, close, query: String::new(), hits: Vec::new() },
        }
    }

    /// A menu hit of CATALOG's search: its menu, opened down to the entry, which is opened if it
    /// opens a page or a submenu; an action waits for OK. Leaving goes up the menu from there.
    fn open_place(&mut self, root: tree::Root, path: &[usize]) {
        let mut menu = Menu::new(root.page(self.mode));
        let Some((&last, pages)) = path.split_last() else { return };
        for &index in pages {
            let level = menu.stack.last_mut().expect("a menu has a page");
            level.selected = index;
            let Some(Kind::Page(page)) = level.page.entries.get(index).map(|entry| &entry.kind) else { return };
            menu.stack.push(Level { page, selected: 0 });
        }
        let level = menu.stack.last_mut().expect("a menu has a page");
        level.selected = last;
        let Some(entry) = level.page.entries.get(last) else { return };
        if matches!(entry.kind, Kind::Do(_) | Kind::Insert(_) | Kind::Info(_) | Kind::Pending) {
            self.screen = Screen::Menu(menu);
            return;
        }
        let outcome = menu.choose(entry, &self.settings);
        self.outcome(menu, outcome);
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
            // A task's form over the input, on its first field.
            Action::TangentForm | Action::AreaForm | Action::PointsForm => {
                let kind = match action {
                    Action::TangentForm => sheet::Kind::Tangent,
                    Action::AreaForm => sheet::Kind::Area,
                    _ => sheet::Kind::Points,
                };
                self.answer = None;
                self.task = Some(kind);
                self.sheets.get_mut(kind).select(0, 0);
            }
            Action::Calc => self.formula(false),
            Action::Solve => self.formula(true),
            // Done by `outcome`, which knows the menu to come back to.
            Action::DefineMatrix | Action::EditMatrix | Action::Estimates => {}
            Action::Calibrate | Action::NewSession | Action::DuplicateSession | Action::DeleteSession => {}
            // Done by `app_menu_key`, inside an app.
            Action::AppInfo | Action::AppRestart | Action::AppDeleteData | Action::AppQuit => {}
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

    /// Sets the wallpaper `name` at the start, dimmed towards the theme's background.
    pub fn set_wallpaper(&mut self, name: String, picture: Wallpaper) {
        let picture = match colors::background() == Palette::DARK.background {
            true => picture,
            false => picture.redimmed().unwrap_or(picture),
        };
        self.wallpaper = Some(picture);
        self.wallpaper_name = Some(name);
    }

    /// Opens the list of themes, the built-in ones and those on the SD card, on the one set now.
    fn open_themes(&mut self, menu: &mut Menu, label: Text) {
        self.themes = quantus_themes::list(self.backend.themes());
        let code = self.settings.language.code();
        let names = self
            .themes
            .iter()
            .map(|entry| match &entry.theme {
                Ok(theme) => theme.name(code).to_string(),
                Err(_) => format!("{} (!)", entry.id),
            })
            .collect();
        let selected = self.theme_index().unwrap_or(0);
        menu.edit = Some(Edit::Themes { label, names, selected });
    }

    /// Shows the theme at `index` of the open list, or the one set if that cannot be read.
    fn preview_theme(&self, index: usize) {
        let theme = self.themes.get(index).and_then(|entry| entry.theme.as_ref().ok()).unwrap_or(&self.theme);
        colors::set(Palette::from_theme(theme));
    }

    /// Sets the theme at `index` of the open list and keeps it, or says why it cannot.
    fn keep_theme(&mut self, menu: &mut Menu, index: usize) {
        let Some(Edit::Themes { label, .. }) = menu.edit.take() else { return };
        let entries = std::mem::take(&mut self.themes);
        let Some(entry) = entries.into_iter().nth(index) else { return };
        match entry.theme {
            Ok(theme) => {
                colors::set(Palette::from_theme(&theme));
                self.backend.keep_theme(&theme.id);
                self.theme = theme;
                self.wallpaper = self.wallpaper.as_ref().and_then(Wallpaper::redimmed);
            }
            Err(error) => {
                colors::set(Palette::from_theme(&self.theme));
                menu.edit = Some(Edit::Note { label, note: format!("themes/{}.toml: {error}", entry.id) });
            }
        }
    }

    /// Puts LaTeX from a menu into the input, as if typed: a shown result is left first.
    fn insert(&mut self, latex: &str) {
        // The input is kept as it was sent until the answer comes.
        if self.pending.is_some() {
            return;
        }
        // d/dx goes after the power it ends; after a result, after the input that gave it.
        if latex == layer::DERIVATIVE && self.sheet_kind().is_none() {
            self.shown = None;
            return self.editor.derivative();
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
        // Into a pair of brackets or bars, at the first argument: `\sinh(|)`, `\max(|;)`, `|x|`;
        // into n of nCr.
        let back = match row.0.last() {
            Some(Node::Binom(..)) => 2,
            _ if row.0.len() < 2 => 0,
            Some(Node::Char(')')) => row.0.iter().rposition(|node| *node == Node::Char('(')).map_or(1, |open| row.0.len() - 1 - open),
            Some(Node::Char('|')) => 1,
            _ => 0,
        };
        (0..back).for_each(|_| {
            editor.left();
        });
    }

    /// What soft key `index` holds, done.
    fn soft_key(&mut self, index: usize) {
        let Some(bar) = soft::bar(self) else { return };
        let Some(slot) = bar.slot(index).cloned() else { return };
        match slot {
            Slot::Empty | Slot::App(..) => {}
            Slot::Page => match &mut self.symbols {
                Some(page) => *page = bar.page + 1,
                None => self.soft_page = bar.page + 1,
            },
            Slot::Symbol(_, latex) => {
                self.symbols = None;
                self.insert(latex);
            }
            Slot::Unit(item) => {
                self.typed_units.push(item.name.clone());
                self.insert(&item.latex);
            }
            Slot::Convert(item) => self.convert(&item),
            Slot::Key(_, key) => self.calc_key(key, false),
            Slot::Base(_, value) => self.settings.set(Setting::NumberBase, value),
            // As OPTN does it, over a page of its own: a message or a list closes back to here.
            Slot::Command(soft) => {
                let mut menu = Menu::new(&tree::SOFT_PAGE);
                let outcome = menu.choose(&soft.entry, &self.settings);
                match outcome {
                    Outcome::Stay if menu.edit.is_none() => {}
                    outcome => self.outcome(menu, outcome),
                }
            }
        }
    }

    /// The shown result converted to the unit `item` and worked out: `Ans ▶ unit`.
    fn convert(&mut self, item: &Item) {
        let Some(Entry { result: Ok(result), .. }) = self.shown_entry() else { return };
        let result = result.clone();
        self.reset();
        self.editor.paste(&result);
        self.typed_units.push(item.name.clone());
        self.insert(&format!(r"\blacktriangleright {}", item.latex));
        self.evaluate();
    }

    /// A prefix and a unit typed on the UNIT layer are remembered as one unit once the layer lets
    /// go of them.
    fn track_unit(&mut self, layer: Layer, resolved: Resolved) {
        match resolved {
            Resolved::Insert(latex) if layer == Layer::Unit => self.unit_run.push_str(&latex.replace(r"\mu ", "µ")),
            _ => self.end_unit(),
        }
    }

    fn end_unit(&mut self) {
        if !self.unit_run.is_empty() {
            let run = std::mem::take(&mut self.unit_run);
            self.typed_units.push(run);
        }
    }

    /// The units typed since the last EXE go to the front of the recent ones.
    fn remember_units(&mut self) {
        self.end_unit();
        for unit in std::mem::take(&mut self.typed_units) {
            self.recent_units.retain(|recent| *recent != unit);
            self.recent_units.insert(0, unit);
        }
        self.recent_units.truncate(RECENT_UNITS);
    }

    /// Asks the backend for Physik's units again when what they go by has changed: the input
    /// up to its last operator, or the shown result. Whether they changed.
    fn refresh_units(&mut self) -> bool {
        let wanted = self.mode == Mode::Physics
            && matches!(self.screen, Screen::Calc)
            && self.answer.is_none()
            && self.pending.is_none()
            && self.sheet_kind().is_none();
        if !wanted {
            return false;
        }
        let result = self.shown.map(|_| match self.shown_entry() {
            Some(Entry { result: Ok(result), .. }) => Some(result.to_latex()),
            _ => None,
        });
        let context = match &result {
            Some(result) => format!("={}", result.as_deref().unwrap_or("")),
            None => soft::context(self.editor.before_cursor(), self.editor.path().len()),
        };
        if self.units_for.as_ref() == Some(&context) {
            return false;
        }
        self.units = match result {
            Some(None) => Units::default(),
            Some(Some(result)) => self.backend.unit_suggestions("", &self.recent_units, Some(&result)),
            None => {
                let before = Row(self.editor.before_cursor().to_vec()).to_latex();
                self.backend.unit_suggestions(&before, &self.recent_units, None)
            }
        };
        self.units_for = Some(context);
        true
    }

    fn calc_key(&mut self, key: Key, shift: bool) {
        match key {
            Key::Mode if shift => return self.screen = Screen::Menu(Menu::new(&tree::SETUP)),
            Key::Mode => return self.screen = Screen::Modes { selected: self.mode.index() },
            Key::Optn => return self.screen = Screen::Menu(Menu::new(tree::optn(self.mode))),
            Key::Const => return self.open_catalogue(Catalogue::Constants),
            Key::Conv => return self.open_catalogue(Catalogue::Units),
            Key::Catalog => return self.open_catalogue(Catalogue::Functions),
            Key::Eng | Key::EngBack => return self.engineering(key == Key::Eng),
            Key::Calc => return self.formula(shift),
            Key::Vars if shift => return self.sto(),
            Key::Vars => return self.vars(),
            Key::Symb => return self.symbols = if self.symbols.is_some() { None } else { Some(0) },
            Key::Soft(index) => return self.soft_key(usize::from(index)),
            _ => {}
        }
        // While the backend calculates, AC drops its answer and the other keys wait; the graph
        // can be moved on while its curve is sampled.
        if self.pending.is_some() && !matches!(self.pending, Some(Pending::Curve)) {
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
            Key::Copy => {
                if let Some(row) = editor.copied() {
                    self.clipboard = row;
                }
            }
            Key::Paste => editor.paste(&self.clipboard),
            Key::Approx => self.evaluate_as(true),
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
        if self.graph_key(key) {
            return;
        }
        if let Some(answer) = &mut self.answer {
            let pages = answer.pages() > 1;
            match key {
                Key::Up => answer.scroll_by(-1),
                Key::Down => answer.scroll_by(1),
                // A task's answer has its graph on a second page.
                Key::Left if pages => {
                    answer.turn_to(answer.page.saturating_sub(1));
                }
                Key::Right if pages => {
                    answer.turn_to(answer.page + 1);
                }
                Key::Left => answer.shift = (answer.shift - SHIFT_STEP).max(0),
                Key::Right => answer.shift += SHIFT_STEP,
                Key::Ac | Key::Del | Key::Ok | Key::Exe => self.answer = None,
                _ => {}
            }
            return;
        }
        // AC on a form over the input, not in a cell: back to the input.
        let over = kind.over_input() || matches!(kind, sheet::Kind::Matrix | sheet::Kind::Estimates);
        if over && key == Key::Ac && self.sheets.get(kind).editing.is_none() {
            match kind {
                sheet::Kind::Formula => self.sheets.asked = None,
                sheet::Kind::Matrix => self.sheets.defining = None,
                sheet::Kind::Estimates => self.sheets.estimating = false,
                _ => self.task = None,
            }
            return;
        }
        if self.sheets.get_mut(kind).key(key, shift) == sheet::Outcome::Calculate {
            match (kind, self.sheets.request(kind, &self.editor.to_latex())) {
                // The matrix is defined as if typed: `A = (…)`, into the history.
                (sheet::Kind::Matrix, _) => {
                    let definition = self.sheets.definition();
                    self.sheets.defining = None;
                    self.reset();
                    self.editor.set(latex::parse(&definition));
                    self.evaluate();
                }
                (sheet::Kind::Formula, Some(request)) => self.ask(Pending::Formula { input: self.editor.row.clone() }, &request),
                (_, Some(request)) => self.solve(&request),
                (_, None) => {}
            }
        }
    }

    /// Asks the backend to work `request` out; the answer comes now or when `tick` gets it.
    fn solve(&mut self, request: &Request) {
        // A curve still being sampled is not wanted any more.
        if matches!(self.pending, Some(Pending::Curve)) {
            self.cancel();
        }
        let settings = self.settings;
        self.asked = request.function().map(String::from);
        self.pending = Some(Pending::Sheet);
        match self.backend.start_solve(&settings, request) {
            Some(reply) => self.finish_sheet(reply),
            None => self.busy = true,
        }
    }

    /// VARS: the session's variables, each with its value; choosing one offers to insert it,
    /// change its value or delete it.
    fn vars(&mut self) {
        let mut menu = Menu::new(&tree::VARIABLES);
        menu.edit = Some(self.variables(VariableCommand::List));
        self.screen = Screen::Menu(menu);
    }

    /// Does `command` to the variables and lists them afterwards; a message if there are none.
    fn variables(&mut self, command: VariableCommand) -> Edit {
        let label = text::VARS;
        let settings = self.settings;
        match self.backend.variables(command, &settings) {
            Ok(variables) if variables.is_empty() => Edit::Message { label, message: text::NO_VARIABLES },
            Ok(variables) => Edit::Variables { label, variables, selected: 0 },
            Err(note) => Edit::Note { label, note },
        }
    }

    /// Changes the value of `variable`: CALC's form with its one field, holding the value now;
    /// a matrix's cells in the grid of Matrizen.
    fn change(&mut self, variable: crate::backend::Stored) {
        if self.pending.is_some() {
            return;
        }
        self.reset();
        self.answer = None;
        if let [Node::Matrix(rows)] = &latex::parse(&variable.value).0[..] {
            self.mode = Mode::Matrix;
            self.settings.matrix_rows = rows.len().clamp(1, 10) as u8;
            self.settings.matrix_columns = rows.iter().map(Vec::len).max().unwrap_or(1).clamp(1, 10) as u8;
            let settings = self.settings;
            self.sheets.define(variable.key, Some(rows), &settings);
            return;
        }
        self.editor.set(latex::parse(&variable.symbol));
        let mode = if self.mode.engine() { self.mode } else { Mode::Physics };
        let asked = crate::backend::Variable { key: variable.key, symbol: variable.symbol };
        self.sheets.ask(sheet::Formula { mode, variables: vec![asked], solve: false });
        self.sheets.formula.set(0, 0, latex::parse(&variable.value));
    }

    /// STO: asks for the name to keep the shown result under, the last one if none is shown.
    fn sto(&mut self) {
        let label = text::STO;
        let mut menu = Menu::new(&tree::VARIABLES);
        let index = self.shown.unwrap_or(0);
        menu.edit = Some(match self.history.iter().rev().nth(index) {
            Some(Entry { result: Ok(result), .. }) => {
                // The last two results exactly as the session has them, an older one as shown.
                let latex = match (index, result.to_latex()) {
                    (0, _) => r"\mathrm{Ans}".to_string(),
                    (1, _) => r"\mathrm{PreAns}".to_string(),
                    (_, latex) => latex.trim_start_matches(r"\approx").trim().to_string(),
                };
                Edit::Text { label, purpose: TextFor::Variable(latex), typed: String::new(), error: false, plain: true }
            }
            _ => Edit::Message { label, message: text::NO_RESULT },
        });
        self.screen = Screen::Menu(menu);
    }

    /// CALC, or SOLVE with `solve`: asks the engine for the letters of the input that have no
    /// value, for a form with a field for each. CALC works an input without any out as it is.
    fn formula(&mut self, solve: bool) {
        if self.pending.is_some() || self.editor.is_empty() || !self.mode.engine() || self.sheet_kind().is_some() {
            return;
        }
        let request = Request::Variables { mode: self.mode, latex: self.editor.to_latex() };
        self.ask(Pending::Variables { mode: self.mode, solve }, &request);
    }

    /// Sends a request of the calculation screen (CALC) to the backend, its answer to come in
    /// `pending`.
    fn ask(&mut self, pending: Pending, request: &Request) {
        let settings = self.settings;
        self.pending = Some(pending);
        match self.backend.start_solve(&settings, request) {
            Some(reply) => self.finish_formula(reply),
            None => self.busy = true,
        }
    }

    /// The letters asked for open CALC's form, none work the input out; CALC's answer becomes an
    /// entry of the history.
    fn finish_formula(&mut self, reply: Result<Reply, String>) {
        self.busy = false;
        let lang = self.settings.language;
        let not_available = || text::NOT_AVAILABLE.get(lang).to_string();
        match self.pending.take() {
            Some(Pending::Variables { mode, solve }) => match reply {
                Ok(Reply::Variables(variables)) if variables.is_empty() && solve => {
                    self.answer_entry(Err(text::NO_UNKNOWN.get(lang).to_string()));
                }
                Ok(Reply::Variables(variables)) if variables.is_empty() => {
                    self.shown = None;
                    self.evaluate();
                }
                Ok(Reply::Variables(variables)) => {
                    self.shown = None;
                    self.sheets.ask(sheet::Formula { mode, variables, solve });
                }
                Ok(_) => self.answer_entry(Err(not_available())),
                Err(message) => self.answer_entry(Err(message)),
            },
            Some(Pending::Formula { input }) => {
                self.sheets.asked = None;
                self.editor.set(input.clone());
                let result = match reply {
                    Ok(Reply::Result(latex)) => Ok(latex),
                    Ok(_) => Err(not_available()),
                    Err(message) => Err(message),
                };
                self.pending = Some(Pending::New { input, decimal: self.settings.decimal });
                self.finish(result);
            }
            _ => {}
        }
    }

    /// Shows `result` for the input as a new entry of the history.
    fn answer_entry(&mut self, result: Result<String, String>) {
        self.pending = Some(Pending::New { input: self.editor.row.clone(), decimal: self.settings.decimal });
        self.finish(result);
    }

    /// A key on a curve sketch: ←/→ turn its pages, ↑/↓ scroll one, AC, DEL, OK and EXE go
    /// back to the input. Another key leaves it too and goes on to the input; returns whether
    /// it does.
    fn sketch_key(&mut self, key: Key) -> bool {
        if self.graph_key(key) {
            return false;
        }
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

    /// A key on a graph page: OK starts moving the graph; then the arrows move it by a quarter
    /// of the window, + and − zoom in and out around its middle, 0 goes back to where it was,
    /// and OK or AC stop. Returns whether the key was for the graph.
    fn graph_key(&mut self, key: Key) -> bool {
        let Some(answer) = self.answer.as_mut().filter(|answer| answer.graph().is_some()) else { return false };
        if !answer.moving {
            if key == Key::Ok {
                answer.moving = true;
                return true;
            }
            return false;
        }
        let Some(window) = answer.graph().map(|graph| graph.window) else { return false };
        let (width, height) = (window.x_max - window.x_min, window.y_max - window.y_min);
        let shift = |dx: f32, dy: f32| Window {
            x_min: window.x_min + dx * width,
            x_max: window.x_max + dx * width,
            y_min: window.y_min + dy * height,
            y_max: window.y_max + dy * height,
        };
        let zoom = |by: f32| {
            let (cx, cy) = ((window.x_min + window.x_max) / 2.0, (window.y_min + window.y_max) / 2.0);
            let (w, h) = ((width * by).clamp(1e-3, 1e6) / 2.0, (height * by).clamp(1e-3, 1e6) / 2.0);
            Window { x_min: cx - w, x_max: cx + w, y_min: cy - h, y_max: cy + h }
        };
        let moved = match key {
            Key::Left => shift(-0.25, 0.0),
            Key::Right => shift(0.25, 0.0),
            Key::Up => shift(0.0, 0.25),
            Key::Down => shift(0.0, -0.25),
            Key::Plus => zoom(0.5),
            Key::Minus => zoom(2.0),
            Key::Digit(0) => match answer.home {
                Some(home) => home,
                None => return true,
            },
            Key::Ok | Key::Ac | Key::Exe => {
                answer.moving = false;
                return true;
            }
            // Nothing else while moving.
            _ => return true,
        };
        answer.move_to(moved);
        self.resample();
        true
    }

    /// Asks for the curve in the window the graph shows now; one at a time, the last window
    /// asked again when the one before comes back.
    fn resample(&mut self) {
        match self.pending {
            Some(Pending::Curve) => self.resample_again = true,
            Some(_) => {}
            None => {
                let Some(answer) = &self.answer else { return };
                let (Some(f), Some(graph)) = (answer.function(), answer.graph()) else { return };
                let request = Request::Sample { f, window: graph.window };
                let settings = self.settings;
                self.pending = Some(Pending::Curve);
                match self.backend.start_solve(&settings, &request) {
                    Some(reply) => self.finish_curve(reply),
                    None => self.busy = true,
                }
            }
        }
    }

    /// Puts the curve sampled into the graph on the screen, and asks again if it moved since.
    fn finish_curve(&mut self, reply: Result<Reply, String>) {
        self.busy = false;
        if !matches!(self.pending.take(), Some(Pending::Curve)) {
            return;
        }
        if let (Ok(Reply::Curve(curve)), Some(graph)) = (reply, self.answer.as_mut().and_then(Answer::graph_mut)) {
            graph.curve = curve;
        }
        if std::mem::take(&mut self.resample_again) {
            self.resample();
        }
    }

    /// Shows the answer to the sheet.
    fn finish_sheet(&mut self, reply: Result<Reply, String>) {
        self.busy = false;
        if let Some(Pending::Sheet) = self.pending.take() {
            let mut reply = reply;
            let asked = self.asked.take().unwrap_or_default();
            match &mut reply {
                Ok(Reply::Analysis(sketch)) => self.sketch = Some((asked, sketch.clone())),
                // A task on the function sketched last is drawn on its graph.
                Ok(Reply::Task(task)) => {
                    if let Some((_, sketch)) = self.sketch.as_ref().filter(|(function, _)| *function == task.function) {
                        task.graph = Some(Graph { overlay: task.overlay.clone(), ..sketch.graph.clone() });
                    }
                }
                _ => {}
            }
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
            // SETUP, and the mode stays as it is.
            Mode::Setup => self.screen = Screen::Menu(Menu::new(&tree::SETUP)),
            Mode::Apps => {
                let mut menu = Menu::new(&tree::APPS);
                let label = tree::APPS.title;
                // No apps in Prüfungsmodus (apps.md decision 6).
                menu.edit = Some(match self.settings.exam {
                    true => Edit::Message { label, message: text::APPS_EXAM },
                    false => menu::apps_edit(label, self.backend.apps(self.settings.language)),
                });
                self.screen = Screen::Menu(menu);
            }
            mode => {
                if mode != self.mode {
                    self.answer = None;
                    self.task = None;
                }
                self.mode = mode;
                self.screen = Screen::Calc;
                self.restore_function();
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

    /// ENG (`up`) or ←ENG on the shown result: written with the exponent a multiple of three,
    /// and by three more on each press. A result with no decimal value stays as it is.
    fn engineering(&mut self, up: bool) {
        let Some(shown) = self.shown else { return };
        if self.pending.is_some() || matches!(self.mode, Mode::BaseN | Mode::Verify) {
            return;
        }
        let index = self.history.len() - 1 - shown;
        let entry = &self.history[index];
        let steps = match (entry.eng, up) {
            (None, true) => 0,
            (None, false) => -1,
            (Some(steps), true) => steps + 1,
            (Some(steps), false) => steps - 1,
        };
        let latex = entry.input.to_latex();
        if let Some(Ok(result)) = self.backend.engineering(self.mode, &self.settings, &latex, steps) {
            let entry = &mut self.history[index];
            entry.result = Ok(latex::parse(&result));
            entry.eng = Some(steps);
        }
    }

    fn evaluate(&mut self) {
        self.evaluate_as(self.settings.decimal);
    }

    /// Works the input out, exact or as a decimal whatever the setting is.
    fn evaluate_as(&mut self, decimal: bool) {
        if self.editor.is_empty() {
            return;
        }
        if self.mode == Mode::Physics {
            self.remember_units();
        }
        // Kurvendiskussion sketches the function, a page per section.
        if self.mode == Mode::Analysis {
            let request = Request::Analysis { f: self.editor.to_latex() };
            return self.solve(&request);
        }
        let pending = Pending::New { input: self.editor.row.clone(), decimal };
        if self.mode.engine() {
            let settings = Settings { decimal, ..self.settings };
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
            _ => self.backend.recalculate(self.mode, settings, latex),
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
                self.history.push(Entry { input: input.clone(), result, decimal, eng: None });
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
                    entry.eng = None;
                }
            }
            Some(Pending::Sheet | Pending::Curve | Pending::Variables { .. } | Pending::Formula { .. }) | None => {}
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
