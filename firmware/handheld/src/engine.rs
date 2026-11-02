//! The engine on a thread of its own, so the screen, the keys and the touch panel stay live while
//! it calculates: an integral takes seconds. One calculation at a time; one dropped with AC runs
//! to its end, and the next waits for it.

use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Instant;

use esp_idf_svc::sys;
use quantus_engine::Engine;
use quantus_ui::app::{Mode, Settings};
use quantus_ui::backend::Backend;
use quantus_ui::text::Lang;

/// The parser and the algebra recurse; the desktop never notices, a FreeRTOS task does. The
/// WROOM needed at most 14 KB for the inputs of the spike's `:bench`.
const ENGINE_STACK: usize = 64 * 1024;

type Answer = Result<String, String>;

struct Job {
    generation: u32,
    mode: Mode,
    settings: Settings,
    latex: String,
}

pub struct Worker {
    jobs: Sender<Job>,
    answers: Receiver<(u32, Answer)>,
    /// Counts the calculations started and dropped; an answer of an older one is thrown away.
    generation: u32,
}

impl Worker {
    pub fn start() -> Self {
        let (jobs, received) = mpsc::channel::<Job>();
        let (answer, answers) = mpsc::channel();
        std::thread::Builder::new()
            .name("engine".into())
            .stack_size(ENGINE_STACK)
            .spawn(move || {
                let mut engine = Engine::default();
                for job in received {
                    let started = Instant::now();
                    let result = engine.evaluate(job.mode, &job.settings, &job.latex);
                    println!(
                        "{}: {}  =>  {result:?}  ({} ms)",
                        job.mode.name(Lang::En),
                        job.latex,
                        started.elapsed().as_millis()
                    );
                    report_memory();
                    if answer.send((job.generation, result)).is_err() {
                        return;
                    }
                }
            })
            .expect("engine thread");
        Worker { jobs, answers, generation: 0 }
    }

    /// Sends a calculation to the engine; its answer comes from [`poll`](Worker::poll).
    pub fn send(&mut self, mode: Mode, settings: &Settings, latex: &str) {
        self.generation = self.generation.wrapping_add(1);
        let job = Job { generation: self.generation, mode, settings: *settings, latex: latex.to_string() };
        if self.jobs.send(job).is_err() {
            println!("the engine thread has stopped");
        }
    }

    /// The answer to the last calculation sent, once it is there.
    pub fn poll(&mut self) -> Option<Answer> {
        while let Ok((generation, answer)) = self.answers.try_recv() {
            if generation == self.generation {
                return Some(answer);
            }
        }
        None
    }

    /// The answer to the last calculation sent, waiting for it.
    pub fn wait(&mut self) -> Answer {
        loop {
            match self.answers.recv() {
                Ok((generation, answer)) if generation == self.generation => return answer,
                Ok(_) => {}
                Err(_) => return Err("the engine thread has stopped".to_string()),
            }
        }
    }

    /// Forgets the calculation that runs: its answer is thrown away when it comes.
    pub fn drop_pending(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }
}

/// The most stack the engine thread has used, and the heap: all of it (internal RAM and PSRAM),
/// then the internal RAM alone. The largest free block matters as much as the total, since big
/// numbers want contiguous memory.
fn report_memory() {
    // SAFETY: a null handle means the calling task; the rest are plain reads of allocator counters.
    let (unused, free, minimum, largest, internal, internal_minimum, internal_largest) = unsafe {
        (
            sys::uxTaskGetStackHighWaterMark(std::ptr::null_mut()),
            sys::esp_get_free_heap_size(),
            sys::esp_get_minimum_free_heap_size(),
            sys::heap_caps_get_largest_free_block(sys::MALLOC_CAP_8BIT),
            sys::heap_caps_get_free_size(sys::MALLOC_CAP_INTERNAL),
            sys::heap_caps_get_minimum_free_size(sys::MALLOC_CAP_INTERNAL),
            sys::heap_caps_get_largest_free_block(sys::MALLOC_CAP_INTERNAL),
        )
    };
    // ESP-IDF counts stack in bytes, not words.
    let stack = ENGINE_STACK as u32 - unused;
    println!("    stack {stack} of {ENGINE_STACK} B; heap {free} B free, {minimum} B lowest, {largest} B largest block");
    println!("    internal RAM {internal} B free, {internal_minimum} B lowest, {internal_largest} B largest block");
}
