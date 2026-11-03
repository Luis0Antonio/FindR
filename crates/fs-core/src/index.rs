use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use jwalk::{Parallelism, WalkDirGeneric};

use crate::entry::extension_of;
use crate::search::PackageFn;

struct Child {
    name: Box<str>,
    lower: Box<str>,
    is_dir: bool,
}

impl Child {
    fn new(name: String, is_dir: bool) -> Child {
        Child { lower: name.to_lowercase().into(), name: name.into(), is_dir }
    }
}

/// The names below a set of folders, kept per folder so a single folder can be re-read when it changes.
/// Hidden entries and excluded folders are left out; packages are listed but not entered.
#[derive(Default)]
pub struct Index {
    dirs: HashMap<PathBuf, Vec<Child>>,
}

fn enters(path: &Path, name: &str, is_package: &PackageFn) -> bool {
    extension_of(name).is_none_or(|ext| !is_package(path, &ext))
}

impl Index {
    pub fn add_tree(&mut self, root: &Path, excluded: &HashSet<String>, is_package: &PackageFn) {
        let (excluded, pkg) = (excluded.clone(), is_package.clone());
        let walker = WalkDirGeneric::<((), bool)>::new(root)
            .skip_hidden(true)
            .follow_links(false)
            .parallelism(Parallelism::RayonNewPool(0))
            .process_read_dir(move |_, _, _, children| {
                children.retain(|c| c.as_ref().is_ok_and(|c| !(c.file_type.is_dir() && excluded.contains(&*c.file_name.to_string_lossy()))));
                for child in children.iter_mut().flatten() {
                    if child.file_type.is_dir() {
                        child.client_state = enters(&child.path(), &child.file_name.to_string_lossy(), &pkg);
                        if !child.client_state {
                            child.read_children = None;
                        }
                    }
                }
            });
        self.dirs.entry(root.to_path_buf()).or_default();
        for dent in walker.into_iter().flatten() {
            if dent.depth == 0 {
                continue;
            }
            if dent.client_state {
                self.dirs.entry(dent.path()).or_default();
            }
            let child = Child::new(dent.file_name.to_string_lossy().into_owned(), dent.file_type.is_dir());
            self.dirs.entry(dent.parent_path.to_path_buf()).or_default().push(child);
        }
    }

    pub fn remove_tree(&mut self, root: &Path) {
        self.dirs.retain(|dir, _| !dir.starts_with(root));
    }

    /// Re-reads one indexed folder after a change: forgets subfolders that went away and indexes new ones.
    pub fn refresh_dir(&mut self, dir: &Path, excluded: &HashSet<String>, is_package: &PackageFn) {
        let Some(old) = self.dirs.get(dir) else { return };
        let Ok(read) = std::fs::read_dir(dir) else {
            self.remove_tree(dir);
            return;
        };
        let mut gone: HashSet<Box<str>> = old.iter().filter(|c| c.is_dir).map(|c| c.name.clone()).collect();
        let mut children = Vec::new();
        let mut added = Vec::new();
        for entry in read.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let is_dir = entry.file_type().is_ok_and(|t| t.is_dir());
            if name.starts_with('.') || (is_dir && excluded.contains(&name)) {
                continue;
            }
            if is_dir && !gone.remove(name.as_str()) && enters(&entry.path(), &name, is_package) {
                added.push(entry.path());
            }
            children.push(Child::new(name, is_dir));
        }
        for name in gone {
            self.remove_tree(&dir.join(&*name));
        }
        self.dirs.insert(dir.to_path_buf(), children);
        for path in added {
            self.add_tree(&path, excluded, is_package);
        }
    }

    pub fn covers(&self, dir: &Path) -> bool {
        self.dirs.contains_key(dir)
    }

    pub fn len(&self) -> usize {
        self.dirs.values().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.dirs.is_empty()
    }

    /// Everything below `root` whose name contains `query`, which must already be lowercase.
    pub fn find(&self, root: &Path, query: &str) -> Vec<PathBuf> {
        self.dirs
            .iter()
            .filter(|(dir, _)| dir.starts_with(root))
            .flat_map(|(dir, children)| children.iter().filter(|c| c.lower.contains(query)).map(|c| dir.join(&*c.name)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::default_is_package;
    use std::fs;
    use std::sync::Arc;

    fn excluded() -> HashSet<String> {
        HashSet::from(["node_modules".to_string()])
    }

    fn names(paths: Vec<PathBuf>, root: &Path) -> Vec<String> {
        let mut v: Vec<_> = paths.iter().map(|p| p.strip_prefix(root).unwrap().to_string_lossy().into_owned()).collect();
        v.sort();
        v
    }

    fn fixture() -> (tempfile::TempDir, Index) {
        let tmp = tempfile::tempdir().unwrap();
        let r = tmp.path();
        fs::create_dir_all(r.join("Work/CV")).unwrap();
        fs::write(r.join("Work/CV/cv-2025.pdf"), "").unwrap();
        fs::write(r.join("cv.txt"), "").unwrap();
        fs::create_dir_all(r.join("app/node_modules/cv")).unwrap();
        fs::write(r.join("app/node_modules/cv/cv.js"), "").unwrap();
        fs::create_dir_all(r.join(".hidden")).unwrap();
        fs::write(r.join(".hidden/cv.txt"), "").unwrap();
        fs::create_dir_all(r.join("Tool.app/Contents")).unwrap();
        fs::write(r.join("Tool.app/Contents/cv.plist"), "").unwrap();
        let pkg: PackageFn = Arc::new(default_is_package);
        let mut index = Index::default();
        index.add_tree(r, &excluded(), &pkg);
        (tmp, index)
    }

    #[test]
    fn skips_excluded_hidden_and_package_contents() {
        let (tmp, index) = fixture();
        assert_eq!(names(index.find(tmp.path(), "cv"), tmp.path()), ["Work/CV", "Work/CV/cv-2025.pdf", "cv.txt"]);
        assert!(index.covers(&tmp.path().join("Work/CV")));
        assert!(!index.covers(&tmp.path().join("app/node_modules")));
        assert!(!index.covers(&tmp.path().join("Tool.app")));
    }

    #[test]
    fn find_is_limited_to_the_given_folder() {
        let (tmp, index) = fixture();
        let work = tmp.path().join("Work");
        assert_eq!(names(index.find(&work, "cv"), &work), ["CV", "CV/cv-2025.pdf"]);
    }

    #[test]
    fn refresh_picks_up_new_and_removed_entries() {
        let (tmp, mut index) = fixture();
        let r = tmp.path();
        let pkg: PackageFn = Arc::new(default_is_package);
        fs::remove_dir_all(r.join("Work/CV")).unwrap();
        fs::create_dir_all(r.join("Work/Jobs/2026")).unwrap();
        fs::write(r.join("Work/Jobs/2026/cv-new.pdf"), "").unwrap();
        index.refresh_dir(&r.join("Work"), &excluded(), &pkg);
        assert_eq!(names(index.find(r, "cv"), r), ["Work/Jobs/2026/cv-new.pdf", "cv.txt"]);
        assert!(!index.covers(&r.join("Work/CV")));

        fs::write(r.join("Work/Jobs/2026/cv-final.pdf"), "").unwrap();
        index.refresh_dir(&r.join("Work/Jobs/2026"), &excluded(), &pkg);
        assert_eq!(index.find(r, "cv-final").len(), 1);
    }
}
