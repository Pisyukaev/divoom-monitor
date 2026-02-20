mod system_metrics;

use base64::{engine::general_purpose, Engine as _};
use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageEncoder};
use serde::{Deserialize, Serialize};
use serde_json::Number;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

// Static counter for PicID, starting from 1000
static PIC_ID_COUNTER: AtomicU32 = AtomicU32::new(1000);

fn get_next_pic_id() -> u32 {
    PIC_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivoomDevice {
    pub name: String,
    pub mac_address: Option<String>,
    pub device_type: String,
    pub ip_address: Option<String>,
    pub signal_strength: Option<i32>,
    pub is_connected: bool,
    pub device_id: Option<u64>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextConfig {
    pub id: u8,
    pub content: String,
    pub x: u8,
    pub y: u8,
    pub font: Option<u8>,
    pub color: Option<String>,
    pub alignment: Option<u8>,
    pub text_width: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LcdInfo {
    pub lcd_clock_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LcdIndependenceInfo {
    pub lcd_independence: u64,
    pub lcd_list: Vec<LcdInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LcdInfoResponse {
    pub device_id: u64,
    pub independence_list: Vec<LcdIndependenceInfo>,
}

async fn send_command(ip: &str, command: &serde_json::Value) -> Result<serde_json::Value, String> {
    send_command_with_timeout(ip, command, Duration::from_millis(500)).await
}

async fn send_command_with_timeout(
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

            let device_id = device_json
                .get("DeviceId")
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
                device_id: Some(device_id),
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
async fn set_switch_screen(ip_address: String, value: Number) {
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
async fn set_temperature_mode(ip_address: String, value: Number) {
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
async fn set_mirror_mode(ip_address: String, value: Number) {
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
async fn set_24_hours_mode(ip_address: String, value: Number) {
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
async fn reboot_device(ip_address: String) {
    let _ = send_command(
        &ip_address,
        &serde_json::json!({
            "Command": "Device/SysReboot",
        }),
    )
    .await
    .map_err(|e| format!("Failed to send command: {}", e));
}

fn resize_image(img: DynamicImage, max_width: u32, max_height: u32) -> Result<Vec<u8>, String> {
    let resized = img.resize_exact(max_width, max_height, image::imageops::FilterType::Lanczos3);
    let rgba = resized.to_rgba8();
    let mut buffer = Vec::new();
    {
        let encoder = JpegEncoder::new(&mut buffer);
        encoder
            .write_image(
                rgba.as_raw(),
                rgba.width(),
                rgba.height(),
                image::ColorType::Rgba8,
            )
            .map_err(|e| format!("Failed to encode image: {}", e))?;
    }

    Ok(buffer)
}

async fn load_image_from_url(url: &str) -> Result<DynamicImage, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to download image: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to download image: {}", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read image bytes: {}", e))?;

    image::load_from_memory(&bytes).map_err(|e| format!("Failed to decode image: {}", e))
}

async fn load_image_from_file(file_path: &str) -> Result<DynamicImage, String> {
    image::open(Path::new(file_path)).map_err(|e| format!("Failed to open image file: {}", e))
}

#[tauri::command]
async fn upload_image_from_url(
    ip_address: String,
    screen_index: u32,
    url: String,
) -> Result<(), String> {
    let img = load_image_from_url(&url).await?;
    let image_data = resize_image(img, 128, 128)?;
    let base64_data = general_purpose::STANDARD.encode(&image_data);

    // Create LCD array with 1 at screen_index position, 0 elsewhere
    let mut lcd_array = [0u8; 5];
    if screen_index < 5 {
        lcd_array[screen_index as usize] = 1;
    }

    let pic_id = get_next_pic_id();

    send_command_with_timeout(
        &ip_address,
        &serde_json::json!({
            "Command": "Draw/SendHttpGif",
            "LCDArray": lcd_array,
            "PicNum": 1,
            "PicWidth": 128,
            "PicOffset": 0,
            "PicID": pic_id,
            "PicSpeed": 1000,
            "PicData": base64_data
        }),
        Duration::from_secs(1),
    )
    .await
    .map_err(|e| format!("Failed to send image command: {}", e))?;

    Ok(())
}

#[tauri::command]
async fn upload_image_from_file(
    ip_address: String,
    screen_index: u32,
    file_path: String,
) -> Result<(), String> {
    let img = load_image_from_file(&file_path).await?;
    let image_data = resize_image(img, 128, 128)?;
    let base64_data = general_purpose::STANDARD.encode(&image_data);

    // Create LCD array with 1 at screen_index position, 0 elsewhere
    let mut lcd_array = [0u8; 5];
    if screen_index < 5 {
        lcd_array[screen_index as usize] = 1;
    }

    let pic_id = get_next_pic_id();

    send_command_with_timeout(
        &ip_address,
        &serde_json::json!({
            "Command": "Draw/SendHttpGif",
            "LCDArray": lcd_array,
            "PicNum": 1,
            "PicWidth": 128,
            "PicOffset": 0,
            "PicID": pic_id,
            "PicSpeed": 1000,
            "PicData": base64_data
        }),
        Duration::from_secs(1),
    )
    .await
    .map_err(|e| format!("Failed to send image command: {}", e))?;

    Ok(())
}

#[tauri::command]
async fn set_screen_text(
    ip_address: String,
    screen_index: u32,
    text_config: TextConfig,
) -> Result<(), String> {
    let color = text_config
        .color
        .unwrap_or_else(|| "255,255,255".to_string());
    let font = text_config.font.unwrap_or(7);
    let alignment = text_config.alignment.unwrap_or(0);
    let text_width = text_config.text_width.unwrap_or(64);

    send_command(
        &ip_address,
        &serde_json::json!({
            "Command": "Draw/SendHttpText",
            "LcdIndex": screen_index,
            "TextId": text_config.id,
            "x": text_config.x,
            "y": text_config.y,
            "dir": 0,
            "font": font,
            "TextWidth": text_width,
            "speed": 100,
            "TextString": text_config.content,
            "color": color,
            "align": alignment
        }),
    )
    .await
    .map_err(|e| format!("Failed to send text command: {}", e))?;

    Ok(())
}

#[tauri::command]
async fn get_lcd_info(ip_address: String) -> Result<LcdInfoResponse, String> {
    let devices = discover_via_divoom_api().await?;

    let device = devices
        .iter()
        .find(|d| d.ip_address.as_deref() == Some(&ip_address))
        .ok_or_else(|| format!("Device with IP {} not found", ip_address))?;

    let device_id = device.device_id.ok_or("Device has no ID")?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let url = format!(
        "https://app.divoom-gz.com/Channel/Get5LcdInfoV2?DeviceType=LCD&DeviceId={}",
        device_id
    );

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to request LCD info: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "LCD info API returned status: {}",
            response.status()
        ));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse LCD info response: {}", e))?;

    let mut independence_list = Vec::new();

    if let Some(list) = json.get("LcdIndependenceList").and_then(|v| v.as_array()) {
        for item in list {
            let lcd_independence = item
                .get("LcdIndependence")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            let mut lcd_list = Vec::new();
            if let Some(lcds) = item.get("LcdList").and_then(|v| v.as_array()) {
                for lcd in lcds {
                    let lcd_clock_id = lcd.get("LcdClockId").and_then(|v| v.as_u64()).unwrap_or(0);
                    lcd_list.push(LcdInfo { lcd_clock_id });
                }
            }

            independence_list.push(LcdIndependenceInfo {
                lcd_independence,
                lcd_list,
            });
        }
    }

    Ok(LcdInfoResponse {
        device_id,
        independence_list,
    })
}

#[tauri::command]
async fn activate_pc_monitor(
    ip_address: String,
    device_id: u64,
    lcd_independence: u64,
    lcd_index: u32,
) -> Result<(), String> {
    send_command(
        &ip_address,
        &serde_json::json!({
            "Command": "Channel/SetClockSelectId",
            "LcdIndependence": lcd_independence,
            "DeviceId": device_id,
            "LcdIndex": lcd_index,
            "ClockId": 625 // PC Monitor clock
        }),
    )
    .await
    .map_err(|e| format!("Failed to activate PC monitor: {}", e))?;

    Ok(())
}

#[tauri::command]
async fn send_pc_metrics(
    ip_address: String,
    lcd_index: u32,
    disp_data: Vec<String>,
) -> Result<(), String> {
    send_command(
        &ip_address,
        &serde_json::json!({
            "Command": "Device/UpdatePCParaInfo",
            "ScreenList": [{
                "LcdId": lcd_index,
                "DispData": disp_data
            }]
        }),
    )
    .await
    .map_err(|e| format!("Failed to send PC metrics: {}", e))?;

    Ok(())
}

#[cfg(debug_assertions)]
fn setup_devtools(app: &tauri::App) {
    if let Some(main_window) = app.get_webview_window("main") {
        main_window.open_devtools();
    }
}

#[cfg(not(debug_assertions))]
fn setup_devtools(_app: &tauri::App) {
    // Ничего не делать в production
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    dotenvy::dotenv().ok();

    // Регистрируем обработчик для остановки sidecar при выходе из процесса
    // Это сработает даже если приложение завершится неожиданно
    let _ = std::panic::set_hook(Box::new(|_| {
        system_metrics::stop_sidecar_service();
    }));

    tauri::Builder::default()
        .setup(|app| {
            setup_devtools(app);
            system_metrics::setup_sidecar_service();

            if let Some(window) = app.get_webview_window("main") {
                let _ = window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { .. } = event {
                        eprintln!("[App] Window closing, stopping sidecar...");
                        system_metrics::stop_sidecar_service();
                    }
                });
            }

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            scan_devices,
            get_device_info,
            set_brightness,
            set_switch_screen,
            set_temperature_mode,
            set_mirror_mode,
            set_24_hours_mode,
            upload_image_from_url,
            upload_image_from_file,
            set_screen_text,
            reboot_device,
            system_metrics::get_system_metrics,
            get_lcd_info,
            activate_pc_monitor,
            send_pc_metrics,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    // Дополнительная защита - остановка sidecar при выходе из run()
    // Это сработает, если приложение завершится нормально
    system_metrics::stop_sidecar_service();
}
