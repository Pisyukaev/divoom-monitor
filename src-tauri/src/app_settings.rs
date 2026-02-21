use std::sync::atomic::{AtomicBool, Ordering};

pub static CLOSE_TO_TRAY: AtomicBool = AtomicBool::new(true);

#[tauri::command]
pub fn set_close_to_tray(enabled: bool) {
    CLOSE_TO_TRAY.store(enabled, Ordering::Relaxed);
}

#[tauri::command]
pub fn get_close_to_tray() -> bool {
    CLOSE_TO_TRAY.load(Ordering::Relaxed)
}
