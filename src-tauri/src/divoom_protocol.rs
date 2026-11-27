//! Pure builders and parsers for the Divoom HTTP API.
//!
//! Every payload and response shape here follows the official third-party API
//! docs (http://doc.divoom-gz.com/web/#/12, section "TimeGate"). Keeping them
//! free of I/O lets the tests below pin the wire format without a device.

use serde_json::{json, Number, Value};

use crate::models::{
    DeviceSettings, DivoomDevice, LcdIndependenceInfo, LcdInfo, LcdInfoResponse,
};

pub const CLOUD_BASE_URL: &str = "https://app.divoom-gz.com";
pub const PATH_FIND_DEVICE: &str = "/Device/ReturnSameLANDevice";
pub const PATH_5LCD_INFO: &str = "/Channel/Get5LcdInfoV2";

/// Number of LCDs on a Times Gate.
pub const LCD_COUNT: usize = 5;
/// ClockId of the "PC Monitor" sub dial.
pub const PC_MONITOR_CLOCK_ID: u64 = 625;

pub fn device_post_url(ip: &str) -> String {
    format!("http://{}/post", ip)
}

// ---------------------------------------------------------------------------
// Responses
// ---------------------------------------------------------------------------

/// Reply of newer firmware when the request has no valid `LocalToken`.
pub const TOKEN_ERROR: &str = "DeviceToken is err";

/// Error returned to the frontend when a read needs a `LocalToken`.
/// `useDevice.ts` matches on it.
pub const TOKEN_REQUIRED_ERROR: &str = "LOCAL_TOKEN_REQUIRED";

/// Adds `"LocalToken": token` to a command object. Not in the public docs;
/// newer Times Gate firmware needs it (field name as in Divoom's own
/// divoom-pc-monitor). Leaves the command unchanged without a token.
pub fn with_local_token(command: &Value, token: Option<u32>) -> Value {
    let mut command = command.clone();
    if let (Some(token), Some(obj)) = (token, command.as_object_mut()) {
        obj.insert("LocalToken".to_string(), Value::from(token));
    }
    command
}

/// True when the device complained about a missing/wrong `LocalToken`.
pub fn is_token_error(response: &Value) -> bool {
    response.get("error_code").and_then(Value::as_str) == Some(TOKEN_ERROR)
}

/// Device replies `{"error_code": 0, ...}`; any other code is a failure.
/// Newer firmware also returns string codes. `"DeviceToken is err"` is let
/// through: tested on a Times Gate, write commands (images, brightness) are
/// still applied with it, while reads come back without data, so readers
/// check `is_token_error` themselves.
pub fn check_device_response(response: Value) -> Result<Value, String> {
    match response.get("error_code") {
        None => Ok(response),
        Some(Value::Number(n)) if n.as_i64() == Some(0) => Ok(response),
        Some(Value::String(s)) if s.is_empty() || s == "0" || s == TOKEN_ERROR => Ok(response),
        Some(Value::String(s)) => Err(format!("Device returned error_code \"{}\"", s)),
        Some(code) => Err(format!("Device returned error_code {}", code)),
    }
}

/// Cloud replies `{"ReturnCode": 0, "ReturnMessage": "", ...}`.
pub fn check_cloud_response(response: Value) -> Result<Value, String> {
    match response.get("ReturnCode").and_then(Value::as_i64) {
        Some(0) | None => Ok(response),
        Some(code) => {
            let message = response
                .get("ReturnMessage")
                .and_then(Value::as_str)
                .unwrap_or("");
            Err(format!("Divoom cloud returned ReturnCode {}: {}", code, message))
        }
    }
}

/// `Hardware` is not in the public docs but the cloud does return it.
pub fn hardware_to_device_type(hardware: u64) -> &'static str {
    match hardware {
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
}

/// Parses the `Device/ReturnSameLANDevice` response.
pub fn parse_device_list(json: &Value) -> Vec<DivoomDevice> {
    let Some(list) = json.get("DeviceList").and_then(Value::as_array) else {
        return Vec::new();
    };

    list.iter()
        .map(|d| DivoomDevice {
            name: d
                .get("DeviceName")
                .and_then(Value::as_str)
                .unwrap_or("Unknown Device")
                .to_string(),
            mac_address: d.get("DeviceMac").and_then(Value::as_str).map(str::to_string),
            device_type: hardware_to_device_type(
                d.get("Hardware").and_then(Value::as_u64).unwrap_or(0),
            )
            .to_string(),
            ip_address: d
                .get("DevicePrivateIP")
                .and_then(Value::as_str)
                .map(str::to_string),
            signal_strength: None,
            is_connected: true,
            device_id: d.get("DeviceId").and_then(Value::as_u64),
        })
        .collect()
}

/// Drops devices that repeat an IP or MAC already seen.
pub fn dedup_devices(devices: Vec<DivoomDevice>) -> Vec<DivoomDevice> {
    let mut unique: Vec<DivoomDevice> = Vec::new();
    for device in devices {
        let is_duplicate = unique.iter().any(|d| {
            (device.ip_address.is_some() && d.ip_address == device.ip_address)
                || (device.mac_address.is_some() && d.mac_address == device.mac_address)
        });
        if !is_duplicate {
            unique.push(device);
        }
    }
    unique
}

/// Parses the `Channel/GetAllConf` response.
pub fn parse_device_settings(json: &Value) -> DeviceSettings {
    let u8_field = |key: &str| json.get(key).and_then(Value::as_u64).map(|v| v as u8);
    DeviceSettings {
        brightness: u8_field("Brightness"),
        rotation_flag: u8_field("RotationFlag"),
        date_format: json
            .get("DateFormat")
            .and_then(Value::as_str)
            .map(str::to_string),
        time24_flag: u8_field("Time24Flag"),
        temperature_mode: u8_field("TemperatureMode"),
        mirror_flag: u8_field("MirrorFlag"),
        light_switch: u8_field("LightSwitch"),
    }
}

/// Parses the `Channel/Get5LcdInfoV2` response.
pub fn parse_lcd_info(json: &Value, device_id: u64) -> LcdInfoResponse {
    let independence_list = json
        .get("LcdIndependenceList")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .map(|item| LcdIndependenceInfo {
                    lcd_independence: item
                        .get("LcdIndependence")
                        .and_then(Value::as_u64)
                        .unwrap_or(0),
                    lcd_list: item
                        .get("LcdList")
                        .and_then(Value::as_array)
                        .map(|lcds| {
                            lcds.iter()
                                .map(|lcd| LcdInfo {
                                    lcd_clock_id: lcd
                                        .get("LcdClockId")
                                        .and_then(Value::as_u64)
                                        .unwrap_or(0),
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default();

    LcdInfoResponse {
        device_id,
        channel_type: json.get("ChannelType").and_then(Value::as_u64).unwrap_or(0),
        lcd_independence: json
            .get("LcdIndependence")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        independence_list,
    }
}

// ---------------------------------------------------------------------------
// Requests
// ---------------------------------------------------------------------------

pub fn get_all_conf() -> Value {
    json!({ "Command": "Channel/GetAllConf" })
}

/// Brightness 0..=100.
pub fn set_brightness(value: &Number) -> Value {
    json!({ "Command": "Channel/SetBrightness", "Brightness": value })
}

/// 1 = on, 0 = off.
pub fn on_off_screen(value: &Number) -> Value {
    json!({ "Command": "Channel/OnOffScreen", "OnOff": value })
}

/// 0 = Celsius, 1 = Fahrenheit.
pub fn set_temperature_mode(value: &Number) -> Value {
    json!({ "Command": "Device/SetDisTempMode", "Mode": value })
}

/// 0 = disable, 1 = enable.
pub fn set_mirror_mode(value: &Number) -> Value {
    json!({ "Command": "Device/SetMirrorMode", "Mode": value })
}

/// 1 = 24-hour, 0 = 12-hour.
pub fn set_time24_flag(value: &Number) -> Value {
    json!({ "Command": "Device/SetTime24Flag", "Mode": value })
}

pub fn sys_reboot() -> Value {
    json!({ "Command": "Device/SysReboot" })
}

/// One-hot `LcdArray` for a single screen; out-of-range index selects none.
pub fn lcd_array_for(screen_index: u32) -> [u8; LCD_COUNT] {
    let mut lcd_array = [0u8; LCD_COUNT];
    if let Some(slot) = lcd_array.get_mut(screen_index as usize) {
        *slot = 1;
    }
    lcd_array
}

/// Single-frame `Draw/SendHttpGif`. `jpeg_base64` is the base64 of a JPEG file.
pub fn send_http_gif(screen_index: u32, pic_id: u32, width: u32, jpeg_base64: &str) -> Value {
    json!({
        "Command": "Draw/SendHttpGif",
        "LcdArray": lcd_array_for(screen_index),
        "PicNum": 1,
        "PicWidth": width,
        "PicOffset": 0,
        "PicID": pic_id,
        "PicSpeed": 1000,
        "PicData": jpeg_base64
    })
}

// Screen text is drawn into the image on the PC (frontend canvas) and sent
// with Draw/SendHttpGif. Tested on a Times Gate: Draw/SendHttpText returns
// error_code 0 but draws nothing; Draw/SendHttpItemList hangs the LCD on
// "loading..." unless BackgroudGif is an internet URL (so no local image under
// the text), and its free-text items (type 22) appear only now and then.

/// Body of the cloud `Channel/Get5LcdInfoV2` request (POST, JSON).
pub fn get_5lcd_info_body(device_id: u64) -> Value {
    json!({ "DeviceId": device_id, "DeviceType": "LCD" })
}

/// `Channel/SetClockSelectId`: select sub dial `clock_id` on one LCD.
/// `DeviceId` is not in the docs but is kept since devices accept it.
pub fn set_clock_select_id(
    device_id: u64,
    lcd_independence: u64,
    lcd_index: u32,
    clock_id: u64,
) -> Value {
    json!({
        "Command": "Channel/SetClockSelectId",
        "LcdIndependence": lcd_independence,
        "DeviceId": device_id,
        "LcdIndex": lcd_index,
        "ClockId": clock_id
    })
}

pub fn update_pc_para_info(lcd_index: u32, disp_data: &[String]) -> Value {
    json!({
        "Command": "Device/UpdatePCParaInfo",
        "ScreenList": [{ "LcdId": lcd_index, "DispData": disp_data }]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(v: u64) -> Number {
        Number::from(v)
    }

    // --- responses ---------------------------------------------------------

    #[test]
    fn device_response_ok_when_error_code_zero_or_missing() {
        assert!(check_device_response(json!({ "error_code": 0 })).is_ok());
        assert!(check_device_response(json!({ "Brightness": 50 })).is_ok());
    }

    #[test]
    fn device_response_err_on_nonzero_error_code() {
        let err = check_device_response(json!({ "error_code": 1 })).unwrap_err();
        assert!(err.contains("error_code 1"), "{err}");
    }

    #[test]
    fn device_response_err_on_string_error_code() {
        // Real Times Gate reply to an unknown command.
        let err =
            check_device_response(json!({ "error_code": "Request data illegal json" })).unwrap_err();
        assert!(err.contains("Request data illegal json"), "{err}");
        assert!(check_device_response(json!({ "error_code": "0" })).is_ok());
    }

    #[test]
    fn with_local_token_adds_numeric_field() {
        let cmd = get_all_conf();
        assert_eq!(
            with_local_token(&cmd, Some(123456)),
            json!({ "Command": "Channel/GetAllConf", "LocalToken": 123456 })
        );
        assert_eq!(with_local_token(&cmd, None), cmd);
        assert_eq!(with_local_token(&json!([1]), Some(1)), json!([1]));
    }

    #[test]
    fn token_error_is_not_fatal_but_detectable() {
        // Real Times Gate reply without LocalToken; writes are still applied.
        let reply = json!({ "error_code": "DeviceToken is err" });
        assert!(is_token_error(&reply));
        assert!(check_device_response(reply).is_ok());
        assert!(!is_token_error(&json!({ "error_code": 0 })));
    }

    #[test]
    fn cloud_response_err_on_nonzero_return_code() {
        assert!(check_cloud_response(json!({ "ReturnCode": 0 })).is_ok());
        let err =
            check_cloud_response(json!({ "ReturnCode": 2, "ReturnMessage": "bad" })).unwrap_err();
        assert!(err.contains("ReturnCode 2") && err.contains("bad"), "{err}");
    }

    #[test]
    fn parses_find_device_doc_example() {
        // Response example from the "Find device" page, plus Hardware.
        let json = json!({
            "ReturnCode": 0,
            "ReturnMessage": "",
            "DeviceList": [
                {
                    "DeviceName": "Pixoo64",
                    "DeviceId": 300000020,
                    "DevicePrivateIP": "10.0.0.100",
                    "DeviceMac": "a8032aff46b1"
                },
                {
                    "DeviceName": "Gate",
                    "DeviceId": 300038420,
                    "DevicePrivateIP": "10.0.0.101",
                    "DeviceMac": "a8032aff46b2",
                    "Hardware": 400
                }
            ]
        });

        let devices = parse_device_list(&json);
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].name, "Pixoo64");
        assert_eq!(devices[0].device_id, Some(300000020));
        assert_eq!(devices[0].ip_address.as_deref(), Some("10.0.0.100"));
        assert_eq!(devices[0].mac_address.as_deref(), Some("a8032aff46b1"));
        assert_eq!(devices[0].device_type, "Unknown Divoom Device");
        assert_eq!(devices[1].device_type, "Times Gate");
    }

    #[test]
    fn parse_device_list_handles_missing_fields() {
        assert!(parse_device_list(&json!({})).is_empty());
        let devices = parse_device_list(&json!({ "DeviceList": [{}] }));
        assert_eq!(devices[0].name, "Unknown Device");
        assert_eq!(devices[0].device_id, None);
        assert_eq!(devices[0].ip_address, None);
    }

    #[test]
    fn hardware_codes_map_to_models() {
        assert_eq!(hardware_to_device_type(400), "Times Gate");
        assert_eq!(hardware_to_device_type(401), "Pixoo 64");
        assert_eq!(hardware_to_device_type(408), "Pixoo Mini");
        assert_eq!(hardware_to_device_type(0), "Unknown Divoom Device");
    }

    #[test]
    fn dedup_removes_same_ip_or_mac() {
        let json = json!({ "DeviceList": [
            { "DeviceName": "a", "DevicePrivateIP": "10.0.0.1", "DeviceMac": "m1" },
            { "DeviceName": "b", "DevicePrivateIP": "10.0.0.1", "DeviceMac": "m2" },
            { "DeviceName": "c", "DevicePrivateIP": "10.0.0.2", "DeviceMac": "m1" },
            { "DeviceName": "d", "DevicePrivateIP": "10.0.0.3", "DeviceMac": "m3" }
        ]});
        let names: Vec<_> = dedup_devices(parse_device_list(&json))
            .into_iter()
            .map(|d| d.name)
            .collect();
        assert_eq!(names, ["a", "d"]);
    }

    #[test]
    fn parses_get_all_conf_doc_example() {
        let json = json!({
            "error_code": 0,
            "Brightness": 100,
            "RotationFlag": 1,
            "DateFormat": "YYYY-MM-DD",
            "Time24Flag": 1,
            "TemperatureMode": 1,
            "MirrorFlag": 1,
            "LightSwitch": 1
        });
        let s = parse_device_settings(&json);
        assert_eq!(s.brightness, Some(100));
        assert_eq!(s.rotation_flag, Some(1));
        assert_eq!(s.date_format.as_deref(), Some("YYYY-MM-DD"));
        assert_eq!(s.time24_flag, Some(1));
        assert_eq!(s.temperature_mode, Some(1));
        assert_eq!(s.mirror_flag, Some(1));
        assert_eq!(s.light_switch, Some(1));
    }

    #[test]
    fn parses_get_5lcd_info_doc_example() {
        // Trimmed response example from "Get Channel Information".
        let json = json!({
            "ReturnCode": 0,
            "ReturnMessage": "",
            "ChannelType": 1,
            "LcdIndependence": 980,
            "ClockId": 217,
            "LcdIndependenceList": [
                {
                    "IndependenceName": "Control1",
                    "LcdIndependence": 978,
                    "LcdList": [
                        { "LcdClockId": 61 }, { "LcdClockId": 182 }, { "LcdClockId": 57 },
                        { "LcdClockId": 61 }, { "LcdClockId": 61 }
                    ]
                },
                {
                    "IndependenceName": "Control2",
                    "LcdIndependence": 980,
                    "LcdIndependPos": 1,
                    "LcdList": [
                        { "LcdClockId": 57 },
                        { "LcdSelectIndex": 0, "LcdClockId": 625, "ClockImagePixelId": "" },
                        {}, { "LcdClockId": 61 }, { "LcdClockId": 59 }
                    ]
                }
            ],
            "DeviceId": 300038420
        });

        let info = parse_lcd_info(&json, 300038420);
        assert_eq!(info.device_id, 300038420);
        assert_eq!(info.channel_type, 1);
        assert_eq!(info.lcd_independence, 980);
        assert_eq!(info.independence_list.len(), 2);
        assert_eq!(info.independence_list[0].lcd_independence, 978);
        let clocks: Vec<_> = info.independence_list[1]
            .lcd_list
            .iter()
            .map(|l| l.lcd_clock_id)
            .collect();
        assert_eq!(clocks, [57, 625, 0, 61, 59]);
    }

    // --- requests ----------------------------------------------------------

    #[test]
    fn device_url_matches_docs() {
        assert_eq!(device_post_url("10.0.0.5"), "http://10.0.0.5/post");
    }

    #[test]
    fn settings_payloads_match_docs() {
        assert_eq!(get_all_conf(), json!({ "Command": "Channel/GetAllConf" }));
        assert_eq!(
            set_brightness(&n(100)),
            json!({ "Command": "Channel/SetBrightness", "Brightness": 100 })
        );
        assert_eq!(
            on_off_screen(&n(1)),
            json!({ "Command": "Channel/OnOffScreen", "OnOff": 1 })
        );
        assert_eq!(
            set_temperature_mode(&n(0)),
            json!({ "Command": "Device/SetDisTempMode", "Mode": 0 })
        );
        assert_eq!(
            set_mirror_mode(&n(0)),
            json!({ "Command": "Device/SetMirrorMode", "Mode": 0 })
        );
        assert_eq!(
            set_time24_flag(&n(0)),
            json!({ "Command": "Device/SetTime24Flag", "Mode": 0 })
        );
        assert_eq!(sys_reboot(), json!({ "Command": "Device/SysReboot" }));
    }

    #[test]
    fn lcd_array_is_one_hot() {
        assert_eq!(lcd_array_for(0), [1, 0, 0, 0, 0]);
        assert_eq!(lcd_array_for(4), [0, 0, 0, 0, 1]);
        assert_eq!(lcd_array_for(5), [0, 0, 0, 0, 0]);
    }

    #[test]
    fn send_http_gif_matches_docs() {
        let payload = send_http_gif(2, 1001, 128, "QUJD");
        assert_eq!(
            payload,
            json!({
                "Command": "Draw/SendHttpGif",
                "LcdArray": [0, 0, 1, 0, 0],
                "PicNum": 1,
                "PicWidth": 128,
                "PicOffset": 0,
                "PicID": 1001,
                "PicSpeed": 1000,
                "PicData": "QUJD"
            })
        );
        // Documented key is "LcdArray"; the old "LCDArray" spelling must not come back.
        assert!(payload.get("LCDArray").is_none());
    }

    #[test]
    fn cloud_lcd_info_body_matches_docs() {
        assert_eq!(
            get_5lcd_info_body(300038420),
            json!({ "DeviceId": 300038420, "DeviceType": "LCD" })
        );
    }

    #[test]
    fn set_clock_select_id_has_documented_fields() {
        let payload = set_clock_select_id(300038420, 980, 2, PC_MONITOR_CLOCK_ID);
        assert_eq!(payload["Command"], "Channel/SetClockSelectId");
        assert_eq!(payload["ClockId"], 625);
        assert_eq!(payload["LcdIndex"], 2);
        assert_eq!(payload["LcdIndependence"], 980);
    }

    #[test]
    fn update_pc_para_info_shape() {
        let data = vec!["10%".to_string(), "20%".to_string()];
        assert_eq!(
            update_pc_para_info(1, &data),
            json!({
                "Command": "Device/UpdatePCParaInfo",
                "ScreenList": [{ "LcdId": 1, "DispData": ["10%", "20%"] }]
            })
        );
    }
}
