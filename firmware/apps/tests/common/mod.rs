//! What the games' tests share.

use quantus_apps::api::Draw;
use quantus_ui::applet;

/// Whether everything `draws` drew lies inside an area of `width` × `height`.
pub fn inside(draws: &[Draw], width: i32, height: i32) -> Result<(), String> {
    let within = |x: i32, y: i32, w: i32, h: i32| x >= 0 && y >= 0 && x + w <= width && y + h <= height;
    for draw in draws {
        let ok = match draw {
            Draw::Clear(_) => true,
            &Draw::Rect(x, y, w, h, _) | &Draw::Round(x, y, w, h, _, _) | &Draw::Frame(x, y, w, h, _, _) => within(x, y, w, h),
            &Draw::Circle(x, y, d, _) => within(x, y, d, d),
            &Draw::Line(x1, y1, x2, y2, _, _) => within(x1.min(x2), y1.min(y2), (x2 - x1).abs(), (y2 - y1).abs()),
            Draw::Text(x, y, text, _, small) => {
                let (w, h) = if *small {
                    (applet::text_small_width(text), applet::text_small_height())
                } else {
                    (applet::text_width(text), applet::text_height())
                };
                within(*x, *y, w, h)
            }
            Draw::Math(..) => true,
        };
        if !ok {
            return Err(format!("{draw:?} outside {width} × {height}"));
        }
    }
    Ok(())
}

