//! The SD card of the ESP32-P4 dev kit, mounted at [`MOUNT`] so `std::fs` reads it: SDMMC slot 0,
//! 4 bits wide, on its fixed pins (CLK 43, CMD 44, D0–D3 39–42, firmware/docs/dev-kit.md).
//! Those pins are in the P4's I/O domain fed by the on-chip LDO 4, which must be set to 3.3 V
//! first; GPIO45 switches the card's own supply. The clock is 10 MHz: at the driver's 20 MHz
//! reads work but every write times out (0x107), with only the internal pull-ups on the data
//! lines. `QUANTUS_SD_KHZ` at build time sets another. The card is FAT32 on an MBR partition table:
//! ESP-IDF's FatFs has neither exFAT nor GPT.

use esp_idf_svc::hal::gpio::{Gpio39, Gpio40, Gpio41, Gpio42, Gpio43, Gpio44, Gpio45, Output, Pin, PinDriver};
use esp_idf_svc::hal::sd::mmc::SDMMC0;
use esp_idf_svc::sys::{self, esp, EspError};
use quantus_ui::backend::Storage;

/// Where the card appears in the file system.
pub const MOUNT: &str = "/sd";

/// The session files (`quantus_engine::Engine`), one JSON file each, as on the desktop.
pub const SESSIONS: &str = "/sd/sessions";

/// The mounted card; it stays mounted. Dropping it switches its supply off.
pub struct SdCard {
    _card: *mut sys::sdmmc_card_t,
    _power: PinDriver<'static, Output>,
}

/// `sdmmc_host_t.flags`: the bus widths, DDR, and a `deinit` that takes the slot.
const FLAG_1BIT: u32 = 1 << 0;
const FLAG_4BIT: u32 = 1 << 1;
const FLAG_DDR: u32 = 1 << 4;
const FLAG_DEINIT_ARG: u32 = 1 << 5;

impl SdCard {
    /// Powers and mounts the card. Fails when there is none, or it is not FAT.
    ///
    /// Mounted through ESP-IDF itself, not esp-idf-hal's SDMMC driver: the controller's slot 1
    /// is the link to the C6 (ESP-Hosted, WLAN), and esp-idf-hal shuts the whole controller down
    /// when its driver is dropped after a failed mount, while ESP-IDF frees only slot 0.
    #[allow(clippy::too_many_arguments)]
    pub fn mount(
        _slot: SDMMC0<'static>,
        clk: Gpio43<'static>,
        cmd: Gpio44<'static>,
        d0: Gpio39<'static>,
        d1: Gpio40<'static>,
        d2: Gpio41<'static>,
        d3: Gpio42<'static>,
        power: Gpio45<'static>,
    ) -> Result<SdCard, EspError> {
        io_at_3v3()?;
        // PWR_CTRL drives the gate of the card's supply switch: low is on.
        let mut power = PinDriver::output(power)?;
        power.set_low()?;
        let khz = option_env!("QUANTUS_SD_KHZ").and_then(|khz| khz.parse().ok()).unwrap_or(10_000);
        let host = sys::sdmmc_host_t {
            flags: FLAG_1BIT | FLAG_4BIT | FLAG_DDR | FLAG_DEINIT_ARG,
            slot: 0,
            max_freq_khz: khz,
            io_voltage: 3.3,
            init: Some(sys::sdmmc_host_init),
            set_bus_width: Some(sys::sdmmc_host_set_bus_width),
            get_bus_width: Some(sys::sdmmc_host_get_slot_width),
            set_bus_ddr_mode: Some(sys::sdmmc_host_set_bus_ddr_mode),
            set_card_clk: Some(sys::sdmmc_host_set_card_clk),
            set_cclk_always_on: Some(sys::sdmmc_host_set_cclk_always_on),
            do_transaction: Some(sys::sdmmc_host_do_transaction),
            __bindgen_anon_1: sys::sdmmc_host_t__bindgen_ty_1 { deinit_p: Some(sys::sdmmc_host_deinit_slot) },
            io_int_enable: Some(sys::sdmmc_host_io_int_enable),
            io_int_wait: Some(sys::sdmmc_host_io_int_wait),
            get_real_freq: Some(sys::sdmmc_host_get_real_freq),
            set_input_delay: Some(sys::sdmmc_host_set_input_delay),
            get_dma_info: Some(sys::sdmmc_host_get_dma_info),
            check_buffer_alignment: Some(sys::sdmmc_host_check_buffer_alignment),
            ..Default::default()
        };
        let slot = sys::sdmmc_slot_config_t {
            clk: clk.pin() as _,
            cmd: cmd.pin() as _,
            d0: d0.pin() as _,
            d1: d1.pin() as _,
            d2: d2.pin() as _,
            d3: d3.pin() as _,
            d4: -1,
            d5: -1,
            d6: -1,
            d7: -1,
            __bindgen_anon_1: sys::sdmmc_slot_config_t__bindgen_ty_1 { cd: -1 },
            __bindgen_anon_2: sys::sdmmc_slot_config_t__bindgen_ty_2 { wp: -1 },
            width: 4,
            flags: 0,
        };
        let mount = sys::esp_vfs_fat_mount_config_t { max_files: 4, ..Default::default() };
        let mut card: *mut sys::sdmmc_card_t = core::ptr::null_mut();
        // SAFETY: the configs live through the call, the path is NUL-terminated, and the pins
        // and the slot are this function's.
        esp!(unsafe { sys::esp_vfs_fat_sdmmc_mount(c"/sd".as_ptr(), &host, (&slot as *const sys::sdmmc_slot_config_t).cast(), &mount, &mut card) })?;
        Ok(SdCard { _card: card, _power: power })
    }
}

/// Sets LDO 4, the supply of GPIO39–48, to 3.3 V (it starts at 1.8 V). The channel stays
/// acquired for good.
fn io_at_3v3() -> Result<(), EspError> {
    let config = sys::esp_ldo_channel_config_t { chan_id: 4, voltage_mv: 3300, ..Default::default() };
    let mut channel: sys::esp_ldo_channel_handle_t = core::ptr::null_mut();
    // SAFETY: plain C call with a valid config and out pointer.
    esp!(unsafe { sys::esp_ldo_acquire_channel(&config, &mut channel) })
}

/// The card's size and free space, from the FAT. The first call after mounting counts the free
/// clusters, which takes a moment on a big card.
pub fn storage() -> Option<Storage> {
    let (mut total, mut free) = (0u64, 0u64);
    // SAFETY: plain C call with a NUL-terminated path and valid out pointers.
    esp!(unsafe { sys::esp_vfs_fat_info(c"/sd".as_ptr(), &mut total, &mut free) }).ok()?;
    Some(Storage { total, free })
}

/// The 480×320 BMP files in `/wallpapers/` on the card: their names without `.bmp`, sorted.
pub fn wallpapers() -> Vec<String> {
    let Ok(files) = std::fs::read_dir(format!("{MOUNT}/wallpapers")) else { return Vec::new() };
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

/// The file of the wallpaper `name`.
pub fn wallpaper(name: &str) -> Option<Vec<u8>> {
    std::fs::read(format!("{MOUNT}/wallpapers/{name}.bmp")).ok()
}
