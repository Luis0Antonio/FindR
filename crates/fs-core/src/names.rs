use std::path::{Path, PathBuf};

use crate::error::{FsError, FsResult};

pub const MAX_NAME_BYTES: usize = 255;

pub fn validate_name(name: &str) -> FsResult<()> {
    let problem = if name.is_empty() {
        Some("A name can't be empty.")
    } else if name == "." || name == ".." {
        Some("This name is reserved.")
    } else if name.contains('/') {
        Some("A name can't contain \"/\".")
    } else if name.contains('\0') {
        Some("A name can't contain NUL characters.")
    } else if name.len() > MAX_NAME_BYTES {
        Some("This name is too long.")
    } else {
        None
    };
    match problem {
        Some(msg) => Err(FsError::InvalidName(msg.into())),
        None => Ok(()),
    }
}

/// `base`, then `base 2`, `base 3`, … (the number goes before the extension, as Finder does).
pub fn unique_name(dir: &Path, name: &str, exists: impl Fn(&Path) -> bool) -> PathBuf {
    let first = dir.join(name);
    if !exists(&first) {
        return first;
    }
    let (stem, ext) = split_ext(name);
    (2..)
        .map(|n| match ext {
            Some(ext) => dir.join(format!("{stem} {n}.{ext}")),
            None => dir.join(format!("{stem} {n}")),
        })
        .find(|p| !exists(p))
        .expect("unbounded range always yields a free name")
}

/// Splits "photo.final.png" into ("photo.final", Some("png")); dotfiles and trailing dots have no extension.
pub fn split_ext(name: &str) -> (&str, Option<&str>) {
    match name.rfind('.') {
        Some(i) if i > 0 && i < name.len() - 1 => (&name[..i], Some(&name[i + 1..])),
        _ => (name, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn validation() {
        assert!(validate_name("ok name.txt").is_ok());
        for bad in ["", ".", "..", "a/b", "x\0y"] {
            assert_eq!(validate_name(bad).unwrap_err().code(), "invalid_name", "{bad:?}");
        }
        assert!(validate_name(&"a".repeat(255)).is_ok());
        assert!(validate_name(&"a".repeat(256)).is_err());
        assert!(validate_name(&"é".repeat(128)).is_err());
    }

    #[test]
    fn unique_names() {
        let taken: HashSet<PathBuf> = ["untitled folder", "untitled folder 2", "a.txt"].iter().map(|n| Path::new("/d").join(n)).collect();
        let exists = |p: &Path| taken.contains(p);
        assert_eq!(unique_name(Path::new("/d"), "untitled folder", exists), Path::new("/d/untitled folder 3"));
        assert_eq!(unique_name(Path::new("/d"), "a.txt", exists), Path::new("/d/a 2.txt"));
        assert_eq!(unique_name(Path::new("/d"), "fresh", exists), Path::new("/d/fresh"));
    }

    #[test]
    fn extension_split() {
        assert_eq!(split_ext("a.tar.gz"), ("a.tar", Some("gz")));
        assert_eq!(split_ext(".zshrc"), (".zshrc", None));
        assert_eq!(split_ext("plain"), ("plain", None));
    }
}
