use std::path::{Path, PathBuf};

use crate::entry::FileEntry;
use crate::error::{FsError, FsResult};
use crate::names::{unique_name, validate_name};

pub type PackageCheck<'a> = &'a dyn Fn(&Path, &str) -> bool;

fn exists_no_follow(p: &Path) -> bool {
    std::fs::symlink_metadata(p).is_ok()
}

pub fn create_folder(parent: &Path, is_package: PackageCheck) -> FsResult<FileEntry> {
    let target = unique_name(parent, "untitled folder", exists_no_follow);
    std::fs::create_dir(&target).map_err(|e| FsError::from_io(e, &target.to_string_lossy()))?;
    FileEntry::from_path(&target, is_package).map_err(|e| FsError::from_io(e, &target.to_string_lossy()))
}

#[cfg(unix)]
fn same_file(a: &Path, b: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (std::fs::symlink_metadata(a), std::fs::symlink_metadata(b)) {
        (Ok(x), Ok(y)) => x.dev() == y.dev() && x.ino() == y.ino(),
        _ => false,
    }
}

/// Renames within the same folder. A target that resolves to the source itself (a case-only rename on a
/// case-insensitive volume) is allowed; any other existing target fails with `exists`.
pub fn rename(path: &Path, new_name: &str, is_package: PackageCheck) -> FsResult<FileEntry> {
    validate_name(new_name)?;
    let parent = path.parent().ok_or_else(|| FsError::InvalidTarget("Can't rename the root folder.".into()))?;
    let target: PathBuf = parent.join(new_name);
    if target == path {
        return FileEntry::from_path(path, is_package).map_err(|e| FsError::from_io(e, &path.to_string_lossy()));
    }
    if exists_no_follow(&target) && !same_file(path, &target) {
        return Err(FsError::Exists(format!("The name \"{new_name}\" is already taken. Please choose a different name.")));
    }
    std::fs::rename(path, &target).map_err(|e| FsError::from_io(e, &path.to_string_lossy()))?;
    FileEntry::from_path(&target, is_package).map_err(|e| FsError::from_io(e, &target.to_string_lossy()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::default_is_package;
    use std::fs;

    #[test]
    fn create_folder_picks_unique_names() {
        let tmp = tempfile::tempdir().unwrap();
        let a = create_folder(tmp.path(), &default_is_package).unwrap();
        let b = create_folder(tmp.path(), &default_is_package).unwrap();
        assert_eq!(a.name, "untitled folder");
        assert_eq!(b.name, "untitled folder 2");
        assert!(b.is_dir);
    }

    #[test]
    fn rename_and_conflicts() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.txt");
        fs::write(&a, "a").unwrap();
        fs::write(tmp.path().join("b.txt"), "b").unwrap();
        assert_eq!(rename(&a, "b.txt", &default_is_package).unwrap_err().code(), "exists");
        assert_eq!(rename(&a, "x/y", &default_is_package).unwrap_err().code(), "invalid_name");
        let e = rename(&a, "c.txt", &default_is_package).unwrap();
        assert_eq!(e.name, "c.txt");
        assert!(!a.exists());
    }

    #[test]
    fn case_only_rename() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.txt");
        fs::write(&a, "a").unwrap();
        let e = rename(&a, "A.txt", &default_is_package).unwrap();
        assert_eq!(e.name, "A.txt");
        let names: Vec<String> = fs::read_dir(tmp.path()).unwrap().map(|d| d.unwrap().file_name().to_string_lossy().into_owned()).collect();
        assert_eq!(names, ["A.txt"]);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConflictPolicy {
    Replace,
    KeepBoth,
    Skip,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Conflict {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct OpError {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize)]
pub struct OpResult {
    pub done: Vec<String>,
    pub conflicts: Vec<Conflict>,
    pub errors: Vec<OpError>,
}

/// Why a drop of `source` into `target_dir` must be refused, if it must.
pub fn drop_rejection(source: &Path, target_dir: &Path) -> Option<&'static str> {
    if source == target_dir {
        Some("An item can't be moved into itself.")
    } else if target_dir.starts_with(source) {
        Some("A folder can't be moved into one of its own subfolders.")
    } else {
        None
    }
}

#[cfg(unix)]
pub fn device_of(path: &Path) -> std::io::Result<u64> {
    use std::os::unix::fs::MetadataExt;
    Ok(std::fs::symlink_metadata(path)?.dev())
}

pub struct Progress<'a> {
    pub total: u64,
    pub done: u64,
    pub report: &'a mut dyn FnMut(u64, u64),
}

impl Progress<'_> {
    fn add(&mut self, bytes: u64) {
        self.done += bytes;
        (self.report)(self.done, self.total);
    }
}

pub fn total_size(path: &Path) -> u64 {
    let Ok(meta) = std::fs::symlink_metadata(path) else { return 0 };
    if meta.is_dir() {
        std::fs::read_dir(path).map(|rd| rd.flatten().map(|d| total_size(&d.path())).sum()).unwrap_or(0)
    } else {
        meta.len()
    }
}

fn copy_times(src_meta: &std::fs::Metadata, dest: &Path, symlink: bool) -> std::io::Result<()> {
    let atime = filetime::FileTime::from_last_access_time(src_meta);
    let mtime = filetime::FileTime::from_last_modification_time(src_meta);
    if symlink {
        filetime::set_symlink_file_times(dest, atime, mtime)
    } else {
        filetime::set_file_times(dest, atime, mtime)
    }
}

const CHUNK: usize = 8 * 1024 * 1024;

fn copy_file(src: &Path, dest: &Path, progress: &mut Option<Progress>) -> std::io::Result<()> {
    match progress {
        None => {
            std::fs::copy(src, dest)?;
        }
        Some(p) => {
            use std::io::{Read, Write};
            let mut input = std::fs::File::open(src)?;
            let mut output = std::fs::File::create(dest)?;
            let mut buf = vec![0u8; CHUNK];
            loop {
                let n = input.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                output.write_all(&buf[..n])?;
                p.add(n as u64);
            }
            std::fs::set_permissions(dest, std::fs::metadata(src)?.permissions())?;
        }
    }
    Ok(())
}

/// Recursive copy that keeps symlinks as links and preserves modification times.
pub fn copy_recursive(src: &Path, dest: &Path, progress: &mut Option<Progress>) -> std::io::Result<()> {
    let meta = std::fs::symlink_metadata(src)?;
    let ft = meta.file_type();
    if ft.is_symlink() {
        std::os::unix::fs::symlink(std::fs::read_link(src)?, dest)?;
        copy_times(&meta, dest, true)?;
    } else if ft.is_dir() {
        std::fs::create_dir(dest)?;
        for dent in std::fs::read_dir(src)? {
            let dent = dent?;
            copy_recursive(&dent.path(), &dest.join(dent.file_name()), progress)?;
        }
        std::fs::set_permissions(dest, meta.permissions())?;
        copy_times(&meta, dest, false)?;
    } else {
        copy_file(src, dest, progress)?;
        copy_times(&meta, dest, false)?;
    }
    Ok(())
}

fn remove_any(path: &Path) -> std::io::Result<()> {
    if std::fs::symlink_metadata(path)?.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
}

/// Moves by rename when both sides are on one volume, falling back to copy + delete across volumes
/// (or when rename reports EXDEV).
pub fn move_one(src: &Path, dest: &Path, same_device: bool, progress: &mut Option<Progress>) -> std::io::Result<()> {
    if same_device {
        match std::fs::rename(src, dest) {
            Ok(()) => return Ok(()),
            Err(e) if e.raw_os_error() == Some(libc::EXDEV) => {}
            Err(e) => return Err(e),
        }
    }
    if let Err(e) = copy_recursive(src, dest, progress) {
        let _ = remove_any(dest);
        return Err(e);
    }
    remove_any(src)
}

/// Moves or copies `sources` into `target_dir`. With no policy, items whose name is taken are reported as
/// conflicts and left untouched; everything else is processed.
pub fn move_items(
    sources: &[String],
    target_dir: &Path,
    copy: bool,
    policy: Option<ConflictPolicy>,
    mut progress: Option<Progress>,
) -> OpResult {
    let mut result = OpResult::default();
    let target_dev = device_of(target_dir).ok();
    for source in sources {
        let src = Path::new(source);
        let fail = |result: &mut OpResult, message: String| result.errors.push(OpError { path: source.clone(), message });
        if let Some(reason) = drop_rejection(src, target_dir) {
            fail(&mut result, reason.into());
            continue;
        }
        let Some(name) = src.file_name() else {
            fail(&mut result, "Can't move the root folder.".into());
            continue;
        };
        if src.parent() == Some(target_dir) && !copy {
            continue;
        }
        let mut dest = target_dir.join(name);
        if exists_no_follow(&dest) {
            match policy {
                None => {
                    result.conflicts.push(Conflict { source: source.clone(), target: dest.to_string_lossy().into_owned() });
                    continue;
                }
                Some(ConflictPolicy::Skip) => continue,
                Some(ConflictPolicy::KeepBoth) => dest = unique_name(target_dir, &name.to_string_lossy(), exists_no_follow),
                Some(ConflictPolicy::Replace) => {
                    if same_file(src, &dest) {
                        continue;
                    }
                    if let Err(e) = move_to_trash(&dest) {
                        fail(&mut result, format!("Couldn't move the existing item to the Trash: {e}"));
                        continue;
                    }
                }
            }
        }
        let outcome = if copy {
            copy_recursive(src, &dest, &mut progress)
        } else {
            let same_device = target_dev.is_some() && device_of(src).ok() == target_dev;
            move_one(src, &dest, same_device, &mut progress)
        };
        match outcome {
            Ok(()) => result.done.push(dest.to_string_lossy().into_owned()),
            Err(e) => fail(&mut result, FsError::from_io(e, &src.to_string_lossy()).to_string()),
        }
    }
    result
}

#[cfg(test)]
mod move_tests {
    use super::*;
    use std::fs;
    use std::time::{Duration, SystemTime};

    fn s(p: &Path) -> String {
        p.to_string_lossy().into_owned()
    }

    #[test]
    fn rejects_drops_into_self_or_descendants() {
        assert!(drop_rejection(Path::new("/a/b"), Path::new("/a/b")).is_some());
        assert!(drop_rejection(Path::new("/a/b"), Path::new("/a/b/c/d")).is_some());
        assert!(drop_rejection(Path::new("/a/b"), Path::new("/a/bc")).is_none());
        assert!(drop_rejection(Path::new("/a/b"), Path::new("/a")).is_none());

        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("dir");
        fs::create_dir_all(dir.join("sub")).unwrap();
        let r = move_items(&[s(&dir)], &dir.join("sub"), false, None, None);
        assert_eq!(r.errors.len(), 1);
        assert!(dir.join("sub").exists());
    }

    #[test]
    fn conflicts_are_reported_then_resolved() {
        let tmp = tempfile::tempdir().unwrap();
        let (src_dir, dst_dir) = (tmp.path().join("src"), tmp.path().join("dst"));
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&dst_dir).unwrap();
        fs::write(src_dir.join("a.txt"), "new").unwrap();
        fs::write(src_dir.join("b.txt"), "b").unwrap();
        fs::write(dst_dir.join("a.txt"), "old").unwrap();

        let r = move_items(&[s(&src_dir.join("a.txt")), s(&src_dir.join("b.txt"))], &dst_dir, false, None, None);
        assert_eq!(r.conflicts, [Conflict { source: s(&src_dir.join("a.txt")), target: s(&dst_dir.join("a.txt")) }]);
        assert_eq!(r.done, [s(&dst_dir.join("b.txt"))]);
        assert!(src_dir.join("a.txt").exists());

        let r = move_items(&[s(&src_dir.join("a.txt"))], &dst_dir, false, Some(ConflictPolicy::KeepBoth), None);
        assert_eq!(r.done, [s(&dst_dir.join("a 2.txt"))]);
        assert_eq!(fs::read_to_string(dst_dir.join("a.txt")).unwrap(), "old");

        fs::write(src_dir.join("c.txt"), "c").unwrap();
        fs::write(dst_dir.join("c.txt"), "c-old").unwrap();
        let r = move_items(&[s(&src_dir.join("c.txt"))], &dst_dir, false, Some(ConflictPolicy::Skip), None);
        assert!(r.done.is_empty() && r.conflicts.is_empty());
        assert!(src_dir.join("c.txt").exists());
    }

    #[test]
    fn copy_keeps_source_and_moves_to_same_parent_are_noops() {
        let tmp = tempfile::tempdir().unwrap();
        let (src_dir, dst_dir) = (tmp.path().join("src"), tmp.path().join("dst"));
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&dst_dir).unwrap();
        fs::write(src_dir.join("a.txt"), "a").unwrap();
        let r = move_items(&[s(&src_dir.join("a.txt"))], &dst_dir, true, None, None);
        assert_eq!(r.done.len(), 1);
        assert!(src_dir.join("a.txt").exists() && dst_dir.join("a.txt").exists());
        let r = move_items(&[s(&src_dir.join("a.txt"))], &src_dir, false, None, None);
        assert_eq!(r, OpResult::default());
    }

    #[test]
    fn cross_volume_fallback_copies_then_deletes_preserving_times_and_links() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("tree");
        fs::create_dir_all(src.join("inner")).unwrap();
        fs::write(src.join("inner/file.txt"), "data").unwrap();
        std::os::unix::fs::symlink("inner/file.txt", src.join("link")).unwrap();
        let old = SystemTime::now() - Duration::from_secs(86_400 * 30);
        filetime::set_file_mtime(src.join("inner/file.txt"), filetime::FileTime::from_system_time(old)).unwrap();

        let dest = tmp.path().join("moved");
        move_one(&src, &dest, false, &mut None).unwrap();
        assert!(!src.exists());
        assert_eq!(fs::read_to_string(dest.join("inner/file.txt")).unwrap(), "data");
        assert!(fs::symlink_metadata(dest.join("link")).unwrap().file_type().is_symlink());
        assert_eq!(fs::read_link(dest.join("link")).unwrap(), Path::new("inner/file.txt"));
        let mtime = fs::metadata(dest.join("inner/file.txt")).unwrap().modified().unwrap();
        assert!(mtime.duration_since(old).unwrap_or_default() < Duration::from_secs(1));
    }

    #[test]
    fn chunked_copy_reports_progress() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("big.bin");
        fs::write(&src, vec![7u8; 3 * 1024 * 1024]).unwrap();
        let mut seen = vec![];
        let mut report = |done: u64, total: u64| seen.push((done, total));
        let total = total_size(&src);
        let mut progress = Some(Progress { total, done: 0, report: &mut report });
        copy_recursive(&src, &tmp.path().join("copy.bin"), &mut progress).unwrap();
        assert_eq!(seen.last(), Some(&(total, total)));
    }
}

// The default macOS method scripts Finder, which needs Automation permission; NSFileManager doesn't.
pub fn move_to_trash(path: &Path) -> Result<(), trash::Error> {
    let mut ctx = trash::TrashContext::default();
    #[cfg(target_os = "macos")]
    {
        use trash::macos::{DeleteMethod, TrashContextExtMacos};
        ctx.set_delete_method(DeleteMethod::NsFileManager);
    }
    ctx.delete(path)
}
