//! A walk through the UI as a person would take it, with a wallpaper behind everything: it
//! picks the first picture on the SD card and shows it everywhere, browses the SETUP pages, the
//! mode tiles and OPTN, works a few calculations and sketches a curve. The simulator plays it with `--tour`, in
//! the window and on the board with `--mirror`; `tests/tour.rs` plays it without the pauses.

use crate::key::Key;

/// A key, after a pause.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    /// How long a person would take before pressing it.
    pub pause_ms: u32,
    pub key: Key,
    /// What the steps from here on do, for the log.
    pub note: Option<&'static str>,
}

/// The walk, start to end. Starts on the calculation screen with the settings as they are on
/// a fresh start, and ends on the mode tiles.
pub fn tour() -> Vec<Step> {
    use Key::*;
    let mut tour = Tour { steps: Vec::new(), seed: 0x2545_f491, note: None, extra: 0 };

    tour.note("SETUP → Gerät → Hintergrundbild: the first picture, shown everywhere");
    tour.read(1500);
    tour.press(&[Shift, Mode]);
    tour.look(&[Down, Down, Down, Down, Down]);
    tour.press(&[Ok]);
    tour.look(&[Down, Down]);
    tour.press(&[Ok, Ok]);
    // Through the pictures and back to the first one, which stays.
    tour.look(&[Down, Down, Up]);
    tour.read(1200);
    tour.press(&[Ok]);
    tour.look(&[Down]);
    tour.press(&[Ok]);
    tour.look(&[Down, Down]);
    tour.press(&[Ok]);
    tour.read(1500);

    tour.note("The SETUP pages");
    // Back past Gerät to the list of pages.
    tour.press(&[Left, Left]);
    // From Gerät up to Allgemein, then into each page and back.
    tour.look(&[Up, Up, Up, Up, Up]);
    for page in 0..5 {
        tour.press(&[Ok]);
        tour.look(&[Down, Down, Down]);
        tour.read(900);
        tour.press(&[Left]);
        if page < 4 {
            tour.look(&[Down]);
        }
    }
    // Verbindungen, on the second page of the list.
    tour.look(&[Down, Down]);
    tour.press(&[Ok]);
    tour.look(&[Down, Down]);
    tour.read(900);
    tour.press(&[Ac]);

    tour.note("The mode tiles, all three pages, then Algebra");
    tour.press(&[Mode]);
    tour.read(1200);
    tour.look(&[Right, Right, Down, Down, Up, Left, Left, Down, Down, Up, Up, Up, Right]);
    tour.read(800);
    tour.press(&[Ok]);

    tour.note("Algebra: 2x + 3 = 11");
    tour.read(800);
    tour.typed(&[Digit(2), Letter('x'), Plus, Digit(3), Equals, Digit(1), Digit(1)]);
    tour.result();

    tour.note("Algebra: x² − 5x + 6 = 0");
    tour.press(&[Ac]);
    tour.typed(&[Letter('x'), Power, Digit(2), Right, Minus, Digit(5), Letter('x'), Plus, Digit(6), Equals, Digit(0)]);
    tour.result();

    tour.note("Algebra: 3/4 + 1/6, then as a decimal and back");
    tour.press(&[Ac]);
    tour.typed(&[Fraction, Digit(3), Down, Digit(4), Right, Plus, Fraction, Digit(1), Down, Digit(6), Right]);
    tour.result();
    tour.press(&[SwapForm]);
    tour.read(1500);
    tour.press(&[SwapForm]);
    tour.read(1000);

    tour.note("The history, back and forth");
    tour.look(&[Up, Up, Down, Down]);
    tour.read(1000);

    tour.note("OPTN in Algebra");
    tour.press(&[Optn]);
    tour.look(&[Down, Down, Up]);
    tour.read(1000);
    tour.press(&[Ac]);

    tour.note("LGS: x + y + z = 6, x − y = 0, 2z = 6 in the matrix, its solution and steps");
    tour.press(&[Mode]);
    tour.read(700);
    tour.press(&[Digit(5)]);
    tour.read(1000);
    tour.typed(&[Digit(1), Exe, Digit(1), Exe, Digit(1), Exe, Digit(6), Exe]);
    tour.typed(&[Digit(1), Exe, Minus, Digit(1), Exe]);
    // The empty cells are 0: past them to the third row.
    tour.look(&[Right, Down, Left]);
    tour.typed(&[Digit(2), Exe, Digit(6), Exe]);
    tour.result();
    tour.look(&[Down, Down, Down, Down]);
    tour.read(1200);
    tour.press(&[Ac]);

    tour.note("Kurvendiskussion: x³ − 3x, the graph and some sections");
    tour.press(&[Mode]);
    tour.read(700);
    tour.press(&[Digit(4)]);
    tour.read(800);
    tour.typed(&[Letter('x'), Power, Digit(3), Right, Minus, Digit(3), Letter('x')]);
    tour.result();
    tour.read(1200);
    tour.note("Kurvendiskussion: the graph moved and zoomed, then the sections, result first");
    tour.press(&[Ok]);
    tour.look(&[Right, Right, Plus]);
    tour.read(1200);
    tour.look(&[Digit(0)]);
    tour.press(&[Ok]);
    tour.look(&[Right]);
    tour.read(1500);
    tour.look(&[Right, Right, Right, Right, Right]);
    tour.read(1200);
    tour.look(&[Down, Down]);
    tour.read(1200);
    tour.look(&[Right, Right]);
    tour.read(1500);
    tour.press(&[Ac]);

    tour.note("Kurvendiskussion: the tangent at x₀ = 2, on the graph");
    tour.press(&[Optn]);
    tour.read(800);
    tour.press(&[Digit(2)]);
    tour.typed(&[Digit(2), Exe]);
    tour.result();
    tour.look(&[Down, Down, Down]);
    tour.press(&[Right]);
    tour.read(2000);
    tour.press(&[Ac, Ac]);

    tour.note("Physics: 3,5 × 4 and √2");
    tour.press(&[Mode]);
    tour.read(700);
    tour.press(&[Digit(1)]);
    tour.press(&[Ac]);
    tour.typed(&[Digit(3), Comma, Digit(5), Times, Digit(4)]);
    tour.result();
    tour.press(&[Ac]);
    tour.typed(&[Root, Digit(2), Right]);
    tour.result();

    tour.note("Back to the mode tiles");
    tour.press(&[Mode]);
    tour.read(2000);
    tour.steps
}

struct Tour {
    steps: Vec<Step>,
    /// For the pauses: people are not metronomes.
    seed: u32,
    note: Option<&'static str>,
    /// Reading time, added to the pause before the next key.
    extra: u32,
}

impl Tour {
    fn note(&mut self, note: &'static str) {
        self.note = Some(note);
    }

    /// A pause between `min` and `max` ms (xorshift).
    fn pause(&mut self, min: u32, max: u32) -> u32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        min + self.seed % (max - min + 1)
    }

    fn key(&mut self, key: Key, min: u32, max: u32) {
        let pause_ms = self.pause(min, max) + std::mem::take(&mut self.extra);
        self.steps.push(Step { pause_ms, key, note: self.note.take() });
    }

    /// Keys that open, choose or leave something.
    fn press(&mut self, keys: &[Key]) {
        keys.iter().for_each(|&key| self.key(key, 600, 1100));
    }

    /// Cursor keys while looking around.
    fn look(&mut self, keys: &[Key]) {
        keys.iter().for_each(|&key| self.key(key, 350, 700));
    }

    /// Typing an input.
    fn typed(&mut self, keys: &[Key]) {
        keys.iter().for_each(|&key| self.key(key, 150, 350));
    }

    /// EXE, after a look at the input, then a while to read the result.
    fn result(&mut self) {
        self.key(Key::Exe, 500, 900);
        self.read(1800);
    }

    /// Time to read the screen before the next key.
    fn read(&mut self, ms: u32) {
        self.extra += ms;
    }
}
