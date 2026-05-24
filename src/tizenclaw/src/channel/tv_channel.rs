//! TV Channel — standalone dashboard launcher on port 9092.
//!
//! Exposes a TV UI/API stream. Works exactly like web_dashboard but starts
//! tizenclaw-web-dashboard child process with --port 9092 and --name tv.

use super::{Channel, ChannelConfig};
use serde_json::json;
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const MAX_OUTBOUND_MESSAGES: usize = 200;

pub struct TvChannel {
    name: String,
    port: u16,
    localhost_only: bool,
    web_root: PathBuf,
    config_dir: PathBuf,
    data_dir: PathBuf,
    child_pid: Option<u32>,
    running: Arc<AtomicBool>,
    monitor: Option<std::thread::JoinHandle<()>>,
}

impl TvChannel {
    const PROCESS_COMM_NAME: &'static str = "tizenclaw-tv"; // Truncated to 15 chars for Linux

    pub fn new(config: &ChannelConfig) -> Self {
        let port = config
            .settings
            .get("port")
            .and_then(|v| v.as_u64())
            .unwrap_or(9092) // default port for TV
            as u16;
        let localhost_only = config
            .settings
            .get("localhost_only")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let data_dir = crate::core::runtime_paths::default_data_dir();
        let default_web_root = data_dir.join("web");
        let web_root = config
            .settings
            .get("web_root")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or(default_web_root);
        let config_dir = data_dir.join("config");

        TvChannel {
            name: config.name.clone(),
            port,
            localhost_only,
            web_root,
            config_dir,
            data_dir,
            child_pid: None,
            running: Arc::new(AtomicBool::new(false)),
            monitor: None,
        }
    }

    fn find_binary() -> PathBuf {
        if let Ok(exe) = std::env::current_exe() {
            let candidate = exe.with_file_name("tizenclaw-web-dashboard");
            if candidate.exists() {
                return candidate;
            }
        }
        PathBuf::from("tizenclaw-web-dashboard")
    }

    fn cleanup_stale_processes() {
        let _ = std::process::Command::new("pkill")
            .args(["-TERM", "-f", "tizenclaw-web-dashboard --port 9092"])
            .status();
        std::thread::sleep(std::time::Duration::from_millis(250));
        let _ = std::process::Command::new("pkill")
            .args(["-KILL", "-f", "tizenclaw-web-dashboard --port 9092"])
            .status();
    }

    fn outbound_queue_path(&self) -> PathBuf {
        self.data_dir.join("outbound").join("tv.jsonl")
    }

    fn persist_outbound_message(&self, msg: &str) -> Result<(), String> {
        let text = msg.trim();
        if text.is_empty() {
            return Err("TV outbound message cannot be empty".to_string());
        }

        let path = self.outbound_queue_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let mut entries = if let Ok(content) = std::fs::read_to_string(&path) {
            content
                .lines()
                .filter_map(|line| {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let record = json!({
            "id": format!("tv-{}", now_ms),
            "channel": "tv",
            "title": "TizenClaw",
            "message": text,
            "created_at_ms": now_ms,
        });
        entries.push(record.to_string());
        if entries.len() > MAX_OUTBOUND_MESSAGES {
            let start = entries.len() - MAX_OUTBOUND_MESSAGES;
            entries = entries.split_off(start);
        }

        let mut file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
        for entry in entries {
            writeln!(file, "{}", entry).map_err(|e| e.to_string())?;
        }

        Ok(())
    }
}

impl Channel for TvChannel {
    fn name(&self) -> &str {
        &self.name
    }

    fn start(&mut self) -> bool {
        if self.is_running() {
            return true;
        }

        self.cleanup_monitor();
        Self::cleanup_stale_processes();

        let bin = Self::find_binary();
        let mut cmd = std::process::Command::new(&bin);
        cmd.arg("--port")
            .arg(self.port.to_string())
            .arg("--name")
            .arg("tv")
            .arg("--web-root")
            .arg(&self.web_root)
            .arg("--config-dir")
            .arg(&self.config_dir)
            .arg("--data-dir")
            .arg(&self.data_dir);
        if self.localhost_only {
            cmd.arg("--localhost-only");
        }
        unsafe {
            cmd.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        cmd.stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit());

        match cmd.spawn() {
            Ok(child) => {
                let pid = child.id();
                let running = Arc::clone(&self.running);
                running.store(true, Ordering::SeqCst);
                let monitor = std::thread::spawn(move || {
                    let mut child = child;
                    let status = child.wait();
                    running.store(false, Ordering::SeqCst);
                    match status {
                        Ok(status) => {
                            log::info!("TV process exited with status {}", status);
                        }
                        Err(err) => {
                            log::warn!("TV process wait failed: {}", err);
                        }
                    }
                });
                log::info!(
                    "TV process started (pid {}, port {})",
                    pid,
                    self.port
                );
                self.child_pid = Some(pid);
                self.monitor = Some(monitor);
                true
            }
            Err(e) => {
                log::error!(
                    "Failed to spawn tizenclaw-web-dashboard ({}): {}",
                    bin.display(),
                    e
                );
                false
            }
        }
    }

    fn stop(&mut self) {
        if let Some(pid) = self.child_pid.take() {
            let pgid = -(pid as libc::pid_t);
            unsafe {
                libc::kill(pgid, libc::SIGTERM);
            }
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
            loop {
                if !self.running.load(Ordering::SeqCst) {
                    break;
                }
                if std::time::Instant::now() >= deadline {
                    unsafe {
                        libc::kill(pgid, libc::SIGKILL);
                    }
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            self.running.store(false, Ordering::SeqCst);
            self.cleanup_monitor();
            log::info!("TV process stopped");
        }
    }

    fn is_running(&self) -> bool {
        if !self.running.load(Ordering::SeqCst) {
            return false;
        }

        let Some(pid) = self.child_pid else {
            return false;
        };

        unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
    }

    fn send_message(&self, msg: &str) -> Result<(), String> {
        self.persist_outbound_message(msg)
    }

    fn configure(&mut self, settings: &serde_json::Value) -> Result<(), String> {
        if let Some(port) = settings.get("port") {
            let port = port
                .as_u64()
                .ok_or_else(|| "TV port must be a number".to_string())?;
            if !(1..=65535).contains(&port) {
                return Err("TV port must be between 1 and 65535".to_string());
            }
            self.port = port as u16;
        }

        if let Some(localhost_only) = settings.get("localhost_only") {
            self.localhost_only = localhost_only
                .as_bool()
                .ok_or_else(|| "localhost_only must be a boolean".to_string())?;
        }

        Ok(())
    }
}

impl TvChannel {
    fn cleanup_monitor(&mut self) {
        if let Some(handle) = self.monitor.take() {
            let _ = handle.join();
        }
    }
}
