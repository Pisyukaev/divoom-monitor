use serde_json::{Number, Value};

use crate::divoom_api::{discover_via_divoom_api, send_command};
use crate::divoom_protocol as proto;
use crate::models::{DeviceSettings, DivoomDevice};

#[tauri::command]
pub async fn scan_devices() -> Result<Vec<DivoomDevice>, String> {
    let devices = discover_via_divoom_api().await.unwrap_or_default();
    Ok(proto::dedup_devices(devices))
}

#[tauri::command]
pub async fn get_device_info(ip_address: String) -> Result<DeviceSettings, String> {
    let result = send_command(&ip_address, &proto::get_all_conf())
        .await
        .map_err(|e| format!("Failed to send command: {}", e))?;

    if proto::is_token_error(&result) {
        return Err(format!(
            "{}: device needs a valid LocalToken to read settings",
            proto::TOKEN_REQUIRED_ERROR
        ));
    }

    Ok(proto::parse_device_settings(&result))
}

/// Sets (or with `None` clears) the `LocalToken` sent to the device at `ip_address`.
#[tauri::command]
pub fn set_local_token(ip_address: String, token: Option<u32>) {
    crate::divoom_api::set_local_token(&ip_address, token);
}

// The setters below intentionally swallow errors: the UI fires them on every
// slider/switch change and has no way to report a failure.
async fn fire_and_forget(ip_address: &str, command: Value) {
    let _ = send_command(ip_address, &command).await;
}

#[tauri::command]
pub async fn set_brightness(ip_address: String, value: Number) {
    fire_and_forget(&ip_address, proto::set_brightness(&value)).await;
}

#[tauri::command]
pub async fn set_switch_screen(ip_address: String, value: Number) {
    fire_and_forget(&ip_address, proto::on_off_screen(&value)).await;
}

#[tauri::command]
pub async fn set_temperature_mode(ip_address: String, value: Number) {
    fire_and_forget(&ip_address, proto::set_temperature_mode(&value)).await;
}

#[tauri::command]
pub async fn set_mirror_mode(ip_address: String, value: Number) {
    fire_and_forget(&ip_address, proto::set_mirror_mode(&value)).await;
}

#[tauri::command]
pub async fn set_24_hours_mode(ip_address: String, value: Number) {
    fire_and_forget(&ip_address, proto::set_time24_flag(&value)).await;
}

#[tauri::command]
pub async fn reboot_device(ip_address: String) {
    fire_and_forget(&ip_address, proto::sys_reboot()).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::divoom_api::tests::host;
    use serde_json::json;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn device_expecting(body: Value) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/post"))
            .and(body_json(body))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "error_code": 0 })))
            .expect(1)
            .mount(&server)
            .await;
        server
    }

    #[tokio::test]
    async fn setters_send_documented_payloads() {
        let s = device_expecting(json!({ "Command": "Channel/SetBrightness", "Brightness": 70 })).await;
        set_brightness(host(&s), 70.into()).await;

        let s = device_expecting(json!({ "Command": "Channel/OnOffScreen", "OnOff": 0 })).await;
        set_switch_screen(host(&s), 0.into()).await;

        let s = device_expecting(json!({ "Command": "Device/SetDisTempMode", "Mode": 1 })).await;
        set_temperature_mode(host(&s), 1.into()).await;

        let s = device_expecting(json!({ "Command": "Device/SetMirrorMode", "Mode": 1 })).await;
        set_mirror_mode(host(&s), 1.into()).await;

        let s = device_expecting(json!({ "Command": "Device/SetTime24Flag", "Mode": 1 })).await;
        set_24_hours_mode(host(&s), 1.into()).await;

        let s = device_expecting(json!({ "Command": "Device/SysReboot" })).await;
        reboot_device(host(&s)).await;
        // `expect(1)` is verified when each MockServer drops.
    }

    #[tokio::test]
    async fn setters_do_not_panic_when_device_is_unreachable() {
        // Port 9 (discard) on localhost is not listening.
        set_brightness("127.0.0.1:9".into(), 10.into()).await;
    }

    #[tokio::test]
    async fn get_device_info_parses_get_all_conf() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(body_json(json!({ "Command": "Channel/GetAllConf" })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "error_code": 0,
                "Brightness": 80,
                "Time24Flag": 0,
                "LightSwitch": 1
            })))
            .mount(&server)
            .await;

        let settings = get_device_info(host(&server)).await.unwrap();
        assert_eq!(settings.brightness, Some(80));
        assert_eq!(settings.time24_flag, Some(0));
        assert_eq!(settings.light_switch, Some(1));
        assert_eq!(settings.mirror_flag, None);
    }

    #[tokio::test]
    async fn get_device_info_propagates_device_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "error_code": 3 })))
            .mount(&server)
            .await;

        assert!(get_device_info(host(&server)).await.is_err());
    }

    #[tokio::test]
    async fn get_device_info_fails_on_token_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "error_code": "DeviceToken is err" })),
            )
            .mount(&server)
            .await;

        let err = get_device_info(host(&server)).await.unwrap_err();
        assert!(err.starts_with(proto::TOKEN_REQUIRED_ERROR), "{err}");
    }
}
