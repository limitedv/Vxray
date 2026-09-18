#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod core;

use core::manager::CoreManager;
use core::settings::SettingsManager;
use std::os::windows::process::CommandExt;
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

fn setup_job_object() {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::JobObjects::{
            CreateJobObjectW, SetInformationJobObject, JobObjectExtendedLimitInformation,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            AssignProcessToJobObject, JOB_OBJECT_LIMIT_BREAKAWAY_OK
        };
        use windows_sys::Win32::System::Threading::GetCurrentProcess;

        let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if job != 0 {
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_BREAKAWAY_OK;
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            AssignProcessToJobObject(job, GetCurrentProcess());
        }
    }
}

fn check_single_instance() -> bool {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;
    
    if let Ok(mut stream) = TcpStream::connect_timeout(&"127.0.0.1:51337".parse().unwrap(), Duration::from_millis(500)) {
        let _ = stream.set_write_timeout(Some(Duration::from_millis(500)));
        let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
        if stream.write_all(b"WAKEUP_VXRAY").is_ok() {
            let mut buf = [0u8; 32];
            if let Ok(sz) = stream.read(&mut buf) {
                let response = String::from_utf8_lossy(&buf[..sz]);
                if response.starts_with("OK:") {
                    if let Ok(pid) = response[3..].parse::<u32>() {
                        #[cfg(windows)]
                        unsafe {
                            windows_sys::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow(pid);
                        }
                    }
                    return true;
                }
            }
        }
    }
    false
}

fn main() {
    if check_single_instance() {
        std::process::exit(0);
    }
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--elevator") {
        // We are the invisible background elevator process.
        // Wait exactly 500ms for the parent process (which spawned us) to fully exit 
        // and release its `single_instance` lock.
        std::thread::sleep(std::time::Duration::from_millis(500));
        
        let exe = std::env::current_exe().unwrap_or_default();
        use std::os::windows::ffi::OsStrExt;
        let exe_wide: Vec<u16> = exe.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
        let verb: Vec<u16> = std::ffi::OsStr::new("runas").encode_wide().chain(std::iter::once(0)).collect();
        
        unsafe {
            // Must initialize COM for ShellExecuteW
            let _ = windows_sys::Win32::System::Com::CoInitializeEx(
                std::ptr::null(),
                (windows_sys::Win32::System::Com::COINIT_APARTMENTTHREADED | windows_sys::Win32::System::Com::COINIT_DISABLE_OLE1DDE) as u32
            );

            windows_sys::Win32::UI::Shell::ShellExecuteW(
                0 as isize,
                verb.as_ptr(),
                exe_wide.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL as i32,
            );
        }
        std::process::exit(0);
    }

    setup_job_object();

    let core_manager = CoreManager::new();
    let settings_manager = SettingsManager::new();

    // Sync autostart registry key on manual startup to repair any profile desyncs
    let is_autostart_launch = std::env::args().any(|arg| arg == "--autostart");
    if !is_autostart_launch {
        let task_name = "VxrayAutoStart";
        if settings_manager.settings.ui.auto_start {
            let exe_path = std::env::current_exe().unwrap_or_default();
            let exe_str = format!("\"{}\" --autostart", exe_path.to_string_lossy());
            if let Ok(hkcu) = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
                .open_subkey_with_flags(
                    r"Software\Microsoft\Windows\CurrentVersion\Run",
                    winreg::enums::KEY_SET_VALUE,
                )
            {
                let _ = hkcu.set_value("Vxray", &exe_str);
            }
            let _ = std::process::Command::new("schtasks")
                .args(&[
                    "/Create", "/F", "/TN", task_name, "/TR", &exe_str, "/SC", "ONLOGON", "/RL",
                    "HIGHEST",
                ])
                .creation_flags(0x08000000)
                .output();
        } else {
            if let Ok(hkcu) = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
                .open_subkey_with_flags(
                    r"Software\Microsoft\Windows\CurrentVersion\Run",
                    winreg::enums::KEY_SET_VALUE,
                )
            {
                let _ = hkcu.delete_value("Vxray");
            }
            let _ = std::process::Command::new("schtasks")
                .args(&["/Delete", "/F", "/TN", task_name])
                .creation_flags(0x08000000)
                .output();
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_shell::init())
        .manage(Mutex::new(core_manager))
        .manage(Mutex::new(settings_manager))
        .setup(|app| {
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                use std::io::{Read, Write};
                let listener = std::net::TcpListener::bind("127.0.0.1:51337");
                if let Ok(listener) = listener {
                    for stream in listener.incoming() {
                        if let Ok(mut s) = stream {
                            let mut buf = [0u8; 12];
                            let _ = s.set_read_timeout(Some(std::time::Duration::from_millis(500)));
                            if s.read_exact(&mut buf).is_ok() && &buf == b"WAKEUP_VXRAY" {
                                let pid = std::process::id();
                                let response = format!("OK:{}!", pid);
                                let _ = s.write_all(response.as_bytes());
                                
                                let app_clone = app_handle.clone();
                                tauri::async_runtime::spawn(async move {
                                    if let Some(window) = app_clone.get_webview_window("main") {
                                        let _ = window.show();
                                        let _ = window.unminimize();
                                        let _ = window.set_focus();
                                    }
                                });
                            }
                        }
                    }
                }
            });

            // Cleanup orphaned cores and proxy settings. Done here so secondary instances (which exit early) don't trigger it.
            {
                let state = app.state::<Mutex<CoreManager>>();
                let core = state.lock().unwrap();
                core.kill_all_cores();
                core.clear_system_proxy();
            }

            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        let state = app.state::<Mutex<CoreManager>>();
                        let (core_p, tracker_p, tun_p) = {
                            let mut core = state.lock().unwrap();
                            core.clear_proxy_if(true);
                            (
                                core.take_core_process(),
                                core.take_tracker_process(),
                                core.take_tun_process(),
                            )
                        };
                        if let Some(mut p) = core_p {
                            let _ = p.kill();
                            let _ = p.wait();
                        }
                        if let Some(mut p) = tracker_p {
                            let _ = p.kill();
                            let _ = p.wait();
                        }
                        if let Some(mut p) = tun_p {
                            let _ = p.kill();
                            let _ = p.wait();
                        }
                        std::process::exit(0);
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
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
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Handle start-minimized: if auto_start + start_minimized are both on AND launched via --autostart, hide the window
            {
                let settings_state = app.state::<Mutex<SettingsManager>>();
                let s = settings_state.lock().unwrap();
                let is_autostart_launch = std::env::args().any(|arg| arg == "--autostart");

                let should_hide = s.settings.ui.auto_start
                    && s.settings.ui.start_minimized
                    && is_autostart_launch;
                if !should_hide {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.unminimize();
                        let _ = window.set_focus();
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
            commands::get_item_stats,
            commands::switch_tun_mode,
            commands::delete_manual_server,
            commands::get_routing_rules,
            commands::add_live_traffic,
            commands::get_routing_settings,
            commands::add_routing_rule,
            commands::delete_routing_rule,
            commands::toggle_routing_rule,
            commands::update_routing_preset,
            commands::update_dns_settings,
            commands::is_start_minimized,
            commands::minimize_window,
            commands::close_window,
            commands::drag_window,
            commands::get_servers,
            commands::add_server,
            commands::add_servers,
            commands::get_subscriptions,
            commands::add_subscription,
            commands::toggle_auto_update,
            commands::delete_server,
            commands::edit_server,
            commands::delete_subscription,
            commands::edit_subscription,
            commands::update_subscription,
            commands::connect,
            commands::disconnect,
            commands::check_ip,
            commands::test_all_latency,
            commands::test_single_latency,
            commands::cancel_latency_tests,
            commands::start_latency_tests,
            commands::get_live_ping,
            commands::maximize_window,
            commands::set_auto_update_subs,
            commands::set_live_ping_enabled,
            commands::set_auto_reconnect,
            commands::set_auto_start,
            commands::set_start_minimized,
            commands::save_last_connection,
            commands::get_auto_reconnect_info,
            commands::update_core_settings,
            commands::get_core_settings,
            commands::restart_as_admin,
            commands::toggle_tun,
            commands::is_admin,
            commands::clear_data,
            commands::is_core_alive,
            commands::get_tun_enabled,
            commands::get_startup_settings,
            commands::toggle_pin_subscription,
            commands::save_subscription_order,
            commands::save_server_order,
            commands::is_tracker_running,
            commands::copy_to_clipboard,
            commands::read_from_clipboard,
            commands::clear_manual_servers
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
