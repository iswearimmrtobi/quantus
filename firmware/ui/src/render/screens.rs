//! The screens under the top bar: one calculation, the mode tiles, the menu lists and the
//! number entry, the storage page and the touch calibration.

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, Rectangle, RoundedRectangle, Triangle};

use super::{
    colors, dot_rect, fit, line_rect, list_edge, number_button, panel, rounded, tile_rect, Shape, BAR,
    HEIGHT, LINE, LINE_GAP, MARGIN, NUMBER_BUTTONS, THUMBNAIL, WIDTH,
};
use crate::app::{App, Mode, TILES_PER_PAGE};
use crate::backend::{Storage, WlanStatus};
use crate::editor::Row;
use crate::font::Scale;
use crate::layout;
use crate::menu::{self, Edit, Kind, Menu, TextFor, ROWS};
use crate::text;
use crate::touch::{self, Calibrating, Target};

/// One calculation, as on a Casio: the input from the top left, its result at the bottom right.
pub(super) fn calculation<B>(shapes: &mut Vec<Shape>, app: &App<B>) {
    let width = WIDTH as i32;
    let scale = app.settings.font_size.scale();

    let input = layout::layout(&app.editor.row, scale);
    let mut origin = Point::new(MARGIN, BAR + 12 + input.ascent);
    let caret = layout::caret(&input, origin, app.editor.path(), app.editor.pos());
    // Scroll a long input so the caret stays on the screen.
    let overflow = caret.top_left.x - (width - MARGIN);
    if overflow > 0 {
        origin.x -= overflow;
    }
    let caret = caret.translate(Point::new(origin.x - MARGIN, 0));
    // The selection, from its first node to its last, under the input.
    if let Some(range) = app.editor.selection() {
        let at = |pos| layout::caret(&input, origin, app.editor.path(), pos);
        let (from, to) = (at(range.start), at(range.end));
        let selected = Rectangle::new(from.top_left, Size::new((to.top_left.x - from.top_left.x).max(0) as u32 + 2, from.size.height));
        shapes.push(Shape::Fill(selected, colors::primary_dim()));
    }
    if !app.editor.is_empty() {
        shapes.push(Shape::Math(origin, input, colors::text()));
    }
    if app.caret_on && app.shown.is_none() {
        shapes.push(Shape::Fill(caret, colors::primary()));
    }

    // Above the soft keys' labels, while they show.
    let soft = if crate::soft::shows(app) { super::SOFT } else { 0 };
    let bottom = HEIGHT as i32 - MARGIN - soft;
    if let Some(entry) = app.shown_entry() {
        match &entry.result {
            Ok(result) => {
                let result = layout::layout(result, scale);
                let x = (width - MARGIN - result.width).max(MARGIN);
                shapes.push(Shape::Math(Point::new(x, bottom - result.descent), result, colors::result()));
            }
            Err(message) => {
                let metrics = scale.metrics();
                let x = (width - MARGIN - scale.width(message)).max(MARGIN);
                let at = Point::new(x, bottom - metrics.descent);
                shapes.push(Shape::Text(at, message.to_string(), scale, colors::error()));
            }
        }
    }

    // How many calculations ↑ can bring back.
    let earlier = app.history.len() - app.shown.map_or(0, |shown| shown + 1);
    if earlier > 0 {
        let baseline = bottom - 2;
        let tip = Point::new(MARGIN + 5, baseline - 10);
        let arrow = Triangle::new(tip, Point::new(MARGIN, baseline), Point::new(MARGIN + 10, baseline));
        shapes.push(Shape::Triangle(arrow, colors::faint()));
        let count = earlier.to_string();
        shapes.push(Shape::Text(Point::new(MARGIN + 15, baseline), count, Scale::Small, colors::faint()));
    }
}

/// The mode menu: pages of tiles, numbered over all pages, the apps with a tile of their own
/// after the modes. While a finger drags them, the page moves with it and its neighbour comes in.
pub(super) fn modes<B>(shapes: &mut Vec<Shape>, app: &App<B>, selected: usize, glass: bool) {
    let lang = app.settings.language;
    // Each tile's name on one line, and with the places it may break at.
    let names: Vec<(String, String)> = Mode::ALL
        .iter()
        .map(|mode| (mode.name(lang), mode.text().get(lang).replace('\u{ad}', "-\u{ad}")))
        .chain(app.app_tiles.iter().map(|info| (info.name.clone(), info.name.clone())))
        .collect();
    let count = super::pages(names.len());
    let page = selected / TILES_PER_PAGE;
    let dx = app.drag().map_or(0, |drag| drag.x);
    let pages = if dx == 0 { page..=page } else { page.saturating_sub(1)..=(page + 1).min(count - 1) };
    for shown in pages {
        let shift = Point::new((shown as i32 - page as i32) * WIDTH as i32 + dx, 0);
        tiles(shapes, &names, shown, selected, app.pressed(), shift, glass);
    }

    if count > 1 {
        for index in 0..count {
            let color = if index == page { colors::text() } else { colors::faint() };
            let dot = dot_rect(index, count);
            shapes.push(Shape::Circle(Circle::new(dot.top_left, dot.size.width), color));
        }
    }
}

/// The tiles of mode page `page`, moved by `shift`, of the tiles `names`.
fn tiles(
    shapes: &mut Vec<Shape>,
    names: &[(String, String)],
    page: usize,
    selected: usize,
    pressed: Option<Target>,
    shift: Point,
    glass: bool,
) {
    let first = page * TILES_PER_PAGE;
    let on_page = names.get(first..(first + TILES_PER_PAGE).min(names.len())).unwrap_or_default();
    let small = Scale::Small.metrics();
    for (offset, (name, breakable)) in on_page.iter().enumerate() {
        let index = first + offset;
        let Rectangle { top_left: at, size } = tile_rect(offset).translate(shift);
        let (tile_width, tile_height) = (size.width as i32, size.height as i32);
        let chosen = index == selected;
        let (tile, lit) = colors::tiles()[index % colors::tiles().len()];
        let fill = if chosen || pressed == Some(Target::Tile(index)) { lit } else { tile };
        shapes.push(panel(rounded(at, size, 6), fill, chosen.then_some(colors::text()), glass));
        let number = (index + 1).to_string();
        shapes.push(Shape::Text(at + Point::new(12, 10 + small.ascent), number, Scale::Small, colors::text()));
        // The name at the bottom, a word per line when it is too wide, and small when a word is.
        let room = tile_width - 18;
        let lines: Vec<String> = if Scale::Normal.width(name) <= room {
            vec![name.clone()]
        } else {
            // At the spaces, and at the soft hyphens with a hyphen.
            breakable.split([' ', '\u{ad}']).map(str::to_string).collect()
        };
        let fits = lines.iter().all(|line| Scale::Normal.width(line) <= room);
        let scale = if fits { Scale::Normal } else { Scale::Small };
        let metrics = scale.metrics();
        let line_height = metrics.ascent + metrics.descent - 2;
        let mut baseline = at.y + tile_height - 12 - metrics.descent;
        for line in lines.iter().rev() {
            shapes.push(Shape::Text(Point::new(at.x + 12, baseline), line.to_string(), scale, colors::text()));
            baseline -= line_height;
        }
    }
}

/// A line of a menu list.
#[derive(Default)]
struct Line {
    label: String,
    value: Option<String>,
    /// Math at the right end: a constant's symbol, a unit; small if it is too tall for the line.
    math: Option<Row>,
    /// What stands at the right end.
    mark: Mark,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Mark {
    #[default]
    None,
    /// Opens a further page.
    Arrow,
    /// The value set now, in a list of choices.
    Dot,
    /// A network: its signal as bars, a lock if it needs a password, a dot if it is saved.
    Signal { bars: u8, secured: bool, saved: bool },
    /// Not a line to choose: a heading over the lines below it, or a hint, in small type
    /// without a box or a number.
    Caption,
    /// A line that cannot be used now: its label faint.
    Faint,
}

/// SETUP or OPTN: the page on the screen, or the submenu open over it.
pub(super) fn menu_screen<B>(shapes: &mut Vec<Shape>, app: &App<B>, menu: &Menu, glass: bool) {
    let lang = app.settings.language;
    let list = List {
        scroll: app.drag().map_or(0, |drag| drag.y),
        pressed: match app.pressed() {
            Some(Target::Line(offset)) => Some(offset),
            _ => None,
        },
        glass,
    };
    match &menu.edit {
        None => {
            let level = menu.level();
            let lines = level.page.entries.iter().map(|entry| Line {
                label: entry.label.get(lang).to_string(),
                value: match entry.kind {
                    Kind::Wallpapers => Some(app.wallpaper_name.clone().unwrap_or_else(|| text::NONE.get(lang).to_string())),
                    Kind::Themes => Some(app.theme.name(lang.code()).to_string()),
                    _ => menu::value(entry, &app.settings, lang),
                },
                math: None,
                mark: if matches!(entry.kind, Kind::Page(_)) { Mark::Arrow } else { Mark::None },
            });
            lines_page(shapes, lines.collect(), level.selected, &list);
        }
        Some(Edit::Choice { entry, selected }) => {
            let Kind::Choice(setting, opts) = &entry.kind else { return };
            let value = app.settings.get(*setting);
            let lines = opts.iter().map(|opt| Line {
                label: opt.text.get(lang).to_string(),
                value: None,
                math: None,
                mark: if opt.value == value { Mark::Dot } else { Mark::None },
            });
            lines_page(shapes, lines.collect(), *selected, &list);
        }
        Some(Edit::Confirm { .. }) => {
            let yes = Line { label: text::YES.get(lang).to_string(), value: None, math: None, mark: Mark::None };
            lines_page(shapes, vec![yes], 0, &list);
        }
        Some(Edit::Number { setting, min, max, typed, error, .. }) => {
            let top = BAR + 30;
            let height = 60;
            let size = Size::new(WIDTH - 2 * MARGIN as u32, height as u32);
            shapes.push(panel(rounded(Point::new(MARGIN, top), size, 6), colors::surface(), Some(colors::primary()), glass));
            let metrics = Scale::Normal.metrics();
            let baseline = top + (height + metrics.digit) / 2;
            let x = MARGIN + 16;
            if typed.is_empty() {
                // The value set now, until a digit is typed.
                let now = app.settings.get(*setting).to_string();
                shapes.push(Shape::Text(Point::new(x + 6, baseline), now, Scale::Normal, colors::faint()));
            } else {
                shapes.push(Shape::Text(Point::new(x, baseline), typed.clone(), Scale::Normal, colors::text()));
            }
            if app.caret_on {
                let caret_x = if typed.is_empty() { x } else { x + Scale::Normal.width(typed) + 2 };
                let caret = Rectangle::new(Point::new(caret_x, baseline - metrics.ascent + 4), Size::new(2, metrics.ascent as u32));
                shapes.push(Shape::Fill(caret, colors::primary()));
            }
            let hint = format!("{}: {min} {} {max}", text::RANGE.get(lang), text::TO.get(lang));
            let color = if *error { colors::error() } else { colors::muted() };
            let at = Point::new(MARGIN + 4, top + height + 28);
            shapes.push(Shape::Text(at, hint, Scale::Small, color));
            for (index, (target, label)) in NUMBER_BUTTONS.iter().enumerate() {
                button(shapes, number_button(index), label, app.pressed() == Some(*target), glass);
            }
        }
        Some(Edit::Pictures { names, selected, .. }) => {
            let set = app.wallpaper_name.as_ref().and_then(|name| names.iter().position(|n| n == name)).map_or(0, |i| i + 1);
            let lines = [text::NONE.get(lang).to_string()].into_iter().chain(names.iter().cloned()).enumerate();
            let lines = lines.map(|(index, label)| Line {
                label,
                value: None,
                math: None,
                mark: if index == set { Mark::Dot } else { Mark::None },
            });
            let x = WIDTH as i32 - MARGIN - THUMBNAIL.width as i32;
            lines_in(shapes, lines.collect(), *selected, &list, list_edge(menu));
            // The picture under the cursor, or why there is none.
            let at = Point::new(x, BAR + (HEIGHT as i32 - BAR - THUMBNAIL.height as i32) / 2);
            let frame = Rectangle::new(at - Point::new(2, 2), THUMBNAIL + Size::new(4, 4));
            shapes.push(Shape::Round(RoundedRectangle::with_equal_corners(frame, Size::new(4, 4)), colors::hairline(), None));
            match app.wallpaper() {
                Some(wallpaper) => shapes.push(Shape::Thumbnail(at, wallpaper.full.clone())),
                None => {
                    shapes.push(Shape::Fill(Rectangle::new(at, THUMBNAIL), colors::background()));
                    let message = if *selected == 0 { text::NONE } else { text::BAD_PICTURE }.get(lang);
                    let message = fit(message, THUMBNAIL.width as i32 - 16, Scale::Small);
                    let middle = at + Point::new((THUMBNAIL.width as i32 - Scale::Small.width(&message)) / 2, THUMBNAIL.height as i32 / 2);
                    shapes.push(Shape::Text(middle, message, Scale::Small, colors::muted()));
                }
            }
        }
        Some(Edit::Themes { names, selected, .. }) => {
            let set = app.theme_index();
            let lines = names.iter().enumerate().map(|(index, label)| Line {
                label: label.clone(),
                value: None,
                math: None,
                mark: if Some(index) == set { Mark::Dot } else { Mark::None },
            });
            lines_page(shapes, lines.collect(), *selected, &list);
        }
        Some(Edit::Apps { view: menu::AppsView::Info, apps, selected, .. }) => {
            if let Some(menu::AppLine::App(index)) = menu::app_lines(apps).get(*selected) {
                app_info(shapes, &apps[*index], lang);
            }
        }
        Some(Edit::Apps { view: menu::AppsView::Confirm, .. }) => {
            let yes = Line { label: text::YES.get(lang).to_string(), value: None, math: None, mark: Mark::None };
            lines_page(shapes, vec![yes], 0, &list);
        }
        Some(Edit::Apps { view: menu::AppsView::Options(option), .. }) => {
            let lines = menu::APP_OPTIONS.iter().map(|label| Line { label: label.get(lang).to_string(), value: None, math: None, mark: Mark::None });
            lines_page(shapes, lines.collect(), *option, &list);
        }
        Some(Edit::Apps { view: menu::AppsView::Note(note), .. }) => {
            for (index, line) in super::sheet::wrap(note, WIDTH as i32 - 2 * MARGIN - 16, Scale::Small).into_iter().enumerate() {
                let x = (WIDTH as i32 - Scale::Small.width(&line)) / 2;
                let y = BAR + (HEIGHT as i32 - BAR) / 2 + index as i32 * (Scale::Small.metrics().ascent + 6);
                shapes.push(Shape::Text(Point::new(x.max(MARGIN), y), line, Scale::Small, colors::muted()));
            }
        }
        Some(Edit::Apps { apps, selected, view: menu::AppsView::List, .. }) => {
            let lines = menu::app_lines(apps).into_iter().map(|line| match line {
                menu::AppLine::Heading(kind) => Line {
                    label: match kind {
                        crate::applet::AppKind::Game => text::GAMES,
                        crate::applet::AppKind::Tool => text::TOOLS,
                    }
                    .get(lang)
                    .to_string(),
                    value: None,
                    math: None,
                    mark: Mark::Caption,
                },
                menu::AppLine::App(index) => {
                    let app = &apps[index];
                    Line {
                        label: app.name.clone(),
                        value: (!app.authors.is_empty()).then(|| app.authors.join(", ")),
                        math: None,
                        mark: if app.usable { Mark::None } else { Mark::Faint },
                    }
                }
                menu::AppLine::Hint => Line { label: text::OWN_APPS.get(lang).to_string(), value: None, math: None, mark: Mark::Caption },
            });
            lines_page(shapes, lines.collect(), *selected, &list);
        }
        Some(Edit::Sessions { sessions, selected, .. }) => {
            let open = app.session.as_ref().map(|session| session.id.as_str());
            let lines = sessions.iter().map(|session| Line {
                label: session.name.clone(),
                value: None,
                math: None,
                mark: if Some(session.id.as_str()) == open { Mark::Dot } else { Mark::None },
            });
            lines_page(shapes, lines.collect(), *selected, &list);
        }
        Some(Edit::Sections { titles, selected, .. }) => {
            let lines = titles.iter().map(|title| Line { label: title.clone(), value: None, math: None, mark: Mark::None });
            lines_page(shapes, lines.collect(), *selected, &list);
        }
        Some(Edit::Text { purpose, typed, error, plain, .. }) => {
            let (hint, color) = match (purpose, error) {
                (TextFor::Name, false) => (text::NAME_HINT.get(lang).to_string(), colors::muted()),
                (TextFor::Name, true) => (text::EMPTY_NAME.get(lang).to_string(), colors::error()),
                (TextFor::Password(ssid), false) => (format!("{ssid}: {}", text::PASSWORD_HINT.get(lang)), colors::muted()),
                (TextFor::Password(_), true) => (text::PASSWORD_LENGTH.get(lang).to_string(), colors::error()),
                (TextFor::Variable(_), false) => (text::STORE_HINT.get(lang).to_string(), colors::muted()),
                (TextFor::Matrix, false) => (text::MATRIX_HINT.get(lang).to_string(), colors::muted()),
                (TextFor::Variable(_) | TextFor::Matrix, true) => (text::BAD_VARIABLE.get(lang).to_string(), colors::error()),
            };
            let shown = if *plain { typed.clone() } else { "*".repeat(typed.chars().count()) };
            typed_field(shapes, &shown, app.caret_on, &fit(&hint, WIDTH as i32 - 2 * MARGIN - 8, Scale::Small), color, glass);
        }
        Some(Edit::Time { field, typed, error, .. }) => {
            let color = if *error { colors::error() } else { colors::muted() };
            typed_field(shapes, &field.show(typed), false, field.hint(*error).get(lang), color, glass);
        }
        Some(Edit::Storage { storage: Some(storage), .. }) => storage_screen(shapes, *storage, lang, glass),
        Some(Edit::Storage { storage: None, .. }) => centred(shapes, text::NO_CARD.get(lang)),
        Some(Edit::Message { message, .. }) => centred(shapes, message.get(lang)),
        Some(Edit::AppInfo { info, .. }) => app_info(shapes, info, lang),
        Some(Edit::Note { note, .. }) => {
            let lines = super::sheet::wrap(note, WIDTH as i32 - 2 * MARGIN - 16, Scale::Small);
            let height = Scale::Small.metrics().ascent + 6;
            let top = BAR + (HEIGHT as i32 - BAR - lines.len() as i32 * height) / 2 + height;
            for (index, line) in lines.into_iter().enumerate() {
                let x = (WIDTH as i32 - Scale::Small.width(&line)) / 2;
                shapes.push(Shape::Text(Point::new(x.max(MARGIN), top + index as i32 * height), line, Scale::Small, colors::error()));
            }
        }
        Some(Edit::Networks { networks: None, .. }) => centred(shapes, text::SEARCHING.get(lang)),
        Some(Edit::Networks { networks: Some(networks), .. }) if networks.is_empty() => {
            centred(shapes, text::NO_NETWORKS.get(lang))
        }
        Some(Edit::Networks { networks: Some(networks), selected, .. }) => {
            let lines = networks.iter().map(|network| Line {
                label: network.ssid.clone(),
                value: None,
                math: None,
                mark: Mark::Signal { bars: network.bars(), secured: network.secured, saved: network.saved },
            });
            lines_page(shapes, lines.collect(), *selected, &list);
        }
        Some(Edit::Saved { names, selected, .. }) => {
            let lines = names.iter().map(|name| Line { label: name.clone(), value: None, math: None, mark: Mark::None });
            lines_page(shapes, lines.collect(), *selected, &list);
        }
        Some(Edit::Forget { ssid, .. }) => {
            let label = format!("{}: {ssid}", text::FORGET.get(lang));
            lines_page(shapes, vec![Line { label, value: None, math: None, mark: Mark::None }], 0, &list);
        }
        Some(Edit::WlanStatus { status, .. }) => wlan_screen(shapes, status, lang, glass),
        Some(Edit::Variables { variables, selected, .. }) => {
            // A matrix is too tall for a line: its size instead.
            let lines = variables.iter().map(|variable| match &crate::latex::parse(&variable.value).0[..] {
                [crate::editor::Node::Matrix(rows)] => {
                    let size = format!("{} {} × {}", text::MATRIX.get(lang), rows.len(), rows.first().map_or(0, Vec::len));
                    Line { label: variable.key.clone(), value: Some(size), ..Line::default() }
                }
                _ => Line { label: variable.key.clone(), math: Some(crate::latex::parse(&variable.value)), ..Line::default() },
            });
            lines_page(shapes, lines.collect(), *selected, &list);
        }
        Some(Edit::Variable { selected, .. }) => {
            let lines = menu::VARIABLE_ACTIONS.iter().map(|action| Line { label: action.get(lang).to_string(), ..Line::default() });
            lines_page(shapes, lines.collect(), *selected, &list);
        }
        Some(Edit::Catalogue { query, hits, .. }) if !query.is_empty() && hits.is_empty() => {
            centred(shapes, text::NOTHING_FOUND.get(lang))
        }
        Some(Edit::Catalogue { query, hits, selected, .. }) if !query.is_empty() => {
            let lines = hits.iter().map(|hit| Line {
                label: hit.name.clone(),
                value: hit.detail.clone(),
                math: hit.latex.as_deref().map(crate::latex::parse),
                ..Line::default()
            });
            lines_page(shapes, lines.collect(), *selected, &list);
        }
        Some(Edit::Catalogue { groups, group: None, selected, .. }) => {
            let lines = groups.iter().map(|group| Line { label: group.name.clone(), mark: Mark::Arrow, ..Line::default() });
            lines_page(shapes, lines.collect(), *selected, &list);
        }
        Some(Edit::Catalogue { groups, group: Some(group), selected, .. }) => {
            let items = groups.get(*group).map_or(&[][..], |group| &group.items);
            let lines = items.iter().map(|item| Line {
                label: item.name.clone(),
                math: Some(crate::latex::parse(&item.latex)),
                ..Line::default()
            });
            lines_page(shapes, lines.collect(), *selected, &list);
        }
    }
}

/// A line of text being typed in a panel, its end shown if it is long, with the caret after it
/// while `caret` is on, and a hint under it in `color`.
fn typed_field(shapes: &mut Vec<Shape>, typed: &str, caret: bool, hint: &str, color: Rgb565, glass: bool) {
    let top = BAR + 30;
    let height = 60;
    let size = Size::new(WIDTH - 2 * MARGIN as u32, height as u32);
    shapes.push(panel(rounded(Point::new(MARGIN, top), size, 6), colors::surface(), Some(colors::primary()), glass));
    let metrics = Scale::Normal.metrics();
    let baseline = top + (height + metrics.digit) / 2;
    let x = MARGIN + 16;
    let room = WIDTH as i32 - 2 * MARGIN - 40;
    let mut shown = typed;
    while Scale::Normal.width(shown) > room {
        let mut chars = shown.chars();
        chars.next();
        shown = chars.as_str();
    }
    shapes.push(Shape::Text(Point::new(x, baseline), shown.to_string(), Scale::Normal, colors::text()));
    if caret {
        let caret_x = x + Scale::Normal.width(shown) + 2;
        let caret = Rectangle::new(Point::new(caret_x, baseline - metrics.ascent + 4), Size::new(2, metrics.ascent as u32));
        shapes.push(Shape::Fill(caret, colors::primary()));
    }
    shapes.push(Shape::Text(Point::new(MARGIN + 4, top + height + 28), hint.to_string(), Scale::Small, color));
}

/// A padlock 10 wide and 16 high with its top left at `at`: a shackle of three bars over a body.
fn lock(shapes: &mut Vec<Shape>, at: Point) {
    let color = colors::muted();
    shapes.push(Shape::Fill(Rectangle::new(at + Point::new(2, 0), Size::new(6, 2)), color));
    shapes.push(Shape::Fill(Rectangle::new(at + Point::new(1, 1), Size::new(2, 7)), color));
    shapes.push(Shape::Fill(Rectangle::new(at + Point::new(7, 1), Size::new(2, 7)), color));
    shapes.push(Shape::Fill(Rectangle::new(at + Point::new(0, 7), Size::new(10, 9)), color));
}

/// SETUP → Verbindungen → WLAN → Status: the network, the address and the signal, or what the
/// connection is doing.
fn wlan_screen(shapes: &mut Vec<Shape>, status: &WlanStatus, lang: text::Lang, glass: bool) {
    let (ssid, ip, rssi) = match status {
        WlanStatus::Connected { ssid, ip, rssi } => (ssid, ip, *rssi),
        WlanStatus::Off => return centred(shapes, text::WLAN_IS_OFF.get(lang)),
        WlanStatus::Searching => return centred(shapes, text::WLAN_SEARCHING.get(lang)),
        WlanStatus::Connecting(ssid) => return centred(shapes, &format!("{} {ssid} ...", text::CONNECTING.get(lang))),
        WlanStatus::Failed(ssid) => return centred(shapes, &format!("{} {ssid}", text::FAILED.get(lang))),
    };
    let top = BAR + 14;
    let size = Size::new(WIDTH - 2 * MARGIN as u32, HEIGHT - top as u32 - 14);
    shapes.push(panel(rounded(Point::new(MARGIN, top), size, 8), colors::surface(), Some(colors::hairline()), glass));
    let (left, right) = (MARGIN + 20, WIDTH as i32 - MARGIN - 20);
    let rows = [
        (text::NETWORK, ssid.clone()),
        (text::ADDRESS, ip.clone()),
        (text::SIGNAL, format!("{rssi} dBm")),
    ];
    for (index, (label, value)) in rows.into_iter().enumerate() {
        let baseline = top + 50 + index as i32 * 56;
        shapes.push(Shape::Text(Point::new(left, baseline), label.get(lang).to_string(), Scale::Small, colors::muted()));
        let value = fit(&value, right - left - 110, Scale::Normal);
        let at = Point::new(right - Scale::Normal.width(&value), baseline);
        shapes.push(Shape::Text(at, value, Scale::Normal, colors::text()));
    }
}

/// A muted line in the middle of the screen under the top bar.
/// An app's credits: its name, the authors, the version, the description, the licence, the API
/// and where it comes from.
fn app_info(shapes: &mut Vec<Shape>, info: &crate::applet::AppInfo, lang: text::Lang) {
    let room = WIDTH as i32 - 2 * MARGIN;
    let mut y = BAR + 14 + Scale::Normal.metrics().ascent;
    shapes.push(Shape::Text(Point::new(MARGIN, y), fit(&info.name, room, Scale::Normal), Scale::Normal, colors::text()));
    y += 10;
    let step = Scale::Small.metrics().ascent + 8;
    let mut line = |shapes: &mut Vec<Shape>, text: String, color| {
        for part in super::sheet::wrap(&text, room, Scale::Small) {
            y += step;
            shapes.push(Shape::Text(Point::new(MARGIN, y), part, Scale::Small, color));
        }
    };
    let authors = match info.authors.len() {
        0 => text::AUTHOR_UNKNOWN.get(lang).to_string(),
        1 => format!("{}: {}", text::AUTHOR.get(lang), info.authors[0]),
        _ => format!("{}: {}", text::AUTHORS.get(lang), info.authors.join(", ")),
    };
    line(shapes, authors, colors::text());
    if let Some(version) = &info.version {
        line(shapes, format!("{}: {version}", text::APP_VERSION.get(lang)), colors::text());
    }
    if let Some(about) = &info.about {
        line(shapes, about.clone(), colors::muted());
    }
    if let Some(license) = &info.license {
        line(shapes, format!("{}: {license}", text::LICENSE.get(lang)), colors::text());
    }
    let source = match info.built_in {
        true => text::BUILT_IN.get(lang).to_string(),
        // The file's name: a card's app beside a built-in one has `card:` before its id.
        false => format!("{}, {}.lua", text::FROM_CARD.get(lang), info.id.strip_prefix("card:").unwrap_or(&info.id)),
    };
    line(shapes, format!("API {} · {source}", info.api), colors::muted());
}

fn centred(shapes: &mut Vec<Shape>, text: &str) {
    let x = (WIDTH as i32 - Scale::Normal.width(text)) / 2;
    let y = BAR + (HEIGHT as i32 - BAR) / 2;
    shapes.push(Shape::Text(Point::new(x.max(MARGIN), y), text.to_string(), Scale::Normal, colors::muted()));
}

/// SETUP → Gerät → Speicherplatz: the card's name and how full it is in percent, a bar of the
/// space used, and the space used, free and in all.
fn storage_screen(shapes: &mut Vec<Shape>, storage: Storage, lang: text::Lang, glass: bool) {
    let top = BAR + 14;
    let size = Size::new(WIDTH - 2 * MARGIN as u32, HEIGHT - top as u32 - 14);
    shapes.push(panel(rounded(Point::new(MARGIN, top), size, 8), colors::surface(), Some(colors::hairline()), glass));
    let (left, right) = (MARGIN + 20, WIDTH as i32 - MARGIN - 20);
    let right_aligned = |text: &str, scale: Scale| right - scale.width(text);

    let heading = top + 38;
    shapes.push(Shape::Text(Point::new(left, heading), text::SD_CARD.get(lang).to_string(), Scale::Normal, colors::text()));
    // A thousandth, so a nearly full card does not show 100 %.
    let permille = match storage.total {
        0 => 0,
        total => (u128::from(storage.used()) * 1000 / u128::from(total)) as u32,
    };
    let percent = format!("{} %", permille / 10);
    let color = if permille >= 900 { colors::error() } else { colors::result() };
    shapes.push(Shape::Text(Point::new(right_aligned(&percent, Scale::Normal), heading), percent, Scale::Normal, color));

    let (bar_top, bar_height, width) = (heading + 18, 22, right - left);
    shapes.push(Shape::Round(rounded(Point::new(left, bar_top), Size::new(width as u32, bar_height), 6), colors::background(), Some(colors::hairline())));
    // At least a sliver while anything is used, inside the outline.
    let filled = ((width - 4) as u32 * permille / 1000).max(u32::from(storage.used() > 0) * 6);
    if filled > 0 {
        let fill = if permille >= 900 { colors::error() } else { colors::primary() };
        shapes.push(Shape::Round(rounded(Point::new(left + 2, bar_top + 2), Size::new(filled, bar_height - 4), 4), fill, None));
    }

    let rows = [(text::USED, storage.used()), (text::FREE, storage.free), (text::TOTAL, storage.total)];
    for (index, (label, value)) in rows.into_iter().enumerate() {
        let baseline = bar_top + bar_height as i32 + 44 + index as i32 * 40;
        shapes.push(Shape::Text(Point::new(left, baseline), label.get(lang).to_string(), Scale::Small, colors::muted()));
        let value = crate::backend::bytes(value, lang);
        shapes.push(Shape::Text(Point::new(right_aligned(&value, Scale::Normal), baseline), value, Scale::Normal, colors::text()));
    }
}

/// How a list is shown: moved by a dragging finger, the line under it lit.
struct List {
    scroll: i32,
    /// The line the finger is on, counted on the page.
    pressed: Option<usize>,
    glass: bool,
}

/// The list template of an app (apps.md §4.4): its items as a menu of the hui's, at the top of
/// the area.
pub(super) fn app_list(shapes: &mut Vec<Shape>, items: &[String], selected: usize, pressed: Option<usize>) {
    let lines = items.iter().map(|item| Line { label: item.clone(), value: None, math: None, mark: Mark::None });
    lines_page(shapes, lines.collect(), selected, &List { scroll: 0, pressed, glass: false });
}

/// A list of lines, [`menu::ROWS`] to a page, numbered on each page, with a scroll bar when
/// there are more.
fn lines_page(shapes: &mut Vec<Shape>, lines: Vec<Line>, selected: usize, list: &List) {
    lines_in(shapes, lines, selected, list, WIDTH as i32 - MARGIN);
}

/// [`lines_page`] with the lines ending at `edge`; a label too long for them is cut short. The
/// lines lie one under the other, the page of `selected` at the top, so a drag shows the pages
/// before and after it moving in.
fn lines_in(shapes: &mut Vec<Shape>, lines: Vec<Line>, selected: usize, list: &List, edge: i32) {
    let first = selected / ROWS * ROWS;
    let normal = Scale::Normal.metrics();
    let small = Scale::Small.metrics();
    let scrolls = lines.len() > ROWS;
    let right = edge - if scrolls { 8 } else { 0 };
    for (index, line) in lines.iter().enumerate() {
        let place = index as i32 - first as i32;
        let area = line_rect(0, right).translate(Point::new(0, place * (LINE + LINE_GAP) + list.scroll));
        let y = area.top_left.y;
        if y + LINE <= BAR || y >= HEIGHT as i32 {
            continue;
        }
        if line.mark == Mark::Caption {
            let label = fit(&line.label, right - MARGIN - 12, Scale::Small);
            let baseline = y + LINE - 12;
            shapes.push(Shape::Text(Point::new(MARGIN + 12, baseline), label, Scale::Small, colors::muted()));
            continue;
        }
        let chosen = index == selected || (list.scroll == 0 && list.pressed.is_some_and(|offset| first + offset == index));
        let fill = if chosen { colors::primary_dim() } else { colors::surface() };
        shapes.push(panel(RoundedRectangle::with_equal_corners(area, Size::new(6, 6)), fill, None, list.glass));
        let baseline = y + (LINE + normal.digit) / 2;
        let number = (index % ROWS + 1).to_string();
        shapes.push(Shape::Text(Point::new(MARGIN + 12, baseline), number, Scale::Small, colors::muted()));
        let middle = y + LINE / 2;
        let mut end = right - 14;
        let math = line.math.as_ref().map(|row| match layout::layout(row, Scale::Normal) {
            math if math.height() > LINE - 8 => layout::layout(row, Scale::Small),
            math => math,
        });
        let math_width = math.as_ref().map_or(0, |math| math.width + 12);
        let label = fit(&line.label, right - MARGIN - 34 - 14 - math_width, Scale::Normal);
        let color = if line.mark == Mark::Faint { colors::faint() } else { colors::text() };
        shapes.push(Shape::Text(Point::new(MARGIN + 34, baseline), label, Scale::Normal, color));
        if let Some(math) = math {
            let at = Point::new(end - math.width, middle + (math.ascent - math.descent) / 2);
            let color = if chosen { colors::text() } else { colors::result() };
            shapes.push(Shape::Math(at, math, color));
            end -= math_width;
        }
        match line.mark {
            Mark::Arrow => {
                let arrow = Triangle::new(
                    Point::new(end, middle),
                    Point::new(end - 7, middle - 6),
                    Point::new(end - 7, middle + 6),
                );
                shapes.push(Shape::Triangle(arrow, colors::muted()));
                end -= 16;
            }
            Mark::Dot => {
                let dot = 10;
                shapes.push(Shape::Circle(Circle::new(Point::new(end - dot, middle - dot / 2), dot as u32), colors::primary()));
                end -= dot + 8;
            }
            Mark::Signal { bars, secured, saved } => {
                // Four bars rising to the right, the ones beyond the signal faint.
                for bar in 0..4 {
                    let height = 5 + 4 * bar;
                    let at = Point::new(end - 4 * 5 + bar * 5 + 1, middle + 8 - height);
                    let color = if bar < i32::from(bars) { colors::text() } else { colors::faint() };
                    shapes.push(Shape::Fill(Rectangle::new(at, Size::new(3, height as u32)), color));
                }
                end -= 4 * 5 + 8;
                if secured {
                    lock(shapes, Point::new(end - 10, middle - 8));
                }
                end -= 18;
                if saved {
                    let dot = 10;
                    shapes.push(Shape::Circle(Circle::new(Point::new(end - dot, middle - dot / 2), dot as u32), colors::primary()));
                }
            }
            Mark::None | Mark::Caption | Mark::Faint => {}
        }
        if let Some(value) = &line.value {
            let width = Scale::Small.width(value);
            let color = if chosen { colors::text() } else { colors::muted() };
            let at = Point::new(end - width, y + (LINE + small.digit) / 2);
            shapes.push(Shape::Text(at, value.clone(), Scale::Small, color));
        }
    }
    if scrolls {
        let top = BAR + 8;
        let height = ROWS as i32 * (LINE + LINE_GAP) - LINE_GAP;
        let x = edge - 3;
        shapes.push(Shape::Fill(Rectangle::new(Point::new(x, top), Size::new(3, height as u32)), colors::hairline()));
        let pages = lines.len().div_ceil(ROWS) as i32;
        let thumb = height / pages;
        let page = (first / ROWS) as i32;
        let at = Point::new(x, top + page * thumb);
        shapes.push(Shape::Fill(Rectangle::new(at, Size::new(3, thumb as u32)), colors::primary()));
    }
}

/// A button with its label in the middle, lit while a finger is on it.
fn button(shapes: &mut Vec<Shape>, area: Rectangle, label: &str, lit: bool, glass: bool) {
    let round = RoundedRectangle::with_equal_corners(area, Size::new(6, 6));
    let fill = if lit { colors::primary_dim() } else { colors::surface() };
    shapes.push(panel(round, fill, Some(colors::hairline()), glass));
    let digit = Scale::Small.metrics().digit;
    let x = (area.size.width as i32 - Scale::Small.width(label)) / 2;
    let at = area.top_left + Point::new(x, (area.size.height as i32 + digit) / 2);
    shapes.push(Shape::Text(at, label.to_string(), Scale::Small, colors::text()));
}

/// The touch calibration: a cross at the target in turn, filled while it is pressed, and what
/// to do in the middle of the screen.
pub(super) fn calibration(shapes: &mut Vec<Shape>, state: &Calibrating, lang: text::Lang) {
    let step = state.readings.len().min(touch::TARGETS.len() - 1);
    let at = touch::TARGETS[step];
    let pressed = state.sum.2 > 0;
    for (from, to) in [(Point::new(-16, 0), Point::new(16, 0)), (Point::new(0, -16), Point::new(0, 16))] {
        shapes.push(Shape::Path(vec![at + from, at + to], colors::text(), 2));
    }
    let ring = if pressed { colors::primary() } else { colors::accent() };
    shapes.push(Shape::Circle(Circle::with_center(at, 10), ring));
    let centred = |shapes: &mut Vec<Shape>, y: i32, text: &str, scale: Scale, color: Rgb565| {
        let x = (WIDTH as i32 - scale.width(text)) / 2;
        shapes.push(Shape::Text(Point::new(x.max(MARGIN), y), text.to_string(), scale, color));
    };
    centred(shapes, 115, text::CALIBRATE_HINT.get(lang), Scale::Normal, colors::text());
    let count = format!("{} / {}", step + 1, touch::TARGETS.len());
    centred(shapes, 220, &count, Scale::Small, colors::muted());
    if state.retry {
        centred(shapes, 245, text::CALIBRATE_RETRY.get(lang), Scale::Small, colors::error());
    }
}
