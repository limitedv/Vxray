use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ServerGroup {
    pub name: String,
    pub servers: Vec<Value>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UiState {
    pub last_selected: Option<String>,
    pub tun_enabled: bool,
    #[serde(default)]
    pub auto_reconnect: bool,
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default)]
    pub start_minimized: bool,
    #[serde(default)]
    pub last_connection_link: Option<String>,
    #[serde(default)]
    pub last_connection_tun: bool,
    #[serde(default)]
    pub pinned_subscriptions: Vec<String>,
    #[serde(default)]
    pub subscription_order: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Settings {
    pub subscriptions: Vec<Value>,
    pub servers: std::collections::HashMap<String, ServerGroup>,
    pub ui: UiState,
}

impl Default for Settings {
    fn default() -> Self {
        let mut servers = std::collections::HashMap::new();
        servers.insert(
            "manual".to_string(),
            ServerGroup {
                name: "Manually Added".to_string(),
                servers: vec![],
            },
        );

        Settings {
            subscriptions: vec![],
            servers,
            ui: UiState {
                last_selected: None,
                tun_enabled: false,
                auto_reconnect: false,
                auto_start: false,
                start_minimized: false,
                last_connection_link: None,
                last_connection_tun: false,
                pinned_subscriptions: vec![],
                subscription_order: vec![],
            },
        }
    }
}

pub struct SettingsManager {
    file_path: PathBuf,
    pub settings: Settings,
}

impl SettingsManager {
    pub fn new() -> Self {
        let mut file_path = dirs::config_dir().unwrap_or_else(|| std::env::current_exe().unwrap_or_default().parent().unwrap().to_path_buf());
        file_path.push("Vxray");
        let _ = fs::create_dir_all(&file_path);
        file_path.push("settings.json");

        let settings = if file_path.exists() {
            if let Ok(content) = fs::read_to_string(&file_path) {
                serde_json::from_str(&content).unwrap_or_else(|_| Settings::default())
            } else {
                Settings::default()
            }
        } else {
            Settings::default()
        };

        Self {
            file_path,
            settings,
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let content = serde_json::to_string_pretty(&self.settings)
            .map_err(|e| format!("Serialization error: {}", e))?;
        fs::write(&self.file_path, content)
            .map_err(|e| format!("Failed to save settings: {}", e))?;
        Ok(())
    }

    pub fn get_server_groups(&self) -> Value {
        json!({
            "groups": &self.settings.servers,
            "pinned": &self.settings.ui.pinned_subscriptions,
            "order": &self.settings.ui.subscription_order,
        })
    }

    pub fn toggle_pin_subscription(&mut self, group_key: &str) -> Vec<String> {
        if self.settings.ui.pinned_subscriptions.contains(&group_key.to_string()) {
            self.settings.ui.pinned_subscriptions.retain(|k| k != group_key);
        } else {
            self.settings.ui.pinned_subscriptions.push(group_key.to_string());
        }
        let _ = self.save();
        self.settings.ui.pinned_subscriptions.clone()
    }

    pub fn save_subscription_order(&mut self, order: Vec<String>) {
        self.settings.ui.subscription_order = order;
        let _ = self.save();
    }

    pub fn set_server_groups(&mut self, groups: std::collections::HashMap<String, ServerGroup>) {
        self.settings.servers = groups;
        let _ = self.save();
    }

    pub fn add_manual_server(&mut self, link: String, remark: String) {
        if let Some(group) = self.settings.servers.get_mut("manual") {
            let server_obj = json!({
                "link": link,
                "remark": remark,
            });
            group.servers.push(server_obj);
            let _ = self.save();
        }
    }

    pub fn get_subscriptions(&self) -> Value {
        json!(&self.settings.subscriptions)
    }

    pub fn add_subscription(&mut self, url: String) {
        self.settings.subscriptions.push(json!({"url": url}));
        let _ = self.save();
    }

    pub fn delete_subscription(&mut self, group_key: &str) {
        // Remove from subscriptions list
        self.settings.subscriptions.retain(|sub| {
            if let Some(url) = sub.get("url").and_then(|v| v.as_str()) {
                url != group_key
            } else {
                true
            }
        });
        // Remove server group
        self.settings.servers.remove(group_key);
        let _ = self.save();
    }

    pub fn update_ui_state(&mut self, last_selected: Option<String>, tun_enabled: Option<bool>) {
        if let Some(ls) = last_selected {
            self.settings.ui.last_selected = Some(ls);
        }
        if let Some(te) = tun_enabled {
            self.settings.ui.tun_enabled = te;
        }
        let _ = self.save();
    }
}
