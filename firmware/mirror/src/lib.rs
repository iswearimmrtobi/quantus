//! Testing only: the simulator on the PC sends its picture over serial and the board shows it,
//! so the display can be tried with the real engine before that runs on the board.
//!
//! A frame is one rectangle of RGB565 pixels, row by row, run-length encoded:
//!
//! | bytes | content |
//! |---|---|
//! | 2 | magic `A5 5B` |
//! | 8 | `x`, `y`, `width`, `height`, each u16 little-endian |
//! | … | pieces until the rectangle is full, each starting with a u16 little-endian `n` (1 to 0x7FFF): |
//! | 2 + 2 | a run: `n`, then the one `color` it repeats |
//! | 2 + 2n | literal pixels: `n` with the high bit set (`0x8000 | n`), then `n` colors |
//! | 2 | the wrapping sum of the header and piece bytes, u16 little-endian |
//!
//! Every u16 is little-endian. Flat areas of the UI become a few runs, and a picture costs
//! 2 bytes a pixel as literals. The decoder skips anything before a magic (such as the board's
//! boot log) and drops a frame whose sum does not match.
//!
//! [`Shadow`] keeps what the board shows, so only the part of a band that changed goes out.

pub const MAGIC: [u8; 2] = [0xA5, 0x5B];

/// The serial speed of both ends: 921600, or `QUANTUS_MIRROR_BAUD` at build time (the board's
/// CH340 manages 2000000). The simulator's `--baud` overrides it on the PC side.
pub const BAUD: u32 = number(option_env!("QUANTUS_MIRROR_BAUD"), 921_600);

/// The decimal number in `text`, or `default` without one; a build-time setting.
pub const fn number(text: Option<&str>, default: u32) -> u32 {
    let Some(text) = text else { return default };
    let bytes = text.as_bytes();
    let mut value: u32 = 0;
    let mut i = 0;
    while i < bytes.len() {
        assert!(bytes[i].is_ascii_digit(), "not a decimal number");
        value = value * 10 + (bytes[i] - b'0') as u32;
        i += 1;
    }
    value
}

/// The longest run or literal piece.
const MAX_PIECE: usize = 0x7FFF;
/// Marks a literal piece.
const LITERAL: u16 = 0x8000;

/// The frame for the `width` × `height` rectangle at (`x`, `y`) with `pixels`, row by row.
pub fn encode(x: u16, y: u16, width: u16, height: u16, pixels: impl IntoIterator<Item = u16>) -> Vec<u8> {
    let pixels: Vec<u16> = pixels.into_iter().collect();
    let mut frame = MAGIC.to_vec();
    for value in [x, y, width, height] {
        frame.extend(value.to_le_bytes());
    }
    let literal = |frame: &mut Vec<u8>, colors: &[u16]| {
        for chunk in colors.chunks(MAX_PIECE) {
            frame.extend((LITERAL | chunk.len() as u16).to_le_bytes());
            chunk.iter().for_each(|color| frame.extend(color.to_le_bytes()));
        }
    };
    // Runs of three or more; what lies between them goes as literals.
    let mut start = 0;
    let mut i = 0;
    while i < pixels.len() {
        let color = pixels[i];
        let run = pixels[i..].iter().take(MAX_PIECE).take_while(|&&c| c == color).count();
        if run >= 3 {
            literal(&mut frame, &pixels[start..i]);
            frame.extend((run as u16).to_le_bytes());
            frame.extend(color.to_le_bytes());
            i += run;
            start = i;
        } else {
            i += run;
        }
    }
    literal(&mut frame, &pixels[start..]);
    let sum = checksum(&frame[MAGIC.len()..]);
    frame.extend(sum.to_le_bytes());
    frame
}

fn checksum(bytes: &[u8]) -> u16 {
    bytes.iter().fold(0u16, |sum, &byte| sum.wrapping_add(byte as u16))
}

/// A decoded rectangle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    /// Row by row, `width * height` of them.
    pub pixels: Vec<u16>,
}

enum State {
    /// How many bytes of the magic were seen.
    Magic(usize),
    Header,
    /// The `n` of the next piece.
    Piece,
    /// The color of a run of this many.
    Run(usize),
    /// This many literal colors still to come.
    Literal(usize),
    Sum,
}

/// Reads frames from a byte stream, one byte at a time.
pub struct Decoder {
    max_pixels: usize,
    state: State,
    /// The header or run bytes of the current frame, for the sum.
    bytes: Vec<u8>,
    /// The bytes of the current header, run or sum.
    field: Vec<u8>,
    frame: Frame,
}

impl Decoder {
    /// Rectangles of more than `max_pixels` are taken for noise and skipped.
    pub fn new(max_pixels: usize) -> Self {
        Decoder {
            max_pixels,
            state: State::Magic(0),
            bytes: Vec::new(),
            field: Vec::new(),
            frame: Frame { x: 0, y: 0, width: 0, height: 0, pixels: Vec::new() },
        }
    }

    /// Takes the next byte; returns a frame when this byte completed one.
    pub fn push(&mut self, byte: u8) -> Option<Frame> {
        match self.state {
            State::Magic(seen) => {
                self.state = if byte == MAGIC[seen] {
                    if seen + 1 == MAGIC.len() {
                        self.bytes.clear();
                        self.field.clear();
                        State::Header
                    } else {
                        State::Magic(seen + 1)
                    }
                } else {
                    State::Magic(usize::from(byte == MAGIC[0]))
                };
                None
            }
            State::Header => {
                self.bytes.push(byte);
                self.field.push(byte);
                if self.field.len() == 8 {
                    let [x, y, width, height] = [0, 2, 4, 6].map(|i| u16::from_le_bytes([self.field[i], self.field[i + 1]]));
                    let pixels = width as usize * height as usize;
                    self.field.clear();
                    if pixels == 0 || pixels > self.max_pixels {
                        self.state = State::Magic(0);
                    } else {
                        self.frame = Frame { x, y, width, height, pixels: Vec::with_capacity(pixels) };
                        self.state = State::Piece;
                    }
                }
                None
            }
            State::Piece | State::Run(_) | State::Literal(_) => {
                self.bytes.push(byte);
                self.field.push(byte);
                if self.field.len() < 2 {
                    return None;
                }
                let value = u16::from_le_bytes([self.field[0], self.field[1]]);
                self.field.clear();
                let missing = self.frame.width as usize * self.frame.height as usize - self.frame.pixels.len();
                self.state = match self.state {
                    State::Piece => {
                        let count = usize::from(value & !LITERAL);
                        if count == 0 || count > missing {
                            State::Magic(0)
                        } else if value & LITERAL != 0 {
                            State::Literal(count)
                        } else {
                            State::Run(count)
                        }
                    }
                    State::Run(count) => {
                        self.frame.pixels.extend(std::iter::repeat_n(value, count));
                        if count == missing { State::Sum } else { State::Piece }
                    }
                    State::Literal(left) => {
                        self.frame.pixels.push(value);
                        match (left, missing) {
                            (_, 1) => State::Sum,
                            (1, _) => State::Piece,
                            _ => State::Literal(left - 1),
                        }
                    }
                    _ => unreachable!(),
                };
                None
            }
            State::Sum => {
                self.field.push(byte);
                if self.field.len() < 2 {
                    return None;
                }
                let sum = u16::from_le_bytes([self.field[0], self.field[1]]);
                self.field.clear();
                self.state = State::Magic(0);
                let frame = std::mem::replace(&mut self.frame, Frame { x: 0, y: 0, width: 0, height: 0, pixels: Vec::new() });
                (sum == checksum(&self.bytes)).then_some(frame)
            }
        }
    }
}

/// What the board shows, so that of each band only the rectangle around the pixels that
/// changed is sent.
pub struct Shadow {
    width: usize,
    /// Row by row.
    pixels: Vec<u16>,
    /// Whether the board shows the row as kept here; none after a reset.
    known: Vec<bool>,
}

impl Shadow {
    pub fn new(width: u16, height: u16) -> Self {
        let (width, height) = (usize::from(width), usize::from(height));
        Shadow { width, pixels: vec![0; width * height], known: vec![false; height] }
    }

    /// The board shows nothing known any more: send everything again.
    pub fn forget(&mut self) {
        self.known.fill(false);
    }

    /// Takes the `width` × `height` rectangle at (`x`, `y`) and returns the frame to send for
    /// it: the smallest rectangle around what changed, or `None` if nothing did.
    pub fn update(&mut self, x: u16, y: u16, width: u16, height: u16, pixels: &[u16]) -> Option<Vec<u8>> {
        let (x0, y0, w, h) = (usize::from(x), usize::from(y), usize::from(width), usize::from(height));
        // The bounds of the changed pixels.
        let (mut left, mut right, mut top, mut bottom) = (usize::MAX, 0, usize::MAX, 0);
        for row in 0..h {
            let new = &pixels[row * w..(row + 1) * w];
            let start = (y0 + row) * self.width + x0;
            let old = &mut self.pixels[start..start + w];
            let (first, last) = if self.known[y0 + row] {
                let Some(first) = new.iter().zip(old.iter()).position(|(a, b)| a != b) else { continue };
                let from_end = new.iter().rev().zip(old.iter().rev()).position(|(a, b)| a != b).unwrap_or(0);
                (first, w - 1 - from_end)
            } else {
                (0, w - 1)
            };
            old.copy_from_slice(new);
            left = left.min(first);
            right = right.max(last);
            top = top.min(row);
            bottom = bottom.max(row);
        }
        // A row is known once a whole band covering it went out; the renderer sends whole
        // bands, so after the first frame every row is.
        if x0 == 0 && w == self.width {
            self.known[y0..y0 + h].fill(true);
        }
        if top == usize::MAX {
            return None;
        }
        let changed = (top..=bottom).flat_map(|row| pixels[row * w + left..=row * w + right].iter().copied());
        Some(encode(
            (x0 + left) as u16,
            (y0 + top) as u16,
            (right - left + 1) as u16,
            (bottom - top + 1) as u16,
            changed,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(bytes: &[u8]) -> Vec<Frame> {
        let mut decoder = Decoder::new(480 * 32);
        bytes.iter().filter_map(|&byte| decoder.push(byte)).collect()
    }

    fn band() -> Frame {
        let pixels = (0..480 * 32).map(|i| if i % 480 < 100 { 0x1234 } else { (i / 7) as u16 }).collect();
        Frame { x: 0, y: 32, width: 480, height: 32, pixels }
    }

    fn encoded(frame: &Frame) -> Vec<u8> {
        encode(frame.x, frame.y, frame.width, frame.height, frame.pixels.iter().copied())
    }

    #[test]
    fn a_frame_comes_back_as_it_was() {
        let frame = band();
        assert_eq!(decode(&encoded(&frame)), vec![frame]);
    }

    #[test]
    fn a_picture_costs_two_bytes_a_pixel() {
        let pixels: Vec<u16> = (0..480 * 32).map(|i| (i * 7919) as u16).collect();
        let frame = Frame { x: 0, y: 0, width: 480, height: 32, pixels };
        let bytes = encoded(&frame);
        assert!(bytes.len() < 2 * 480 * 32 + 64, "{} bytes", bytes.len());
        assert_eq!(decode(&bytes), vec![frame]);
    }

    #[test]
    fn runs_and_literals_mix() {
        let mut pixels = vec![1, 2, 3, 3, 3, 3, 4, 4, 5];
        pixels.resize(480, 9);
        let frame = Frame { x: 0, y: 0, width: 480, height: 1, pixels };
        assert_eq!(decode(&encoded(&frame)), vec![frame]);
    }

    #[test]
    fn only_what_changed_is_sent_again() {
        let mut shadow = Shadow::new(480, 320);
        let mut band = band();
        // Black, as the shadow starts: still sent the first time.
        band.pixels[0] = 0;
        let first = shadow.update(band.x, band.y, band.width, band.height, &band.pixels).unwrap();
        assert_eq!(decode(&first), vec![band.clone()]);
        assert_eq!(shadow.update(band.x, band.y, band.width, band.height, &band.pixels), None);

        // Two pixels change: the rectangle between them goes out.
        band.pixels[3 * 480 + 10] = 0xFFFF;
        band.pixels[5 * 480 + 20] = 0xFFFF;
        let frames = decode(&shadow.update(band.x, band.y, band.width, band.height, &band.pixels).unwrap());
        assert_eq!((frames[0].x, frames[0].y, frames[0].width, frames[0].height), (10, 35, 11, 3));
        assert_eq!(frames[0].pixels[0], 0xFFFF);

        // After a reset, everything again.
        shadow.forget();
        assert_eq!(decode(&shadow.update(band.x, band.y, band.width, band.height, &band.pixels).unwrap()), vec![band]);
    }

    #[test]
    fn a_flat_band_is_a_single_run() {
        let bytes = encode(0, 0, 480, 32, std::iter::repeat_n(7, 480 * 32));
        assert_eq!(bytes.len(), 2 + 8 + 4 + 2);
    }

    #[test]
    fn a_run_longer_than_a_u16_is_split() {
        let frame = Frame { x: 0, y: 0, width: 480, height: 320, pixels: vec![5; 480 * 320] };
        let mut decoder = Decoder::new(480 * 320);
        let frames: Vec<Frame> = encoded(&frame).iter().filter_map(|&byte| decoder.push(byte)).collect();
        assert_eq!(frames, vec![frame]);
    }

    #[test]
    fn a_boot_log_before_the_frame_is_skipped() {
        let frame = band();
        let mut bytes = b"rst:0x1 (POWERON_RESET),boot:0x13 \xA5 READY\n".to_vec();
        bytes.extend(encoded(&frame));
        assert_eq!(decode(&bytes), vec![frame]);
    }

    #[test]
    fn a_damaged_frame_is_dropped_and_the_next_one_read() {
        let frame = band();
        let mut damaged = encoded(&frame);
        let middle = damaged.len() / 2;
        damaged[middle] ^= 0x01;
        let mut bytes = damaged;
        bytes.extend(encoded(&frame));
        assert_eq!(decode(&bytes), vec![frame]);
    }

    #[test]
    fn a_too_large_rectangle_is_skipped() {
        let mut bytes = encode(0, 0, 480, 320, std::iter::repeat_n(1, 480 * 320));
        let frame = band();
        bytes.extend(encoded(&frame));
        assert_eq!(decode(&bytes), vec![frame]);
    }
}
