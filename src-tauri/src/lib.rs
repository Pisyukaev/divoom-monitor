use serde::{de::value, Deserialize, Serialize};
use serde_json::{json, Number, Value};
use std::time::Duration;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivoomDevice {
    pub name: String,
    pub mac_address: Option<String>,
    pub device_type: String,
    pub ip_address: Option<String>,
    pub signal_strength: Option<i32>,
    pub is_connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSettings {
    pub ssid: Option<String>,
    pub ip_address: Option<String>,
    pub mac_address: Option<String>,
    pub signal_strength: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSettings {
    pub brightness: Option<u8>,
    pub rotation_flag: Option<u8>,
    pub date_format: Option<String>,
    pub time24_flag: Option<u8>,
    pub temperature_mode: Option<u8>,
    pub mirror_flag: Option<u8>,
    pub light_switch: Option<u8>,
}

async fn send_command(ip: &str, command: &serde_json::Value) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .post(format!("http://{}/post", ip))
        .json(command)
        .send()
        .await
        .map_err(|e| format!("Failed to send command: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Command failed with status: {}", response.status()));
    }

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(result)
}

#[tauri::command]
async fn scan_devices() -> Result<Vec<DivoomDevice>, String> {
    let mut devices = Vec::new();

    // Try Divoom cloud API first (most reliable)
    if let Ok(api_devices) = discover_via_divoom_api().await {
        devices.extend(api_devices);
    }

    // Remove duplicates based on IP address or MAC address
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

async fn discover_via_divoom_api() -> Result<Vec<DivoomDevice>, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .post("https://app.divoom-gz.com/Device/ReturnSameLANDevice")
        .send()
        .await
        .map_err(|e| format!("Failed to request Divoom API: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Divoom API returned status: {}", response.status()));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    let mut devices = Vec::new();

    if let Some(device_list) = json.get("DeviceList").and_then(|v| v.as_array()) {
        for device_json in device_list {
            let name = device_json
                .get("DeviceName")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown Device")
                .to_string();

            let ip_address = device_json
                .get("DevicePrivateIP")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let mac_address = device_json
                .get("DeviceMac")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let hardware = device_json
                .get("Hardware")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            // Map hardware code to device type
            let device_type = match hardware {
                400 => "Times Gate",
                401 => "Pixoo 64",
                402 => "Pixoo 32",
                403 => "Pixoo 16",
                404 => "Ditoo",
                405 => "Ditoo Plus",
                406 => "Ditoo Pro",
                407 => "Pixoo Max",
                408 => "Pixoo Mini",
                _ => "Unknown Divoom Device",
            }
            .to_string();

            devices.push(DivoomDevice {
                name,
                mac_address,
                device_type,
                ip_address,
                signal_strength: None,
                is_connected: true,
            });
        }
    }

    Ok(devices)
}

#[tauri::command]
async fn get_device_info(ip_address: String) -> Result<DeviceSettings, String> {
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
async fn set_brightness(ip_address: String, value: Number) {
    let result = send_command(
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
async fn set_switch_screen(ip_address: String, value: Number) {
    let result = send_command(
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
async fn set_temperature_mode(ip_address: String, value: Number) {
    let result = send_command(
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
async fn set_mirror_mode(ip_address: String, value: Number) {
    let result = send_command(
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
async fn set_24_hours_mode(ip_address: String, value: Number) {
    let result = send_command(
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            scan_devices,
            get_device_info,
            set_brightness,
            set_switch_screen,
            set_temperature_mode,
            set_mirror_mode,
            set_24_hours_mode,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
