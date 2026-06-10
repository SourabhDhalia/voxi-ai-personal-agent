//! Health monitor — tracks system resource usage.

#[cfg(not(target_os = "macos"))]
use std::fs;

pub struct HealthStatus {
    pub memory_used_kb: u64,
    pub memory_total_kb: u64,
    pub cpu_load_percent: f64,
    pub uptime_secs: u64,
}

pub struct HealthMonitor {
    start_time: std::time::Instant,
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthMonitor {
    pub fn new() -> Self {
        HealthMonitor {
            start_time: std::time::Instant::now(),
        }
    }

    /// Get current system health snapshot.
    pub fn get_status(&self) -> HealthStatus {
        let (mem_used, mem_total) = read_meminfo();
        let cpu_load = read_loadavg();
        HealthStatus {
            memory_used_kb: mem_total.saturating_sub(mem_used),
            memory_total_kb: mem_total,
            cpu_load_percent: cpu_load,
            uptime_secs: self.start_time.elapsed().as_secs(),
        }
    }
}

#[cfg(target_os = "macos")]
fn read_meminfo() -> (u64, u64) {
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
    
    let available_kb = available_bytes / 1024;
    let total_kb = total_bytes / 1024;
    (available_kb, total_kb)
}

#[cfg(not(target_os = "macos"))]
fn read_meminfo() -> (u64, u64) {
    let content = fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut total = 0u64;
    let mut available = 0u64;
    for line in content.lines() {
        if line.starts_with("MemTotal:") {
            total = parse_kb_value(line);
        } else if line.starts_with("MemAvailable:") {
            available = parse_kb_value(line);
        }
    }
    (available, total)
}

#[cfg(not(target_os = "macos"))]
fn parse_kb_value(line: &str) -> u64 {
    line.split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

#[cfg(target_os = "macos")]
fn read_loadavg() -> f64 {
    let mut loadavg = [0.0f64; 3];
    let count = unsafe { libc::getloadavg(loadavg.as_mut_ptr(), 3) };
    if count > 0 {
        loadavg[0]
    } else {
        0.0
    }
}

#[cfg(not(target_os = "macos"))]
fn read_loadavg() -> f64 {
    let content = fs::read_to_string("/proc/loadavg").unwrap_or_default();
    content
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0)
}
