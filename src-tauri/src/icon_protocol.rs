use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use lru::LruCache;
use percent_encoding::percent_decode_str;
use tauri::http::{header, Response, StatusCode};

use fs_core::FileKind;

use crate::macos::icons;

type Cache = Mutex<LruCache<String, Arc<Vec<u8>>>>;

fn cache() -> &'static Cache {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(LruCache::new(NonZeroUsize::new(2000).unwrap())))
}

enum Source {
    Icon,
    Thumbnail,
}

fn cache_key(path: &str, px: u32) -> (String, Source) {
    let p = Path::new(path);
    let meta = std::fs::metadata(p).ok().filter(|m| m.is_file());
    let ext = fs_core::entry::extension_of(&p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
    match (meta, ext) {
        (Some(meta), Some(ext)) if wants_thumbnail(&ext) => {
            let modified = meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).unwrap_or_default();
            (format!("t:{path}:{}:{px}", modified.as_nanos()), Source::Thumbnail)
        }
        (Some(_), Some(ext)) if !icons::has_custom_icon(path) => (format!("e:{ext}:{px}"), Source::Icon),
        _ => (format!("p:{path}:{px}"), Source::Icon),
    }
}

fn wants_thumbnail(ext: &str) -> bool {
    matches!(fs_core::kind::kind_for_ext(Some(ext)), FileKind::Image | FileKind::Video) || ext == "pdf"
}

fn render(file: &str, px: u32, source: Source) -> Option<Vec<u8>> {
    match source {
        Source::Thumbnail => icons::thumbnail_png(file, px).or_else(|| icons::icon_png(file, px)),
        Source::Icon => icons::icon_png(file, px),
    }
}

/// `icon://localhost/<size>/<urlencoded absolute path>`, rendered at 2x for Retina.
pub fn handle(path_and_query: &str) -> Response<Vec<u8>> {
    let mut parts = path_and_query.trim_start_matches('/').splitn(2, '/');
    let size: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(32).clamp(8, 256);
    let file = percent_decode_str(parts.next().unwrap_or("")).decode_utf8_lossy().into_owned();
    let px = size * 2;
    let (key, source) = cache_key(&file, px);

    let cached = cache().lock().unwrap().get(&key).cloned();
    let png = match cached {
        Some(png) => png,
        None => match render(&file, px, source) {
            Some(png) => {
                let png = Arc::new(png);
                cache().lock().unwrap().put(key, png.clone());
                png
            }
            None => {
                return Response::builder().status(StatusCode::NOT_FOUND).body(Vec::new()).unwrap();
            }
        },
    };
    Response::builder()
        .header(header::CONTENT_TYPE, "image/png")
        .header(header::CACHE_CONTROL, "max-age=600")
        .body(png.as_ref().clone())
        .unwrap()
}
