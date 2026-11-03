use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use fs_core::search::{search, stream, SearchOptions};
use fs_core::FileKind;
use serde_json::json;
use tauri::{AppHandle, Emitter};

const LIMIT: usize = 10_000;

fn running() -> &'static Mutex<HashMap<String, Arc<AtomicBool>>> {
    static RUNNING: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();
    RUNNING.get_or_init(Default::default)
}

#[tauri::command]
pub fn start_search(
    app: AppHandle,
    search_id: String,
    root: String,
    query: String,
    kinds: Vec<FileKind>,
    show_hidden: bool,
) -> String {
    let id = search_id;
    let cancel = Arc::new(AtomicBool::new(false));
    running().lock().unwrap().insert(id.clone(), cancel.clone());
    let search_id = id.clone();
    std::thread::spawn(move || {
        let root = PathBuf::from(root);
        let indexed = if show_hidden { None } else { crate::index::find(&root, &query) };
        let opts = SearchOptions { query, kinds, show_hidden, excluded: crate::index::excluded(), limit: LIMIT };
        let emit = |entries| {
            let _ = app.emit("search://results", json!({ "searchId": search_id, "entries": entries, "done": false }));
        };
        let outcome = match indexed {
            Some(paths) => stream(paths.into_iter(), &opts, &crate::commands::is_package, &cancel, emit),
            None => search(&root, &opts, Arc::new(crate::commands::is_package), &cancel, emit),
        };
        running().lock().unwrap().remove(&search_id);
        if !outcome.cancelled {
            let _ = app.emit(
                "search://results",
                json!({ "searchId": search_id, "entries": [], "done": true, "truncated": outcome.truncated }),
            );
        }
    });
    id
}

#[tauri::command]
pub fn cancel_search(search_id: String) {
    if let Some(flag) = running().lock().unwrap().remove(&search_id) {
        flag.store(true, Ordering::Relaxed);
    }
}
