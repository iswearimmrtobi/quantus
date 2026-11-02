//! The SD card of the ESP32-P4 dev kit, mounted at [`MOUNT`] so `std::fs` reads it: SDMMC slot 0,
//! 4 bits wide, on its fixed pins (CLK 43, CMD 44, D0–D3 39–42, design/esp32-p4-dev-kit.md).
//! Those pins are in the P4's I/O domain fed by the on-chip LDO 4, which must be set to 3.3 V
//! first; GPIO45 switches the card's own supply. The card is FAT32 on an MBR partition table:
//! ESP-IDF's FatFs has neither exFAT nor GPT.

use esp_idf_svc::fs::fatfs::Fatfs;
use esp_idf_svc::hal::gpio::{Gpio39, Gpio40, Gpio41, Gpio42, Gpio43, Gpio44, Gpio45, Output, PinDriver};
use esp_idf_svc::hal::sd::mmc::{SdMmcHostConfiguration, SdMmcHostDriver, SDMMC0};
use esp_idf_svc::hal::sd::{SdCardConfiguration, SdCardDriver};
use esp_idf_svc::io::vfs::MountedFatfs;
use esp_idf_svc::sys::{self, esp, EspError};

/// Where the card appears in the file system.
pub const MOUNT: &str = "/sd";

/// The mounted card. Dropping it unmounts the card and switches its supply off.
pub struct SdCard {
    _mounted: MountedFatfs<Fatfs<SdCardDriver<SdMmcHostDriver<'static>>>>,
    _power: PinDriver<'static, Output>,
}

impl SdCard {
    /// Powers and mounts the card. Fails when there is none, or it is not FAT.
    #[allow(clippy::too_many_arguments)]
    pub fn mount(
        slot: SDMMC0<'static>,
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
        let host = SdMmcHostDriver::new_4bits(
            slot,
            cmd,
            clk,
            d0,
            d1,
            d2,
            d3,
            Option::<Gpio45>::None,
            Option::<Gpio45>::None,
            &SdMmcHostConfiguration::new(),
        )?;
        let card = SdCardDriver::new_mmc(host, &SdCardConfiguration::new())?;
        let mounted = MountedFatfs::mount(Fatfs::new_sdcard(0, card)?, MOUNT, 4)?;
        Ok(SdCard { _mounted: mounted, _power: power })
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
