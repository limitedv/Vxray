#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod core;
mod commands;

use core::manager::CoreManager;
use core::settings::SettingsManager;
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

fn main() {
    let core_manager = CoreManager::new();
    let settings_manager = SettingsManager::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(Mutex::new(core_manager))
        .manage(Mutex::new(settings_manager))
        .setup(|app| {
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        let state = app.state::<Mutex<CoreManager>>();
                        let mut core = state.lock().unwrap();
                        core.stop_core(true);
                        core.stop_tun_core();
                        std::process::exit(0);
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                        }
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
                        if let Some(window) = tray.app_handle().get_webview_window("main") {
                            let _ = window.show();
                        }
                    }
                })
                .build(app)?;
            
            // Handle start-minimized: if auto_start + start_minimized are both on AND launched via --autostart, hide the window
            {
                let settings_state = app.state::<Mutex<SettingsManager>>();
                let s = settings_state.lock().unwrap();
                let is_autostart_launch = std::env::args().any(|arg| arg == "--autostart");
                
                let should_hide = s.settings.ui.auto_start && s.settings.ui.start_minimized && is_autostart_launch;
                if !should_hide {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                    }
                }
            }

            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                window.hide().unwrap();
                api.prevent_close();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_servers,
            commands::add_server,
            commands::get_subscriptions,
            commands::add_subscription,
            commands::delete_server,
            commands::delete_subscription,
            commands::update_subscription,
            commands::connect,
            commands::disconnect,
            commands::check_ip,
            commands::test_all_latency,
            commands::test_single_latency,
            commands::get_live_ping,
            commands::toggle_tun,
            commands::is_admin,
            commands::clear_data,
            commands::is_core_alive,
            commands::get_tun_enabled,
            commands::get_startup_settings,
            commands::set_auto_reconnect,
            commands::set_auto_start,
            commands::set_start_minimized,
            commands::save_last_connection,
            commands::get_auto_reconnect_info,
            commands::is_start_minimized,
            commands::minimize_window,
            commands::maximize_window,
            commands::close_window,
            commands::drag_window,
            commands::toggle_pin_subscription,
            commands::save_subscription_order,
            commands::restart_as_admin
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[test]
fn test_dump_xray2() {
    let link = "vless://0e61c812-49f0-4dd3-992d-654d47c52a0f@fsxhh1.eorqen.ir:443?encryption=none&security=tls&sni=speedtest.net&fp=firefox&alpn=h2&insecure=0&allowInsecure=0&pcs=CD6E838B9BFE31CAB8B3D5C858B0D6223FE84ECE236C1FDB281BE93C302E1E6E&type=xhttp&host=happily-noted-goblin.global.ssl.fastly.net&path=%2Flizzard&mode=packet-up#%7C%F0%9F%87%B3%F0%9F%87%B1%7C-netherlands%20MCI%202";
    let mut cfg = crate::core::config::generate_xray_config(link, 28000).unwrap();
    if let Some(obj) = cfg.as_object_mut() { obj.remove("__core__"); }
    std::fs::write("../bin/test_xray2.json", serde_json::to_string(&cfg).unwrap()).unwrap();
    
    let mut child = std::process::Command::new("../bin/xray.exe")
        .args(["run", "-c", "../bin/test_xray2.json"])
        .spawn()
        .unwrap();
        
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    let client = reqwest::blocking::Client::builder()
        .proxy(reqwest::Proxy::all("socks5h://127.0.0.1:28000").unwrap())
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();
        
    match client.get("http://google.com/generate_204").send() {
        Ok(resp) => println!("Ping response: {}", resp.status()),
        Err(e) => println!("Ping error: {}", e),
    }
    
    child.kill().unwrap();
}
#[tokio::test]
async fn test_proxy_methods() {
    let link = "vless://0e61c812-49f0-4dd3-992d-654d47c52a0f@fsxhh1.eorqen.ir:443?encryption=none&security=tls&sni=speedtest.net&fp=firefox&alpn=h2&insecure=0&allowInsecure=0&pcs=CD6E838B9BFE31CAB8B3D5C858B0D6223FE84ECE236C1FDB281BE93C302E1E6E&type=xhttp&host=happily-noted-goblin.global.ssl.fastly.net&path=%2Flizzard&mode=packet-up#%7C%F0%9F%87%B3%F0%9F%87%B1%7C-netherlands%20MCI%202";
    let mut cfg = crate::core::config::generate_xray_config(link, 28000).unwrap();
    if let Some(obj) = cfg.as_object_mut() { obj.remove("__core__"); }
    std::fs::write("../bin/test_xray2.json", serde_json::to_string(&cfg).unwrap()).unwrap();
    
    let mut child = std::process::Command::new("../bin/xray.exe")
        .args(["run", "-c", "../bin/test_xray2.json"])
        .spawn()
        .unwrap();
        
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Test HTTP proxy (like in test_single_latency)
    let proxy_http = reqwest::Proxy::http("http://127.0.0.1:28000").unwrap();
    let client_http = reqwest::Client::builder().proxy(proxy_http).timeout(std::time::Duration::from_secs(5)).build().unwrap();
    match client_http.head("http://www.google.com/generate_204").send().await {
        Ok(resp) => println!("HTTP proxy response: {}", resp.status()),
        Err(e) => println!("HTTP proxy error: {}", e),
    }

    // Test SOCKS5 proxy
    let proxy_socks = reqwest::Proxy::all("socks5h://127.0.0.1:28000").unwrap();
    let client_socks = reqwest::Client::builder().proxy(proxy_socks).timeout(std::time::Duration::from_secs(5)).build().unwrap();
    match client_socks.head("http://www.google.com/generate_204").send().await {
        Ok(resp) => println!("SOCKS5 proxy response: {}", resp.status()),
        Err(e) => println!("SOCKS5 proxy error: {}", e),
    }
    
    child.kill().unwrap();
}
