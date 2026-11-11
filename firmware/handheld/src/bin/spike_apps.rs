//! Throwaway (firmware/docs/apps.md, step 0): Rhai on the board. Flash with
//! `cargo run --release --no-default-features --features spike-apps --bin spike_apps`.
//! Runs the spike's checks at start, then takes a script per line from the serial monitor.

use std::io::Read;

use esp_idf_svc::sys;
use rhai::Engine;

fn main() {
    sys::link_patches();
    println!("spike: Rhai 1.27");
    let engine = Engine::new();
    let sum: i32 = engine.eval("let s = 0; for i in 0..10 { s += i * i; } s").expect("script");
    println!("spike: sum of squares = {sum}");
    memory("start");

    let mut line = String::new();
    let mut stdin = std::io::stdin();
    let mut byte = [0u8; 1];
    loop {
        if !matches!(stdin.read(&mut byte), Ok(1)) {
            esp_idf_svc::hal::delay::FreeRtos::delay_ms(20);
            continue;
        }
        match byte[0] {
            b'\n' | b'\r' => {
                if !line.trim().is_empty() {
                    let started = std::time::Instant::now();
                    let result = engine.eval::<rhai::Dynamic>(&line);
                    println!("spike: {line}  =>  {result:?}  ({} us)", started.elapsed().as_micros());
                }
                line.clear();
            }
            b => line.push(b as char),
        }
    }
}

/// The task's stack high water mark and the heap, as the engine's `report_memory` prints them.
fn memory(what: &str) {
    // SAFETY: plain reads of allocator counters; a null handle is the calling task.
    let (unused, free, minimum, internal, internal_minimum) = unsafe {
        (
            sys::uxTaskGetStackHighWaterMark(std::ptr::null_mut()),
            sys::esp_get_free_heap_size(),
            sys::esp_get_minimum_free_heap_size(),
            sys::heap_caps_get_free_size(sys::MALLOC_CAP_INTERNAL),
            sys::heap_caps_get_minimum_free_size(sys::MALLOC_CAP_INTERNAL),
        )
    };
    println!("spike: [{what}] stack unused {unused} B; heap {free} B free, {minimum} B lowest; internal {internal} B free, {internal_minimum} B lowest");
}
