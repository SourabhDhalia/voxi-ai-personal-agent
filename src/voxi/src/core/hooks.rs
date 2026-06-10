use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::io::Write;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct HookRule {
    pub event: String,       // "pre_tool" or "post_tool"
    pub matcher: String,     // tool name pattern or "*"
    pub action: String,      // "ask", "deny", "allow", or name of external script in hooks_dir
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HooksConfig {
    pub external_enabled: bool,
    pub hooks_dir: String,
    pub timeout_ms: u64,
    pub rules: Vec<HookRule>,
    /// When false (default), a relative/project-level `hooks_dir` (e.g.
    /// ".voxi/hooks") is treated as untrusted and ignored: scripts are resolved
    /// only from the installed trusted directory (`~/.voxi/hooks/`). Set true to
    /// opt into executing hooks from the project-local directory.
    #[serde(default)]
    pub enable_project_hooks: bool,
    /// Absolute path to the trusted installed hooks directory (`<data>/hooks`).
    /// Populated at load() time; never serialized.
    #[serde(skip)]
    installed_hooks_dir: PathBuf,
}

impl Default for HooksConfig {
    fn default() -> Self {
        HooksConfig {
            external_enabled: false,
            hooks_dir: ".voxi/hooks".to_string(),
            timeout_ms: 30000, // 30 seconds default
            rules: Vec::new(),
            enable_project_hooks: false,
            installed_hooks_dir: PathBuf::new(),
        }
    }
}

pub enum HookDecision {
    Allow,
    Deny(String),
    Ask,
}

impl HooksConfig {
    pub fn load(config_dir: &Path) -> Self {
        // The trusted installed hooks directory is a sibling of the config dir:
        // <data>/config → <data>/hooks (i.e. ~/.voxi/hooks).
        let installed_hooks_dir = config_dir
            .parent()
            .map(|data_dir| data_dir.join("hooks"))
            .unwrap_or_else(|| PathBuf::from("hooks"));

        let path = config_dir.join("hooks.json");
        let mut config = if !path.exists() {
            Self::default()
        } else {
            match std::fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(config) => config,
                    Err(e) => {
                        log::error!("Failed to parse hooks.json: {}", e);
                        Self::default()
                    }
                },
                Err(e) => {
                    log::error!("Failed to read hooks.json: {}", e);
                    Self::default()
                }
            }
        };
        config.installed_hooks_dir = installed_hooks_dir;
        config
    }

    /// Resolve the directory external hook scripts are loaded from, applying the
    /// trusted-location policy. Absolute `hooks_dir` paths are honored as-is.
    /// Relative/project paths are only used when `enable_project_hooks` is true;
    /// otherwise the installed trusted directory is used.
    fn resolve_hooks_dir(&self) -> PathBuf {
        let configured = Path::new(&self.hooks_dir);
        if configured.is_absolute() {
            configured.to_path_buf()
        } else if self.enable_project_hooks {
            configured.to_path_buf()
        } else {
            self.installed_hooks_dir.clone()
        }
    }

    pub fn save(&self, config_dir: &Path) -> Result<(), String> {
        let path = config_dir.join("hooks.json");
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize hooks: {}", e))?;
        std::fs::write(&path, content)
            .map_err(|e| format!("Failed to write hooks.json: {}", e))?;
        Ok(())
    }

    pub fn evaluate_pre_tool(&self, tool_name: &str, args: &serde_json::Value) -> HookDecision {
        for rule in &self.rules {
            if rule.event != "pre_tool" {
                continue;
            }

            if rule.matcher != "*" && rule.matcher != tool_name {
                continue;
            }

            match rule.action.as_str() {
                "allow" => return HookDecision::Allow,
                "deny" => return HookDecision::Deny("Blocked by pre_tool hook".into()),
                "ask" => return HookDecision::Ask,
                script_filename => {
                    if !self.external_enabled {
                        return HookDecision::Deny("External hooks are disabled".into());
                    }

                    // Resolve hooks directory under the trusted-location policy.
                    let base_dir = self.resolve_hooks_dir();
                    if !base_dir.exists() {
                        return HookDecision::Deny(format!(
                            "Hooks directory does not exist: {}",
                            base_dir.display()
                        ));
                    }

                    let payload = serde_json::json!({
                        "event": "pre_tool",
                        "tool": tool_name,
                        "arguments": args
                    });

                    match execute_external_hook(&base_dir, script_filename, &payload, self.timeout_ms) {
                        Ok(true) => return HookDecision::Allow,
                        Ok(false) => return HookDecision::Deny("Denied by external pre_tool hook".into()),
                        Err(e) => return HookDecision::Deny(format!("External hook execution error: {}", e)),
                    }
                }
            }
        }

        HookDecision::Allow
    }

    pub fn evaluate_post_tool(&self, tool_name: &str, args: &serde_json::Value, result: &serde_json::Value) -> HookDecision {
        for rule in &self.rules {
            if rule.event != "post_tool" {
                continue;
            }

            if rule.matcher != "*" && rule.matcher != tool_name {
                continue;
            }

            match rule.action.as_str() {
                "allow" => return HookDecision::Allow,
                "deny" => return HookDecision::Deny("Blocked by post_tool hook".into()),
                "ask" => return HookDecision::Ask,
                script_filename => {
                    if !self.external_enabled {
                        return HookDecision::Deny("External hooks are disabled".into());
                    }

                    let base_dir = self.resolve_hooks_dir();
                    if !base_dir.exists() {
                        return HookDecision::Deny(format!(
                            "Hooks directory does not exist: {}",
                            base_dir.display()
                        ));
                    }

                    let payload = serde_json::json!({
                        "event": "post_tool",
                        "tool": tool_name,
                        "arguments": args,
                        "result": result
                    });

                    match execute_external_hook(&base_dir, script_filename, &payload, self.timeout_ms) {
                        Ok(true) => return HookDecision::Allow,
                        Ok(false) => return HookDecision::Deny("Denied by external post_tool hook".into()),
                        Err(e) => return HookDecision::Deny(format!("External hook execution error: {}", e)),
                    }
                }
            }
        }

        HookDecision::Allow
    }
}

fn execute_external_hook(
    hooks_dir: &Path,
    action: &str,
    payload: &serde_json::Value,
    timeout_ms: u64,
) -> Result<bool, String> {
    let resolved_hooks_dir = std::fs::canonicalize(hooks_dir)
        .map_err(|e| format!("Failed to canonicalize hooks directory: {}", e))?;

    let script_path = hooks_dir.join(action);
    let resolved_script_path = match std::fs::canonicalize(&script_path) {
        Ok(p) => p,
        Err(e) => return Err(format!("Script path does not exist: {} ({})", script_path.display(), e)),
    };

    if !resolved_script_path.starts_with(&resolved_hooks_dir) {
        return Err("Security Violation: Hook script path resolves outside hooks directory".to_string());
    }

    let mut cmd = Command::new(&resolved_script_path);
    cmd.env_clear();

    for var in &["PATH", "HOME", "USER", "SHELL", "TMPDIR"] {
        if let Ok(val) = std::env::var(var) {
            cmd.env(var, val);
        }
    }

    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn()
        .map_err(|e| format!("Failed to spawn hook script: {}", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        let payload_str = payload.to_string();
        if let Err(e) = stdin.write_all(payload_str.as_bytes()) {
            return Err(format!("Failed to write to hook stdin: {}", e));
        }
    }

    let child_id = child.id();
    
    let (tx, rx) = std::sync::mpsc::channel();
    
    let thread_handle = std::thread::spawn(move || {
        let res = child.wait_with_output();
        let _ = tx.send(res);
    });

    match rx.recv_timeout(std::time::Duration::from_millis(timeout_ms)) {
        Ok(res) => {
            let _ = thread_handle.join();
            let output = res.map_err(|e| format!("Failed to wait for hook script: {}", e))?;
            if output.status.success() {
                Ok(true)
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                Err(format!("Hook script exited with non-zero status. Stderr: {}", stderr))
            }
        }
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            #[cfg(unix)]
            unsafe {
                libc::kill(child_id as libc::pid_t, libc::SIGKILL);
            }
            #[cfg(not(unix))]
            {
                let _ = Command::new("taskkill")
                    .args(&["/F", "/PID", &child_id.to_string()])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            }
            Err(format!("Hook script execution timed out after {} ms", timeout_ms))
        }
        Err(e) => {
            Err(format!("Channel error while waiting for hook script: {}", e))
        }
    }
}
