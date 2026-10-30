//! Feasibility spike: runs a fixed set of inputs through the engine on a classic ESP32 (no PSRAM)
//! and prints each result with its time, plus heap and stack figures, over serial. No display, no
//! keypad.

use std::time::Instant;

use esp_idf_svc::sys;
use quantus_core::api::{self, AnalysisRequest, CalculationRequest};
use quantus_core::session::SessionStore;

/// The parser and the algebra recurse; the desktop never notices, a FreeRTOS task does.
const ENGINE_STACK: usize = 32 * 1024;

/// One or two inputs per category of the generated test set (`test/cases`).
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

/// Curve sketching, the heaviest thing the engine does.
const FUNCTIONS: &[&str] = &[r"f(x) = x^{3} - 3x", r"f(x) = \frac{x^{2} - 1}{x - 2}"];

fn main() {
    sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let engine = std::thread::Builder::new()
        .stack_size(ENGINE_STACK)
        .spawn(run)
        .expect("engine thread");
    if engine.join().is_err() {
        println!("engine thread panicked");
    }
    println!("done");
}

fn run() {
    report_memory();
    let mut store = SessionStore::new();
    for &latex in INPUTS {
        let request = CalculationRequest { latex: latex.into(), ..Default::default() };
        let started = Instant::now();
        let outcome = api::evaluate(&mut store, &request);
        let elapsed = started.elapsed();
        match outcome {
            Ok(response) => println!("{:>8.1?}  {latex}  =>  {}", elapsed, response.result),
            Err(error) => println!("{:>8.1?}  {latex}  !!  {error}", elapsed),
        }
        report_memory();
    }
    for &latex in FUNCTIONS {
        let request = AnalysisRequest { latex: latex.into(), variable: None, precision: None };
        let started = Instant::now();
        let outcome = api::analyze(&store.settings, &request);
        let elapsed = started.elapsed();
        match outcome {
            Ok(response) => println!(
                "{:>8.1?}  {latex}  =>  {} sections, {} points, {} asymptotes",
                elapsed,
                response.sections.len(),
                response.points.len(),
                response.asymptotes.len()
            ),
            Err(error) => println!("{:>8.1?}  {latex}  !!  {error}", elapsed),
        }
        report_memory();
    }
}

/// After each input: the most stack this thread has used, and the heap. On a chip without PSRAM
/// the largest free block matters as much as the total, since big numbers want contiguous memory.
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
