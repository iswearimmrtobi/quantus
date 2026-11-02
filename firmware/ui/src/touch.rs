//! Touch: what the panel reports, what a tap hits, and the calibration that turns the panel's
//! readings into screen points. A tap does what the keys do, so the menus work with the finger
//! alone; a list follows the finger while it is dragged; numbers are stepped with − and +, there
//! is no keyboard on the screen (design/handheld-menus.md §7.7).

use embedded_graphics::prelude::Point;

/// What the board or the simulator reports: the finger comes down, moves, lifts. The points are
/// the panel's readings; [`Calibration`] turns them into screen points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Touch {
    Press(Point),
    Drag(Point),
    Release,
}

/// What lies under a tap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// The left of the top bar: back one level.
    Back,
    /// A line of a menu list, counted on its page.
    Line(usize),
    /// A mode tile, counted over all pages.
    Tile(usize),
    /// A page dot of the mode menu.
    Page(usize),
    /// − or + beside a number.
    Step(i32),
    /// OK under a number.
    Ok,
    Button(Button),
    /// Anywhere on a message.
    Anywhere,
}

/// The buttons on the calculation screen, for testing until the keypad exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Menu,
    Setup,
    Optn,
}

/// What the finger did, once it lifted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gesture {
    /// Where it came down.
    Tap(Point),
    /// How far it moved from there.
    Drag(Point),
}

/// A movement shorter than this is a tap; a resistive panel jitters by a few pixels.
pub const SLOP: i32 = 20;

/// Where the finger came down, where it is now, and whether it has moved far enough to drag.
#[derive(Clone, Copy, Debug, Default)]
pub struct Tracker {
    start: Option<Point>,
    last: Point,
    dragging: bool,
}

impl Tracker {
    /// The gesture that ends with this report, if one does.
    pub fn feed(&mut self, touch: Touch) -> Option<Gesture> {
        match touch {
            Touch::Press(point) => {
                *self = Tracker { start: Some(point), last: point, dragging: false };
                None
            }
            Touch::Drag(point) => {
                self.last = point;
                let moved = self.moved()?;
                self.dragging |= moved.x.abs().max(moved.y.abs()) >= SLOP;
                None
            }
            Touch::Release => {
                let start = self.start.take()?;
                let moved = self.last - start;
                Some(if std::mem::take(&mut self.dragging) { Gesture::Drag(moved) } else { Gesture::Tap(start) })
            }
        }
    }

    /// Where the finger came down, while it is down.
    pub fn start(&self) -> Option<Point> {
        self.start
    }

    /// How far the finger has moved, while it drags.
    pub fn drag(&self) -> Option<Point> {
        self.dragging.then(|| self.moved()).flatten()
    }

    fn moved(&self) -> Option<Point> {
        Some(self.last - self.start?)
    }
}

/// Panel readings to screen points: `x = a·rx + b·ry + c`, `y = d·rx + e·ry + f`. An affine map
/// covers the axes being swapped, turned or mirrored as well as their scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Calibration(pub [f32; 6]);

impl Default for Calibration {
    /// The readings are screen points already, as in the simulator.
    fn default() -> Self {
        Calibration([1.0, 0.0, 0.0, 0.0, 1.0, 0.0])
    }
}

/// Further than this from a target, in pixels, and the calibration is done again.
pub const CALIBRATION_ERROR: f32 = 20.0;

impl Calibration {
    pub fn map(&self, raw: Point) -> Point {
        let [a, b, c, d, e, f] = self.0;
        let (x, y) = (raw.x as f32, raw.y as f32);
        Point::new((a * x + b * y + c).round() as i32, (d * x + e * y + f).round() as i32)
    }

    /// The map that fits the readings to their targets best, by least squares, and the largest
    /// distance in pixels that is left; `None` if the readings do not span the screen.
    pub fn fit(pairs: &[(Point, Point)]) -> Option<(Calibration, f32)> {
        // f64: the sums of squares of 12-bit readings are too large for an f32's precision.
        let mut m = [[0.0f64; 3]; 3];
        let (mut vx, mut vy) = ([0.0f64; 3], [0.0f64; 3]);
        for (raw, screen) in pairs {
            let row = [f64::from(raw.x), f64::from(raw.y), 1.0];
            for i in 0..3 {
                for j in 0..3 {
                    m[i][j] += row[i] * row[j];
                }
                vx[i] += row[i] * f64::from(screen.x);
                vy[i] += row[i] * f64::from(screen.y);
            }
        }
        let [a, b, c] = solve(m, vx)?;
        let [d, e, f] = solve(m, vy)?;
        let calibration = Calibration([a, b, c, d, e, f].map(|value| value as f32));
        let error = pairs
            .iter()
            .map(|&(raw, screen)| {
                let off = calibration.map(raw) - screen;
                ((off.x * off.x + off.y * off.y) as f32).sqrt()
            })
            .fold(0.0, f32::max);
        Some((calibration, error))
    }

    /// For storing it: six little-endian f32s.
    pub fn to_bytes(&self) -> [u8; 24] {
        let mut bytes = [0; 24];
        for (index, value) in self.0.iter().enumerate() {
            bytes[4 * index..4 * index + 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Calibration> {
        if bytes.len() != 24 {
            return None;
        }
        let mut values = [0.0; 6];
        for (index, value) in values.iter_mut().enumerate() {
            *value = f32::from_le_bytes(bytes[4 * index..4 * index + 4].try_into().ok()?);
        }
        values.iter().all(|value| value.is_finite()).then_some(Calibration(values))
    }
}

/// `m · x = v` by Cramer's rule; `None` if `m` is singular.
fn solve(m: [[f64; 3]; 3], v: [f64; 3]) -> Option<[f64; 3]> {
    let det = |m: [[f64; 3]; 3]| {
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    };
    let whole = det(m);
    if whole.abs() < 1e-6 {
        return None;
    }
    let mut x = [0.0; 3];
    for (column, x) in x.iter_mut().enumerate() {
        let mut replaced = m;
        for row in 0..3 {
            replaced[row][column] = v[row];
        }
        *x = det(replaced) / whole;
    }
    Some(x)
}

/// The calibration screen: a cross at each target in turn, held until the finger lifts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Calibrating {
    /// The readings taken, one per target so far.
    pub readings: Vec<Point>,
    /// The readings of the press under way: their sum and how many.
    pub sum: (i32, i32, i32),
    /// The last round was too far off and is being done again.
    pub retry: bool,
}

/// The targets, in turn: the four corners, inset, and the middle.
pub const TARGETS: [Point; 5] =
    [Point::new(40, 50), Point::new(440, 50), Point::new(440, 280), Point::new(40, 280), Point::new(240, 165)];
