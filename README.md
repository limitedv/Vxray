<div align="center">
  <h1> 🔒Vxray</h1>
  <p><strong>The Next-Generation, Ultra-Secure Proxy Client for Windows</strong></p>
</div>

Vxray is a highly-polished, modern proxy client built from the ground up with **Rust** and **Tauri**. Powered by the blazing-fast Sing-box core, Vxray delivers a premium user experience and military-grade network security without the bloated, confusing interfaces of traditional proxy clients. 

Whether you need a simple system proxy or a leak-proof VPN experience (TUN mode), Vxray offers maximum performance with minimum configuration.

---

## ✨ Why Choose Vxray over old V2ray clients?

Traditional clients are powerful, but they suffer from outdated, cluttered interfaces, steep learning curves, and manual configuration requirements to prevent IP leaks. **Vxray solves all of this.**

### 🎨 Premium, Modern User Interface
Say goodbye to spreadsheet-like menus and clunky drop-downs. Vxray features a stunning, dynamic, glassmorphic UI. It’s designed to be intuitive enough for beginners, yet powerful enough for advanced users. It feels like a native, premium app—not a developer tool.

### 🪶 Ultra-Lightweight (Rust + Tauri)
Unlike Electron apps or heavy Qt/WPF frameworks, Vxray runs on the Tauri engine backed by Rust. This means it consumes a fraction of the RAM and CPU, keeping your PC lightning fast while maintaining robust background connections.

### 🛡️ Iron-Clad Security & Leak Protection (Out of the Box!)
We designed Vxray to be **watertight** without forcing you to write complex routing rules. 
- **True IPv6 Leak Protection:** Vxray binds a ULA IPv6 address to the virtual adapter, forcing Windows to secure *all* IPv6 traffic through the proxy engine instead of leaking your real IP.
- **Strict Route Kill Switch:** If your VPN connection falters, Vxray's strict routing engine physically prevents Windows from falling back to your unencrypted Wi-Fi/Ethernet adapter.
- **Zero DNS Leaks:** Vxray automatically intercepts rogue DNS requests and forces them through Cloudflare's DoH (`1.1.1.1`) inside the encrypted tunnel.
- **Smart LAN Bypass:** Access your local printers, routers, and smart TVs seamlessly. Vxray automatically bypasses `192.168.x.x` and `10.x.x.x` traffic out of the encrypted tunnel.

### ⚡ True-Routing Mass Latency Testing
Instead of relying on basic ICMP pings that can be spoofed, Vxray performs lightning-fast, concurrent HTTP latency tests *through* the proxy layer to servers like Google. This guarantees you are connecting to the fastest, genuinely available node.

### 🔧 Effortless System Integration
- **Invisible Startup:** Vxray can boot directly into your system tray when Windows starts without flashing windows on your screen.
- **Smart Admin Elevation:** When you enable TUN mode, Vxray seamlessly requests Administrator privileges with a beautiful UI overlay, and cleanly restarts itself to handle virtual adapters.

## 🌐 Supported Protocols
Powered by the Sing-box core, Vxray supports all modern, censorship-resistant protocols:
- **VLESS** (Reality, TCP, WS, gRPC, HTTPUpgrade, xhttp)
- **VMess** (TCP, WS, gRPC, HTTPUpgrade)
- **Trojan**
- **Shadowsocks**
- **Hysteria2**
- **TUIC**

---
*Ready to upgrade your network security? Head over to the Releases page to download the latest setup installer!*
