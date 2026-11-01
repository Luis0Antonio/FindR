use std::collections::HashMap;
use std::path::PathBuf;

use fs_core::sort::{SortDir, SortKey};
use fs_core::{FsError, FsResult};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaneSettings {
    pub path: String,
    pub sort_key: SortKey,
    pub sort_dir: SortDir,
    pub folders_first: bool,
    pub item_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub favorites: Vec<String>,
    pub split_enabled: bool,
    pub split_ratio: f64,
    pub show_hidden: bool,
    #[serde(default)]
    pub sidebar_hidden: bool,
    pub panes: HashMap<String, PaneSettings>,
}

fn settings_path(app: &AppHandle) -> FsResult<PathBuf> {
    let dir = app.path().app_data_dir().map_err(|e| FsError::Io(e.to_string()))?;
    Ok(dir.join("settings.json"))
}

#[tauri::command]
pub fn load_settings(app: AppHandle) -> FsResult<Option<Settings>> {
    let path = settings_path(&app)?;
    let Ok(text) = std::fs::read_to_string(&path) else { return Ok(None) };
    Ok(serde_json::from_str(&text).ok())
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Settings) -> FsResult<()> {
    let path = settings_path(&app)?;
    let dir = path.parent().expect("settings path has a parent");
    std::fs::create_dir_all(dir).map_err(|e| FsError::from_io(e, "settings dir"))?;
    let tmp = path.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(&settings).map_err(|e| FsError::Io(e.to_string()))?;
    std::fs::write(&tmp, text).map_err(|e| FsError::from_io(e, "settings"))?;
    std::fs::rename(&tmp, &path).map_err(|e| FsError::from_io(e, "settings"))
}
