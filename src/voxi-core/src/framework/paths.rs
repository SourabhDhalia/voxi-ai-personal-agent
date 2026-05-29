//! Platform-resolved paths.
//!
//! Determines the correct directories for data, config, tools, skills,
//! plugins, embedded tool descriptors, and web assets based on
//! environment variables or defaults.
//!
//! Priority:
//! 1. Environment variables (VOXI_DATA_DIR, VOXI_TOOLS_DIR, etc.)
//! 2. Host Linux paths (~/.voxi)

use std::path::PathBuf;

/// All resolved platform paths.
#[derive(Debug, Clone)]
pub struct PlatformPaths {
    /// Main data directory (configs, sessions, etc.)
    pub data_dir: PathBuf,
    /// Configuration files directory
    pub config_dir: PathBuf,
    /// Tool scripts directory
    pub tools_dir: PathBuf,
    /// Textual skills directory
    pub skills_dir: PathBuf,
    /// Skill hub mount directory containing external OpenClaw-style roots
    pub skill_hubs_dir: PathBuf,
    /// Voxi-owned embedded tool descriptor directory
    pub embedded_tools_dir: PathBuf,
    /// Plugin .so files directory
    pub plugins_dir: PathBuf,
    /// Packaged reference docs directory
    pub docs_dir: PathBuf,
    /// Web dashboard static files
    pub web_root: PathBuf,
    /// Workflows directory
    pub workflows_dir: PathBuf,
    /// Generated and reusable code directory
    pub codes_dir: PathBuf,
    /// Log directory
    pub logs_dir: PathBuf,
    /// Actions directory
    pub actions_dir: PathBuf,
    /// Pipelines directory
    pub pipelines_dir: PathBuf,
    /// LLM backend plugins directory
    pub llm_plugins_dir: PathBuf,
    /// CLI plugins metadata directory
    pub cli_plugins_dir: PathBuf,
}

impl PlatformPaths {
    /// Auto-detect paths based on environment and OS.
    pub fn detect() -> Self {
        // Check environment overrides first
        if let Ok(data_dir) = std::env::var("VOXI_DATA_DIR") {
            return Self::from_base(PathBuf::from(data_dir));
        }

        // Fallback: XDG-compliant Linux paths
        Self::linux_defaults()
    }

    /// Build paths from a custom base directory.
    pub fn from_base(base: PathBuf) -> Self {
        let tools_dir = std::env::var("VOXI_TOOLS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| base.join("tools"));

        let skills_dir = base.join("workspace/skills");
        let skill_hubs_dir = std::env::var("VOXI_SKILL_HUBS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| base.join("workspace/skill-hubs"));
        let embedded_tools_dir = std::env::var("VOXI_EMBEDDED_TOOLS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| base.join("embedded"));

        PlatformPaths {
            config_dir: base.join("config"),
            tools_dir,
            skills_dir,
            skill_hubs_dir,
            embedded_tools_dir,
            plugins_dir: base.join("plugins"),
            docs_dir: base.join("docs"),
            web_root: base.join("web"),
            workflows_dir: base.join("workflows"),
            codes_dir: base.join("codes"),
            logs_dir: base.join("logs"),
            actions_dir: base.join("actions"),
            pipelines_dir: base.join("pipelines"),
            llm_plugins_dir: base.join("plugins/llm"),
            cli_plugins_dir: base.join("plugins/cli"),
            data_dir: base,
        }
    }

    /// Host Linux paths.
    fn linux_defaults() -> Self {
        let base = dirs_or_home().join(".voxi");
        Self::from_base(base)
    }

    /// Ensure all directories exist (create if missing).
    pub fn ensure_dirs(&self) {
        let dirs = [
            &self.data_dir,
            &self.config_dir,
            &self.tools_dir,
            &self.skills_dir,
            &self.skill_hubs_dir,
            &self.embedded_tools_dir,
            &self.plugins_dir,
            &self.docs_dir,
            &self.web_root,
            &self.workflows_dir,
            &self.codes_dir,
            &self.logs_dir,
        ];
        for dir in &dirs {
            if !dir.exists() {
                if let Err(e) = std::fs::create_dir_all(dir) {
                    log::error!("Warning: failed to create dir {:?}: {}", dir, e);
                }
            }
        }
    }

    /// Get the session database path.
    pub fn sessions_db_path(&self) -> PathBuf {
        self.data_dir.join("sessions/sessions.db")
    }

    /// Get the app data directory for file-based storage.
    pub fn app_data_dir(&self) -> PathBuf {
        self.data_dir.clone()
    }

    /// Discover external skill roots mounted under `workspace/skill-hubs`.
    pub fn discover_skill_hub_roots(&self) -> Vec<PathBuf> {
        let mut roots = Vec::new();
        let Ok(entries) = std::fs::read_dir(&self.skill_hubs_dir) else {
            return roots;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                roots.push(path);
            }
        }

        roots.sort();
        roots
    }
}

/// Get the user's home directory.
fn dirs_or_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_base_places_embedded_tools_under_base() {
        let base = PathBuf::from("/tmp/voxi-paths");
        let paths = PlatformPaths::from_base(base.clone());

        assert_eq!(paths.tools_dir, base.join("tools"));
        assert_eq!(paths.skills_dir, base.join("workspace/skills"));
        assert_eq!(paths.skill_hubs_dir, base.join("workspace/skill-hubs"));
        assert_eq!(paths.embedded_tools_dir, base.join("embedded"));
        assert_eq!(paths.codes_dir, base.join("codes"));
    }

    #[test]
    fn ensure_dirs_creates_embedded_directory() {
        let unique = format!(
            "voxi-paths-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let base = std::env::temp_dir().join(unique);
        let paths = PlatformPaths::from_base(base.clone());

        paths.ensure_dirs();

        assert!(paths.skill_hubs_dir.exists());
        assert!(paths.embedded_tools_dir.exists());
        assert!(paths.codes_dir.exists());

        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn discover_skill_hub_roots_lists_child_directories() {
        let unique = format!(
            "voxi-skill-hubs-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let base = std::env::temp_dir().join(unique);
        let paths = PlatformPaths::from_base(base.clone());
        std::fs::create_dir_all(&paths.skill_hubs_dir).unwrap();
        std::fs::create_dir_all(paths.skill_hubs_dir.join("openclaw")).unwrap();
        std::fs::write(paths.skill_hubs_dir.join("README.md"), "ignore").unwrap();

        let roots = paths.discover_skill_hub_roots();

        assert_eq!(roots, vec![paths.skill_hubs_dir.join("openclaw")]);

        let _ = std::fs::remove_dir_all(base);
    }
}
