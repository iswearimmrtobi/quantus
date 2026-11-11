//! The header of an app: the comment lines at the top of its file, read to list the app without
//! running it (apps.md §2).
//!
//! ```lua
//! -- name: 2048
//! -- kind: game
//! -- api: 1
//! -- author: Quantus
//! ```

use quantus_ui::applet::{AppInfo, AppKind};
use quantus_ui::text::Lang;

/// The version of the script API this firmware has.
pub const API: u32 = 1;

/// The longest a header's value may be; longer is cut.
const LONGEST: usize = 80;

/// What a header says.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Header {
    pub name: String,
    /// `name.<lang>`: the name in one language, by its code.
    pub names: Vec<(String, String)>,
    pub kind: AppKind,
    pub api: u32,
    pub authors: Vec<String>,
    pub version: Option<String>,
    pub about: Option<String>,
    pub abouts: Vec<(String, String)>,
    pub license: Option<String>,
}

impl Header {
    /// The name in `lang`, the plain one where it has none.
    pub fn name(&self, lang: Lang) -> &str {
        localised(&self.names, lang).unwrap_or(&self.name)
    }

    /// The description in `lang`, the plain one where it has none.
    pub fn about(&self, lang: Lang) -> Option<&str> {
        localised(&self.abouts, lang).or(self.about.as_deref())
    }
}

fn localised(values: &[(String, String)], lang: Lang) -> Option<&str> {
    values.iter().find(|(code, _)| code == lang.code()).map(|(_, value)| value.as_str())
}

/// The header of a script's `text`: the comment lines before its first line of code. `None`
/// without a `name`.
pub fn parse(text: &str) -> Option<Header> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut header = Header { api: 1, ..Header::default() };
    let mut named = false;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some(comment) = line.strip_prefix("--") else { break };
        let Some((key, value)) = comment.split_once(':') else { continue };
        let (key, value) = (key.trim(), cut(value.trim()));
        if value.is_empty() {
            continue;
        }
        match key {
            "name" => {
                header.name = value;
                named = true;
            }
            "kind" => header.kind = if value == "game" { AppKind::Game } else { AppKind::Tool },
            "api" => header.api = value.parse().unwrap_or(u32::MAX),
            "author" => header.authors.push(value),
            "version" => header.version = Some(value),
            "about" => header.about = Some(value),
            "license" => header.license = Some(value),
            key => {
                if let Some(code) = key.strip_prefix("name.") {
                    header.names.push((code.to_string(), value));
                } else if let Some(code) = key.strip_prefix("about.") {
                    header.abouts.push((code.to_string(), value));
                }
            }
        }
    }
    named.then_some(header)
}

/// `value`, at most [`LONGEST`] characters.
fn cut(value: &str) -> String {
    value.chars().take(LONGEST).collect()
}

/// What the list shows of the app `id` whose file holds `text`, in `lang`.
pub fn info(id: &str, text: &str, built_in: bool, lang: Lang) -> Option<AppInfo> {
    let header = parse(text)?;
    Some(AppInfo {
        id: id.to_string(),
        name: header.name(lang).to_string(),
        kind: header.kind,
        authors: header.authors.clone(),
        version: header.version.clone(),
        about: header.about(lang).map(str::to_string),
        license: header.license.clone(),
        api: header.api,
        built_in,
        usable: header.api <= API,
    })
}
