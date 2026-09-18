use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct ServerStats {
    #[serde(default)]
    pub usage_history: Vec<(u64, u64)>, // Keeping for backwards compatibility
    #[serde(default)]
    pub daily_usage: std::collections::HashMap<String, u64>,
    #[serde(default)]
    pub ping_history: Vec<u64>,
    #[serde(default)]
    pub total_used: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ServerGroup {
    pub name: String,
    pub servers: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upload: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire: Option<u64>,
    #[serde(default = "default_true")]
    pub auto_update: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_interval: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub announcement: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct RoutingRule {
    pub id: String,
    pub pattern: String,
    pub action: String, // "direct", "block", "proxy"
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_remote_dns() -> String {
    "https://1.1.1.1/dns-query".to_string()
}

fn default_local_dns() -> String {
    "local".to_string()
}

fn default_core() -> String {
    "auto".to_string()
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
    #[serde(default = "default_true")]
    pub live_ping_enabled: bool,
    #[serde(default)]
    pub last_connection_link: Option<String>,
    #[serde(default)]
    pub last_connection_tun: bool,
    #[serde(default)]
    pub pinned_subscriptions: Vec<String>,
    #[serde(default)]
    pub subscription_order: Vec<String>,
    #[serde(default = "default_true")]
    pub auto_update_subs: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Settings {
    #[serde(default)]
    pub stats: std::collections::HashMap<String, ServerStats>,
    #[serde(default)]
    pub routing_rules: Vec<RoutingRule>,
    #[serde(default = "default_true")]
    pub bypass_iran: bool,
    #[serde(default = "default_true")]
    pub bypass_lan: bool,
    #[serde(default)]
    pub block_ads: bool,
    #[serde(default = "default_remote_dns")]
    pub remote_dns: String,
    #[serde(default = "default_local_dns")]
    pub local_dns: String,
    #[serde(default)]
    pub tls_fragment: bool,
    #[serde(default = "default_true")]
    pub strict_route: bool,
    #[serde(default = "default_core")]
    pub core_choice: String,
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
                upload: None,
                download: None,
                total: None,
                expire: None,
                auto_update: false,
                update_interval: None,
                announcement: None,
            },
        );

        Settings {
            stats: std::collections::HashMap::new(),
            routing_rules: vec![],
            bypass_iran: true,
            bypass_lan: true,
            block_ads: false,
            remote_dns: "https://1.1.1.1/dns-query".to_string(),
            local_dns: "local".to_string(),
            tls_fragment: false,
            strict_route: true,
            core_choice: "auto".to_string(),
            subscriptions: vec![],
            servers,
            ui: UiState {
                last_selected: None,
                tun_enabled: false,
                auto_reconnect: false,
                auto_start: false,
                start_minimized: false,
                live_ping_enabled: true,
                last_connection_link: None,
                last_connection_tun: false,
                pinned_subscriptions: vec![],
                subscription_order: vec![],
                auto_update_subs: true,
            },
        }
    }
}

pub struct SettingsManager {
    pub file_path: PathBuf,
    pub settings: Settings,
}

impl SettingsManager {
    pub fn new() -> Self {
        let mut custom_dir = None;
        let args: Vec<String> = std::env::args().collect();
        if let Some(idx) = args.iter().position(|a| a == "--config-dir") {
            if idx + 1 < args.len() {
                custom_dir = Some(std::path::PathBuf::from(&args[idx + 1]));
            }
        }

        let mut file_path = custom_dir.unwrap_or_else(|| {
            let mut p = dirs::config_dir().unwrap_or_else(|| {
                std::env::current_exe()
                    .unwrap_or_default()
                    .parent()
                    .unwrap()
                    .to_path_buf()
            });
            p.push("Vxray");
            p
        });
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
        if self
            .settings
            .ui
            .pinned_subscriptions
            .contains(&group_key.to_string())
        {
            self.settings
                .ui
                .pinned_subscriptions
                .retain(|k| k != group_key);
        } else {
            self.settings
                .ui
                .pinned_subscriptions
                .push(group_key.to_string());
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
        if !self.settings.servers.contains_key("manual") {
            self.settings.servers.insert(
                "manual".to_string(),
                ServerGroup {
                    name: "Manually Added".to_string(),
                    servers: vec![],
                    upload: None,
                    download: None,
                    total: None,
                    expire: None,
                    auto_update: false,
                    update_interval: None,
                    announcement: None,
                },
            );
        }
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
        let mut list = Vec::new();
        for (url, group) in &self.settings.servers {
            if url == "manual" {
                continue;
            }
            let is_pinned = self.settings.ui.pinned_subscriptions.contains(url);
            let mut stats = self.settings.stats.get(url).cloned().unwrap_or_default();
            
            let mut all_pings = Vec::new();
            for server in &group.servers {
                if let Some(link) = server.get("link").and_then(|v| v.as_str()) {
                    if let Some(server_stat) = self.settings.stats.get(link) {
                        all_pings.extend(server_stat.ping_history.clone());
                    }
                }
            }
            if !all_pings.is_empty() {
                stats.ping_history = all_pings;
            }

            list.push(json!({
                "url": url,
                "name": group.name,
                "servers_count": group.servers.len(),
                "upload": group.upload,
                "download": group.download,
                "total": group.total,
                "expire": group.expire,
                "auto_update": group.auto_update,
                "update_interval": group.update_interval.unwrap_or(24),
                "pinned": is_pinned,
                "stats": stats,
                "announcement": &group.announcement,
            }));
        }
        json!(list)
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
