//! What computes the results: `quantus-core` (the `quantus-engine` crate), or a stub.

use crate::app::{Mode, Settings};
use crate::clock;
use crate::settings::{DistributionKind, StatisticsType};
use crate::text;
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

    /// The result of `latex` in engineering notation (ENG): the exponent a multiple of three,
    /// moved by three per step (`steps` 0 is the first press of ENG, −1 the first of ←ENG), or
    /// an error if it has no decimal value. `None` for a backend without it.
    fn engineering(&mut self, _mode: Mode, _settings: &Settings, _latex: &str, _steps: i32) -> Option<Result<String, String>> {
        None
    }

    /// Forgets the running calculation: its answer never comes from [`poll`](Backend::poll) or
    /// [`poll_solve`](Backend::poll_solve).
    fn drop_pending(&mut self) {}

    /// What a mode with a sheet (LGS, Wertetabelle, Statistik, Verteilungen) asks for, worked
    /// out, or an error message. The backend keeps the request in the open session, so the
    /// sheet is filled again when the session is opened.
    fn solve(&mut self, settings: &Settings, _request: &Request) -> Result<Reply, String> {
        Err(text::NOT_AVAILABLE.get(settings.language).to_string())
    }

    /// [`solve`](Backend::solve) as [`start`](Backend::start) does it: the answer now, or
    /// `None` and later from [`poll_solve`](Backend::poll_solve).
    fn start_solve(&mut self, settings: &Settings, request: &Request) -> Option<Result<Reply, String>> {
        Some(self.solve(settings, request))
    }

    /// The answer to the last [`start_solve`](Backend::start_solve) that returned `None`.
    fn poll_solve(&mut self) -> Option<Result<Reply, String>> {
        None
    }

    /// The physical constants (CONST) or the units a value converts to (CONV), by category,
    /// named in `lang`.
    fn catalogue(&mut self, _catalogue: Catalogue, _lang: crate::text::Lang) -> Vec<Group> {
        Vec::new()
    }

    /// What CATALOG's search finds besides the menus: the constants, units, functions and
    /// prefixes, named in `lang`. Without an engine, nothing.
    fn search_index(&mut self, _lang: crate::text::Lang) -> Vec<IndexItem> {
        Vec::new()
    }

    /// The units the soft keys offer in Physik (firmware/docs/unit-keys.md §2): for the input
    /// `before` the cursor (its LaTeX), the labels of the units used last, newest first; or, with
    /// `result`, the units a shown result converts to. Without an engine, the common units.
    fn unit_suggestions(&mut self, _before: &str, _recent: &[String], result: Option<&str>) -> Units {
        match result {
            Some(_) => Units::default(),
            None => Units::common(),
        }
    }

    /// Does `command` to the variables of the open session and returns them all afterwards, or
    /// why it failed. A backend without sessions has none.
    fn variables(&mut self, _command: VariableCommand, settings: &Settings) -> Result<Vec<Stored>, String> {
        Err(text::NOT_AVAILABLE.get(settings.language).to_string())
    }

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

    /// The colour themes in `/themes/` on the SD card: each file's name without `.toml` and
    /// its text, in any order.
    fn themes(&mut self) -> Vec<(String, String)> {
        Vec::new()
    }

    /// The id of the theme kept from the last run, if any.
    fn load_theme(&mut self) -> Option<String> {
        None
    }

    /// Keeps the theme `id` for the next run.
    fn keep_theme(&mut self, _id: &str) {}

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

    /// The WLAN connection now. Asked often: an answer from memory, not from the radio.
    fn wlan_status(&mut self) -> WlanStatus {
        WlanStatus::Off
    }

    /// Starts looking for networks; [`networks`](Backend::networks) has them when done.
    fn scan(&mut self) {}

    /// The networks the last [`scan`](Backend::scan) found, strongest first, once it is done.
    fn networks(&mut self) -> Option<Vec<Network>> {
        Some(Vec::new())
    }

    /// Connects to the network `ssid`, with `password` for a new one that needs it (the board
    /// saves it once the connection stands), or the saved one.
    fn connect(&mut self, _ssid: &str, _password: Option<&str>) {}

    /// The networks saved, in the order they were saved.
    fn saved_networks(&mut self) -> Vec<String> {
        Vec::new()
    }

    /// Forgets the saved network `ssid` and its password, and leaves it if connected.
    fn forget(&mut self, _ssid: &str) {}

    /// Sets the clock to `utc_ms` (milliseconds since 1970, UTC), typed by hand. A board that
    /// can set its system clock does that instead of moving [`clock::now`].
    /// The apps there are, built in and on the card, with what the list shows of them.
    fn apps(&mut self, _lang: crate::text::Lang) -> Vec<crate::applet::AppInfo> {
        Vec::new()
    }

    /// The app `id`, started.
    fn open_app(&mut self, _id: &str) -> Result<Box<dyn crate::applet::Applet>, String> {
        Err("No apps".to_string())
    }

    /// A random number, for apps. The default is a generator over a fixed seed.
    fn random(&mut self) -> u32 {
        use core::sync::atomic::{AtomicU32, Ordering};
        static STATE: AtomicU32 = AtomicU32::new(0x2545_f491);
        let mut x = STATE.load(Ordering::Relaxed);
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        STATE.store(x, Ordering::Relaxed);
        x
    }

    fn set_time(&mut self, utc_ms: i64) {
        clock::set(utc_ms);
    }
}

/// What STO and VARS ask of the variables of the open session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VariableCommand {
    /// Nothing: just the variables.
    List,
    /// The value of `latex`, worked out in `mode`, kept under the name `key` (`A`, `r_1`).
    Store { mode: Mode, key: String, latex: String },
    Delete(String),
}

/// A variable of the session: its key (`r_1`), its name as LaTeX (`r_{1}`) and its value with
/// its unit, as LaTeX.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stored {
    pub key: String,
    pub symbol: String,
    pub value: String,
}

/// What CONST and CONV list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Catalogue {
    Constants,
    Units,
    /// SHIFT+OPTN: every function and template of the mode's OPTN menu, alphabetically; built
    /// by the app ([`crate::tree::functions`]), not by the backend.
    Functions,
}

/// A category of a [`Catalogue`]: Universelle Konstanten, Länge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    pub name: String,
    pub items: Vec<Item>,
}

/// A constant or a unit: its name, and the LaTeX that is inserted and shown beside the name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub name: String,
    pub latex: String,
}

/// What an entry of the search index is: a unit is inserted after ▶, the others as they are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexKind {
    Constant,
    Unit,
    Function,
    Prefix,
}

/// An entry of the engine's search index (`api::library_index`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexItem {
    pub kind: IndexKind,
    /// What is inserted and shown beside the name.
    pub latex: String,
    /// The name in the set language.
    pub name: String,
    /// Every name it is found by: the set language's, the English one, aliases.
    pub names: Vec<String>,
    /// The symbols as typed (`c0`, `km/h`, `sqrt`).
    pub symbols: Vec<String>,
}

/// The units for the soft keys, best first: `best` for the first row, `more` for the second
/// and the pages after it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Units {
    pub best: Vec<Item>,
    pub more: Vec<Item>,
}

impl Units {
    /// The common units of mechanics, electricity and heat.
    pub fn common() -> Units {
        let item = |name: &str| Item { name: name.to_string(), latex: format!(r"\mathrm{{{name}}}") };
        Units { best: ["m", "s", "kg", "N", "J", "W"].map(item).to_vec(), more: ["V", "A", "Pa", "K", "Hz", "km"].map(item).to_vec() }
    }
}

/// The WLAN connection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WlanStatus {
    Off,
    /// On, but connected to none: no saved network is in reach.
    Searching,
    Connecting(String),
    /// `ip` written out, `rssi` in dBm.
    Connected { ssid: String, ip: String, rssi: i8 },
    /// The network `ssid` did not take the connection: the password was wrong, or it went away.
    Failed(String),
}

impl WlanStatus {
    pub fn connected(&self) -> bool {
        matches!(self, WlanStatus::Connected { .. })
    }
}

/// A network found by a scan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Network {
    pub ssid: String,
    /// In dBm.
    pub rssi: i8,
    /// Whether it needs a password.
    pub secured: bool,
    /// Whether it is saved, with its password.
    pub saved: bool,
}

impl Network {
    /// The signal as 1 to 4 bars.
    pub fn bars(&self) -> u8 {
        bars(self.rssi)
    }
}

/// The signal `rssi` (dBm) as 1 to 4 bars.
pub fn bars(rssi: i8) -> u8 {
    match rssi {
        -55.. => 4,
        -67.. => 3,
        -78.. => 2,
        _ => 1,
    }
}

/// A session as the menus show it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
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
/// result written as the settings say; and what the sheets held, as their requests.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Opened {
    pub info: SessionInfo,
    pub history: Vec<(String, String)>,
    pub inputs: Vec<Request>,
}

/// What a sheet asks the engine, its cells as LaTeX (an empty one is left out or 0).
#[derive(Clone, Debug, PartialEq)]
pub enum Request {
    /// The rows of `[A | b]`.
    Linear { rows: Vec<Vec<String>> },
    /// f(x), and g(x) beside it if given, from `from` to `to` in steps of `step`.
    Table { f: String, g: Option<String>, from: String, to: String, step: String },
    /// The data; `y` for pairs (a regression), `freq` how often each row counts; where the
    /// regression is to estimate x̂ and ŷ.
    Statistics { kind: StatisticsType, x: Vec<String>, y: Option<Vec<String>>, freq: Option<Vec<String>>, at: Estimates },
    /// The fields the distribution reads; `k` one value or several.
    Distribution { kind: DistributionKind, fields: Vec<(Field, String)>, k: Vec<String> },
    /// The curve sketch of f(x), section by section, with its graph.
    Analysis { f: String },
    /// The tangent and the normal to f at `x0`.
    Tangent { f: String, x0: String },
    /// The area between f and the x-axis from `from` to `to`; both empty for the outer zeros.
    Area { f: String, from: String, to: String },
    /// The curve of f in `window`, for a graph moved or zoomed.
    Sample { f: String, window: Window },
    /// The letters of `latex` that have no value yet: what CALC asks for.
    Variables { mode: Mode, latex: String },
    /// CALC: `latex` worked out in `mode` with the letters given these values, each a key
    /// and its value as LaTeX. The values stay in the session's variables.
    Calc { mode: Mode, latex: String, values: Vec<(String, String)> },
    /// SOLVE: `latex` solved for the letter `variable`, the others given `values`; `start`
    /// begins Newton's method where no exact method applies.
    Solve { mode: Mode, latex: String, variable: String, values: Vec<(String, String)>, start: Option<String> },
}

impl Request {
    /// The function a request of Kurvendiskussion is about.
    pub fn function(&self) -> Option<&str> {
        match self {
            Request::Analysis { f } | Request::Tangent { f, .. } | Request::Area { f, .. } => Some(f),
            _ => None,
        }
    }
}

/// The values a regression estimates at: a y for x̂, an x for ŷ.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Estimates {
    pub x_hat: Option<String>,
    pub y_hat: Option<String>,
}

/// An input of a distribution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    X,
    Lower,
    Upper,
    Area,
    Sigma,
    Mu,
    N,
    P,
    K,
}

impl Field {
    pub const ALL: [Field; 9] =
        [Field::X, Field::Lower, Field::Upper, Field::Area, Field::Sigma, Field::Mu, Field::N, Field::P, Field::K];

    /// The fields `kind` reads, in the order its form shows them.
    pub fn of(kind: DistributionKind) -> &'static [Field] {
        use DistributionKind as D;
        match kind {
            D::NormalDensity => &[Field::X, Field::Sigma, Field::Mu],
            D::NormalCumulative => &[Field::Lower, Field::Upper, Field::Sigma, Field::Mu],
            D::InverseNormal => &[Field::Area, Field::Sigma, Field::Mu],
            D::BinomialDensity | D::BinomialCumulative => &[Field::K, Field::N, Field::P],
            D::PoissonDensity | D::PoissonCumulative => &[Field::K, Field::Mu],
        }
    }

    pub fn label(self) -> text::Text {
        use text::{same, tx};
        match self {
            Field::X => same("x"),
            Field::Lower => tx!("hui.backend.lower"),
            Field::Upper => tx!("hui.backend.upper"),
            Field::Area => tx!("hui.backend.area"),
            Field::Sigma => same("σ"),
            Field::Mu => same("μ"),
            Field::N => same("n"),
            Field::P => same("p"),
            Field::K => same("k"),
        }
    }
}

/// What the engine answers a sheet: lines to read (LGS, Statistik, one probability), a table
/// (Wertetabelle, probabilities for several k), a curve sketch, or a task on the curve.
#[derive(Clone, Debug, PartialEq)]
pub enum Reply {
    Lines(Vec<Line>),
    /// The column heads and the rows, as LaTeX; an empty cell shows as a dash.
    Table { header: Vec<String>, rows: Vec<Vec<String>> },
    Analysis(Analysis),
    Task(Task),
    /// The curve of a [`Request::Sample`], in pieces.
    Curve(Vec<Vec<(f32, f32)>>),
    /// The letters a [`Request::Variables`] found.
    Variables(Vec<Variable>),
    /// A result as LaTeX, written as the settings say: a calculation's (CALC), the solutions of
    /// an equation (SOLVE).
    Result(String),
}

impl Reply {
    /// How many lines or rows it has, the steps of its scrolling; the pages of a sketch.
    pub fn len(&self) -> usize {
        match self {
            Reply::Lines(lines) => lines.len(),
            Reply::Table { rows, .. } => rows.len(),
            Reply::Analysis(analysis) => analysis.pages.len(),
            Reply::Task(task) => task.lines.len() + task.result.len(),
            Reply::Curve(pieces) => pieces.len(),
            Reply::Variables(variables) => variables.len(),
            Reply::Result(_) => 1,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A letter of an input: the key the session knows it by (`m_1`), and how it is written
/// (`m_{1}`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variable {
    pub key: String,
    pub symbol: String,
}

/// A line of an answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Line {
    /// LaTeX: `x_{1} = 2`, `\bar{x} = 3`.
    Math(String),
    /// Plain text, a heading or what a step does.
    Note(String),
    /// A step of the elimination: the matrix `[A | b]`.
    Matrix(Matrix),
    /// A sign table: rows of cells as LaTeX, the first row the heads (the x intervals).
    Grid(Vec<Vec<String>>),
}

/// A curve sketch: f, a page per section, and the graph.
#[derive(Clone, Debug, PartialEq)]
pub struct Analysis {
    /// `f(x) = …`, as LaTeX.
    pub function: String,
    pub pages: Vec<Page>,
    pub graph: Graph,
}

/// A section of the sketch: Ableitungen, Definitionsbereich, Nullstellen …
#[derive(Clone, Debug, PartialEq)]
pub struct Page {
    pub title: String,
    pub status: Status,
    /// Why it is partial or failed.
    pub message: Option<String>,
    /// The steps.
    pub lines: Vec<Line>,
    /// What the section finds, after its steps.
    pub result: Vec<Line>,
}

impl Page {
    /// The steps and the result, each line in turn.
    pub fn len(&self) -> usize {
        self.lines.len() + self.result.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// How far the engine got with a section.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Status {
    #[default]
    Ok,
    Partial,
    Failed,
}

/// The graph of f in its window, and what is marked on it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Graph {
    pub window: Window,
    /// The curve, in pieces that break at poles and gaps.
    pub curve: Vec<Vec<(f32, f32)>>,
    pub points: Vec<Mark>,
    pub asymptotes: Vec<Asymptote>,
    /// A task's tangent, normal or area, drawn over it.
    pub overlay: Overlay,
}

/// The part of the plane a graph shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Window {
    pub x_min: f32,
    pub x_max: f32,
    pub y_min: f32,
    pub y_max: f32,
}

impl Default for Window {
    fn default() -> Self {
        Window { x_min: -5.0, x_max: 5.0, y_min: -5.0, y_max: 5.0 }
    }
}

/// A point marked on the graph.
#[derive(Clone, Debug, PartialEq)]
pub struct Mark {
    pub kind: MarkKind,
    /// Its name as LaTeX: `H`, `N_{1}`, `S_y`.
    pub name: String,
    pub x: f32,
    pub y: f32,
    /// For a member of a periodic family: the distance to the next; the graph marks them all.
    pub period: Option<f32>,
}

/// An asymptote; a vertical one with a period stands for all the poles of its family (tan).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Asymptote {
    pub line: Straight,
    pub period: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkKind {
    Zero,
    YIntercept,
    Max,
    Min,
    Inflection,
    Saddle,
    /// A gap in the curve, drawn as a ring.
    Hole,
    /// The highest or lowest point at an end of the domain.
    Boundary,
    /// Where a tangent touches.
    Touch,
}

/// A straight line: an asymptote, a tangent, a normal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Straight {
    Vertical(f32),
    Sloped { slope: f32, intercept: f32 },
}

impl Straight {
    /// y at `x`; `None` for a vertical line.
    pub fn at(self, x: f32) -> Option<f32> {
        match self {
            Straight::Vertical(_) => None,
            Straight::Sloped { slope, intercept } => Some(slope * x + intercept),
        }
    }
}

/// What a task adds to the graph.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Overlay {
    pub tangent: Option<Straight>,
    pub normal: Option<Straight>,
    pub point: Option<Mark>,
    pub regions: Vec<Region>,
}

/// A piece of an area: the curve over it, from one zero or bound to the next.
#[derive(Clone, Debug, PartialEq)]
pub struct Region {
    /// Above the x-axis.
    pub positive: bool,
    pub curve: Vec<(f32, f32)>,
}

/// The answer to a task on the curve: its steps, its result and what it adds to the graph.
#[derive(Clone, Debug, PartialEq)]
pub struct Task {
    /// f, as the task was given it.
    pub function: String,
    /// What the task is: Tangente, Normale; Fläche.
    pub title: String,
    pub lines: Vec<Line>,
    pub result: Vec<Line>,
    pub overlay: Overlay,
    /// The sketch's graph with the overlay on it, when the app has a sketch of the same f.
    pub graph: Option<Graph>,
}

/// An augmented matrix as a step shows it, its cells as LaTeX.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Matrix {
    /// The rows' names left of the matrix (I, II, …), if any.
    pub names: Vec<String>,
    pub rows: Vec<Vec<String>>,
    /// The columns left of the bar.
    pub split: usize,
    /// What was done to each row, right of the matrix (`2 \cdot \text{II} - \text{I}`), if anything.
    pub notes: Vec<String>,
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
    let text = text.replace('.', lang.decimal());
    format!("{text} {}", UNITS[unit])
}

/// y = x² from −3 to 3, with its vertex: the stub's graph.
fn parabola() -> Graph {
    let curve = vec![(-30..=30).map(|i| (i as f32 / 10.0, (i as f32 / 10.0).powi(2))).collect()];
    let vertex = Mark { kind: MarkKind::Min, name: "T".into(), x: 0.0, y: 0.0, period: None };
    Graph { window: Window { x_min: -3.0, x_max: 3.0, y_min: -1.0, y_max: 9.0 }, curve, points: vec![vertex], ..Graph::default() }
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

    /// The steps, to see ENG without the engine.
    fn engineering(&mut self, _mode: Mode, _settings: &Settings, latex: &str, steps: i32) -> Option<Result<String, String>> {
        Some(Ok(format!("{latex} \\cdot 10^{{{}}}", 3 * steps)))
    }

    /// Two constants and two units, to see the menus without the engine.
    fn catalogue(&mut self, catalogue: Catalogue, _lang: crate::text::Lang) -> Vec<Group> {
        let item = |name: &str, latex: &str| Item { name: name.into(), latex: latex.into() };
        match catalogue {
            Catalogue::Constants => vec![Group {
                name: "Universell".into(),
                items: vec![item("Lichtgeschwindigkeit", "c_{0}"), item("Planck-Konstante", "h_{P}")],
            }],
            Catalogue::Units => vec![Group {
                name: "Länge".into(),
                items: vec![item("Meter", r"\mathrm{m}"), item("Zoll", r"\mathrm{in}")],
            }],
            Catalogue::Functions => Vec::new(),
        }
    }

    /// The cells back: the first row of an LGS, the table's f(x) for x = from, the x column of
    /// the data, a distribution's fields; enough to see the result page without the engine.
    fn solve(&mut self, _settings: &Settings, request: &Request) -> Result<Reply, String> {
        let math = |cells: &[String]| Line::Math(cells.join("; "));
        Ok(match request {
            Request::Linear { rows } => Reply::Lines(rows.iter().map(|row| math(row)).collect()),
            Request::Table { f, from, .. } => {
                Reply::Table { header: vec!["x".into(), "f(x)".into()], rows: vec![vec![from.clone(), f.clone()]] }
            }
            Request::Statistics { x, .. } => Reply::Lines(vec![Line::Math(format!("n = {}", x.len())), math(x)]),
            Request::Distribution { fields, k, .. } => {
                let fields: Vec<String> = fields.iter().map(|(_, cell)| cell.clone()).collect();
                Reply::Lines(vec![math(&fields), math(k)])
            }
            Request::Analysis { f } => Reply::Analysis(Analysis {
                function: format!("f(x) = {f}"),
                pages: vec![
                    Page { title: "f(x)".into(), status: Status::Ok, message: None, lines: vec![Line::Note("f".into())], result: vec![Line::Math(f.clone())] },
                    Page { title: "?".into(), status: Status::Failed, message: Some("Stub".into()), lines: Vec::new(), result: Vec::new() },
                ],
                graph: parabola(),
            }),
            Request::Tangent { f, x0 } => {
                let tangent = Straight::Sloped { slope: 1.0, intercept: 0.0 };
                let overlay = Overlay { tangent: Some(tangent), ..Overlay::default() };
                let result = vec![Line::Math(format!("x_0 = {x0}"))];
                Reply::Task(Task { function: f.clone(), title: "t".into(), lines: Vec::new(), result, overlay, graph: None })
            }
            Request::Sample { window, .. } => {
                let (low, high) = (window.x_min, window.x_max);
                Reply::Curve(vec![(0..=20).map(|i| low + (high - low) * i as f32 / 20.0).map(|x| (x, x * x)).collect()])
            }
            // The single letters, in order, each once; a command's name is not one.
            Request::Variables { latex, .. } => {
                let mut variables: Vec<Variable> = Vec::new();
                let mut command = false;
                for c in latex.chars() {
                    command = match c {
                        '\\' => true,
                        c if c.is_ascii_alphabetic() && command => true,
                        c if c.is_ascii_alphabetic() => {
                            let key = c.to_string();
                            if !variables.iter().any(|variable| variable.key == key) {
                                variables.push(Variable { symbol: key.clone(), key });
                            }
                            false
                        }
                        _ => false,
                    };
                }
                Reply::Variables(variables)
            }
            // The values given, after the input.
            Request::Calc { latex, values, .. } => {
                let values: Vec<String> = values.iter().map(|(key, value)| format!("{key}={value}")).collect();
                Reply::Result(format!("{latex}; {}", values.join("; ")))
            }
            Request::Solve { variable, start, .. } => Reply::Result(format!("{variable}={}", start.as_deref().unwrap_or("?"))),
            Request::Area { f, from, to } => {
                let curve = (0..=10).map(|i| (i as f32 / 10.0, (i as f32 / 10.0).powi(2))).collect();
                let overlay = Overlay { regions: vec![Region { positive: true, curve }], ..Overlay::default() };
                let result = vec![Line::Math(format!("{from}; {to}"))];
                Reply::Task(Task { function: f.clone(), title: "A".into(), lines: Vec::new(), result, overlay, graph: None })
            }
        })
    }
}
