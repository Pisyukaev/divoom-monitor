use base64::{engine::general_purpose, Engine as _};
use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageEncoder};
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use crate::divoom_api::{
    discover_devices_at, get_5lcd_info_at, send_command, send_command_with_timeout,
};
use crate::divoom_protocol::{self as proto, CLOUD_BASE_URL};
use crate::models::LcdInfoResponse;

/// Times Gate LCD side, in pixels.
const LCD_SIZE: u32 = 128;
const IMAGE_UPLOAD_TIMEOUT: Duration = Duration::from_secs(1);

// The docs require PicID to grow with every animation sent.
static PIC_ID_COUNTER: AtomicU32 = AtomicU32::new(1000);

fn get_next_pic_id() -> u32 {
    PIC_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Resizes to exactly `width`x`height` and encodes as JPEG (RGB, as the docs expect).
fn resize_image(img: DynamicImage, width: u32, height: u32) -> Result<Vec<u8>, String> {
    let rgb = img
        .resize_exact(width, height, image::imageops::FilterType::Lanczos3)
        .to_rgb8();
    let mut buffer = Vec::new();
    JpegEncoder::new(&mut buffer)
        .write_image(rgb.as_raw(), rgb.width(), rgb.height(), image::ColorType::Rgb8)
        .map_err(|e| format!("Failed to encode image: {}", e))?;
    Ok(buffer)
}

async fn load_image_from_url(url: &str) -> Result<DynamicImage, String> {
    let bytes = download_image(url).await?;
    image::load_from_memory(&bytes).map_err(|e| format!("Failed to decode image: {}", e))
}

async fn download_image(url: &str) -> Result<Vec<u8>, String> {
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

    Ok(bytes.to_vec())
}

/// Strips an optional `data:<mime>;base64,` prefix and decodes.
fn decode_base64_image(data: &str) -> Result<DynamicImage, String> {
    let b64 = data.split_once(";base64,").map_or(data, |(_, b64)| b64);
    let bytes = general_purpose::STANDARD
        .decode(b64.trim())
        .map_err(|e| format!("Invalid image data: {}", e))?;
    image::load_from_memory(&bytes).map_err(|e| format!("Failed to decode image: {}", e))
}

fn load_image_from_file(file_path: &str) -> Result<DynamicImage, String> {
    image::open(Path::new(file_path)).map_err(|e| format!("Failed to open image file: {}", e))
}

async fn send_image(ip_address: &str, screen_index: u32, img: DynamicImage) -> Result<(), String> {
    let jpeg = resize_image(img, LCD_SIZE, LCD_SIZE)?;
    let payload = proto::send_http_gif(
        screen_index,
        get_next_pic_id(),
        LCD_SIZE,
        &general_purpose::STANDARD.encode(&jpeg),
    );

    let result = send_command_with_timeout(ip_address, &payload, IMAGE_UPLOAD_TIMEOUT)
        .await
        .map(|_| ())
        .map_err(|e| format!("Failed to send image command: {}", e));

    #[cfg(all(debug_assertions, not(test)))]
    dump_sent_image(ip_address, screen_index, &jpeg, &result);

    result
}

/// Dev builds only: keeps the exact JPEG sent to each screen, plus a log line
/// with the device's answer, so a webcam photo of the device can be compared
/// with what was actually sent. Files: `<temp>/divoom-monitor/sent/`.
#[cfg(all(debug_assertions, not(test)))]
fn dump_sent_image(ip_address: &str, screen_index: u32, jpeg: &[u8], result: &Result<(), String>) {
    use std::io::Write;

    let dir = std::env::temp_dir().join("divoom-monitor").join("sent");
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let _ = std::fs::write(dir.join(format!("screen{}.jpg", screen_index)), jpeg);

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let status = match result {
        Ok(()) => "ok".to_string(),
        Err(e) => format!("error: {}", e),
    };
    if let Ok(mut log) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("log.txt"))
    {
        let _ = writeln!(log, "{} ip={} screen={} {}", ts, ip_address, screen_index, status);
    }
}

#[tauri::command]
pub async fn upload_image_from_url(
    ip_address: String,
    screen_index: u32,
    url: String,
) -> Result<(), String> {
    let img = load_image_from_url(&url).await?;
    send_image(&ip_address, screen_index, img).await
}

#[tauri::command]
pub async fn upload_image_from_file(
    ip_address: String,
    screen_index: u32,
    file_path: String,
) -> Result<(), String> {
    let img = load_image_from_file(&file_path)?;
    send_image(&ip_address, screen_index, img).await
}

/// Sends a screen the frontend composed itself (image + texts drawn on a
/// canvas). `data` is base64, optionally as a `data:` URL.
#[tauri::command]
pub async fn upload_image_data(
    ip_address: String,
    screen_index: u32,
    data: String,
) -> Result<(), String> {
    let img = decode_base64_image(&data)?;
    send_image(&ip_address, screen_index, img).await
}

/// Downloads an image for the frontend canvas as a `data:` URL. Drawing a
/// cross-origin image directly would taint the canvas and block export.
#[tauri::command]
pub async fn fetch_image_base64(url: String) -> Result<String, String> {
    let bytes = download_image(&url).await?;
    let format = image::guess_format(&bytes)
        .map_err(|e| format!("Failed to decode image: {}", e))?;
    let mime = match format {
        image::ImageFormat::Png => "image/png",
        image::ImageFormat::Jpeg => "image/jpeg",
        image::ImageFormat::Gif => "image/gif",
        image::ImageFormat::WebP => "image/webp",
        image::ImageFormat::Bmp => "image/bmp",
        _ => return Err(format!("Unsupported image format: {:?}", format)),
    };
    Ok(format!(
        "data:{};base64,{}",
        mime,
        general_purpose::STANDARD.encode(&bytes)
    ))
}

#[tauri::command]
pub async fn get_lcd_info(ip_address: String) -> Result<LcdInfoResponse, String> {
    get_lcd_info_from(CLOUD_BASE_URL, &ip_address).await
}

/// Looks the device up in the cloud by IP (to get its DeviceId), then fetches its channel info.
async fn get_lcd_info_from(
    cloud_base_url: &str,
    ip_address: &str,
) -> Result<LcdInfoResponse, String> {
    let devices = discover_devices_at(cloud_base_url).await?;

    let device_id = devices
        .iter()
        .find(|d| d.ip_address.as_deref() == Some(ip_address))
        .ok_or_else(|| format!("Device with IP {} not found", ip_address))?
        .device_id
        .ok_or("Device has no ID")?;

    get_5lcd_info_at(cloud_base_url, device_id).await
}

#[tauri::command]
pub async fn activate_pc_monitor(
    ip_address: String,
    device_id: u64,
    lcd_independence: u64,
    lcd_index: u32,
) -> Result<(), String> {
    let payload = proto::set_clock_select_id(
        device_id,
        lcd_independence,
        lcd_index,
        proto::PC_MONITOR_CLOCK_ID,
    );
    send_command(&ip_address, &payload)
        .await
        .map_err(|e| format!("Failed to activate PC monitor: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn send_pc_metrics(
    ip_address: String,
    lcd_index: u32,
    disp_data: Vec<String>,
) -> Result<(), String> {
    send_command(&ip_address, &proto::update_pc_para_info(lcd_index, &disp_data))
        .await
        .map_err(|e| format!("Failed to send PC metrics: {}", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::divoom_api::tests::host;
    use serde_json::{json, Value};
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn ok_device() -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/post"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "error_code": 0 })))
            .mount(&server)
            .await;
        server
    }

    async fn sent_bodies(server: &MockServer) -> Vec<Value> {
        server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .filter(|r| r.url.path() == "/post")
            .map(|r| serde_json::from_slice(&r.body).unwrap())
            .collect()
    }

    fn write_test_png(name: &str, w: u32, h: u32) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "divoom-test-{}-{}.png",
            std::process::id(),
            name
        ));
        image::RgbaImage::from_pixel(w, h, image::Rgba([255, 0, 0, 255]))
            .save(&path)
            .unwrap();
        path
    }

    #[test]
    fn resize_image_produces_exact_size_jpeg() {
        let img = DynamicImage::new_rgba8(300, 50);
        let jpeg = resize_image(img, 128, 128).unwrap();
        assert_eq!(&jpeg[..2], &[0xFF, 0xD8], "JPEG SOI marker");
        let decoded = image::load_from_memory(&jpeg).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (128, 128));
    }

    #[test]
    fn pic_ids_increase() {
        let a = get_next_pic_id();
        let b = get_next_pic_id();
        assert!(b > a);
    }

    #[tokio::test]
    async fn upload_image_from_file_sends_send_http_gif() {
        let server = ok_device().await;
        let png = write_test_png("upload", 64, 32);

        upload_image_from_file(host(&server), 3, png.to_string_lossy().into())
            .await
            .unwrap();
        let _ = std::fs::remove_file(&png);

        let bodies = sent_bodies(&server).await;
        assert_eq!(bodies.len(), 1);
        let body = &bodies[0];
        assert_eq!(body["Command"], "Draw/SendHttpGif");
        assert_eq!(body["LcdArray"], json!([0, 0, 0, 1, 0]));
        assert_eq!(body["PicNum"], 1);
        assert_eq!(body["PicOffset"], 0);
        assert_eq!(body["PicWidth"], 128);
        assert!(body["PicID"].as_u64().unwrap() >= 1000);

        let jpeg = general_purpose::STANDARD
            .decode(body["PicData"].as_str().unwrap())
            .unwrap();
        let decoded = image::load_from_memory(&jpeg).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (128, 128));
    }

    #[tokio::test]
    async fn upload_image_from_url_downloads_then_sends() {
        let server = ok_device().await;
        let png = write_test_png("url", 16, 16);
        Mock::given(method("GET"))
            .and(path("/img.png"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(std::fs::read(&png).unwrap()))
            .mount(&server)
            .await;
        let _ = std::fs::remove_file(&png);

        upload_image_from_url(host(&server), 0, format!("{}/img.png", server.uri()))
            .await
            .unwrap();

        let bodies = sent_bodies(&server).await;
        assert_eq!(bodies.len(), 1);
        assert_eq!(bodies[0]["LcdArray"], json!([1, 0, 0, 0, 0]));
    }

    #[tokio::test]
    async fn upload_image_from_missing_file_fails_without_request() {
        let server = ok_device().await;
        let err = upload_image_from_file(host(&server), 0, "Z:/no/such/file.png".into())
            .await
            .unwrap_err();
        assert!(err.contains("Failed to open image file"), "{err}");
        assert!(sent_bodies(&server).await.is_empty());
    }

    #[tokio::test]
    async fn upload_image_data_sends_composed_screen() {
        let server = ok_device().await;
        let data_url = format!(
            "data:image/png;base64,{}",
            general_purpose::STANDARD.encode(png_bytes(200, 100, [255, 0, 0]))
        );

        upload_image_data(host(&server), 4, data_url).await.unwrap();

        let bodies = sent_bodies(&server).await;
        assert_eq!(bodies.len(), 1);
        assert_eq!(bodies[0]["Command"], "Draw/SendHttpGif");
        assert_eq!(bodies[0]["LcdArray"], json!([0, 0, 0, 0, 1]));
        let jpeg = general_purpose::STANDARD
            .decode(bodies[0]["PicData"].as_str().unwrap())
            .unwrap();
        let decoded = image::load_from_memory(&jpeg).unwrap().to_rgb8();
        assert_eq!(decoded.dimensions(), (128, 128));
        let px = decoded.get_pixel(64, 64);
        assert!(px[0] > 200 && px[1] < 60 && px[2] < 60, "still red: {px:?}");
    }

    #[tokio::test]
    async fn upload_image_data_accepts_plain_base64() {
        let server = ok_device().await;
        let b64 = general_purpose::STANDARD.encode(png_bytes(8, 8, [0, 0, 255]));
        upload_image_data(host(&server), 0, b64).await.unwrap();
        assert_eq!(sent_bodies(&server).await.len(), 1);
    }

    #[tokio::test]
    async fn upload_image_data_rejects_bad_data_without_request() {
        let server = ok_device().await;
        let err = upload_image_data(host(&server), 0, "not base64!".into())
            .await
            .unwrap_err();
        assert!(err.contains("Invalid image data"), "{err}");
        let err = upload_image_data(host(&server), 0, general_purpose::STANDARD.encode("text"))
            .await
            .unwrap_err();
        assert!(err.contains("Failed to decode image"), "{err}");
        assert!(sent_bodies(&server).await.is_empty());
    }

    #[tokio::test]
    async fn fetch_image_base64_returns_data_url() {
        let server = MockServer::start().await;
        let png = png_bytes(4, 4, [0, 255, 0]);
        Mock::given(method("GET"))
            .and(path("/pic"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(png.clone()))
            .mount(&server)
            .await;

        let url = fetch_image_base64(format!("{}/pic", server.uri())).await.unwrap();
        let b64 = url.strip_prefix("data:image/png;base64,").expect(&url);
        assert_eq!(general_purpose::STANDARD.decode(b64).unwrap(), png);
    }

    #[tokio::test]
    async fn fetch_image_base64_rejects_non_images() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("<html>"))
            .mount(&server)
            .await;
        assert!(fetch_image_base64(server.uri()).await.is_err());
    }

    fn png_bytes(w: u32, h: u32, rgb: [u8; 3]) -> Vec<u8> {
        let img = image::RgbImage::from_pixel(w, h, image::Rgb(rgb));
        let mut out = std::io::Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageOutputFormat::Png).unwrap();
        out.into_inner()
    }

    #[tokio::test]
    async fn activate_pc_monitor_selects_clock_625() {
        let server = ok_device().await;
        activate_pc_monitor(host(&server), 300038420, 978, 4)
            .await
            .unwrap();

        let body = &sent_bodies(&server).await[0];
        assert_eq!(body["Command"], "Channel/SetClockSelectId");
        assert_eq!(body["ClockId"], 625);
        assert_eq!(body["LcdIndex"], 4);
        assert_eq!(body["LcdIndependence"], 978);
    }

    #[tokio::test]
    async fn send_pc_metrics_sends_update_pc_para_info() {
        let server = ok_device().await;
        send_pc_metrics(host(&server), 2, vec!["CPU 10%".into(), "GPU 20%".into()])
            .await
            .unwrap();

        assert_eq!(
            sent_bodies(&server).await[0],
            json!({
                "Command": "Device/UpdatePCParaInfo",
                "ScreenList": [{ "LcdId": 2, "DispData": ["CPU 10%", "GPU 20%"] }]
            })
        );
    }

    #[tokio::test]
    async fn device_commands_surface_error_code() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "error_code": 1 })))
            .mount(&server)
            .await;

        assert!(send_pc_metrics(host(&server), 0, vec![]).await.is_err());
        assert!(activate_pc_monitor(host(&server), 1, 1, 0).await.is_err());
    }

    #[tokio::test]
    async fn draw_commands_succeed_on_token_error() {
        // Verified on a Times Gate: images are drawn even when it replies this.
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "error_code": "DeviceToken is err" })),
            )
            .mount(&server)
            .await;

        assert!(send_pc_metrics(host(&server), 0, vec![]).await.is_ok());
        assert!(activate_pc_monitor(host(&server), 1, 1, 0).await.is_ok());
    }

    #[tokio::test]
    async fn get_lcd_info_resolves_device_id_by_ip() {
        let cloud = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/Device/ReturnSameLANDevice"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "ReturnCode": 0,
                "DeviceList": [
                    { "DeviceName": "Other", "DeviceId": 1, "DevicePrivateIP": "10.0.0.2" },
                    { "DeviceName": "Gate", "DeviceId": 300038420, "DevicePrivateIP": "10.0.0.100", "Hardware": 400 }
                ]
            })))
            .mount(&cloud)
            .await;
        Mock::given(method("POST"))
            .and(path("/Channel/Get5LcdInfoV2"))
            .and(body_json(json!({ "DeviceId": 300038420, "DeviceType": "LCD" })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "ReturnCode": 0,
                "ChannelType": 1,
                "LcdIndependence": 980,
                "LcdIndependenceList": [
                    { "LcdIndependence": 978, "LcdList": [] },
                    { "LcdIndependence": 980, "LcdList": [{ "LcdClockId": 625 }] }
                ]
            })))
            .expect(1)
            .mount(&cloud)
            .await;

        let info = get_lcd_info_from(&cloud.uri(), "10.0.0.100").await.unwrap();
        assert_eq!(info.device_id, 300038420);
        assert_eq!(info.lcd_independence, 980);
        assert_eq!(info.independence_list.len(), 2);
    }

    #[tokio::test]
    async fn get_lcd_info_fails_for_unknown_ip() {
        let cloud = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/Device/ReturnSameLANDevice"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "ReturnCode": 0, "DeviceList": []
            })))
            .mount(&cloud)
            .await;

        let err = get_lcd_info_from(&cloud.uri(), "10.0.0.100")
            .await
            .unwrap_err();
        assert!(err.contains("not found"), "{err}");
    }
}
