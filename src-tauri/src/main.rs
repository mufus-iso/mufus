#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .unwrap_or_else(|error| {
            eprintln!("Failed to start mufus: {error}");
            std::process::exit(1);
        });
}
