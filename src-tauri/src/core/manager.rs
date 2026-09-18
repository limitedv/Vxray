use serde_json::Value;
use std::os::windows::process::CommandExt;
use std::process::Command;
use winreg::enums::*;
use winreg::RegKey;

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub struct CoreManager {
    pub process: Option<std::process::Child>,
    pub tracker_process: Option<std::process::Child>,
    pub tun_process: Option<std::process::Child>,
    pub test_process: Option<std::process::Child>,
    pub bin_dir: std::path::PathBuf,
    pub data_dir: std::path::PathBuf,
    pub current_server_ip: Option<String>,
}

impl CoreManager {
    pub fn new() -> Self {
        let app_dir = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
            
        let mut bin_dir = app_dir.join("bin");
        if !bin_dir.exists() {
            let tauri_up_bin = app_dir.join("_up_").join("bin");
            if tauri_up_bin.exists() {
                bin_dir = tauri_up_bin;
            } else {
                let current_dir = std::env::current_dir().unwrap_or_default();
                let dev_bin = current_dir.join("bin");
                if dev_bin.exists() {
                    bin_dir = dev_bin;
                } else if current_dir.parent().unwrap().join("bin").exists() {
                    bin_dir = current_dir.parent().unwrap().join("bin");
                }
            }
        }
        
        let mut data_dir = dirs::config_dir().unwrap_or_else(|| app_dir.clone());
        data_dir.push("Vxray");
        data_dir.push("data");

        if !data_dir.exists() {
            let _ = std::fs::create_dir_all(&data_dir);
        }

        Self {
            process: None,
            tracker_process: None,
            tun_process: None,
            test_process: None,
            bin_dir,
            data_dir,
            current_server_ip: None,
        }
    }

    pub fn is_admin() -> bool {
        unsafe { windows_sys::Win32::UI::Shell::IsUserAnAdmin() != 0 }
    }

    pub fn is_tracker_running(&self) -> bool {
        self.tracker_process.is_some()
    }

    pub fn get_tun_enabled(&self) -> bool {
        self.tun_process.is_some()
    }

    fn allow_firewall(exe_path: &std::path::Path) {
        if !Self::is_admin() {
            return;
        }
        if let Some(path_str) = exe_path.to_str() {
            let rule_name = format!(
                "Vxray Core ({})",
                exe_path.file_name().unwrap_or_default().to_string_lossy()
            );
            // Add inbound rule
            let _ = Command::new("netsh")
                .args([
                    "advfirewall",
                    "firewall",
                    "add",
                    "rule",
                    &format!("name={}", rule_name),
                    "dir=in",
                    "action=allow",
                    &format!("program={}", path_str),
                    "enable=yes",
                ])
                .creation_flags(CREATE_NO_WINDOW)
                .output();
            // Add outbound rule
            let _ = Command::new("netsh")
                .args([
                    "advfirewall",
                    "firewall",
                    "add",
                    "rule",
                    &format!("name={}", rule_name),
                    "dir=out",
                    "action=allow",
                    &format!("program={}", path_str),
                    "enable=yes",
                ])
                .creation_flags(CREATE_NO_WINDOW)
                .output();
        }
    }

    pub fn is_alive(&mut self) -> bool {
        let main_alive = if let Some(p) = self.process.as_mut() {
            matches!(p.try_wait(), Ok(None))
        } else {
            false
        };

        if !main_alive {
            return false;
        }

        // Check TUN process if it exists (critical for TUN mode)
        if let Some(p) = self.tun_process.as_mut() {
            if !matches!(p.try_wait(), Ok(None)) {
                return false;
            }
        }

        true
    }

    fn refresh_wininet(&self) {
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(10));
            unsafe {
                windows_sys::Win32::Networking::WinInet::InternetSetOptionW(
                    std::ptr::null_mut(),
                    windows_sys::Win32::Networking::WinInet::INTERNET_OPTION_SETTINGS_CHANGED,
                    std::ptr::null_mut(),
                    0,
                );
                windows_sys::Win32::Networking::WinInet::InternetSetOptionW(
                    std::ptr::null_mut(),
                    windows_sys::Win32::Networking::WinInet::INTERNET_OPTION_REFRESH,
                    std::ptr::null_mut(),
                    0,
                );
            }
        });
    }

    pub fn set_system_proxy(&self, host: &str, port: u16) {
        if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings",
            KEY_WRITE,
        ) {
            let proxy_server = format!("{}:{}", host, port);
            let _ = hkcu.set_value("ProxyEnable", &1u32);
            let _ = hkcu.set_value("ProxyServer", &proxy_server);
            let _ = hkcu.set_value("ProxyOverride", &"<local>");
            let _ = hkcu.delete_value("AutoConfigURL");
            self.refresh_wininet();
        }
    }

    pub fn clear_system_proxy(&self) {
        if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings",
            KEY_WRITE,
        ) {
            let _ = hkcu.set_value("ProxyEnable", &0u32);
            let _ = hkcu.delete_value("ProxyServer");
            self.refresh_wininet();
        }
    }

    pub fn kill_all_cores(&self) {
        let _ = Command::new("taskkill")
            .args(["/F", "/IM", "sing-box.exe", "/T"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        let _ = Command::new("taskkill")
            .args(["/F", "/IM", "xray.exe", "/T"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
            
        let _ = Command::new("taskkill")
            .args(["/F", "/IM", "vxray-tun.exe", "/T"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
            
        let _ = Command::new("taskkill")
            .args(["/F", "/IM", "stat-tracker.exe", "/T"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
    }

    pub fn start_core(&mut self, mut config: Value) -> Result<(), String> {
        let mut exe_name = "sing-box.exe";
        if let Some(core_type) = config.get("__core__").and_then(|v| v.as_str()) {
            if core_type == "xray" {
                exe_name = "xray.exe";
            }
        }
        if let Some(obj) = config.as_object_mut() {
            obj.remove("__core__");
        }

        let config_path = self.data_dir.join("config.json");
        std::fs::write(&config_path, config.to_string()).map_err(|e| e.to_string())?;

        let log_file =
            std::fs::File::create(self.data_dir.join("core.log")).map_err(|e| e.to_string())?;

        let exe_path = self.bin_dir.join(exe_name);
        Self::allow_firewall(&exe_path);

        let mut cmd = Command::new(&exe_path);
        let child = cmd
            .current_dir(&self.bin_dir)
            .args(["run", "-c", &config_path.to_string_lossy()])
            .stdout(log_file.try_clone().unwrap())
            .stderr(log_file)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| e.to_string())?;

        self.process = Some(child);
        Ok(())
    }

    pub fn take_core_process(&mut self) -> Option<std::process::Child> {
        self.process.take()
    }

    pub fn clear_proxy_if(&self, clear_proxy: bool) {
        if clear_proxy {
            self.clear_system_proxy();
        }
    }

    pub fn start_test_core(&mut self, mut config: Value) -> Result<(), String> {
        self.stop_test_core();

        let mut exe_name = "sing-box.exe";
        if let Some(core_type) = config.get("__core__").and_then(|v| v.as_str()) {
            if core_type == "xray" {
                exe_name = "xray.exe";
            }
        }
        if let Some(obj) = config.as_object_mut() {
            obj.remove("__core__");
        }

        let config_path = self.data_dir.join("test_config.json");
        std::fs::write(&config_path, config.to_string()).map_err(|e| e.to_string())?;

        let exe_path = self.bin_dir.join(exe_name);
        let mut cmd = Command::new(&exe_path);
        let child = cmd
            .current_dir(&self.bin_dir)
            .args(["run", "-c", &config_path.to_string_lossy()])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| e.to_string())?;

        self.test_process = Some(child);
        Ok(())
    }

    pub fn stop_test_core(&mut self) {
        if let Some(mut p) = self.test_process.take() {
            std::thread::spawn(move || {
                let _ = p.kill();
                let _ = p.wait();
            });
        }
    }

    pub fn start_tun_core(&mut self, config: Value, _is_xray: bool) -> Result<(), String> {
        let config_path = self.data_dir.join("tun_config.json");
        std::fs::write(&config_path, config.to_string()).map_err(|e| e.to_string())?;

        let log_file =
            std::fs::File::create(self.data_dir.join("tun.log")).map_err(|e| e.to_string())?;

        let tun_path = self.data_dir.join("vxray-tun.exe");
        let source_bin = self.bin_dir.join("sing-box.exe");
        if !tun_path.exists() || std::fs::metadata(&tun_path).map(|m| m.len()).unwrap_or(0) != std::fs::metadata(&source_bin).map(|m| m.len()).unwrap_or(1) {
            let _ = std::fs::copy(&source_bin, &tun_path);
        }
        let exe_path = tun_path;
        Self::allow_firewall(&exe_path);

        let mut cmd = Command::new(&exe_path);
        let child = cmd
            .current_dir(&self.bin_dir)
            .env("ENABLE_DEPRECATED_TUN_ADDRESS_X", "1")
            .args(["run", "-c", &config_path.to_string_lossy()])
            .stdout(log_file.try_clone().unwrap())
            .stderr(log_file)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| e.to_string())?;

        self.tun_process = Some(child);
        Ok(())
    }

    pub fn take_tun_process(&mut self) -> Option<std::process::Child> {
        self.tun_process.take()
    }

    pub fn start_tracker_core(&mut self, config: Value) -> Result<(), String> {
        if let Some(mut old) = self.tracker_process.take() {
            let _ = old.kill();
            let _ = old.wait();
        }
        let config_path = self.data_dir.join("tracker_config.json");
        std::fs::write(&config_path, config.to_string()).map_err(|e| e.to_string())?;

        let exe_path = self.data_dir.join("stat-tracker.exe");
        let source_bin = self.bin_dir.join("sing-box.exe");
        if !exe_path.exists() || std::fs::metadata(&exe_path).map(|m| m.len()).unwrap_or(0) != std::fs::metadata(&source_bin).map(|m| m.len()).unwrap_or(1) {
            let _ = std::fs::copy(&source_bin, &exe_path);
        }
        Self::allow_firewall(&exe_path);

        let tracker_path = exe_path.clone();
        let mut cmd = Command::new(&tracker_path);
        let child = cmd
            .current_dir(&self.bin_dir)
            .args(["run", "-c", &config_path.to_string_lossy()])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| e.to_string())?;

        self.tracker_process = Some(child);
        Ok(())
    }

    pub fn take_tracker_process(&mut self) -> Option<std::process::Child> {
        self.tracker_process.take()
    }
}
