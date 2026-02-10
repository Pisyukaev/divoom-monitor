use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::Mutex;
use std::time::Duration;
use sysinfo::{Components, Disks, System};

#[cfg(target_os = "windows")]
use wmi::{COMLibrary, WMIConnection};

// Sidecar process handle
static SIDECAR_PROCESS: Mutex<Option<Child>> = Mutex::new(None);
// Флаг запуска sidecar для предотвращения повторных попыток
static SIDECAR_STARTING: Mutex<bool> = Mutex::new(false);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskUsage {
    pub name: String,
    pub mount_point: String,
    pub total_space: u64,
    pub available_space: u64,
    pub used_space: u64,
    pub usage_percent: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_usage: f32,
    pub cpu_temperature: Option<f32>,
    pub gpu_temperature: Option<f32>,
    pub memory_total: u64,
    pub memory_used: u64,
    pub disks: Vec<DiskUsage>,
}

#[derive(Debug, Deserialize)]
struct SidecarTemperatures {
    cpu_temperature: Option<f32>,
    gpu_temperature: Option<f32>,
}

#[cfg(target_os = "windows")]
#[derive(Deserialize, Debug)]
struct ThermalZoneTemperature {
    #[serde(rename = "CurrentTemperature")]
    current_temperature: u32,
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

fn normalize_temperature(value: Option<f32>) -> Option<f32> {
    value.and_then(|temp| {
        if (-30.0..=200.0).contains(&temp) {
            Some(temp)
        } else {
            None
        }
    })
}

#[allow(dead_code)]
async fn sidecar_metrics() -> Option<SystemMetrics> {
    // Пытаемся получить данные через HTTP запрос к sidecar серверу
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
    {
        Ok(c) => c,
        Err(_e) => {
            return None;
        }
    };

    let response = match client.get("http://localhost:8765/").send().await {
        Ok(r) => r,
        Err(_e) => {
            return None;
        }
    };

    if !response.status().is_success() {
        #[cfg(debug_assertions)]
        return None;
    }

    let json_text = match response.text().await {
        Ok(t) => t,
        Err(_e) => {
            return None;
        }
    };

    let metrics: SystemMetrics = match serde_json::from_str(&json_text) {
        Ok(t) => t,
        Err(_e) => {
            return None;
        }
    };

    Some(metrics)
}

async fn sidecar_temperatures() -> Option<SidecarTemperatures> {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
    {
        Ok(c) => c,
        Err(_) => return None,
    };

    let response = match client.get("http://localhost:8765/").send().await {
        Ok(r) => r,
        Err(_) => return None,
    };

    if !response.status().is_success() {
        return None;
    }

    let json_text = match response.text().await {
        Ok(t) => t,
        Err(_) => return None,
    };

    let mut temps: SidecarTemperatures = match serde_json::from_str(&json_text) {
        Ok(t) => t,
        Err(_) => return None,
    };

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

fn start_sidecar_service() -> Result<(), String> {
    // Безопасная обертка для всех операций
    let result = std::panic::catch_unwind(|| {
        let sidecar_path = std::env::var("LHM_SIDECAR_PATH")
            .map_err(|_| "LHM_SIDECAR_PATH environment variable not set")?;

        let resolved_path = resolve_sidecar_path(&sidecar_path)
            .ok_or_else(|| format!("Failed to resolve sidecar path: {}", sidecar_path))?;

        if !resolved_path.exists() {
            return Err(format!(
                "Sidecar executable not found at: {:?}",
                resolved_path
            ));
        }

        // Проверяем, не запущен ли уже sidecar (простая TCP проверка порта)
        use std::net::{SocketAddr, TcpStream};
        use std::time::Duration as StdDuration;

        // Безопасная проверка порта с таймаутом
        let addr: SocketAddr = "127.0.0.1:8765"
            .parse()
            .map_err(|_| "Failed to parse socket address")?;

        let port_check = TcpStream::connect_timeout(&addr, StdDuration::from_millis(100));

        if port_check.is_ok() {
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
                return Err(format!(
                    "Sidecar process exited immediately with status: {:?}. Stderr: {}",
                    status, stderr
                ));
            }
            Ok(None) => {
                // Процесс работает - это хорошо
            }
            Err(e) => {
                return Err(format!("Failed to check sidecar process status: {}", e));
            }
        }

        // Сохраняем handle процесса
        let mut sidecar_guard = SIDECAR_PROCESS
            .lock()
            .map_err(|e| format!("Failed to lock sidecar process mutex: {}", e))?;
        *sidecar_guard = Some(process);

        // Даем серверу время на запуск
        std::thread::sleep(Duration::from_millis(1000));

        // Проверяем, что сервер действительно запустился (простая TCP проверка)
        let check_addr: SocketAddr = "127.0.0.1:8765"
            .parse()
            .map_err(|_| "Failed to parse socket address")?;
        let _ = TcpStream::connect_timeout(&check_addr, StdDuration::from_millis(500));

        Ok(())
    });

    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(_) => Err("Panic occurred while starting sidecar service".to_string()),
    }
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
    if let Some(temps) = sidecar_temperatures().await {
        if temps.cpu_temperature.is_some() {
            return temps.cpu_temperature;
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(wmi_temp) = wmi_cpu_temperature() {
            return Some(wmi_temp);
        }
        if let Some(sysinfo_temp) = find_temperature(components, &["cpu", "package"]) {
            return Some(sysinfo_temp);
        }
        None
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Some(sysinfo_temp) = find_temperature(components, &["cpu", "package"]) {
            return Some(sysinfo_temp);
        }
        None
    }
}

async fn get_gpu_temperature(components: &Components) -> Option<f32> {
    if let Some(temps) = sidecar_temperatures().await {
        if temps.gpu_temperature.is_some() {
            return temps.gpu_temperature;
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(nvml_temp) = nvml_gpu_temperature() {
            return Some(nvml_temp);
        }
        if let Some(sysinfo_temp) = find_temperature(components, &["gpu", "graphics"]) {
            return Some(sysinfo_temp);
        }
        None
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Some(sysinfo_temp) = find_temperature(components, &["gpu", "graphics"]) {
            return Some(sysinfo_temp);
        }
        None
    }
}

#[tauri::command]
pub async fn get_system_metrics() -> Result<SystemMetrics, String> {
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

    let cpu_temperature = get_cpu_temperature(&components).await;
    let gpu_temperature = get_gpu_temperature(&components).await;

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

pub fn stop_sidecar_service() {
    let mut sidecar_guard = match SIDECAR_PROCESS.lock() {
        Ok(guard) => guard,
        Err(_) => return,
    };

    if let Some(mut child) = sidecar_guard.take() {
        let _ = child.kill();
    }
}

pub fn setup_sidecar_service() {
    let mut starting_guard = match SIDECAR_STARTING.lock() {
        Ok(guard) => guard,
        Err(_) => return,
    };

    if *starting_guard {
        return;
    }

    *starting_guard = true;
    drop(starting_guard);

    std::thread::spawn(|| {
        let result = std::panic::catch_unwind(|| {
            std::thread::sleep(Duration::from_millis(500));
            let _ = start_sidecar_service();

            if let Ok(mut guard) = SIDECAR_STARTING.lock() {
                *guard = false;
            }
        });

        if result.is_err() {
            if let Ok(mut guard) = SIDECAR_STARTING.lock() {
                *guard = false;
            }
        }
    });
}
