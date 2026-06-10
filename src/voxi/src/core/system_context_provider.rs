//! System context provider — gathers device/system state for LLM context.

use serde_json::{json, Value};

pub struct SystemContextProvider {
    cached_context: Option<Value>,
    last_update: std::time::Instant,
}

impl Default for SystemContextProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemContextProvider {
    pub fn new() -> Self {
        SystemContextProvider {
            cached_context: None,
            last_update: std::time::Instant::now(),
        }
    }

    pub fn start(&mut self) {
        self.refresh();
        log::info!("SystemContextProvider ready");
    }

    /// Get current system context (cached for 30s).
    pub fn get_context(&mut self) -> Value {
        if self.last_update.elapsed().as_secs() > 30 {
            self.refresh();
        }
        self.cached_context.clone().unwrap_or(json!({}))
    }

    fn refresh(&mut self) {
        let mut ctx = json!({});

        // Time
        ctx["current_time"] = json!(chrono_now());
        ctx["timezone"] = json!(get_timezone());

        // Battery
        #[cfg(target_os = "macos")]
        if let Some(level) = get_macos_battery() {
            ctx["battery_level"] = json!(level);
        }
        #[cfg(not(target_os = "macos"))]
        if let Some(level) = read_sys_file("/sys/class/power_supply/battery/capacity") {
            ctx["battery_level"] = json!(level.trim());
        }

        // Network
        ctx["network_available"] = json!(std::net::TcpStream::connect("8.8.8.8:53")
            .map(|_| true)
            .unwrap_or(false));

        // Hostname
        #[cfg(target_os = "macos")]
        if let Some(name) = get_macos_hostname() {
            ctx["hostname"] = json!(name);
        }
        #[cfg(not(target_os = "macos"))]
        if let Ok(name) = std::fs::read_to_string("/etc/hostname") {
            ctx["hostname"] = json!(name.trim());
        }

        // Memory
        #[cfg(target_os = "macos")]
        {
            let mut total_bytes = 0u64;
            let mut page_size = 4096u64;
            
            if let Ok(out) = std::process::Command::new("sysctl")
                .args(&["-n", "hw.memsize"])
                .output() {
                let text = String::from_utf8_lossy(&out.stdout);
                total_bytes = text.trim().parse().unwrap_or(0);
            }
            
            if let Ok(out) = std::process::Command::new("sysctl")
                .args(&["-n", "vm.pagesize"])
                .output() {
                let text = String::from_utf8_lossy(&out.stdout);
                page_size = text.trim().parse().unwrap_or(4096);
            }
            
            let mut pages_free = 0u64;
            let mut pages_inactive = 0u64;
            let mut pages_speculative = 0u64;
            let mut pages_purgeable = 0u64;
            
            if let Ok(out) = std::process::Command::new("vm_stat").output() {
                let text = String::from_utf8_lossy(&out.stdout);
                for line in text.lines() {
                    if let Some(pos) = line.find(':') {
                        let key = line[..pos].trim();
                        let mut val_str = line[pos + 1..].trim();
                        if val_str.ends_with('.') {
                            val_str = &val_str[..val_str.len() - 1];
                        }
                        let val = val_str.trim().parse::<u64>().unwrap_or(0);
                        match key {
                            "Pages free" => pages_free = val,
                            "Pages inactive" => pages_inactive = val,
                            "Pages speculative" => pages_speculative = val,
                            "Pages purgeable" => pages_purgeable = val,
                            _ => {}
                        }
                    }
                }
            }
            
            let available_pages = pages_free + pages_inactive + pages_speculative + pages_purgeable;
            let available_bytes = available_pages * page_size;
            
            let available_mb = available_bytes / (1024 * 1024);
            let total_mb = total_bytes / (1024 * 1024);
            if total_mb > 0 {
                ctx["memory_total_mb"] = json!(total_mb);
                ctx["memory_available_mb"] = json!(available_mb);
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            if let Some(meminfo) = read_sys_file("/proc/meminfo") {
                let mut total_kb = 0u64;
                let mut avail_kb = 0u64;
                for line in meminfo.lines() {
                    if line.starts_with("MemTotal:") {
                        total_kb = parse_kb(line);
                    } else if line.starts_with("MemAvailable:") {
                        avail_kb = parse_kb(line);
                    }
                }
                if total_kb > 0 {
                    ctx["memory_total_mb"] = json!(total_kb / 1024);
                    ctx["memory_available_mb"] = json!(avail_kb / 1024);
                }
            }
        }

        // Disk
        // SAFETY: `buf` is written by `statvfs`, and the path is a static
        // NUL-terminated C string literal with process lifetime.
        let statvfs = unsafe {
            let mut buf: libc::statvfs = std::mem::zeroed();
            if libc::statvfs(c"/".as_ptr(), &mut buf) == 0 {
                Some(buf)
            } else {
                None
            }
        };
        if let Some(s) = statvfs {
            let block_size = s.f_frsize as u64;
            let total = (s.f_blocks as u64).saturating_mul(block_size) / (1024 * 1024);
            let free = (s.f_bfree as u64).saturating_mul(block_size) / (1024 * 1024);
            ctx["disk_total_mb"] = json!(total);
            ctx["disk_free_mb"] = json!(free);
        }

        self.cached_context = Some(ctx);
        self.last_update = std::time::Instant::now();
    }

    /// Format context as a string for system prompt injection.
    pub fn format_for_prompt(&mut self) -> String {
        let ctx = self.get_context();
        let mut parts = vec![];
        if let Some(t) = ctx.get("current_time").and_then(|v| v.as_str()) {
            parts.push(format!("Current time: {}", t));
        }
        if let Some(b) = ctx.get("battery_level").and_then(|v| v.as_str()) {
            parts.push(format!("Battery: {}%", b));
        }
        if let Some(n) = ctx.get("network_available").and_then(|v| v.as_bool()) {
            parts.push(format!(
                "Network: {}",
                if n { "connected" } else { "offline" }
            ));
        }
        if let Some(m) = ctx.get("memory_available_mb").and_then(|v| v.as_u64()) {
            parts.push(format!("Free memory: {}MB", m));
        }
        if parts.is_empty() {
            return String::new();
        }
        format!("[System Context]\n{}", parts.join("\n"))
    }
}

fn chrono_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Simple ISO-ish format
    format!("{}", secs)
}

#[cfg(target_os = "macos")]
fn get_timezone() -> String {
    if let Ok(path) = std::fs::read_link("/etc/localtime") {
        if let Some(path_str) = path.to_str() {
            if let Some(idx) = path_str.find("zoneinfo/") {
                return path_str[idx + 9..].to_string();
            }
        }
    }
    "UTC".to_string()
}

#[cfg(not(target_os = "macos"))]
fn get_timezone() -> String {
    std::fs::read_to_string("/etc/timezone")
        .unwrap_or_else(|_| "UTC".into())
        .trim()
        .to_string()
}

#[cfg(target_os = "macos")]
fn get_macos_battery() -> Option<String> {
    if let Ok(out) = std::process::Command::new("pmset")
        .args(&["-g", "batt"])
        .output() {
        let text = String::from_utf8_lossy(&out.stdout);
        if let Some(pos) = text.find('%') {
            let s = &text[..pos];
            if let Some(start) = s.rfind(|c: char| !c.is_digit(10)) {
                return Some(s[start + 1..].to_string());
            } else {
                return Some(s.to_string());
            }
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn get_macos_hostname() -> Option<String> {
    if let Ok(out) = std::process::Command::new("hostname").output() {
        return Some(String::from_utf8_lossy(&out.stdout).trim().to_string());
    }
    None
}

#[cfg(not(target_os = "macos"))]
fn read_sys_file(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

#[cfg(not(target_os = "macos"))]
fn parse_kb(line: &str) -> u64 {
    line.split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

