use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use jwalk::{Parallelism, WalkDir};

use crate::entry::{extension_of, FileEntry, FileKind};

pub type PackageFn = Arc<dyn Fn(&Path, &str) -> bool + Send + Sync>;

pub struct SearchOptions {
    pub query: String,
    pub kinds: Vec<FileKind>,
    pub show_hidden: bool,
    /// Folder names never descended into or listed, such as `node_modules`.
    pub excluded: HashSet<String>,
    pub limit: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SearchOutcome {
    pub truncated: bool,
    pub cancelled: bool,
}

const BATCH_SIZE: usize = 200;
const BATCH_INTERVAL: Duration = Duration::from_millis(50);

fn kind_matches(kinds: &[FileKind], kind: FileKind) -> bool {
    let kind = if kind == FileKind::Package { FileKind::Other } else { kind };
    kinds.is_empty() || kinds.contains(&kind)
}

/// Streams name matches below `root` in batches. Never descends into packages; skips hidden entries unless asked.
pub fn search(
    root: &Path,
    opts: &SearchOptions,
    is_package: PackageFn,
    cancel: &AtomicBool,
    on_batch: impl FnMut(Vec<FileEntry>),
) -> SearchOutcome {
    let query = opts.query.to_lowercase();
    let pkg = is_package.clone();
    let excluded = opts.excluded.clone();
    let walker = WalkDir::new(root)
        .skip_hidden(!opts.show_hidden)
        .follow_links(false)
        .parallelism(Parallelism::RayonNewPool(0))
        .process_read_dir(move |_, _, _, children| {
            children.retain(|c| c.as_ref().map_or(true, |c| !(c.file_type.is_dir() && excluded.contains(&*c.file_name.to_string_lossy()))));
            for child in children.iter_mut().flatten() {
                if child.file_type.is_dir() {
                    let name = child.file_name.to_string_lossy();
                    if let Some(ext) = extension_of(&name) {
                        if pkg(&child.path(), &ext) {
                            child.read_children = None;
                        }
                    }
                }
            }
        });

    let candidates = walker
        .into_iter()
        .take_while(|_| !cancel.load(Ordering::Relaxed))
        .flatten()
        .filter(|dent| dent.depth > 0 && dent.file_name.to_string_lossy().to_lowercase().contains(&query))
        .map(|dent| dent.path());
    stream(candidates, opts, &*is_package, cancel, on_batch)
}

/// Streams the candidates that pass the hidden and kind filters, in batches, up to the limit.
pub fn stream(
    candidates: impl Iterator<Item = PathBuf>,
    opts: &SearchOptions,
    is_package: &dyn Fn(&Path, &str) -> bool,
    cancel: &AtomicBool,
    mut on_batch: impl FnMut(Vec<FileEntry>),
) -> SearchOutcome {
    let mut batch = Vec::new();
    let mut last_flush = Instant::now();
    let mut found = 0usize;
    let mut outcome = SearchOutcome { truncated: false, cancelled: false };

    for path in candidates {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let Ok(entry) = FileEntry::from_path(&path, is_package) else { continue };
        if (entry.is_hidden && !opts.show_hidden) || !kind_matches(&opts.kinds, entry.kind) {
            continue;
        }
        if found == opts.limit {
            outcome.truncated = true;
            break;
        }
        found += 1;
        batch.push(entry);
        if batch.len() >= BATCH_SIZE || last_flush.elapsed() >= BATCH_INTERVAL {
            on_batch(std::mem::take(&mut batch));
            last_flush = Instant::now();
        }
    }
    outcome.cancelled = cancel.load(Ordering::Relaxed);
    if !batch.is_empty() && !outcome.cancelled {
        on_batch(batch);
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::default_is_package;
    use std::fs;

    fn opts(query: &str) -> SearchOptions {
        SearchOptions { query: query.into(), kinds: vec![], show_hidden: false, excluded: HashSet::new(), limit: 10_000 }
    }

    fn run(root: &Path, o: &SearchOptions) -> (Vec<String>, SearchOutcome) {
        let mut names = Vec::new();
        let cancel = AtomicBool::new(false);
        let out = search(root, o, Arc::new(default_is_package), &cancel, |b| names.extend(b.into_iter().map(|e| e.name)));
        names.sort();
        (names, out)
    }

    fn fixture() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let r = tmp.path();
        fs::create_dir_all(r.join("a/b/c")).unwrap();
        fs::write(r.join("Report.txt"), "").unwrap();
        fs::write(r.join("a/b/c/report-final.pdf"), "").unwrap();
        fs::write(r.join("a/notes.md"), "").unwrap();
        fs::create_dir_all(r.join(".secret")).unwrap();
        fs::write(r.join(".secret/report.txt"), "").unwrap();
        fs::create_dir_all(r.join("Tool.app/Contents")).unwrap();
        fs::write(r.join("Tool.app/Contents/report.plist"), "").unwrap();
        tmp
    }

    #[test]
    fn finds_nested_case_insensitive_skipping_hidden_and_packages() {
        let tmp = fixture();
        let (names, out) = run(tmp.path(), &opts("REPORT"));
        assert_eq!(names, ["Report.txt", "report-final.pdf"]);
        assert_eq!(out, SearchOutcome { truncated: false, cancelled: false });
    }

    #[test]
    fn hidden_included_when_requested() {
        let tmp = fixture();
        let mut o = opts("report");
        o.show_hidden = true;
        let (names, _) = run(tmp.path(), &o);
        assert_eq!(names, ["Report.txt", "report-final.pdf", "report.txt"]);
    }

    #[test]
    fn excluded_folders_are_skipped() {
        let tmp = fixture();
        fs::create_dir_all(tmp.path().join("node_modules/report")).unwrap();
        fs::write(tmp.path().join("node_modules/report/report.js"), "").unwrap();
        let mut o = opts("report");
        o.excluded = HashSet::from(["node_modules".to_string()]);
        let (names, _) = run(tmp.path(), &o);
        assert_eq!(names, ["Report.txt", "report-final.pdf"]);
    }

    #[test]
    fn kind_filter_and_limit() {
        let tmp = fixture();
        let mut o = opts("report");
        o.kinds = vec![FileKind::Document];
        o.limit = 1;
        let (names, out) = run(tmp.path(), &o);
        assert_eq!(names.len(), 1);
        assert!(out.truncated);
    }

    #[test]
    fn cancelled_search_emits_nothing() {
        let tmp = fixture();
        let cancel = AtomicBool::new(true);
        let mut got = 0;
        let out = search(tmp.path(), &opts(""), Arc::new(default_is_package), &cancel, |b| got += b.len());
        assert!(out.cancelled);
        assert_eq!(got, 0);
    }
}
