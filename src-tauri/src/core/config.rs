use base64::{engine::general_purpose, Engine as _};

use serde_json::{json, Value};
use url::Url;

/// Sanitize remote DNS: reject Cloudflare 1.1.1.1 (intercepted by Iranian ISPs with HTTP 301)
/// and "local" (causes TUN DNS loop). Falls back to Google DoH.
fn sanitize_remote_dns(dns: &str) -> String {
    let dns = dns.trim();
    if dns.is_empty() || dns == "local" || dns.contains("8.8.8.8") {
        return "https://1.1.1.1/dns-query".to_string();
    }
    if !dns.starts_with("http") && !dns.starts_with("tcp") && !dns.starts_with("tls") && dns.parse::<std::net::IpAddr>().is_ok() {
        return format!("https://{}/dns-query", dns);
    }
    dns.to_string()
}

/// Sanitize local DNS: reject "local" (causes TUN DNS loop via svchost.exe interception).
/// Falls back to Google public DNS.
fn sanitize_local_dns(dns: &str) -> String {
    let dns = dns.trim();
    if dns.is_empty() || dns == "local" {
        "8.8.8.8".to_string()
    } else {
        dns.to_string()
    }
}

pub fn decode_base64_content(content: &str) -> String {
    let mut padded = content.replace("-", "+").replace("_", "/");
    match padded.len() % 4 {
        2 => padded.push_str("=="),
        3 => padded.push_str("="),
        _ => {}
    }

    if let Ok(decoded) = general_purpose::STANDARD.decode(&padded) {
        if let Ok(s) = String::from_utf8(decoded) {
            return s;
        }
    }

    if let Ok(decoded) = general_purpose::URL_SAFE.decode(&padded) {
        if let Ok(s) = String::from_utf8(decoded) {
            return s;
        }
    }
    String::new()
}

pub fn is_ip_address(addr: &str) -> bool {
    addr.parse::<std::net::IpAddr>().is_ok()
}

pub fn is_ip_or_cidr(pat: &str) -> bool {
    if let Some((ip, mask)) = pat.split_once('/') {
        ip.parse::<std::net::IpAddr>().is_ok() && mask.parse::<u8>().is_ok()
    } else {
        is_ip_address(pat)
    }
}

pub fn clean_routing_pattern(pat: &str) -> String {
    let mut cleaned = pat.trim_start_matches("http://").trim_start_matches("https://");
    if let Some(idx) = cleaned.find('/') {
        if !is_ip_or_cidr(pat) {
            cleaned = &cleaned[..idx];
        }
    }
    cleaned.to_string()
}

fn parse_query(url: &Url) -> std::collections::HashMap<String, String> {
    url.query_pairs().into_owned().collect()
}

pub fn parse_vless(link: &str) -> Option<Value> {
    let url = Url::parse(link).ok()?;
    let uuid = percent_encoding::percent_decode_str(url.username()).decode_utf8_lossy().to_string();
    let host = url.host_str()?;
    let port = url.port().unwrap_or(443);
    let params = parse_query(&url);

    let mut out = json!({
        "type": "vless",
        "tag": "proxy",
        "server": host,
        "server_port": port,
        "uuid": uuid,
        "packet_encoding": "xudp"
    });

    if let Some(flow) = params.get("flow") {
        if !flow.is_empty() {
            out["flow"] = json!(flow);
        }
    }

    // Preserve VLESS encryption parameter (e.g. mlkem768x25519plus.native.0rtt.xxx)
    if let Some(enc) = params.get("encryption") {
        if !enc.is_empty() {
            out["encryption"] = json!(enc);
        }
    }

    let security = params.get("security").map(|s| s.as_str()).unwrap_or("");
    if security == "tls" {
        let mut tls = json!({
            "enabled": true,
            "server_name": params.get("sni").unwrap_or(&host.to_string()),
            "insecure": params.get("allowInsecure").unwrap_or(&"0".to_string()) == "1",
        });
        if let Some(alpn) = params.get("alpn") {
            tls["alpn"] = json!(alpn.split(',').collect::<Vec<_>>());
        } else {
            let transport = params.get("type").map(|s| s.as_str()).unwrap_or("tcp");
            if transport == "ws" {
                tls["alpn"] = json!(["http/1.1"]);
            } else if transport == "grpc" || transport == "httpupgrade" || transport == "xhttp" || transport == "splithttp" || transport == "h2" || transport == "http" {
                tls["alpn"] = json!(["h2"]);
            }
        }
        if let Some(fp) = params.get("fp") {
            if !fp.is_empty() {
                tls["utls"] = json!({
                    "enabled": true,
                    "fingerprint": fp
                });
            }
        }
        out["tls"] = tls;
    } else if security == "reality" {
        let mut tls = json!({
            "enabled": true,
            "server_name": params.get("sni").unwrap_or(&host.to_string()),
            "reality": {
                "enabled": true,
                "public_key": params.get("pbk").unwrap_or(&"".to_string()),
                "short_id": params.get("sid").unwrap_or(&"".to_string())
            }
        });
        let fp = params.get("fp").map(|s| s.as_str()).unwrap_or("chrome");
        if !fp.is_empty() {
            tls["utls"] = json!({
                "enabled": true,
                "fingerprint": fp
            });
        }
        if let Some(alpn) = params.get("alpn") {
            tls["alpn"] = json!(alpn.split(',').collect::<Vec<_>>());
        } else {
            let transport = params.get("type").map(|s| s.as_str()).unwrap_or("tcp");
            if transport == "ws" {
                tls["alpn"] = json!(["http/1.1"]);
            } else if transport == "grpc" || transport == "httpupgrade" || transport == "xhttp" || transport == "splithttp" || transport == "h2" || transport == "http" {
                tls["alpn"] = json!(["h2"]);
            }
        }
        out["tls"] = tls;
    }

    let transport = params.get("type").map(|s| s.as_str()).unwrap_or("tcp");
    match transport {
        "ws" => {
            out["transport"] = json!({
                "type": "ws",
                "path": params.get("path").unwrap_or(&"/".to_string()),
                "headers": {
                    "Host": params.get("host").unwrap_or(&host.to_string())
                }
            });
        }
        "grpc" => {
            out["transport"] = json!({
                "type": "grpc",
                "service_name": params.get("serviceName").unwrap_or(&"".to_string())
            });
        }
        "httpupgrade" => {
            out["transport"] = json!({
                "type": "httpupgrade",
                "path": params.get("path").unwrap_or(&"/".to_string()),
                "host": params.get("host").unwrap_or(&host.to_string())
            });
        }
        "xhttp" | "splithttp" => {
            let mut xhttp = json!({
                "type": "xhttp",
                "path": params.get("path").unwrap_or(&"/".to_string()),
                "host": params.get("host").unwrap_or(&host.to_string())
            });
            if let Some(mode) = params.get("mode") {
                if !mode.is_empty() {
                    xhttp["mode"] = json!(mode);
                }
            }
            out["transport"] = xhttp;
        }
        "h2" | "http" => {
            out["transport"] = json!({
                "type": "http",
                "host": [params.get("host").unwrap_or(&host.to_string())],
                "path": params.get("path").unwrap_or(&"/".to_string())
            });
        }
        "tcp" | _ => {
            let mut t = json!({
                "type": "tcp"
            });
            if let Some(header) = params.get("headerType") {
                if header == "http" {
                    t["header_type"] = json!("http");
                    let mut host_arr = vec![host.to_string()];
                    if let Some(h) = params.get("host") {
                        if !h.is_empty() {
                            host_arr = vec![h.to_string()];
                        }
                    }
                    t["host"] = json!(host_arr);
                    t["path"] = json!(params.get("path").unwrap_or(&"/".to_string()));
                }
            }
            out["transport"] = t;
        }
    }

    Some(out)
}

pub fn parse_vmess(link: &str) -> Option<Value> {
    let b64 = link.strip_prefix("vmess://")?;
    let decoded = decode_base64_content(b64);
    let v: Value = serde_json::from_str(&decoded).ok()?;

    let host = v.get("add")?.as_str()?;
    let port = v.get("port")?;
    // port could be int or string
    let port_num = if port.is_string() {
        port.as_str().unwrap().parse::<u16>().ok()?
    } else {
        port.as_u64()? as u16
    };

    let mut out = json!({
        "type": "vmess",
        "tag": "proxy",
        "server": host,
        "server_port": port_num,
        "uuid": v.get("id").and_then(|v| v.as_str()).unwrap_or(""),
        "security": "auto",
        "alter_id": v.get("aid").and_then(|v| if v.is_string() { v.as_str().unwrap().parse::<u16>().ok() } else { Some(v.as_u64().unwrap_or(0) as u16) }).unwrap_or(0)
    });

    let tls = v.get("tls").and_then(|v| v.as_str()).unwrap_or("");
    if tls == "tls" {
        let sni = v.get("sni").and_then(|v| v.as_str()).unwrap_or(host);
        out["tls"] = json!({
            "enabled": true,
            "server_name": sni,
            "insecure": true
        });
    }

    let net = v.get("net").and_then(|v| v.as_str()).unwrap_or("tcp");
    match net {
        "ws" => {
            let path = v.get("path").and_then(|v| v.as_str()).unwrap_or("/");
            let host_header = v.get("host").and_then(|v| v.as_str()).unwrap_or(host);
            out["transport"] = json!({
                "type": "ws",
                "path": path,
                "headers": { "Host": host_header }
            });
        }
        "grpc" => {
            let service_name = v.get("path").and_then(|v| v.as_str()).unwrap_or("");
            out["transport"] = json!({
                "type": "grpc",
                "service_name": service_name
            });
        }
        "httpupgrade" => {
            let path = v.get("path").and_then(|v| v.as_str()).unwrap_or("/");
            let host_header = v.get("host").and_then(|v| v.as_str()).unwrap_or(host);
            out["transport"] = json!({
                "type": "httpupgrade",
                "path": path,
                "host": host_header
            });
        }
        "xhttp" | "splithttp" => {
            let path = v.get("path").and_then(|v| v.as_str()).unwrap_or("/");
            let host_header = v.get("host").and_then(|v| v.as_str()).unwrap_or(host);
            let mut xhttp = json!({
                "type": "xhttp",
                "path": path,
                "host": host_header
            });
            if let Some(mode) = v.get("mode") {
                if let Some(m_str) = mode.as_str() {
                    if !m_str.is_empty() {
                        xhttp["mode"] = json!(m_str);
                    }
                }
            }
            out["transport"] = xhttp;
        }
        "h2" | "http" => {
            let path = v.get("path").and_then(|v| v.as_str()).unwrap_or("/");
            let host_header = v.get("host").and_then(|v| v.as_str()).unwrap_or(host);
            out["transport"] = json!({
                "type": "http",
                "host": [host_header],
                "path": path
            });
        }
        "tcp" | _ => {
            let header_type = v.get("type").and_then(|v| v.as_str()).unwrap_or("none");
            let mut t = json!({ "type": "tcp" });
            if header_type == "http" {
                t["header_type"] = json!("http");
                let host_header = v.get("host").and_then(|v| v.as_str()).unwrap_or(host);
                let path = v.get("path").and_then(|v| v.as_str()).unwrap_or("/");
                t["host"] = json!(vec![host_header]);
                t["path"] = json!(path);
            }
            out["transport"] = t;
        }
    }

    Some(out)
}

pub fn parse_ss(link: &str) -> Option<Value> {
    let url = Url::parse(link).ok()?;

    let (method, password, host, port) = if let Some(host) = url.host_str() {
        // format: ss://method:password@host:port
        let raw_user = percent_encoding::percent_decode_str(url.username()).decode_utf8_lossy();
        let decoded_user = decode_base64_content(&raw_user);
        let mut parts = decoded_user.split(':');
        let m = parts.next()?;
        let p = parts.next()?;
        (m.to_string(), p.to_string(), host.to_string(), url.port()?)
    } else {
        // format: ss://base64
        let b64 = link.strip_prefix("ss://")?.split('#').next()?;
        let decoded = decode_base64_content(b64);
        // decoded format: method:password@host:port
        let mut parts = decoded.split('@');
        let auth = parts.next()?;
        let addr = parts.next()?;

        let mut auth_parts = auth.split(':');
        let m = auth_parts.next()?;
        let p = auth_parts.next()?;

        let mut addr_parts = addr.split(':');
        let h = addr_parts.next()?;
        let port_str = addr_parts.next()?;
        (
            m.to_string(),
            p.to_string(),
            h.to_string(),
            port_str.parse().ok()?,
        )
    };

    Some(json!({
        "type": "shadowsocks",
        "tag": "proxy",
        "server": host,
        "server_port": port,
        "method": method,
        "password": password
    }))
}

pub fn parse_trojan(link: &str) -> Option<Value> {
    let url = Url::parse(link).ok()?;
    let password = percent_encoding::percent_decode_str(url.username()).decode_utf8_lossy().to_string();
    let host = url.host_str()?;
    let port = url.port().unwrap_or(443);
    let params = parse_query(&url);

    let mut out = json!({
        "type": "trojan",
        "tag": "proxy",
        "server": host,
        "server_port": port,
        "password": password
    });

    let security = params.get("security").map(|s| s.as_str()).unwrap_or("tls");
    if security == "tls" {
        let mut tls = json!({
            "enabled": true,
            "server_name": params.get("sni").unwrap_or(&host.to_string()),
            "insecure": true
        });
        if let Some(alpn) = params.get("alpn") {
            tls["alpn"] = json!(alpn.split(',').collect::<Vec<_>>());
        } else {
            let transport = params.get("type").map(|s| s.as_str()).unwrap_or("tcp");
            if transport == "ws" {
                tls["alpn"] = json!(["http/1.1"]);
            } else if transport == "grpc" || transport == "httpupgrade" || transport == "xhttp" || transport == "splithttp" || transport == "h2" || transport == "http" {
                tls["alpn"] = json!(["h2"]);
            }
        }
        if let Some(fp) = params.get("fp") {
            if !fp.is_empty() {
                tls["utls"] = json!({
                    "enabled": true,
                    "fingerprint": fp
                });
            }
        }
        out["tls"] = tls;
    } else if security == "reality" {
        let mut tls = json!({
            "enabled": true,
            "server_name": params.get("sni").unwrap_or(&host.to_string()),
            "reality": {
                "enabled": true,
                "public_key": params.get("pbk").unwrap_or(&"".to_string()),
                "short_id": params.get("sid").unwrap_or(&"".to_string())
            }
        });
        let fp = params.get("fp").map(|s| s.as_str()).unwrap_or("chrome");
        if !fp.is_empty() {
            tls["utls"] = json!({
                "enabled": true,
                "fingerprint": fp
            });
        }
        out["tls"] = tls;
    } else if security == "none" || security.is_empty() {
        // No TLS — nothing to add
    } else if security != "" {
        // Default: treat as TLS for unrecognized security values
        out["tls"] = json!({
            "enabled": true,
            "server_name": params.get("sni").unwrap_or(&host.to_string()),
            "insecure": true
        });
    }

    let transport = params.get("type").map(|s| s.as_str()).unwrap_or("tcp");
    match transport {
        "ws" => {
            out["transport"] = json!({
                "type": "ws",
                "path": params.get("path").unwrap_or(&"/".to_string()),
                "headers": {
                    "Host": params.get("host").unwrap_or(&host.to_string())
                }
            });
        }
        "grpc" => {
            out["transport"] = json!({
                "type": "grpc",
                "service_name": params.get("serviceName").unwrap_or(&"".to_string())
            });
        }
        "httpupgrade" => {
            out["transport"] = json!({
                "type": "httpupgrade",
                "path": params.get("path").unwrap_or(&"/".to_string()),
                "host": params.get("host").unwrap_or(&host.to_string())
            });
        }
        "xhttp" | "splithttp" => {
            let mut xhttp = json!({
                "type": "xhttp",
                "path": params.get("path").unwrap_or(&"/".to_string()),
                "host": params.get("host").unwrap_or(&host.to_string())
            });
            if let Some(mode) = params.get("mode") {
                if !mode.is_empty() {
                    xhttp["mode"] = json!(mode);
                }
            }
            out["transport"] = xhttp;
        }
        "h2" | "http" => {
            out["transport"] = json!({
                "type": "http",
                "host": [params.get("host").unwrap_or(&host.to_string())],
                "path": params.get("path").unwrap_or(&"/".to_string())
            });
        }
        "tcp" | _ => {
            if let Some(header) = params.get("headerType") {
                if header == "http" {
                    let mut host_arr = vec![host.to_string()];
                    if let Some(h) = params.get("host") {
                        if !h.is_empty() {
                            host_arr = vec![h.to_string()];
                        }
                    }
                    out["transport"] = json!({
                        "type": "tcp",
                        "header_type": "http",
                        "host": host_arr,
                        "path": params.get("path").unwrap_or(&"/".to_string())
                    });
                }
            }
        }
    }

    Some(out)
}

pub fn parse_hysteria2(link: &str) -> Option<Value> {
    let url = Url::parse(link).ok()?;
    let password = url.username();
    let host = url.host_str()?;
    let port = url.port().unwrap_or(443);
    let params = parse_query(&url);

    Some(json!({
        "type": "hysteria2",
        "tag": "proxy",
        "server": host,
        "server_port": port,
        "password": password,
        "tls": {
            "enabled": true,
            "server_name": params.get("sni").unwrap_or(&host.to_string()),
            "insecure": true
        }
    }))
}

pub fn parse_tuic(link: &str) -> Option<Value> {
    let url = Url::parse(link).ok()?;
    let uuid = url.username();
    let password = url.password().unwrap_or("");
    let host = url.host_str()?;
    let port = url.port().unwrap_or(443);
    let params = parse_query(&url);

    Some(json!({
        "type": "tuic",
        "tag": "proxy",
        "server": host,
        "server_port": port,
        "uuid": uuid,
        "password": password,
        "congestion_control": "bbr",
        "tls": {
            "enabled": true,
            "server_name": params.get("sni").unwrap_or(&host.to_string()),
            "insecure": true
        }
    }))
}

pub fn clean_singbox_proxy(mut proxy: Value) -> Value {
    let mut remove_transport = false;
    if let Some(transport) = proxy.get_mut("transport").and_then(|t| t.as_object_mut()) {
        let t_type = transport.get("type").and_then(|t| t.as_str()).unwrap_or("").to_string();
        if t_type == "tcp" {
            // Plain TCP or TCP+http header: remove entirely for Sing-box
            remove_transport = true;
        } else if t_type == "ws" || t_type == "httpupgrade" {
            if let Some(path_val) = transport.get("path").and_then(|p| p.as_str()) {
                let path_str = path_val.to_string();
                let mut real_path = path_str.clone();
                let mut ed_val: Option<u32> = None;
                
                if let Some(idx) = path_str.find("?ed=") {
                    real_path = path_str[..idx].to_string();
                    let ed_str = &path_str[idx + 4..];
                    if let Ok(ed) = ed_str.split('&').next().unwrap_or("").parse::<u32>() {
                        ed_val = Some(ed);
                    }
                } else if let Some(idx) = path_str.find("&ed=") {
                    real_path = path_str[..idx].to_string();
                    let ed_str = &path_str[idx + 4..];
                    if let Ok(ed) = ed_str.split('&').next().unwrap_or("").parse::<u32>() {
                        ed_val = Some(ed);
                    }
                }
                
                if ed_val.is_some() {
                    if real_path.is_empty() {
                        real_path = "/".to_string();
                    }
                    transport.insert("path".to_string(), json!(real_path));
                }
            }
        } else if t_type == "xhttp" {
            // Remove 'mode' field — it's Xray-only and not valid in Sing-box's xhttp transport
            transport.remove("mode");
        }
    }
    if remove_transport {
        proxy.as_object_mut().unwrap().remove("transport");
    }
    // Remove the encryption field since it's Xray-only and Sing-box rejects it
    if let Some(obj) = proxy.as_object_mut() {
        obj.remove("encryption");
    }
    proxy
}

pub fn parse_link(link: &str) -> Option<Value> {
    let link = link.trim();
    if link.starts_with("vless://") {
        parse_vless(link)
    } else if link.starts_with("vmess://") {
        parse_vmess(link)
    } else if link.starts_with("ss://") {
        parse_ss(link)
    } else if link.starts_with("trojan://") {
        parse_trojan(link)
    } else if link.starts_with("hysteria2://") || link.starts_with("hy2://") {
        parse_hysteria2(link)
    } else if link.starts_with("tuic://") {
        parse_tuic(link)
    } else {
        None
    }
}

pub fn extract_host_and_port(link: &str) -> Option<(String, u16)> {
    if let Some(v) = parse_link(link) {
        if let (Some(server), Some(port)) = (
            v.get("server").and_then(|s| s.as_str()),
            v.get("server_port").and_then(|p| p.as_u64()),
        ) {
            return Some((server.to_string(), port as u16));
        }
    }
    None
}

// ===== Iran domains & IPs for bypass =====
fn iran_domains() -> Vec<&'static str> {
    vec![
        ".ir",
        "digikala.com",
        "snapp.ir",
        "shaparak.ir",
        "saman.ir",
        "ap.ir",
        "irancell.ir",
        "mci.ir",
        "rightel.ir",
        "bale.ai",
        "rubika.ir",
        "eitaa.com",
        "tebyan.net",
        "iribnews.ir",
        "varzesh3.com",
        "telewebion.com",
        "filimo.com",
        "aparat.com",
        "namasha.com",
        "tamasha.com",
        "divar.ir",
        "sheypoor.com",
        "torob.com",
        "emalls.ir",
        "tgju.org",
        "codal.ir",
        "irankhodro.ir",
        "saipacorp.com",
        "snapp.taxi",
        "tapsi.ir",
        "alibaba.ir",
        "flytoday.ir",
        "mrbilit.ir",
        "ghatreh.com",
        "khabaronline.ir",
        "tabnak.ir",
        "farsnews.ir",
        "isna.ir",
        "mehrnews.com",
        "tasnimnews.com",
        "yjc.ir",
        "irna.ir",
        "alef.ir",
        "entekhab.ir",
        "jamejamonline.ir",
        "eghtesadonline.com",
        "boursenews.ir",
        "sena.ir",
        "ifb.ir",
        "tsetmc.com",
        "fipiran.ir",
        "bmi.ir",
        "bankmellat.ir",
        "bpi.ir",
        "banksepah.ir",
        "bsi.ir",
        "tejaratbank.ir",
        "refah-bank.ir",
        "sb24.ir",
        "en-bank.ir",
        "bank-maskan.ir",
        "postbank.ir",
        "keshavarzi.ir",
        "bki.ir",
        "eghtesadnovin.com",
        "bankmelli-iran.com",
        "parsianbank.ir",
        "karafarinbank.ir",
        "sinabank.ir",
        "ghbi.ir",
        "shahr-bank.ir",
        "day24.ir",
        "hekmatbank.ir",
        "middleeastbank.ir",
        "tourismbank.ir",
        "ivbb.ir",
        "ansarbank.ir",
        "melatbank.org",
        "bim.ir",
        "emofid.com",
        "agah.com",
        "exir.io",
        "ramzinex.com",
        "nobitex.ir",
        "wallex.ir",
        "bitpin.ir",
        "aban-tether.com",
    ]
}

fn iran_ip_cidrs() -> Vec<&'static str> {
    vec![
        "2.144.0.0/14",
        "2.176.0.0/12",
        "5.1.43.0/24",
        "5.22.0.0/17",
        "5.22.192.0/21",
        "5.23.112.0/21",
        "5.34.192.0/20",
        "5.42.217.0/24",
        "5.42.223.0/24",
        "5.52.0.0/15",
        "5.56.128.0/22",
        "5.57.32.0/21",
        "5.62.160.0/19",
        "5.63.8.0/21",
        "5.72.0.0/13",
        "5.104.208.0/21",
        "5.106.0.0/16",
        "5.112.0.0/12",
        "5.134.128.0/18",
        "5.160.0.0/16",
        "5.190.0.0/16",
        "5.198.160.0/19",
        "5.200.64.0/18",
        "5.201.128.0/17",
        "5.202.0.0/16",
        "5.208.0.0/12",
        "31.2.0.0/17",
        "31.7.64.0/21",
        "31.14.80.0/20",
        "31.24.200.0/21",
        "31.40.0.0/18",
        "31.56.0.0/14",
        "37.10.64.0/22",
        "37.19.0.0/17",
        "37.32.0.0/14",
        "37.44.56.0/21",
        "37.63.128.0/17",
        "37.98.0.0/17",
        "37.114.192.0/18",
        "37.128.240.0/20",
        "37.129.0.0/16",
        "37.130.200.0/21",
        "37.137.0.0/16",
        "37.143.144.0/21",
        "37.148.0.0/17",
        "37.152.160.0/19",
        "37.153.128.0/22",
        "37.153.176.0/20",
        "37.156.0.0/16",
        "37.191.64.0/19",
        "37.202.0.0/16",
        "37.221.0.0/18",
        "37.228.131.0/24",
        "37.235.16.0/20",
        "37.254.0.0/16",
        "46.18.248.0/21",
        "46.21.80.0/20",
        "46.28.72.0/21",
        "46.32.0.0/19",
        "46.34.96.0/21",
        "46.36.96.0/20",
        "46.38.128.0/18",
        "46.41.192.0/18",
        "46.51.0.0/17",
        "46.62.128.0/17",
        "46.100.0.0/16",
        "46.102.120.0/21",
        "46.102.128.0/17",
        "46.143.0.0/17",
        "46.148.32.0/20",
        "46.164.0.0/16",
        "46.167.128.0/19",
        "46.182.32.0/21",
        "46.209.0.0/16",
        "46.224.0.0/15",
        "46.235.76.0/23",
        "46.245.0.0/17",
        "46.249.96.0/24",
        "46.251.224.0/24",
        "46.255.56.0/21",
        "62.3.14.0/24",
        "62.3.41.0/24",
        "62.3.42.0/24",
        "62.32.49.0/24",
        "62.32.53.0/24",
        "62.32.61.0/24",
        "62.60.128.0/17",
        "62.102.128.0/20",
        "62.133.46.0/24",
        "62.220.96.0/19",
        "63.243.185.0/24",
        "64.214.116.0/23",
        "66.79.96.0/19",
        "69.194.64.0/18",
        "77.36.128.0/17",
        "77.42.0.0/17",
        "77.77.64.0/18",
        "77.81.32.0/20",
        "77.81.76.0/24",
        "77.81.128.0/21",
        "77.81.192.0/20",
        "77.90.0.0/17",
        "77.95.220.0/24",
        "77.104.64.0/18",
        "77.237.64.0/19",
        "77.238.112.0/20",
        "77.245.224.0/20",
        "78.31.184.0/22",
        "78.38.0.0/15",
        "78.110.112.0/20",
        "78.111.0.0/20",
        "78.154.32.0/19",
        "78.157.32.0/19",
        "78.158.0.0/19",
        "79.127.0.0/17",
        "79.132.192.0/18",
        "79.143.84.0/22",
        "79.174.160.0/21",
        "79.175.128.0/18",
        "80.66.176.0/20",
        "80.71.112.0/20",
        "80.75.0.0/20",
        "80.191.0.0/16",
        "80.210.0.0/16",
        "80.235.0.0/17",
        "80.242.0.0/20",
        "80.249.112.0/22",
        "80.253.128.0/19",
        "81.12.0.0/17",
        "81.16.112.0/20",
        "81.28.32.0/19",
        "81.29.240.0/20",
        "81.31.160.0/22",
        "81.31.224.0/22",
        "81.31.248.0/22",
        "81.90.144.0/20",
        "81.91.128.0/19",
        "81.92.216.0/24",
        "82.99.192.0/18",
        "82.138.140.0/25",
        "82.180.192.0/18",
        "83.120.0.0/14",
        "83.147.192.0/23",
        "83.147.194.0/24",
        "83.149.208.0/22",
        "84.47.192.0/18",
        "84.241.0.0/18",
        "85.9.64.0/18",
        "85.15.0.0/18",
        "85.133.128.0/17",
        "85.185.0.0/16",
        "85.198.0.0/19",
        "85.204.80.0/20",
        "85.208.252.0/22",
        "85.239.192.0/19",
        "86.55.0.0/16",
        "86.57.0.0/17",
        "86.104.32.0/20",
        "86.104.80.0/20",
        "86.104.96.0/20",
        "86.104.232.0/21",
        "86.105.40.0/21",
        "86.105.128.0/20",
        "86.106.142.0/24",
        "86.106.186.0/24",
        "86.107.0.0/20",
        "86.107.80.0/20",
        "86.107.144.0/20",
        "86.107.172.0/22",
        "86.107.208.0/20",
        "87.107.0.0/16",
        "87.236.209.0/24",
        "87.236.210.0/23",
        "87.236.214.0/23",
        "87.247.168.0/21",
        "87.248.128.0/19",
        "87.248.141.0/24",
        "87.248.143.0/24",
        "87.248.150.0/24",
        "87.248.152.0/21",
        "87.248.155.0/24",
        "88.131.240.0/20",
        "88.135.32.0/20",
        "89.32.0.0/20",
        "89.33.18.0/23",
        "89.33.100.0/22",
        "89.33.204.0/23",
        "89.34.20.0/23",
        "89.34.88.0/21",
        "89.34.168.0/21",
        "89.34.176.0/23",
        "89.35.132.0/22",
        "89.36.16.0/23",
        "89.36.96.0/21",
        "89.36.176.0/20",
        "89.37.0.0/20",
        "89.37.144.0/20",
        "89.37.168.0/22",
        "89.37.208.0/22",
        "89.37.240.0/20",
        "89.38.80.0/20",
        "89.38.212.0/22",
        "89.39.8.0/22",
        "89.39.208.0/20",
        "89.40.78.0/23",
        "89.40.106.0/23",
        "89.40.110.0/23",
        "89.40.128.0/18",
        "89.41.8.0/21",
        "89.41.16.0/21",
        "89.41.32.0/23",
        "89.41.40.0/21",
        "89.42.32.0/22",
        "89.42.44.0/22",
        "89.42.56.0/22",
        "89.42.68.0/22",
        "89.42.96.0/21",
        "89.42.136.0/22",
        "89.42.196.0/22",
        "89.42.208.0/22",
        "89.42.228.0/22",
        "89.43.0.0/21",
        "89.43.36.0/22",
        "89.43.188.0/22",
        "89.43.204.0/22",
        "89.43.216.0/21",
        "89.44.112.0/22",
        "89.44.128.0/21",
        "89.44.146.0/23",
        "89.44.190.0/23",
        "89.44.202.0/23",
        "89.44.240.0/22",
        "89.45.48.0/20",
        "89.45.68.0/22",
        "89.45.80.0/21",
        "89.45.112.0/22",
        "89.45.126.0/23",
        "89.45.152.0/21",
        "89.46.44.0/22",
        "89.46.60.0/22",
        "89.46.94.0/23",
        "89.46.184.0/21",
        "89.47.64.0/20",
        "89.47.196.0/22",
        "89.47.232.0/22",
        "89.144.128.0/18",
        "89.165.0.0/17",
        "89.196.0.0/16",
        "89.219.64.0/18",
        "89.221.80.0/20",
        "89.235.64.0/18",
        "91.92.108.0/22",
        "91.92.114.0/24",
        "91.92.129.0/24",
        "91.98.0.0/16",
        "91.99.0.0/16",
        "91.106.64.0/19",
        "91.107.128.0/17",
        "91.108.128.0/19",
        "91.109.104.0/21",
        "91.133.128.0/17",
        "91.147.64.0/19",
        "91.184.64.0/19",
        "91.185.128.0/19",
        "91.186.192.0/23",
        "91.186.201.0/24",
        "91.186.216.0/23",
        "91.186.218.0/24",
        "91.190.88.0/21",
        "91.199.18.0/24",
        "91.199.27.0/24",
        "91.199.30.0/24",
        "91.208.165.0/24",
        "91.209.96.0/24",
        "91.209.179.0/24",
        "91.209.183.0/24",
        "91.210.104.0/24",
        "91.211.88.0/24",
        "91.212.16.0/24",
        "91.212.252.0/24",
        "91.213.151.0/24",
        "91.213.157.0/24",
        "91.213.172.0/24",
        "91.216.4.0/23",
        "91.217.64.0/23",
        "91.220.79.0/24",
        "91.220.113.0/24",
        "91.220.243.0/24",
        "91.221.240.0/23",
        "91.222.196.0/22",
        "91.223.16.0/24",
        "91.224.20.0/23",
        "91.224.110.0/23",
        "91.225.52.0/24",
        "91.226.2.0/23",
        "91.226.36.0/23",
        "91.227.84.0/22",
        "91.227.246.0/23",
        "91.228.22.0/23",
        "91.228.132.0/23",
        "91.228.189.0/24",
        "91.229.46.0/23",
        "91.229.214.0/23",
        "91.230.32.0/24",
        "91.232.64.0/22",
        "91.232.68.0/23",
        "91.233.56.0/22",
        "91.234.52.0/24",
        "91.236.168.0/23",
        "91.237.254.0/23",
        "91.238.0.0/24",
        "91.239.14.0/24",
        "91.239.108.0/22",
        "91.240.60.0/22",
        "91.240.116.0/24",
        "91.240.180.0/22",
        "91.241.20.0/23",
        "91.242.44.0/23",
        "91.243.126.0/24",
        "91.243.160.0/21",
        "91.244.120.0/22",
        "91.245.228.0/22",
        "91.246.44.0/24",
        "91.247.66.0/23",
        "91.247.171.0/24",
        "91.247.174.0/24",
        "91.248.0.0/21",
        "91.250.224.0/20",
        "91.251.0.0/16",
        "92.42.48.0/21",
        "92.43.160.0/22",
        "92.61.176.0/20",
        "92.114.16.0/20",
        "92.242.192.0/19",
        "92.246.144.0/22",
        "92.246.156.0/22",
        "93.110.0.0/16",
        "93.113.224.0/20",
        "93.114.16.0/20",
        "93.114.104.0/21",
        "93.115.120.0/21",
        "93.115.224.0/21",
        "93.117.0.0/16",
        "93.118.96.0/19",
        "93.118.180.0/22",
        "93.119.32.0/19",
        "93.126.0.0/18",
        "94.24.0.0/17",
        "94.74.128.0/18",
        "94.101.0.0/19",
        "94.101.128.0/20",
        "94.101.176.0/20",
        "94.101.240.0/20",
        "94.139.160.0/19",
        "94.176.8.0/21",
        "94.176.32.0/21",
        "94.177.0.0/20",
        "94.182.0.0/15",
        "94.184.0.0/16",
        "94.199.136.0/22",
        "94.232.168.0/21",
        "94.241.164.0/22",
        "95.38.0.0/16",
        "95.64.0.0/17",
        "95.80.128.0/18",
        "95.81.64.0/18",
        "95.130.56.0/21",
        "95.142.224.0/20",
        "95.156.220.0/22",
        "95.156.226.0/24",
        "95.156.232.0/21",
        "95.156.248.0/22",
        "95.156.252.0/23",
        "95.162.0.0/16",
        "188.0.240.0/20",
        "188.75.64.0/18",
        "188.94.188.0/24",
        "188.95.68.0/24",
        "188.118.64.0/18",
        "188.121.96.0/19",
        "188.122.96.0/19",
        "188.136.128.0/17",
        "188.158.0.0/15",
        "188.159.112.0/20",
        "188.173.0.0/16",
        "188.174.0.0/16",
        "188.175.0.0/17",
        "188.176.0.0/14",
        "188.191.176.0/21",
        "188.208.56.0/21",
        "188.208.64.0/19",
        "188.208.144.0/20",
        "188.208.176.0/20",
        "188.208.210.0/23",
        "188.209.8.0/21",
        "188.209.192.0/19",
        "188.210.64.0/20",
        "188.210.80.0/21",
        "188.210.96.0/19",
        "188.210.128.0/18",
        "188.210.192.0/20",
        "188.210.232.0/22",
        "188.211.0.0/20",
        "188.211.32.0/20",
        "188.211.48.0/21",
        "188.211.57.0/24",
        "188.211.58.0/24",
        "188.211.61.0/24",
        "188.211.96.0/19",
        "188.211.128.0/17",
        "188.212.22.0/24",
        "188.212.48.0/20",
        "188.212.64.0/19",
        "188.212.128.0/17",
        "188.213.64.0/20",
        "188.213.96.0/19",
        "188.213.192.0/18",
        "188.214.4.0/22",
        "188.214.84.0/22",
        "188.214.96.0/22",
        "188.214.160.0/19",
        "188.215.24.0/22",
        "188.215.88.0/22",
        "188.215.128.0/20",
        "188.215.160.0/19",
        "188.215.192.0/19",
        "188.215.235.0/24",
        "188.215.240.0/22",
        "188.229.0.0/17",
        "188.240.196.0/24",
        "188.240.212.0/22",
        "188.240.248.0/21",
        "188.253.2.0/23",
        "188.253.32.0/19",
        "188.253.64.0/18",
        "193.0.156.0/24",
        "193.3.31.0/24",
        "193.3.182.0/24",
        "193.8.139.0/24",
        "193.19.144.0/23",
        "193.22.20.0/24",
        "193.28.181.0/24",
        "193.29.24.0/24",
        "193.29.26.0/24",
        "193.32.80.0/22",
        "193.34.244.0/22",
        "193.35.62.0/24",
        "193.38.247.0/24",
        "193.39.9.0/24",
        "193.56.59.0/24",
        "193.56.61.0/24",
        "193.56.107.0/24",
        "193.56.118.0/24",
        "193.104.22.0/24",
        "193.104.29.0/24",
        "193.104.212.0/24",
        "193.105.2.0/24",
        "193.105.6.0/24",
        "193.105.234.0/24",
        "193.106.190.0/24",
        "193.107.48.0/22",
        "193.111.234.0/23",
        "193.141.64.0/23",
        "193.142.30.0/24",
        "193.142.232.0/23",
        "193.148.64.0/22",
        "193.150.66.0/24",
        "193.151.128.0/19",
        "193.162.129.0/24",
        "193.176.240.0/22",
        "193.178.200.0/22",
        "193.186.32.0/21",
        "193.189.122.0/23",
        "193.200.102.0/24",
        "193.200.148.0/24",
        "193.201.72.0/23",
        "193.222.51.0/24",
        "193.228.90.0/23",
        "193.228.136.0/24",
        "193.242.125.0/24",
        "193.242.194.0/23",
        "193.242.208.0/23",
        "194.5.40.0/22",
        "194.5.175.0/24",
        "194.5.176.0/22",
        "194.5.195.0/24",
        "194.5.205.0/24",
        "194.9.56.0/23",
        "194.9.80.0/23",
        "194.26.2.0/23",
        "194.26.20.0/23",
        "194.26.117.0/24",
        "194.26.195.0/24",
        "194.33.104.0/22",
        "194.33.122.0/23",
        "194.33.124.0/22",
        "194.36.0.0/24",
        "194.36.174.0/24",
        "194.50.204.0/24",
        "194.50.209.0/24",
        "194.50.216.0/24",
        "194.50.218.0/24",
        "194.53.118.0/23",
        "194.53.122.0/24",
        "194.56.148.0/22",
        "194.59.170.0/23",
        "194.60.208.0/22",
        "194.60.228.0/22",
        "194.62.17.0/24",
        "194.62.43.0/24",
        "194.143.140.0/24",
        "194.146.148.0/22",
        "194.147.164.0/22",
        "194.150.68.0/24",
        "195.2.234.0/24",
        "195.8.102.0/24",
        "195.13.104.0/24",
        "195.28.10.0/23",
        "195.28.168.0/23",
        "195.88.188.0/23",
        "195.110.32.0/23",
        "195.146.32.0/19",
        "195.181.0.0/19",
        "195.190.130.0/24",
        "195.191.22.0/23",
        "195.191.44.0/23",
        "195.211.44.0/22",
        "195.214.235.0/24",
        "195.225.232.0/24",
        "195.226.219.0/24",
        "195.230.105.0/24",
        "195.230.107.0/24",
        "195.230.124.0/24",
        "195.234.191.0/24",
        "196.3.91.0/24",
        "213.57.32.0/19",
        "213.108.240.0/23",
        "213.108.242.0/24",
        "213.109.192.0/18",
        "213.176.0.0/19",
        "213.176.64.0/18",
        "213.176.192.0/19",
        "213.195.0.0/20",
        "213.207.192.0/18",
        "213.217.32.0/19",
        "213.232.124.0/22",
        "213.233.160.0/19",
        "217.11.16.0/20",
        "217.24.17.0/24",
        "217.24.144.0/20",
        "217.25.48.0/20",
        "217.60.0.0/16",
        "217.66.192.0/19",
        "217.77.112.0/20",
        "217.144.104.0/22",
        "217.146.208.0/20",
        "217.170.240.0/20",
        "217.171.145.0/24",
        "217.172.98.0/23",
        "217.172.102.0/23",
        "217.172.104.0/21",
        "217.172.116.0/22",
        "217.172.120.0/21",
        "217.174.16.0/20",
        "217.218.0.0/15",
    ]
}

fn ad_domains() -> Vec<&'static str> {
    vec![
        "doubleclick.net",
        "adservice.google.com",
        "googlesyndication.com",
        "googleadservices.com",
        "google-analytics.com",
        "googletagmanager.com",
        "googletagservices.com",
        "pagead2.googlesyndication.com",
        "adnxs.com",
        "adsrvr.org",
        "adsymptotic.com",
        "adtechus.com",
        "advertising.com",
        "amazon-adsystem.com",
        "amgdgt.com",
        "app-measurement.com",
        "appsflyer.com",
        "aps.amazon.com",
        "bidswitch.net",
        "branch.io",
        "casalemedia.com",
        "chartbeat.com",
        "criteo.com",
        "criteo.net",
        "demdex.net",
        "everesttech.net",
        "exelator.com",
        "facebook.net",
        "fbcdn.net",
        "hotjar.com",
        "indexww.com",
        "intentiq.com",
        "krxd.net",
        "liadm.com",
        "lijit.com",
        "mathtag.com",
        "media.net",
        "mediavine.com",
        "moatads.com",
        "mookie1.com",
        "newrelic.com",
        "nr-data.net",
        "omtrdc.net",
        "openx.net",
        "outbrain.com",
        "pardot.com",
        "pubmatic.com",
        "quantserve.com",
        "rfihub.com",
        "rlcdn.com",
        "rubiconproject.com",
        "scorecardresearch.com",
        "segment.com",
        "segment.io",
        "serving-sys.com",
        "sharethrough.com",
        "smaato.net",
        "smartadserver.com",
        "taboola.com",
        "tapad.com",
        "tidaltv.com",
        "turn.com",
        "yahoo.com/ads",
        "yieldmo.com",
        "ads.yahoo.com",
    ]
}

/// Build routing rules for Sing-box format from user settings
pub fn build_singbox_routing_rules(settings: &crate::core::settings::Settings) -> Vec<Value> {
    let mut rules = vec![];

    // Bypass Windows NCSI
    rules.push(json!({
        "domain_suffix": [
            "msftconnecttest.com",
            "msftncsi.com"
        ],
        "outbound": "direct"
    }));

    // Bypass LAN (private IPs)
    if settings.bypass_lan {
        rules.push(json!({
            "ip_is_private": true,
            "outbound": "direct"
        }));
    }

    // Bypass Iran
    if settings.bypass_iran {
        // Domain rules
        let domains: Vec<String> = iran_domains().iter().map(|d| d.to_string()).collect();
        rules.push(json!({
            "domain_suffix": domains,
            "outbound": "direct"
        }));

        // IP rules
        let ips: Vec<String> = iran_ip_cidrs().iter().map(|ip| ip.to_string()).collect();
        rules.push(json!({
            "ip_cidr": ips,
            "outbound": "direct"
        }));
    }

    // Block Ads
    if settings.block_ads {
        let domains: Vec<String> = ad_domains().iter().map(|d| d.to_string()).collect();
        rules.push(json!({
            "domain_suffix": domains,
            "outbound": "block"
        }));
    }

    // Custom user rules
    for rule in &settings.routing_rules {
        if !rule.enabled {
            continue;
        }
        let outbound = match rule.action.as_str() {
            "bypass" | "direct" => "direct",
            "block" => "block",
            _ => continue,
        };

        let pattern = rule.pattern.trim();
        if pattern.is_empty() {
            continue;
        }

        if is_ip_or_cidr(pattern) {
            rules.push(json!({
                "ip_cidr": [pattern],
                "outbound": outbound
            }));
        } else {
            // Domain pattern - use domain_suffix for bare domains (matches itself and subdomains)
            rules.push(json!({
                "domain_suffix": [pattern.trim_start_matches("*.")],
                "outbound": outbound
            }));
        }
    }

    rules
}

/// Build routing rules for Xray format from user settings
pub fn build_xray_routing_rules(settings: &crate::core::settings::Settings) -> Vec<Value> {
    let mut rules = vec![];

    // Bypass Windows NCSI
    rules.push(json!({
        "type": "field",
        "domain": [
            "msftconnecttest.com",
            "msftncsi.com"
        ],
        "outboundTag": "direct"
    }));

    // Bypass LAN
    if settings.bypass_lan {
        rules.push(json!({
            "type": "field",
            "ip": [
                "geoip:private",
                "0.0.0.0/8",
                "10.0.0.0/8",
                "100.64.0.0/10",
                "127.0.0.0/8",
                "169.254.0.0/16",
                "172.16.0.0/12",
                "192.0.0.0/24",
                "192.168.0.0/16",
                "198.18.0.0/15",
                "::1/128",
                "fe80::/10"
            ],
            "outboundTag": "direct"
        }));
    }

    // Bypass Iran
    if settings.bypass_iran {
        // Domain rules
        let domains: Vec<String> = iran_domains().iter().map(|d| format!("domain:{}", d)).collect();
        rules.push(json!({
            "type": "field",
            "domain": domains,
            "outboundTag": "direct"
        }));

        // IP rules
        let ips: Vec<String> = iran_ip_cidrs().iter().map(|ip| ip.to_string()).collect();
        rules.push(json!({
            "type": "field",
            "ip": ips,
            "outboundTag": "direct"
        }));
    }

    // Block Ads
    if settings.block_ads {
        let domains: Vec<String> = ad_domains().iter().map(|d| format!("domain:{}", d)).collect();
        rules.push(json!({
            "type": "field",
            "domain": domains,
            "outboundTag": "block"
        }));
    }

    // Custom user rules
    for rule in &settings.routing_rules {
        if !rule.enabled {
            continue;
        }
        let outbound_tag = match rule.action.as_str() {
            "bypass" | "direct" => "direct",
            "block" => "block",
            _ => continue,
        };

        let pattern = rule.pattern.trim();
        if pattern.is_empty() {
            continue;
        }

        if is_ip_or_cidr(pattern) {
            rules.push(json!({
                "type": "field",
                "ip": [pattern],
                "outboundTag": outbound_tag
            }));
        } else {
            rules.push(json!({
                "type": "field",
                "domain": [format!("domain:{}", pattern.trim_start_matches("*."))],
                "outboundTag": outbound_tag
            }));
        }
    }

    rules
}

pub fn generate_config(
    link: &str,
    port: u16,
    settings: Option<&crate::core::settings::Settings>,
) -> Result<(Value, Option<String>, bool), String> {
    let parsed_proxy = parse_link(link).ok_or_else(|| "Invalid or unsupported proxy link".to_string())?;

    let mut use_xray = false;
    if let Some(transport) = parsed_proxy.get("transport") {
        let t_type = transport.get("type").and_then(|t| t.as_str()).unwrap_or("");
        let h_type = transport
            .get("header_type")
            .and_then(|h| h.as_str())
            .unwrap_or("");
        if t_type == "xhttp" || h_type == "http" {
            use_xray = true;
        }
    }

    if let Some(s) = settings {
        if s.core_choice == "xray" {
            use_xray = true;
        }
    }

    if use_xray {
        if let Some((mut cfg, ip)) = generate_xray_config(link, port, settings) {
            cfg["__core__"] = json!("xray");
            return Ok((cfg, Some(ip), true));
        }
    }

    let mut proxy = clean_singbox_proxy(parsed_proxy);

    proxy["tag"] = json!("proxy");
    let mut rules = vec![json!({"protocol": "dns", "outbound": "dns-out"})];

    // Inject user routing rules (bypass iran, bypass lan, block ads, custom rules)
    if let Some(s) = settings {
        rules.extend(build_singbox_routing_rules(s));
    }
    // Read the proxy server domain BEFORE any IP resolution (needed for DNS rules below)
    let proxy_server_domain = proxy
        .get("server")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();

    use std::net::ToSocketAddrs;
    let proxy_ip = proxy
        .get("server")
        .and_then(|s| s.as_str())
        .map(|s| (s, port).to_socket_addrs().ok().and_then(|mut a| a.next()).map(|a| a.ip().to_string()).unwrap_or(s.to_string()));

    // Do NOT overwrite proxy["server"] with the resolved IP.
    // Sing-box must resolve the proxy domain itself using dns_local (direct detour)
    // so it picks the optimal CDN edge for the user's location.

    let sb_remote = settings.map(|s| s.remote_dns.as_str()).filter(|s| !s.is_empty()).unwrap_or("https://1.1.1.1/dns-query").to_string();
    let sb_local = settings.map(|s| s.local_dns.as_str()).filter(|s| !s.is_empty()).unwrap_or("local").to_string();

    let mut dns_rules = vec![];
    // All internal queries from outbounds (like 'proxy' or 'fragment' resolving their own domains) MUST use dns_local
    // This avoids DNS loopbacks because it prevents them from falling back to dns_remote, which detours to proxy.
    dns_rules.push(json!({
        "outbound": ["proxy", "fragment", "any"],
        "server": "dns_local"
    }));
    // Extract domain from sb_remote (e.g. https://dns.google/dns-query -> dns.google)
    if let Ok(url) = url::Url::parse(&sb_remote) {
        if let Some(host) = url.host_str() {
            if host.parse::<std::net::IpAddr>().is_err() {
                dns_rules.push(json!({"domain": [host], "server": "dns_local"}));
            }
        }
    }
    dns_rules.push(json!({"outbound": ["direct", "block"], "server": "dns_local"}));

    Ok((
        json!({
            "log": {
                "level": "info",
                "timestamp": true
            },
            "experimental": {
                "clash_api": {
                    "external_controller": "127.0.0.1:9090"
                }
            },
            "dns": {
                "servers": [
                    {"tag": "dns_remote", "address": sb_remote, "detour": "proxy"},
                    {"tag": "dns_local", "address": sb_local, "detour": "direct"}
                ],
                "rules": dns_rules,
                "final": "dns_remote",
                "strategy": "ipv4_only"
            },
            "inbounds": [
                {
                    "type": "mixed",
                    "tag": "mixed-in",
                    "listen": "127.0.0.1",
                    "listen_port": port,
                    "sniff": true,
                    "sniff_override_destination": true
                }
            ],
            "outbounds": [
                proxy,
                {"type": "direct", "tag": "direct"},
                {"type": "block", "tag": "block"},
                {"type": "dns", "tag": "dns-out"}
            ],
            "route": {
                "rules": rules,
                "auto_detect_interface": true,
                "final": "proxy"
            }
        }),
        proxy_ip,
        false,
    ))
}

pub fn generate_tun_config(
    port: u16,
    link: &str,
    exact_ip: Option<String>,
    is_xray: bool,
    strict_route: bool,
    remote_dns: String,
    local_dns: String,
    settings: Option<&crate::core::settings::Settings>,
) -> Value {
    // Sanitize DNS for TUN mode (see sanitize_remote_dns / sanitize_local_dns docs)
    let remote_dns = sanitize_remote_dns(&remote_dns);
    let local_dns = sanitize_local_dns(&local_dns);

    let mut server_ips = vec![];
    let mut server_domain = None;

    if let Some(ip) = exact_ip {
        if let Ok(parsed_ip) = ip.parse::<std::net::IpAddr>() {
            let mask = if parsed_ip.is_ipv4() { "32" } else { "128" };
            server_ips.push(format!("{}/{}", ip, mask));
        }
    }

    if let Some((host, _)) = extract_host_and_port(link) {
        if is_ip_address(&host) {
            if let Ok(parsed_ip) = host.parse::<std::net::IpAddr>() {
                let mask = if parsed_ip.is_ipv4() { "32" } else { "128" };
                server_ips.push(format!("{}/{}", host, mask));
            }
        } else {
            server_domain = Some(host);
        }
    }

    server_ips.sort();
    server_ips.dedup();

    let mut route_rules = vec![
        json!({"protocol": "dns", "outbound": "dns-out"}),
        json!({
            "process_name": [
                "xray.exe",
                "sing-box.exe",
                "stat-tracker.exe"
            ],
            "outbound": "direct"
        }),
        json!({
            "ip_cidr": [
                "224.0.0.0/3"
            ],
            "outbound": "block",
            "source_ip_cidr": [
                "224.0.0.0/3"
            ]
        }),
    ];

    // All custom routing (ads, iran, user rules) is handled by the MAIN proxy core.
    // The TUN sidecar only needs to bypass private IPs, the proxy itself, and forward the rest.

    route_rules.push(json!({
        "ip_cidr": [
            "0.0.0.0/8",
            "10.0.0.0/8",
            "100.64.0.0/10",
            "127.0.0.0/8",
            "169.254.0.0/16",
            "172.16.0.0/12",
            "192.0.0.0/24",
            "192.0.2.0/24",
            "192.88.99.0/24",
            "192.168.0.0/16",
            "198.18.0.0/15",
            "198.51.100.0/24",
            "203.0.113.0/24",
            "224.0.0.0/4",
            "240.0.0.0/4",
            "255.255.255.255/32"
        ],
        "outbound": "direct"
    }));

    let server_ips_clone = server_ips.clone();
    for ip in server_ips {
        route_rules.push(json!({"ip_cidr": [ip], "outbound": "direct"}));
    }
    if let Some(domain) = &server_domain {
        route_rules.push(json!({"domain": [domain.clone()], "outbound": "direct"}));
    }

    // Bypass Windows NCSI domains to prevent "No internet access" on restricted proxies
    route_rules.push(json!({
        "domain_suffix": [
            "msftconnecttest.com",
            "msftncsi.com"
        ],
        "outbound": "direct"
    }));

    if let Some(s) = settings {
        route_rules.extend(build_singbox_routing_rules(s));
    }

    // Build route_exclude_address: must include loopback (127.0.0.0/8) so that
    // strict_route WFP filters don't intercept the SOCKS connection to the local main core.
    let exclude_addrs = vec!["127.0.0.0/8".to_string()];

    let mut inbounds = vec![json!({
        "type": "tun",
        "tag": "tun-in",
        "interface_name": "Vxray",
        "address": [
            "172.19.0.1/30"
        ],
        "mtu": 9000,
        "auto_route": true,
        "strict_route": strict_route,
        "route_exclude_address": exclude_addrs,
        "endpoint_independent_nat": true,
        "stack": "gvisor",
        "sniff": true,
        "sniff_override_destination": true
    })];

    let mut dns_rules = vec![];
    if let Some(domain) = &server_domain {
        dns_rules.push(json!({"domain": [domain], "server": "dns-direct"}));
    }
    if let Ok(url) = url::Url::parse(&remote_dns) {
        if let Some(host) = url.host_str() {
            if host.parse::<std::net::IpAddr>().is_err() {
                dns_rules.push(json!({"domain": [host], "server": "dns-direct"}));
            }
        }
    }
    dns_rules.push(json!({
        "domain_suffix": [
            "msftconnecttest.com",
            "msftncsi.com"
        ],
        "server": "dns-ncsi"
    }));
    dns_rules.push(json!({"outbound": ["direct", "block"], "server": "dns-direct"}));

    let mut config = json!({
        "log": {
            "level": "warn",
            "timestamp": true
        },
        "dns": {
            "servers": [
                {"tag": "dns-remote", "address": if remote_dns.is_empty() { "1.1.1.1" } else { &remote_dns }, "detour": "proxy"},
                {"tag": "dns-direct", "address": if local_dns.is_empty() { "local" } else { &local_dns }, "detour": "direct"},
                {"tag": "dns-ncsi", "address": "8.8.8.8", "detour": "direct"}
            ],
            "rules": dns_rules,
            "final": "dns-remote",
            "strategy": "ipv4_only"
        },
        "inbounds": inbounds,
        "outbounds": [
            {
                "type": "socks",
                "tag": "proxy",
                "server": "127.0.0.1",
                "server_port": port,
                "version": "5"
            },
            {"type": "direct", "tag": "direct"},
            {"type": "block", "tag": "block"},
            {"type": "dns", "tag": "dns-out"}
        ],
        "route": {
            "rules": route_rules,
            "auto_detect_interface": true,
            "final": "proxy"
        }
    });

    config
}

pub fn generate_mass_test_config(links: Vec<String>, start_port: u16) -> Value {
    let mut inbounds = vec![];
    let mut outbounds = vec![
        json!({"type": "direct", "tag": "direct"}),
        json!({"type": "block", "tag": "block"}),
        json!({"type": "dns", "tag": "dns-out"}),
    ];
    let mut rules = vec![json!({"protocol": "dns", "outbound": "dns-out"})];

    for (i, link) in links.iter().enumerate() {
        let tag = format!("proxy-{}", i);
        if let Some(mut proxy) = parse_link(link).map(clean_singbox_proxy) {
            proxy["tag"] = json!(tag.clone());
            outbounds.push(proxy);

            let port = start_port + (i as u16);
            inbounds.push(json!({
                "type": "mixed",
                "tag": format!("in-{}", i),
                "listen": "127.0.0.1",
                "listen_port": port
            }));

            rules.push(json!({
                "inbound": format!("in-{}", i),
                "outbound": tag
            }));
        }
    }

    json!({
        "log": {"level": "fatal"},
        "dns": {
            "servers": [
                {"tag": "dns_local", "address": "local", "detour": "direct"}
            ]
        },
        "inbounds": inbounds,
        "outbounds": outbounds,
        "route": {
            "rules": rules,
            "auto_detect_interface": true
        }
    })
}

pub fn generate_single_test_config(link: &str, port: u16) -> Value {
    if let Some(parsed) = parse_link(link) {
        if let Some(transport) = parsed.get("transport") {
            let t_type = transport.get("type").and_then(|t| t.as_str()).unwrap_or("");
            let h_type = transport
                .get("header_type")
                .and_then(|h| h.as_str())
                .unwrap_or("");
            if t_type == "xhttp" || h_type == "http" {
                if let Some((cfg, _)) = generate_xray_config(link, port, None) {
                    return cfg;
                }
            }
        }
    }
    generate_mass_test_config(vec![link.to_string()], port)
}
pub fn generate_xray_config(
    link: &str,
    port: u16,
    settings: Option<&crate::core::settings::Settings>,
) -> Option<(Value, String)> {
    let sb = parse_link(link)?;

    let protocol = sb.get("type").and_then(|t| t.as_str())?;
    if protocol != "vless" && protocol != "vmess" && protocol != "trojan" {
        return None;
    }

    let host = sb.get("server").and_then(|s| s.as_str())?;
    let server_port = sb
        .get("server_port")
        .and_then(|p| p.as_u64())
        .map(|p| p as u16)
        .unwrap_or(443);

    use std::net::ToSocketAddrs;
    let proxy_ip = (host, server_port).to_socket_addrs().ok().and_then(|mut a| a.next()).map(|a| a.ip().to_string()).unwrap_or(host.to_string());

    let mut stream_settings = json!({
        "network": "tcp",
        "security": "none"
    });

    if let Some(transport) = sb.get("transport") {
        let net = transport
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("tcp");

        // Map transport types to Xray network names
        let xray_net = match net {
            "xhttp" | "splithttp" => "xhttp",
            "http" | "h2" => "h2",
            other => other,  // httpupgrade, ws, grpc, tcp stay as-is
        };
        stream_settings["network"] = json!(xray_net);

        if xray_net == "tcp" {
            if let Some(header_type) = transport.get("header_type").and_then(|h| h.as_str()) {
                if header_type == "http" {
                    let mut t_host = transport.get("host").unwrap_or(&json!([host])).clone();
                    if !t_host.is_array() {
                        t_host = json!([t_host]);
                    }
                    stream_settings["tcpSettings"] = json!({
                        "header": {
                            "type": "http",
                            "request": {
                                "version": "1.1",
                                "method": "GET",
                                "path": [transport.get("path").unwrap_or(&json!("/"))],
                                "headers": {
                                    "Host": t_host,
                                    "User-Agent": ["Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/115.0.0.0 Safari/537.36"]
                                }
                            },
                            "response": {
                                "version": "1.1",
                                "status": "200",
                                "reason": "OK",
                                "headers": {
                                    "Content-Type": ["application/octet-stream", "application/x-msdownload", "text/html", "application/vnd.sputnik.interactive"],
                                    "Transfer-Encoding": ["chunked"],
                                    "Connection": ["keep-alive"],
                                    "Pragma": ["no-cache"]
                                }
                            }
                        }
                    });
                }
            }
        } else if xray_net == "ws" {
            let mut ws_host = transport
                .get("headers")
                .and_then(|h| h.get("Host"))
                .unwrap_or(&json!(host))
                .clone();
            if ws_host.is_array() {
                ws_host = ws_host[0].clone();
            }
            stream_settings["wsSettings"] = json!({
                "path": transport.get("path").unwrap_or(&json!("/")),
                "headers": {
                    "Host": ws_host
                }
            });
        } else if xray_net == "grpc" {
            stream_settings["grpcSettings"] = json!({
                "serviceName": transport.get("service_name").unwrap_or(&json!("")),
                "multiMode": false
            });
        } else if xray_net == "httpupgrade" {
            let mut hu_host = transport.get("host").unwrap_or(&json!(host)).clone();
            if hu_host.is_array() {
                hu_host = hu_host[0].clone();
            }
            stream_settings["httpupgradeSettings"] = json!({
                "host": hu_host,
                "path": transport.get("path").unwrap_or(&json!("/"))
            });
        } else if xray_net == "xhttp" {
            let mut t_host = transport.get("host").unwrap_or(&json!(host)).clone();
            if t_host.is_array() {
                t_host = t_host[0].clone();
            }
            let mode = transport.get("mode")
                .and_then(|m| m.as_str())
                .unwrap_or("auto");
            stream_settings["xhttpSettings"] = json!({
                "mode": mode,
                "host": t_host,
                "path": transport.get("path").unwrap_or(&json!("/"))
            });
        } else if xray_net == "h2" {
            let mut h2_host = transport.get("host").unwrap_or(&json!([host])).clone();
            if !h2_host.is_array() {
                h2_host = json!([h2_host]);
            }
            stream_settings["httpSettings"] = json!({
                "host": h2_host,
                "path": transport.get("path").unwrap_or(&json!("/"))
            });
        }
    }

    if let Some(tls) = sb.get("tls") {
        if tls.get("reality").is_some() {
            let reality = tls.get("reality").unwrap();
            stream_settings["security"] = json!("reality");
            stream_settings["realitySettings"] = json!({
                "serverName": tls.get("server_name").unwrap_or(&json!(host)),
                "publicKey": reality.get("public_key").unwrap_or(&json!("")),
                "shortId": reality.get("short_id").unwrap_or(&json!("")),
                "spiderX": "/",
            });
            if let Some(utls) = tls.get("utls") {
                stream_settings["realitySettings"]["fingerprint"] =
                    utls.get("fingerprint").unwrap_or(&json!("chrome")).clone();
            }
        } else if tls
            .get("enabled")
            .and_then(|b| b.as_bool())
            .unwrap_or(false)
        {
            stream_settings["security"] = json!("tls");
            let mut tls_settings = json!({
                "serverName": tls.get("server_name").unwrap_or(&json!(host))
            });
            if let Some(alpn) = tls.get("alpn") {
                tls_settings["alpn"] = alpn.clone();
            }
            if let Some(utls) = tls.get("utls") {
                tls_settings["fingerprint"] =
                    utls.get("fingerprint").unwrap_or(&json!("chrome")).clone();
            }
            stream_settings["tlsSettings"] = tls_settings;
        }
    }

    let outbound_settings = if protocol == "vless" {
        let vless_encryption = sb.get("encryption")
            .and_then(|e| e.as_str())
            .unwrap_or("none");
        let mut user = json!({
            "id": sb.get("uuid").unwrap_or(&json!("")),
            "encryption": vless_encryption,
            "packetEncoding": "xudp"
        });
        if let Some(flow) = sb.get("flow") {
            if flow.as_str().unwrap_or("") != "" {
                user["flow"] = flow.clone();
            }
        }
        json!({
            "vnext": [{
                "address": proxy_ip,
                "port": server_port,
                "users": [user]
            }]
        })
    } else if protocol == "vmess" {
        json!({
            "vnext": [{
                "address": proxy_ip,
                "port": server_port,
                "users": [{
                    "id": sb.get("uuid").unwrap_or(&json!("")),
                    "alterId": sb.get("alter_id").unwrap_or(&json!(0)),
                    "security": sb.get("security").unwrap_or(&json!("auto"))
                }]
            }]
        })
    } else {
        // trojan
        json!({
            "servers": [{
                "address": proxy_ip,
                "port": server_port,
                "password": sb.get("password").unwrap_or(&json!(""))
            }]
        })
    };

    let mut outbounds = vec![
        json!({
            "tag": "proxy",
            "protocol": protocol,
            "settings": outbound_settings,
            "streamSettings": stream_settings
        }),
        json!({"protocol": "freedom", "tag": "direct"}),
        json!({"protocol": "blackhole", "tag": "block"}),
    ];

    let mut rules: Vec<Value> = vec![];

    // Inject user routing rules (bypass iran, bypass lan, block ads, custom rules)
    if let Some(s) = settings {
        rules.extend(build_xray_routing_rules(s));
    }

    Some((
        json!({
            "__core__": "xray",
            "log": {
                "loglevel": "warning"
            },
            "inbounds": if settings.is_some() {
                vec![json!({
                    "listen": "127.0.0.1",
                    "port": port + 1,
                    "protocol": "socks",
                    "settings": { "udp": true },
                    "sniffing": { "enabled": true, "destOverride": ["http", "tls"] }
                })]
            } else {
                vec![
                    json!({
                        "listen": "127.0.0.1",
                        "port": port,
                        "protocol": "http",
                        "settings": { "allowTransparent": false },
                        "sniffing": { "enabled": true, "destOverride": ["http", "tls"] }
                    }),
                    json!({
                        "listen": "127.0.0.1",
                        "port": port + 1,
                        "protocol": "socks",
                        "settings": { "udp": true },
                        "sniffing": { "enabled": true, "destOverride": ["http", "tls"] }
                    })
                ]
            },
            "outbounds": outbounds,
            "routing": {
                "domainStrategy": "AsIs",
                "rules": rules
            }
        }),
        proxy_ip,
    ))
}
