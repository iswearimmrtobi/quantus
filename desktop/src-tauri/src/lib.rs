//! The Tauri shell. A library so the same code runs as the desktop binary (`main.rs`) and as the
//! Android app, which loads it as a shared library through `mobile_entry_point`.

#[cfg(desktop)]
use tauri::Manager;

mod commands;
#[cfg(desktop)]
mod shell;

fn app<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    let builder = builder
        // Links to the outside belong in the browser, not in the app window.
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::Engine::default());
    #[cfg(mobile)]
    let builder = builder.plugin(tauri_plugin_fs::init());
    // Phones have no menu bar; the UI offers the same actions as buttons.
    #[cfg(desktop)]
    let builder = builder
        .manage(shell::Zoom::default())
        .menu(|app| shell::build_menu(app, quantus_core::settings::Language::En))
        .on_menu_event(shell::on_menu_event);
    builder
        .invoke_handler(tauri::generate_handler![
            commands::engine_info,
            commands::evaluate,
            commands::define_variable,
            commands::engineering,
            commands::free_variables,
            commands::calc,
            commands::solve_for,
            commands::new_session,
            commands::open_session,
            commands::list_sessions,
            commands::rename_session,
            commands::duplicate_session,
            commands::delete_session,
            commands::export_session,
            commands::import_session,
            commands::history,
            commands::clear_history,
            commands::get_mode_inputs,
            commands::set_mode_inputs,
            commands::statistics,
            commands::distribution,
            commands::constants,
            commands::conversion_units,
            commands::search_library,
            commands::solve_circuit,
            commands::solve_kirchhoff,
            commands::geometry,
            commands::geometry_view,
            commands::load_circuit,
            commands::save_circuit,
            commands::analyze,
            commands::sample_function,
            commands::tangent,
            commands::area_under_curve,
            commands::value_table,
            commands::solve_linear_system,
            commands::list_variables,
            commands::writeup,
            commands::delete_variable,
            commands::get_settings,
            commands::set_settings,
        ])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    std::panic::set_hook(Box::new(|info| fatal(&info.to_string())));
    let builder = tauri::Builder::default();
    // Not part of `app()`: it needs a real application identifier (a D-Bus name on Linux), which
    // the mock runtime of the tests does not have. Android starts an app only once anyway.
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        shell::focus_main(app);
    }));
    app(builder)
        // Here rather than in `app()`, so the tests never read or write the user's settings and sessions.
        .setup(|app| {
            commands::restore_settings(app.handle());
            commands::restore_sessions(app.handle());
            #[cfg(desktop)]
            shell::set_menu_language(app.handle(), app.state::<commands::Engine>().language());
            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|error| fatal(&error.to_string()));
}

/// The settings are not readable this early, so the system's language decides: `de_DE.UTF-8`
/// is German, a language Quantus does not have is English.
#[cfg(desktop)]
fn system_language() -> quantus_core::settings::Language {
    use quantus_core::settings::Language;
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .find_map(|name| std::env::var(name).ok().filter(|value| !value.is_empty()))
        .or_else(system_locale)
        .and_then(|value| Language::from_code(&value.get(..2)?.to_lowercase()))
        .unwrap_or_default()
}

/// Windows does not set `LANG`; the user locale (`de-DE`, `en-US`, …) stands in for it.
#[cfg(windows)]
fn system_locale() -> Option<String> {
    use windows_sys::Win32::Globalization::GetUserDefaultLocaleName;
    let mut name = [0u16; 85]; // LOCALE_NAME_MAX_LENGTH
    let written = unsafe { GetUserDefaultLocaleName(name.as_mut_ptr(), name.len() as i32) };
    // `written` counts the terminating NUL; 0 means the call failed.
    (written > 1).then(|| String::from_utf16_lossy(&name[..written as usize - 1]))
}

#[cfg(all(desktop, not(windows)))]
fn system_locale() -> Option<String> {
    None
}

/// The engine lives in the process, so what can still fail is the shell itself (no display, no
/// webview library). Say so in a dialog instead of exiting silently when started from a launcher.
#[cfg(desktop)]
fn fatal(message: &str) -> ! {
    eprintln!("Quantus: {message}");
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title(quantus_texts::tr!(system_language(), "shell.cannot_start"))
        .set_description(message)
        .show();
    std::process::exit(1);
}

/// On Android stderr ends up in logcat; the system shows that the app stopped.
#[cfg(mobile)]
fn fatal(message: &str) -> ! {
    eprintln!("Quantus: {message}");
    std::process::exit(1);
}

#[cfg(test)]
mod tests;
