//! The screen the UI is drawn for. The layout works on a logical canvas of `width` × `height`
//! pixels; the renderer scales it by `num / den` to the panel's pixels, with fonts of the scaled
//! size, so a panel with more pixels per millimetre shows the same layout as sharply as it can
//! (firmware/docs/duo.md §2). One profile for the thread that draws, the handheld's until
//! [`set`] gives another, as the palette in [`crate::render::colors`].

use std::cell::Cell;

use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Profile {
    /// The logical canvas the layout works on.
    pub width: u32,
    pub height: u32,
    /// Logical pixels to the panel's: `num / den`.
    num: u32,
    den: u32,
}

impl Profile {
    /// The handheld's 3.5" 480×320 screen, drawn as laid out.
    pub const HANDHELD: Profile = Profile { width: 480, height: 320, num: 1, den: 1 };
    /// The Duo's 5" main screen, 1280×720 at about 1.78 times the handheld's pixels per
    /// millimetre: a logical 720×405, so the same layout is the same size and shows more.
    pub const DUO: Profile = Profile { width: 720, height: 405, num: 16, den: 9 };

    /// Whether the panel's pixels are the layout's.
    pub fn is_unit(&self) -> bool {
        self.num == self.den
    }

    /// The panel's size.
    pub fn physical(&self) -> Size {
        Size::new(self.width * self.num / self.den, self.height * self.num / self.den)
    }

    /// A logical coordinate in the panel's pixels.
    pub fn scale(&self, value: i32) -> i32 {
        (value * self.num as i32).div_euclid(self.den as i32)
    }

    /// A logical length in the panel's pixels, at least one where it was.
    pub fn length(&self, value: u32) -> u32 {
        if value == 0 { 0 } else { (value * self.num / self.den).max(1) }
    }

    pub fn point(&self, point: Point) -> Point {
        Point::new(self.scale(point.x), self.scale(point.y))
    }

    /// A logical rectangle in the panel's pixels: from its scaled top left to its scaled bottom
    /// right, so neighbouring rectangles still meet.
    pub fn rect(&self, rect: Rectangle) -> Rectangle {
        let top_left = self.point(rect.top_left);
        let end = self.point(rect.top_left + rect.size);
        Rectangle::new(top_left, Size::new((end.x - top_left.x).max(0) as u32, (end.y - top_left.y).max(0) as u32))
    }
}

thread_local! {
    static CURRENT: Cell<Profile> = const { Cell::new(Profile::HANDHELD) };
}

/// The profile in force on this thread.
pub fn get() -> Profile {
    CURRENT.with(Cell::get)
}

/// Draws for `profile` from now on, on this thread.
pub fn set(profile: Profile) {
    CURRENT.with(|current| current.set(profile));
}

/// The logical canvas's width.
pub fn width() -> u32 {
    get().width
}

/// The logical canvas's height.
pub fn height() -> u32 {
    get().height
}
