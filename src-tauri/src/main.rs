#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod analyzer;
mod commands;
mod types;
mod utils;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::analyze_image,
            commands::analyze_batch,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
