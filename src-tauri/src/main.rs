#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod api;
mod config;
mod ludusavi;
mod scanner;
mod uploader;

use std::sync::Mutex;
use tauri::{CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu};

pub struct AppState {
    pub config: Mutex<config::Config>,
    pub is_scanning: Mutex<bool>,
}

fn main() {
    env_logger::init();

    let quit = CustomMenuItem::new("quit".to_string(), "Quit SaveKnight");
    let show = CustomMenuItem::new("show".to_string(), "Show Window");
    let scan = CustomMenuItem::new("scan".to_string(), "Scan for Saves");
    
    let tray_menu = SystemTrayMenu::new()
        .add_item(show)
        .add_item(scan)
        .add_native_item(tauri::SystemTrayMenuItem::Separator)
        .add_item(quit);
    
    let system_tray = SystemTray::new().with_menu(tray_menu);

    tauri::Builder::default()
        .manage(AppState {
            config: Mutex::new(config::Config::load().unwrap_or_default()),
            is_scanning: Mutex::new(false),
        })
        .system_tray(system_tray)
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::LeftClick { .. } => {
                if let Some(window) = app.get_window("main") {
                    window.show().unwrap();
                    window.set_focus().unwrap();
                }
            }
            SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                "quit" => {
                    std::process::exit(0);
                }
                "show" => {
                    if let Some(window) = app.get_window("main") {
                        window.show().unwrap();
                        window.set_focus().unwrap();
                    }
                }
                "scan" => {
                    if let Some(window) = app.get_window("main") {
                        window.emit("trigger-scan", ()).unwrap();
                    }
                }
                _ => {}
            },
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            api::get_config,
            api::save_config,
            api::login,
            api::logout,
            api::get_auth_status,
            api::scan_games,
            api::get_detected_games,
            api::upload_saves,
            api::get_upload_history,
            api::get_game_profiles,
            api::create_game_profile,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
