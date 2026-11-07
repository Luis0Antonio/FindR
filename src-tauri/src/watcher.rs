use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

const DEBOUNCE: Duration = Duration::from_millis(150);
const ROOT_POLL: Duration = Duration::from_secs(1);

struct PaneWatch {
    _watcher: RecommendedWatcher,
    alive: Arc<AtomicBool>,
}

impl Drop for PaneWatch {
    fn drop(&mut self) {
        self.alive.store(false, Ordering::SeqCst);
    }
}

fn watches() -> &'static Mutex<HashMap<String, PaneWatch>> {
    static WATCHES: OnceLock<Mutex<HashMap<String, PaneWatch>>> = OnceLock::new();
    WATCHES.get_or_init(Default::default)
}

fn emit_changed(app: &AppHandle, payload: &Value) {
    let _ = app.emit("dir://changed", payload.clone());
}

/// notify-debouncer-full drops FSEvents removals (they arrive with the "created" flag still set and get
/// collapsed into nothing), so events are coalesced here instead.
#[tauri::command]
pub fn watch_dir(app: AppHandle, pane_id: String, path: String) {
    let mut map = watches().lock().unwrap();
    map.remove(&pane_id);
    let payload = json!({ "paneId": pane_id, "path": path });
    let pending = Arc::new(AtomicBool::new(false));
    let (ev_app, ev_payload) = (app.clone(), payload.clone());
    let watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if res.is_err() || pending.swap(true, Ordering::SeqCst) {
            return;
        }
        let (app, pending, payload) = (ev_app.clone(), pending.clone(), ev_payload.clone());
        std::thread::spawn(move || {
            std::thread::sleep(DEBOUNCE);
            pending.store(false, Ordering::SeqCst);
            emit_changed(&app, &payload);
        });
    });
    let Ok(mut watcher) = watcher else { return };
    if watcher.watch(Path::new(&path), RecursiveMode::NonRecursive).is_err() {
        return;
    }

    // FSEvents does not report the deletion of the watched folder itself, so poll for its existence.
    let alive = Arc::new(AtomicBool::new(true));
    let poll_alive = alive.clone();
    let dir = PathBuf::from(&path);
    std::thread::spawn(move || {
        while poll_alive.load(Ordering::SeqCst) {
            std::thread::sleep(ROOT_POLL);
            if poll_alive.load(Ordering::SeqCst) && !dir.exists() {
                emit_changed(&app, &payload);
                break;
            }
        }
    });
    map.insert(pane_id, PaneWatch { _watcher: watcher, alive });
}

#[tauri::command]
pub fn unwatch_dir(pane_id: String) {
    watches().lock().unwrap().remove(&pane_id);
}

/// Watch keys start with the window's label, so a closed window's watches can be dropped together.
pub fn unwatch_window(label: &str) {
    let prefix = format!("{label}:");
    watches().lock().unwrap().retain(|key, _| !key.starts_with(&prefix));
}
