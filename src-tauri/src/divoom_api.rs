use std::time::Duration;

use crate::models::DivoomDevice;

pub async fn send_command(
    ip: &str,
    command: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    send_command_with_timeout(ip, command, Duration::from_millis(500)).await
}

pub async fn send_command_with_timeout(
    ip: &str,
    command: &serde_json::Value,
    timeout: Duration,
) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(timeout)
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

pub async fn discover_via_divoom_api() -> Result<Vec<DivoomDevice>, String> {
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
        return Err(format!(
            "Divoom API returned status: {}",
            response.status()
        ));
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

            let device_id = device_json
                .get("DeviceId")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

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
                device_id: Some(device_id),
            });
        }
    }

    Ok(devices)
}
