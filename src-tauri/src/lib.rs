use base64::{engine::general_purpose, Engine as _};
use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageEncoder};
use serde::{Deserialize, Serialize};
use serde_json::Number;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::{fs, io};
use std::time::{Duration, Instant};
use sysinfo::{Components, Disks, System};
use tauri::Manager;

#[cfg(target_os = "windows")]
use wmi::{COMLibrary, WMIConnection};

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

#[derive(Debug, Clone, Serialize)]
pub struct DiskUsage {
    pub name: String,
    pub mount_point: String,
    pub total_space: u64,
    pub available_space: u64,
    pub used_space: u64,
    pub usage_percent: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemMetrics {
    pub cpu_usage: f32,
    pub cpu_temperature: Option<f32>,
    pub gpu_temperature: Option<f32>,
    pub memory_total: u64,
    pub memory_used: u64,
    pub disks: Vec<DiskUsage>,
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

fn find_temperature(components: &Components, keywords: &[&str]) -> Option<f32> {
    let mut best_temp: Option<f32> = None;
    for component in components.iter() {
        let label = component.label().to_lowercase();
        if keywords.iter().any(|keyword| label.contains(keyword)) {
            let temperature = component.temperature();
            best_temp = Some(best_temp.map_or(temperature, |current| current.max(temperature)));
        }
    }
    best_temp
}

#[derive(Debug, Deserialize)]
struct SidecarTemperatures {
    cpu_temperature: Option<f32>,
    gpu_temperature: Option<f32>,
}

fn normalize_temperature(value: Option<f32>) -> Option<f32> {
    value.and_then(|temp| {
        if (-30.0..=200.0).contains(&temp) {
            Some(temp)
        } else {
            None
        }
    })
}

fn sidecar_temperatures() -> Option<SidecarTemperatures> {
    let sidecar_path = std::env::var("LHM_SIDECAR_PATH").ok()?;
    let resolved_path = resolve_sidecar_path(&sidecar_path)?;
    let resolved_path = prepare_sidecar_path(&resolved_path).unwrap_or(resolved_path);
    let output = run_sidecar_with_timeout(&resolved_path, Duration::from_secs(2))?;
    if !output.status.success() {
        return None;
    }

    let mut temps: SidecarTemperatures = serde_json::from_slice(&output.stdout).ok()?;
    temps.cpu_temperature = normalize_temperature(temps.cpu_temperature);
    temps.gpu_temperature = normalize_temperature(temps.gpu_temperature);
    Some(temps)
}

fn resolve_sidecar_path(raw_path: &str) -> Option<PathBuf> {
    let path = Path::new(raw_path);
    if path.is_absolute() {
        return Some(path.to_path_buf());
    }

    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    Some(exe_dir.join(path))
}

fn prepare_sidecar_path(resolved_path: &Path) -> Option<PathBuf> {
    if !cfg!(debug_assertions) {
        return Some(resolved_path.to_path_buf());
    }

    let current_dir = std::env::current_dir().ok()?;
    if !resolved_path.starts_with(&current_dir) {
        return Some(resolved_path.to_path_buf());
    }

    let source_dir = resolved_path.parent()?;
    let temp_dir = std::env::temp_dir().join("divoom-monitor-sidecar");
    let target_path = temp_dir.join(resolved_path.file_name()?);

    if let Ok(()) = copy_dir_recursive(source_dir, &temp_dir) {
        return Some(target_path);
    }

    Some(resolved_path.to_path_buf())
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else if should_copy(&entry.path(), &target)? {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn should_copy(source: &Path, destination: &Path) -> io::Result<bool> {
    if !destination.exists() {
        return Ok(true);
    }

    let source_modified = fs::metadata(source)?.modified()?;
    let dest_modified = fs::metadata(destination)?.modified()?;
    Ok(source_modified > dest_modified)
}

fn run_sidecar_with_timeout(
    resolved_path: &Path,
    timeout: Duration,
) -> Option<std::process::Output> {
    let mut child = Command::new(resolved_path)
        .current_dir(std::env::temp_dir())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let start = Instant::now();
    loop {
        if start.elapsed() >= timeout {
            let _ = child.kill();
            return None;
        }

        match child.try_wait() {
            Ok(Some(_status)) => return child.wait_with_output().ok(),
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(_) => return None,
        }
    }
}

#[cfg(target_os = "windows")]
#[derive(Deserialize, Debug)]
struct ThermalZoneTemperature {
    #[serde(rename = "CurrentTemperature")]
    current_temperature: u32,
}

#[cfg(target_os = "windows")]
fn wmi_cpu_temperature() -> Option<f32> {
    let com_library = COMLibrary::new().ok()?;
    let wmi_connection = WMIConnection::new(com_library.into()).ok()?;
    let temps: Vec<ThermalZoneTemperature> = wmi_connection
        .raw_query("SELECT CurrentTemperature FROM MSAcpi_ThermalZoneTemperature")
        .ok()?;

    temps
        .iter()
        .map(|entry| (entry.current_temperature as f32 / 10.0) - 273.15)
        .reduce(f32::max)
}

#[cfg(target_os = "windows")]
fn nvml_gpu_temperature() -> Option<f32> {
    use nvml_wrapper::enum_wrappers::device::TemperatureSensor;
    let nvml = nvml_wrapper::Nvml::init().ok()?;
    let device_count = nvml.device_count().ok()?;
    let mut best_temp = None;

    for index in 0..device_count {
        let device = nvml.device_by_index(index).ok()?;
        if let Ok(temp) = device.temperature(TemperatureSensor::Gpu) {
            let temp = temp as f32;
            best_temp = Some(best_temp.map_or(temp, |current: f32| current.max(temp)));
        }
    }

    best_temp
}

fn get_cpu_temperature(components: &Components) -> Option<f32> {
    if let Some(temps) = sidecar_temperatures() {
        if temps.cpu_temperature.is_some() {
            return temps.cpu_temperature;
        }
    }

    #[cfg(target_os = "windows")]
    {
        wmi_cpu_temperature().or_else(|| find_temperature(components, &["cpu", "package"]))
    }

    #[cfg(not(target_os = "windows"))]
    {
        find_temperature(components, &["cpu", "package"])
    }
}

fn get_gpu_temperature(components: &Components) -> Option<f32> {
    if let Some(temps) = sidecar_temperatures() {
        if temps.gpu_temperature.is_some() {
            return temps.gpu_temperature;
        }
    }

    #[cfg(target_os = "windows")]
    {
        nvml_gpu_temperature().or_else(|| find_temperature(components, &["gpu", "graphics"]))
    }

    #[cfg(not(target_os = "windows"))]
    {
        find_temperature(components, &["gpu", "graphics"])
    }
}

#[tauri::command]
fn get_system_metrics() -> Result<SystemMetrics, String> {
    let mut system = System::new_all();
    let mut components = Components::new();
    let mut disks = Disks::new();

    system.refresh_cpu();
    std::thread::sleep(Duration::from_millis(200));
    system.refresh_cpu();
    system.refresh_memory();
    components.refresh();
    disks.refresh();

    let cpu_usage = system.global_cpu_info().cpu_usage();

    let cpu_temperature = get_cpu_temperature(&components);
    let gpu_temperature = get_gpu_temperature(&components);

    let disks = disks
        .iter()
        .map(|disk| {
            let total_space = disk.total_space();
            let available_space = disk.available_space();
            let used_space = total_space.saturating_sub(available_space);
            let usage_percent = if total_space > 0 {
                (used_space as f32 / total_space as f32) * 100.0
            } else {
                0.0
            };

            DiskUsage {
                name: disk.name().to_string_lossy().to_string(),
                mount_point: disk.mount_point().to_string_lossy().to_string(),
                total_space,
                available_space,
                used_space,
                usage_percent,
            }
        })
        .collect();

    Ok(SystemMetrics {
        cpu_usage,
        cpu_temperature,
        gpu_temperature,
        memory_total: system.total_memory(),
        memory_used: system.used_memory(),
        disks,
    })
}

#[cfg(debug_assertions)]
fn setup_devtools(app: &tauri::App) {
    let main_window = app.get_webview_window("main").unwrap();
    main_window.open_devtools();
}

#[cfg(not(debug_assertions))]
fn setup_devtools(_app: &tauri::App) {
    // Ничего не делать в production
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    dotenvy::dotenv().ok();
    tauri::Builder::default()
        .setup(|app| {
            setup_devtools(app);
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
            get_system_metrics,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
