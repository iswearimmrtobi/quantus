//! The engine on a thread of its own, so the screen, the keys and the touch panel stay live while
//! it calculates: an integral takes seconds. One calculation at a time; one dropped with AC runs
//! to its end, and the next waits for it. The sessions live on this thread too, in the engine;
//! the session menu's requests wait behind a running calculation.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Instant;

use esp_idf_svc::sys;
use quantus_engine::Engine;
use quantus_ui::app::{Mode, Settings};
use quantus_ui::backend::{
    Backend, Catalogue, Group, IndexItem, Opened, Reply, Request, SessionCommand, SessionInfo, Stored, Units, VariableCommand,
};
use quantus_texts::tr;
use quantus_ui::text::Lang;

/// The parser and the algebra recurse; the desktop never notices, a FreeRTOS task does. The
/// WROOM needed at most 14 KB for the inputs of the spike's `:bench`.
const ENGINE_STACK: usize = 64 * 1024;

type Answer = Result<String, String>;

/// What the engine thread hands back: a calculation's LaTeX, or a sheet's answer.
enum Done {
    Latex(Answer),
    Sheet(Result<Reply, String>),
}

enum Job {
    Calculate { generation: u32, mode: Mode, settings: Settings, latex: String, record: bool },
    /// A sheet (LGS, Wertetabelle, Statistik, Verteilungen), answered like a calculation.
    Solve { generation: u32, settings: Settings, request: Request },
    /// The sessions, answered on a channel of their own.
    Sessions(Sender<Vec<SessionInfo>>),
    Session(SessionCommand, Settings, Sender<Result<Opened, String>>),
    /// CONST and CONV, answered like the sessions.
    Catalogue(Catalogue, Lang, Sender<Vec<Group>>),
    /// What CATALOG's search looks through, answered like the sessions.
    SearchIndex(Lang, Sender<Vec<IndexItem>>),
    /// STO and VARS, answered like the sessions.
    Variables(VariableCommand, Settings, Sender<Result<Vec<Stored>, String>>),
    /// The units for the soft keys: the input before the cursor, the recent units, a result.
    Units(String, Vec<String>, Option<String>, Sender<Units>),
}

pub struct Worker {
    jobs: Sender<Job>,
    answers: Receiver<(u32, Done)>,
    /// Counts the calculations started and dropped; an answer of an older one is thrown away.
    generation: u32,
    /// The language of the last settings sent, for saying the engine thread has stopped.
    lang: Lang,
}

impl Worker {
    /// The engine thread, with the session files in `sessions` (none without an SD card).
    pub fn start(sessions: Option<PathBuf>) -> Self {
        let (jobs, received) = mpsc::channel::<Job>();
        let (answer, answers) = mpsc::channel();
        std::thread::Builder::new()
            .name("engine".into())
            .stack_size(ENGINE_STACK)
            .spawn(move || {
                let loaded = Instant::now();
                // Reading a session works its variables out again: here, on the big stack.
                let mut engine = Engine::new(sessions);
                println!("sessions read in {} ms, open: {}", loaded.elapsed().as_millis(), engine.current());
                for job in received {
                    match job {
                        Job::Calculate { generation, mode, settings, latex, record } => {
                            let started = Instant::now();
                            let result = match record {
                                true => engine.evaluate(mode, &settings, &latex),
                                false => engine.recalculate(mode, &settings, &latex).expect("the engine answers at once"),
                            };
                            println!("{}: {latex}  =>  {result:?}  ({} ms)", mode.name(Lang::En), started.elapsed().as_millis());
                            report_memory();
                            if answer.send((generation, Done::Latex(result))).is_err() {
                                return;
                            }
                        }
                        Job::Solve { generation, settings, request } => {
                            let started = Instant::now();
                            let reply = engine.solve(&settings, &request);
                            let outcome = if reply.is_ok() { "ok" } else { "error" };
                            println!("sheet: {request:?}  =>  {outcome}  ({} ms)", started.elapsed().as_millis());
                            report_memory();
                            if answer.send((generation, Done::Sheet(reply))).is_err() {
                                return;
                            }
                        }
                        Job::Sessions(reply) => {
                            let _ = reply.send(engine.sessions());
                        }
                        Job::Session(command, settings, reply) => {
                            println!("session: {command:?}");
                            let _ = reply.send(engine.session(command, &settings));
                        }
                        Job::Catalogue(catalogue, lang, reply) => {
                            let _ = reply.send(engine.catalogue(catalogue, lang));
                        }
                        Job::SearchIndex(lang, reply) => {
                            let _ = reply.send(engine.search_index(lang));
                        }
                        Job::Units(before, recent, result, reply) => {
                            let _ = reply.send(engine.unit_suggestions(&before, &recent, result.as_deref()));
                        }
                        Job::Variables(command, settings, reply) => {
                            println!("variables: {command:?}");
                            let _ = reply.send(engine.variables(command, &settings));
                        }
                    }
                }
            })
            .expect("engine thread");
        Worker { jobs, answers, generation: 0, lang: Lang::De }
    }

    /// Sends a calculation to the engine; its answer comes from [`poll`](Worker::poll).
    /// `record: false` leaves the history alone (S⇔D).
    pub fn send(&mut self, mode: Mode, settings: &Settings, latex: &str, record: bool) {
        self.lang = settings.language;
        self.generation = self.generation.wrapping_add(1);
        let job = Job::Calculate { generation: self.generation, mode, settings: *settings, latex: latex.to_string(), record };
        if self.jobs.send(job).is_err() {
            println!("the engine thread has stopped");
        }
    }

    /// Sends a sheet to the engine; its answer comes from [`poll_solve`](Worker::poll_solve).
    pub fn solve(&mut self, settings: &Settings, request: &Request) {
        self.lang = settings.language;
        self.generation = self.generation.wrapping_add(1);
        let job = Job::Solve { generation: self.generation, settings: *settings, request: request.clone() };
        if self.jobs.send(job).is_err() {
            println!("the engine thread has stopped");
        }
    }

    /// The sessions, waiting for a running calculation to end first.
    pub fn sessions(&mut self) -> Vec<SessionInfo> {
        let (reply, answer) = mpsc::channel();
        if self.jobs.send(Job::Sessions(reply)).is_err() {
            return Vec::new();
        }
        answer.recv().unwrap_or_default()
    }

    /// Does `command` to the sessions, waiting like [`sessions`](Worker::sessions).
    pub fn session(&mut self, command: SessionCommand, settings: &Settings) -> Result<Opened, String> {
        let (reply, answer) = mpsc::channel();
        self.lang = settings.language;
        let stopped = || tr!(self.lang, "hui.engine_stopped");
        self.jobs.send(Job::Session(command, *settings, reply)).map_err(|_| stopped())?;
        answer.recv().map_err(|_| stopped())?
    }

    /// The constants or the units, waiting like [`sessions`](Worker::sessions).
    pub fn catalogue(&mut self, catalogue: Catalogue, lang: Lang) -> Vec<Group> {
        let (reply, answer) = mpsc::channel();
        if self.jobs.send(Job::Catalogue(catalogue, lang, reply)).is_err() {
            return Vec::new();
        }
        answer.recv().unwrap_or_default()
    }

    /// What CATALOG's search looks through, waiting like [`sessions`](Worker::sessions).
    pub fn search_index(&mut self, lang: Lang) -> Vec<IndexItem> {
        let (reply, answer) = mpsc::channel();
        if self.jobs.send(Job::SearchIndex(lang, reply)).is_err() {
            return Vec::new();
        }
        answer.recv().unwrap_or_default()
    }

    /// The units for the soft keys, waiting like [`sessions`](Worker::sessions).
    pub fn unit_suggestions(&mut self, before: &str, recent: &[String], result: Option<&str>) -> Units {
        let (reply, answer) = mpsc::channel();
        let job = Job::Units(before.to_string(), recent.to_vec(), result.map(str::to_string), reply);
        if self.jobs.send(job).is_err() {
            return Units::default();
        }
        answer.recv().unwrap_or_default()
    }

    /// Does `command` to the variables, waiting like [`sessions`](Worker::sessions).
    pub fn variables(&mut self, command: VariableCommand, settings: &Settings) -> Result<Vec<Stored>, String> {
        let (reply, answer) = mpsc::channel();
        self.lang = settings.language;
        let stopped = || tr!(self.lang, "hui.engine_stopped");
        self.jobs.send(Job::Variables(command, *settings, reply)).map_err(|_| stopped())?;
        answer.recv().map_err(|_| stopped())?
    }

    /// The answer to the last calculation sent, once it is there.
    pub fn poll(&mut self) -> Option<Answer> {
        while let Ok((generation, done)) = self.answers.try_recv() {
            if let (true, Done::Latex(answer)) = (generation == self.generation, done) {
                return Some(answer);
            }
        }
        None
    }

    /// The answer to the last sheet sent, once it is there.
    pub fn poll_solve(&mut self) -> Option<Result<Reply, String>> {
        while let Ok((generation, done)) = self.answers.try_recv() {
            if let (true, Done::Sheet(reply)) = (generation == self.generation, done) {
                return Some(reply);
            }
        }
        None
    }

    /// The answer to the last calculation sent, waiting for it.
    pub fn wait(&mut self) -> Answer {
        loop {
            match self.answers.recv() {
                Ok((generation, Done::Latex(answer))) if generation == self.generation => return answer,
                Ok(_) => {}
                Err(_) => return Err(tr!(self.lang, "hui.engine_stopped")),
            }
        }
    }

    /// The answer to the last sheet sent, waiting for it.
    pub fn wait_solve(&mut self) -> Result<Reply, String> {
        loop {
            match self.answers.recv() {
                Ok((generation, Done::Sheet(reply))) if generation == self.generation => return reply,
                Ok(_) => {}
                Err(_) => return Err(tr!(self.lang, "hui.engine_stopped")),
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
