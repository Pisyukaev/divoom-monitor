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
    #[serde(default)]
    pub gpu_usage: Option<f32>,
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

async fn sidecar_metrics() -> Option<SystemMetrics> {
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

    let metrics: SystemMetrics = match serde_json::from_str(&json_text) {
        Ok(t) => t,
        Err(_) => return None,
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

const SIDECAR_EXE_NAME: &str = "HardwareMonitorCli.exe";

fn find_sidecar_path() -> Result<PathBuf, String> {
    if let Ok(env_path) = std::env::var("LHM_SIDECAR_PATH") {
        let path = Path::new(&env_path);

        if path.is_absolute() && path.exists() {
            return Ok(path.to_path_buf());
        }

        let cwd_resolved = std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .ok();
        if let Some(ref p) = cwd_resolved {
            if p.exists() {
                return Ok(p.clone());
            }
        }

        if let Some(exe_resolved) = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(|d| d.join(path)))
        {
            if exe_resolved.exists() {
                return Ok(exe_resolved);
            }
        }

        return Err(format!(
            "LHM_SIDECAR_PATH='{}' set but file not found (tried CWD: {:?})",
            env_path, cwd_resolved
        ));
    }

    if let Some(exe_dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.to_path_buf())) {
        let next_to_exe = exe_dir.join(SIDECAR_EXE_NAME);
        if next_to_exe.exists() {
            return Ok(next_to_exe);
        }

        let in_sidecar_dir = exe_dir.join("sidecar").join(SIDECAR_EXE_NAME);
        if in_sidecar_dir.exists() {
            return Ok(in_sidecar_dir);
        }
    }

    Err(format!(
        "Sidecar '{}' not found. Set LHM_SIDECAR_PATH or run `pnpm build:sidecar`",
        SIDECAR_EXE_NAME
    ))
}

fn start_sidecar_service() -> Result<(), String> {
    let result = std::panic::catch_unwind(|| {
        let resolved_path = find_sidecar_path()?;

        use std::net::{SocketAddr, TcpStream};
        use std::time::Duration as StdDuration;

        let addr: SocketAddr = "127.0.0.1:8765"
            .parse()
            .map_err(|_| "Failed to parse socket address")?;

        if TcpStream::connect_timeout(&addr, StdDuration::from_millis(100)).is_ok() {
            eprintln!("[Sidecar] Already running on port 8765");
            return Ok(());
        }

        #[cfg(target_os = "windows")]
        {
            let path_str = resolved_path.to_string_lossy().to_string();
            let working_dir = resolved_path
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;

            let mut process = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-WindowStyle",
                    "Hidden",
                    "-Command",
                    &format!(
                        "Start-Process -FilePath '{}' -WorkingDirectory '{}' -Verb RunAs -WindowStyle Hidden",
                        path_str, working_dir
                    ),
                ])
                .creation_flags(CREATE_NO_WINDOW)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| format!("Failed to launch elevated sidecar: {}", e))?;

            let exit_status = process
                .wait()
                .map_err(|e| format!("Failed to wait for elevation launcher: {}", e))?;

            if !exit_status.success() {
                let mut stderr_output = String::new();
                if let Some(mut child_stderr) = process.stderr.take() {
                    use std::io::Read;
                    let _ = child_stderr.read_to_string(&mut stderr_output);
                }
                return Err(format!(
                    "Elevated launch failed (status: {:?}). Stderr: {}",
                    exit_status, stderr_output
                ));
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let mut process = Command::new(&resolved_path)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| format!("Failed to start sidecar process: {}", e))?;

            match process.try_wait() {
                Ok(Some(status)) => {
                    let mut stderr_output = String::new();
                    if let Some(mut child_stderr) = process.stderr.take() {
                        use std::io::Read;
                        let _ = child_stderr.read_to_string(&mut stderr_output);
                    }
                    return Err(format!(
                        "Sidecar exited immediately (status: {:?}). Stderr: {}",
                        status, stderr_output
                    ));
                }
                Ok(None) => {}
                Err(e) => {
                    return Err(format!("Failed to check sidecar process status: {}", e));
                }
            }

            let mut sidecar_guard = SIDECAR_PROCESS
                .lock()
                .map_err(|e| format!("Failed to lock sidecar mutex: {}", e))?;
            *sidecar_guard = Some(process);
        }

        // Wait for server to become available (up to 5 seconds)
        for i in 0..10 {
            std::thread::sleep(Duration::from_millis(500));
            if TcpStream::connect_timeout(&addr, StdDuration::from_millis(200)).is_ok() {
                eprintln!("[Sidecar] Started successfully after {}ms", (i + 1) * 500);
                return Ok(());
            }
        }

        Err("Sidecar started but did not respond on port 8765 within 5 seconds".to_string())
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
fn nvml_gpu_usage() -> Option<f32> {
    let nvml = nvml_wrapper::Nvml::init().ok()?;
    let device_count = nvml.device_count().ok()?;
    let mut best_usage = None;

    for index in 0..device_count {
        let device = nvml.device_by_index(index).ok()?;
        if let Ok(utilization) = device.utilization_rates() {
            let gpu_pct = utilization.gpu as f32;
            best_usage = Some(best_usage.map_or(gpu_pct, |current: f32| current.max(gpu_pct)));
        }
    }

    best_usage
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
    if let Some(mut metrics) = sidecar_metrics().await {
        if metrics.gpu_usage.is_none() {
            #[cfg(target_os = "windows")]
            {
                metrics.gpu_usage = nvml_gpu_usage();
            }
        }
        return Ok(metrics);
    }

    // Fallback на sysinfo, если sidecar недоступен
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

    #[cfg(target_os = "windows")]
    let gpu_usage = nvml_gpu_usage();
    #[cfg(not(target_os = "windows"))]
    let gpu_usage: Option<f32> = None;

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
        gpu_usage,
        gpu_temperature,
        memory_total: system.total_memory(),
        memory_used: system.used_memory(),
        disks,
    })
}

pub fn stop_sidecar_service() {
    eprintln!("[Sidecar] Stopping service...");

    // Try graceful HTTP shutdown first (works regardless of privilege level)
    let shutdown_ok = std::panic::catch_unwind(|| {
        use std::io::{Read, Write};
        use std::net::{SocketAddr, TcpStream};
        use std::time::Duration as StdDuration;

        let addr: SocketAddr = match "127.0.0.1:8765".parse() {
            Ok(a) => a,
            Err(_) => return false,
        };
        let mut stream =
            match TcpStream::connect_timeout(&addr, StdDuration::from_millis(500)) {
                Ok(s) => s,
                Err(_) => return false,
            };
        let _ = stream.set_read_timeout(Some(StdDuration::from_secs(2)));
        let _ = stream.set_write_timeout(Some(StdDuration::from_secs(2)));

        let request = "GET /shutdown HTTP/1.1\r\nHost: localhost:8765\r\nConnection: close\r\n\r\n";
        if stream.write_all(request.as_bytes()).is_err() {
            return false;
        }

        let mut buf = [0u8; 256];
        let _ = stream.read(&mut buf);
        eprintln!("[Sidecar] Graceful shutdown request sent");
        true
    })
    .unwrap_or(false);

    // Kill the child handle if we have one (non-elevated launch)
    if let Ok(mut guard) = SIDECAR_PROCESS.lock() {
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            eprintln!("[Sidecar] Child process killed");
            return;
        }
    }

    // If HTTP shutdown didn't work, force-kill by process name (Windows)
    if !shutdown_ok {
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;

            let _ = Command::new("taskkill")
                .args(["/F", "/IM", "HardwareMonitorCli.exe"])
                .creation_flags(CREATE_NO_WINDOW)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .and_then(|mut c| c.wait());
            eprintln!("[Sidecar] Force-killed via taskkill");
        }
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
            match start_sidecar_service() {
                Ok(()) => eprintln!("[Sidecar] Service is ready"),
                Err(e) => eprintln!("[Sidecar] Failed to start: {}", e),
            }

            if let Ok(mut guard) = SIDECAR_STARTING.lock() {
                *guard = false;
            }
        });

        if result.is_err() {
            eprintln!("[Sidecar] Panic during startup");
            if let Ok(mut guard) = SIDECAR_STARTING.lock() {
                *guard = false;
            }
        }
    });
}
