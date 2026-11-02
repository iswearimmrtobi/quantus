//! The ESP32-C6's firmware, kept at the ESP-Hosted version of the host side: the image built
//! from `firmware/c6` is flashed into the P4's `c6_fw` partition, and at boot, while the C6 runs
//! another version, the P4 sends it to the C6 over the SDIO link (ESP-Hosted's co-processor OTA)
//! and restarts. Each image is tried once (its version and size kept in NVS), so a failed update
//! does not loop. See firmware/docs/wlan.md.

use core::ffi::c_void;

use esp_idf_svc::sys::{self, esp, EspError};

/// ESP-Hosted's `eh_host_coprocessor_fwver_t`.
#[repr(C)]
#[derive(Default)]
#[allow(dead_code)]
struct FwVersion {
    major1: u32,
    minor1: u32,
    patch1: u32,
    revision: i32,
    prerelease: i32,
    build: i32,
}

extern "C" {
    fn eh_host_wait_auto_init_ready(timeout_ms: u32) -> i32;
    fn eh_host_sys_get_cp_fw_version(version: *mut FwVersion) -> sys::esp_err_t;
    fn eh_host_cp_ota_begin() -> sys::esp_err_t;
    fn eh_host_cp_ota_write(data: *const u8, len: u32) -> sys::esp_err_t;
    fn eh_host_cp_ota_end() -> sys::esp_err_t;
    fn eh_host_cp_ota_activate() -> sys::esp_err_t;
    fn eh_host_deinit() -> i32;
}

/// The partition with the C6's image: data, subtype `undefined` (ESP-Hosted's examples use 0x40,
/// which espflash does not take).
const LABEL: &core::ffi::CStr = c"c6_fw";
const SUBTYPE: sys::esp_partition_subtype_t = 0x06;

/// Bytes per OTA write: below ESP-Hosted's payload limit over SDIO.
const CHUNK: usize = 1400;

/// How long the link to the C6 may take to come up at boot.
const LINK_MS: u32 = 8000;

/// An image in the `c6_fw` partition.
pub struct Image {
    partition: *const sys::esp_partition_t,
    pub size: usize,
    /// The version in its app description, ESP-Hosted's ("3.0.10").
    pub version: String,
}

impl Image {
    /// The image in the partition, if there is one and it is an ESP image.
    pub fn find() -> Option<Image> {
        // SAFETY: plain C call with a NUL-terminated label.
        let partition = unsafe { sys::esp_partition_find_first(sys::esp_partition_type_t_ESP_PARTITION_TYPE_DATA, SUBTYPE, LABEL.as_ptr()) };
        if partition.is_null() {
            return None;
        }
        let read = |offset: usize, buffer: &mut [u8]| {
            // SAFETY: the partition exists and the buffer is as long as said.
            esp!(unsafe { sys::esp_partition_read(partition, offset, buffer.as_mut_ptr().cast::<c_void>(), buffer.len()) })
        };
        // The image header: magic 0xE9, the segment count at byte 1, 24 bytes in all.
        let mut header = [0u8; 24];
        read(0, &mut header).ok()?;
        if header[0] != sys::ESP_IMAGE_HEADER_MAGIC as u8 {
            return None;
        }
        let hash = header[23] == 1;
        // Each segment: load address and length (8 bytes), then the data.
        let mut offset = header.len();
        let mut version = String::new();
        for segment in 0..header[1] {
            let mut head = [0u8; 8];
            read(offset, &mut head).ok()?;
            let length = u32::from_le_bytes([head[4], head[5], head[6], head[7]]) as usize;
            if segment == 0 {
                // The app description opens the first segment; its version is at byte 16.
                let mut text = [0u8; 32];
                read(offset + 8 + 16, &mut text).ok()?;
                let end = text.iter().position(|&b| b == 0).unwrap_or(text.len());
                version = String::from_utf8_lossy(&text[..end]).into_owned();
            }
            offset += 8 + length;
        }
        // Padding to 16 bytes with the checksum byte at the end, then the SHA-256 if appended.
        let size = (offset + 1).div_ceil(16) * 16 + if hash { 32 } else { 0 };
        Some(Image { partition, size, version })
    }

    /// What NVS keeps of an image tried: its version and size.
    pub fn id(&self) -> String {
        format!("{} {}", self.version, self.size)
    }
}

/// The C6's ESP-Hosted version, once the link is up; `None` without one. A firmware too old to
/// tell is 0.0.0.
pub fn version() -> Option<String> {
    // SAFETY: plain C calls; the version is written into a valid struct.
    unsafe {
        // In short waits: the call answers at once once the link is up, but a long one can
        // miss it when another task takes the one signal that it is.
        let ready = (0..LINK_MS / 100).any(|_| eh_host_wait_auto_init_ready(100) == 0);
        if !ready {
            println!("C6: ESP-Hosted's start did not report ready; asking anyway");
        }
        let mut version = FwVersion::default();
        if let Err(error) = esp!(eh_host_sys_get_cp_fw_version(&mut version)) {
            // The kit's first firmware does not answer this, and says 0.0.0 when the link
            // comes up.
            println!("C6: no version ({error}): an old firmware");
            return Some("0.0.0".to_string());
        }
        Some(format!("{}.{}.{}", version.major1, version.minor1, version.patch1))
    }
}

/// Sends `image` to the C6, telling `progress` the percent done. On success the C6 boots it;
/// the host has to restart to find it again ([`restart`]).
pub fn send(image: &Image, running: &str, mut progress: impl FnMut(u8)) -> Result<(), EspError> {
    // SAFETY: plain C calls; each write's buffer is valid for its length.
    unsafe {
        esp!(eh_host_cp_ota_begin())?;
        let mut buffer = vec![0u8; CHUNK];
        let mut sent = 0;
        while sent < image.size {
            let length = CHUNK.min(image.size - sent);
            esp!(sys::esp_partition_read(image.partition, sent, buffer.as_mut_ptr().cast::<c_void>(), length))?;
            esp!(eh_host_cp_ota_write(buffer.as_ptr(), length as u32))?;
            sent += length;
            progress((sent * 100 / image.size) as u8);
        }
        esp!(eh_host_cp_ota_end())?;
        // From ESP-Hosted 2.6 on the new image is activated by a call of its own; older ones
        // switch at the end.
        let parts: Vec<u32> = running.split('.').filter_map(|part| part.parse().ok()).collect();
        if parts.as_slice() >= [2, 6].as_slice() {
            esp!(eh_host_cp_ota_activate())?;
        }
    }
    Ok(())
}

/// Lets the C6 boot its new firmware, then restarts the P4 to bring up the link again.
pub fn restart() -> ! {
    // SAFETY: plain C calls.
    unsafe {
        eh_host_deinit();
        // Longer than the C6 takes to switch and boot (about 600 ms): reset inside its
        // pending-verify window, it would roll the update back.
        sys::vTaskDelay(2000 * sys::configTICK_RATE_HZ / 1000);
        sys::esp_restart();
    }
}
