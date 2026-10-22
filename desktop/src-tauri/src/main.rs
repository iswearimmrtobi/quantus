#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn app<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder
        .manage(commands::Engine::default())
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
