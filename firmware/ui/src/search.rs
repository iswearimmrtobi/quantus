//! CATALOG's search (firmware/docs/feature-ideas.md §1, now plans.md §6): a few letters find the
//! engine's constants, units, functions and prefixes together with the entries of SETUP and the
//! mode's OPTN. The ranking is the engine's (`quantus_search`), so `wurz` finds the same on the
//! handheld as on the desktop.

use quantus_search::{rank, Candidate, Query};

use crate::app::Mode;
use crate::backend::{IndexItem, IndexKind};
use crate::menu::Kind;
use crate::text::{Lang, Text};
use crate::tree::{self, Root};

/// The most hits a search lists.
pub const LIMIT: usize = 40;

/// The longest query.
pub const QUERY_LENGTH: usize = 24;

/// A line of the hits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub name: String,
    /// Shown at the right: the symbol or template.
    pub latex: Option<String>,
    /// Shown at the right without LaTeX: the page a menu entry stands on.
    pub detail: Option<String>,
    pub target: Target,
}

/// What choosing a hit does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// Inserts the LaTeX at the cursor.
    Insert(String),
    /// Opens the menu where the entry lives, on it: SETUP's settings, OPTN's pages.
    Open(Root, Vec<usize>),
    /// Opens the app.
    App(crate::applet::AppInfo),
}

/// Everything a search looks through, for one language and mode.
pub struct Index {
    pub lang: Lang,
    pub mode: Mode,
    /// The ids of the apps in it: the index is made again when they change.
    pub apps: Vec<String>,
    entries: Vec<(Hit, Candidate)>,
}

impl Index {
    /// The engine's `library` and the menus of `mode`, named in `lang`. OPTN's entries that insert
    /// what the engine already lists are left out.
    pub fn new(library: Vec<IndexItem>, mode: Mode, lang: Lang) -> Self {
        Index::with_apps(library, &[], mode, lang)
    }

    /// [`Index::new`] and the apps that can run, found by their names (none in Prüfungsmodus:
    /// the caller leaves them out).
    pub fn with_apps(library: Vec<IndexItem>, apps: &[crate::applet::AppInfo], mode: Mode, lang: Lang) -> Self {
        let mut entries = Vec::new();
        for item in &library {
            let insert = match item.kind {
                IndexKind::Unit => format!(r"\blacktriangleright {}", item.latex),
                IndexKind::Constant | IndexKind::Function | IndexKind::Prefix => item.latex.clone(),
            };
            let hit = Hit { name: item.name.clone(), latex: Some(item.latex.clone()), detail: None, target: Target::Insert(insert) };
            entries.push((hit, Candidate::new(item.names.iter().map(String::as_str), item.symbols.clone())));
        }
        for place in tree::places(mode) {
            let label = place.entry.label;
            let (latex, detail, target) = match &place.entry.kind {
                Kind::Insert(latex) if library.iter().any(|item| item.latex == *latex) => continue,
                Kind::Insert(latex) => (Some(latex.to_string()), None, Target::Insert(latex.to_string())),
                _ => (None, Some(place.parent.get(lang).to_string()), Target::Open(place.root, place.path)),
            };
            let hit = Hit { name: label.get(lang).to_string(), latex, detail, target };
            entries.push((hit, candidate(label, lang)));
        }
        let apps: Vec<&crate::applet::AppInfo> = apps.iter().filter(|app| app.usable).collect();
        for app in &apps {
            let hit = Hit { name: app.name.clone(), latex: None, detail: Some(crate::text::APPS.get(lang).to_string()), target: Target::App((*app).clone()) };
            entries.push((hit, Candidate::new([app.name.as_str(), app.id.as_str()], Vec::new())));
        }
        let apps = apps.iter().map(|app| app.id.clone()).collect();
        Index { lang, mode, apps, entries }
    }

    /// The best hits for `query`, best first: by how well they match, then the shorter name.
    pub fn search(&self, query: &str) -> Vec<Hit> {
        let query = Query::new(query);
        let mut ranked: Vec<_> = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(index, (hit, candidate))| rank(candidate, &query).map(|tier| (tier, hit.name.chars().count(), index, hit)))
            .collect();
        ranked.sort_by_key(|(tier, length, index, _)| (*tier, *length, *index));
        let mut hits: Vec<Hit> = Vec::new();
        for (.., hit) in ranked {
            // The same entry on two pages of the same title once.
            if !hits.iter().any(|known| known.name == hit.name && known.latex == hit.latex && known.detail == hit.detail) {
                hits.push(hit.clone());
            }
            if hits.len() == LIMIT {
                break;
            }
        }
        hits
    }
}

/// A menu label's names in `lang` and in English.
fn candidate(label: Text, lang: Lang) -> Candidate {
    Candidate::new([label.get(lang), label.get(Lang::En)], Vec::new())
}
