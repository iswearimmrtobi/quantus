//! Testing only: the UI tour (`quantus_ui::tour`) played on the board, with a wallpaper
//! everywhere, to see the real latency. Each key prints how long it took until the screen was
//! sent: laying out the frame, drawing the bands and sending them to the display. The stub
//! answers. The wallpaper sits in flash as raw pixels (see `build.rs`); the tour starts again
//! when it is done, after a summary.

#[path = "../display.rs"]
#[macro_use]
mod display;

use std::time::{Duration, Instant};

use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::sys;
use quantus_ui::app::{App, Mode, Settings};
use quantus_ui::backend::{Backend, Stub};
use quantus_ui::render::{Frame, Renderer};
use quantus_ui::tour::tour;
use quantus_ui::wallpaper::Wallpaper;

static FULL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tour_full.raw"));
static DIM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tour_dim.raw"));
const NAME: &str = include_str!(concat!(env!("OUT_DIR"), "/tour_name.txt"));

/// The stub, and an SD card with the one picture from flash.
struct Board;

impl Backend for Board {
    fn evaluate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> Result<String, String> {
        Stub.evaluate(mode, settings, latex)
    }

    fn wallpapers(&mut self) -> Vec<String> {
        vec![NAME.to_string()]
    }

    fn picture(&mut self, name: &str) -> Option<Wallpaper> {
        (name == NAME).then(|| Wallpaper::from_raw(FULL, DIM)).flatten()
    }
}

fn main() {
    sys::link_patches();
    let peripherals = Peripherals::take().expect("peripherals");
    let (mut display, _touch) = open_display!(peripherals);
    println!("tour: SPI at {} kHz, wallpaper {NAME}", display.actual_khz());
    let start = Instant::now();

    loop {
        let mut app = App::new(Board);
        let mut renderer = Renderer::new();
        renderer.render(&Frame::new(&app), &mut display).expect("render");
        // (ms, bands) per key.
        let mut times: Vec<(u32, usize)> = Vec::new();
        for (index, step) in tour().iter().enumerate() {
            // Wait like a person, blinking the caret meanwhile.
            let pressed = Instant::now() + Duration::from_millis(step.pause_ms.into());
            while Instant::now() < pressed {
                FreeRtos::delay_ms(20);
                if app.tick(start.elapsed().as_millis() as u64) {
                    renderer.render(&Frame::new(&app), &mut display).expect("render");
                }
            }
            if let Some(note) = step.note {
                println!("tour: -- {note}");
            }
            let key = Instant::now();
            app.key(step.key);
            let frame = Frame::new(&app);
            let laid_out = key.elapsed();
            let bands = renderer.render(&frame, &mut display).expect("render");
            let total = key.elapsed();
            println!(
                "tour: {index:3} {:<12} {:4} ms ({} ms layout), {bands} bands",
                format!("{:?}", step.key),
                total.as_millis(),
                laid_out.as_millis()
            );
            times.push((total.as_millis() as u32, bands));
        }
        summary(&times);
        FreeRtos::delay_ms(5000);
    }
}

fn summary(times: &[(u32, usize)]) {
    let mut sorted: Vec<u32> = times.iter().map(|&(ms, _)| ms).collect();
    sorted.sort_unstable();
    let percentile = |p: usize| sorted[(sorted.len() - 1) * p / 100];
    let full: Vec<u32> = times.iter().filter(|&&(_, bands)| bands >= 9).map(|&(ms, _)| ms).collect();
    println!(
        "tour: summary of {} keys: median {} ms, 90% under {} ms, slowest {} ms",
        sorted.len(),
        percentile(50),
        percentile(90),
        sorted[sorted.len() - 1]
    );
    if !full.is_empty() {
        println!(
            "tour: {} full redraws (9+ bands): {} ms on average",
            full.len(),
            full.iter().sum::<u32>() / full.len() as u32
        );
    }
}
