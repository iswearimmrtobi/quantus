//! Feasibility spike: the engine on a classic ESP32 (no PSRAM), driven over serial. Each line
//! typed into the monitor is evaluated like an input in the app's current mode, and the result is
//! printed with its time and the heap and stack it left. No display, no keypad.

use std::io::{self, Read, Write};
use std::time::{Duration, Instant};

use esp_idf_svc::sys;
use quantus_core::api::{self, AnalysisRequest, CalculationRequest, LinearSystemRequest};
use quantus_core::session::SessionStore;
use quantus_core::settings::{Mode, Settings};

/// The parser and the algebra recurse; the desktop never notices, a FreeRTOS task does.
const ENGINE_STACK: usize = 32 * 1024;

/// What `:bench` runs: one or two inputs per category of the generated test set (`test/cases`).
const INPUTS: &[&str] = &[
    r"20{,}5 + (-65{,}6)",
    r"(-16) \cdot (-12) \times 19 \cdot 9",
    r"\frac{1}{\frac{1}{173\,\mathrm{\Omega}} + \frac{1}{787\,\mathrm{M\Omega}}}",
    r"E_k = \frac{1}{2} \cdot 68{,}7\,\mathrm{g} \cdot \left(16\,\frac{\mathrm{km}}{\mathrm{h}}\right)^{2}",
    r"\frac{x}{10} + 10 = -2",
    r"2x^{2} - 59x + 86 = -29x - 22",
    r"\left.\frac{d}{dz} \frac{z^{2} - 8}{-4z^{3} - 3z^{2} - 6z}\right|_{z=-2{,}5}",
    r"\int_0^{4\,\mathrm{s}} 18\,\frac{\mathrm{m}}{\mathrm{s}^2}\,dt",
    r"\int_{0}^{1} \ln x\,dx",
    r"\int_{6{,}5}^{37} \frac{1}{x}\,dx",
    r"e^{4i\pi/2}",
];

/// Curve sketching for `:bench`, the heaviest thing the engine does.
const FUNCTIONS: &[&str] = &[r"f(x) = x^{3} - 3x", r"f(x) = \frac{x^{2} - 1}{x - 2}"];

/// The app's modes that work as text. The electronics mode is a circuit editor.
const MODES: &[(&str, Mode)] = &[
    ("algebra", Mode::Algebra),
    ("physics", Mode::Physics),
    ("imaginary", Mode::Imaginary),
    ("analysis", Mode::Analysis),
    ("linear", Mode::Linear),
];

const HELP: &str = r"Type an input as LaTeX, e.g. \frac{3}{4} + 2\,\mathrm{m}.
Variables need a subscript: r_1 = 5\,\mathrm{cm}, then 2 \cdot r_1.
  :mode [name]           show or switch the mode: algebra, physics, imaginary,
                         analysis (each line a curve sketch) or linear
                         (2x + y = 5; x - y = 1)
  :sketch <f(x) = ...>   curve sketching in any mode
  :bench                 run the fixed inputs in physics
  :mem                   heap and stack
  :help                  this text
Variables stay defined until the board is reset.";

fn main() {
    sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let engine = std::thread::Builder::new()
        .stack_size(ENGINE_STACK)
        .spawn(prompt)
        .expect("engine thread");
    if engine.join().is_err() {
        println!("engine thread panicked");
    }
}

fn prompt() {
    println!("\nQuantus on the ESP32. :help lists the commands.");
    report_memory();
    let mut store = SessionStore::new();
    loop {
        print!("{}> ", mode_name(store.settings.mode));
        let line = read_line();
        let line = line.trim();
        if let Some(function) = line.strip_prefix(":sketch") {
            sketch(&store, function.trim());
        } else if let Some(name) = line.strip_prefix(":mode") {
            switch_mode(&mut store, name.trim());
        } else {
            match line {
                "" => continue,
                ":help" => println!("{HELP}"),
                ":mem" => report_memory(),
                ":bench" => bench(&mut store),
                _ if line.starts_with(':') => println!("unknown command, :help lists them"),
                _ => match store.settings.mode {
                    Mode::Analysis => sketch(&store, line),
                    Mode::Linear => solve_linear(&store, line),
                    _ => evaluate(&mut store, line),
                },
            }
        }
    }
}

fn mode_name(mode: Mode) -> &'static str {
    MODES.iter().find(|(_, m)| *m == mode).map_or("?", |(name, _)| name)
}

fn switch_mode(store: &mut SessionStore, name: &str) {
    if !name.is_empty() {
        let Some(&(_, mode)) = MODES.iter().find(|(n, _)| *n == name) else {
            let names: Vec<_> = MODES.iter().map(|(n, _)| *n).collect();
            println!("unknown mode, one of: {}", names.join(", "));
            return;
        };
        set_mode(store, mode);
    }
    println!("mode: {}", mode_name(store.settings.mode));
}

fn set_mode(store: &mut SessionStore, mode: Mode) {
    let settings = Settings { mode, ..store.settings.clone() };
    if let Err(error) = api::set_settings(store, settings) {
        println!("!!  {error}");
    }
}

fn evaluate(store: &mut SessionStore, latex: &str) {
    let request = CalculationRequest { latex: latex.into(), ..Default::default() };
    let started = Instant::now();
    let outcome = api::evaluate(store, &request);
    let elapsed = started.elapsed();
    match outcome {
        Ok(response) => println!("{:>8.1?}  {latex}  =>  {}", elapsed, response.result),
        Err(error) => println!("{:>8.1?}  {latex}  !!  {error}", elapsed),
    }
    report_memory();
}

fn sketch(store: &SessionStore, latex: &str) {
    let request = AnalysisRequest { latex: latex.into(), variable: None, precision: None };
    let started = Instant::now();
    let outcome = api::analyze(&store.settings, &request);
    let elapsed = started.elapsed();
    match outcome {
        Ok(response) => {
            println!("{:>8.1?}  {latex}", elapsed);
            for section in &response.sections {
                let result = section.result.as_deref().or(section.message.as_deref()).unwrap_or("");
                let status = if section.status == "ok" { String::new() } else { format!(" ({})", section.status) };
                println!("          {}{status}: {result}", section.title);
            }
            for point in &response.points {
                println!("          {}: {}", point.kind, point.latex);
            }
            for asymptote in &response.asymptotes {
                println!("          {} asymptote: {}", asymptote.kind, asymptote.latex);
            }
        }
        Err(error) => println!("{:>8.1?}  {latex}  !!  {error}", elapsed),
    }
    report_memory();
}

fn solve_linear(store: &SessionStore, latex: &str) {
    let request = LinearSystemRequest::Equations(latex.into());
    let started = Instant::now();
    let outcome = api::linear_system(&store.settings, &request);
    let elapsed = started.elapsed();
    match outcome {
        Ok(response) => println!("{:>8.1?}  {latex}  =>  {}", elapsed, response.result),
        Err(error) => println!("{:>8.1?}  {latex}  !!  {error}", elapsed),
    }
    report_memory();
}

/// The fixed inputs are written for physics (units); the mode is put back afterwards.
fn bench(store: &mut SessionStore) {
    let mode = store.settings.mode;
    set_mode(store, Mode::Physics);
    for &latex in INPUTS {
        evaluate(store, latex);
    }
    for &latex in FUNCTIONS {
        sketch(store, latex);
    }
    set_mode(store, mode);
}

/// One line from the serial console, echoed as it is typed (the monitor does not echo).
/// The console's stdin does not block, so an empty read means "nothing yet".
fn read_line() -> String {
    let _ = io::stdout().flush();
    let mut line = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        match io::stdin().read(&mut byte) {
            Ok(1) => {}
            _ => {
                std::thread::sleep(Duration::from_millis(10));
                continue;
            }
        }
        match byte[0] {
            b'\r' | b'\n' => {
                println!();
                return String::from_utf8_lossy(&line).into_owned();
            }
            // Backspace and delete
            0x08 | 0x7f => {
                if line.pop().is_some() {
                    // Drop the rest of a multi-byte character too.
                    while line.last().is_some_and(|b| b & 0xc0 == 0x80) {
                        line.pop();
                    }
                    print!("\x08 \x08");
                }
            }
            b => {
                line.push(b);
                let _ = io::stdout().write_all(&[b]);
            }
        }
        let _ = io::stdout().flush();
    }
}

/// The most stack the engine thread has used, and the heap. On a chip without PSRAM the largest
/// free block matters as much as the total, since big numbers want contiguous memory.
fn report_memory() {
    // SAFETY: a null handle means the calling task; the rest are plain reads of allocator counters.
    let (unused, free, minimum, largest) = unsafe {
        (
            sys::uxTaskGetStackHighWaterMark(std::ptr::null_mut()),
            sys::esp_get_free_heap_size(),
            sys::esp_get_minimum_free_heap_size(),
            sys::heap_caps_get_largest_free_block(sys::MALLOC_CAP_8BIT),
        )
    };
    // ESP-IDF counts stack in bytes, not words.
    let stack = ENGINE_STACK as u32 - unused;
    println!("          stack {stack} of {ENGINE_STACK} B; heap {free} B free, {minimum} B lowest, {largest} B largest block");
}
