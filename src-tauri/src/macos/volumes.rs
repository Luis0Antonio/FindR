use std::path::Path;

use serde::Serialize;

#[derive(Serialize)]
pub struct Volume {
    pub name: String,
    pub path: String,
}

/// Mounted volumes from /Volumes; the boot volume's alias there is reported as "/".
pub fn list_volumes() -> Vec<Volume> {
    let mut out = vec![];
    let Ok(rd) = std::fs::read_dir("/Volumes") else { return out };
    for dent in rd.flatten() {
        let name = dent.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let path = dent.path();
        let is_root = std::fs::canonicalize(&path).is_ok_and(|p| p == Path::new("/"));
        if !is_root && !path.is_dir() {
            continue;
        }
        out.push(Volume { name, path: if is_root { "/".into() } else { path.to_string_lossy().into_owned() } });
    }
    out.sort_by(|a, b| (a.path != "/").cmp(&(b.path != "/")).then_with(|| a.name.cmp(&b.name)));
    out
}
