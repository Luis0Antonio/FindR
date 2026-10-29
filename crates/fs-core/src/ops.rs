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
