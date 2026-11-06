//! Testing only: the window's picture sent to the board over serial (`--mirror <port>`), where
//! the `mirror` binary of `firmware/handheld` shows it. The protocol is in `quantus-mirror`.

use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use quantus_mirror::Shadow;
use quantus_ui::render::{HEIGHT, WIDTH};
use serialport::SerialPort;

pub struct Mirror {
    port: Box<dyn SerialPort>,
    /// Set when the board said `READY`: it was reset and needs the whole screen.
    ready: Arc<AtomicBool>,
    /// What the board shows, so only what changed in a band goes out.
    shadow: Shadow,
}

impl Mirror {
    /// Opens `path` at `baud`, which must match the board's ([`quantus_mirror::BAUD`]).
    pub fn open(path: &str, baud: u32) -> Result<Mirror, String> {
        let mut port = serialport::new(path, baud)
            .timeout(Duration::from_secs(2))
            .open()
            .map_err(|error| format!("{path}: {error} (is the serial monitor still open?)"))?;
        // Neither line held, so the board's auto-reset circuit lets it run.
        let _ = port.write_data_terminal_ready(false);
        let _ = port.write_request_to_send(false);
        let ready = Arc::new(AtomicBool::new(false));
        let mut reader = port.try_clone().map_err(|error| format!("{path}: {error}"))?;
        let flag = ready.clone();
        std::thread::spawn(move || {
            let mut line = Vec::new();
            let mut byte = [0u8; 1];
            loop {
                match reader.read(&mut byte) {
                    Ok(1) if byte[0] == b'\n' => {
                        if line.ends_with(b"READY") {
                            flag.store(true, Ordering::Relaxed);
                        }
                        line.clear();
                    }
                    Ok(1) => line.push(byte[0]),
                    Err(error) if error.kind() == std::io::ErrorKind::TimedOut => {}
                    _ => return,
                }
            }
        });
        Ok(Mirror { port, ready, shadow: Shadow::new(WIDTH as u16, HEIGHT as u16) })
    }

    /// Whether the board was reset since the last call; then it gets everything again.
    pub fn take_ready(&mut self) -> bool {
        let ready = self.ready.swap(false, Ordering::Relaxed);
        if ready {
            self.shadow.forget();
        }
        ready
    }

    /// Sends the part of the band that differs from what the board shows.
    fn send(&mut self, area: &Rectangle, pixels: &[Rgb565]) {
        let size = area.size;
        let raw: Vec<u16> = pixels.iter().map(|color| color.into_storage()).collect();
        let (x, y) = (area.top_left.x as u16, area.top_left.y as u16);
        let Some(frame) = self.shadow.update(x, y, size.width as u16, size.height as u16, &raw) else { return };
        if let Err(error) = self.port.write_all(&frame) {
            eprintln!("mirror: {error}");
        }
    }
}

/// Draws into the window's display and, if there is one, sends the same to the board.
pub struct Tee<'a, D> {
    pub display: &'a mut D,
    pub mirror: Option<&'a mut Mirror>,
}

impl<D: DrawTarget<Color = Rgb565>> OriginDimensions for Tee<'_, D> {
    fn size(&self) -> Size {
        self.display.bounding_box().size
    }
}

impl<D: DrawTarget<Color = Rgb565>> DrawTarget for Tee<'_, D> {
    type Color = Rgb565;
    type Error = D::Error;

    /// The renderer only sends whole bands; single pixels go to the window alone.
    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Rgb565>>,
    {
        self.display.draw_iter(pixels)
    }

    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Rgb565>,
    {
        let pixels: Vec<Rgb565> = colors.into_iter().collect();
        if let Some(mirror) = self.mirror.as_deref_mut() {
            mirror.send(area, &pixels);
        }
        self.display.fill_contiguous(area, pixels)
    }
}
