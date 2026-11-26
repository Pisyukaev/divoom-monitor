//! HTTP transport to Divoom devices (`POST http://<ip>/post`) and the Divoom cloud.
//! Payload shapes live in `divoom_protocol.rs`.

use std::time::Duration;

use serde_json::Value;

use crate::divoom_protocol::{
    check_cloud_response, check_device_response, device_post_url, get_5lcd_info_body,
    parse_device_list, parse_lcd_info, CLOUD_BASE_URL, PATH_5LCD_INFO, PATH_FIND_DEVICE,
};
use crate::models::{DivoomDevice, LcdInfoResponse};

pub const DEVICE_TIMEOUT: Duration = Duration::from_millis(500);
pub const CLOUD_TIMEOUT: Duration = Duration::from_secs(10);

fn client(timeout: Duration) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))
}

pub async fn send_command(ip: &str, command: &Value) -> Result<Value, String> {
    send_command_with_timeout(ip, command, DEVICE_TIMEOUT).await
}

/// Sends one device command. Fails on transport errors, non-2xx status,
/// a non-JSON body, or a non-zero `error_code`.
pub async fn send_command_with_timeout(
    ip: &str,
    command: &Value,
    timeout: Duration,
) -> Result<Value, String> {
    let response = client(timeout)?
        .post(device_post_url(ip))
        .json(command)
        .send()
        .await
        .map_err(|e| format!("Failed to send command: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Command failed with status: {}", response.status()));
    }

    let result: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    check_device_response(result)
}

async fn cloud_post(base_url: &str, path: &str, body: Option<&Value>) -> Result<Value, String> {
    let mut request = client(CLOUD_TIMEOUT)?.post(format!("{}{}", base_url, path));
    if let Some(body) = body {
        request = request.json(body);
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("Failed to request Divoom API: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Divoom API returned status: {}", response.status()));
    }

    let json: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    check_cloud_response(json)
}

/// Devices the Divoom cloud sees behind the same public IP.
pub async fn discover_via_divoom_api() -> Result<Vec<DivoomDevice>, String> {
    discover_devices_at(CLOUD_BASE_URL).await
}

pub async fn discover_devices_at(base_url: &str) -> Result<Vec<DivoomDevice>, String> {
    let json = cloud_post(base_url, PATH_FIND_DEVICE, None).await?;
    Ok(parse_device_list(&json))
}

/// Times Gate channel info (`Channel/Get5LcdInfoV2`, POST with JSON body per docs).
pub async fn get_5lcd_info_at(base_url: &str, device_id: u64) -> Result<LcdInfoResponse, String> {
    let json = cloud_post(base_url, PATH_5LCD_INFO, Some(&get_5lcd_info_body(device_id)))
        .await
        .map_err(|e| format!("Failed to get LCD info: {}", e))?;
    Ok(parse_lcd_info(&json, device_id))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// `MockServer::uri()` is `http://127.0.0.1:PORT`; devices are addressed by host only.
    pub(crate) fn host(server: &MockServer) -> String {
        server.uri().trim_start_matches("http://").to_string()
    }

    #[tokio::test]
    async fn send_command_posts_json_to_post_endpoint() {
        let server = MockServer::start().await;
        let cmd = json!({ "Command": "Channel/GetAllConf" });
        Mock::given(method("POST"))
            .and(path("/post"))
            .and(body_json(&cmd))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "error_code": 0, "Brightness": 42
            })))
            .expect(1)
            .mount(&server)
            .await;

        let result = send_command(&host(&server), &cmd).await.unwrap();
        assert_eq!(result["Brightness"], 42);
    }

    #[tokio::test]
    async fn send_command_fails_on_device_error_code() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "error_code": 1 })))
            .mount(&server)
            .await;

        let err = send_command(&host(&server), &json!({ "Command": "X" }))
            .await
            .unwrap_err();
        assert!(err.contains("error_code 1"), "{err}");
    }

    #[tokio::test]
    async fn send_command_fails_on_http_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let err = send_command(&host(&server), &json!({})).await.unwrap_err();
        assert!(err.contains("500"), "{err}");
    }

    #[tokio::test]
    async fn send_command_fails_on_non_json_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
            .mount(&server)
            .await;

        let err = send_command(&host(&server), &json!({})).await.unwrap_err();
        assert!(err.contains("parse"), "{err}");
    }

    #[tokio::test]
    async fn send_command_times_out() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "error_code": 0 }))
                    .set_delay(Duration::from_millis(300)),
            )
            .mount(&server)
            .await;

        let err = send_command_with_timeout(&host(&server), &json!({}), Duration::from_millis(50))
            .await
            .unwrap_err();
        assert!(err.contains("Failed to send command"), "{err}");
    }

    #[tokio::test]
    async fn discover_posts_to_find_device_endpoint() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/Device/ReturnSameLANDevice"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "ReturnCode": 0,
                "ReturnMessage": "",
                "DeviceList": [{
                    "DeviceName": "Gate",
                    "DeviceId": 300038420,
                    "DevicePrivateIP": "10.0.0.100",
                    "DeviceMac": "a8032aff46b1",
                    "Hardware": 400
                }]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let devices = discover_devices_at(&server.uri()).await.unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].device_type, "Times Gate");
        assert_eq!(devices[0].device_id, Some(300038420));
    }

    #[tokio::test]
    async fn discover_fails_on_cloud_return_code() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "ReturnCode": 1, "ReturnMessage": "oops"
            })))
            .mount(&server)
            .await;

        let err = discover_devices_at(&server.uri()).await.unwrap_err();
        assert!(err.contains("oops"), "{err}");
    }

    #[tokio::test]
    async fn get_5lcd_info_posts_documented_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/Channel/Get5LcdInfoV2"))
            .and(body_json(json!({ "DeviceId": 300038420, "DeviceType": "LCD" })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "ReturnCode": 0,
                "ChannelType": 1,
                "LcdIndependence": 978,
                "LcdIndependenceList": [{
                    "LcdIndependence": 978,
                    "LcdList": [{ "LcdClockId": 625 }]
                }]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let info = get_5lcd_info_at(&server.uri(), 300038420).await.unwrap();
        assert_eq!(info.lcd_independence, 978);
        assert_eq!(info.independence_list[0].lcd_list[0].lcd_clock_id, 625);
    }
}
