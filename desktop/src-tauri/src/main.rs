#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod shell;

fn app<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder
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
    app(tauri::Builder::default())
        .run(tauri::generate_context!())
        .expect("error while running Quantus");
}

#[cfg(test)]
mod tests;
