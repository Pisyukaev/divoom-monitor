use base64::{engine::general_purpose, Engine as _};
use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageEncoder};
use serde::{Deserialize, Serialize};
use serde_json::Number;
use std::path::{Path, PathBuf};
use std::process::{Command, Child};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use sysinfo::{Components, Disks, System};
use tauri::Manager;

#[cfg(target_os = "windows")]
use wmi::{COMLibrary, WMIConnection};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

// Static counter for PicID, starting from 1000
static PIC_ID_COUNTER: AtomicU32 = AtomicU32::new(1000);

// Sidecar process handle
static SIDECAR_PROCESS: Mutex<Option<Child>> = Mutex::new(None);
// Флаг запуска sidecar для предотвращения повторных попыток
static SIDECAR_STARTING: Mutex<bool> = Mutex::new(false);

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
        #[cfg(debug_assertions)]
        eprintln!("[Normalize] Checking temperature: {:.1}°C", temp);
        
        if (-30.0..=200.0).contains(&temp) {
            #[cfg(debug_assertions)]
            eprintln!("[Normalize] Temperature {:.1}°C is within valid range", temp);
            Some(temp)
        } else {
            #[cfg(debug_assertions)]
            eprintln!("[Normalize] Temperature {:.1}°C is OUTSIDE valid range (-30..200), filtering out", temp);
            None
        }
    })
}

async fn sidecar_temperatures() -> Option<SidecarTemperatures> {
    // Пытаемся получить данные через HTTP запрос к sidecar серверу
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            #[cfg(debug_assertions)]
            eprintln!("[LibreHardwareMonitor] Failed to create HTTP client: {}", e);
            return None;
        }
    };
    
    #[cfg(debug_assertions)]
    eprintln!("[LibreHardwareMonitor] Requesting temperatures from http://localhost:8765/");
    
    let response = match client
        .get("http://localhost:8765/")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            #[cfg(debug_assertions)]
            eprintln!("[LibreHardwareMonitor] HTTP request failed: {} (sidecar may not be running)", e);
            return None;
        }
    };
    
    if !response.status().is_success() {
        #[cfg(debug_assertions)]
        eprintln!("[LibreHardwareMonitor] HTTP request failed with status: {}", response.status());
        return None;
    }
    
    let json_text = match response.text().await {
        Ok(t) => t,
        Err(e) => {
            #[cfg(debug_assertions)]
            eprintln!("[LibreHardwareMonitor] Failed to read response body: {}", e);
            return None;
        }
    };
    
    #[cfg(debug_assertions)]
    eprintln!("[LibreHardwareMonitor] Raw JSON response: {}", json_text);
    
    let mut temps: SidecarTemperatures = match serde_json::from_str(&json_text) {
        Ok(t) => t,
        Err(e) => {
            #[cfg(debug_assertions)]
            eprintln!("[LibreHardwareMonitor] Failed to parse JSON: {} (response was: {})", e, json_text);
            return None;
        }
    };
    
    #[cfg(debug_assertions)]
    eprintln!("[LibreHardwareMonitor] Parsed temperatures - CPU: {:?}°C, GPU: {:?}°C", 
        temps.cpu_temperature, temps.gpu_temperature);
    
    let cpu_before = temps.cpu_temperature;
    let gpu_before = temps.gpu_temperature;
    
    temps.cpu_temperature = normalize_temperature(temps.cpu_temperature);
    temps.gpu_temperature = normalize_temperature(temps.gpu_temperature);
    
    #[cfg(debug_assertions)]
    {
        if cpu_before != temps.cpu_temperature {
            eprintln!("[LibreHardwareMonitor] CPU temperature normalized: {:?}°C -> {:?}°C", 
                cpu_before, temps.cpu_temperature);
        }
        if gpu_before != temps.gpu_temperature {
            eprintln!("[LibreHardwareMonitor] GPU temperature normalized: {:?}°C -> {:?}°C", 
                gpu_before, temps.gpu_temperature);
        }
        eprintln!("[LibreHardwareMonitor] Final temperatures - CPU: {:?}°C, GPU: {:?}°C", 
            temps.cpu_temperature, temps.gpu_temperature);
    }
    
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

fn start_sidecar_service() -> Result<(), String> {
    // Безопасная обертка для всех операций
    let result = std::panic::catch_unwind(|| {
        let sidecar_path = std::env::var("LHM_SIDECAR_PATH")
            .map_err(|_| "LHM_SIDECAR_PATH environment variable not set")?;
        
        let resolved_path = resolve_sidecar_path(&sidecar_path)
            .ok_or_else(|| format!("Failed to resolve sidecar path: {}", sidecar_path))?;
        
        if !resolved_path.exists() {
            #[cfg(debug_assertions)]
            eprintln!("Warning: Sidecar executable not found at: {:?}", resolved_path);
            #[cfg(debug_assertions)]
            eprintln!("Sidecar service will not be started. Temperature monitoring may be limited.");
            return Err(format!("Sidecar executable not found at: {:?}", resolved_path));
        }
        
        // Проверяем, не запущен ли уже sidecar (простая TCP проверка порта)
        use std::net::{TcpStream, SocketAddr};
        use std::time::Duration as StdDuration;
        
        // Безопасная проверка порта с таймаутом
        let addr: SocketAddr = "127.0.0.1:8765".parse()
            .map_err(|_| "Failed to parse socket address")?;
        
        let port_check = TcpStream::connect_timeout(&addr, StdDuration::from_millis(100));
        
        if port_check.is_ok() {
            #[cfg(debug_assertions)]
            eprintln!("Sidecar service is already running (port 8765 is open)");
            return Ok(()); // Sidecar уже запущен
        }
        
        let mut process = Command::new(&resolved_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to start sidecar process: {}", e))?;
        
        // Проверяем, что процесс запустился успешно
        match process.try_wait() {
            Ok(Some(status)) => {
                // Читаем stderr для диагностики
                let mut stderr = String::new();
                if let Some(mut child_stderr) = process.stderr.take() {
                    use std::io::Read;
                    let _ = std::io::BufReader::new(&mut child_stderr).read_to_string(&mut stderr);
                }
                return Err(format!("Sidecar process exited immediately with status: {:?}. Stderr: {}", status, stderr));
            }
            Ok(None) => {
                // Процесс работает - это хорошо
            }
            Err(e) => {
                return Err(format!("Failed to check sidecar process status: {}", e));
            }
        }
        
        // Сохраняем handle процесса
        let mut sidecar_guard = SIDECAR_PROCESS.lock()
            .map_err(|e| format!("Failed to lock sidecar process mutex: {}", e))?;
        *sidecar_guard = Some(process);
        
        // Даем серверу время на запуск
        std::thread::sleep(Duration::from_millis(1000));
        
        // Проверяем, что сервер действительно запустился (простая TCP проверка)
        let check_addr: SocketAddr = "127.0.0.1:8765".parse()
            .map_err(|_| "Failed to parse socket address")?;
        let _ = TcpStream::connect_timeout(&check_addr, StdDuration::from_millis(500));
        
        #[cfg(debug_assertions)]
        eprintln!("Sidecar service started successfully");
        
        Ok(())
    });
    
    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(_) => Err("Panic occurred while starting sidecar service".to_string()),
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

async fn get_cpu_temperature(components: &Components) -> Option<f32> {
    #[cfg(debug_assertions)]
    eprintln!("[CPU Temperature] Starting temperature retrieval...");
    
    if let Some(temps) = sidecar_temperatures().await {
        #[cfg(debug_assertions)]
        eprintln!("[CPU Temperature] Sidecar response received, CPU temp: {:?}", temps.cpu_temperature);
        
        if temps.cpu_temperature.is_some() {
            #[cfg(debug_assertions)]
            eprintln!("[CPU Temperature] Using LibreHardwareMonitor: {:.1}°C", temps.cpu_temperature.unwrap());
            return temps.cpu_temperature;
        } else {
            #[cfg(debug_assertions)]
            eprintln!("[CPU Temperature] LibreHardwareMonitor returned None (temperature was filtered or not found), trying fallback methods");
        }
    } else {
        #[cfg(debug_assertions)]
        eprintln!("[CPU Temperature] LibreHardwareMonitor unavailable (sidecar not responding or error), trying fallback methods");
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(wmi_temp) = wmi_cpu_temperature() {
            #[cfg(debug_assertions)]
            eprintln!("[CPU Temperature] Using WMI: {:.1}°C", wmi_temp);
            return Some(wmi_temp);
        }
        if let Some(sysinfo_temp) = find_temperature(components, &["cpu", "package"]) {
            #[cfg(debug_assertions)]
            eprintln!("[CPU Temperature] Using sysinfo: {:.1}°C", sysinfo_temp);
            return Some(sysinfo_temp);
        }
        #[cfg(debug_assertions)]
        eprintln!("[CPU Temperature] No temperature data available from any source");
        None
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Some(sysinfo_temp) = find_temperature(components, &["cpu", "package"]) {
            #[cfg(debug_assertions)]
            eprintln!("[CPU Temperature] Using sysinfo: {:.1}°C", sysinfo_temp);
            return Some(sysinfo_temp);
        }
        #[cfg(debug_assertions)]
        eprintln!("[CPU Temperature] No temperature data available");
        None
    }
}

async fn get_gpu_temperature(components: &Components) -> Option<f32> {
    if let Some(temps) = sidecar_temperatures().await {
        if temps.gpu_temperature.is_some() {
            #[cfg(debug_assertions)]
            eprintln!("[GPU Temperature] Using LibreHardwareMonitor: {:.1}°C", temps.gpu_temperature.unwrap());
            return temps.gpu_temperature;
        } else {
            #[cfg(debug_assertions)]
            eprintln!("[GPU Temperature] LibreHardwareMonitor returned None, trying fallback methods");
        }
    } else {
        #[cfg(debug_assertions)]
        eprintln!("[GPU Temperature] LibreHardwareMonitor unavailable, trying fallback methods");
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(nvml_temp) = nvml_gpu_temperature() {
            #[cfg(debug_assertions)]
            eprintln!("[GPU Temperature] Using NVML: {:.1}°C", nvml_temp);
            return Some(nvml_temp);
        }
        if let Some(sysinfo_temp) = find_temperature(components, &["gpu", "graphics"]) {
            #[cfg(debug_assertions)]
            eprintln!("[GPU Temperature] Using sysinfo: {:.1}°C", sysinfo_temp);
            return Some(sysinfo_temp);
        }
        #[cfg(debug_assertions)]
        eprintln!("[GPU Temperature] No temperature data available from any source");
        None
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Some(sysinfo_temp) = find_temperature(components, &["gpu", "graphics"]) {
            #[cfg(debug_assertions)]
            eprintln!("[GPU Temperature] Using sysinfo: {:.1}°C", sysinfo_temp);
            return Some(sysinfo_temp);
        }
        #[cfg(debug_assertions)]
        eprintln!("[GPU Temperature] No temperature data available");
        None
    }
}

#[tauri::command]
async fn get_system_metrics() -> Result<SystemMetrics, String> {
    let mut system = System::new_all();
    let mut components = Components::new();
    let mut disks = Disks::new();

    system.refresh_cpu();
    tokio::time::sleep(Duration::from_millis(200)).await;
    system.refresh_cpu();
    system.refresh_memory();
    components.refresh();
    disks.refresh();

    let cpu_usage = system.global_cpu_info().cpu_usage();

    // Получаем температуры (в режиме разработки ошибки не приводят к падению приложения)
    let cpu_temperature = get_cpu_temperature(&components).await;
    let gpu_temperature = get_gpu_temperature(&components).await;
    
    #[cfg(debug_assertions)]
    eprintln!("[SystemMetrics] Final metrics - CPU: {:.1}% usage, CPU temp: {:?}°C, GPU temp: {:?}°C, Memory: {:.1}% used",
        cpu_usage,
        cpu_temperature.map(|t| format!("{:.1}", t)).unwrap_or_else(|| "N/A".to_string()),
        gpu_temperature.map(|t| format!("{:.1}", t)).unwrap_or_else(|| "N/A".to_string()),
        (system.used_memory() as f32 / system.total_memory() as f32) * 100.0
    );

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
    if let Some(main_window) = app.get_webview_window("main") {
        main_window.open_devtools();
    }
}

#[cfg(not(debug_assertions))]
fn setup_devtools(_app: &tauri::App) {
    // Ничего не делать в production
}

#[allow(dead_code)]
fn stop_sidecar_service() {
    let mut sidecar_guard = match SIDECAR_PROCESS.lock() {
        Ok(guard) => guard,
        Err(_) => {
            #[cfg(debug_assertions)]
            eprintln!("Failed to lock sidecar process mutex for shutdown");
            return;
        }
    };
    
    if let Some(mut child) = sidecar_guard.take() {
        #[cfg(debug_assertions)]
        eprintln!("Stopping sidecar service (PID: {:?})...", child.id());
        
        // Пытаемся корректно завершить процесс
        let kill_result = child.kill();
        
        #[cfg(debug_assertions)]
        {
            if kill_result.is_ok() {
                eprintln!("Sidecar service stop signal sent");
            } else {
                eprintln!("Failed to send stop signal to sidecar: {:?}", kill_result);
            }
        }
        
        // Не ждем завершения, чтобы не блокировать поток
        // Процесс завершится автоматически
    } else {
        #[cfg(debug_assertions)]
        eprintln!("No sidecar process to stop");
    }
}

fn setup_sidecar_service() {
    // Проверяем, не запускается ли уже sidecar
    let mut starting_guard = match SIDECAR_STARTING.lock() {
        Ok(guard) => guard,
        Err(_) => {
            #[cfg(debug_assertions)]
            eprintln!("Failed to lock SIDECAR_STARTING mutex - skipping sidecar startup");
            return;
        }
    };
    
    if *starting_guard {
        #[cfg(debug_assertions)]
        eprintln!("Sidecar is already starting - skipping");
        return;
    }
    
    *starting_guard = true;
    drop(starting_guard);
    
    // Запускаем sidecar в отдельном потоке, чтобы не блокировать запуск приложения
    // Обертываем в catch_unwind для защиты от паник
    std::thread::spawn(|| {
        let result = std::panic::catch_unwind(|| {
            // Небольшая задержка перед запуском sidecar
            std::thread::sleep(Duration::from_millis(500));
            
            // В режиме разработки не падаем при ошибках запуска sidecar
            match start_sidecar_service() {
                Ok(_) => {
                    #[cfg(debug_assertions)]
                    eprintln!("LibreHardwareMonitor sidecar service started");
                }
                Err(e) => {
                    #[cfg(debug_assertions)]
                    eprintln!("Warning: Failed to start sidecar service: {}", e);
                    #[cfg(debug_assertions)]
                    eprintln!("Application will continue, but temperature monitoring may be limited.");
                    // В production режиме можно логировать ошибку, но не падать
                    #[cfg(not(debug_assertions))]
                    eprintln!("Failed to start sidecar service: {}", e);
                }
            }
            
            // Сбрасываем флаг после попытки запуска
            if let Ok(mut guard) = SIDECAR_STARTING.lock() {
                *guard = false;
            }
        });
        
        if result.is_err() {
            #[cfg(debug_assertions)]
            eprintln!("Panic occurred in sidecar setup thread - ignoring");
            // Сбрасываем флаг при панике
            if let Ok(mut guard) = SIDECAR_STARTING.lock() {
                *guard = false;
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    dotenvy::dotenv().ok();
    
    // Регистрируем обработчик для остановки sidecar при выходе из процесса
    // Это сработает даже если приложение завершится неожиданно
    let _ = std::panic::set_hook(Box::new(|_| {
        stop_sidecar_service();
    }));
    
    tauri::Builder::default()
        .setup(|app| {
            setup_devtools(app);
            setup_sidecar_service();
            
            // Останавливаем sidecar при закрытии окна
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { .. } = event {
                        #[cfg(debug_assertions)]
                        eprintln!("Window closing, stopping sidecar service...");
                        stop_sidecar_service();
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
            get_system_metrics,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
    
    // Дополнительная защита - остановка sidecar при выходе из run()
    // Это сработает, если приложение завершится нормально
    stop_sidecar_service();
}
