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
}

impl Default for HooksConfig {
    fn default() -> Self {
        HooksConfig {
            external_enabled: false,
            hooks_dir: ".voxi/hooks".to_string(),
            timeout_ms: 30000, // 30 seconds default
            rules: Vec::new(),
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
        let path = config_dir.join("hooks.json");
        if !path.exists() {
            return Self::default();
        }

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

                    // Resolve hooks directory path (relative to the voxi run context, usually current dir or data dir)
                    let base_dir = Path::new(&self.hooks_dir);
                    if !base_dir.exists() {
                        return HookDecision::Deny(format!("Hooks directory does not exist: {}", self.hooks_dir));
                    }

                    let payload = serde_json::json!({
                        "event": "pre_tool",
                        "tool": tool_name,
                        "arguments": args
                    });

                    match execute_external_hook(base_dir, script_filename, &payload) {
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

                    let base_dir = Path::new(&self.hooks_dir);
                    if !base_dir.exists() {
                        return HookDecision::Deny(format!("Hooks directory does not exist: {}", self.hooks_dir));
                    }

                    let payload = serde_json::json!({
                        "event": "post_tool",
                        "tool": tool_name,
                        "arguments": args,
                        "result": result
                    });

                    match execute_external_hook(base_dir, script_filename, &payload) {
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
        stdin.write_all(payload_str.as_bytes())
            .map_err(|e| format!("Failed to write to hook stdin: {}", e))?;
    }

    let output = child.wait_with_output()
        .map_err(|e| format!("Failed to wait for hook script: {}", e))?;

    if output.status.success() {
        Ok(true)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("Hook script exited with non-zero status. Stderr: {}", stderr))
    }
}
