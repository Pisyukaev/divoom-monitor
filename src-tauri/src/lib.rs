use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;
use std::time::Duration;
use tokio::time::timeout;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivoomDevice {
    pub name: String,
    pub mac_address: Option<String>,
    pub device_type: String,
    pub ip_address: Option<String>,
    pub signal_strength: Option<i32>,
    pub is_connected: bool,
    pub model: Option<String>,
}

#[tauri::command]
async fn scan_devices() -> Result<Vec<DivoomDevice>, String> {
    let mut devices = Vec::new();

    // Try Divoom cloud API first (most reliable)
    if let Ok(api_devices) = discover_via_divoom_api().await {
        devices.extend(api_devices);
    }

    // Try mDNS discovery as fallback
    if let Ok(mdns_devices) = discover_via_mdns().await {
        devices.extend(mdns_devices);
    }

    // Scan local network for Divoom devices as last resort
    if let Ok(network_devices) = scan_local_network().await {
        devices.extend(network_devices);
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

            // Try to get more info from the device
            let mut device = DivoomDevice {
                name,
                mac_address,
                device_type,
                ip_address,
                signal_strength: None,
                is_connected: true,
                model: None,
            };

            // Try to get additional info from device API
            if let Some(ref ip) = device.ip_address {
                if let Ok(detailed_info) = check_divoom_device(ip).await {
                    // Merge information
                    if detailed_info.model.is_some() {
                        device.model = detailed_info.model;
                    }
                    if detailed_info.signal_strength.is_some() {
                        device.signal_strength = detailed_info.signal_strength;
                    }
                    device.is_connected = detailed_info.is_connected;
                }
            }

            devices.push(device);
        }
    }

    Ok(devices)
}

async fn discover_via_mdns() -> Result<Vec<DivoomDevice>, String> {
    let mut devices = Vec::new();

    // Try to discover Divoom devices via mDNS/Bonjour
    // This is a fallback method, so we'll keep it simple
    // The mdns crate API may vary, so we'll skip detailed implementation for now
    // and rely primarily on the cloud API

    Ok(devices)
}

async fn scan_local_network() -> Result<Vec<DivoomDevice>, String> {
    let mut devices = Vec::new();

    // Get local network range (simplified - assumes common 192.168.1.x range)
    // In production, you'd want to detect the actual network range
    let base_ip = Ipv4Addr::new(192, 168, 1, 0);

    // Scan a limited range to avoid long waits
    let start = 1;
    let end = 50; // Scan first 50 IPs

    let mut handles = Vec::new();

    for i in start..=end {
        let ip = Ipv4Addr::new(
            base_ip.octets()[0],
            base_ip.octets()[1],
            base_ip.octets()[2],
            i,
        );
        let ip_str = ip.to_string();

        let handle = tokio::spawn(async move { check_divoom_device(&ip_str).await });

        handles.push(handle);
    }

    // Wait for all checks with a timeout
    for handle in handles {
        if let Ok(Ok(device)) = timeout(Duration::from_millis(500), handle).await {
            if let Ok(device) = device {
                devices.push(device);
            }
        }
    }

    Ok(devices)
}

async fn check_divoom_device(ip: &str) -> Result<DivoomDevice, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // Common Divoom API endpoints
    let endpoints = vec![
        format!("http://{}/device/info", ip),
        format!("http://{}/api/device/info", ip),
        format!("http://{}/divoom/device/info", ip),
    ];

    for endpoint in endpoints {
        if let Ok(response) = client.get(&endpoint).send().await {
            if response.status().is_success() {
                // Try to parse device info
                if let Ok(json) = response.json::<serde_json::Value>().await {
                    return parse_device_info(ip, json);
                }
            }
        }
    }

    // If no API endpoint works, try a simple HTTP check
    if let Ok(response) = client.get(format!("http://{}", ip)).send().await {
        if response.status().is_success() {
            // Check if response headers indicate Divoom device
            if let Some(server) = response.headers().get("server") {
                if let Ok(server_str) = server.to_str() {
                    if server_str.to_lowercase().contains("divoom") {
                        return Ok(DivoomDevice {
                            name: format!("Divoom Device ({})", ip),
                            mac_address: None,
                            device_type: "Unknown".to_string(),
                            ip_address: Some(ip.to_string()),
                            signal_strength: None,
                            is_connected: true,
                            model: None,
                        });
                    }
                }
            }
        }
    }

    Err("Not a Divoom device".to_string())
}

fn parse_device_info(ip: &str, json: serde_json::Value) -> Result<DivoomDevice, String> {
    let name = json["name"]
        .as_str()
        .unwrap_or("Unknown Divoom Device")
        .to_string();

    let device_type = json["device_type"]
        .as_str()
        .or_else(|| json["type"].as_str())
        .unwrap_or("Unknown")
        .to_string();

    let mac_address = json["mac_address"]
        .as_str()
        .or_else(|| json["mac"].as_str())
        .map(|s| s.to_string());

    let model = json["model"]
        .as_str()
        .or_else(|| json["model_name"].as_str())
        .map(|s| s.to_string());

    let signal_strength = json["signal_strength"]
        .as_i64()
        .or_else(|| json["rssi"].as_i64())
        .map(|v| v as i32);

    let is_connected = json["connected"]
        .as_bool()
        .or_else(|| json["is_connected"].as_bool())
        .unwrap_or(true);

    Ok(DivoomDevice {
        name,
        mac_address,
        device_type,
        ip_address: Some(ip.to_string()),
        signal_strength,
        is_connected,
        model,
    })
}

#[tauri::command]
async fn get_device_info(ip_address: String) -> Result<DivoomDevice, String> {
    check_divoom_device(&ip_address).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            scan_devices,
            get_device_info
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
