use std::path::{Path, PathBuf};
use std::sync::Mutex;

use percent_encoding::percent_decode_str;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, Url};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenTarget {
    path: String,
    select: Option<String>,
}

#[derive(Default)]
struct Inner {
    ready: bool,
    pending: Option<OpenTarget>,
}

#[derive(Default)]
pub struct Links(Mutex<Inner>);

/// Handles `rfinder://` links and files or folders opened with the app (Dock, "Open With").
pub fn handle_opened(app: &AppHandle, urls: &[Url]) {
    let Some(target) = urls.iter().find_map(path_for).map(|p| target_for(&p)) else { return };
    let state = app.state::<Links>();
    let mut links = state.0.lock().unwrap();
    // A link that launches the app arrives before the UI exists; it picks it up from `startup_target`.
    if !links.ready {
        links.pending = Some(target);
        return;
    }
    let _ = app.emit("link://open", target);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[tauri::command]
pub fn startup_target(links: State<Links>) -> Option<OpenTarget> {
    let mut links = links.0.lock().unwrap();
    links.ready = true;
    links.pending.take().or_else(|| std::env::args().skip(1).find(|a| Path::new(a).is_dir()).map(|path| OpenTarget { path, select: None }))
}

fn path_for(url: &Url) -> Option<PathBuf> {
    match url.scheme() {
        "file" => url.to_file_path().ok(),
        "rfinder" => {
            // Accept rfinder:///Users/me, rfinder://Users/me and rfinder://~/Downloads alike.
            let raw = format!("{}{}", url.host_str().unwrap_or(""), url.path());
            let decoded = percent_decode_str(&raw).decode_utf8().ok()?;
            let trimmed = decoded.trim_start_matches('/').trim_end_matches('/');
            if trimmed.is_empty() {
                return None;
            }
            let home = std::env::var("HOME").unwrap_or_else(|_| "/".into());
            Some(match trimmed.strip_prefix('~') {
                Some(rest) => PathBuf::from(format!("{home}{rest}")),
                None => PathBuf::from(format!("/{trimmed}")),
            })
        }
        _ => None,
    }
}

fn target_for(path: &Path) -> OpenTarget {
    let as_string = |p: &Path| p.to_string_lossy().into_owned();
    match path.parent() {
        Some(parent) if path.exists() && !path.is_dir() => OpenTarget { path: as_string(parent), select: Some(as_string(path)) },
        _ => OpenTarget { path: as_string(path), select: None },
    }
}
