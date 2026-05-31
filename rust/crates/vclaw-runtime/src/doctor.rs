use serde::{Deserialize, Serialize};
use crate::config::RuntimeConfig;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PathCheckResult {
    pub name: String,
    pub path: String,
    pub readable: bool,
    pub writable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCheckResult {
    pub name: String,
    pub found: bool,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnvCheckResult {
    pub name: String,
    pub present: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpCheckResult {
    pub name: String,
    pub configured: bool,
    pub reachable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DoctorSummary {
    pub paths: Vec<PathCheckResult>,
    pub tools: Vec<ToolCheckResult>,
    pub env: Vec<EnvCheckResult>,
    pub mcp: Vec<McpCheckResult>,
}

pub fn run_diagnostics(config: &RuntimeConfig) -> DoctorSummary {
    // 1. Audit paths
    let paths = vec![
        check_path("root_dir", &config.paths.root_dir),
        check_path("session_dir", &config.paths.session_dir),
        check_path("plugin_dir", &config.paths.plugin_dir),
        check_path("log_dir", &config.paths.log_dir),
    ];

    // 2. Audit tools
    let tools = vec![
        check_tool("git"),
        check_tool("curl"),
        check_tool("python3"),
    ];

    // 3. Audit env keys
    let env = vec![
        check_env("GEMINI_API_KEY"),
        check_env("ANTHROPIC_API_KEY"),
        check_env("OPENAI_API_KEY"),
    ];

    // 4. Audit MCP servers configured
    let mcp = config
        .mcp
        .servers
        .iter()
        .map(|server| {
            let path_found = check_command_exists(&server.command);
            McpCheckResult {
                name: server.server_name.clone(),
                configured: true,
                reachable: path_found,
            }
        })
        .collect::<Vec<_>>();

    DoctorSummary {
        paths,
        tools,
        env,
        mcp,
    }
}

fn check_path(name: &str, path_str: &str) -> PathCheckResult {
    let path = std::path::Path::new(path_str);
    let readable = path.exists();
    let mut writable = false;
    if readable {
        let test_file = path.join(".doctor_write_test");
        if std::fs::write(&test_file, b"test").is_ok() {
            writable = true;
            let _ = std::fs::remove_file(test_file);
        }
    } else {
        let mut ancestor = path;
        while let Some(parent) = ancestor.parent() {
            let check_p = if parent.as_os_str().is_empty() {
                std::path::Path::new(".")
            } else {
                parent
            };
            if check_p.exists() {
                ancestor = check_p;
                break;
            }
            ancestor = parent;
        }
        if ancestor.exists() {
            let test_file = ancestor.join(".doctor_write_test");
            if std::fs::write(&test_file, b"test").is_ok() {
                writable = true;
                let _ = std::fs::remove_file(test_file);
            }
        }
    }
    PathCheckResult {
        name: name.to_string(),
        path: path_str.to_string(),
        readable,
        writable,
    }
}

fn check_tool(name: &str) -> ToolCheckResult {
    let found = check_command_exists(name);
    ToolCheckResult {
        name: name.to_string(),
        found,
        version: None,
    }
}

fn check_command_exists(cmd: &str) -> bool {
    #[cfg(target_os = "windows")]
    let check_cmd = "where";
    #[cfg(not(target_os = "windows"))]
    let check_cmd = "which";

    std::process::Command::new(check_cmd)
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn check_env(name: &str) -> EnvCheckResult {
    EnvCheckResult {
        name: name.to_string(),
        present: std::env::var(name).is_ok(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_path_existing_writable() {
        let result = check_path("test_existing", ".");
        assert_eq!(result.name, "test_existing");
        assert_eq!(result.path, ".");
        assert!(result.readable);
        assert!(result.writable);
    }

    #[test]
    fn test_check_path_nonexistent_but_writable_parent() {
        let result = check_path("test_nonexistent", "./some_nonexistent_path_abc123/xyz");
        assert_eq!(result.name, "test_nonexistent");
        assert_eq!(result.path, "./some_nonexistent_path_abc123/xyz");
        assert!(!result.readable);
        assert!(result.writable);
    }
}
