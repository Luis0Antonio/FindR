use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::entry::FileEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SortKey {
    Name,
    Modified,
    Created,
    Kind,
    Size,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SortDir {
    Asc,
    Desc,
}

pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    natord::compare_ignore_case(a, b)
}

fn kind_key(e: &FileEntry) -> (String, String) {
    (format!("{:?}", e.kind), e.ext.clone().unwrap_or_default())
}

/// Folders first (optional, name order only, like Finder), then the key in the chosen direction, then name ascending as the stable tie-breaker.
/// Missing modification dates always sort last.
pub fn sort_entries(entries: &mut [FileEntry], key: SortKey, dir: SortDir, folders_first: bool) {
    entries.sort_by(|a, b| {
        if folders_first && key == SortKey::Name {
            let group = b.is_dir.cmp(&a.is_dir);
            if group != Ordering::Equal {
                return group;
            }
        }
        let primary = match key {
            SortKey::Name => natural_cmp(&a.name, &b.name),
            SortKey::Modified | SortKey::Created => {
                let date = |e: &FileEntry| if key == SortKey::Modified { e.modified } else { e.created };
                match (date(a), date(b)) {
                    (Some(x), Some(y)) => x.cmp(&y),
                    (Some(_), None) => return Ordering::Less,
                    (None, Some(_)) => return Ordering::Greater,
                    (None, None) => Ordering::Equal,
                }
            }
            SortKey::Kind => kind_key(a).cmp(&kind_key(b)),
            SortKey::Size => a.size.unwrap_or(0).cmp(&b.size.unwrap_or(0)),
        };
        let primary = if dir == SortDir::Desc { primary.reverse() } else { primary };
        primary.then_with(|| natural_cmp(&a.name, &b.name))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::FileKind;

    fn entry(name: &str, is_dir: bool, size: Option<u64>, modified: Option<i64>) -> FileEntry {
        FileEntry {
            path: format!("/x/{name}"),
            name: name.into(),
            ext: crate::entry::extension_of(name),
            kind: if is_dir { FileKind::Folder } else { FileKind::Other },
            is_dir,
            is_hidden: false,
            is_symlink: false,
            size,
            modified,
            created: None,
        }
    }

    fn names(v: &[FileEntry]) -> Vec<&str> {
        v.iter().map(|e| e.name.as_str()).collect()
    }

    #[test]
    fn natural_case_insensitive() {
        let mut v = vec![entry("file10", false, None, None), entry("File2", false, None, None), entry("file1", false, None, None)];
        sort_entries(&mut v, SortKey::Name, SortDir::Asc, true);
        assert_eq!(names(&v), ["file1", "File2", "file10"]);
        sort_entries(&mut v, SortKey::Name, SortDir::Desc, true);
        assert_eq!(names(&v), ["file10", "File2", "file1"]);
    }

    #[test]
    fn folders_first_and_tie_breaker() {
        let mut v = vec![
            entry("b", false, Some(5), None),
            entry("dir", true, None, None),
            entry("a", false, Some(5), None),
            entry("c", false, Some(1), None),
        ];
        sort_entries(&mut v, SortKey::Name, SortDir::Desc, true);
        assert_eq!(names(&v), ["dir", "c", "b", "a"]);
        sort_entries(&mut v, SortKey::Size, SortDir::Desc, true);
        assert_eq!(names(&v), ["a", "b", "c", "dir"]);
        sort_entries(&mut v, SortKey::Size, SortDir::Asc, false);
        assert_eq!(names(&v), ["dir", "c", "a", "b"]);
    }

    #[test]
    fn missing_dates_last() {
        let mut v = vec![entry("none", false, None, None), entry("new", false, None, Some(20)), entry("old", false, None, Some(10))];
        sort_entries(&mut v, SortKey::Modified, SortDir::Asc, false);
        assert_eq!(names(&v), ["old", "new", "none"]);
        sort_entries(&mut v, SortKey::Modified, SortDir::Desc, false);
        assert_eq!(names(&v), ["new", "old", "none"]);
    }
}
