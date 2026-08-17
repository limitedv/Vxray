use base64::{engine::general_purpose, Engine as _};

use serde_json::{json, Value};
use std::net::ToSocketAddrs;
use url::Url;

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
    addr.parse::<std::net::Ipv4Addr>().is_ok()
}

fn parse_query(url: &Url) -> std::collections::HashMap<String, String> {
    url.query_pairs().into_owned().collect()
}

pub fn parse_vless(link: &str) -> Option<Value> {
    let url = Url::parse(link).ok()?;
    let uuid = url.username();
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
            } else if transport == "grpc" || transport == "httpupgrade" || transport == "xhttp" {
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
            } else if transport == "grpc" || transport == "httpupgrade" || transport == "xhttp" {
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
        },
        "grpc" => {
            out["transport"] = json!({
                "type": "grpc",
                "service_name": params.get("serviceName").unwrap_or(&"".to_string())
            });
        },
        "httpupgrade" | "xhttp" => {
            out["transport"] = json!({
                "type": "httpupgrade",
                "path": params.get("path").unwrap_or(&"/".to_string()),
                "host": params.get("host").unwrap_or(&host.to_string())
            });
        },
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
                        "type": "http",
                        "host": host_arr,
                        "path": params.get("path").unwrap_or(&"/".to_string())
                    });
                }
            }
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
        },
        "grpc" => {
            let service_name = v.get("path").and_then(|v| v.as_str()).unwrap_or("");
            out["transport"] = json!({
                "type": "grpc",
                "service_name": service_name
            });
        },
        "httpupgrade" | "xhttp" => {
            let path = v.get("path").and_then(|v| v.as_str()).unwrap_or("/");
            let host_header = v.get("host").and_then(|v| v.as_str()).unwrap_or(host);
            out["transport"] = json!({
                "type": "httpupgrade",
                "path": path,
                "host": host_header
            });
        },
        "tcp" | _ => {
            let header_type = v.get("type").and_then(|v| v.as_str()).unwrap_or("none");
            if header_type == "http" {
                let host_header = v.get("host").and_then(|v| v.as_str()).unwrap_or(host);
                let path = v.get("path").and_then(|v| v.as_str()).unwrap_or("/");
                out["transport"] = json!({
                    "type": "http",
                    "host": vec![host_header],
                    "path": path
                });
            }
        }
    }

    Some(out)
}

pub fn parse_ss(link: &str) -> Option<Value> {
    let url = Url::parse(link).ok()?;
    
    let (method, password, host, port) = if let Some(host) = url.host_str() {
        // format: ss://method:password@host:port
        let decoded_user = decode_base64_content(url.username());
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
        (m.to_string(), p.to_string(), h.to_string(), port_str.parse().ok()?)
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
    let password = url.username();
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
        },
        "grpc" => {
            out["transport"] = json!({
                "type": "grpc",
                "service_name": params.get("serviceName").unwrap_or(&"".to_string())
            });
        },
        _ => {}
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
        if let (Some(server), Some(port)) = (v.get("server").and_then(|s| s.as_str()), v.get("server_port").and_then(|p| p.as_u64())) {
            return Some((server.to_string(), port as u16));
        }
    }
    None
}

pub fn generate_config(link: &str, port: u16) -> (Value, Option<String>, bool) {
    if link.contains("type=xhttp") {
        if let Some((cfg, ip)) = generate_xray_config(link, port) {
            return (cfg, Some(ip), true);
        }
    }
    
    let mut proxy = parse_link(link).unwrap_or(json!({"type":"direct","tag":"proxy"}));
    
    if let Some(server) = proxy.get("server").and_then(|s| s.as_str()) {
        if !is_ip_address(server) {
            let port_num = proxy.get("server_port").and_then(|p| p.as_u64()).unwrap_or(443) as u16;
            if let Ok(addrs) = format!("{}:{}", server, port_num).to_socket_addrs() {
                let addr = addrs.clone().find(|a| a.is_ipv4()).or_else(|| addrs.into_iter().next());
                if let Some(a) = addr {
                    proxy["server"] = json!(a.ip().to_string());
                }
            }
        }
    }
    proxy["tag"] = json!("proxy");
    let proxy_ip = proxy.get("server").and_then(|s| s.as_str()).map(|s| s.to_string());

    (json!({
        "log": {
            "level": "info",
            "timestamp": true
        },
        "dns": {
            "servers": [
                {"tag": "dns_remote", "address": "https://1.1.1.1/dns-query", "detour": "proxy"},
                {"tag": "dns_local", "address": "local", "detour": "direct"}
            ],
            "rules": [
                {"outbound": "proxy", "server": "dns_local"},
                {"outbound": "any", "server": "dns_remote"}
            ],
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
            "rules": [
                {"protocol": "dns", "outbound": "dns-out"}
            ],
            "auto_detect_interface": true,
            "final": "proxy"
        }
    }), proxy_ip, false)
}

pub fn generate_tun_config(port: u16, link: &str, exact_ip: Option<String>, is_xray: bool) -> Value {
    let mut server_ips = vec![];
    let mut server_domain = None;
    
    if let Some(ip) = exact_ip {
        if let Ok(parsed_ip) = ip.parse::<std::net::IpAddr>() {
            let mask = if parsed_ip.is_ipv4() { "32" } else { "128" };
            server_ips.push(format!("{}/{}", ip, mask));
        }
    }
    
    if let Ok(url) = url::Url::parse(link) {
        if let Some(host) = url.host_str() {
            let server_port = url.port().unwrap_or(443);
            if is_ip_address(host) {
                let mask = if host.contains(':') { "128" } else { "32" };
                server_ips.push(format!("{}/{}", host, mask));
            } else {
                server_domain = Some(host.to_string());
                if let Ok(addrs) = format!("{}:{}", host, server_port).to_socket_addrs() {
                    for addr in addrs {
                        let mask = if addr.is_ipv4() { "32" } else { "128" };
                        server_ips.push(format!("{}/{}", addr.ip(), mask));
                    }
                }
            }
        }
    }
    
    let mut route_rules = vec![
        json!({
            "protocol": "dns",
            "outbound": "dns-out"
        }),
        json!({
            "process_name": [
                "xray.exe",
                "sing-box.exe"
            ],
            "outbound": "direct"
        }),
        json!({
            "ip_cidr": [
                "127.0.0.0/8",
                "192.168.0.0/16",
                "10.0.0.0/8",
                "172.16.0.0/12",
                "fc00::/7",
                "fe80::/10",
                "::1/128"
            ],
            "outbound": "direct"
        })
    ];
    
    let server_ips_clone = server_ips.clone();
    for ip in server_ips {
        route_rules.push(json!({"ip_cidr": [ip], "outbound": "direct"}));
    }
    if let Some(domain) = &server_domain {
        route_rules.push(json!({"domain": [domain.clone()], "outbound": "direct"}));
    }

    let mut dns_rules = vec![];
    if let Some(domain) = &server_domain {
        dns_rules.push(json!({
            "domain": [domain.clone()],
            "server": "dns-direct"
        }));
    }
    dns_rules.push(json!({
        "outbound": "any",
        "server": "dns-remote"
    }));

    json!({
        "log": {
            "level": "warn",
            "timestamp": true
        },
        "dns": {
            "servers": [
                {"tag": "dns-remote", "address": "https://1.1.1.1/dns-query", "detour": "proxy"},
                {"tag": "dns-remote-backup", "address": "https://8.8.8.8/dns-query", "detour": "proxy"},
                {"tag": "dns-direct", "address": "local", "detour": "direct"}
            ],
            "rules": dns_rules,
            "final": "dns-remote",
            "strategy": "ipv4_only"
        },
        "inbounds": [
            {
                "type": "tun",
                "tag": "tun-in",
                "interface_name": "Vxray",
                "inet4_address": "172.19.0.1/30",
                "inet6_address": "fdfe:dcba:9876::1/126",
                "mtu": 9000,
                "auto_route": true,
                "strict_route": true,
                "route_exclude_address": server_ips_clone,
                "endpoint_independent_nat": true,
                "stack": "gvisor",
                "sniff": true,
                "sniff_override_destination": true
            }
        ],
        "outbounds": [
            {
                "type": "socks",
                "tag": "proxy",
                "server": "127.0.0.1",
                "server_port": if is_xray { port + 1 } else { port },
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
    })
}

pub fn generate_mass_test_config(links: Vec<String>, start_port: u16) -> Value {
    let mut inbounds = vec![];
    let mut outbounds = vec![
        json!({"type": "direct", "tag": "direct"}),
        json!({"type": "block", "tag": "block"}),
        json!({"type": "dns", "tag": "dns-out"})
    ];
    let mut rules = vec![
        json!({"protocol": "dns", "outbound": "dns-out"})
    ];

    for (i, link) in links.iter().enumerate() {
        let tag = format!("proxy-{}", i);
        if let Some(mut proxy) = parse_link(link) {
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
    if link.contains("type=xhttp") {
        if let Some((cfg, _)) = generate_xray_config(link, port) {
            return cfg;
        }
    }
    generate_mass_test_config(vec![link.to_string()], port)
}
pub fn generate_xray_config(link: &str, port: u16) -> Option<(Value, String)> {
    let url = url::Url::parse(link).ok()?;
    let uuid = url.username();
    let host = url.host_str()?;
    let server_port = url.port().unwrap_or(443);
    let params: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();

    let mut proxy_ip = host.to_string();
    if !is_ip_address(host) {
        if let Ok(addrs) = format!("{}:{}", host, server_port).to_socket_addrs() {
            let addr = addrs.clone().find(|a| a.is_ipv4()).or_else(|| addrs.into_iter().next());
            if let Some(a) = addr {
                proxy_ip = a.ip().to_string();
            }
        }
    }

    let mut stream_settings = json!({
        "network": "xhttp",
        "security": "tls",
        "tlsSettings": {
            "serverName": params.get("sni").unwrap_or(&host.to_string()),
            "fingerprint": params.get("fp").unwrap_or(&"chrome".to_string())
        },
        "xhttpSettings": {
            "mode": params.get("mode").unwrap_or(&"auto".to_string()),
            "host": params.get("host").unwrap_or(&host.to_string()),
            "path": params.get("path").unwrap_or(&"/".to_string()),
        }
    });

    if let Some(alpn) = params.get("alpn") {
        stream_settings["tlsSettings"]["alpn"] = json!(alpn.split(',').collect::<Vec<_>>());
    } else {
        stream_settings["tlsSettings"]["alpn"] = json!(["h2"]);
    }

    if let Some(pcs) = params.get("pcs") {
        stream_settings["tlsSettings"]["pinnedPeerCertSha256"] = json!(pcs.to_string());
    }

    Some((json!({
        "__core__": "xray",
        "log": {
            "loglevel": "warning"
        },
        "inbounds": [
            {
                "listen": "127.0.0.1",
                "port": port,
                "protocol": "mixed",
                "settings": {
                    "udp": true
                },
                "sniffing": {
                    "enabled": true,
                    "destOverride": ["http", "tls"]
                }
            },
            {
                "listen": "127.0.0.1",
                "port": port + 1,
                "protocol": "socks",
                "settings": {
                    "udp": true
                },
                "sniffing": {
                    "enabled": true,
                    "destOverride": ["http", "tls"]
                }
            }
        ],
        "outbounds": [
            {
                "protocol": "vless",
                "settings": {
                    "vnext": [
                        {
                            "address": proxy_ip,
                            "port": server_port,
                            "users": [
                                {
                                    "id": uuid,
                                    "encryption": "none",
                                    "packetEncoding": "xudp"
                                }
                            ]
                        }
                    ]
                },
                "streamSettings": stream_settings
            }
        ]
    }), proxy_ip))
}

