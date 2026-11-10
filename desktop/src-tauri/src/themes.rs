//! The colour themes: the built-in ones of `quantus-themes` and the user's own `*.toml` files in
//! `themes/` of the app's config directory. The folder is read again on every call, so a theme
//! the user just saved shows after a reload, without a restart.

use std::fs;
use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime, State};

use crate::commands::Engine;

/// The folder of the user's themes, made if missing.
fn themes_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?.join("themes");
    fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    Ok(dir)
}

/// The user's theme files: (id, text), in no order. Files that cannot be read are left out.
fn user_themes<R: Runtime>(app: &AppHandle<R>) -> Vec<(String, String)> {
    let Ok(dir) = themes_dir(app) else { return Vec::new() };
    let Ok(entries) = fs::read_dir(&dir) else { return Vec::new() };
    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension()?.to_str()? != "toml" {
                return None;
            }
            let id = path.file_stem()?.to_str()?.to_string();
            Some((id, fs::read_to_string(&path).ok()?))
        })
        .collect()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeInfo {
    id: String,
    /// In the language of the settings.
    name: String,
    dark: bool,
    built_in: bool,
    /// Why the file cannot be used; such a theme cannot be chosen.
    error: Option<String>,
    warnings: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Themes {
    /// The folder of the user's themes, to show; empty if there is none.
    dir: String,
    themes: Vec<ThemeInfo>,
}

#[tauri::command]
pub fn themes<R: Runtime>(app: AppHandle<R>, engine: State<Engine>) -> Themes {
    let code = engine.language().code();
    let themes = quantus_themes::list(user_themes(&app))
        .into_iter()
        .map(|entry| match entry.theme {
            Ok(theme) => ThemeInfo {
                name: theme.name(code).to_string(),
                dark: theme.dark,
                warnings: theme.warnings,
                id: entry.id,
                built_in: entry.built_in,
                error: None,
            },
            Err(error) => ThemeInfo {
                name: entry.id.clone(),
                dark: true,
                warnings: Vec::new(),
                id: entry.id,
                built_in: entry.built_in,
                error: Some(error.to_string()),
            },
        })
        .collect();
    let dir = themes_dir(&app).map(|dir| dir.display().to_string()).unwrap_or_default();
    Themes { dir, themes }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeColors {
    /// The theme shown: the one asked for, or the default one if that cannot be used.
    id: String,
    dark: bool,
    /// CSS variables and their values: `["--background", "#17161d"]`.
    vars: Vec<(String, String)>,
    /// Why the theme asked for is not shown.
    error: Option<String>,
}

/// The colours of the theme `id`; the default theme's, with the reason, if it cannot be used.
#[tauri::command]
pub fn theme<R: Runtime>(app: AppHandle<R>, id: String) -> ThemeColors {
    let (theme, error) = match quantus_themes::find(&id, user_themes(&app)) {
        Ok(theme) => (theme, None),
        Err(error) => (quantus_themes::default_theme(), Some(format!("{id}: {error}"))),
    };
    ThemeColors { id: theme.id.clone(), dark: theme.dark, vars: theme.css_vars(), error }
}

/// Opens the folder of the user's themes in the file manager.
#[tauri::command]
pub fn open_themes_folder<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let dir = themes_dir(&app)?;
    app.opener().open_path(dir.display().to_string(), None::<&str>).map_err(|e| e.to_string())
}
