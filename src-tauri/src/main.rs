#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "macos")]
#[tauri::command]
async fn check_disk_discovery() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(mufus_core::macos::collect_external_snapshot)
        .await
        .map_err(|_| "The disk check could not be completed.".to_owned())?
        .map(|_| ())
        .map_err(|_| "The disk check failed. Check your devices and try again.".to_owned())
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
fn check_disk_discovery() -> Result<(), String> {
    Err("The disk check is currently available only on macOS.".to_owned())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![check_disk_discovery])
        .run(tauri::generate_context!())
        .unwrap_or_else(|error| {
            eprintln!("Failed to start mufus: {error}");
            std::process::exit(1);
        });
}
