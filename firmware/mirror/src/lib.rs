//! Testing only: the simulator on the PC sends its picture over serial and the board shows it,
//! so the display can be tried with the real engine before that runs on the board.
//!
//! A frame is one rectangle of RGB565 pixels, row by row, run-length encoded:
//!
//! | bytes | content |
//! |---|---|
//! | 2 | magic `A5 5A` |
//! | 8 | `x`, `y`, `width`, `height`, each u16 little-endian |
//! | 4 each | runs: `count` (1 or more) and `color`, each u16 little-endian, until the rectangle is full |
//! | 2 | the wrapping sum of the header and run bytes, u16 little-endian |
//!
//! The decoder skips anything before a magic (such as the board's boot log) and drops a frame
//! whose sum does not match.

pub const MAGIC: [u8; 2] = [0xA5, 0x5A];

/// The frame for the `width` × `height` rectangle at (`x`, `y`) with `pixels`, row by row.
pub fn encode(x: u16, y: u16, width: u16, height: u16, pixels: impl IntoIterator<Item = u16>) -> Vec<u8> {
    let mut frame = MAGIC.to_vec();
    for value in [x, y, width, height] {
        frame.extend(value.to_le_bytes());
    }
    let mut run: Option<(u16, u16)> = None;
    let push = |frame: &mut Vec<u8>, (count, color): (u16, u16)| {
        frame.extend(count.to_le_bytes());
        frame.extend(color.to_le_bytes());
    };
    for color in pixels {
        run = match run {
            Some((count, current)) if current == color && count < u16::MAX => Some((count + 1, color)),
            Some(done) => {
                push(&mut frame, done);
                Some((1, color))
            }
            None => Some((1, color)),
        };
    }
    if let Some(done) = run {
        push(&mut frame, done);
    }
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
    Runs,
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
                        self.state = State::Runs;
                    }
                }
                None
            }
            State::Runs => {
                self.bytes.push(byte);
                self.field.push(byte);
                if self.field.len() == 4 {
                    let count = u16::from_le_bytes([self.field[0], self.field[1]]) as usize;
                    let color = u16::from_le_bytes([self.field[2], self.field[3]]);
                    self.field.clear();
                    let missing = self.frame.width as usize * self.frame.height as usize - self.frame.pixels.len();
                    if count == 0 || count > missing {
                        self.state = State::Magic(0);
                    } else {
                        self.frame.pixels.extend(std::iter::repeat_n(color, count));
                        if count == missing {
                            self.state = State::Sum;
                        }
                    }
                }
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
