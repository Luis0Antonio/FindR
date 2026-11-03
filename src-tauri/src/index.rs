use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::Duration;

use fs_core::index::Index;
use fs_core::search::PackageFn;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;

const SETTLE: Duration = Duration::from_millis(300);

#[derive(Default)]
struct State {
    roots: Vec<PathBuf>,
    excluded: HashSet<String>,
    index: Index,
    building: bool,
    generation: u64,
    watcher: Option<RecommendedWatcher>,
}

fn shared() -> &'static RwLock<State> {
    static STATE: OnceLock<RwLock<State>> = OnceLock::new();
    STATE.get_or_init(Default::default)
}

fn pending() -> &'static Mutex<HashSet<PathBuf>> {
    static PENDING: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
    PENDING.get_or_init(Default::default)
}

static SETTLING: AtomicBool = AtomicBool::new(false);

fn is_package() -> PackageFn {
    Arc::new(crate::commands::is_package)
}

pub fn excluded() -> HashSet<String> {
    shared().read().unwrap().excluded.clone()
}

// File events always carry real paths, so the index is keyed by them.
fn real(path: &str) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path))
}

/// Matching paths below `root`, or None when the index doesn't cover it (yet).
/// Results are given below `root` as passed, even when it leads through a symlink.
pub fn find(root: &Path, query: &str) -> Option<Vec<PathBuf>> {
    let real_root = real(&root.to_string_lossy());
    let state = shared().read().unwrap();
    let found = state.index.covers(&real_root).then(|| state.index.find(&real_root, &query.to_lowercase()))?;
    Some(found.into_iter().map(|p| root.join(p.strip_prefix(&real_root).unwrap_or(&p))).collect())
}

#[tauri::command]
pub fn configure_index(roots: Vec<String>, excluded: Vec<String>) {
    let roots: Vec<PathBuf> = roots.iter().map(|r| real(r)).collect();
    let excluded: HashSet<String> = excluded.into_iter().collect();
    {
        let state = shared().read().unwrap();
        if state.roots == roots && state.excluded == excluded && (state.building || !state.index.is_empty() || roots.is_empty()) {
            return;
        }
    }
    let mut state = shared().write().unwrap();
    state.roots = roots;
    state.excluded = excluded;
    state.watcher = watch(&state.roots);
    rebuild(&mut state);
}

fn rebuild(state: &mut State) {
    state.generation += 1;
    state.index = Index::default();
    state.building = !state.roots.is_empty();
    pending().lock().unwrap().clear();
    let (generation, roots, excluded) = (state.generation, state.roots.clone(), state.excluded.clone());
    std::thread::spawn(move || {
        let mut index = Index::default();
        for root in &roots {
            index.add_tree(root, &excluded, &is_package());
        }
        let mut state = shared().write().unwrap();
        if state.generation == generation {
            state.index = index;
            state.building = false;
            apply(&mut state);
        }
    });
}

fn watch(roots: &[PathBuf]) -> Option<RecommendedWatcher> {
    let mut watcher = notify::recommended_watcher(|res: notify::Result<notify::Event>| {
        let Ok(event) = res else { return };
        if event.need_rescan() {
            let mut state = shared().write().unwrap();
            rebuild(&mut state);
            return;
        }
        pending().lock().unwrap().extend(event.paths);
        if !SETTLING.swap(true, Ordering::SeqCst) {
            std::thread::spawn(|| {
                std::thread::sleep(SETTLE);
                SETTLING.store(false, Ordering::SeqCst);
                let mut state = shared().write().unwrap();
                if !state.building {
                    apply(&mut state);
                }
            });
        }
    })
    .ok()?;
    for root in roots {
        let _ = watcher.watch(root, RecursiveMode::Recursive);
    }
    Some(watcher)
}

/// Re-reads the folders that changed: the parent of each reported path, and the path itself if it's an indexed folder.
fn apply(state: &mut State) {
    let paths = std::mem::take(&mut *pending().lock().unwrap());
    let mut dirs: HashSet<PathBuf> = HashSet::new();
    for path in paths {
        if let Some(parent) = path.parent() {
            dirs.insert(parent.to_path_buf());
        }
        dirs.insert(path);
    }
    let (excluded, pkg) = (state.excluded.clone(), is_package());
    for dir in dirs {
        state.index.refresh_dir(&dir, &excluded, &pkg);
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexStatus {
    building: bool,
    items: usize,
}

#[tauri::command]
pub fn index_status() -> IndexStatus {
    let state = shared().read().unwrap();
    IndexStatus { building: state.building, items: state.index.len() }
}
