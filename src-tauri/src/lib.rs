#[cfg(target_os = "macos")]
mod macos;

use serde::Serialize;

#[derive(Serialize)]
struct SpikeEntry {
    path: String,
    name: String,
    is_dir: bool,
}

#[tauri::command]
fn spike_list(path: String) -> Vec<SpikeEntry> {
    let mut out: Vec<SpikeEntry> = std::fs::read_dir(&path)
        .map(|rd| {
            rd.flatten()
                .map(|e| SpikeEntry {
                    path: e.path().to_string_lossy().into_owned(),
                    name: e.file_name().to_string_lossy().into_owned(),
                    is_dir: e.file_type().map(|t| t.is_dir()).unwrap_or(false),
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

#[tauri::command]
fn debug_log(msg: String) {
    println!("[ui] {msg}");
}

#[tauri::command]
fn quicklook_show(app: tauri::AppHandle, paths: Vec<String>, index: usize) {
    macos::quicklook::show(&app, paths, index);
}

#[tauri::command]
fn quicklook_update(app: tauri::AppHandle, paths: Vec<String>, index: usize) {
    macos::quicklook::update(&app, paths, index);
}

#[tauri::command]
fn quicklook_close(app: tauri::AppHandle) {
    macos::quicklook::close(&app);
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            macos::quicklook::init(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![spike_list, debug_log, quicklook_show, quicklook_update, quicklook_close])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
