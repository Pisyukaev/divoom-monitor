use crate::dota2_gsi::{self, Dota2Status, SharedDota2State};

#[tauri::command]
pub async fn start_dota2_server(
    state: tauri::State<'_, SharedDota2State>,
    device_ip: String,
    port: u16,
) -> Result<(), String> {
    {
        let mut inner = state.write().await;
        inner.device_ip = Some(device_ip);
    }

    dota2_gsi::start_server(state.inner().clone(), port).await
}

#[tauri::command]
pub async fn stop_dota2_server(
    state: tauri::State<'_, SharedDota2State>,
) -> Result<(), String> {
    dota2_gsi::stop_server(state.inner().clone()).await;
    Ok(())
}

#[tauri::command]
pub async fn get_dota2_status(
    state: tauri::State<'_, SharedDota2State>,
) -> Result<Dota2Status, String> {
    let inner = state.read().await;
    Ok(Dota2Status {
        server_running: inner.server_handle.is_some(),
        port: inner.port,
        game_state: inner.game_state.clone(),
    })
}

#[tauri::command]
pub fn detect_dota2_path() -> Result<Option<String>, String> {
    Ok(dota2_gsi::detect_dota2_install_path())
}

#[tauri::command]
pub fn setup_dota2_gsi_config(dota_path: String, port: u16) -> Result<String, String> {
    dota2_gsi::setup_gsi_config(&dota_path, port)
}
