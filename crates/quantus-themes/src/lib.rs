//! The colour themes of the desktop app and the handheld: one TOML file per theme, the same
//! file for both. The built-in ones are `themes/*.toml` at the top of the repository, built into
//! the program; the users' own lie in a `themes/` folder (the app's config folder, the SD card)
//! and are read with [`list`] and [`find`].
//!
//! A theme names the theme it starts from (`base`, `dark` if not given) and gives only what it
//! changes, so a [`Theme`] read from a file always has every colour. The format is described in
//! `docs/docs/reference/themes.md`.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::OnceLock;

use toml::{Table, Value};

/// The keys of `[colors]`, shared by both programs. The desktop sets each as the CSS variable of
/// the same name (`--background`, …).
pub const COLORS: &[&str] = &[
    "background",
    "surface",
    "surface-raised",
    "surface-hover",
    "hairline",
    "hairline-strong",
    "text",
    "text-muted",
    "text-faint",
    "primary",
    "primary-dim",
    "accent",
    "accent-dim",
    "danger",
    "glow",
    "overlay",
    "overlay-strong",
    "point-zero",
    "point-y-intercept",
    "point-max",
    "point-min",
    "point-inflection",
    "point-saddle",
    "geometry-0",
    "geometry-1",
    "geometry-2",
    "geometry-3",
    "geometry-4",
    "geometry-5",
    "geometry-6",
    "geometry-7",
];

/// The keys of `[handheld]` that hold one colour; `tiles` holds [`TILES`] pairs.
pub const HANDHELD: &[&str] = &["result", "error", "shift", "alpha", "unit"];

/// The number of mode tiles on the handheld, each with its own pair of colours.
pub const TILES: usize = 14;

/// The theme every other one starts from, unless it names another base.
pub const DEFAULT: &str = "dark";

/// The built-in themes: their ids and files, `dark` first, each after its base.
pub const BUILT_IN: &[(&str, &str)] = &[
    ("dark", include_str!("../../../themes/dark.toml")),
    ("light", include_str!("../../../themes/light.toml")),
    ("black", include_str!("../../../themes/black.toml")),
    ("paper", include_str!("../../../themes/paper.toml")),
    ("contrast-dark", include_str!("../../../themes/contrast-dark.toml")),
    ("contrast-light", include_str!("../../../themes/contrast-light.toml")),
];

/// A colour with its opacity, 255 being opaque.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b, a: 255 }
    }

    /// Reads `#rrggbb` or `#rrggbbaa`, in either case.
    pub fn parse(text: &str) -> Option<Self> {
        let hex = text.trim().strip_prefix('#')?;
        if !(hex.len() == 6 || hex.len() == 8) || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
        Some(Color { r: byte(0)?, g: byte(2)?, b: byte(4)?, a: if hex.len() == 8 { byte(6)? } else { 255 } })
    }
}

/// `#rrggbb`, or `#rrggbbaa` if not opaque.
impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)?;
        if self.a != 255 {
            write!(f, "{:02x}", self.a)?;
        }
        Ok(())
    }
}

/// The colours only the handheld has.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Handheld {
    pub colors: BTreeMap<String, Color>,
    /// One pair per mode tile, in the order of the mode menu: the tile, the chosen tile.
    pub tiles: [(Color, Color); TILES],
}

/// A theme, with every colour.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Theme {
    /// The file's name without `.toml`.
    pub id: String,
    /// Its name per language code; may be empty, then the id is its name.
    pub names: BTreeMap<String, String>,
    /// Whether it is a dark theme: the desktop tells the system (scroll bars, form controls).
    pub dark: bool,
    pub colors: BTreeMap<String, Color>,
    pub handheld: Handheld,
    /// What in the file was ignored: unknown keys.
    pub warnings: Vec<String>,
}

impl Theme {
    /// Its name in the language `code`, else in English, else any, else its id.
    pub fn name(&self, code: &str) -> &str {
        self.names
            .get(code)
            .or_else(|| self.names.get("en"))
            .or_else(|| self.names.values().next())
            .map_or(&self.id, |name| name)
    }

    /// The colour of a key of `[colors]` ([`COLORS`]).
    pub fn color(&self, key: &str) -> Color {
        self.colors.get(key).copied().unwrap_or(Color::rgb(255, 0, 255))
    }

    /// The colour of a one-colour key of `[handheld]` ([`HANDHELD`]).
    pub fn handheld(&self, key: &str) -> Color {
        self.handheld.colors.get(key).copied().unwrap_or(Color::rgb(255, 0, 255))
    }

    /// The CSS variables to set: `("--background", "#17161d")`, …
    pub fn css_vars(&self) -> Vec<(String, String)> {
        COLORS.iter().map(|key| (format!("--{key}"), self.color(key).to_string())).collect()
    }
}

/// Why a theme file cannot be used.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

fn built_ins() -> &'static [Theme] {
    static THEMES: OnceLock<Vec<Theme>> = OnceLock::new();
    THEMES.get_or_init(|| {
        let mut themes: Vec<Theme> = Vec::new();
        for (id, text) in BUILT_IN {
            let theme = read(id, text, |base| themes.iter().find(|t| t.id == base).cloned())
                .unwrap_or_else(|e| panic!("themes/{id}.toml: {e}"));
            themes.push(theme);
        }
        themes
    })
}

/// The built-in theme `id`.
pub fn built_in(id: &str) -> Option<Theme> {
    built_ins().iter().find(|t| t.id == id).cloned()
}

/// The default theme, [`DEFAULT`].
pub fn default_theme() -> Theme {
    built_ins()[0].clone()
}

/// Reads a user's theme file; its `base` must be a built-in theme.
pub fn parse(id: &str, text: &str) -> Result<Theme, Error> {
    read(id, text, built_in)
}

/// A theme on offer: a built-in one, or a file of the user's, which may be broken.
#[derive(Clone, Debug)]
pub struct Entry {
    pub id: String,
    pub built_in: bool,
    pub theme: Result<Theme, Error>,
}

/// Every theme: the built-in ones, then the user's (`(id, file text)`) sorted by id. A user's
/// file with the id of a built-in theme takes its place.
pub fn list(user: impl IntoIterator<Item = (String, String)>) -> Vec<Entry> {
    let mut user: Vec<(String, String)> = user.into_iter().collect();
    user.sort_by(|a, b| a.0.cmp(&b.0));
    let mut entries: Vec<Entry> = built_ins()
        .iter()
        .map(|t| Entry { id: t.id.clone(), built_in: true, theme: Ok(t.clone()) })
        .collect();
    for (id, text) in user {
        let entry = Entry { theme: parse(&id, &text), id, built_in: false };
        match entries.iter_mut().find(|e| e.id == entry.id) {
            Some(slot) => *slot = entry,
            None => entries.push(entry),
        }
    }
    entries
}

/// The theme `id`: the user's file of that id if there is one, else the built-in one.
pub fn find(id: &str, user: impl IntoIterator<Item = (String, String)>) -> Result<Theme, Error> {
    match user.into_iter().find(|(name, _)| name == id) {
        Some((_, text)) => parse(id, &text),
        None => built_in(id).ok_or_else(|| Error(format!("there is no theme \u{201c}{id}\u{201d}"))),
    }
}

/// Reads the file `text` of the theme `id`, looking up its base with `base`. The built-in
/// default theme has no base and has to give every key.
fn read(id: &str, text: &str, base: impl Fn(&str) -> Option<Theme>) -> Result<Theme, Error> {
    let table: Table = text.parse().map_err(|e: toml::de::Error| Error(e.message().to_string()))?;
    let mut warnings = Vec::new();

    let base_id = match table.get("base") {
        None => (id != DEFAULT).then_some(DEFAULT),
        Some(Value::String(name)) => Some(name.as_str()),
        Some(_) => return Err(Error("base: must be the id of a theme, in quotes".into())),
    };
    let base = match base_id {
        Some(name) => {
            Some(base(name).ok_or_else(|| Error(format!("base: there is no built-in theme \u{201c}{name}\u{201d}")))?)
        }
        None => None,
    };

    let names = match table.get("name") {
        None => BTreeMap::new(),
        Some(Value::String(name)) => BTreeMap::from([("en".to_string(), name.clone())]),
        Some(Value::Table(names)) => names
            .iter()
            .map(|(code, name)| match name {
                Value::String(name) => Ok((code.clone(), name.clone())),
                _ => Err(Error(format!("name.{code}: must be text in quotes"))),
            })
            .collect::<Result<_, _>>()?,
        Some(_) => return Err(Error("name: must be text, or a table of texts per language".into())),
    };

    let dark = match (table.get("dark"), &base) {
        (Some(Value::Boolean(dark)), _) => *dark,
        (Some(_), _) => return Err(Error("dark: must be true or false".into())),
        (None, Some(base)) => base.dark,
        (None, None) => return Err(Error("dark: missing".into())),
    };

    let empty = Table::new();
    let section = |name: &str| match table.get(name) {
        None => Ok(&empty),
        Some(Value::Table(section)) => Ok(section),
        Some(_) => Err(Error(format!("{name}: must be a section, [{name}]"))),
    };

    let colors = colors(
        "colors",
        section("colors")?,
        COLORS,
        base.as_ref().map(|b| &b.colors),
        &[],
        &mut warnings,
    )?;
    let handheld_section = section("handheld")?;
    let handheld_colors = colors_of_handheld(handheld_section, base.as_ref(), &mut warnings)?;
    let tiles = match (handheld_section.get("tiles"), &base) {
        (Some(value), _) => tiles(value)?,
        (None, Some(base)) => base.handheld.tiles,
        (None, None) => return Err(Error("handheld.tiles: missing".into())),
    };

    for key in table.keys() {
        if !["name", "base", "dark", "colors", "handheld"].contains(&key.as_str()) {
            warnings.push(format!("{key}: unknown, ignored"));
        }
    }

    Ok(Theme {
        id: id.to_string(),
        names,
        dark,
        colors,
        handheld: Handheld { colors: handheld_colors, tiles },
        warnings,
    })
}

fn colors_of_handheld(
    section: &Table,
    base: Option<&Theme>,
    warnings: &mut Vec<String>,
) -> Result<BTreeMap<String, Color>, Error> {
    colors("handheld", section, HANDHELD, base.map(|b| &b.handheld.colors), &["tiles"], warnings)
}

/// The colours of `section` over those of the base; without a base, every key must be there.
/// `other` are keys of the section read elsewhere.
fn colors(
    name: &str,
    section: &Table,
    keys: &[&str],
    base: Option<&BTreeMap<String, Color>>,
    other: &[&str],
    warnings: &mut Vec<String>,
) -> Result<BTreeMap<String, Color>, Error> {
    let mut colors = base.cloned().unwrap_or_default();
    for (key, value) in section {
        if other.contains(&key.as_str()) {
            continue;
        }
        if !keys.contains(&key.as_str()) {
            warnings.push(format!("{name}.{key}: unknown, ignored"));
            continue;
        }
        colors.insert(key.clone(), color(&format!("{name}.{key}"), value)?);
    }
    if let Some(missing) = keys.iter().find(|key| !colors.contains_key(**key)) {
        return Err(Error(format!("{name}.{missing}: missing")));
    }
    Ok(colors)
}

fn color(key: &str, value: &Value) -> Result<Color, Error> {
    match value {
        Value::String(text) => Color::parse(text)
            .ok_or_else(|| Error(format!("{key}: \u{201c}{text}\u{201d} is not a colour, write #rrggbb or #rrggbbaa"))),
        _ => Err(Error(format!("{key}: must be a colour in quotes, \"#rrggbb\""))),
    }
}

fn tiles(value: &Value) -> Result<[(Color, Color); TILES], Error> {
    let shape = || Error(format!("handheld.tiles: must be {TILES} pairs of colours, [[\"#rrggbb\", \"#rrggbb\"], …]"));
    let Value::Array(pairs) = value else { return Err(shape()) };
    if pairs.len() != TILES {
        return Err(shape());
    }
    let mut tiles = [(Color::rgb(0, 0, 0), Color::rgb(0, 0, 0)); TILES];
    for (i, pair) in pairs.iter().enumerate() {
        let Value::Array(pair) = pair else { return Err(shape()) };
        let [tile, chosen] = pair.as_slice() else { return Err(shape()) };
        tiles[i] = (color(&format!("handheld.tiles[{i}]"), tile)?, color(&format!("handheld.tiles[{i}]"), chosen)?);
    }
    Ok(tiles)
}
