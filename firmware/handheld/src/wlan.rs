//! WLAN through the ESP32-C6 (ESP-Hosted, firmware/docs/wlan.md): a thread of its own that owns
//! the Wi-Fi driver and network time, driven by [`Command`]s. What the UI asks for is answered
//! from [`State`], so a scan or a connection never holds up the screen.
//!
//! The networks connected to are kept with their passwords in NVS (namespace `wlan`), at most
//! [`SAVED`]. While WLAN is on and not connected, the strongest saved network in reach is joined,
//! and tried again every [`RETRY`]; with none saved, nothing is scanned unless asked.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::modem::Modem;
use esp_idf_svc::nvs::{EspDefaultNvs, EspDefaultNvsPartition};
use esp_idf_svc::sntp::{EspSntp, SyncStatus};
use esp_idf_svc::sys::EspError;
use esp_idf_svc::wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi};
use quantus_ui::backend::{Network, WlanStatus};

/// Saved networks kept at most; the oldest goes.
const SAVED: usize = 8;

/// How often a lost or missing connection is tried again.
const RETRY: Duration = Duration::from_secs(10);

/// How often the signal of the connection is read.
const SIGNAL: Duration = Duration::from_secs(5);

const NAMESPACE: &str = "wlan";
const NETWORKS: &str = "networks";

/// What the WLAN thread is asked to do.
enum Command {
    /// The radio on or off (SETUP, the exam mode).
    Radio(bool),
    Scan,
    /// Join a network, with a password for a new one.
    Connect(String, Option<String>),
    Forget(String),
    /// Whether the clock comes from the network.
    Ntp(bool),
}

/// What the UI reads.
struct State {
    status: WlanStatus,
    /// The last scan's networks, `None` while one runs.
    networks: Option<Vec<Network>>,
    /// The saved networks' names, in the order saved.
    saved: Vec<String>,
}

/// The handle the board keeps: sends commands, reads the state.
pub struct Wlan {
    commands: Sender<Command>,
    state: Arc<Mutex<State>>,
}

fn lock(state: &Mutex<State>) -> MutexGuard<'_, State> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Wlan {
    /// Starts the thread; WLAN stays off until [`Wlan::radio`] switches it on.
    pub fn start(modem: Modem<'static>, sysloop: EspSystemEventLoop, nvs: EspDefaultNvsPartition) -> Wlan {
        let state = Arc::new(Mutex::new(State { status: WlanStatus::Off, networks: Some(Vec::new()), saved: Vec::new() }));
        let (commands, received) = mpsc::channel();
        let shared = state.clone();
        std::thread::Builder::new()
            .name("wlan".into())
            .stack_size(12 * 1024)
            .spawn(move || match Radio::new(modem, sysloop, nvs, shared) {
                Ok(radio) => radio.run(received),
                Err(error) => println!("WLAN: no Wi-Fi driver ({error}); is the C6's ESP-Hosted firmware current?"),
            })
            .expect("WLAN thread");
        Wlan { commands, state }
    }

    pub fn radio(&self, on: bool) {
        let _ = self.commands.send(Command::Radio(on));
    }

    pub fn ntp(&self, on: bool) {
        let _ = self.commands.send(Command::Ntp(on));
    }

    pub fn status(&self) -> WlanStatus {
        lock(&self.state).status.clone()
    }

    pub fn scan(&self) {
        lock(&self.state).networks = None;
        let _ = self.commands.send(Command::Scan);
    }

    pub fn networks(&self) -> Option<Vec<Network>> {
        lock(&self.state).networks.clone()
    }

    pub fn connect(&self, ssid: &str, password: Option<&str>) {
        lock(&self.state).status = WlanStatus::Connecting(ssid.to_string());
        let _ = self.commands.send(Command::Connect(ssid.to_string(), password.map(str::to_string)));
    }

    pub fn saved(&self) -> Vec<String> {
        lock(&self.state).saved.clone()
    }

    pub fn forget(&self, ssid: &str) {
        lock(&self.state).saved.retain(|name| name != ssid);
        let _ = self.commands.send(Command::Forget(ssid.to_string()));
    }
}

/// The thread's side: the driver, the saved networks, network time.
struct Radio {
    wifi: BlockingWifi<EspWifi<'static>>,
    store: Option<EspDefaultNvs>,
    /// Name and password.
    saved: Vec<(String, String)>,
    state: Arc<Mutex<State>>,
    on: bool,
    ntp: bool,
    sntp: Option<EspSntp<'static>>,
    synced: bool,
    last_try: Option<Instant>,
    last_signal: Instant,
}

impl Radio {
    fn new(modem: Modem<'static>, sysloop: EspSystemEventLoop, nvs: EspDefaultNvsPartition, state: Arc<Mutex<State>>) -> Result<Radio, EspError> {
        let wifi = EspWifi::new(modem, sysloop.clone(), Some(nvs.clone()))?;
        let wifi = BlockingWifi::wrap(wifi, sysloop)?;
        let store = EspDefaultNvs::new(nvs, NAMESPACE, true).map_err(|error| println!("WLAN: saved networks not kept: {error}")).ok();
        let saved = store.as_ref().map(load).unwrap_or_default();
        lock(&state).saved = saved.iter().map(|(ssid, _)| ssid.clone()).collect();
        let last_signal = Instant::now();
        Ok(Radio { wifi, store, saved, state, on: false, ntp: false, sntp: None, synced: false, last_try: None, last_signal })
    }

    /// Sets the status the UI reads, and logs it unless only the signal changed.
    fn set_status(&self, status: WlanStatus) {
        let mut state = lock(&self.state);
        let news = match (&state.status, &status) {
            (WlanStatus::Connected { ssid: was, ip: had, .. }, WlanStatus::Connected { ssid, ip, .. }) => was != ssid || had != ip,
            (was, now) => was != now,
        };
        if news {
            println!("WLAN: {status:?}");
        }
        state.status = status;
    }

    fn run(mut self, commands: Receiver<Command>) {
        loop {
            match commands.recv_timeout(Duration::from_secs(1)) {
                Ok(command) => self.command(command),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            self.keep_up();
        }
    }

    fn command(&mut self, command: Command) {
        match command {
            Command::Radio(on) if on == self.on => {}
            Command::Radio(true) => {
                let started = self
                    .wifi
                    .set_configuration(&Configuration::Client(ClientConfiguration::default()))
                    .and_then(|()| self.wifi.start());
                match started {
                    Ok(()) => {
                        self.on = true;
                        self.set_status(WlanStatus::Searching);
                        self.last_try = None;
                    }
                    Err(error) => println!("WLAN: could not start: {error}"),
                }
            }
            Command::Radio(false) => {
                self.sntp = None;
                if let Err(error) = self.wifi.stop() {
                    println!("WLAN: could not stop: {error}");
                }
                self.on = false;
                self.set_status(WlanStatus::Off);
            }
            Command::Scan => {
                let found = if self.on { self.scan() } else { Vec::new() };
                lock(&self.state).networks = Some(found);
            }
            Command::Connect(ssid, password) => {
                if !self.on {
                    return self.set_status(WlanStatus::Off);
                }
                let known = self.saved.iter().find(|(name, _)| *name == ssid).map(|(_, password)| password.clone());
                let given = password.is_some();
                let password = password.or(known).unwrap_or_default();
                if self.join(&ssid, password.clone()) && given {
                    self.save(&ssid, password);
                }
                self.last_try = Some(Instant::now());
            }
            Command::Forget(ssid) => {
                self.saved.retain(|(name, _)| *name != ssid);
                self.keep();
                let joined = matches!(&lock(&self.state).status, WlanStatus::Connected { ssid: name, .. } | WlanStatus::Connecting(name) if *name == ssid);
                if joined {
                    let _ = self.wifi.disconnect();
                    self.set_status(WlanStatus::Searching);
                    self.last_try = None;
                }
            }
            Command::Ntp(on) => self.ntp = on,
        }
    }

    /// The networks in reach, strongest first, each once.
    fn scan(&mut self) -> Vec<Network> {
        let started = Instant::now();
        let found = match self.wifi.scan() {
            Ok(found) => found,
            Err(error) => {
                println!("WLAN: scan failed: {error}");
                return Vec::new();
            }
        };
        let mut networks: Vec<Network> = Vec::new();
        for ap in found {
            if ap.ssid.is_empty() || networks.iter().any(|network| network.ssid == ap.ssid.as_str()) {
                continue;
            }
            networks.push(Network {
                ssid: ap.ssid.to_string(),
                rssi: ap.signal_strength,
                secured: !matches!(ap.auth_method, None | Some(AuthMethod::None)),
                saved: self.saved.iter().any(|(name, _)| name == ap.ssid.as_str()),
            });
        }
        networks.sort_by_key(|network| std::cmp::Reverse(network.rssi));
        println!("WLAN: {} networks in {} ms", networks.len(), started.elapsed().as_millis());
        networks
    }

    /// Joins `ssid`; returns whether the connection stands with an address.
    fn join(&mut self, ssid: &str, password: String) -> bool {
        self.set_status(WlanStatus::Connecting(ssid.to_string()));
        if self.wifi.is_connected().unwrap_or(false) {
            let _ = self.wifi.disconnect();
        }
        let config = ClientConfiguration {
            ssid: ssid.try_into().unwrap_or_default(),
            auth_method: if password.is_empty() { AuthMethod::None } else { AuthMethod::WPA2Personal },
            password: password.as_str().try_into().unwrap_or_default(),
            ..Default::default()
        };
        let joined = self
            .wifi
            .set_configuration(&Configuration::Client(config))
            .and_then(|()| self.wifi.connect())
            .and_then(|()| self.wifi.wait_netif_up());
        match joined {
            Ok(()) => {
                self.read_status(ssid);
                true
            }
            Err(error) => {
                println!("WLAN: {ssid} refused: {error}");
                let _ = self.wifi.disconnect();
                self.set_status(WlanStatus::Failed(ssid.to_string()));
                false
            }
        }
    }

    /// The status of the connection to `ssid` as it stands: address and signal.
    fn read_status(&mut self, ssid: &str) {
        let ip = self.wifi.wifi().sta_netif().get_ip_info().map(|info| info.ip.to_string()).unwrap_or_default();
        let rssi = self.wifi.wifi().driver().get_ap_info().map_or(0, |ap| ap.signal_strength);
        self.last_signal = Instant::now();
        self.set_status(WlanStatus::Connected { ssid: ssid.to_string(), ip, rssi });
    }

    /// Joins the strongest saved network in reach while not connected, follows the signal, and
    /// runs network time while connected and wanted.
    fn keep_up(&mut self) {
        if !self.on {
            return;
        }
        let connected = self.wifi.is_connected().unwrap_or(false);
        let status = lock(&self.state).status.clone();
        match status {
            WlanStatus::Connected { ssid, .. } if connected => {
                if self.last_signal.elapsed() >= SIGNAL {
                    self.read_status(&ssid);
                }
            }
            _ if self.last_try.is_some_and(|last| last.elapsed() < RETRY) => {}
            // Nothing to join: no scans in the background.
            _ if self.saved.is_empty() => {
                if matches!(status, WlanStatus::Connected { .. } | WlanStatus::Connecting(_)) {
                    self.set_status(WlanStatus::Searching);
                }
            }
            _ => {
                self.last_try = Some(Instant::now());
                let networks = self.scan();
                lock(&self.state).networks = Some(networks.clone());
                let best = networks.into_iter().find(|network| network.saved);
                match best {
                    Some(network) => {
                        let password = self.saved.iter().find(|(name, _)| *name == network.ssid).map(|(_, p)| p.clone());
                        self.join(&network.ssid, password.unwrap_or_default());
                    }
                    None => self.set_status(WlanStatus::Searching),
                }
            }
        }
        let connected = lock(&self.state).status.connected();
        match (connected && self.ntp, &self.sntp) {
            (true, None) => match EspSntp::new_default() {
                Ok(sntp) => {
                    println!("WLAN: network time started");
                    self.sntp = Some(sntp);
                    self.synced = false;
                }
                Err(error) => println!("WLAN: no network time: {error}"),
            },
            (false, Some(_)) => self.sntp = None,
            _ => {}
        }
        if let Some(sntp) = &self.sntp {
            if !self.synced && sntp.get_sync_status() == SyncStatus::Completed {
                self.synced = true;
                println!("WLAN: clock set from the network");
            }
        }
    }

    /// Keeps `ssid` with its `password`, as the newest.
    fn save(&mut self, ssid: &str, password: String) {
        self.saved.retain(|(name, _)| name != ssid);
        self.saved.push((ssid.to_string(), password));
        if self.saved.len() > SAVED {
            self.saved.remove(0);
        }
        self.keep();
    }

    /// Writes the saved networks to NVS and shows them to the UI.
    fn keep(&mut self) {
        lock(&self.state).saved = self.saved.iter().map(|(ssid, _)| ssid.clone()).collect();
        let text: String = self.saved.iter().map(|(ssid, password)| format!("{ssid}\t{password}\n")).collect();
        if let Some(Err(error)) = self.store.as_ref().map(|store| store.set_str(NETWORKS, &text)) {
            println!("WLAN: saved networks not kept: {error}");
        }
    }
}

/// The saved networks from NVS: a name and a password per line, split by a tab.
fn load(store: &EspDefaultNvs) -> Vec<(String, String)> {
    let mut buffer = vec![0u8; SAVED * 100 + 1];
    let Ok(Some(text)) = store.get_str(NETWORKS, &mut buffer) else { return Vec::new() };
    text.lines().filter_map(|line| line.split_once('\t')).map(|(ssid, password)| (ssid.to_string(), password.to_string())).collect()
}
