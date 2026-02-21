use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

pub static CLOSE_TO_TRAY: AtomicBool = AtomicBool::new(true);
static SETTINGS_PATH: OnceLock<PathBuf> = OnceLock::new();

#[derive(Serialize, Deserialize)]
struct PersistedSettings {
    close_to_tray: bool,
}

pub fn init(app_data_dir: PathBuf) {
    let path = app_data_dir.join("settings.json");

    if let Ok(data) = fs::read_to_string(&path) {
        if let Ok(settings) = serde_json::from_str::<PersistedSettings>(&data) {
            CLOSE_TO_TRAY.store(settings.close_to_tray, Ordering::Relaxed);
        }
    }

    SETTINGS_PATH.set(path).ok();
}

fn persist() {
    let Some(path) = SETTINGS_PATH.get() else {
        return;
    };

    let settings = PersistedSettings {
        close_to_tray: CLOSE_TO_TRAY.load(Ordering::Relaxed),
    };

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, serde_json::to_string_pretty(&settings).unwrap_or_default());
}

#[tauri::command]
pub fn set_close_to_tray(enabled: bool) {
    CLOSE_TO_TRAY.store(enabled, Ordering::Relaxed);
    persist();
}

#[tauri::command]
pub fn get_close_to_tray() -> bool {
    CLOSE_TO_TRAY.load(Ordering::Relaxed)
}
