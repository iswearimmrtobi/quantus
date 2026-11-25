//! The Duo's main screen (firmware/docs/duo.md) on the dev kit's MIPI-DSI connector: for now the
//! 10.1" Waveshare 10.1-DSI-TOUCH-A bench panel, 800×1280 RGB565 over two lanes, a JD9365
//! controller set up over MIPI-DBI, then a DPI video stream from two frame buffers in PSRAM.
//!
//! The UI draws landscape, 1280×800, into a buffer of its own, row by row as the CPU cache likes
//! it. [`MainScreen::present`] has the P4's pixel processing accelerator (PPA) turn the part that
//! changed into the frame buffer that is not on screen, and the DPI driver switches to it at the
//! next frame end. Drawing straight into the portrait frame buffer writes each pixel a row
//! (1600 bytes) from the last, which the SmartPanel firmware measured as several times slower.
//!
//! Timings and the start follow Waveshare's BSP for this panel on the ESP32-P4-Module-DEV-KIT
//! (`waveshare/esp_lcd_jd9365_10_1` 2.0.0) and the SmartPanel firmware, which runs the same panel
//! on the same board. The panel's power, its backlight and its touch controller (`goodix.rs`) are
//! on the I²C bus of the 3.5" screen's touch ([`power_on`]).

use core::ffi::c_void;
use core::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use embedded_graphics::pixelcolor::raw::RawU16;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use esp_idf_svc::hal::delay::{FreeRtos, BLOCK};
use esp_idf_svc::hal::i2c::I2cDriver;
use esp_idf_svc::sys::{self, esp, EspError};

use crate::jd9365;

/// The panel, portrait.
const PANEL_W: u32 = 800;
const PANEL_H: u32 = 1280;
const PIXELS: usize = (PANEL_W * PANEL_H) as usize;
/// The picture, landscape.
pub const WIDTH: u32 = PANEL_H;
pub const HEIGHT: u32 = PANEL_W;

/// The LDO channel that powers the MIPI D-PHY (LDO_VO3), at 2.5 V.
const PHY_LDO_CHANNEL: i32 = 3;
const PHY_LDO_MV: i32 = 2500;
const LANE_MBPS: f32 = 1500.0;
const PIXEL_CLOCK_MHZ: f32 = 80.0;
/// PSRAM buffers for DMA start on a cache line (L1 and L2, 64 or 128 bytes).
const DMA_ALIGN: usize = 128;

/// The panel's controller for its power and backlight, and its registers: power, and the
/// brightness 0–255.
const BACKLIGHT: u8 = 0x45;
const POWER: u8 = 0x95;
const BRIGHTNESS: u8 = 0x96;

/// Which way the picture is turned onto the portrait panel: `QUANTUS_DSI_TURN=270` at build time
/// turns it the other way, for a panel mounted upside down.
const TURN_BACK: bool = matches!(option_env!("QUANTUS_DSI_TURN"), Some(turn) if turn.len() == 3 && turn.as_bytes()[0] == b'2');

/// Set from the DPI interrupt when the buffer switched away from is free again.
static FRAME_FREE: AtomicBool = AtomicBool::new(false);

/// Called from the DMA interrupt, which also runs while the flash is written and the cache is off
/// (`CONFIG_LCD_DSI_ISR_CACHE_SAFE`), so it lives in IRAM.
#[link_section = ".iram1.quantus_frame_buffer_free"]
unsafe extern "C" fn frame_buffer_free(
    _panel: sys::esp_lcd_panel_handle_t,
    _data: *mut sys::esp_lcd_dpi_panel_event_data_t,
    _context: *mut c_void,
) -> bool {
    FRAME_FREE.store(true, Ordering::Release);
    false
}

/// Switches the panel's power on and its backlight to full, as Waveshare's driver does: before
/// [`MainScreen::open`], or the controller does not answer.
pub fn power_on(i2c: &mut I2cDriver) -> Result<(), EspError> {
    i2c.write(BACKLIGHT, &[POWER, 0x11], BLOCK)?;
    i2c.write(BACKLIGHT, &[POWER, 0x17], BLOCK)?;
    i2c.write(BACKLIGHT, &[BRIGHTNESS, 0x00], BLOCK)?;
    FreeRtos::delay_ms(100);
    i2c.write(BACKLIGHT, &[BRIGHTNESS, 0xFF], BLOCK)
}

/// Sets the backlight to `level`, 1–10 as the settings give it.
pub fn set_brightness(i2c: &mut I2cDriver, level: u8) -> Result<(), EspError> {
    let level = u32::from(level.clamp(1, 10));
    let value = (255 * level * level / 100).max(3) as u8;
    i2c.write(BACKLIGHT, &[BRIGHTNESS, value], BLOCK)
}

pub struct MainScreen {
    panel: sys::esp_lcd_panel_handle_t,
    buffers: [*mut u16; 2],
    /// The frame buffer not on screen, which the next picture goes into.
    back: usize,
    /// Whether the last switch may still be pending: the back buffer is scanned out until the
    /// frame end after it.
    switching: bool,
    /// The landscape picture the UI draws into.
    picture: &'static mut [u16],
    ppa: sys::ppa_client_handle_t,
    /// What was drawn since the last [`MainScreen::present`].
    changed: Option<Rectangle>,
    /// What changed in the last picture: the back buffer, one picture behind, still lacks it.
    last: Option<Rectangle>,
}

impl MainScreen {
    /// Brings the panel up and shows black. [`power_on`] must have run.
    pub fn open() -> Result<MainScreen, EspError> {
        // SAFETY: plain ESP-IDF calls with configurations that live through each call; the
        // handles they return are kept for good.
        unsafe {
            let mut ldo = null_mut();
            let ldo_config = sys::esp_ldo_channel_config_t { chan_id: PHY_LDO_CHANNEL, voltage_mv: PHY_LDO_MV, ..Default::default() };
            esp!(sys::esp_ldo_acquire_channel(&ldo_config, &mut ldo))?;

            let bus_config = sys::esp_lcd_dsi_bus_config_t {
                bus_id: 0,
                num_data_lanes: 2,
                lane_bit_rate_mbps: LANE_MBPS,
                ..Default::default()
            };
            let mut bus = null_mut();
            esp!(sys::esp_lcd_new_dsi_bus(&bus_config, &mut bus))?;

            let io_config = sys::esp_lcd_dbi_io_config_t { virtual_channel: 0, lcd_cmd_bits: 8, lcd_param_bits: 8 };
            let mut io = null_mut();
            esp!(sys::esp_lcd_new_panel_io_dbi(bus, &io_config, &mut io))?;
            init_controller(io)?;

            let mut dpi = sys::esp_lcd_dpi_panel_config_t {
                virtual_channel: 0,
                dpi_clock_freq_mhz: PIXEL_CLOCK_MHZ,
                pixel_format: sys::lcd_color_rgb_pixel_format_t_LCD_COLOR_PIXEL_FORMAT_RGB565,
                num_fbs: 2,
                video_timing: sys::esp_lcd_video_timing_t {
                    h_size: PANEL_W,
                    v_size: PANEL_H,
                    hsync_back_porch: 20,
                    hsync_pulse_width: 20,
                    hsync_front_porch: 40,
                    vsync_back_porch: 10,
                    vsync_pulse_width: 4,
                    vsync_front_porch: 30,
                },
                ..Default::default()
            };
            dpi.flags.set_use_dma2d(0);
            let mut panel = null_mut();
            esp!(sys::esp_lcd_new_panel_dpi(bus, &dpi, &mut panel))?;
            esp!(sys::esp_lcd_panel_init(panel))?;

            let (mut first, mut second): (*mut c_void, *mut c_void) = (null_mut(), null_mut());
            esp!(sys::esp_lcd_dpi_panel_get_frame_buffer(panel, 2, &mut first, &mut second))?;

            let mut callbacks = sys::esp_lcd_dpi_panel_event_callbacks_t::default();
            callbacks.__bindgen_anon_1.on_frame_buf_complete = Some(frame_buffer_free);
            esp!(sys::esp_lcd_dpi_panel_register_event_callbacks(panel, &callbacks, null_mut()))?;

            let bytes = (WIDTH * HEIGHT) as usize * 2;
            let picture = sys::heap_caps_aligned_calloc(DMA_ALIGN, 1, bytes, sys::MALLOC_CAP_SPIRAM);
            if picture.is_null() {
                return Err(EspError::from_infallible::<{ sys::ESP_ERR_NO_MEM }>());
            }
            let picture = std::slice::from_raw_parts_mut(picture.cast::<u16>(), (WIDTH * HEIGHT) as usize);

            let ppa_config = sys::ppa_client_config_t {
                oper_type: sys::ppa_operation_t_PPA_OPERATION_SRM,
                max_pending_trans_num: 1,
                ..Default::default()
            };
            let mut ppa = null_mut();
            esp!(sys::ppa_register_client(&ppa_config, &mut ppa))?;

            let mut screen = MainScreen {
                panel,
                buffers: [first.cast(), second.cast()],
                back: 1,
                switching: false,
                picture,
                ppa,
                changed: None,
                last: None,
            };
            // Both frame buffers black, whatever they held.
            screen.changed = Some(screen.bounding_box());
            screen.present();
            screen.changed = Some(screen.bounding_box());
            screen.present();
            log::info!("main screen: {WIDTH}×{HEIGHT} on the {PANEL_W}×{PANEL_H} DSI panel");
            Ok(screen)
        }
    }

    /// Shows what was drawn: turned into the back frame buffer together with the last picture's
    /// change, which that buffer has not seen, then switched to.
    pub fn present(&mut self) {
        let changed = self.changed.take();
        let area = match (changed, self.last) {
            (Some(a), Some(b)) => envelope(a, b),
            (Some(a), None) | (None, Some(a)) => a,
            (None, None) => return,
        };
        self.last = changed;
        self.wait_free();
        if let Err(error) = self.rotate(area) {
            log::warn!("main screen: PPA failed: {error}");
            return;
        }
        // The panel rows the area lands on.
        let (x, w) = (area.top_left.x as u32, area.size.width);
        let rows = if TURN_BACK { PANEL_H - (x + w)..PANEL_H - x } else { x..x + w };
        self.switch(rows);
    }

    /// A point the touch controller reports on the portrait panel, on the landscape picture.
    pub fn landscape(&self, panel: Point) -> Point {
        let (x, y) = (panel.x.clamp(0, PANEL_W as i32 - 1), panel.y.clamp(0, PANEL_H as i32 - 1));
        if TURN_BACK { Point::new(PANEL_H as i32 - 1 - y, x) } else { Point::new(y, PANEL_W as i32 - 1 - x) }
    }

    /// The PPA turns `area` of the picture into the back frame buffer.
    fn rotate(&mut self, area: Rectangle) -> Result<(), EspError> {
        let (x, y, w, h) = (area.top_left.x as u32, area.top_left.y as u32, area.size.width, area.size.height);
        // The PPA turns counter-clockwise; the picture is turned clockwise, or back.
        let (angle, out_x, out_y) = if TURN_BACK {
            (sys::ppa_srm_rotation_angle_t_PPA_SRM_ROTATION_ANGLE_90, y, PANEL_H - (x + w))
        } else {
            (sys::ppa_srm_rotation_angle_t_PPA_SRM_ROTATION_ANGLE_270, PANEL_W - (y + h), x)
        };
        let rgb565 = sys::ppa_srm_color_mode_t_PPA_SRM_COLOR_MODE_RGB565;
        let mut config = sys::ppa_srm_oper_config_t {
            rotation_angle: angle,
            scale_x: 1.0,
            scale_y: 1.0,
            mode: sys::ppa_trans_mode_t_PPA_TRANS_MODE_BLOCKING,
            ..Default::default()
        };
        config.in_.buffer = self.picture.as_ptr().cast();
        config.in_.pic_w = WIDTH;
        config.in_.pic_h = HEIGHT;
        config.in_.block_w = w;
        config.in_.block_h = h;
        config.in_.block_offset_x = x;
        config.in_.block_offset_y = y;
        config.in_.__bindgen_anon_1.srm_cm = rgb565;
        config.out.buffer = self.buffers[self.back].cast();
        config.out.buffer_size = (PIXELS * 2) as u32;
        config.out.pic_w = PANEL_W;
        config.out.pic_h = PANEL_H;
        config.out.block_offset_x = out_x;
        config.out.block_offset_y = out_y;
        config.out.__bindgen_anon_1.srm_cm = rgb565;
        // SAFETY: both buffers are the full size the configuration names; the PPA's DMA writes
        // the input back from the cache itself.
        unsafe { esp!(sys::ppa_do_scale_rotate_mirror(self.ppa, &config)) }
    }

    /// Hands the back buffer to the DPI driver, which shows it from the next frame on.
    fn switch(&mut self, rows: core::ops::Range<u32>) {
        // At least one row: the driver switches buffers only for a non-empty area.
        let start = rows.start.min(PANEL_H - 1);
        let end = rows.end.clamp(start + 1, PANEL_H);
        FRAME_FREE.store(false, Ordering::Release);
        // Any pointer into a frame buffer makes the driver switch to it instead of copying.
        // SAFETY: the row is inside the buffer.
        let buffer = unsafe { self.buffers[self.back].add((start * PANEL_W) as usize) }.cast::<c_void>();
        // SAFETY: the panel is open and the buffer is its own.
        let result = unsafe { esp!(sys::esp_lcd_panel_draw_bitmap(self.panel, 0, start as i32, PANEL_W as i32, end as i32, buffer)) };
        match result {
            Ok(()) => {
                self.back ^= 1;
                self.switching = true;
            }
            Err(error) => log::warn!("main screen: switching buffers failed: {error}"),
        }
    }

    /// Waits until the buffer switched away from is no longer scanned out, so it can be the back
    /// buffer.
    fn wait_free(&mut self) {
        if !self.switching {
            return;
        }
        // One frame is 1/60 s; more than three means the stream stopped.
        let started = Instant::now();
        while !FRAME_FREE.load(Ordering::Acquire) {
            if started.elapsed() > Duration::from_millis(50) {
                log::warn!("main screen: no frame end within 50 ms");
                break;
            }
            FreeRtos::delay_ms(1);
        }
        self.switching = false;
    }

    fn mark(&mut self, area: Rectangle) {
        self.changed = Some(match self.changed {
            Some(changed) => envelope(changed, area),
            None => area,
        });
    }
}

/// The smallest rectangle around both.
fn envelope(a: Rectangle, b: Rectangle) -> Rectangle {
    let top_left = Point::new(a.top_left.x.min(b.top_left.x), a.top_left.y.min(b.top_left.y));
    let end = |r: Rectangle| r.top_left + r.size;
    let (ea, eb) = (end(a), end(b));
    let bottom_right = Point::new(ea.x.max(eb.x), ea.y.max(eb.y));
    Rectangle::with_corners(top_left, bottom_right - Point::new(1, 1))
}

/// The JD9365's start: soft reset, its ID, the pixel format, two lanes, the vendor sequence.
unsafe fn init_controller(io: sys::esp_lcd_panel_io_handle_t) -> Result<(), EspError> {
    let send = |command: u8, data: &[u8]| {
        let pointer = if data.is_empty() { core::ptr::null() } else { data.as_ptr().cast() };
        esp!(sys::esp_lcd_panel_io_tx_param(io, i32::from(command), pointer, data.len()))
    };
    send(jd9365::SWRESET, &[])?;
    FreeRtos::delay_ms(120);
    let mut id = [0u8; 3];
    match esp!(sys::esp_lcd_panel_io_rx_param(io, 0x04, id.as_mut_ptr().cast(), id.len())) {
        Ok(()) => log::info!("main screen: JD9365 ID {:02X} {:02X} {:02X}", id[0], id[1], id[2]),
        Err(error) => log::warn!("main screen: no controller ID ({error})"),
    }
    send(jd9365::CMD_PAGE, &[0x00])?;
    send(jd9365::MADCTL, &[0x00])?;
    send(jd9365::COLMOD, &[jd9365::COLMOD_16BIT])?;
    send(jd9365::CMD_DSI_INT0, &[jd9365::DSI_2_LANE])?;
    for &(command, data, delay) in jd9365::INIT.iter() {
        send(command, &[data])?;
        if delay > 0 {
            FreeRtos::delay_ms(u32::from(delay));
        }
    }
    Ok(())
}

impl OriginDimensions for MainScreen {
    fn size(&self) -> Size {
        Size::new(WIDTH, HEIGHT)
    }
}

impl DrawTarget for MainScreen {
    type Color = Rgb565;
    type Error = EspError;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Rgb565>>,
    {
        for Pixel(point, color) in pixels {
            let (Ok(x), Ok(y)) = (u32::try_from(point.x), u32::try_from(point.y)) else { continue };
            if x < WIDTH && y < HEIGHT {
                self.picture[(y * WIDTH + x) as usize] = RawU16::from(color).into_inner();
                self.mark(Rectangle::new(point, Size::new(1, 1)));
            }
        }
        Ok(())
    }

    /// The renderer sends whole bands this way.
    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Rgb565>,
    {
        let clipped = area.intersection(&self.bounding_box());
        if clipped.is_zero_sized() {
            return Ok(());
        }
        let mut colors = colors.into_iter();
        if clipped == *area {
            let width = area.size.width as usize;
            for row in 0..area.size.height {
                let start = ((area.top_left.y as u32 + row) * WIDTH + area.top_left.x as u32) as usize;
                for (pixel, color) in self.picture[start..start + width].iter_mut().zip(colors.by_ref()) {
                    *pixel = RawU16::from(color).into_inner();
                }
            }
        } else {
            for point in area.points() {
                let Some(color) = colors.next() else { break };
                if clipped.contains(point) {
                    self.picture[(point.y as u32 * WIDTH + point.x as u32) as usize] = RawU16::from(color).into_inner();
                }
            }
        }
        self.mark(clipped);
        Ok(())
    }
}
