use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use lru::LruCache;
use percent_encoding::percent_decode_str;
use tauri::http::{header, Response, StatusCode};

use crate::macos::icons;

type Cache = Mutex<LruCache<String, Arc<Vec<u8>>>>;

fn cache() -> &'static Cache {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(LruCache::new(NonZeroUsize::new(2000).unwrap())))
}

fn cache_key(path: &str, px: u32) -> String {
    let p = Path::new(path);
    let is_plain_file = std::fs::metadata(p).map(|m| m.is_file()).unwrap_or(false);
    match fs_core::entry::extension_of(&p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()) {
        Some(ext) if is_plain_file && !icons::has_custom_icon(path) => format!("e:{ext}:{px}"),
        _ => format!("p:{path}:{px}"),
    }
}

/// `icon://localhost/<size>/<urlencoded absolute path>`, rendered at 2x for Retina.
pub fn handle(path_and_query: &str) -> Response<Vec<u8>> {
    let mut parts = path_and_query.trim_start_matches('/').splitn(2, '/');
    let size: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(32).clamp(8, 256);
    let file = percent_decode_str(parts.next().unwrap_or("")).decode_utf8_lossy().into_owned();
    let px = size * 2;
    let key = cache_key(&file, px);

    let cached = cache().lock().unwrap().get(&key).cloned();
    let png = match cached {
        Some(png) => png,
        None => match icons::icon_png(&file, px) {
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
