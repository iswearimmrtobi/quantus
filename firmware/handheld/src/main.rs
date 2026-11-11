//! The handheld's firmware on the ESP32-P4 dev kit with the 3.5" ST7796S capacitive touch display:
//! the UI of `quantus-ui` with the engine (`quantus-core`, or a stub without the `engine` feature),
//! keys from the serial monitor or a USB keyboard, and the display's touch panel. See the README.

#[cfg(feature = "engine")]
mod engine;

use std::io::Read;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys;
use quantus_ui::app::{App, Mode, Settings};
use quantus_ui::backend::{Backend, Catalogue, Group, Network, Reply, Request, Storage, WlanStatus};
use quantus_texts::tr;
use quantus_ui::text::Lang;
#[cfg(feature = "engine")]
use quantus_ui::backend::{Opened, SessionCommand, SessionInfo, Stored, Units, VariableCommand};
use quantus_ui::font::Scale;
use quantus_ui::key::Key;
use quantus_ui::render::{Frame, Renderer, WIDTH};

use quantus_handheld::display::Display;
use quantus_handheld::c6;
use quantus_handheld::store::Store;
use quantus_handheld::wlan::Wlan;
use quantus_handheld::{keyboard, open_display, sd};
use quantus_ui::touch::Touch;

/// Prints the touch controller's raw state four times a second (`QUANTUS_TOUCH_DEBUG=1`).
const DEBUG_TOUCH: bool = option_env!("QUANTUS_TOUCH_DEBUG").is_some();

fn main() {
    sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    let start = Instant::now();

    let peripherals = Peripherals::take().expect("peripherals");
    let (mut display, mut touch) = open_display!(peripherals);
    println!("display ready after {} ms", start.elapsed().as_millis());

    test_pattern(&mut display);
    let shown = Instant::now();

    let mounted = Instant::now();
    let card = sd::SdCard::mount(
        peripherals.sdmmc0,
        peripherals.pins.gpio43,
        peripherals.pins.gpio44,
        peripherals.pins.gpio39,
        peripherals.pins.gpio40,
        peripherals.pins.gpio41,
        peripherals.pins.gpio42,
        peripherals.pins.gpio45,
    );
    let card = match card {
        Ok(card) => {
            println!("SD card mounted at {} in {} ms", sd::MOUNT, mounted.elapsed().as_millis());
            Some(card)
        }
        Err(error) => {
            println!("no SD card: {error}");
            None
        }
    };
    FreeRtos::delay_ms(1500u32.saturating_sub(shown.elapsed().as_millis() as u32));

    // The touch calibration, the wallpaper's name and the settings from flash.
    let nvs = EspDefaultNvsPartition::take().map_err(|error| log::warn!("no NVS partition: {error}")).ok();
    let open = || {
        let store = Store::open(nvs.clone()?);
        store.map_err(|error| log::warn!("no NVS for the settings: {error}")).ok()
    };
    let store = open();
    update_c6(store.as_ref(), &mut display);
    // WLAN through the C6; off until the settings switch it on.
    let wlan = match (EspSystemEventLoop::take(), nvs.clone()) {
        (Ok(sysloop), Some(nvs)) => Some(Wlan::start(peripherals.modem, sysloop, nvs)),
        (Err(error), _) => {
            println!("no WLAN: {error}");
            None
        }
        (_, None) => None,
    };
    let mut board = Board {
        store: open(),
        wlan,
        radio: None,
        #[cfg(feature = "engine")]
        engine: engine::Worker::start(card.is_some().then(|| std::path::PathBuf::from(sd::SESSIONS))),
        card,
    };
    // The wallpaper set before the last restart, if the card still has it.
    let wallpaper = store.as_ref().and_then(Store::load_wallpaper).and_then(|name| match board.picture(&name) {
        Some(picture) => Some((name, picture)),
        None => {
            println!("wallpaper {name:?} not on the SD card");
            None
        }
    });
    let mut app = App::new(board);
    if let Some((name, picture)) = wallpaper {
        app.wallpaper = Some(picture);
        app.wallpaper_name = Some(name);
    }
    // Without a calibration, the calibration screen comes first.
    match store.as_ref().and_then(Store::load) {
        Some(calibration) => app.calibration = calibration,
        None => app.calibrate(),
    }
    let mut kept_wallpaper = app.wallpaper_name.clone();
    let mut renderer = Renderer::new();
    let drawn = Instant::now();
    let bands = renderer.render(&Frame::new(&app), &mut display).expect("render");
    println!("first frame: {bands} bands in {} ms, {} ms after boot", drawn.elapsed().as_millis(), start.elapsed().as_millis());
    println!("Type keys here or on a USB keyboard, or touch the screen; the README lists them.");

    let (keys, received) = mpsc::channel();
    keyboard::start(keys.clone());
    std::thread::Builder::new().stack_size(8 * 1024).spawn(move || read_keys(keys)).expect("key thread");
    let mut last_debug = Instant::now();
    let mut kept_brightness = None;
    loop {
        // Short enough that the touch panel follows a finger.
        let mut changed = match received.recv_timeout(Duration::from_millis(20)) {
            Ok(key) => {
                app.key(key);
                true
            }
            Err(RecvTimeoutError::Timeout) => false,
            Err(RecvTimeoutError::Disconnected) => return,
        };
        if DEBUG_TOUCH && last_debug.elapsed() >= Duration::from_millis(250) {
            last_debug = Instant::now();
            touch.debug();
        }
        if let Some(report) = touch.poll() {
            if let Touch::Press(raw) = report {
                let at = app.calibration.map(raw);
                log::info!("touch: raw x {}, y {} at {}, {}", raw.x, raw.y, at.x, at.y);
            }
            changed |= app.touch(report);
        }
        if let Some(calibration) = app.take_calibration() {
            match store.as_ref().map(|store| store.save(&calibration)) {
                Some(Ok(())) => println!("touch calibration saved: {:?}", calibration.0),
                _ => println!("touch calibration in use, but not saved"),
            }
        }
        if app.wallpaper_name != kept_wallpaper {
            kept_wallpaper = app.wallpaper_name.clone();
            if let Some(Err(error)) = store.as_ref().map(|store| store.save_wallpaper(kept_wallpaper.as_deref())) {
                println!("wallpaper not kept: {error}");
            }
        }
        if kept_brightness != Some(app.settings.brightness) {
            kept_brightness = Some(app.settings.brightness);
            if let Err(error) = display.set_brightness(app.settings.brightness) {
                println!("brightness not set: {error}");
            }
        }
        changed |= app.tick(start.elapsed().as_millis() as u64);
        if changed {
            let drawn = Instant::now();
            let bands = renderer.render(&Frame::new(&app), &mut display).expect("render");
            if bands > 1 {
                log::info!("{bands} bands in {} ms", drawn.elapsed().as_millis());
            }
        }
    }
}

/// The engine on its thread (or the stub), printing what it is asked over serial, and the SD
/// card.
struct Board {
    card: Option<sd::SdCard>,
    /// Where the settings are kept.
    store: Option<Store>,
    wlan: Option<Wlan>,
    /// Whether the radio was last switched on; `None` before the settings came.
    radio: Option<bool>,
    #[cfg(feature = "engine")]
    engine: engine::Worker,
}

impl Backend for Board {
    #[cfg(feature = "engine")]
    fn evaluate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> Result<String, String> {
        self.engine.send(mode, settings, latex, true);
        self.engine.wait()
    }

    #[cfg(not(feature = "engine"))]
    fn evaluate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> Result<String, String> {
        let result = quantus_ui::backend::Stub.evaluate(mode, settings, latex);
        println!("{}: {latex}  =>  {result:?}", mode.name(quantus_ui::text::Lang::En));
        result
    }

    /// The engine calculates on its thread; `poll` brings the answer.
    #[cfg(feature = "engine")]
    fn start(&mut self, mode: Mode, settings: &Settings, latex: &str) -> Option<Result<String, String>> {
        self.engine.send(mode, settings, latex, true);
        None
    }

    #[cfg(feature = "engine")]
    fn recalculate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> Option<Result<String, String>> {
        self.engine.send(mode, settings, latex, false);
        None
    }

    #[cfg(feature = "engine")]
    fn sessions(&mut self) -> Vec<SessionInfo> {
        self.engine.sessions()
    }

    #[cfg(feature = "engine")]
    fn session(&mut self, command: SessionCommand, settings: &Settings) -> Result<Opened, String> {
        self.engine.session(command, settings)
    }

    #[cfg(feature = "engine")]
    fn catalogue(&mut self, catalogue: Catalogue, lang: Lang) -> Vec<Group> {
        self.engine.catalogue(catalogue, lang)
    }

    #[cfg(feature = "engine")]
    fn search_index(&mut self, lang: Lang) -> Vec<quantus_ui::backend::IndexItem> {
        self.engine.search_index(lang)
    }

    #[cfg(feature = "engine")]
    fn unit_suggestions(&mut self, before: &str, recent: &[String], result: Option<&str>) -> Units {
        self.engine.unit_suggestions(before, recent, result)
    }

    #[cfg(feature = "engine")]
    fn variables(&mut self, command: VariableCommand, settings: &Settings) -> Result<Vec<Stored>, String> {
        self.engine.variables(command, settings)
    }

    #[cfg(not(feature = "engine"))]
    fn catalogue(&mut self, catalogue: Catalogue, lang: Lang) -> Vec<Group> {
        quantus_ui::backend::Stub.catalogue(catalogue, lang)
    }

    #[cfg(feature = "engine")]
    fn poll(&mut self) -> Option<Result<String, String>> {
        self.engine.poll()
    }

    #[cfg(feature = "engine")]
    fn solve(&mut self, settings: &Settings, request: &Request) -> Result<Reply, String> {
        self.engine.solve(settings, request);
        self.engine.wait_solve()
    }

    #[cfg(not(feature = "engine"))]
    fn solve(&mut self, settings: &Settings, request: &Request) -> Result<Reply, String> {
        quantus_ui::backend::Stub.solve(settings, request)
    }

    /// The engine works the sheet out on its thread; `poll_solve` brings the answer.
    #[cfg(feature = "engine")]
    fn start_solve(&mut self, settings: &Settings, request: &Request) -> Option<Result<Reply, String>> {
        self.engine.solve(settings, request);
        None
    }

    #[cfg(feature = "engine")]
    fn poll_solve(&mut self) -> Option<Result<Reply, String>> {
        self.engine.poll_solve()
    }

    #[cfg(feature = "engine")]
    fn drop_pending(&mut self) {
        self.engine.drop_pending();
    }

    fn wallpapers(&mut self) -> Vec<String> {
        if self.card.is_some() { sd::wallpapers() } else { Vec::new() }
    }

    fn wallpaper(&mut self, name: &str) -> Option<Vec<u8>> {
        self.card.as_ref().and_then(|_| sd::wallpaper(name))
    }

    fn load_settings(&mut self) -> Option<Settings> {
        self.store.as_ref()?.load_settings()
    }

    fn use_settings(&mut self, settings: &Settings) {
        if let Some(wlan) = &self.wlan {
            if self.radio != Some(settings.wlan) {
                self.radio = Some(settings.wlan);
                wlan.radio(settings.wlan);
            }
            wlan.ntp(settings.time_from_network);
        }
        if let Some(Err(error)) = self.store.as_ref().map(|store| store.save_settings(settings)) {
            println!("settings not kept: {error}");
        }
    }

    fn wlan_status(&mut self) -> WlanStatus {
        self.wlan.as_ref().map_or(WlanStatus::Off, Wlan::status)
    }

    fn scan(&mut self) {
        if let Some(wlan) = &self.wlan {
            wlan.scan();
        }
    }

    fn networks(&mut self) -> Option<Vec<Network>> {
        self.wlan.as_ref().map_or(Some(Vec::new()), Wlan::networks)
    }

    fn connect(&mut self, ssid: &str, password: Option<&str>) {
        if let Some(wlan) = &self.wlan {
            wlan.connect(ssid, password);
        }
    }

    fn saved_networks(&mut self) -> Vec<String> {
        self.wlan.as_ref().map_or(Vec::new(), Wlan::saved)
    }

    fn forget(&mut self, ssid: &str) {
        if let Some(wlan) = &self.wlan {
            wlan.forget(ssid);
        }
    }

    /// The system clock itself, which network time then keeps right.
    fn set_time(&mut self, utc_ms: i64) {
        let time = sys::timeval { tv_sec: utc_ms.div_euclid(1000) as _, tv_usec: (utc_ms.rem_euclid(1000) * 1000) as _ };
        // SAFETY: plain C call with a valid timeval and no time zone.
        if unsafe { sys::settimeofday(&time, core::ptr::null()) } != 0 {
            println!("the clock could not be set");
        }
    }

    fn storage(&mut self) -> Option<Storage> {
        self.card.as_ref()?;
        let asked = Instant::now();
        let storage = sd::storage();
        println!("SD card: {storage:?} in {} ms", asked.elapsed().as_millis());
        storage
    }
}

/// Sends the C6 the image in the `c6_fw` partition if it runs another version and the image was
/// not tried before, saying so on the screen; restarts after a success (src/c6.rs).
fn update_c6(store: Option<&Store>, display: &mut Display) {
    // Before the UI starts: the language of the saved settings.
    let lang = store.and_then(Store::load_settings).map_or(Lang::De, |settings| settings.language);
    let Some(image) = c6::Image::find() else { return };
    let tried = store.and_then(Store::c6_tried);
    if tried.as_deref() == Some(image.id().as_str()) {
        return;
    }
    let Some(running) = c6::version() else {
        println!("C6: no link, its firmware not checked");
        return;
    };
    if running == image.version {
        println!("C6: firmware {running} is current");
        if let Some(store) = store {
            let _ = store.set_c6_tried(&image.id());
        }
        return;
    }
    println!("C6: firmware {running}, sending {} ({} bytes)", image.version, image.size);
    if let Some(Err(error)) = store.map(|store| store.set_c6_tried(&image.id())) {
        // Without a note it would be tried at every boot: better not at all.
        println!("C6: not updated, the attempt could not be noted: {error}");
        return;
    }
    let message = |display: &mut Display, text: &str| {
        let line = Rectangle::new(Point::new(0, 280), Size::new(WIDTH, 40));
        let _ = line.into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK)).draw(display);
        Scale::Small.draw(text, Point::new(10, 306), Rgb565::WHITE, display);
    };
    let started = Instant::now();
    let mut shown = u8::MAX;
    let sent = c6::send(&image, &running, |percent| {
        if percent / 10 != shown / 10 {
            shown = percent;
            message(display, &tr!(lang, "hui.c6.updating"));
        }
    });
    match sent {
        Ok(()) => {
            println!("C6: sent in {} s, restarting", started.elapsed().as_secs());
            message(display, &tr!(lang, "hui.c6.updated"));
            c6::restart();
        }
        Err(error) => {
            println!("C6: update failed: {error}; it keeps firmware {running}");
            message(display, &tr!(lang, "hui.c6.not_updated"));
            FreeRtos::delay_ms(1500);
        }
    }
}

/// Turns the bytes typed into the serial monitor into keys: characters by `Key::from_char`,
/// the arrows' escape sequences, Delete (`ESC [ 3 ~`) as AC and F5 (`ESC [ 1 5 ~`) as OK, as on
/// a USB keyboard.
fn read_keys(keys: Sender<Key>) {
    let mut stdin = std::io::stdin();
    let mut pending: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        if !matches!(stdin.read(&mut byte), Ok(1)) {
            FreeRtos::delay_ms(10);
            continue;
        }
        pending.push(byte[0]);
        let key = match pending.as_slice() {
            [0x1b] | [0x1b, b'['] | [0x1b, b'[', b'3'] | [0x1b, b'[', b'1'] | [0x1b, b'[', b'1', b'5'] => continue,
            [0x1b, b'[', b'A'] => Some(Key::Up),
            [0x1b, b'[', b'B'] => Some(Key::Down),
            [0x1b, b'[', b'C'] => Some(Key::Right),
            [0x1b, b'[', b'D'] => Some(Key::Left),
            [0x1b, b'[', b'3', b'~'] => Some(Key::Ac),
            [0x1b, b'[', b'1', b'5', b'~'] => Some(Key::Ok),
            [0x1b, ..] => None,
            bytes => match std::str::from_utf8(bytes) {
                Ok(text) => text.chars().next().and_then(Key::from_char),
                // The start of a character of several bytes.
                Err(error) if error.error_len().is_none() => continue,
                Err(_) => None,
            },
        };
        pending.clear();
        if let Some(key) = key {
            if keys.send(key).is_err() {
                return;
            }
        }
    }
}

/// Red, green, blue and white bars with their names: colours and orientation at a glance.
fn test_pattern(display: &mut Display) {
    let bars = [(Rgb565::RED, "red"), (Rgb565::GREEN, "green"), (Rgb565::BLUE, "blue"), (Rgb565::WHITE, "white")];
    let width = WIDTH / bars.len() as u32;
    for (index, (color, name)) in bars.into_iter().enumerate() {
        let x = (index as u32 * width) as i32;
        let bar = Rectangle::new(Point::new(x, 0), Size::new(width, 280));
        let _ = bar.into_styled(PrimitiveStyle::with_fill(color)).draw(display);
        let label = Rectangle::new(Point::new(x, 280), Size::new(width, 40));
        let _ = label.into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK)).draw(display);
        Scale::Small.draw(name, Point::new(x + 10, 306), Rgb565::WHITE, display);
    }
    // The top left corner, to see where (0, 0) is.
    let corner = Rectangle::new(Point::zero(), Size::new(20, 20));
    let _ = corner.into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK)).draw(display);
}
