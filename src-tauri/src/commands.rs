use std::path::{Path, PathBuf};

use fs_core::{FileEntry, FsError, FsResult};
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

pub fn is_package(path: &Path, ext: &str) -> bool {
    fs_core::kind::is_package_ext(ext) || crate::macos::icons::is_file_package(&path.to_string_lossy())
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> FsResult<T> + Send + 'static) -> FsResult<T> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| FsError::Io(e.to_string()))?
}

#[tauri::command]
pub async fn list_dir(path: String, show_hidden: bool) -> FsResult<Vec<FileEntry>> {
    blocking(move || fs_core::list_dir(&PathBuf::from(path), show_hidden, &is_package)).await
}

#[tauri::command]
pub async fn open_path(app: AppHandle, path: String) -> FsResult<()> {
    app.opener().open_path(path, None::<&str>).map_err(|e| FsError::Io(e.to_string()))
}

#[tauri::command]
pub async fn open_privacy_settings(app: AppHandle) -> FsResult<()> {
    app.opener()
        .open_url("x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles", None::<&str>)
        .map_err(|e| FsError::Io(e.to_string()))
}

#[tauri::command]
pub fn home_dir() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/".into())
}

#[tauri::command]
pub async fn reveal_in_finder(app: AppHandle, paths: Vec<String>) -> FsResult<()> {
    app.opener().reveal_items_in_dir(paths).map_err(|e| FsError::Io(e.to_string()))
}

#[tauri::command]
pub async fn drag_out(app: AppHandle, paths: Vec<String>) {
    crate::macos::dragout::start(&app, paths);
}

#[tauri::command]
pub fn quicklook_show(app: AppHandle, paths: Vec<String>, index: usize) {
    crate::macos::quicklook::show(&app, paths, index);
}

#[tauri::command]
pub fn quicklook_update(app: AppHandle, paths: Vec<String>, index: usize) {
    crate::macos::quicklook::update(&app, paths, index);
}

#[tauri::command]
pub fn quicklook_close(app: AppHandle) {
    crate::macos::quicklook::close(&app);
}

#[tauri::command]
pub async fn create_folder(parent: String) -> FsResult<FileEntry> {
    blocking(move || fs_core::ops::create_folder(Path::new(&parent), &is_package)).await
}

#[tauri::command]
pub async fn rename(path: String, new_name: String) -> FsResult<FileEntry> {
    blocking(move || fs_core::ops::rename(Path::new(&path), &new_name, &is_package)).await
}

#[tauri::command]
pub fn list_volumes() -> Vec<crate::macos::volumes::Volume> {
    crate::macos::volumes::list_volumes()
}

#[tauri::command]
pub fn paths_exist(paths: Vec<String>) -> Vec<bool> {
    paths.iter().map(|p| Path::new(p).is_dir()).collect()
}

const PROGRESS_THRESHOLD: u64 = 50 * 1000 * 1000;

#[tauri::command]
pub async fn move_items(
    app: AppHandle,
    sources: Vec<String>,
    target_dir: String,
    copy: bool,
    policy: Option<fs_core::ops::ConflictPolicy>,
    op_id: String,
) -> FsResult<fs_core::ops::OpResult> {
    use fs_core::ops::{device_of, move_items, total_size, Progress};
    use tauri::Emitter;
    blocking(move || {
        let target = PathBuf::from(&target_dir);
        let target_dev = device_of(&target).ok();
        let copies_bytes = copy || sources.iter().any(|s| device_of(Path::new(s)).ok() != target_dev);
        let total = if copies_bytes { sources.iter().map(|s| total_size(Path::new(s))).sum() } else { 0 };
        let mut last = std::time::Instant::now();
        let mut report = |done: u64, total: u64| {
            if done == total || last.elapsed() >= std::time::Duration::from_millis(100) {
                last = std::time::Instant::now();
                let _ = app.emit(
                    "op://progress",
                    serde_json::json!({ "opId": op_id, "doneBytes": done, "totalBytes": total }),
                );
            }
        };
        let progress = (total > PROGRESS_THRESHOLD).then(|| Progress { total, done: 0, report: &mut report });
        Ok(move_items(&sources, &target, copy, policy, progress))
    })
    .await
}

#[tauri::command]
pub fn volume_of(path: String) -> FsResult<u64> {
    fs_core::ops::device_of(Path::new(&path)).map_err(|e| FsError::from_io(e, &path))
}

#[tauri::command]
pub async fn convert_file(path: String, target: String, keep_original: bool) -> FsResult<String> {
    blocking(move || {
        crate::convert::convert_file(Path::new(&path), &target, keep_original).map(|p| p.to_string_lossy().into_owned())
    })
    .await
}

#[tauri::command]
pub async fn move_to_trash(paths: Vec<String>) -> FsResult<()> {
    blocking(move || {
        let failures: Vec<String> = paths
            .iter()
            .filter_map(|p| fs_core::ops::move_to_trash(Path::new(p)).err().map(|e| format!("“{}”: {e}", display_name(p))))
            .collect();
        match failures.as_slice() {
            [] => Ok(()),
            [one] => Err(FsError::Io(format!("Couldn't move {one}"))),
            [first, rest @ ..] => Err(FsError::Io(format!("Couldn't move {first} (and {} more)", rest.len()))),
        }
    })
    .await
}

fn display_name(path: &str) -> String {
    Path::new(path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.to_string())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardFiles {
    paths: Vec<String>,
    change_count: isize,
}

#[tauri::command]
pub fn clipboard_write_files(paths: Vec<String>) -> isize {
    crate::macos::clipboard::write_files(&paths)
}

#[tauri::command]
pub fn clipboard_read_files() -> ClipboardFiles {
    let (paths, change_count) = crate::macos::clipboard::read_files();
    ClipboardFiles { paths, change_count }
}
