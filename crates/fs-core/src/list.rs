use std::path::Path;

use crate::entry::FileEntry;
use crate::error::{FsError, FsResult};

pub fn list_dir(path: &Path, show_hidden: bool, is_package: &dyn Fn(&Path, &str) -> bool) -> FsResult<Vec<FileEntry>> {
    let rd = std::fs::read_dir(path).map_err(|e| FsError::from_io(e, &path.to_string_lossy()))?;
    let mut out = Vec::new();
    for dent in rd.flatten() {
        let p = dent.path();
        let Ok(lmeta) = std::fs::symlink_metadata(&p) else { continue };
        let entry = FileEntry::from_metadata(&p, &lmeta, is_package);
        if show_hidden || !entry.is_hidden {
            out.push(entry);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::default_is_package;

    #[test]
    fn lists_and_hides_dotfiles() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a.txt"), "x").unwrap();
        std::fs::write(tmp.path().join(".hidden"), "x").unwrap();
        std::fs::create_dir(tmp.path().join("dir")).unwrap();
        let visible = list_dir(tmp.path(), false, &default_is_package).unwrap();
        assert_eq!(visible.len(), 2);
        let all = list_dir(tmp.path(), true, &default_is_package).unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn missing_dir_is_not_found() {
        let err = list_dir(Path::new("/definitely/not/here"), false, &default_is_package).unwrap_err();
        assert_eq!(err.code(), "not_found");
    }
}
