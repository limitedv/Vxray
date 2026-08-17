use serde_json::Value;
use std::os::windows::process::CommandExt;
use std::process::{Child, Command};
use std::path::PathBuf;
use winreg::enums::*;
use winreg::RegKey;

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub struct CoreManager {
    process: Option<Child>,
    test_process: Option<Child>,
    tun_process: Option<Child>,
    bin_dir: PathBuf,
    data_dir: PathBuf,
}

impl CoreManager {
    pub fn new() -> Self {
        let mut data_dir = dirs::config_dir().unwrap_or_else(|| std::env::current_exe().unwrap_or_default().parent().unwrap().to_path_buf());
        data_dir.push("Vxray");
        let _ = std::fs::create_dir_all(&data_dir);
        
        let mut cm = Self {
            process: None,
            test_process: None,
            tun_process: None,
            data_dir,
            bin_dir: std::env::current_exe()
                .unwrap_or_default()
                .parent()
                .unwrap()
                .join("../../../bin"), // Will adjust based on where the app runs, usually we need to find root bin
        };
        // Best effort to find bin dir
        let mut path = std::env::current_exe().unwrap_or_default();
        while path.pop() {
            if path.join("bin").join("sing-box.exe").exists() {
                cm.bin_dir = path.join("bin");
                break;
            }
            // Tauri places parent-directory resources into a _up_ folder upon installation
            if path.join("_up_").join("bin").join("sing-box.exe").exists() {
                cm.bin_dir = path.join("_up_").join("bin");
                break;
            }
        }
        
        cm.kill_all_cores();
        cm
    }

    pub fn is_admin() -> bool {
        unsafe {
            windows_sys::Win32::UI::Shell::IsUserAnAdmin() != 0
        }
    }

    pub fn get_tun_enabled(&self) -> bool {
        self.tun_process.is_some()
    }

    fn allow_firewall(exe_path: &std::path::Path) {
        if !Self::is_admin() {
            return;
        }
        if let Some(path_str) = exe_path.to_str() {
            let rule_name = format!("Vxray Core ({})", exe_path.file_name().unwrap_or_default().to_string_lossy());
            // Add inbound rule
            let _ = Command::new("netsh")
                .args(["advfirewall", "firewall", "add", "rule", &format!("name={}", rule_name), "dir=in", "action=allow", &format!("program={}", path_str), "enable=yes"])
                .creation_flags(CREATE_NO_WINDOW)
                .output();
            // Add outbound rule
            let _ = Command::new("netsh")
                .args(["advfirewall", "firewall", "add", "rule", &format!("name={}", rule_name), "dir=out", "action=allow", &format!("program={}", path_str), "enable=yes"])
                .creation_flags(CREATE_NO_WINDOW)
                .output();
        }
    }

    pub fn is_alive(&mut self) -> bool {
        if let Some(p) = self.process.as_mut() {
            p.try_wait().map(|status| status.is_none()).unwrap_or(false)
        } else {
            false
        }
    }

    fn refresh_wininet(&self) {
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
            self.refresh_wininet();
        }
    }

    pub fn kill_all_cores(&self) {
        let _ = Command::new("taskkill")
            .args(["/F", "/IM", "sing-box.exe", "/T"])
            .creation_flags(CREATE_NO_WINDOW)
            .status();
            
        let _ = Command::new("taskkill")
            .args(["/F", "/IM", "xray.exe", "/T"])
            .creation_flags(CREATE_NO_WINDOW)
            .status();
    }

    pub fn start_core(&mut self, mut config: Value) -> Result<(), String> {
        self.stop_core(false);
        
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
        std::fs::write(&config_path, config.to_string())
            .map_err(|e| e.to_string())?;

        let log_file = std::fs::File::create(self.data_dir.join("core.log")).map_err(|e| e.to_string())?;

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

    pub fn stop_core(&mut self, clear_proxy: bool) {
        if let Some(mut p) = self.process.take() {
            let _ = p.kill();
        }
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
        std::fs::write(&config_path, config.to_string())
            .map_err(|e| e.to_string())?;

        let exe_path = self.bin_dir.join(exe_name);
        let child = Command::new(&exe_path)
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
            let _ = p.kill();
        }
    }

    pub fn start_tun_core(&mut self, config: Value) -> Result<(), String> {
        self.stop_tun_core();
        
        let config_path = self.data_dir.join("tun_config.json");
        std::fs::write(&config_path, config.to_string())
            .map_err(|e| e.to_string())?;

        let log_file = std::fs::File::create(self.data_dir.join("tun.log")).map_err(|e| e.to_string())?;

        let exe_path = self.bin_dir.join("sing-box.exe");
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

    pub fn stop_tun_core(&mut self) {
        if let Some(mut p) = self.tun_process.take() {
            let _ = p.kill();
        }
    }
}
