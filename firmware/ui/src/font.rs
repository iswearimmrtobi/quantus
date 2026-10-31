//! Text in two sizes: normal for the input and results, small for exponents, indices and the
//! status bar. Each size is a Latin-1 font (FreeUniversal) with a Greek and a symbol font behind
//! it for the characters it lacks.

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use u8g2_fonts::types::{FontColor, VerticalPosition};
use u8g2_fonts::{fonts, FontRenderer};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scale {
    Normal,
    Small,
}

static NORMAL: [FontRenderer; 3] = [
    FontRenderer::new::<fonts::u8g2_font_fur20_tf>(),
    FontRenderer::new::<fonts::u8g2_font_10x20_t_greek>(),
    FontRenderer::new::<fonts::u8g2_font_fur20_t_symbol>(),
];

static SMALL: [FontRenderer; 3] = [
    FontRenderer::new::<fonts::u8g2_font_fur14_tf>(),
    FontRenderer::new::<fonts::u8g2_font_cu12_t_greek>(),
    FontRenderer::new::<fonts::u8g2_font_fur14_t_symbol>(),
];

/// Vertical measures of a size, in pixels from the baseline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Metrics {
    /// Above the baseline, for the tallest letters.
    pub ascent: i32,
    /// Below the baseline, for descenders.
    pub descent: i32,
    /// Height of a digit.
    pub digit: i32,
    /// Height of the math axis: the middle of a minus sign, where fraction bars sit.
    pub axis: i32,
}

impl Scale {
    fn fonts(self) -> &'static [FontRenderer; 3] {
        match self {
            Scale::Normal => &NORMAL,
            Scale::Small => &SMALL,
        }
    }

    pub fn metrics(self) -> Metrics {
        let main = &self.fonts()[0];
        let digit = main
            .get_rendered_dimensions('0', Point::zero(), VerticalPosition::Baseline)
            .ok()
            .and_then(|dimensions| dimensions.bounding_box)
            .map_or(i32::from(main.get_ascent()), |bounds| bounds.size.height as i32);
        Metrics {
            ascent: i32::from(main.get_ascent()),
            descent: -i32::from(main.get_descent()),
            digit,
            axis: digit * 2 / 5,
        }
    }

    /// The first of the size's fonts that has `c`.
    fn font_for(self, c: char) -> Option<&'static FontRenderer> {
        self.fonts().iter().find(|font| {
            font.get_rendered_dimensions(c, Point::zero(), VerticalPosition::Baseline).is_ok()
        })
    }

    /// How far the text advances, in pixels. Characters no font has count as nothing.
    pub fn width(self, text: &str) -> i32 {
        text.chars()
            .filter_map(|c| {
                let font = self.font_for(c)?;
                let dimensions =
                    font.get_rendered_dimensions(c, Point::zero(), VerticalPosition::Baseline);
                dimensions.ok().map(|dimensions| dimensions.advance.x)
            })
            .sum()
    }

    /// Draws `text` with its baseline at `origin.y`.
    pub fn draw<D: DrawTarget<Color = Rgb565>>(
        self,
        text: &str,
        origin: Point,
        color: Rgb565,
        target: &mut D,
    ) {
        let mut at = origin;
        for c in text.chars() {
            let Some(font) = self.font_for(c) else { continue };
            let drawn =
                font.render(c, at, VerticalPosition::Baseline, FontColor::Transparent(color), target);
            if let Ok(dimensions) = drawn {
                at.x += dimensions.advance.x;
            }
        }
    }
}
