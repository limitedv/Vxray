use crate::core::config::*;
use crate::core::manager::CoreManager;
use crate::core::settings::SettingsManager;
use serde_json::Value;
use std::sync::Mutex;
use tauri::{State, Manager};
use reqwest::Client;
use std::time::Duration;
use std::net::ToSocketAddrs;
use std::os::windows::ffi::OsStrExt;

#[tauri::command]
pub fn get_servers(settings: State<Mutex<SettingsManager>>) -> Value {
    settings.lock().unwrap().get_server_groups()
}

#[tauri::command]
pub fn add_server(link: String, settings: State<Mutex<SettingsManager>>) {
    let remark = if let Some(idx) = link.rfind('#') {
        let r = &link[idx + 1..];
        urlencoding::decode(r).unwrap_or(std::borrow::Cow::Borrowed("")).to_string()
    } else {
        "Unknown".to_string()
    };
    settings.lock().unwrap().add_manual_server(link, remark);
}

#[tauri::command]
pub fn get_subscriptions(settings: State<Mutex<SettingsManager>>) -> Value {
    settings.lock().unwrap().get_subscriptions()
}

#[tauri::command]
pub fn add_subscription(url: String, settings: State<Mutex<SettingsManager>>) {
    settings.lock().unwrap().add_subscription(url);
}

#[tauri::command]
pub fn delete_server(group: String, link: String, settings: State<Mutex<SettingsManager>>) {
    let mut s = settings.lock().unwrap();
    if let Some(g) = s.settings.servers.get_mut(&group) {
        g.servers.retain(|server| {
            if let Some(l) = server.get("link").and_then(|v| v.as_str()) {
                l != link
            } else {
                true
            }
        });
    }
    let _ = s.save();
}

#[tauri::command]
pub fn delete_subscription(group_key: String, settings: State<Mutex<SettingsManager>>) {
    let mut s = settings.lock().unwrap();
    s.delete_subscription(&group_key);
}

#[tauri::command]
pub async fn update_subscription(url: String, settings: State<'_, Mutex<SettingsManager>>) -> Result<(), String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
        
    let response = client.get(&url).send().await.map_err(|e| e.to_string())?;
    let content = response.text().await.map_err(|e| e.to_string())?;
    
    let decoded = decode_base64_content(&content);
    let mut servers = vec![];
    
    for line in decoded.lines() {
        let link = line.trim();
        if link.is_empty() { continue; }
        
        let remark = if let Some(idx) = link.rfind('#') {
            urlencoding::decode(&link[idx + 1..]).unwrap_or(std::borrow::Cow::Borrowed("")).to_string()
        } else {
            "Unknown".to_string()
        };
        
        servers.push(serde_json::json!({
            "link": link,
            "remark": remark,
            "latency": serde_json::Value::Null
        }));
    }
    
    let group_name = url.split('/').nth(2).unwrap_or("Subscription").to_string();
    
    {
        let mut s = settings.lock().unwrap();
        s.settings.servers.insert(url.clone(), crate::core::settings::ServerGroup {
            name: group_name,
            servers,
        });
        let _ = s.save();
    }
    
    Ok(())
}

#[tauri::command]
pub async fn connect(link: String, tun_mode: bool, core: State<'_, Mutex<CoreManager>>, settings: State<'_, Mutex<SettingsManager>>) -> Result<(), String> {
    let port = 2080;
    let (config, exact_ip, is_xray) = generate_config(&link, port);
    
    if tun_mode && !CoreManager::is_admin() {
        return Err("Administrator privileges required for TUN mode".into());
    }

    {
        let mut c = core.lock().unwrap();
        c.start_core(config)?;
    }
    
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    {
        let mut c = core.lock().unwrap();
        if !c.is_alive() {
            c.stop_core(false);
            return Err("Core crashed immediately after startup.".into());
        }
        
        if tun_mode {
            c.clear_system_proxy();
            let tun_config = generate_tun_config(port, &link, exact_ip, is_xray);
            c.start_tun_core(tun_config)?;
        } else {
            c.stop_tun_core();
            c.set_system_proxy("127.0.0.1", port);
        }
    }
    
    {
        let mut s = settings.lock().unwrap();
        s.update_ui_state(Some(link), Some(tun_mode));
    }
    
    Ok(())
}

#[tauri::command]
pub fn disconnect(core: State<Mutex<CoreManager>>) -> Result<(), String> {
    let mut c = core.lock().unwrap();
    c.stop_core(true);
    c.stop_tun_core();
    Ok(())
}

#[tauri::command]
pub async fn check_ip(use_proxy: bool) -> Result<Value, String> {
    let mut builder = Client::builder().timeout(Duration::from_secs(10));
    if use_proxy {
        let proxy = reqwest::Proxy::all("socks5h://127.0.0.1:2080").map_err(|e| e.to_string())?;
        builder = builder.proxy(proxy);
    }
    
    let client = builder.build().map_err(|e| e.to_string())?;
    let resp: Value = client.get("http://ip-api.com/json")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
        
    Ok(resp)
}

#[tauri::command]
pub async fn test_all_latency(links: Vec<String>, _core: State<'_, Mutex<CoreManager>>) -> Result<Value, String> {
    let mut tasks = vec![];
    
    for link in links.into_iter() {
        let link_clone = link.clone();
        
        let task = tokio::spawn(async move {
            let start = tokio::time::Instant::now();
            if let Some((host, port)) = crate::core::config::extract_host_and_port(&link_clone) {
                if let Ok(addrs) = format!("{}:{}", host, port).to_socket_addrs() {
                    for addr in addrs {
                        if tokio::time::timeout(Duration::from_secs(3), tokio::net::TcpStream::connect(addr)).await.is_ok() {
                            return (link_clone, start.elapsed().as_millis() as u64);
                        }
                    }
                }
            }
            (link_clone, u64::MAX)
        });
        
        tasks.push(task);
    }
    
    let mut results = serde_json::Map::new();
    for task in tasks {
        if let Ok((link, latency)) = task.await {
            if latency == u64::MAX {
                results.insert(link, serde_json::json!(-1));
            } else {
                results.insert(link, serde_json::json!(latency));
            }
        }
    }
    
    Ok(serde_json::Value::Object(results))
}

#[tauri::command]
pub async fn test_single_latency(link: String, _core: State<'_, Mutex<CoreManager>>) -> Result<u64, String> {
    if let Some((host, port)) = crate::core::config::extract_host_and_port(&link) {
        if let Ok(addrs) = format!("{}:{}", host, port).to_socket_addrs() {
            let start = tokio::time::Instant::now();
            for addr in addrs {
                if tokio::time::timeout(Duration::from_secs(3), tokio::net::TcpStream::connect(addr)).await.is_ok() {
                    return Ok(start.elapsed().as_millis() as u64);
                }
            }
        }
    }
    Err("Timeout".into())
}

#[tauri::command]
pub async fn get_live_ping(link: String) -> Result<u64, String> {
    if let Some((host, port)) = crate::core::config::extract_host_and_port(&link) {
        if let Ok(addrs) = format!("{}:{}", host, port).to_socket_addrs() {
            let start = tokio::time::Instant::now();
            for addr in addrs {
                if tokio::time::timeout(Duration::from_secs(3), tokio::net::TcpStream::connect(addr)).await.is_ok() {
                    return Ok(start.elapsed().as_millis() as u64);
                }
            }
        }
    }
    Err("Timeout".into())
}

#[tauri::command]
pub fn toggle_tun(enabled: bool, settings: State<Mutex<SettingsManager>>) {
    settings.lock().unwrap().update_ui_state(None, Some(enabled));
}

#[tauri::command]
pub fn is_admin() -> bool {
    CoreManager::is_admin()
}

#[tauri::command]
pub fn clear_data(settings: State<Mutex<SettingsManager>>) -> Result<(), String> {
    let mut path = dirs::config_dir().unwrap_or_else(|| std::env::current_exe().unwrap_or_default().parent().unwrap().to_path_buf());
    path.push("Vxray");
    path.push("settings.json");
    
    {
        let mut s = settings.lock().unwrap();
        s.settings = crate::core::settings::Settings::default();
        let _ = s.save();
    }
    
    std::fs::remove_file(path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn is_core_alive(core: State<Mutex<CoreManager>>) -> bool {
    core.lock().unwrap().is_alive()
}

#[tauri::command]
pub fn get_tun_enabled(settings: State<Mutex<SettingsManager>>) -> bool {
    settings.lock().unwrap().settings.ui.tun_enabled
}

#[tauri::command]
pub fn get_startup_settings(settings: State<Mutex<SettingsManager>>) -> Value {
    let s = settings.lock().unwrap();
    serde_json::json!({
        "auto_reconnect": s.settings.ui.auto_reconnect,
        "auto_start": s.settings.ui.auto_start,
        "start_minimized": s.settings.ui.start_minimized,
    })
}

#[tauri::command]
pub fn set_auto_reconnect(enabled: bool, settings: State<Mutex<SettingsManager>>) {
    let mut s = settings.lock().unwrap();
    s.settings.ui.auto_reconnect = enabled;
    let _ = s.save();
}

#[tauri::command]
pub fn set_auto_start(enabled: bool, settings: State<Mutex<SettingsManager>>) {
    let mut s = settings.lock().unwrap();
    s.settings.ui.auto_start = enabled;
    let _ = s.save();

    // Manage Windows startup registry
    let exe_path = std::env::current_exe().unwrap_or_default();
    let exe_str = format!("\"{}\" --autostart", exe_path.to_string_lossy());

    if let Ok(hkcu) = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
        .open_subkey_with_flags(
            r"Software\Microsoft\Windows\CurrentVersion\Run",
            winreg::enums::KEY_SET_VALUE,
        )
    {
        if enabled {
            let _ = hkcu.set_value("Vxray", &exe_str);
        } else {
            let _ = hkcu.delete_value("Vxray");
        }
    }
}

#[tauri::command]
pub fn set_start_minimized(enabled: bool, settings: State<Mutex<SettingsManager>>) {
    let mut s = settings.lock().unwrap();
    s.settings.ui.start_minimized = enabled;
    let _ = s.save();
}

#[tauri::command]
pub fn save_last_connection(link: String, tun_mode: bool, settings: State<Mutex<SettingsManager>>) {
    let mut s = settings.lock().unwrap();
    s.settings.ui.last_connection_link = Some(link);
    s.settings.ui.last_connection_tun = tun_mode;
    let _ = s.save();
}

#[tauri::command]
pub fn get_auto_reconnect_info(settings: State<Mutex<SettingsManager>>) -> Value {
    let s = settings.lock().unwrap();
    serde_json::json!({
        "auto_reconnect": s.settings.ui.auto_reconnect,
        "link": s.settings.ui.last_connection_link,
        "tun_mode": s.settings.ui.last_connection_tun,
    })
}

#[tauri::command]
pub fn is_start_minimized(settings: State<Mutex<SettingsManager>>) -> bool {
    settings.lock().unwrap().settings.ui.start_minimized
        && settings.lock().unwrap().settings.ui.auto_start
}

#[tauri::command]
pub fn minimize_window(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.minimize();
    }
}

#[tauri::command]
pub fn maximize_window(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if let Ok(is_maximized) = window.is_maximized() {
            if is_maximized {
                let _ = window.unmaximize();
            } else {
                let _ = window.maximize();
            }
        }
    }
}

#[tauri::command]
pub fn close_window(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

#[tauri::command]
pub fn drag_window(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.start_dragging();
    }
}

#[tauri::command]
pub fn toggle_pin_subscription(group_key: String, settings: State<Mutex<SettingsManager>>) -> Vec<String> {
    settings.lock().unwrap().toggle_pin_subscription(&group_key)
}

#[tauri::command]
pub fn save_subscription_order(order: Vec<String>, settings: State<Mutex<SettingsManager>>) {
    settings.lock().unwrap().save_subscription_order(order);
}

#[tauri::command]
pub fn restart_as_admin() {
    let exe_path = std::env::current_exe().unwrap_or_default();
    if let Some(path_str) = exe_path.to_str() {
        let wide_path: Vec<u16> = std::ffi::OsStr::new(path_str).encode_wide().chain(std::iter::once(0)).collect();
        let wide_verb: Vec<u16> = std::ffi::OsStr::new("runas").encode_wide().chain(std::iter::once(0)).collect();
        
        unsafe {
            windows_sys::Win32::UI::Shell::ShellExecuteW(
                0,
                wide_verb.as_ptr(),
                wide_path.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                windows_sys::Win32::UI::WindowsAndMessaging::SW_NORMAL,
            );
        }
        std::process::exit(0);
    }
}
