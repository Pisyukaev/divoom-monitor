use base64::{engine::general_purpose, Engine as _};
use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageEncoder};
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use crate::divoom_api::{discover_via_divoom_api, send_command, send_command_with_timeout};
use crate::models::{LcdIndependenceInfo, LcdInfo, LcdInfoResponse, TextConfig};

static PIC_ID_COUNTER: AtomicU32 = AtomicU32::new(1000);

fn get_next_pic_id() -> u32 {
    PIC_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
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
pub async fn upload_image_from_url(
    ip_address: String,
    screen_index: u32,
    url: String,
) -> Result<(), String> {
    let img = load_image_from_url(&url).await?;
    let image_data = resize_image(img, 128, 128)?;
    let base64_data = general_purpose::STANDARD.encode(&image_data);

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
pub async fn upload_image_from_file(
    ip_address: String,
    screen_index: u32,
    file_path: String,
) -> Result<(), String> {
    let img = load_image_from_file(&file_path).await?;
    let image_data = resize_image(img, 128, 128)?;
    let base64_data = general_purpose::STANDARD.encode(&image_data);

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
pub async fn set_screen_text(
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
pub async fn get_lcd_info(ip_address: String) -> Result<LcdInfoResponse, String> {
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
pub async fn activate_pc_monitor(
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
pub async fn send_pc_metrics(
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
