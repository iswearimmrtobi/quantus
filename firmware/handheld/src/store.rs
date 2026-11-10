//! What the handheld keeps in flash: a namespace in the NVS partition with the touch
//! calibration, the wallpaper's name, the theme's id, the settings and the C6 image sent last.

use esp_idf_svc::nvs::{EspDefaultNvs, EspDefaultNvsPartition};
use esp_idf_svc::sys::EspError;
use quantus_ui::app::Settings;
use quantus_ui::touch::Calibration;

pub struct Store(EspDefaultNvs);

const NAMESPACE: &str = "quantus";
/// Not "touch_cal": that one was made for the resistive panel's readings and would be wrong for
/// the capacitive panel.
const CALIBRATION: &str = "touch_cal_i2c";
const WALLPAPER: &str = "wallpaper";
const THEME: &str = "theme";
const SETTINGS: &str = "settings";
const C6_TRIED: &str = "c6_tried";

impl Store {
    /// The namespace in `partition`; several stores may be open on it at once.
    pub fn open(partition: EspDefaultNvsPartition) -> Result<Store, EspError> {
        Ok(Store(EspDefaultNvs::new(partition, NAMESPACE, true)?))
    }

    pub fn load(&self) -> Option<Calibration> {
        let mut buffer = [0u8; 24];
        let bytes = self.0.get_blob(CALIBRATION, &mut buffer).ok()??;
        Calibration::from_bytes(bytes)
    }

    pub fn save(&self, calibration: &Calibration) -> Result<(), EspError> {
        self.0.set_blob(CALIBRATION, &calibration.to_bytes())
    }

    /// The name of the wallpaper set, if any.
    pub fn load_wallpaper(&self) -> Option<String> {
        let mut buffer = [0u8; 256];
        self.0.get_str(WALLPAPER, &mut buffer).ok()?.map(str::to_string)
    }

    /// Keeps the wallpaper `name`, or forgets it for `None`.
    pub fn save_wallpaper(&self, name: Option<&str>) -> Result<(), EspError> {
        match name {
            Some(name) => self.0.set_str(WALLPAPER, name),
            None => self.0.remove(WALLPAPER).map(|_| ()),
        }
    }

    /// The id of the colour theme set, if one was kept.
    pub fn load_theme(&self) -> Option<String> {
        let mut buffer = [0u8; 256];
        self.0.get_str(THEME, &mut buffer).ok()?.map(str::to_string)
    }

    pub fn save_theme(&self, id: &str) -> Result<(), EspError> {
        self.0.set_str(THEME, id)
    }

    /// The settings of the last run, if they were kept.
    pub fn load_settings(&self) -> Option<Settings> {
        let mut buffer = [0u8; 2048];
        let text = self.0.get_str(SETTINGS, &mut buffer).ok()??;
        Some(Settings::from_text(text))
    }

    /// Keeps `settings`, unless they are already what is kept (a flash write saved).
    pub fn save_settings(&self, settings: &Settings) -> Result<(), EspError> {
        if self.load_settings().as_ref() == Some(settings) {
            return Ok(());
        }
        self.0.set_str(SETTINGS, &settings.to_text())
    }

    /// The C6 image sent last (`c6::Image::id`), so it is not sent again.
    pub fn c6_tried(&self) -> Option<String> {
        let mut buffer = [0u8; 64];
        self.0.get_str(C6_TRIED, &mut buffer).ok()?.map(str::to_string)
    }

    pub fn set_c6_tried(&self, id: &str) -> Result<(), EspError> {
        self.0.set_str(C6_TRIED, id)
    }
}
