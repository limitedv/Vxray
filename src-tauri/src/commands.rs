fn decode_base64_forgiving(s: &str) -> Result<Vec<u8>, base64::DecodeError> {
    use base64::Engine;
    let mut padded = s.trim().to_string();
    while padded.len() % 4 != 0 {
        padded.push('=');
    }
    base64::engine::general_purpose::STANDARD.decode(&padded)
}

#[tauri::command]
pub fn delete_manual_server(
    link: String,
    settings: tauri::State<std::sync::Mutex<crate::core::settings::SettingsManager>>,
) {
    let mut s = settings.lock().unwrap();
    if let Some(manual) = s.settings.servers.get_mut("manual") {
        manual.servers.retain(|v| v.as_str() != Some(&link));
    }
    let _ = s.save();
}

#[tauri::command]
pub fn get_routing_rules(
    settings: tauri::State<std::sync::Mutex<crate::core::settings::SettingsManager>>,
) -> Result<serde_json::Value, String> {
    let s = settings.lock().unwrap();
    Ok(json!(s.settings.routing_rules))
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn get_item_stats(
    is_group: bool,
    key: String,
    settings: tauri::State<std::sync::Mutex<crate::core::settings::SettingsManager>>,
) -> Result<serde_json::Value, String> {
    let s = settings.lock().unwrap();
    let mut stat = s.settings.stats.get(&key).cloned().unwrap_or_default();
    
    if is_group {
        if let Some(group) = s.settings.servers.get(&key) {
            let mut all_pings = Vec::new();
            for server in &group.servers {
                if let Some(link) = server.get("link").and_then(|v| v.as_str()) {
                    if let Some(server_stat) = s.settings.stats.get(link) {
                        all_pings.extend(server_stat.ping_history.clone());
                    }
                }
            }
            if !all_pings.is_empty() {
                stat.ping_history = all_pings;
            }
        }
    }
    
    Ok(json!(stat))
}

#[tauri::command]
pub fn add_live_traffic(
    link: String,
    group: Option<String>,
    speed: u64,
    date: String,
    settings: tauri::State<std::sync::Mutex<crate::core::settings::SettingsManager>>,
) -> Result<serde_json::Value, String> {
    let mut s = settings.lock().unwrap();
    let mut server_total = 0;
    let mut group_total = 0;
    
    // Add to specific server link
    {
        let stat = s
            .settings
            .stats
            .entry(link)
            .or_insert_with(crate::core::settings::ServerStats::default);
        stat.total_used += speed;
        server_total = stat.total_used;
        let daily = stat.daily_usage.entry(date.clone()).or_insert(0);
        *daily += speed;
        stat.usage_history.clear();
    }

    // Add to group link if available
    if let Some(g) = group {
        let stat = s
            .settings
            .stats
            .entry(g)
            .or_insert_with(crate::core::settings::ServerStats::default);
        stat.total_used += speed;
        group_total = stat.total_used;
        let daily = stat.daily_usage.entry(date).or_insert(0);
        *daily += speed;
        stat.usage_history.clear();
    }

    if speed > 0 {
        let _ = s.save();
    }
    
    Ok(serde_json::json!({
        "server_total": server_total,
        "group_total": group_total
    }))
}

#[tauri::command]
pub fn get_routing_settings(
    settings: tauri::State<std::sync::Mutex<crate::core::settings::SettingsManager>>,
) -> Result<serde_json::Value, String> {
    let s = settings.lock().unwrap();
    Ok(json!({
        "bypass_iran": s.settings.bypass_iran,
        "bypass_lan": s.settings.bypass_lan,
        "block_ads": s.settings.block_ads
    }))
}

#[tauri::command]
pub fn add_routing_rule(
    rule: crate::core::settings::RoutingRule,
    settings: tauri::State<std::sync::Mutex<crate::core::settings::SettingsManager>>,
) -> Result<(), String> {
    let mut s = settings.lock().unwrap();
    s.settings.routing_rules.push(rule);
    let _ = s.save();
    Ok(())
}

#[tauri::command]
pub fn delete_routing_rule(
    id: String,
    settings: tauri::State<std::sync::Mutex<crate::core::settings::SettingsManager>>,
) -> Result<(), String> {
    let mut s = settings.lock().unwrap();
    s.settings.routing_rules.retain(|r| r.id != id);
    let _ = s.save();
    Ok(())
}

#[tauri::command]
pub fn toggle_routing_rule(
    id: String,
    enabled: bool,
    settings: tauri::State<std::sync::Mutex<crate::core::settings::SettingsManager>>,
) -> Result<(), String> {
    let mut s = settings.lock().unwrap();
    if let Some(rule) = s.settings.routing_rules.iter_mut().find(|r| r.id == id) {
        rule.enabled = enabled;
        let _ = s.save();
    }
    Ok(())
}

#[tauri::command]
pub fn update_routing_preset(
    preset: String,
    enabled: bool,
    settings: tauri::State<std::sync::Mutex<crate::core::settings::SettingsManager>>,
) -> Result<(), String> {
    let mut s = settings.lock().unwrap();
    match preset.as_str() {
        "bypass_iran" => s.settings.bypass_iran = enabled,
        "bypass_lan" => s.settings.bypass_lan = enabled,
        "block_ads" => s.settings.block_ads = enabled,
        _ => {}
    }
    let _ = s.save();
    Ok(())
}

#[tauri::command]
pub fn update_dns_settings(
    remote: String,
    local: String,
    settings: tauri::State<std::sync::Mutex<crate::core::settings::SettingsManager>>,
) -> Result<(), String> {
    let mut s = settings.lock().unwrap();
    s.settings.remote_dns = remote;
    s.settings.local_dns = local;
    let _ = s.save();
    Ok(())
}

#[tauri::command]
pub fn is_start_minimized(
    settings: tauri::State<std::sync::Mutex<crate::core::settings::SettingsManager>>,
) -> bool {
    let s = settings.lock().unwrap();
    s.settings.ui.start_minimized
}

#[tauri::command]
pub fn minimize_window(window: tauri::Window) {
    window.minimize().unwrap_or(());
}

#[tauri::command]
pub fn close_window(window: tauri::Window) {
    window.close().unwrap_or(());
}

#[tauri::command]
pub fn drag_window(window: tauri::Window) {
    window.start_dragging().unwrap_or(());
}
use crate::core::config::*;
use crate::core::manager::CoreManager;
use crate::core::settings::SettingsManager;
use reqwest::Client;
use serde_json::json;
use serde_json::Value;
use std::os::windows::ffi::OsStrExt;
use std::sync::Mutex;
use std::time::Duration;
use tauri::State;

#[tauri::command]
pub fn get_servers(settings: State<Mutex<SettingsManager>>) -> Value {
    settings.lock().unwrap().get_server_groups()
}

#[tauri::command]
pub fn add_server(link: String, settings: State<Mutex<SettingsManager>>) {
    let remark = if let Some(idx) = link.rfind('#') {
        urlencoding::decode(&link[idx + 1..])
            .unwrap_or(std::borrow::Cow::Borrowed(""))
            .to_string()
    } else if link.starts_with("vmess://") {
        if let Some(b64) = link.strip_prefix("vmess://") {
            let decoded_json = crate::core::config::decode_base64_content(b64);
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&decoded_json) {
                v.get("ps")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown")
                    .to_string()
            } else {
                "Unknown".to_string()
            }
        } else {
            "Unknown".to_string()
        }
    } else {
        "Unknown".to_string()
    };
    settings.lock().unwrap().add_manual_server(link, remark);
}

#[tauri::command]
pub fn add_servers(links: Vec<String>, settings: State<Mutex<SettingsManager>>) -> usize {
    let mut s = settings.lock().unwrap();
    let mut added = 0;

    if !s.settings.servers.contains_key("manual") {
        s.settings.servers.insert(
            "manual".to_string(),
            crate::core::settings::ServerGroup {
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

    if let Some(group) = s.settings.servers.get_mut("manual") {
        for link in links {
            let link_trimmed = link.trim();
            if link_trimmed.is_empty() {
                continue;
            }

            // Avoid duplicates
            let exists = group.servers.iter().any(|srv| {
                srv.get("link").and_then(|l| l.as_str()) == Some(link_trimmed)
            });
            if exists {
                continue;
            }

            let remark = if let Some(idx) = link_trimmed.rfind('#') {
                urlencoding::decode(&link_trimmed[idx + 1..])
                    .unwrap_or(std::borrow::Cow::Borrowed(""))
                    .to_string()
            } else if link_trimmed.starts_with("vmess://") {
                if let Some(b64) = link_trimmed.strip_prefix("vmess://") {
                    let decoded_json = crate::core::config::decode_base64_content(b64);
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&decoded_json) {
                        v.get("ps")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown")
                            .to_string()
                    } else {
                        "Unknown".to_string()
                    }
                } else {
                    "Unknown".to_string()
                }
            } else {
                "Unknown".to_string()
            };

            group.servers.push(json!({
                "link": link_trimmed.to_string(),
                "remark": remark,
            }));
            added += 1;
        }
    }

    if added > 0 {
        let _ = s.save();
    }
    added
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
pub fn toggle_auto_update(
    group_key: String,
    enabled: bool,
    settings: State<Mutex<SettingsManager>>,
) {
    let mut s = settings.lock().unwrap();
    if let Some(group) = s.settings.servers.get_mut(&group_key) {
        group.auto_update = enabled;
    }
    let _ = s.save();
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
pub fn edit_server(
    group: String,
    old_link: String,
    new_link: String,
    settings: State<Mutex<SettingsManager>>,
) {
    let mut s = settings.lock().unwrap();
    
    // Calculate new remark from new_link
    let new_link_trimmed = new_link.trim();
    if new_link_trimmed.is_empty() {
        return;
    }
    
    let remark = if let Some(idx) = new_link_trimmed.rfind('#') {
        urlencoding::decode(&new_link_trimmed[idx + 1..])
            .unwrap_or(std::borrow::Cow::Borrowed(""))
            .to_string()
    } else if new_link_trimmed.starts_with("vmess://") {
        if let Some(b64) = new_link_trimmed.strip_prefix("vmess://") {
            let decoded_json = crate::core::config::decode_base64_content(b64);
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&decoded_json) {
                v.get("ps")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown")
                    .to_string()
            } else {
                "Unknown".to_string()
            }
        } else {
            "Unknown".to_string()
        }
    } else {
        "Unknown".to_string()
    };

    if let Some(g) = s.settings.servers.get_mut(&group) {
        for srv in &mut g.servers {
            if let Some(l) = srv.get("link").and_then(|l| l.as_str()) {
                if l == old_link.trim() {
                    *srv = json!({
                        "link": new_link_trimmed.to_string(),
                        "remark": remark,
                    });
                    break;
                }
            }
        }
    }
    
    // Migrate stats
    if let Some(stats) = s.settings.stats.remove(&old_link) {
        s.settings.stats.insert(new_link_trimmed.to_string(), stats);
    }
    
    let _ = s.save();
}

#[tauri::command]
pub fn delete_subscription(group_key: String, settings: State<Mutex<SettingsManager>>) {
    let mut s = settings.lock().unwrap();
    s.delete_subscription(&group_key);
}

#[tauri::command]
pub fn edit_subscription(
    url: String,
    new_url: Option<String>,
    name: String,
    auto_update: bool,
    update_interval: u32,
    settings: State<Mutex<SettingsManager>>,
) -> Result<(), String> {
    let mut s = settings.lock().unwrap();
    let target_url = new_url.as_ref().map(|u| u.trim().to_string()).filter(|u| !u.is_empty()).unwrap_or_else(|| url.clone());

    if target_url != url {
        // Move server group to target_url
        if let Some(mut group) = s.settings.servers.remove(&url) {
            group.name = name;
            group.auto_update = auto_update;
            group.update_interval = Some(update_interval);
            s.settings.servers.insert(target_url.clone(), group);
        } else {
            return Err("Subscription not found".to_string());
        }

        // Update subscriptions array
        for sub in &mut s.settings.subscriptions {
            if let Some(u) = sub.get_mut("url") {
                if u.as_str() == Some(&url) {
                    *u = serde_json::json!(target_url);
                }
            }
        }

        // Move stats entry
        if let Some(stat) = s.settings.stats.remove(&url) {
            s.settings.stats.insert(target_url.clone(), stat);
        }

        // Update pinned subscriptions
        for p in &mut s.settings.ui.pinned_subscriptions {
            if p == &url {
                *p = target_url.clone();
            }
        }

        // Update subscription order
        for o in &mut s.settings.ui.subscription_order {
            if o == &url {
                *o = target_url.clone();
            }
        }

        s.save().map_err(|e| e.to_string())?;
        Ok(())
    } else {
        if let Some(group) = s.settings.servers.get_mut(&url) {
            group.name = name;
            group.auto_update = auto_update;
            group.update_interval = Some(update_interval);
            s.save().map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err("Subscription not found".to_string())
        }
    }
}

#[tauri::command]
pub async fn update_subscription(
    url: String,
    settings: State<'_, Mutex<SettingsManager>>,
) -> Result<(), String> {
    let client = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client.get(&url).send().await.map_err(|e| e.to_string())?;

    let mut upload = None;
    let mut download = None;
    let mut total = None;
    let mut expire = None;

    if let Some(userinfo) = response.headers().get("subscription-userinfo") {
        if let Ok(info_str) = userinfo.to_str() {
            for part in info_str.split(';') {
                let parts: Vec<&str> = part.trim().split('=').collect();
                if parts.len() == 2 {
                    if let Ok(val) = parts[1].parse::<u64>() {
                        match parts[0].trim() {
                            "upload" => upload = Some(val),
                            "download" => download = Some(val),
                            "total" => total = Some(val),
                            "expire" => expire = Some(val),
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    // 1. Detect backend title from HTTP headers or URL fragment
    let mut backend_title: Option<String> = None;
    if let Some(title_header) = response.headers().get("profile-title") {
        if let Ok(title_str) = title_header.to_str() {
            let t = title_str.trim();
            if let Some(b64_content) = t.strip_prefix("base64:") {
                use base64::Engine;
                if let Ok(bytes) = decode_base64_forgiving(b64_content) {
                    if let Ok(s) = String::from_utf8(bytes) {
                        let clean = s.trim();
                        if !clean.is_empty() {
                            backend_title = Some(clean.to_string());
                        }
                    }
                }
            } else {
                let decoded = urlencoding::decode(t).unwrap_or(std::borrow::Cow::Borrowed(t));
                let clean = decoded.trim();
                if !clean.is_empty() {
                    backend_title = Some(clean.to_string());
                }
            }
        }
    }

    // 2. Fallback to Content-Disposition filename
    if backend_title.is_none() {
        if let Some(cd_header) = response.headers().get("content-disposition") {
            if let Ok(cd_str) = cd_header.to_str() {
                if let Some(idx) = cd_str.find("filename*=") {
                    let rest = &cd_str[idx + 10..];
                    let val = rest.split(';').next().unwrap_or("").trim().trim_matches('"');
                    let clean = if let Some(stripped) = val.strip_prefix("UTF-8''") { stripped } else { val };
                    let decoded = urlencoding::decode(clean).unwrap_or(std::borrow::Cow::Borrowed(clean));
                    let trimmed = decoded.trim();
                    if !trimmed.is_empty() {
                        backend_title = Some(trimmed.to_string());
                    }
                } else if let Some(idx) = cd_str.find("filename=") {
                    let rest = &cd_str[idx + 9..];
                    let val = rest.split(';').next().unwrap_or("").trim().trim_matches('"');
                    if !val.trim().is_empty() {
                        backend_title = Some(val.trim().to_string());
                    }
                }
            }
        }
    }

    // 3. Fallback to URL fragment (#SubscriptionName)
    if backend_title.is_none() {
        if let Some(idx) = url.rfind('#') {
            let fragment = &url[idx + 1..];
            let decoded = urlencoding::decode(fragment).unwrap_or(std::borrow::Cow::Borrowed(fragment));
            let trimmed = decoded.trim();
            if !trimmed.is_empty() {
                backend_title = Some(trimmed.to_string());
            }
        }
    }

    // 4. Detect recommended update interval from header (in hours or seconds)
    let mut header_interval: Option<u32> = None;
    if let Some(interval_header) = response.headers().get("profile-update-interval") {
        if let Ok(s) = interval_header.to_str() {
            if let Ok(n) = s.trim().parse::<u32>() {
                let hours = if n > 72 { (n / 3600).max(1) } else { n.max(1) };
                header_interval = Some(hours);
            }
        }
    }

    // 5. Detect announcement from headers (standard V2Box / proxy subscription headers)
    let mut backend_announcement: Option<String> = None;
    let announcement_headers = [
        "announcement",
        "announce",
        "profile-announcement",
        "subscription-announcement",
        "notice",
        "profile-notice",
        "x-announcement",
    ];
    for h in announcement_headers {
        if let Some(val) = response.headers().get(h) {
            if let Ok(raw_str) = val.to_str() {
                let t = raw_str.trim();
                if t.is_empty() {
                    continue;
                }
                if let Some(b64_content) = t.strip_prefix("base64:") {
                    use base64::Engine;
                    if let Ok(bytes) = decode_base64_forgiving(b64_content) {
                        if let Ok(s) = String::from_utf8(bytes) {
                            let clean = s.trim();
                            if !clean.is_empty() {
                                backend_announcement = Some(clean.to_string());
                                break;
                            }
                        }
                    }
                } else if t.contains('%') {
                    let decoded = urlencoding::decode(t).unwrap_or(std::borrow::Cow::Borrowed(t));
                    let clean = decoded.trim();
                    if !clean.is_empty() {
                        backend_announcement = Some(clean.to_string());
                        break;
                    }
                } else {
                    use base64::Engine;
                    let mut handled = false;
                    if t.len() % 4 == 0 && t.len() >= 4 && !t.contains(' ') {
                        if let Ok(bytes) = decode_base64_forgiving(t) {
                            if let Ok(s) = String::from_utf8(bytes) {
                                let clean = s.trim();
                                if !clean.is_empty() && clean.chars().all(|c| !c.is_control() || c == '\n' || c == '\r' || c == '\t') {
                                    backend_announcement = Some(clean.to_string());
                                    handled = true;
                                }
                            }
                        }
                    }
                    if !handled {
                        backend_announcement = Some(t.to_string());
                    }
                    break;
                }
            }
        }
    }

    let content = response.text().await.map_err(|e| e.to_string())?;

    // Check JSON content for announcement if not found in headers
    if backend_announcement.is_none() {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(ann) = v.get("announcement").or_else(|| v.get("notice")).or_else(|| v.get("message")) {
                if let Some(s) = ann.as_str() {
                    let clean = s.trim();
                    if !clean.is_empty() {
                        backend_announcement = Some(clean.to_string());
                    }
                }
            }
        }
    }

    let mut decoded = decode_base64_content(&content);
    if decoded.is_empty() && content.contains("://") {
        decoded = content.clone();
    }
    let mut servers = vec![];

    for line in decoded.lines() {
        let link = line.trim();
        if link.is_empty() {
            continue;
        }

        let remark = if let Some(idx) = link.rfind('#') {
            urlencoding::decode(&link[idx + 1..])
                .unwrap_or(std::borrow::Cow::Borrowed(""))
                .to_string()
        } else if link.starts_with("vmess://") {
            if let Some(b64) = link.strip_prefix("vmess://") {
                let decoded_json = crate::core::config::decode_base64_content(b64);
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&decoded_json) {
                    v.get("ps")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown")
                        .to_string()
                } else {
                    "Unknown".to_string()
                }
            } else {
                "Unknown".to_string()
            }
        } else {
            "Unknown".to_string()
        };

        // Check for dummy announcement server node (common in V2Ray subscriptions)
        let is_dummy_announcement = (link.contains("127.0.0.1") || link.contains("0.0.0.0") || link.contains("localhost") || link.contains("1.0.0.1")) && (remark.to_lowercase().contains("announcement") || remark.to_lowercase().contains("notice"));
        
        let is_explicit_announcement = remark.starts_with("Announcement:");

        if (is_dummy_announcement || is_explicit_announcement) && backend_announcement.is_none() {
            backend_announcement = Some(remark.clone());
            if is_dummy_announcement {
                continue; // Skip adding dummy node to active server list
            }
        }

        servers.push(serde_json::json!({
            "link": link,
            "remark": remark,
            "latency": serde_json::Value::Null
        }));
    }

    let default_domain = url.split('/').nth(2).unwrap_or("Subscription").to_string();

    {
        let mut s = settings.lock().unwrap();
        // Preserve existing settings if this group already exists, or apply smart backend title & announcement
        let (name, prev_auto_update, prev_update_interval, prev_up, prev_down, prev_tot, prev_exp, prev_ann) =
            if let Some(existing) = s.settings.servers.get(&url) {
                // If current name is just the default domain or empty, upgrade it to backend title if found
                let active_name = if (existing.name == default_domain || existing.name == "Subscription" || existing.name.is_empty())
                    && backend_title.is_some()
                {
                    backend_title.unwrap()
                } else {
                    existing.name.clone()
                };
                (
                    active_name,
                    existing.auto_update,
                    existing.update_interval.or(header_interval),
                    existing.upload,
                    existing.download,
                    existing.total,
                    existing.expire,
                    existing.announcement.clone(),
                )
            } else {
                let initial_name = backend_title.unwrap_or(default_domain);
                (initial_name, true, header_interval.or(Some(24)), None, None, None, None, None)
            };
        s.settings.servers.insert(
            url.clone(),
            crate::core::settings::ServerGroup {
                name,
                servers,
                upload: upload.or(prev_up),
                download: download.or(prev_down),
                total: total.or(prev_tot),
                expire: expire.or(prev_exp),
                auto_update: prev_auto_update,
                update_interval: prev_update_interval,
                announcement: backend_announcement.or(prev_ann),
            },
        );
        let _ = s.save();
    }

    Ok(())
}

fn get_default_gateway() -> Option<String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let output = std::process::Command::new("route")
        .creation_flags(CREATE_NO_WINDOW)
        .arg("print")
        .arg("0.0.0.0")
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 && parts[0] == "0.0.0.0" && parts[1] == "0.0.0.0" {
            let gw = parts[2];
            if gw != "On-link" && gw != "172.19.0.1" && gw != "127.0.0.1" {
                return Some(gw.to_string());
            }
        }
    }
    None
}

#[tauri::command]
pub async fn connect(
    link: String,
    tun_mode: bool,
    core: tauri::State<'_, std::sync::Mutex<crate::core::manager::CoreManager>>,
    settings: tauri::State<'_, std::sync::Mutex<crate::core::settings::SettingsManager>>,
) -> Result<(), String> {
    let port = 2080;
    let (config, exact_ip, is_xray, strict_route, remote_dns, local_dns, settings_clone) = {
        let s = settings.lock().unwrap();
        let (cfg, ip, xray) = crate::core::config::generate_config(&link, port, Some(&s.settings))?;
        (cfg, ip, xray, s.settings.strict_route, s.settings.remote_dns.clone(), s.settings.local_dns.clone(), s.settings.clone())
    };

    if tun_mode && !crate::core::manager::CoreManager::is_admin() {
        return Err("Administrator privileges required for TUN mode".into());
    }

    let (mut old_core, mut old_tracker, mut old_tun) = (None, None, None);
    let mut start_new_tun = false;
    {
        let mut c = core.lock().unwrap();
        let old_ip = c.current_server_ip.clone();
        c.current_server_ip = exact_ip.clone();
        old_core = c.take_core_process();
        if !is_xray {
            old_tracker = c.take_tracker_process();
        }
        
        if tun_mode {
            c.clear_system_proxy();
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            if c.get_tun_enabled() { // If tun is already running
                // Keep it alive, manage routes dynamically
                if let (Some(gw), Some(new_ip)) = (get_default_gateway(), &exact_ip) {
                    if let Some(old) = old_ip {
                        let _ = std::process::Command::new("route").creation_flags(CREATE_NO_WINDOW).args(&["delete", &old]).output();
                    }
                    let _ = std::process::Command::new("route").creation_flags(CREATE_NO_WINDOW).args(&["add", new_ip, "mask", "255.255.255.255", &gw]).output();
                }
            } else {
                old_tun = c.take_tun_process();
                start_new_tun = true;
                // Add route for new TUN
                if let (Some(gw), Some(new_ip)) = (get_default_gateway(), &exact_ip) {
                    let _ = std::process::Command::new("route").creation_flags(CREATE_NO_WINDOW).args(&["add", new_ip, "mask", "255.255.255.255", &gw]).output();
                }
            }
        } else {
            old_tun = c.take_tun_process();
            // Delete old route if we are disabling TUN
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            if let Some(old) = old_ip {
                let _ = std::process::Command::new("route").creation_flags(CREATE_NO_WINDOW).args(&["delete", &old]).output();
            }
        }
    }

    // Await process termination on a blocking thread outside the lock to guarantee ports are free
    // This prevents "bind: Only one usage" port collisions across rapid connect/disconnect/tun toggles.
    let _ = tokio::task::spawn_blocking(move || {
        let mut tun_killed = false;
        let mut core_killed = false;
        if let Some(mut p) = old_core {
            let _ = p.kill();
            let _ = p.wait();
            core_killed = true;
        }
        if let Some(mut p) = old_tracker {
            let _ = p.kill();
            let _ = p.wait();
            core_killed = true;
        }
        if let Some(mut p) = old_tun {
            let _ = p.kill();
            let _ = p.wait();
            tun_killed = true;
        }

        if tun_killed {
            // Give Wintun driver a brief moment to deregister
            std::thread::sleep(std::time::Duration::from_millis(150));
        } else if core_killed {
            // TCP ports are freed immediately on process exit
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    })
    .await;

    // Step 1: Start the main core and auxiliary cores (quick lock)
    {
        let mut c = core.lock().unwrap();
        c.start_core(config)?;
        
        let current_tun_mode = settings.lock().unwrap().settings.ui.tun_enabled;
        
        if is_xray {
            if !c.is_tracker_running() {
                let tracker_config = serde_json::json!({
                    "log": { "level": "fatal" },
                    "experimental": {
                        "clash_api": {
                            "external_controller": "127.0.0.1:9090"
                        }
                    },
                    "inbounds": [{
                        "type": "mixed",
                        "tag": "mixed-in",
                        "listen": "127.0.0.1",
                        "listen_port": port,
                        "sniff": true,
                        "sniff_override_destination": true
                    }],
                    "outbounds": [{
                        "type": "socks",
                        "tag": "proxy",
                        "server": "127.0.0.1",
                        "server_port": port + 1,
                        "version": "5"
                    }],
                    "route": {
                        "final": "proxy"
                    }
                });
                c.start_tracker_core(tracker_config)?;
            }
        }

        if current_tun_mode {
            if start_new_tun {
                let tun_config = generate_tun_config(port, &link, exact_ip.clone(), is_xray, strict_route, remote_dns, local_dns, Some(&settings_clone));
                c.start_tun_core(tun_config, is_xray)?;
            }
        } else {
            c.set_system_proxy("127.0.0.1", port);
        }
    }

    // Step 2: Wait for the main core to bind to its port (prevents UI from showing "Connected" too early)
    let socks_port = if is_xray { port + 1 } else { port };
    for _ in 0..30 {
        if std::net::TcpStream::connect(("127.0.0.1", socks_port)).is_ok() {
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
    
    // Check if user clicked Disconnect during wait
    {
        let mut c = core.lock().unwrap();
        if !c.is_alive() {
            return Err("Connection aborted by user".to_string());
        }
    }

    // Wait for core to initialize with smart retry (up to 1.5 seconds)
    let mut alive = false;
    for _ in 0..30 {
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        let mut c = core.lock().unwrap();
        if c.is_alive() {
            alive = true;
            break;
        }
    }

    if !alive {
        println!(
            "Core failed to start within timeout, but proceeding to let auto-reconnect handle it."
        );
    }

    {
        let mut s = settings.lock().unwrap();
        s.update_ui_state(Some(link), None); // Preserve whatever tun mode is currently set
    }

    Ok(())
}

#[tauri::command]
pub async fn disconnect(core: State<'_, Mutex<CoreManager>>) -> Result<(), String> {
    let (core_p, tracker_p, tun_p) = {
        let mut c = core.lock().unwrap();
        c.clear_proxy_if(true);
        (
            c.take_core_process(),
            c.take_tracker_process(),
            c.take_tun_process(),
        )
    };

    let _ = tokio::task::spawn_blocking(move || {
        if let Some(mut p) = core_p {
            let _ = p.kill();
            let _ = p.wait();
        }
        if let Some(mut p) = tracker_p {
            let _ = p.kill();
            let _ = p.wait();
        }
        if let Some(mut p) = tun_p {
            std::thread::spawn(move || {
                let _ = p.kill();
                let _ = p.wait();
            });
        }
    })
    .await;

    Ok(())
}

#[tauri::command]
pub async fn check_ip(use_proxy: bool) -> Result<serde_json::Value, String> {
    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(7))
        .no_proxy()
        .local_address("0.0.0.0".parse().ok());
    if use_proxy {
        let proxy = reqwest::Proxy::all("http://127.0.0.1:2080").map_err(|e| e.to_string())?;
        builder = builder.proxy(proxy);
    }

    let client = builder.build().map_err(|e| e.to_string())?;

        // Try ip-api.com first
    if let Ok(res) = client.get("http://ip-api.com/json").send().await {
        if let Ok(val) = res.json::<serde_json::Value>().await {
            if val.get("query").is_some() {
                return Ok(val);
            }
        }
    }

    // Fallback to ipwho.is
    if let Ok(res) = client.get("https://ipwho.is/").send().await {
        if let Ok(mut val) = res.json::<serde_json::Value>().await {
            if let Some(ip) = val.get("ip").cloned() {
                val.as_object_mut().unwrap().insert("query".to_string(), ip);
                if let Some(country_code) = val.get("country_code").cloned() {
                    val.as_object_mut()
                        .unwrap()
                        .insert("countryCode".to_string(), country_code);
                }
                if let Some(country) = val.get("country").cloned() {
                    val.as_object_mut()
                        .unwrap()
                        .insert("country".to_string(), country);
                }
                return Ok(val);
            }
        }
    }

    // Fallback to freeipapi.com
    if let Ok(res) = client.get("https://freeipapi.com/api/json/").send().await {
        if let Ok(mut val) = res.json::<serde_json::Value>().await {
            if let Some(ip) = val.get("ipAddress").cloned() {
                val.as_object_mut().unwrap().insert("query".to_string(), ip);
                if let Some(country_code) = val.get("countryCode").cloned() {
                    val.as_object_mut()
                        .unwrap()
                        .insert("countryCode".to_string(), country_code);
                }
                if let Some(country) = val.get("countryName").cloned() {
                    val.as_object_mut()
                        .unwrap()
                        .insert("country".to_string(), country);
                }
                return Ok(val);
            }
        }
    }

    // Fallback to api.myip.com
    if let Ok(res) = client.get("https://api.myip.com/").send().await {
        if let Ok(mut val) = res.json::<serde_json::Value>().await {
            if let Some(ip) = val.get("ip").cloned() {
                val.as_object_mut().unwrap().insert("query".to_string(), ip);
                if let Some(country_code) = val.get("cc").cloned() {
                    val.as_object_mut()
                        .unwrap()
                        .insert("countryCode".to_string(), country_code);
                }
                if let Some(country) = val.get("country").cloned() {
                    val.as_object_mut()
                        .unwrap()
                        .insert("country".to_string(), country);
                }
                return Ok(val);
            }
        }
    }

    // Fallback to Cloudflare trace to get IP directly (very resilient)
    if let Ok(res) = client.get("https://1.1.1.1/cdn-cgi/trace").send().await {
        if let Ok(text) = res.text().await {
            let mut ip = String::new();
            let mut loc = String::new();
            for line in text.lines() {
                if let Some(rest) = line.strip_prefix("ip=") {
                    ip = rest.trim().to_string();
                } else if let Some(rest) = line.strip_prefix("loc=") {
                    loc = rest.trim().to_string();
                }
            }
            if !ip.is_empty() {
                return Ok(serde_json::json!({
                    "query": ip,
                    "country": loc.clone(),
                    "countryCode": loc
                }));
            }
        }
    }

    // Final fallback just to check internet connectivity
    if client
        .get("https://www.google.com/generate_204")
        .send()
        .await
        .is_ok()
    {
        return Ok(serde_json::json!({
            "query": "Unknown IP",
            "country": "Connected",
            "countryCode": "UN"
        }));
    }

    Err("All IP checks failed".into())
}


async fn do_single_ping(
    link: String,
    bin_dir: std::path::PathBuf,
    data_dir: std::path::PathBuf,
) -> i64 {
    if CANCEL_TESTS.load(std::sync::atomic::Ordering::SeqCst) {
        return -1;
    }
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let mut port = PING_PORT.fetch_add(2, std::sync::atomic::Ordering::SeqCst);
    if port > 40000 {
        PING_PORT.store(30000, std::sync::atomic::Ordering::SeqCst);
        port = 30000;
    }
    let mut config = crate::core::config::generate_single_test_config(&link, port);
    
    let mut exe_name = "sing-box.exe";
    if let Some(core_type) = config.get("__core__").and_then(|v| v.as_str()) {
        if core_type == "xray" {
            exe_name = "xray.exe";
        }
    }
    if let Some(obj) = config.as_object_mut() {
        obj.remove("__core__");
    }


    struct FileCleanup(std::path::PathBuf);
    impl Drop for FileCleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    
    let config_path = data_dir.join(format!("test_config_{}.json", port));
    let _cleanup = FileCleanup(config_path.clone());
    let _ = std::fs::write(&config_path, config.to_string());


    let exe_path = bin_dir.join(exe_name);
    
    let child_res = std::process::Command::new(&exe_path)
        .current_dir(&bin_dir)
        .args(["run", "-c", &config_path.to_string_lossy()])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();

    let mut latency = -1;
    if let Ok(mut child) = child_res {
        let mut ready = false;
        for _ in 0..30 {
            if CANCEL_TESTS.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
            if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
                ready = true;
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        
        if ready && !CANCEL_TESTS.load(std::sync::atomic::Ordering::SeqCst) {
            latency = real_delay_ping(port, false).await;
        }
        
        let _ = child.kill();
        let _ = child.wait();
    }
        latency
}

#[tauri::command]
pub async fn test_all_latency(
    links: Vec<String>,
    core: tauri::State<'_, std::sync::Mutex<crate::core::manager::CoreManager>>,
    settings: tauri::State<'_, std::sync::Mutex<SettingsManager>>,
) -> Result<serde_json::Value, String> {
    let (bin_dir, data_dir) = {
        let c = core.lock().unwrap();
        (c.bin_dir.clone(), c.data_dir.clone())
    };

    let mut sb_links = vec![];
    let mut xray_links = vec![];

    for link in links {
        let mut is_xray = false;
        if let Some(parsed) = crate::core::config::parse_link(&link) {
            if let Some(transport) = parsed.get("transport") {
                let t_type = transport.get("type").and_then(|t| t.as_str()).unwrap_or("");
                let h_type = transport.get("header_type").and_then(|h| h.as_str()).unwrap_or("");
                if t_type == "xhttp" || h_type == "http" {
                    is_xray = true;
                }
            }
        }
        if is_xray {
            xray_links.push(link);
        } else {
            sb_links.push(link);
        }
    }

    let config = crate::core::config::generate_mass_test_config(sb_links.clone(), 25000);
    if !sb_links.is_empty() {
        let mut c = core.lock().unwrap();
        if let Err(_e) = c.start_test_core(config) {
            // Ignore failure, just continue with empty
        }
    }

    if !sb_links.is_empty() {
        for _ in 0..30 {
            if CANCEL_TESTS.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
            if std::net::TcpStream::connect(("127.0.0.1", 25000)).is_ok() {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    let mut tasks = vec![];
    for (i, link) in sb_links.into_iter().enumerate() {
        let port = 25000 + (i as u16);
        let task = tokio::spawn(async move {
            if CANCEL_TESTS.load(std::sync::atomic::Ordering::SeqCst) {
                return (link, -1);
            }
            let latency = real_delay_ping(port, false).await;
            (link, latency)
        });
        tasks.push(task);
    }

    for link in xray_links {
        let l = link.clone();
        let bd = bin_dir.clone();
        let dd = data_dir.clone();
        let task = tokio::spawn(async move {
            if CANCEL_TESTS.load(std::sync::atomic::Ordering::SeqCst) {
                return (l, -1);
            }
            let latency = do_single_ping(l.clone(), bd, dd).await;
            (l, latency)
        });
        tasks.push(task);
    }

    let mut results = serde_json::Map::new();
    for task in tasks {
        if let Ok((link, latency)) = task.await {
            results.insert(link.clone(), serde_json::json!(latency));
            if latency > 0 {
                let mut s = settings.lock().unwrap();
                let stat = s
                    .settings
                    .stats
                    .entry(link)
                    .or_insert_with(crate::core::settings::ServerStats::default);
                stat.ping_history.push(latency as u64);
                if stat.ping_history.len() > 30 {
                    stat.ping_history.remove(0);
                }
                let _ = s.save();
            }
        }
    }

    {
        let mut c = core.lock().unwrap();
        c.stop_test_core();
    }

    Ok(serde_json::Value::Object(results))
}

static PING_PORT: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(30000);
static CANCEL_TESTS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[tauri::command]
pub fn cancel_latency_tests() {
    CANCEL_TESTS.store(true, std::sync::atomic::Ordering::SeqCst);
}

#[tauri::command]
pub fn start_latency_tests() {
    CANCEL_TESTS.store(false, std::sync::atomic::Ordering::SeqCst);
}

#[tauri::command]
pub async fn test_single_latency(
    link: String,
    core: tauri::State<'_, std::sync::Mutex<crate::core::manager::CoreManager>>,
    settings: tauri::State<'_, std::sync::Mutex<SettingsManager>>,
) -> Result<u64, String> {
    if CANCEL_TESTS.load(std::sync::atomic::Ordering::SeqCst) {
        return Ok(0);
    }
    let (bin_dir, data_dir) = {
        let c = core.lock().unwrap();
        (c.bin_dir.clone(), c.data_dir.clone())
    };

    let latency = do_single_ping(link.clone(), bin_dir, data_dir).await;

    if latency > 0 {
        let lat = latency as u64;
        let mut s = settings.lock().unwrap();
        let stat = s
            .settings
            .stats
            .entry(link)
            .or_insert_with(crate::core::settings::ServerStats::default);
        stat.ping_history.push(lat);
        if stat.ping_history.len() > 30 {
            stat.ping_history.remove(0);
        }
        let _ = s.save();
        Ok(lat)
    } else {
        Err("Ping failed".to_string())
    }
}

#[tauri::command]
pub async fn get_live_ping(
    link: String,
    settings: tauri::State<'_, std::sync::Mutex<SettingsManager>>,
) -> Result<u64, String> {
    let latency = real_delay_ping(2080, true).await;

    if latency > 0 {
        let lat = latency as u64;
        let mut s = settings.lock().unwrap();
        let stat = s
            .settings
            .stats
            .entry(link)
            .or_insert_with(crate::core::settings::ServerStats::default);
        stat.ping_history.push(lat);
        if stat.ping_history.len() > 50 {
            stat.ping_history.remove(0);
        }
        let _ = s.save();
        Ok(lat)
    } else {
        {
            reset_live_client();
            Err("Timeout".into())
        }
    }
}

/// Real delay ping exactly mirroring V2RayN algorithm
use once_cell::sync::Lazy;

static LIVE_CLIENT: Lazy<Mutex<Option<Client>>> = Lazy::new(|| Mutex::new(None));

pub fn get_or_create_live_client(port: u16) -> reqwest::Client {
    let mut state = LIVE_CLIENT.lock().unwrap();
    if let Some(c) = state.as_ref() {
        return c.clone();
    }
    let proxy = reqwest::Proxy::all(format!("http://127.0.0.1:{}", port)).unwrap();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .connect_timeout(std::time::Duration::from_secs(3))
        .proxy(proxy)
        .build()
        .unwrap();
    *state = Some(client.clone());
    client
}

pub fn reset_live_client() {
    let mut guard = LIVE_CLIENT.lock().unwrap();
    *guard = None;
}

async fn real_delay_ping(port: u16, reuse: bool) -> i64 {
    let client = if reuse {
        get_or_create_live_client(port)
    } else {
        let proxy = match reqwest::Proxy::all(format!("http://127.0.0.1:{}", port)) {
            Ok(p) => p,
            Err(_) => return -1,
        };
        match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3))
            .connect_timeout(std::time::Duration::from_secs(3))
            .proxy(proxy)
            .build()
        {
            Ok(c) => c,
            Err(_) => return -1,
        }
    };

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let url = format!("https://www.google.com/generate_204?nocache={}", ts);

    let mut attempts = if reuse { 1 } else { 2 };
    let mut one_time = Vec::new();

    for i in 0..2 {
        let start = tokio::time::Instant::now();
        if let Ok(res) = client.get(&url).send().await {
            if res.status().is_success()
                || res.status().as_u16() == 204
                || res.status().as_u16() == 200
            {
                one_time.push(start.elapsed().as_millis() as i64);
            }
        }
        
        if reuse && attempts == 1 && !one_time.is_empty() {
            if one_time[0] < 600 {
                break;
            } else {
                attempts = 2; // Cold start detected, try again to get a real ping
            }
        }
        
        if i < attempts - 1 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        } else {
            break;
        }
    }

    if one_time.is_empty() {
        if reuse {
            reset_live_client();
        }
        -1
    } else {
        *one_time.iter().min().unwrap()
    }
}

#[tauri::command]
pub fn maximize_window(window: tauri::Window) {
    if window.is_maximized().unwrap_or(false) {
        window.unmaximize().unwrap_or(());
    } else {
        window.maximize().unwrap_or(());
    }
}

#[tauri::command]
pub fn set_auto_update_subs(
    enabled: bool,
    settings: tauri::State<std::sync::Mutex<SettingsManager>>,
) -> Result<(), String> {
    let mut s = settings.lock().unwrap();
    s.settings.ui.auto_update_subs = enabled;
    let _ = s.save();
    Ok(())
}

#[tauri::command]
pub fn set_live_ping_enabled(
    enabled: bool,
    settings: tauri::State<std::sync::Mutex<SettingsManager>>,
) -> Result<(), String> {
    let mut s = settings.lock().unwrap();
    s.settings.ui.live_ping_enabled = enabled;
    let _ = s.save();
    Ok(())
}

#[tauri::command]
pub fn set_auto_reconnect(
    enabled: bool,
    settings: tauri::State<std::sync::Mutex<SettingsManager>>,
) -> Result<(), String> {
    let mut s = settings.lock().unwrap();
    s.settings.ui.auto_reconnect = enabled;
    let _ = s.save();
    Ok(())
}

#[tauri::command]
pub fn set_auto_start(
    enabled: bool,
    settings: tauri::State<std::sync::Mutex<SettingsManager>>,
) -> Result<(), String> {
    let mut s = settings.lock().unwrap();
    s.settings.ui.auto_start = enabled;
    let _ = s.save();
    Ok(())
}

#[tauri::command]
pub fn set_start_minimized(
    enabled: bool,
    settings: tauri::State<std::sync::Mutex<SettingsManager>>,
) -> Result<(), String> {
    let mut s = settings.lock().unwrap();
    s.settings.ui.start_minimized = enabled;
    Ok(())
}

#[tauri::command]
pub fn save_last_connection(
    link: String,
    tun_mode: bool,
    settings: tauri::State<std::sync::Mutex<SettingsManager>>,
) -> Result<(), String> {
    let mut s = settings.lock().unwrap();
    s.settings.ui.last_connection_link = Some(link);
    s.settings.ui.last_connection_tun = tun_mode;
    let _ = s.save();
    Ok(())
}

#[tauri::command]
pub fn get_auto_reconnect_info(
    settings: tauri::State<std::sync::Mutex<SettingsManager>>,
) -> Result<serde_json::Value, String> {
    let s = settings.lock().unwrap();
    let enabled = s.settings.ui.auto_reconnect;
    let link = s.settings.ui.last_connection_link.clone();
    let tun_mode = s.settings.ui.last_connection_tun;

    Ok(json!({
        "auto_reconnect": enabled,
        "link": link,
        "tun_mode": tun_mode
    }))
}

#[tauri::command]
pub fn update_core_settings(
    fragment: bool,
    strict_route: bool,
    core_choice: String,
    settings: tauri::State<std::sync::Mutex<SettingsManager>>,
) -> Result<(), String> {
    let mut s = settings.lock().unwrap();
    s.settings.tls_fragment = fragment;
    s.settings.strict_route = strict_route;
    s.settings.core_choice = core_choice;
    s.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_core_settings(
    settings: tauri::State<std::sync::Mutex<SettingsManager>>,
) -> Result<serde_json::Value, String> {
    let s = settings.lock().unwrap();
    Ok(json!({
        "tls_fragment": s.settings.tls_fragment,
        "strict_route": s.settings.strict_route,
        "core_choice": s.settings.core_choice
    }))
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn switch_tun_mode(
    tunMode: bool,
    link: String,
    core: tauri::State<'_, std::sync::Mutex<crate::core::manager::CoreManager>>,
    settings: tauri::State<'_, std::sync::Mutex<SettingsManager>>,
) -> Result<(), String> {
    let (is_xray, strict_route, remote_dns, local_dns, settings_clone) = {
        let s = settings.lock().unwrap();
        let (_, _, is_xray) = crate::core::config::generate_config(&link, 2080, Some(&s.settings)).unwrap_or((serde_json::Value::Null, None, false));
        (is_xray, s.settings.strict_route, s.settings.remote_dns.clone(), s.settings.local_dns.clone(), s.settings.clone())
    };

    let (mut old_tracker, mut old_tun, current_server_ip) = {
        let mut c = core.lock().unwrap();
        if !c.is_alive() {
            return Ok(());
        }

        let ip = c.current_server_ip.clone();
        if tunMode {
            c.clear_system_proxy();
        }
        (None::<std::process::Child>, c.take_tun_process(), ip)
    };

    // Await tracker termination to free up ports, but detach TUN teardown to prevent UI freezing
    let _ = tokio::task::spawn_blocking(move || {
        if let Some(mut p) = old_tracker {
            let _ = p.kill();
            let _ = p.wait();
        }
        
        // TUN tear down takes a few seconds in Windows. Detach it so we don't block.
        if let Some(mut p) = old_tun {
            std::thread::spawn(move || {
                let _ = p.kill();
                let _ = p.wait();
            });
        }
    })
    .await;

    let mut c = core.lock().unwrap();
    if is_xray {
        if !c.is_tracker_running() {
            let tracker_config = serde_json::json!({
                "log": { "level": "fatal" },
                "experimental": {
                    "clash_api": {
                        "external_controller": "127.0.0.1:9090"
                    }
                },
                "inbounds": [{
                    "type": "mixed",
                    "tag": "mixed-in",
                    "listen": "127.0.0.1",
                    "listen_port": 2080,
                    "sniff": true,
                    "sniff_override_destination": true
                }],
                "outbounds": [{
                    "type": "socks",
                    "tag": "proxy",
                    "server": "127.0.0.1",
                    "server_port": 2081,
                    "version": "5"
                }],
                "route": {
                    "final": "proxy"
                }
            });
            let _ = c.start_tracker_core(tracker_config);
        }
    }

    if tunMode {
        c.clear_system_proxy();
        let tun_config = crate::core::config::generate_tun_config(
            2080,
            &link,
            current_server_ip,
            is_xray,
            strict_route,
            remote_dns,
            local_dns,
            Some(&settings_clone),
        );
        c.start_tun_core(tun_config, is_xray)
            .map_err(|e| e.to_string())?;
    } else {
        c.set_system_proxy("127.0.0.1", 2080);
    }
    Ok(())
}

#[tauri::command]
pub fn restart_as_admin(core: tauri::State<'_, std::sync::Mutex<crate::core::manager::CoreManager>>) -> Result<(), String> {
    // Kill processes before exiting to prevent zombies holding port 9090
    {
        let mut c = core.lock().unwrap();
        c.clear_proxy_if(true);
        if let Some(mut p) = c.take_core_process() { let _ = p.kill(); }
        if let Some(mut p) = c.take_tracker_process() { let _ = p.kill(); }
        if let Some(mut p) = c.take_tun_process() { let _ = p.kill(); }
    }
    
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    
    // Spawn ourselves with a special flag to act as an invisible elevator process.
    // We use CREATE_BREAKAWAY_FROM_JOB (0x01000000) so this background process isn't
    // killed instantly when the parent's Job Object closes.
    use std::os::windows::process::CommandExt;
    let _ = std::process::Command::new(exe)
        .arg("--elevator")
        .creation_flags(0x01000000)
        .spawn();

    std::process::exit(0);
}
#[tauri::command]
pub fn toggle_tun(enabled: bool, settings: tauri::State<std::sync::Mutex<SettingsManager>>) {
    let mut s = settings.lock().unwrap();
    s.settings.ui.tun_enabled = enabled;
    let _ = s.save();
}

#[tauri::command]
pub fn is_admin(core: tauri::State<std::sync::Mutex<CoreManager>>) -> bool {
    let _c = core.lock().unwrap();
    crate::core::manager::CoreManager::is_admin()
}

#[tauri::command]
pub fn clear_data(settings: tauri::State<std::sync::Mutex<SettingsManager>>) -> Result<(), String> {
    let mut s = settings.lock().unwrap();
    s.settings.stats.clear();
    let _ = s.save();
    Ok(())
}

#[tauri::command]
pub fn is_tracker_running(
    core: tauri::State<std::sync::Mutex<crate::core::manager::CoreManager>>,
) -> bool {
    let c = core.lock().unwrap();
    c.is_tracker_running()
}

#[tauri::command]
pub fn is_core_alive(
    core: tauri::State<std::sync::Mutex<crate::core::manager::CoreManager>>,
) -> bool {
    let mut c = core.lock().unwrap();
    c.is_alive()
}

#[tauri::command]
pub fn get_tun_enabled(settings: tauri::State<std::sync::Mutex<SettingsManager>>) -> bool {
    settings.lock().unwrap().settings.ui.tun_enabled
}

#[tauri::command]
pub fn get_startup_settings(
    settings: tauri::State<std::sync::Mutex<SettingsManager>>,
) -> Result<serde_json::Value, String> {
    Ok(serde_json::to_value(&settings.lock().unwrap().settings.ui).unwrap())
}

#[tauri::command]
pub fn toggle_pin_subscription(
    url: String,
    settings: tauri::State<std::sync::Mutex<crate::core::settings::SettingsManager>>,
) -> Result<Vec<String>, String> {
    let mut s = settings.lock().unwrap();
    let pinned = s.toggle_pin_subscription(&url);
    let _ = s.save();
    Ok(pinned)
}

#[tauri::command]
pub fn save_subscription_order(
    urls: Vec<String>,
    settings: tauri::State<std::sync::Mutex<crate::core::settings::SettingsManager>>,
) -> Result<(), String> {
    let mut s = settings.lock().unwrap();
    s.save_subscription_order(urls);
    let _ = s.save();
    Ok(())
}

#[tauri::command]
pub fn save_server_order(
    group_key: String,
    ordered_links: Vec<String>,
    settings: tauri::State<std::sync::Mutex<crate::core::settings::SettingsManager>>,
) -> Result<(), String> {
    let mut s = settings.lock().unwrap();
    if let Some(group) = s.settings.servers.get_mut(&group_key) {
        let mut new_servers = Vec::new();
        for link in &ordered_links {
            if let Some(pos) = group.servers.iter().position(|srv| {
                srv.get("link").and_then(|l| l.as_str()) == Some(link.as_str())
            }) {
                new_servers.push(group.servers.remove(pos));
            }
        }
        new_servers.append(&mut group.servers);
        group.servers = new_servers;
        let _ = s.save();
    }
    Ok(())
}

#[tauri::command]
pub fn copy_to_clipboard(text: String) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(text).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn read_from_clipboard() -> Result<String, String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.get_text().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_manual_servers(
    settings: tauri::State<std::sync::Mutex<crate::core::settings::SettingsManager>>,
) -> Result<(), String> {
    let mut s = settings.lock().unwrap();
    if let Some(manual) = s.settings.servers.get_mut("manual") {
        manual.servers.clear();
        let _ = s.save();
    }
    Ok(())
}










