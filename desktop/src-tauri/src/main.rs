#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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
        .menu(|app| shell::build_menu(app))
        .on_menu_event(shell::on_menu_event)
        .invoke_handler(tauri::generate_handler![
            commands::engine_info,
            commands::evaluate,
            commands::define_variable,
            commands::solve_circuit,
            commands::load_circuit,
            commands::save_circuit,
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
            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|error| fatal(&error.to_string()));
}

/// The engine lives in the process, so what can still fail is the shell itself (no display, no
/// webview library). Say so in a dialog instead of exiting silently when started from a launcher.
fn fatal(message: &str) -> ! {
    eprintln!("Quantus: {message}");
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("Quantus startet nicht")
        .set_description(message)
        .show();
    std::process::exit(1);
}

#[cfg(test)]
mod tests;
