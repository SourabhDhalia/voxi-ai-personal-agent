use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
#[cfg(unix)]
use std::os::unix::process::CommandExt;

use serde::{Deserialize, Serialize};

use crate::DiscoveredPlugin;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HookPhase {
    PrePrompt,
    PreTool,
    PostTool,
    PostSession,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookSpec {
    pub name: String,
    pub phase: HookPhase,
    pub command: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_enabled() -> bool {
    true
}

fn default_timeout_ms() -> u64 {
    30000
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HookExecutionStatus {
    Succeeded,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookExecutionResult {
    pub hook_name: String,
    pub phase: HookPhase,
    pub command_path: PathBuf,
    pub status: HookExecutionStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub stdout: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub stderr: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookExecutionReport {
    pub plugin_name: String,
    pub phase: HookPhase,
    pub results: Vec<HookExecutionResult>,
}

impl HookExecutionReport {
    pub fn has_failures(&self) -> bool {
        self.results
            .iter()
            .any(|result| result.status == HookExecutionStatus::Failed)
    }
}

pub fn execute_plugin_hooks(
    plugin: &DiscoveredPlugin,
    phase: HookPhase,
    extra_env: &BTreeMap<String, String>,
) -> HookExecutionReport {
    let results = plugin
        .manifest
        .lifecycle
        .hooks
        .iter()
        .filter(|hook| hook.phase == phase)
        .map(|hook| execute_hook(&plugin.root, &plugin.manifest.name, hook, extra_env))
        .collect();

    HookExecutionReport {
        plugin_name: plugin.manifest.name.clone(),
        phase,
        results,
    }
}

fn execute_hook(
    plugin_root: &Path,
    plugin_name: &str,
    hook: &HookSpec,
    extra_env: &BTreeMap<String, String>,
) -> HookExecutionResult {
    let resolved = resolve_command_path(plugin_root, &hook.command);

    if !hook.enabled {
        return HookExecutionResult {
            hook_name: hook.name.clone(),
            phase: hook.phase.clone(),
            command_path: resolved.unwrap_or_else(|_| plugin_root.join(&hook.command)),
            status: HookExecutionStatus::Skipped,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            failure: None,
        };
    }

    let command_path = match resolved {
        Ok(path) => path,
        Err(err) => {
            return HookExecutionResult {
                hook_name: hook.name.clone(),
                phase: hook.phase.clone(),
                command_path: plugin_root.join(&hook.command),
                status: HookExecutionStatus::Failed,
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                failure: Some(err),
            };
        }
    };

    let mut cmd = Command::new("sh");
    cmd.arg(&command_path)
        .current_dir(plugin_root)
        .env("VCLAW_PLUGIN_NAME", plugin_name)
        .env("VCLAW_HOOK_NAME", &hook.name)
        .env(
            "VCLAW_HOOK_PHASE",
            serde_json::to_string(&hook.phase)
                .unwrap_or_else(|_| "\"unknown\"".to_string())
                .trim_matches('"')
                .to_string(),
        )
        .envs(extra_env)
        .envs(&hook.env)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(unix)]
    {
        cmd.process_group(0);
    }

    let child = match cmd.spawn() {
        Ok(child) => child,
        Err(err) => {
            return HookExecutionResult {
                hook_name: hook.name.clone(),
                phase: hook.phase.clone(),
                command_path,
                status: HookExecutionStatus::Failed,
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                failure: Some(format!("failed to execute hook: {err}")),
            };
        }
    };

    let child_id = child.id();
    let (tx, rx) = std::sync::mpsc::channel();
    let thread_handle = std::thread::spawn(move || {
        let res = child.wait_with_output();
        let _ = tx.send(res);
    });

    match rx.recv_timeout(std::time::Duration::from_millis(hook.timeout_ms)) {
        Ok(res) => {
            let _ = thread_handle.join();
            match res {
                Ok(output) => HookExecutionResult {
                    hook_name: hook.name.clone(),
                    phase: hook.phase.clone(),
                    command_path,
                    status: if output.status.success() {
                        HookExecutionStatus::Succeeded
                    } else {
                        HookExecutionStatus::Failed
                    },
                    exit_code: output.status.code(),
                    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                    failure: (!output.status.success())
                        .then(|| format!("hook exited with status {:?}", output.status.code())),
                },
                Err(err) => HookExecutionResult {
                    hook_name: hook.name.clone(),
                    phase: hook.phase.clone(),
                    command_path,
                    status: HookExecutionStatus::Failed,
                    exit_code: None,
                    stdout: String::new(),
                    stderr: String::new(),
                    failure: Some(format!("failed to wait for hook: {err}")),
                },
            }
        }
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            #[cfg(unix)]
            {
                let pgid = child_id as libc::pid_t;
                if pgid > 1 {
                    unsafe {
                        libc::kill(-pgid, libc::SIGKILL);
                    }
                }
            }
            #[cfg(not(unix))]
            {
                let _ = Command::new("taskkill")
                    .args(&["/F", "/T", "/PID", &child_id.to_string()])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            }
            let _ = thread_handle.join();
            HookExecutionResult {
                hook_name: hook.name.clone(),
                phase: hook.phase.clone(),
                command_path,
                status: HookExecutionStatus::Failed,
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                failure: Some(format!("hook timed out after {} ms", hook.timeout_ms)),
            }
        }
        Err(e) => {
            let _ = thread_handle.join();
            HookExecutionResult {
                hook_name: hook.name.clone(),
                phase: hook.phase.clone(),
                command_path,
                status: HookExecutionStatus::Failed,
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                failure: Some(format!("channel error: {e}")),
            }
        }
    }
}

fn resolve_command_path(plugin_root: &Path, command: &str) -> Result<PathBuf, String> {
    let candidate = plugin_root.join(command);
    if !candidate.exists() {
        return Err(format!("hook command `{command}` does not exist"));
    }

    let canonical_root = plugin_root
        .canonicalize()
        .map_err(|err| format!("failed to canonicalize plugin root: {err}"))?;
    let canonical_path = candidate
        .canonicalize()
        .map_err(|err| format!("failed to canonicalize hook command `{command}`: {err}"))?;

    if !canonical_path.starts_with(&canonical_root) {
        return Err(format!(
            "hook command `{command}` escapes the plugin root `{}`",
            plugin_root.display()
        ));
    }

    Ok(canonical_path)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;

    use crate::{
        DiscoveredPlugin, PluginDiscoverySource, PluginKind, PluginLifecycleDefinition,
        PluginManifest, PluginMetadata,
    };

    use super::*;

    fn temp_dir(prefix: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "{prefix}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    fn test_plugin(root: PathBuf, hooks: Vec<HookSpec>) -> DiscoveredPlugin {
        DiscoveredPlugin {
            source: PluginDiscoverySource::Directory,
            root: root.clone(),
            manifest_path: root.join("plugin.json"),
            manifest: PluginManifest {
                schema_version: 1,
                name: "sample-hooks".to_string(),
                kind: PluginKind::Tooling,
                summary: "Hook test plugin".to_string(),
                metadata: PluginMetadata {
                    version: "1.0.0".to_string(),
                    ..PluginMetadata::default()
                },
                permissions: Vec::new(),
                lifecycle: PluginLifecycleDefinition {
                    default_phase: None,
                    hooks,
                },
                commands: Vec::new(),
                tools: Vec::new(),
            },
        }
    }

    #[test]
    fn hook_execution_captures_structured_success_and_failure() {
        let root = temp_dir("vclaw-plugin-hook-exec");
        let hooks_dir = root.join("hooks");
        fs::create_dir_all(&hooks_dir).expect("create hooks dir");
        fs::write(
            hooks_dir.join("pre.sh"),
            "#!/bin/sh\necho pre:$VCLAW_PLUGIN_NAME\n",
        )
        .expect("write pre hook");
        fs::write(
            hooks_dir.join("post.sh"),
            "#!/bin/sh\necho boom >&2\nexit 7\n",
        )
        .expect("write post hook");

        let plugin = test_plugin(
            root.clone(),
            vec![
                HookSpec {
                    name: "pre".to_string(),
                    phase: HookPhase::PreTool,
                    command: "hooks/pre.sh".to_string(),
                    enabled: true,
                    env: BTreeMap::new(),
                    timeout_ms: 30000,
                },
                HookSpec {
                    name: "post".to_string(),
                    phase: HookPhase::PreTool,
                    command: "hooks/post.sh".to_string(),
                    enabled: true,
                    env: BTreeMap::new(),
                    timeout_ms: 30000,
                },
            ],
        );

        let report = execute_plugin_hooks(&plugin, HookPhase::PreTool, &BTreeMap::new());

        assert_eq!(report.results.len(), 2);
        assert_eq!(report.results[0].status, HookExecutionStatus::Succeeded);
        assert!(report.results[0].stdout.contains("pre:sample-hooks"));
        assert_eq!(report.results[1].status, HookExecutionStatus::Failed);
        assert_eq!(report.results[1].exit_code, Some(7));
        assert!(report.results[1].stderr.contains("boom"));
        assert!(report.has_failures());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn disabled_hook_is_reported_as_skipped() {
        let root = temp_dir("vclaw-plugin-hook-skip");
        let plugin = test_plugin(
            root.clone(),
            vec![HookSpec {
                name: "skip".to_string(),
                phase: HookPhase::PostSession,
                command: "hooks/missing.sh".to_string(),
                enabled: false,
                env: BTreeMap::new(),
                timeout_ms: 30000,
            }],
        );

        let report = execute_plugin_hooks(&plugin, HookPhase::PostSession, &BTreeMap::new());

        assert_eq!(report.results.len(), 1);
        assert_eq!(report.results[0].status, HookExecutionStatus::Skipped);
        assert!(!report.has_failures());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn hook_execution_times_out_and_is_killed() {
        let root = temp_dir("vclaw-plugin-hook-timeout");
        let hooks_dir = root.join("hooks");
        fs::create_dir_all(&hooks_dir).expect("create hooks dir");
        fs::write(
            hooks_dir.join("slow.sh"),
            "#!/bin/sh\nsleep 5\necho done\n",
        )
        .expect("write slow hook");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(hooks_dir.join("slow.sh"), fs::Permissions::from_mode(0o755)).expect("set permissions");
        }

        let plugin = test_plugin(
            root.clone(),
            vec![HookSpec {
                name: "slow".to_string(),
                phase: HookPhase::PrePrompt,
                command: "hooks/slow.sh".to_string(),
                enabled: true,
                env: BTreeMap::new(),
                timeout_ms: 100,
            }],
        );

        let report = execute_plugin_hooks(&plugin, HookPhase::PrePrompt, &BTreeMap::new());

        assert_eq!(report.results.len(), 1);
        assert_eq!(report.results[0].status, HookExecutionStatus::Failed);
        assert!(report.results[0]
            .failure
            .as_ref()
            .map(|f| f.contains("timed out after 100 ms"))
            .unwrap_or(false));
        assert!(report.has_failures());

        let _ = fs::remove_dir_all(root);
    }
}
