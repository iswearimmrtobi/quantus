#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod shell;

fn app<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder
        // Links to the outside belong in the browser, not in the app window.
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(commands::Engine::default())
        .manage(shell::Zoom::default())
        .menu(|app| shell::build_menu(app))
        .on_menu_event(shell::on_menu_event)
        .invoke_handler(tauri::generate_handler![
            commands::engine_info,
            commands::evaluate,
            commands::define_variable,
            commands::list_variables,
            commands::delete_variable,
        ])
}

fn main() {
    // Not part of `app()`: it needs a real application identifier (a D-Bus name on Linux), which
    // the mock runtime of the tests does not have.
    let single_instance = tauri_plugin_single_instance::init(|app, _args, _cwd| {
        shell::focus_main(app);
    });
    app(tauri::Builder::default().plugin(single_instance))
        .run(tauri::generate_context!())
        .expect("error while running Quantus");
}

#[cfg(test)]
mod tests;
