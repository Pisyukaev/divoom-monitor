use serde::{Deserialize, Serialize};

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
