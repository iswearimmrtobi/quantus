#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;

mod commands;
mod shell;

fn app<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder
        // Links to the outside belong in the browser, not in the app window.
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::Engine::default())
        .manage(shell::Zoom::default())
        .menu(|app| shell::build_menu(app, quantus_core::settings::Language::En))
        .on_menu_event(shell::on_menu_event)
        .invoke_handler(tauri::generate_handler![
            commands::engine_info,
            commands::evaluate,
            commands::define_variable,
            commands::solve_circuit,
            commands::load_circuit,
            commands::save_circuit,
            commands::analyze,
            commands::sample_function,
            commands::tangent,
            commands::area_under_curve,
            commands::value_table,
            commands::list_variables,
            commands::delete_variable,
            commands::get_settings,
            commands::set_settings,
        ])
}

fn main() {
    std::panic::set_hook(Box::new(|info| fatal(&info.to_string())));
    // Not part of `app()`: it needs a real application identifier (a D-Bus name on Linux), which
    // the mock runtime of the tests does not have.
    let single_instance = tauri_plugin_single_instance::init(|app, _args, _cwd| {
        shell::focus_main(app);
    });
    app(tauri::Builder::default().plugin(single_instance))
        // Here rather than in `app()`, so the tests never read or write the user's settings.
        .setup(|app| {
            commands::restore_settings(app.handle());
            shell::set_menu_language(app.handle(), app.state::<commands::Engine>().language());
            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|error| fatal(&error.to_string()));
}

/// The settings are not readable this early, so the system's language decides.
fn system_is_german() -> bool {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .find_map(|name| std::env::var(name).ok().filter(|value| !value.is_empty()))
        .or_else(system_locale)
        .is_some_and(|value| value.starts_with("de"))
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

#[cfg(not(windows))]
fn system_locale() -> Option<String> {
    None
}

/// The engine lives in the process, so what can still fail is the shell itself (no display, no
/// webview library). Say so in a dialog instead of exiting silently when started from a launcher.
fn fatal(message: &str) -> ! {
    eprintln!("Quantus: {message}");
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title(if system_is_german() { "Quantus startet nicht" } else { "Quantus cannot start" })
        .set_description(message)
        .show();
    std::process::exit(1);
}

#[cfg(test)]
mod tests;
