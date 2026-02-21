mod app_settings;
mod device_commands;
mod divoom_api;
mod draw_commands;
mod models;
mod system_metrics;

use std::sync::atomic::Ordering;
use tauri::Manager;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

#[cfg(debug_assertions)]
fn setup_devtools(app: &tauri::App) {
    if let Some(main_window) = app.get_webview_window("main") {
        main_window.open_devtools();
    }
}

#[cfg(not(debug_assertions))]
fn setup_devtools(_app: &tauri::App) {}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    dotenvy::dotenv().ok();

    let _ = std::panic::set_hook(Box::new(|_| {
        system_metrics::stop_sidecar_service();
    }));

    tauri::Builder::default()
        .setup(|app| {
            setup_devtools(app);

            if let Some(data_dir) = app.path().app_data_dir().ok() {
                app_settings::init(data_dir);
            }

            system_metrics::setup_sidecar_service();

            let show_item = MenuItemBuilder::with_id("show", "Показать").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "Выход").build(app)?;
            let tray_menu = MenuBuilder::new(app)
                .items(&[&show_item, &quit_item])
                .build()?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Divoom Monitor")
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.unminimize();
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        system_metrics::stop_sidecar_service();
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.unminimize();
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                let app_handle = app.handle().clone();
                let _ = window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        if app_settings::CLOSE_TO_TRAY.load(Ordering::Relaxed) {
                            api.prevent_close();
                            let _ = window_clone.hide();
                        } else {
                            system_metrics::stop_sidecar_service();
                            app_handle.exit(0);
                        }
                    }
                });

                let args: Vec<String> = std::env::args().collect();
                if args.iter().any(|a| a == "--minimized") {
                    let _ = window.hide();
                }
            }

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .invoke_handler(tauri::generate_handler![
            device_commands::scan_devices,
            device_commands::get_device_info,
            device_commands::set_brightness,
            device_commands::set_switch_screen,
            device_commands::set_temperature_mode,
            device_commands::set_mirror_mode,
            device_commands::set_24_hours_mode,
            device_commands::reboot_device,
            draw_commands::upload_image_from_url,
            draw_commands::upload_image_from_file,
            draw_commands::set_screen_text,
            draw_commands::get_lcd_info,
            draw_commands::activate_pc_monitor,
            draw_commands::send_pc_metrics,
            system_metrics::get_system_metrics,
            app_settings::set_close_to_tray,
            app_settings::get_close_to_tray,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    system_metrics::stop_sidecar_service();
}
