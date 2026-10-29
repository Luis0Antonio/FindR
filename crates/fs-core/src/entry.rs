use serde::{Deserialize, Serialize};
use std::fs::Metadata;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::kind::{is_package_ext, kind_for_ext};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FileKind {
    Folder,
    Package,
    Image,
    Document,
    Video,
    Audio,
    Archive,
    Code,
    Other,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub path: String,
    pub name: String,
    pub ext: Option<String>,
    pub kind: FileKind,
    pub is_dir: bool,
    pub is_hidden: bool,
    pub is_symlink: bool,
    pub size: Option<u64>,
    pub modified: Option<i64>,
    pub created: Option<i64>,
}

const UF_HIDDEN: u32 = 0x8000;

fn unix_ms(t: std::io::Result<SystemTime>) -> Option<i64> {
    let t = t.ok()?;
    Some(match t.duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_millis() as i64,
        Err(e) => -(e.duration().as_millis() as i64),
    })
}

pub fn extension_of(name: &str) -> Option<String> {
    let stem_end = name.rfind('.')?;
    if stem_end == 0 || stem_end == name.len() - 1 {
        return None;
    }
    Some(name[stem_end + 1..].to_lowercase())
}

#[cfg(target_os = "macos")]
fn has_hidden_flag(meta: &Metadata) -> bool {
    use std::os::macos::fs::MetadataExt;
    meta.st_flags() & UF_HIDDEN != 0
}

#[cfg(not(target_os = "macos"))]
fn has_hidden_flag(_meta: &Metadata) -> bool {
    let _ = UF_HIDDEN;
    false
}

impl FileEntry {
    /// `lmeta` is the symlink_metadata of `path`; symlinks are described by their target when it resolves.
    pub fn from_metadata(path: &Path, lmeta: &Metadata, is_package: &dyn Fn(&Path, &str) -> bool) -> FileEntry {
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.to_string_lossy().into_owned());
        let is_symlink = lmeta.file_type().is_symlink();
        let target_meta = if is_symlink { std::fs::metadata(path).ok() } else { None };
        let meta = target_meta.as_ref().unwrap_or(lmeta);
        let ext = extension_of(&name);
        let dir = meta.is_dir();
        let package = dir && ext.as_deref().is_some_and(|e| is_package(path, e));
        let kind = if package {
            FileKind::Package
        } else if dir {
            FileKind::Folder
        } else {
            kind_for_ext(ext.as_deref())
        };
        FileEntry {
            path: path.to_string_lossy().into_owned(),
            is_hidden: name.starts_with('.') || has_hidden_flag(lmeta),
            name,
            ext,
            kind,
            is_dir: dir && !package,
            is_symlink,
            size: if dir { None } else { Some(meta.len()) },
            modified: unix_ms(meta.modified()),
            created: unix_ms(meta.created()),
        }
    }

    pub fn from_path(path: &Path, is_package: &dyn Fn(&Path, &str) -> bool) -> std::io::Result<FileEntry> {
        let lmeta = std::fs::symlink_metadata(path)?;
        Ok(FileEntry::from_metadata(path, &lmeta, is_package))
    }
}

pub fn default_is_package(_path: &Path, ext: &str) -> bool {
    is_package_ext(ext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensions() {
        assert_eq!(extension_of("a.TXT").as_deref(), Some("txt"));
        assert_eq!(extension_of(".bashrc"), None);
        assert_eq!(extension_of("noext"), None);
        assert_eq!(extension_of("trailing."), None);
        assert_eq!(extension_of("a.tar.gz").as_deref(), Some("gz"));
    }

    #[test]
    fn package_dir_is_not_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("Thing.app");
        std::fs::create_dir(&app).unwrap();
        let e = FileEntry::from_path(&app, &default_is_package).unwrap();
        assert_eq!(e.kind, FileKind::Package);
        assert!(!e.is_dir);
        let dir = tmp.path().join("Folder");
        std::fs::create_dir(&dir).unwrap();
        let e = FileEntry::from_path(&dir, &default_is_package).unwrap();
        assert_eq!(e.kind, FileKind::Folder);
        assert!(e.is_dir);
        assert_eq!(e.size, None);
    }
}
