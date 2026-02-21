use serde_json::Number;

use crate::divoom_api::{discover_via_divoom_api, send_command};
use crate::models::{DeviceSettings, DivoomDevice};

#[tauri::command]
pub async fn scan_devices() -> Result<Vec<DivoomDevice>, String> {
    let mut devices = Vec::new();

    if let Ok(api_devices) = discover_via_divoom_api().await {
        devices.extend(api_devices);
    }

    let mut unique_devices = Vec::new();
    for device in devices {
        let is_duplicate = unique_devices.iter().any(|d: &DivoomDevice| {
            (device.ip_address.is_some() && d.ip_address == device.ip_address)
                || (device.mac_address.is_some() && d.mac_address == device.mac_address)
        });
        if !is_duplicate {
            unique_devices.push(device);
        }
    }

    Ok(unique_devices)
}

#[tauri::command]
pub async fn get_device_info(ip_address: String) -> Result<DeviceSettings, String> {
    let result = send_command(
        &ip_address,
        &serde_json::json!({
            "Command": "Channel/GetAllConf"
        }),
    )
    .await
    .map_err(|e| format!("Failed to send command: {}", e))?;

    Ok(DeviceSettings {
        brightness: result
            .get("Brightness")
            .and_then(|v| v.as_u64())
            .map(|v| v as u8),
        rotation_flag: result
            .get("RotationFlag")
            .and_then(|v| v.as_u64())
            .map(|v| v as u8),
        date_format: result
            .get("DateFormat")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        time24_flag: result
            .get("Time24Flag")
            .and_then(|v| v.as_u64())
            .map(|v| v as u8),
        temperature_mode: result
            .get("TemperatureMode")
            .and_then(|v| v.as_u64())
            .map(|v| v as u8),
        mirror_flag: result
            .get("MirrorFlag")
            .and_then(|v| v.as_u64())
            .map(|v| v as u8),
        light_switch: result
            .get("LightSwitch")
            .and_then(|v| v.as_u64())
            .map(|v| v as u8),
    })
}

#[tauri::command]
pub async fn set_brightness(ip_address: String, value: Number) {
    let _ = send_command(
        &ip_address,
        &serde_json::json!({
            "Command": "Channel/SetBrightness",
            "Brightness": value
        }),
    )
    .await
    .map_err(|e| format!("Failed to send command: {}", e));
}

#[tauri::command]
pub async fn set_switch_screen(ip_address: String, value: Number) {
    let _ = send_command(
        &ip_address,
        &serde_json::json!({
            "Command": "Channel/OnOffScreen",
            "OnOff": value
        }),
    )
    .await
    .map_err(|e| format!("Failed to send command: {}", e));
}

#[tauri::command]
pub async fn set_temperature_mode(ip_address: String, value: Number) {
    let _ = send_command(
        &ip_address,
        &serde_json::json!({
            "Command": "Device/SetDisTempMode",
            // 0 - celsius, 1 - fahrenheit
            "Mode": value
        }),
    )
    .await
    .map_err(|e| format!("Failed to send command: {}", e));
}

#[tauri::command]
pub async fn set_mirror_mode(ip_address: String, value: Number) {
    let _ = send_command(
        &ip_address,
        &serde_json::json!({
            "Command": "Device/SetMirrorMode",
            // 0 - disable, 1 - enable
            "Mode": value
        }),
    )
    .await
    .map_err(|e| format!("Failed to send command: {}", e));
}

#[tauri::command]
pub async fn set_24_hours_mode(ip_address: String, value: Number) {
    let _ = send_command(
        &ip_address,
        &serde_json::json!({
            "Command": "Device/SetTime24Flag",
            // 0 - 0:12, 1 - 1:24
            "Mode": value
        }),
    )
    .await
    .map_err(|e| format!("Failed to send command: {}", e));
}

#[tauri::command]
pub async fn reboot_device(ip_address: String) {
    let _ = send_command(
        &ip_address,
        &serde_json::json!({
            "Command": "Device/SysReboot",
        }),
    )
    .await
    .map_err(|e| format!("Failed to send command: {}", e));
}
