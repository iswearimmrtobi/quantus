//! The native shell: application menu and the accelerators of the keyboard workflow (F10).
//!
//! Everything that is not typing runs through menu accelerators, so the shortcuts work no
//! matter what has focus. Items that belong to the UI are forwarded as a `quantus://shortcut`
//! event; zoom and fullscreen are handled here because only the shell can do them.

use std::sync::Mutex;

use quantus_core::settings::Language;
use tauri::menu::{Menu, MenuBuilder, MenuEvent, MenuItem, PredefinedMenuItem, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Manager, Runtime};

/// The event the renderer listens for; the payload is one of the ids in [`FORWARDED`].
pub const SHORTCUT_EVENT: &str = "quantus://shortcut";

/// Menu ids that the renderer handles itself. Mirrored by `Shortcut` in the UI.
pub const FORWARDED: [&str; 9] = [
    "new",
    "evaluate",
    "toggle-steps",
    "copy-result",
    "copy-all",
    "toggle-variables",
    "toggle-history",
    "toggle-latex",
    "toggle-settings",
];

const ZOOM_STEP: f64 = 0.1;
const ZOOM_MIN: f64 = 0.5;
const ZOOM_MAX: f64 = 3.0;

/// The current page zoom; the webview has no getter.
pub struct Zoom(Mutex<f64>);

impl Default for Zoom {
    fn default() -> Self {
        Self(Mutex::new(1.0))
    }
}

fn item<R: Runtime>(
    app: &AppHandle<R>,
    id: &str,
    label: &str,
    accelerator: Option<&str>,
) -> tauri::Result<MenuItem<R>> {
    MenuItem::with_id(app, id, label, true, accelerator)
}

pub fn build_menu<R: Runtime>(app: &AppHandle<R>, language: Language) -> tauri::Result<Menu<R>> {
    let l = |en: &'static str, de: &'static str| match language {
        Language::En => en,
        Language::De => de,
    };
    let calculation = SubmenuBuilder::new(app, l("Calculation", "Rechnung"))
        .item(&item(app, "new", l("New calculation", "Neue Rechnung"), Some("CmdOrCtrl+N"))?)
        .item(&item(app, "evaluate", l("Evaluate", "Auswerten"), Some("CmdOrCtrl+Enter"))?)
        .item(&item(app, "toggle-latex", l("Write LaTeX", "LaTeX schreiben"), Some("CmdOrCtrl+L"))?)
        .separator()
        .item(&item(app, "copy-result", l("Copy result", "Ergebnis kopieren"), Some("CmdOrCtrl+Shift+C"))?)
        .item(&item(app, "copy-all", l("Copy whole calculation", "Ganze Rechnung kopieren"), Some("CmdOrCtrl+Alt+C"))?)
        .separator()
        .item(&PredefinedMenuItem::quit(app, Some(l("Quit", "Beenden")))?)
        .build()?;

    let edit = SubmenuBuilder::new(app, l("Edit", "Bearbeiten"))
        .item(&PredefinedMenuItem::undo(app, Some(l("Undo", "Rückgängig")))?)
        .item(&PredefinedMenuItem::redo(app, Some(l("Redo", "Wiederherstellen")))?)
        .separator()
        .item(&PredefinedMenuItem::cut(app, Some(l("Cut", "Ausschneiden")))?)
        .item(&PredefinedMenuItem::copy(app, Some(l("Copy", "Kopieren")))?)
        .item(&PredefinedMenuItem::paste(app, Some(l("Paste", "Einfügen")))?)
        .item(&PredefinedMenuItem::select_all(app, Some(l("Select all", "Alles auswählen")))?)
        .build()?;

    let mut view = SubmenuBuilder::new(app, l("View", "Ansicht"))
        .item(&item(app, "toggle-steps", l("Steps", "Rechenweg"), Some("CmdOrCtrl+J"))?)
        .item(&item(app, "toggle-variables", l("Variables", "Variablen"), Some("CmdOrCtrl+,"))?)
        .item(&item(app, "toggle-history", l("History", "Verlauf"), Some("CmdOrCtrl+H"))?)
        .item(&item(app, "toggle-settings", l("Settings", "Einstellungen"), Some("CmdOrCtrl+."))?)
        .separator()
        .item(&item(app, "zoom-reset", l("Reset zoom", "Zoom zurücksetzen"), Some("CmdOrCtrl+0"))?)
        .item(&item(app, "zoom-in", l("Zoom in", "Größer"), Some("CmdOrCtrl+Plus"))?)
        .item(&item(app, "zoom-out", l("Zoom out", "Kleiner"), Some("CmdOrCtrl+-"))?)
        .item(&item(app, "fullscreen", l("Full screen", "Vollbild"), Some("F11"))?);
    if cfg!(debug_assertions) {
        view = view.item(&item(app, "devtools", l("Developer tools", "Entwicklerwerkzeuge"), Some("CmdOrCtrl+Shift+I"))?);
    }

    MenuBuilder::new(app).item(&calculation).item(&edit).item(&view.build()?).build()
}

pub fn on_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    let id = event.id().as_ref();
    if FORWARDED.contains(&id) {
        let _ = app.emit(SHORTCUT_EVENT, id);
        return;
    }
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    match id {
        "zoom-in" => zoom(app, &window, |z| z + ZOOM_STEP),
        "zoom-out" => zoom(app, &window, |z| z - ZOOM_STEP),
        "zoom-reset" => zoom(app, &window, |_| 1.0),
        "fullscreen" => {
            let on = window.is_fullscreen().unwrap_or(false);
            let _ = window.set_fullscreen(!on);
        }
        #[cfg(debug_assertions)]
        "devtools" => {
            if window.is_devtools_open() {
                window.close_devtools();
            } else {
                window.open_devtools();
            }
        }
        _ => {}
    }
}

fn zoom<R: Runtime>(
    app: &AppHandle<R>,
    window: &tauri::WebviewWindow<R>,
    change: impl FnOnce(f64) -> f64,
) {
    let state = app.state::<Zoom>();
    let mut current = state.0.lock().unwrap();
    // Round so repeated steps do not drift (0.1 is inexact in binary).
    let next = ((change(*current) * 100.0).round() / 100.0).clamp(ZOOM_MIN, ZOOM_MAX);
    if window.set_zoom(next).is_ok() {
        *current = next;
    }
}

/// Brings the window forward, as the second launch of the app asks for.
pub fn focus_main<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Replaces the application menu by one in `language`.
pub fn set_menu_language<R: Runtime>(app: &AppHandle<R>, language: Language) {
    if let Ok(menu) = build_menu(app, language) {
        let _ = app.set_menu(menu);
    }
}
