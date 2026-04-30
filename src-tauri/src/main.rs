#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use image_analyzer::commands;

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
