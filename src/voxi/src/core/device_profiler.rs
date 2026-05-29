//! Device profiler — collects device hardware/software profile information.

use serde_json::{json, Value};

pub struct DeviceProfiler;

impl Default for DeviceProfiler {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceProfiler {
    pub fn new() -> Self {
        DeviceProfiler
    }

    pub fn get_profile(&self) -> Value {
        let mut profile = json!({});

        // CPU info
        #[cfg(target_os = "macos")]
        {
            if let Ok(out) = std::process::Command::new("sysctl").args(["-n", "hw.ncpu"]).output() {
                if let Ok(cores_str) = String::from_utf8(out.stdout) {
                    if let Ok(cores) = cores_str.trim().parse::<usize>() {
                        profile["cpu_cores"] = json!(cores);
                    }
                }
            }
            if let Ok(out) = std::process::Command::new("sysctl").args(["-n", "machdep.cpu.brand_string"]).output() {
                let model = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !model.is_empty() {
                    profile["cpu_model"] = json!(model);
                }
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
                let cores = cpuinfo.matches("processor").count();
                profile["cpu_cores"] = json!(cores);
                for line in cpuinfo.lines() {
                    if line.starts_with("model name") {
                        if let Some(name) = line.split(':').nth(1) {
                            profile["cpu_model"] = json!(name.trim());
                            break;
                        }
                    }
                }
            }
        }

        // Memory info
        #[cfg(target_os = "macos")]
        {
            if let Ok(out) = std::process::Command::new("sysctl").args(["-n", "hw.memsize"]).output() {
                if let Ok(mem_str) = String::from_utf8(out.stdout) {
                    if let Ok(bytes) = mem_str.trim().parse::<u64>() {
                        profile["memory_mb"] = json!(bytes / 1024 / 1024);
                    }
                }
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
                for line in meminfo.lines() {
                    if line.starts_with("MemTotal:") {
                        let kb: u64 = line
                            .split_whitespace()
                            .nth(1)
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(0);
                        profile["memory_mb"] = json!(kb / 1024);
                        break;
                    }
                }
            }
        }

        // OS version
        #[cfg(target_os = "macos")]
        {
            if let Ok(out) = std::process::Command::new("sw_vers").arg("-productVersion").output() {
                let ver = String::from_utf8_lossy(&out.stdout).trim().to_string();
                profile["os_version"] = json!(format!("macOS {}", ver));
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
                for line in content.lines() {
                    if let Some(val) = line.strip_prefix("PRETTY_NAME=") {
                        profile["os_version"] = json!(val.trim_matches('"').to_string());
                        break;
                    }
                }
            }
        }

        profile
    }
}
