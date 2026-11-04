//! The handheld's UI in a window, at twice its size, with the PC keyboard as the keypad (see the
//! README), and the real engine answering; LaTeX typed into the terminal is evaluated too.
//!
//! - `--keys <text> --latex <latex> --png <file>`: types the keys, evaluates the LaTeX (both
//!   optional) and saves the screen instead of opening the window.
//! - `--stub`: the stub answers instead of the engine.
//! - `--mirror <port>`: also shows the picture on the board; `--baud <n>` if the board's
//!   `mirror` was built with another `QUANTUS_MIRROR_BAUD`.
//! - `--charging`, `--busy`: start with the battery charging or the busy spinner turning; F9 and
//!   F10 switch them in the window.
//! - `--tour`: plays `quantus_ui::tour` in the window, with a person's pauses between the keys:
//!   the first wallpaper everywhere, the SETUP pages, the mode tiles, OPTN and some
//!   calculations. Works with `--mirror`.
//! - The mouse is the finger: a click taps, a drag moves a list or the mode tiles.
//! - `--sd <dir>`: the folder that stands for the SD card (`sd` by default); the wallpapers are
//!   the 480×320 BMP files in its `wallpapers` folder, the sessions the JSON files in its
//!   `sessions` folder (with `--png` they are kept in memory, so a screenshot changes nothing),
//!   the settings `settings.txt` in it, the saved WLAN networks `wlan.txt`. WLAN is played: three
//!   made-up networks, any password taken.

mod mirror;

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics_simulator::sdl2::{Keycode, Mod, MouseButton};
use embedded_graphics_simulator::{OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window};
use mirror::{Mirror, Tee};
use quantus_engine::Engine;
use quantus_ui::app::{App, Mode, Settings};
use quantus_ui::backend::{Backend, Catalogue, Group, Network, Opened, Reply, Request, SessionCommand, SessionInfo, Storage, Stub, WlanStatus};
use quantus_ui::text::Lang;
use quantus_ui::key::Key;
use quantus_ui::render::{Frame, Renderer, HEIGHT, WIDTH};
use quantus_ui::touch::Touch;

fn main() {
    let mut display = SimulatorDisplay::<Rgb565>::new(Size::new(WIDTH, HEIGHT));
    let args: Vec<String> = std::env::args().collect();
    let sd = args.iter().position(|arg| arg == "--sd").and_then(|i| args.get(i + 1)).map_or("sd", String::as_str);
    let sessions = (!args.iter().any(|arg| arg == "--png")).then(|| Path::new(sd).join("sessions"));
    let keep = sessions.is_some();
    let answers = if args.iter().any(|arg| arg == "--stub") { Answers::Stub(Stub) } else { Answers::Engine(Engine::new(sessions)) };
    let wlan = FakeWlan::load(Path::new(sd));
    let mut app = App::new(Sim { answers, sd: PathBuf::from(sd), keep, wlan });
    // The PC has no battery to read; show a full one.
    app.battery = Some(100);
    // What the board will report, shown on demand: F9 and F10 switch them in the window.
    app.charging = args.iter().any(|arg| arg == "--charging");
    app.busy = args.iter().any(|arg| arg == "--busy");
    let mut renderer = Renderer::new();

    let option = |name: &str| args.iter().position(|arg| arg == name).and_then(|i| args.get(i + 1));
    if let Some(png) = option("--png") {
        let keys = option("--keys").map_or("", String::as_str);
        // `\e` for Esc is not needed here: Ctrl+X clears. `\n` is EXE, `>`/`<` and `↑`/`↓` move the
        // cursor, `✓` is OK.
        for c in keys.replace("\\n", "\n").chars() {
            let key = match c {
                '>' => Some(Key::Right),
                '<' => Some(Key::Left),
                '↓' => Some(Key::Down),
                '↑' => Some(Key::Up),
                '✓' => Some(Key::Ok),
                c => Key::from_char(c),
            };
            key.into_iter().for_each(|key| app.key(key));
        }
        if let Some(latex) = option("--latex") {
            evaluate_latex(&mut app, latex);
        }
        renderer.render(&Frame::new(&app), &mut display).unwrap();
        let settings = OutputSettingsBuilder::new().build();
        display.to_rgb_output_image(&settings).save_png(png).expect("write the png");
        return;
    }

    let baud = option("--baud").and_then(|baud| baud.parse().ok()).unwrap_or(quantus_mirror::BAUD);
    let mut mirror = match option("--mirror").map(|port| Mirror::open(port, baud)) {
        Some(Ok(mirror)) => Some(mirror),
        Some(Err(error)) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
        None => None,
    };

    print!("{KEYS}");
    let latex = terminal_lines();
    let settings = OutputSettingsBuilder::new().scale(2).build();
    let mut window = Window::new("Quantus", &settings);
    let start = Instant::now();
    let mut tour: std::collections::VecDeque<_> =
        if args.iter().any(|arg| arg == "--tour") { quantus_ui::tour::tour().into() } else { Default::default() };
    let mut last_step = Instant::now();
    // Draw only when something changed: a key, the caret, the spinner or a new mirror.
    let mut changed = true;
    loop {
        if let Ok(line) = latex.try_recv() {
            evaluate_latex(&mut app, &line);
            changed = true;
        }
        if mirror.as_mut().is_some_and(Mirror::take_ready) {
            renderer.invalidate();
            changed = true;
        }
        if changed {
            let mut target = Tee { display: &mut display, mirror: mirror.as_mut() };
            renderer.render(&Frame::new(&app), &mut target).unwrap();
        }
        window.update(&display);
        changed = false;
        for event in window.events() {
            match event {
                SimulatorEvent::Quit => return,
                SimulatorEvent::KeyDown { keycode: Keycode::F9, .. } => {
                    app.charging = !app.charging;
                    changed = true;
                }
                SimulatorEvent::KeyDown { keycode: Keycode::F10, .. } => {
                    app.busy = !app.busy;
                    changed = true;
                }
                SimulatorEvent::KeyDown { keycode, keymod, .. } => {
                    if let Some(key) = key(keycode, keymod) {
                        app.key(key);
                        changed = true;
                    }
                }
                // The mouse is the finger.
                SimulatorEvent::MouseButtonDown { mouse_btn: MouseButton::Left, point } => {
                    changed |= app.touch(Touch::Press(point));
                }
                SimulatorEvent::MouseMove { point } => changed |= app.touch(Touch::Drag(point)),
                SimulatorEvent::MouseButtonUp { mouse_btn: MouseButton::Left, .. } => {
                    changed |= app.touch(Touch::Release);
                }
                _ => {}
            }
        }
        if let Some(step) = tour.front().filter(|step| last_step.elapsed() >= Duration::from_millis(step.pause_ms.into())) {
            if let Some(note) = step.note {
                println!("tour: {note}");
            }
            app.key(step.key);
            changed = true;
            tour.pop_front();
            last_step = Instant::now();
            if tour.is_empty() {
                println!("tour: done");
            }
        }
        changed |= app.tick(start.elapsed().as_millis() as u64);
        std::thread::sleep(Duration::from_millis(15));
    }
}

/// The lines typed into the terminal, read in the background.
fn terminal_lines() -> Receiver<String> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        for line in std::io::stdin().lines() {
            let Ok(line) = line else { return };
            if !line.trim().is_empty() && sender.send(line.trim().to_string()).is_err() {
                return;
            }
        }
    });
    receiver
}

/// LaTeX typed into the terminal, as the input on the screen, evaluated. The editor reads it the
/// way it reads results, so the LaTeX that goes to the engine is printed: anything the editor
/// does not know shows up there.
fn evaluate_latex(app: &mut App<Sim>, latex: &str) {
    app.key(Key::Ac);
    app.editor.set(quantus_ui::latex::parse(latex));
    println!("sent: {}", app.editor.to_latex());
    app.key(Key::Exe);
}

/// What answers: the engine, or the stub with `--stub`.
enum Answers {
    Engine(Engine),
    Stub(Stub),
}

/// The size of the card the SD folder stands for.
const SD_CAPACITY: u64 = 32_000_000_000;

/// The answers, and a folder on the PC as the SD card.
struct Sim {
    answers: Answers,
    sd: PathBuf,
    /// Whether changes are written to the folder; not for a `--png` screenshot.
    keep: bool,
    wlan: FakeWlan,
}

/// The networks the simulator pretends to find: name, signal, whether it needs a password.
const NETWORKS: [(&str, i8, bool); 3] = [("Quantus-Labor", -45, true), ("FRITZ!Box 7590", -63, true), ("Café Gast", -79, false)];

/// How long a scan and a connection take, as on the board.
const SCAN: Duration = Duration::from_millis(1500);
const CONNECT: Duration = Duration::from_millis(1000);

/// WLAN as the simulator plays it: the networks of [`NETWORKS`], any password taken, the saved
/// ones in `wlan.txt` in the SD folder (a name and a password per line, split by a tab).
struct FakeWlan {
    on: bool,
    saved: Vec<(String, String)>,
    scanned: Option<Instant>,
    /// The network and since when it is being joined.
    joined: Option<(String, Instant)>,
}

impl FakeWlan {
    fn load(sd: &Path) -> FakeWlan {
        let text = std::fs::read_to_string(sd.join("wlan.txt")).unwrap_or_default();
        let saved = text.lines().filter_map(|line| line.split_once('\t')).map(|(s, p)| (s.to_string(), p.to_string())).collect();
        FakeWlan { on: false, saved, scanned: None, joined: None }
    }

    fn keep(&self, sd: &Path) {
        let text: String = self.saved.iter().map(|(ssid, password)| format!("{ssid}\t{password}\n")).collect();
        if let Err(error) = std::fs::create_dir_all(sd).and_then(|()| std::fs::write(sd.join("wlan.txt"), text)) {
            eprintln!("saved networks not kept: {error}");
        }
    }

    /// Joins the first saved network in reach.
    fn join_saved(&mut self) {
        let saved = NETWORKS.iter().find(|(ssid, ..)| self.saved.iter().any(|(name, _)| name == ssid));
        self.joined = saved.map(|(ssid, ..)| (ssid.to_string(), Instant::now()));
    }
}

impl Backend for Sim {
    fn evaluate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> Result<String, String> {
        match &mut self.answers {
            Answers::Engine(engine) => engine.evaluate(mode, settings, latex),
            Answers::Stub(stub) => stub.evaluate(mode, settings, latex),
        }
    }

    fn recalculate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> Option<Result<String, String>> {
        match &mut self.answers {
            Answers::Engine(engine) => engine.recalculate(mode, settings, latex),
            Answers::Stub(stub) => stub.recalculate(mode, settings, latex),
        }
    }

    fn sessions(&mut self) -> Vec<SessionInfo> {
        match &mut self.answers {
            Answers::Engine(engine) => engine.sessions(),
            Answers::Stub(stub) => stub.sessions(),
        }
    }

    fn catalogue(&mut self, catalogue: Catalogue, lang: Lang) -> Vec<Group> {
        match &mut self.answers {
            Answers::Engine(engine) => engine.catalogue(catalogue, lang),
            Answers::Stub(stub) => stub.catalogue(catalogue, lang),
        }
    }

    fn solve(&mut self, settings: &Settings, request: &Request) -> Result<Reply, String> {
        match &mut self.answers {
            Answers::Engine(engine) => engine.solve(settings, request),
            Answers::Stub(stub) => stub.solve(settings, request),
        }
    }

    fn session(&mut self, command: SessionCommand, settings: &Settings) -> Result<Opened, String> {
        match &mut self.answers {
            Answers::Engine(engine) => engine.session(command, settings),
            Answers::Stub(stub) => stub.session(command, settings),
        }
    }

    /// `settings.txt` in the SD folder.
    fn load_settings(&mut self) -> Option<Settings> {
        std::fs::read_to_string(self.sd.join("settings.txt")).ok().map(|text| Settings::from_text(&text))
    }

    fn wlan_status(&mut self) -> WlanStatus {
        match &self.wlan.joined {
            _ if !self.wlan.on => WlanStatus::Off,
            None => WlanStatus::Searching,
            Some((ssid, since)) if since.elapsed() < CONNECT => WlanStatus::Connecting(ssid.clone()),
            Some((ssid, _)) => {
                let rssi = NETWORKS.iter().find(|(name, ..)| name == ssid).map_or(-70, |network| network.1);
                WlanStatus::Connected { ssid: ssid.clone(), ip: "192.168.178.42".to_string(), rssi }
            }
        }
    }

    fn scan(&mut self) {
        self.wlan.scanned = Some(Instant::now());
    }

    fn networks(&mut self) -> Option<Vec<Network>> {
        if self.wlan.scanned.is_some_and(|since| since.elapsed() < SCAN) {
            return None;
        }
        let saved = |ssid: &str| self.wlan.saved.iter().any(|(name, _)| name == ssid);
        let found = NETWORKS.iter().map(|&(ssid, rssi, secured)| Network { ssid: ssid.to_string(), rssi, secured, saved: saved(ssid) });
        Some(found.collect())
    }

    fn connect(&mut self, ssid: &str, password: Option<&str>) {
        if let Some(password) = password {
            self.wlan.saved.retain(|(name, _)| name != ssid);
            self.wlan.saved.push((ssid.to_string(), password.to_string()));
            if self.keep {
                self.wlan.keep(&self.sd);
            }
        }
        self.wlan.joined = Some((ssid.to_string(), Instant::now()));
    }

    fn saved_networks(&mut self) -> Vec<String> {
        self.wlan.saved.iter().map(|(ssid, _)| ssid.clone()).collect()
    }

    fn forget(&mut self, ssid: &str) {
        self.wlan.saved.retain(|(name, _)| name != ssid);
        if self.keep {
            self.wlan.keep(&self.sd);
        }
        if self.wlan.joined.as_ref().is_some_and(|(name, _)| name == ssid) {
            self.wlan.join_saved();
        }
    }

    fn use_settings(&mut self, settings: &Settings) {
        if settings.wlan != self.wlan.on {
            self.wlan.on = settings.wlan;
            if settings.wlan {
                self.wlan.join_saved();
            }
        }
        let path = self.sd.join("settings.txt");
        let text = settings.to_text();
        if self.keep && std::fs::read_to_string(&path).ok().as_deref() != Some(text.as_str()) {
            if let Err(error) = std::fs::create_dir_all(&self.sd).and_then(|()| std::fs::write(&path, text)) {
                eprintln!("settings not kept in {}: {error}", path.display());
            }
        }
    }

    fn wallpapers(&mut self) -> Vec<String> {
        let Ok(files) = std::fs::read_dir(self.sd.join("wallpapers")) else { return Vec::new() };
        let mut names: Vec<String> = files
            .filter_map(|file| {
                let path = file.ok()?.path();
                let bmp = path.extension()?.eq_ignore_ascii_case("bmp");
                bmp.then(|| path.file_stem()?.to_str().map(str::to_string)).flatten()
            })
            .collect();
        names.sort();
        names
    }

    fn wallpaper(&mut self, name: &str) -> Option<Vec<u8>> {
        std::fs::read(self.sd.join("wallpapers").join(format!("{name}.bmp"))).ok()
    }

    /// The folder as a card of [`SD_CAPACITY`]: what its files take is used.
    fn storage(&mut self) -> Option<Storage> {
        fn size(path: &Path) -> u64 {
            let Ok(files) = std::fs::read_dir(path) else { return 0 };
            files
                .filter_map(Result::ok)
                .map(|file| match file.file_type() {
                    Ok(kind) if kind.is_dir() => size(&file.path()),
                    _ => file.metadata().map_or(0, |metadata| metadata.len()),
                })
                .sum()
        }
        self.sd.is_dir().then(|| {
            let used = size(&self.sd).min(SD_CAPACITY);
            Storage { total: SD_CAPACITY, free: SD_CAPACITY - used }
        })
    }
}

/// The PC keys for the calculator's, printed at start (the full table is in the README).
const KEYS: &str = "\
Quantus simulator keys
  MODE (mode tiles)    Tab, then 1-9 or arrows and Enter (12 tiles, 2 pages)
  Setup (SHIFT+MODE)   F1, then Tab; a digit or Up/Down and Enter open an entry,
                       Left goes back, Esc closes
  SHIFT / ALPHA        F1 or Ctrl+F / F2 or Ctrl+K
  S<=>D                F3 or Ctrl+D
  OPTN (mode's options) F4 or Ctrl+N
  OK                   F5 (on a sheet: edits the cell as it is; Enter works it out)
  CONST                F6 or Ctrl+U (the constants, in Physik)
  CONV                 F7 or Ctrl+Y (convert to a unit)
  EXE / DEL / AC       Enter / Backspace / Esc, Delete or Ctrl+X
  fraction ^ index     /  ^  _
  sqrt x^2 pi e        Ctrl+W  Ctrl+Q  Ctrl+P  Ctrl+E
  sin cos tan          Ctrl+S  Ctrl+O  Ctrl+T
  ln log Ans           Ctrl+L  Ctrl+G  Ctrl+A
  ;                    Shift+, (between equations and arguments)
  history              Up/Down outside a fraction
  charging / busy      F9 / F10 (what the board will report)
  touch                the mouse is the finger: click to tap, drag to scroll
  Shifted characters follow a German layout (Shift+7 is /, Shift+0 is =).
LaTeX: type it here in the terminal and press Enter; it becomes the input and is evaluated.
";

/// The key for a key press. SDL reports the key's unshifted character, so the shifted ones are
/// those of a German layout.
fn key(keycode: Keycode, keymod: Mod) -> Option<Key> {
    let shift = keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD);
    let ctrl = keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD);
    match keycode {
        Keycode::Left => return Some(Key::Left),
        Keycode::Right => return Some(Key::Right),
        Keycode::Up => return Some(Key::Up),
        Keycode::Down => return Some(Key::Down),
        Keycode::Escape | Keycode::Delete => return Some(Key::Ac),
        Keycode::Return | Keycode::KpEnter => return Some(Key::Exe),
        Keycode::F1 => return Some(Key::Shift),
        Keycode::F2 => return Some(Key::Alpha),
        Keycode::F3 => return Some(Key::SwapForm),
        Keycode::F4 => return Some(Key::Optn),
        Keycode::F5 => return Some(Key::Ok),
        Keycode::F6 => return Some(Key::Const),
        Keycode::F7 => return Some(Key::Conv),
        Keycode::KpPlus => return Some(Key::Plus),
        Keycode::KpMinus => return Some(Key::Minus),
        Keycode::KpMultiply => return Some(Key::Times),
        Keycode::KpDivide => return Some(Key::Fraction),
        Keycode::KpComma | Keycode::KpPeriod => return Some(Key::Comma),
        _ => {}
    }
    // Keypad digits.
    match keycode.into_i32() - Keycode::Kp1.into_i32() {
        offset @ 0..=8 => return Some(Key::Digit(offset as u8 + 1)),
        9 => return Some(Key::Digit(0)),
        _ => {}
    }
    let c = char::from_u32(keycode.into_i32() as u32)?;
    let c = match c {
        _ if ctrl && c.is_ascii_lowercase() => char::from(c as u8 - b'a' + 1),
        _ if !shift => c,
        '7' => '/',
        '8' => '(',
        '9' => ')',
        '0' => '=',
        '+' => '*',
        '.' => ':',
        ',' => ';',
        '-' => '_',
        c => c.to_uppercase().next()?,
    };
    Key::from_char(c)
}
