use crate::entry::FileKind;

pub const PACKAGE_EXTENSIONS: &[&str] = &[
    "app", "bundle", "framework", "photoslibrary", "pkg", "mpkg", "rtfd", "plugin", "kext", "xcodeproj",
    "xcworkspace", "playground", "musiclibrary", "photolibrary", "imovielibrary", "tvlibrary", "fcpbundle",
    "logicx", "band", "pages", "numbers", "key", "appex", "qlgenerator", "mdimporter", "prefPane", "saver",
    "wdgt", "xpc", "component", "lproj_bundle", "scptd", "workflow", "sparsebundle",
];

pub fn is_package_ext(ext: &str) -> bool {
    PACKAGE_EXTENSIONS.iter().any(|p| p.eq_ignore_ascii_case(ext))
}

pub fn kind_for_ext(ext: Option<&str>) -> FileKind {
    let Some(ext) = ext else { return FileKind::Other };
    match ext {
        "png" | "jpg" | "jpeg" | "gif" | "heic" | "heif" | "webp" | "tiff" | "tif" | "bmp" | "svg" | "raw" | "cr2"
        | "nef" | "arw" | "dng" | "ico" | "icns" => FileKind::Image,
        "pdf" | "doc" | "docx" | "pages" | "txt" | "md" | "rtf" | "rtfd" | "key" | "ppt" | "pptx" | "numbers"
        | "xls" | "xlsx" | "csv" | "odt" | "ods" | "odp" | "epub" => FileKind::Document,
        "mp4" | "mov" | "m4v" | "avi" | "mkv" | "webm" | "wmv" | "flv" | "mpg" | "mpeg" => FileKind::Video,
        "mp3" | "m4a" | "wav" | "aiff" | "aif" | "flac" | "aac" | "ogg" | "opus" | "wma" => FileKind::Audio,
        "zip" | "tar" | "gz" | "tgz" | "bz2" | "xz" | "7z" | "rar" | "dmg" | "iso" | "zst" => FileKind::Archive,
        "rs" | "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "py" | "go" | "java" | "c" | "h" | "cpp" | "hpp"
        | "cc" | "m" | "mm" | "swift" | "kt" | "rb" | "sh" | "zsh" | "bash" | "json" | "yaml" | "yml" | "toml"
        | "html" | "css" | "scss" | "sql" | "php" | "lua" | "xml" | "vue" | "svelte" => FileKind::Code,
        _ => FileKind::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_extensions_map_to_kinds() {
        let table: &[(&[&str], FileKind)] = &[
            (&["png", "jpg", "jpeg", "gif", "heic", "webp", "tiff", "bmp", "svg", "raw"], FileKind::Image),
            (
                &["pdf", "doc", "docx", "pages", "txt", "md", "rtf", "key", "ppt", "pptx", "numbers", "xls", "xlsx", "csv"],
                FileKind::Document,
            ),
            (&["mp4", "mov", "m4v", "avi", "mkv", "webm"], FileKind::Video),
            (&["mp3", "m4a", "wav", "aiff", "flac", "aac", "ogg"], FileKind::Audio),
            (&["zip", "tar", "gz", "tgz", "bz2", "xz", "7z", "rar", "dmg"], FileKind::Archive),
            (
                &[
                    "rs", "ts", "tsx", "js", "jsx", "py", "go", "java", "c", "h", "cpp", "swift", "kt", "rb", "sh", "json",
                    "yaml", "yml", "toml", "html", "css", "sql",
                ],
                FileKind::Code,
            ),
        ];
        for (exts, kind) in table {
            for ext in *exts {
                assert_eq!(kind_for_ext(Some(ext)), *kind, "{ext}");
            }
        }
        assert_eq!(kind_for_ext(Some("xyz")), FileKind::Other);
        assert_eq!(kind_for_ext(None), FileKind::Other);
    }

    #[test]
    fn package_extensions() {
        assert!(is_package_ext("app"));
        assert!(is_package_ext("APP"));
        assert!(is_package_ext("photoslibrary"));
        assert!(!is_package_ext("txt"));
    }
}
