//! Live screens: a static background with values (CPU load, temperatures, ...)
//! that the Times Gate keeps up to date by itself, without flicker.
//!
//! `send_live_screen` stores the screen's background (as a 128 × 128 GIF) and
//! its text templates, then sends `Draw/SendHttpItemList` with URLs pointing at
//! a small HTTP server in this process. The device downloads the background
//! once and then polls one URL per value every `UPDATE_SECONDS`; the server
//! answers `{"DispData": "<template with current metrics>"}`. Re-sending a
//! picture every second instead made the screen flash "loading..." on about
//! one update in ten.
//!
//! The device opens connections to this PC, so Windows Firewall must allow
//! inbound connections for the app (the installer adds a rule; in dev Windows
//! asks once). Values stop updating while the app is not running.

use std::collections::HashMap;
use std::net::{IpAddr, UdpSocket};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

use image::DynamicImage;
use serde::Deserialize;
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::divoom_api::send_command_with_timeout;
use crate::divoom_protocol::{self as proto, NetTextItem};
use crate::draw_commands::decode_base64_image;
use crate::system_metrics::{get_system_metrics, SystemMetrics};

/// Port tried first; any free port is used if it is taken.
const PREFERRED_PORT: u16 = 47630;
/// How often the device fetches each value. The author of a similar
/// integration notes that very short intervals can freeze the device's lights.
const UPDATE_SECONDS: u32 = 2;
/// Several values are fetched at once; they share one metrics reading.
const METRICS_MAX_AGE: Duration = Duration::from_millis(1500);
const SEND_TIMEOUT: Duration = Duration::from_secs(2);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_REQUEST_BYTES: usize = 8 * 1024;
const LCD_SIZE: u32 = 128;
/// Shown in place of a value that is not available (e.g. no GPU sensor).
const MISSING_VALUE: &str = "--";

/// A text whose `template` contains placeholders such as `{cpu}`; fields keep
/// their snake_case names on the JS side.
#[derive(Debug, Clone, Deserialize)]
pub struct LiveText {
    pub id: u32,
    pub template: String,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    /// Device font id (see DEVICE_FONTS in src/constants.ts).
    pub font: u32,
    pub color: String,
    /// 1 left, 2 center, 3 right.
    pub align: u32,
}

struct LiveScreen {
    gif: Vec<u8>,
    /// Goes into the background URL so the device downloads a new one.
    version: u64,
    templates: HashMap<u32, String>,
}

type ScreenKey = (String, u32);

fn screens() -> &'static Mutex<HashMap<ScreenKey, LiveScreen>> {
    static SCREENS: OnceLock<Mutex<HashMap<ScreenKey, LiveScreen>>> = OnceLock::new();
    SCREENS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

/// Called when a screen gets a static picture: its live values are no longer served.
pub fn forget(ip_address: &str, screen_index: u32) {
    lock(screens()).remove(&(ip_address.to_string(), screen_index));
}

#[tauri::command]
pub async fn send_live_screen(
    ip_address: String,
    screen_index: u32,
    background: String,
    texts: Vec<LiveText>,
) -> Result<(), String> {
    let gif = encode_gif(decode_base64_image(&background)?)?;
    let port = server_port().await?;
    let host = local_ip_towards(&ip_address)?;

    let version = {
        let mut screens = lock(screens());
        let key = (ip_address.clone(), screen_index);
        let version = screens.get(&key).map_or(1, |s| s.version + 1);
        let templates = texts.iter().map(|t| (t.id, t.template.clone())).collect();
        screens.insert(key, LiveScreen { gif, version, templates });
        version
    };

    let base = format!("http://{}:{}", host, port);
    let items: Vec<NetTextItem> = texts
        .iter()
        .map(|t| NetTextItem {
            text_id: t.id,
            x: t.x.min(LCD_SIZE - 1),
            y: t.y.min(LCD_SIZE - 1),
            width: t.width.clamp(1, LCD_SIZE),
            font: t.font,
            color: t.color.clone(),
            align: t.align,
            url: format!("{}/text/{}/{}/{}", base, ip_address, screen_index, t.id),
            update_seconds: UPDATE_SECONDS,
        })
        .collect();
    let background_url = format!("{}/bg/{}/{}/{}.gif", base, ip_address, screen_index, version);

    send_command_with_timeout(
        &ip_address,
        &proto::send_http_item_list(screen_index, &background_url, &items),
        SEND_TIMEOUT,
    )
    .await
    .map(|_| ())
    .map_err(|e| format!("Failed to send live screen: {}", e))
}

fn encode_gif(img: DynamicImage) -> Result<Vec<u8>, String> {
    let rgba = img
        .resize_exact(LCD_SIZE, LCD_SIZE, image::imageops::FilterType::Lanczos3)
        .to_rgba8();
    let mut buffer = Vec::new();
    image::codecs::gif::GifEncoder::new(&mut buffer)
        .encode(rgba.as_raw(), LCD_SIZE, LCD_SIZE, image::ColorType::Rgba8)
        .map_err(|e| format!("Failed to encode background: {}", e))?;
    Ok(buffer)
}

/// This PC's address on the interface that routes to the device. Connecting
/// a UDP socket sends nothing; it only picks the route.
fn local_ip_towards(device_ip: &str) -> Result<IpAddr, String> {
    let socket = UdpSocket::bind(("0.0.0.0", 0))
        .map_err(|e| format!("Failed to find this PC's address: {}", e))?;
    socket
        .connect((device_ip, 80))
        .map_err(|e| format!("Failed to find this PC's address: {}", e))?;
    socket
        .local_addr()
        .map(|addr| addr.ip())
        .map_err(|e| format!("Failed to find this PC's address: {}", e))
}

// ---------------------------------------------------------------------------
// HTTP server
// ---------------------------------------------------------------------------

async fn server_port() -> Result<u16, String> {
    static PORT: tokio::sync::OnceCell<u16> = tokio::sync::OnceCell::const_new();
    PORT.get_or_try_init(|| async {
        let listener = match TcpListener::bind(("0.0.0.0", PREFERRED_PORT)).await {
            Ok(listener) => listener,
            Err(_) => TcpListener::bind(("0.0.0.0", 0))
                .await
                .map_err(|e| format!("Failed to start live screen server: {}", e))?,
        };
        let port = listener
            .local_addr()
            .map_err(|e| format!("Failed to start live screen server: {}", e))?
            .port();
        tauri::async_runtime::spawn(serve(listener));
        Ok(port)
    })
    .await
    .copied()
}

async fn serve(listener: TcpListener) {
    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                tauri::async_runtime::spawn(async move {
                    let _ = handle(stream, peer.ip()).await;
                });
            }
            // e.g. too many open files: don't spin.
            Err(_) => tokio::time::sleep(Duration::from_millis(200)).await,
        }
    }
}

async fn handle(mut stream: TcpStream, peer: IpAddr) -> std::io::Result<()> {
    let mut request = Vec::with_capacity(512);
    let read = tokio::time::timeout(REQUEST_TIMEOUT, async {
        let mut chunk = [0u8; 1024];
        loop {
            let n = stream.read(&mut chunk).await?;
            if n == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..n]);
            if request.windows(4).any(|w| w == b"\r\n\r\n") || request.len() > MAX_REQUEST_BYTES {
                break;
            }
        }
        Ok::<_, std::io::Error>(())
    })
    .await;
    if !matches!(read, Ok(Ok(()))) {
        return Ok(());
    }

    let response = match request_path(&String::from_utf8_lossy(&request)) {
        Some(path) => respond(path, peer).await,
        None => Response::status(400, "Bad Request"),
    };
    stream.write_all(&response.into_bytes()).await?;
    stream.shutdown().await
}

struct Response {
    status: u16,
    reason: &'static str,
    content_type: &'static str,
    body: Vec<u8>,
}

impl Response {
    fn status(status: u16, reason: &'static str) -> Self {
        Response { status, reason, content_type: "text/plain", body: Vec::new() }
    }

    fn into_bytes(self) -> Vec<u8> {
        let mut bytes = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
            self.status,
            self.reason,
            self.content_type,
            self.body.len()
        )
        .into_bytes();
        bytes.extend(self.body);
        bytes
    }
}

async fn respond(path: &str, peer: IpAddr) -> Response {
    let Some(route) = parse_route(path) else {
        return Response::status(404, "Not Found");
    };
    // Each device may read only its own screens (plus this PC, for debugging).
    if !peer.is_loopback() && peer.to_string() != route.ip() {
        return Response::status(403, "Forbidden");
    }

    match route {
        Route::Background { ip, screen } => {
            let gif = lock(screens()).get(&(ip, screen)).map(|s| s.gif.clone());
            match gif {
                Some(body) => Response { status: 200, reason: "OK", content_type: "image/gif", body },
                None => Response::status(404, "Not Found"),
            }
        }
        Route::Text { ip, screen, id } => {
            let template = lock(screens())
                .get(&(ip, screen))
                .and_then(|s| s.templates.get(&id).cloned());
            let Some(template) = template else {
                return Response::status(404, "Not Found");
            };
            let metrics = cached_metrics().await;
            let text = render_template(&template, metrics.as_ref());
            Response {
                status: 200,
                reason: "OK",
                content_type: "application/json",
                body: json!({ "DispData": text }).to_string().into_bytes(),
            }
        }
    }
}

async fn cached_metrics() -> Option<SystemMetrics> {
    static CACHE: tokio::sync::Mutex<Option<(Instant, SystemMetrics)>> =
        tokio::sync::Mutex::const_new(None);
    let mut cache = CACHE.lock().await;
    if let Some((at, metrics)) = cache.as_ref() {
        if at.elapsed() < METRICS_MAX_AGE {
            return Some(metrics.clone());
        }
    }
    let metrics = get_system_metrics().await.ok()?;
    *cache = Some((Instant::now(), metrics.clone()));
    Some(metrics)
}

/// Path of a `GET` request line, without the query string.
fn request_path(request: &str) -> Option<&str> {
    let mut parts = request.lines().next()?.split_whitespace();
    if parts.next()? != "GET" {
        return None;
    }
    let target = parts.next()?;
    Some(target.split('?').next().unwrap_or(target))
}

#[derive(Debug, PartialEq)]
enum Route {
    /// `/bg/<ip>/<screen>/<version>.gif`
    Background { ip: String, screen: u32 },
    /// `/text/<ip>/<screen>/<text id>`
    Text { ip: String, screen: u32, id: u32 },
}

impl Route {
    fn ip(&self) -> &str {
        match self {
            Route::Background { ip, .. } | Route::Text { ip, .. } => ip,
        }
    }
}

fn parse_route(path: &str) -> Option<Route> {
    let segments: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    match segments.as_slice() {
        ["bg", ip, screen, file] if file.ends_with(".gif") => Some(Route::Background {
            ip: ip.to_string(),
            screen: screen.parse().ok()?,
        }),
        ["text", ip, screen, id] => Some(Route::Text {
            ip: ip.to_string(),
            screen: screen.parse().ok()?,
            id: id.parse().ok()?,
        }),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Templates
// ---------------------------------------------------------------------------

/// Replaces placeholders such as `{cpu}` with current values. Keys must match
/// LIVE_VALUES in src/constants.ts. Unknown `{...}` is kept as typed.
pub fn render_template(template: &str, metrics: Option<&SystemMetrics>) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('}') {
            Some(end) => match placeholder_value(&after[..end], metrics) {
                Some(value) => {
                    out.push_str(&value.unwrap_or_else(|| MISSING_VALUE.to_string()));
                    rest = &after[end + 1..];
                }
                None => {
                    out.push('{');
                    rest = after;
                }
            },
            None => {
                out.push_str(&rest[start..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// `None` for an unknown key; `Some(None)` when the value is unavailable.
fn placeholder_value(key: &str, metrics: Option<&SystemMetrics>) -> Option<Option<String>> {
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    let whole = |v: f32| format!("{:.0}", v);
    let value = |f: fn(&SystemMetrics) -> Option<String>| metrics.and_then(f);

    Some(match key {
        "cpu" => value(|m| Some(format!("{:.0}", m.cpu_usage))),
        "cpu_temp" => metrics.and_then(|m| m.cpu_temperature).map(whole),
        "gpu" => metrics.and_then(|m| m.gpu_usage).map(whole),
        "gpu_temp" => metrics.and_then(|m| m.gpu_temperature).map(whole),
        "ram" => value(|m| {
            (m.memory_total > 0)
                .then(|| format!("{:.0}", m.memory_used as f64 / m.memory_total as f64 * 100.0))
        }),
        "ram_used" => value(|m| Some(format!("{:.1}", m.memory_used as f64 / GIB))),
        "ram_total" => value(|m| Some(format!("{:.1}", m.memory_total as f64 / GIB))),
        "disk" => value(|m| {
            // The system disk if there is one, else the first.
            m.disks
                .iter()
                .find(|d| d.mount_point.to_ascii_uppercase().starts_with("C:"))
                .or_else(|| m.disks.first())
                .map(|d| format!("{:.0}", d.usage_percent))
        }),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system_metrics::DiskUsage;

    fn metrics() -> SystemMetrics {
        let gib = 1024 * 1024 * 1024;
        SystemMetrics {
            cpu_usage: 41.6,
            cpu_temperature: Some(55.2),
            gpu_usage: None,
            gpu_temperature: Some(61.0),
            memory_total: 16 * gib,
            memory_used: 4 * gib,
            disks: vec![
                DiskUsage {
                    name: "Data".into(),
                    mount_point: "D:\\".into(),
                    total_space: 100,
                    available_space: 90,
                    used_space: 10,
                    usage_percent: 10.0,
                },
                DiskUsage {
                    name: "System".into(),
                    mount_point: "C:\\".into(),
                    total_space: 100,
                    available_space: 25,
                    used_space: 75,
                    usage_percent: 75.0,
                },
            ],
        }
    }

    #[test]
    fn renders_known_placeholders() {
        let m = metrics();
        assert_eq!(render_template("CPU {cpu}% {cpu_temp}C", Some(&m)), "CPU 42% 55C");
        assert_eq!(render_template("{ram}% {ram_used}/{ram_total}G", Some(&m)), "25% 4.0/16.0G");
        assert_eq!(render_template("GPU {gpu_temp}°", Some(&m)), "GPU 61°");
        assert_eq!(render_template("C: {disk}%", Some(&m)), "C: 75%");
    }

    #[test]
    fn missing_values_show_dashes() {
        assert_eq!(render_template("GPU {gpu}%", Some(&metrics())), "GPU --%");
        assert_eq!(render_template("{cpu}/{ram}", None), "--/--");
    }

    #[test]
    fn unknown_or_unclosed_braces_are_kept() {
        let m = metrics();
        assert_eq!(render_template("{foo} {cpu}", Some(&m)), "{foo} 42");
        assert_eq!(render_template("a {cpu", Some(&m)), "a {cpu");
        assert_eq!(render_template("{{cpu}}", Some(&m)), "{42}");
        assert_eq!(render_template("plain", Some(&m)), "plain");
    }

    #[test]
    fn parses_get_paths_and_routes() {
        assert_eq!(
            request_path("GET /text/192.168.1.5/2/7?x=1 HTTP/1.1\r\nHost: a\r\n\r\n"),
            Some("/text/192.168.1.5/2/7")
        );
        assert_eq!(request_path("POST / HTTP/1.1\r\n\r\n"), None);
        assert_eq!(
            parse_route("/bg/192.168.1.5/0/3.gif"),
            Some(Route::Background { ip: "192.168.1.5".into(), screen: 0 })
        );
        assert_eq!(
            parse_route("/text/192.168.1.5/4/12"),
            Some(Route::Text { ip: "192.168.1.5".into(), screen: 4, id: 12 })
        );
        assert_eq!(parse_route("/bg/192.168.1.5/0/3.png"), None);
        assert_eq!(parse_route("/text/192.168.1.5/x/1"), None);
        assert_eq!(parse_route("/"), None);
    }

    #[tokio::test]
    async fn serves_only_the_device_that_owns_the_screen() {
        let ip = "10.9.8.7";
        lock(screens()).insert(
            (ip.to_string(), 1),
            LiveScreen { gif: vec![1, 2, 3], version: 1, templates: HashMap::new() },
        );

        let own = respond("/bg/10.9.8.7/1/1.gif", ip.parse().unwrap()).await;
        assert_eq!((own.status, own.body), (200, vec![1, 2, 3]));

        let other = respond("/bg/10.9.8.7/1/1.gif", "10.9.8.6".parse().unwrap()).await;
        assert_eq!(other.status, 403);

        forget(ip, 1);
        let gone = respond("/bg/10.9.8.7/1/1.gif", ip.parse().unwrap()).await;
        assert_eq!(gone.status, 404);
    }

    /// Manual check on a real Times Gate: sends a live screen to LCD 0 and keeps
    /// serving it for `DIVOOM_TEST_SECONDS` (default 40) while you look at it.
    /// Reads `DIVOOM_LOCAL_TOKEN` from the environment or `.env`.
    ///
    /// DIVOOM_TEST_IP=192.168.1.147 cargo test live_screen_on_device -- --ignored --nocapture
    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn live_screen_on_device() {
        use base64::{engine::general_purpose, Engine as _};

        dotenvy::dotenv().ok();
        let ip = std::env::var("DIVOOM_TEST_IP").expect("set DIVOOM_TEST_IP");
        let token = std::env::var("DIVOOM_LOCAL_TOKEN").ok().and_then(|t| t.trim().parse().ok());
        crate::divoom_api::set_local_token(&ip, token);
        let seconds = std::env::var("DIVOOM_TEST_SECONDS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(40);

        let mut bg = image::RgbImage::from_pixel(128, 128, image::Rgb([10, 30, 80]));
        for y in 96..128 {
            for x in 0..128 {
                bg.put_pixel(x, y, image::Rgb([40, 70, 70]));
            }
        }
        let mut png = Vec::new();
        DynamicImage::ImageRgb8(bg)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageOutputFormat::Png)
            .unwrap();

        let text = |id, template: &str, y, font, color: &str| LiveText {
            id,
            template: template.into(),
            x: 6,
            y,
            width: 116,
            font,
            color: color.into(),
            align: 1,
        };
        send_live_screen(
            ip,
            0,
            general_purpose::STANDARD.encode(png),
            vec![
                text(1, "CPU {cpu}% {cpu_temp}C", 16, 4, "#00FF00"),
                text(2, "RAM {ram}% {ram_used}G", 48, 32, "#FFFFFF"),
                text(3, "GPU {gpu}% {gpu_temp}C", 104, 4, "#FFFF00"),
            ],
        )
        .await
        .unwrap();
        println!("live screen sent, serving for {} s", seconds);
        tokio::time::sleep(Duration::from_secs(seconds)).await;
    }

    #[test]
    fn background_is_a_128px_gif() {
        let gif = encode_gif(DynamicImage::new_rgb8(64, 32)).unwrap();
        assert_eq!(&gif[..6], b"GIF89a");
        assert_eq!(u16::from_le_bytes([gif[6], gif[7]]), 128);
        assert_eq!(u16::from_le_bytes([gif[8], gif[9]]), 128);
    }
}
