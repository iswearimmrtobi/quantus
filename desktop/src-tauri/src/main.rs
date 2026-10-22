#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    tauri::Builder::default()
        .manage(commands::Engine::default())
        .invoke_handler(tauri::generate_handler![
            commands::engine_info,
            commands::evaluate,
            commands::define_variable,
            commands::list_variables,
            commands::delete_variable,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Quantus");
}
