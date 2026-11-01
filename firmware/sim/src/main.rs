//! The handheld's UI in a window, at twice its size, with the PC keyboard as the keypad (see the
//! README), and the real engine answering. `--keys <text> --png <file>` types the text and saves
//! the screen instead; `--stub` answers with the stub instead of the engine; `--mirror <port>`
//! also shows the picture on the board.

mod engine;
mod mirror;

use std::time::{Duration, Instant};

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics_simulator::sdl2::{Keycode, Mod};
use embedded_graphics_simulator::{OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window};
use engine::Engine;
use mirror::{Mirror, Tee};
use quantus_ui::app::{App, Mode, Settings};
use quantus_ui::backend::{Backend, Stub};
use quantus_ui::key::Key;
use quantus_ui::render::{Frame, Renderer, HEIGHT, WIDTH};

fn main() {
    let mut display = SimulatorDisplay::<Rgb565>::new(Size::new(WIDTH, HEIGHT));
    let args: Vec<String> = std::env::args().collect();
    let backend = if args.iter().any(|arg| arg == "--stub") { Answers::Stub(Stub) } else { Answers::Engine(Engine::default()) };
    let mut app = App::new(backend);
    // The PC has no battery to read; show a full one.
    app.battery = Some(100);
    let mut renderer = Renderer::new();

    let option = |name: &str| args.iter().position(|arg| arg == name).and_then(|i| args.get(i + 1));
    if let (Some(keys), Some(png)) = (option("--keys"), option("--png")) {
        // `\e` for Esc is not needed here: Ctrl+X clears. `\n` is EXE, `>`/`<` move the cursor.
        for c in keys.replace("\\n", "\n").chars() {
            let key = match c {
                '>' => Some(Key::Right),
                '<' => Some(Key::Left),
                c => Key::from_char(c),
            };
            key.into_iter().for_each(|key| app.key(key));
        }
        renderer.render(&Frame::new(&app), &mut display).unwrap();
        let settings = OutputSettingsBuilder::new().build();
        display.to_rgb_output_image(&settings).save_png(png).expect("write the png");
        return;
    }

    let mut mirror = match option("--mirror").map(|port| Mirror::open(port)) {
        Some(Ok(mirror)) => Some(mirror),
        Some(Err(error)) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
        None => None,
    };

    print!("{KEYS}");
    let settings = OutputSettingsBuilder::new().scale(2).build();
    let mut window = Window::new("Quantus", &settings);
    let start = Instant::now();
    loop {
        if mirror.as_ref().is_some_and(Mirror::take_ready) {
            renderer.invalidate();
        }
        let mut target = Tee { display: &mut display, mirror: mirror.as_mut() };
        renderer.render(&Frame::new(&app), &mut target).unwrap();
        window.update(&display);
        for event in window.events() {
            match event {
                SimulatorEvent::Quit => return,
                SimulatorEvent::KeyDown { keycode, keymod, .. } => {
                    if let Some(key) = key(keycode, keymod) {
                        app.key(key);
                    }
                }
                _ => {}
            }
        }
        app.tick(start.elapsed().as_millis() as u64);
        std::thread::sleep(Duration::from_millis(15));
    }
}

/// What answers: the engine, or the stub with `--stub`.
enum Answers {
    Engine(Engine),
    Stub(Stub),
}

impl Backend for Answers {
    fn evaluate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> Result<String, String> {
        match self {
            Answers::Engine(engine) => engine.evaluate(mode, settings, latex),
            Answers::Stub(stub) => stub.evaluate(mode, settings, latex),
        }
    }
}

/// The PC keys for the calculator's, printed at start (the full table is in the README).
const KEYS: &str = "\
Quantus simulator keys
  MODE (mode tiles)    Tab, then 1-5 or arrows and Enter
  Setup (SHIFT+MODE)   F1, then Tab; Up/Down pick a line, Left/Right change it
  SHIFT / ALPHA        F1 or Ctrl+F / F2 or Ctrl+K
  S<=>D                F3 or Ctrl+D
  EXE / DEL / AC       Enter / Backspace / Esc, Delete or Ctrl+X
  fraction ^ index     /  ^  _
  sqrt x^2 pi e        Ctrl+W  Ctrl+Q  Ctrl+P  Ctrl+E
  sin cos tan          Ctrl+S  Ctrl+O  Ctrl+T
  ln log Ans           Ctrl+L  Ctrl+G  Ctrl+A
  history              Up/Down outside a fraction
  Shifted characters follow a German layout (Shift+7 is /, Shift+0 is =).
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
        '-' => '_',
        c => c.to_uppercase().next()?,
    };
    Key::from_char(c)
}
